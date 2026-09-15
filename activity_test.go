//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestActivityMergesBothSourcesNewestFirst(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	// Source « moteur » : décisions locales, hors machine à états.
	if e := s.controlEvent(w.ID, "dispatch", "t1 : départ automatique · fixture"); e != nil {
		t.Fatal(e)
	}
	// Source « métier » : mutations du travail, portées par une révision.
	applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})

	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	// Compter les entrées ne prouverait rien : chaque table porte déjà, à elle
	// seule, plusieurs entrées et les deux origines. Seul un type qu'une seule
	// table peut produire atteste qu'elle a réellement été lue : « dispatch »
	// n'existe que dans cockpit_events, « task.update » que dans events.
	kinds := map[string]bool{}
	for _, x := range page.Entries {
		kinds[x.Kind] = true
	}
	if !kinds["dispatch"] {
		t.Fatalf("cockpit_events non fusionnée : %+v", page.Entries)
	}
	if !kinds["task.update"] {
		t.Fatalf("events non fusionnée : %+v", page.Entries)
	}
	for i := 1; i < len(page.Entries); i++ {
		if page.Entries[i-1].At < page.Entries[i].At {
			t.Fatalf("ordre non décroissant : %s avant %s", page.Entries[i-1].At, page.Entries[i].At)
		}
	}
	origines := map[string]bool{}
	for _, x := range page.Entries {
		origines[x.Origin] = true
	}
	if !origines[activityEngine] || !origines[activityHuman] {
		t.Fatalf("les deux origines doivent être distinguées : %+v", page.Entries)
	}
}

// Le lancement d'agent sérialise un Launch, qui nomme la tâche « task_id » et
// n'a aucun champ « id ». Lire la seule orthographe des mutations laisserait la
// tentative orpheline : ce fil dirait qu'un départ a eu lieu sans dire sur quoi.
func TestActivityNamesTaskOfAgentStart(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if _, _, e := s.prepare(w.ID, r); e != nil {
		t.Fatal(e)
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	found := false
	for _, x := range page.Entries {
		if x.Kind != "agent.start" {
			continue
		}
		found = true
		if x.TaskID != r.TaskID {
			t.Fatalf("tentative non rattachée à sa tâche : task_id=%q attendu %q", x.TaskID, r.TaskID)
		}
		if !strings.HasPrefix(x.Message, r.TaskID) {
			t.Fatalf("le message doit nommer la tâche %q : %q", r.TaskID, x.Message)
		}
	}
	if !found {
		t.Fatalf("aucune tentative dans le fil : %+v", page.Entries)
	}
}

// Une dérogation est le geste humain le plus lourd du produit. La classer côté
// moteur ferait croire à une autonomie qui n'a pas eu lieu — l'inverse exact de
// ce que ce fil doit montrer.
func TestActivityAttributesOperatorDecisionsToHuman(t *testing.T) {
	for _, kind := range []string{"task.override", "task.submit", "plan.adopt", "brief.adopt", "retex-save"} {
		if got := activityOrigin(kind, activityPayload{}); got != activityHuman {
			t.Errorf("%s est un geste d'opérateur, classé %q", kind, got)
		}
		if activityLabel(kind) == kind {
			t.Errorf("%s sans libellé français : la chaîne technique serait affichée telle quelle", kind)
		}
	}
	// Les départs et relais restent au moteur : c'est ce que le fil doit rendre visible.
	for _, kind := range []string{"dispatch", "conductor"} {
		if got := activityOrigin(kind, activityPayload{}); got != activityEngine {
			t.Errorf("%s est une action du moteur, classée %q", kind, got)
		}
	}
}

func TestActivityDecisionsOnlyKeepsHumanArbitration(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if e := s.controlEvent(w.ID, "dispatch", "t1 : départ automatique"); e != nil {
		t.Fatal(e)
	}
	if e := s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 50, DecisionsOnly: true})
	if e != nil {
		t.Fatal(e)
	}
	if len(page.Entries) == 0 {
		t.Fatal("la suspension des départs est un arbitrage humain")
	}
	for _, x := range page.Entries {
		if x.Origin != activityHuman {
			t.Fatalf("entrée moteur conservée par le filtre : %+v", x)
		}
		if x.Kind == "dispatch" {
			t.Fatal("un départ automatique n'est pas un arbitrage humain")
		}
	}
}

func TestActivityCursorPaginatesWithoutLossOrDuplicate(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	for i := 0; i < 12; i++ {
		if e := s.controlEvent(w.ID, "dispatch", fmt.Sprintf("t%02d : départ automatique", i)); e != nil {
			t.Fatal(e)
		}
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 5})
	if e != nil {
		t.Fatal(e)
	}
	if !page.More {
		t.Fatal("douze entrées pour des pages de cinq : une suite existe")
	}
	vus := map[string]int{}
	for tours := 0; tours < 12; tours++ {
		for _, x := range page.Entries {
			vus[x.At+"|"+x.Message]++
		}
		if !page.More {
			break
		}
		if page, e = s.activity(w.ID, activityQuery{Limit: 5, Before: page.Next}); e != nil {
			t.Fatal(e)
		}
	}
	for cle, n := range vus {
		if n > 1 {
			t.Fatalf("entrée rendue %d fois par la pagination : %s", n, cle)
		}
	}
	// Douze départs, plus les mutations écrites par les helpers de fixture.
	if len(vus) < 12 {
		t.Fatalf("entrées perdues par la pagination : %d vues pour 12 écrites", len(vus))
	}
}

// Une demande excessive doit donner le plafond, jamais la plus petite page.
func TestActivityLimitClampsToCeiling(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	for i := 0; i < 60; i++ {
		if e := s.controlEvent(w.ID, "dispatch", fmt.Sprintf("t%02d : départ", i)); e != nil {
			t.Fatal(e)
		}
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 500})
	if e != nil {
		t.Fatal(e)
	}
	if len(page.Entries) <= 50 {
		t.Fatalf("une demande de 500 doit être ramenée au plafond, %d entrées rendues", len(page.Entries))
	}
}

// Plusieurs entrées peuvent partager un horodatage. Un curseur qui ne connaît
// que le temps les saute ensemble : la pagination perdait alors des entrées
// sans le dire, ce qu'aucun affichage ne peut rattraper.
func TestActivityCursorKeepsSimultaneousEntries(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	const simultane = "2026-09-14T10:00:00.000000000Z"
	const attendues = 6
	for i := 0; i < attendues; i++ {
		if _, e := s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)",
			w.ID, simultane, "dispatch", "départ simultané"); e != nil {
			t.Fatal(e)
		}
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 3})
	if e != nil {
		t.Fatal(e)
	}
	rendues, curseurs := 0, map[string]bool{}
	for tours := 0; tours < 12; tours++ {
		for _, x := range page.Entries {
			if x.At == simultane {
				rendues++
			}
			if curseurs[x.cursor()] {
				t.Fatalf("curseur non unique, entrée rendue deux fois : %s", x.cursor())
			}
			curseurs[x.cursor()] = true
		}
		if !page.More {
			break
		}
		if page, e = s.activity(w.ID, activityQuery{Limit: 3, Before: page.Next}); e != nil {
			t.Fatal(e)
		}
	}
	if rendues != attendues {
		t.Fatalf("entrées simultanées perdues par la pagination : %d rendues sur %d", rendues, attendues)
	}
}

// Un curseur rendu en fin de liste ferait demander une page qui n'existe pas.
func TestActivityCursorEmptyWhenNothingFollows(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if e := s.controlEvent(w.ID, "dispatch", "t1 : départ automatique"); e != nil {
		t.Fatal(e)
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	if page.More {
		t.Fatalf("préalable : tout doit tenir en une page, %+v", page)
	}
	if page.Next != "" {
		t.Fatalf("aucune suite : le curseur doit rester vide, obtenu %q", page.Next)
	}

	// Avec une suite, le curseur est rendu et mène à des entrées.
	tronque, e := s.activity(w.ID, activityQuery{Limit: 2})
	if e != nil {
		t.Fatal(e)
	}
	if !tronque.More || tronque.Next == "" {
		t.Fatalf("une suite existe : curseur attendu, %+v", tronque)
	}
	suite, e := s.activity(w.ID, activityQuery{Limit: 2, Before: tronque.Next})
	if e != nil {
		t.Fatal(e)
	}
	if len(suite.Entries) == 0 {
		t.Fatal("le curseur rendu doit mener à des entrées")
	}
}

// Le fil lit un nombre borné de lignes par source. Au-delà, il s'arrêtait sans
// le dire, ce qui laissait croire que l'historique s'arrêtait là.
func TestActivityDeclaresTruncatedHistory(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	for i := 0; i <= activitySourceLimit; i++ {
		if _, e := s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)",
			w.ID, now(), "dispatch", fmt.Sprintf("départ %d", i)); e != nil {
			t.Fatal(e)
		}
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	if !page.Truncated {
		t.Fatal("au-delà du plafond de lecture, le fil doit annoncer un historique tronqué")
	}

	// Sous le plafond, rien à signaler : ne pas alarmer sans raison.
	petit := storeTest(t)
	w2 := taskTest(t, petit, createTest(t, petit))
	if e := petit.controlEvent(w2.ID, "dispatch", "un seul départ"); e != nil {
		t.Fatal(e)
	}
	page2, e := petit.activity(w2.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	if page2.Truncated {
		t.Fatal("sous le plafond, l'historique est complet et ne doit pas se dire tronqué")
	}
}

// Chaque entrée du fil doit nommer la tâche concernée quand la charge utile la
// porte. La gate était la seule à ne pas le faire, alors que sa charge utile
// réelle (gate_dialog.go) contient bien task_id.
func TestActivityNamesTaskOfGate(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	// Charge utile identique à celle qu'écrit recordDialogGate.
	raw, _ := json.Marshal(map[string]any{
		"schema_version": 1, "event_id": newID("operator-"), "expected_revision": w.Revision,
		"task_id": "t1", "phase": "delivery", "name": "Gate de recette",
	})
	if _, e := s.mutate(w.ID, "gate", newID("operator-"), w.Revision, raw, func(*Work) error { return nil }); e != nil {
		t.Fatal(e)
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	for _, x := range page.Entries {
		if x.Kind != "gate" {
			continue
		}
		if x.TaskID != "t1" {
			t.Fatalf("gate non rattachée à sa tâche : %+v", x)
		}
		if !strings.HasPrefix(x.Message, "t1") {
			t.Fatalf("le message de gate doit nommer sa tâche : %q", x.Message)
		}
		return
	}
	t.Fatal("aucune entrée de gate au fil")
}

// « task.update » est écrit par l'opérateur et par le moteur. Tant que le fil
// se fiait au seul type d'événement, tout geste d'opérateur sur une tâche —
// blocage, reprise, acceptation — s'affichait « moteur », c'est-à-dire une
// autonomie qui n'a pas eu lieu. Ce test emprunte les deux vrais chemins, et
// non la fonction de classement, parce que c'est le câblage qui manquait.
func TestActivitySeparatesOperatorFromEngineOnTaskUpdate(t *testing.T) {
	s := storeTest(t)
	w, a := conductorAgent(t, s)

	// Chemin moteur : fin de tentative relayée par le conducteur.
	writeReport(t, s, "t1.md", "handoff relayé")
	finishCompleted(t, s, a)

	// Chemin opérateur : après le relais, l'humain reprend la main sur la
	// tâche — exactement ce que fait le cockpit web.
	if _, e := s.webAction(webRequest{Kind: "task", Work: w.ID, Task: "t1", Event: newID("op-"),
		Revision: currentRevision(t, s, w.ID),
		Request:  Request{Status: "blocked", Blocker: "dépendance externe"}}); e != nil {
		t.Fatal(e)
	}

	// Un appelant qui se déclare moteur reste un humain : le point d'entrée
	// efface le champ. Sans cela, n'importe quel client masquerait ses gestes
	// derrière l'autonomie du produit.
	if _, e := s.webAction(webRequest{Kind: "task", Work: w.ID, Task: "t1", Event: newID("op2-"),
		Revision: currentRevision(t, s, w.ID),
		Request:  Request{Status: "running", Origin: conductorAuthor, Next: "reprise manuelle"}}); e != nil {
		t.Fatal(e)
	}

	page, e := s.activity(w.ID, activityQuery{Limit: 100})
	if e != nil {
		t.Fatal(e)
	}
	var humain, moteur int
	for _, x := range page.Entries {
		if x.Kind != "task.update" {
			continue
		}
		switch x.Origin {
		case activityHuman:
			humain++
		case activityEngine:
			moteur++
		}
	}
	var menteur bool
	for _, x := range page.Entries {
		if x.Kind == "task.update" && strings.Contains(x.Message, "En cours") {
			menteur = true
			if x.Origin != activityHuman {
				t.Fatalf("un appelant s'est déclaré moteur et a été cru : %+v", x)
			}
		}
	}
	if !menteur {
		t.Fatal("la reprise envoyée par le réseau est absente du fil")
	}
	if humain == 0 {
		t.Fatalf("la reprise décidée par l'opérateur doit lui être attribuée : %+v", page.Entries)
	}
	if moteur == 0 {
		t.Fatalf("la fin de tentative relayée par le moteur doit lui être attribuée : %+v", page.Entries)
	}

	// Le filtre « décisions seulement » sert à retrouver ce qu'un humain a
	// tranché : il doit garder le blocage et écarter les écritures du moteur.
	seules, e := s.activity(w.ID, activityQuery{Limit: 100, DecisionsOnly: true})
	if e != nil {
		t.Fatal(e)
	}
	var garde bool
	for _, x := range seules.Entries {
		if x.Origin != activityHuman {
			t.Fatalf("le filtre laisse passer une écriture du moteur : %+v", x)
		}
		if x.Kind == "task.update" && strings.Contains(x.Message, "dépendance externe") {
			garde = true
		}
	}
	if !garde {
		t.Fatalf("la décision humaine est absente du filtre des décisions : %+v", seules.Entries)
	}
}

func currentRevision(t *testing.T, s *Store, work string) int {
	t.Helper()
	w, e := s.get(work)
	if e != nil {
		t.Fatal(e)
	}
	return w.Revision
}

// La même règle vaut pour la ligne de commande : un script qui se déclare
// moteur écrirait une autonomie qui n'a pas eu lieu, et le fil existe pour
// démentir cela, pas pour le relayer.
func TestActivityCommandLineCannotClaimEngineOrigin(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	r := Request{Schema: 1, EventID: newID("cli-"), Revision: current.Revision, ID: "t1",
		Status: "blocked", Blocker: "attente de décision", Origin: conductorAuthor}
	raw, _ := json.Marshal(r)
	path := filepath.Join(s.root, "demande.json")
	if e := os.WriteFile(path, raw, 0600); e != nil {
		t.Fatal(e)
	}
	var out, errs bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "task", "update", w.ID, "--input", path}, &out, &errs); code != 0 {
		t.Fatal(code, errs.String())
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	var vu bool
	for _, x := range page.Entries {
		if x.Kind == "task.update" && strings.Contains(x.Message, "attente de décision") {
			vu = true
			if x.Origin != activityHuman {
				t.Fatalf("la ligne de commande s'est déclarée moteur et a été crue : %+v", x)
			}
		}
	}
	if !vu {
		t.Fatal("la mutation envoyée par la ligne de commande est absente du fil")
	}
}

// Troisième porte : le terminal. Aucun écran ne permet aujourd'hui de saisir
// une origine, mais le geste passe par une fonction qui accepte une requête
// entière ; le jour où un appelant la remplit, elle doit rester humaine.
func TestActivityConsoleCannotClaimEngineOrigin(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	if e := s.operatorTask(w.ID, Request{ID: "t1", Status: "blocked",
		Blocker: "saisie au terminal", Origin: conductorAuthor}); e != nil {
		t.Fatal(e)
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	for _, x := range page.Entries {
		if x.Kind == "task.update" && strings.Contains(x.Message, "saisie au terminal") {
			if x.Origin != activityHuman {
				t.Fatalf("le terminal s'est déclaré moteur et a été cru : %+v", x)
			}
			return
		}
	}
	t.Fatalf("la saisie au terminal est absente du fil : %+v", page.Entries)
}

// Le moteur ferme lui-même des demandes : revalidation constatée, sujet
// remplacé, dérive devenue un état. Classées « vous », ces fermetures
// prêtaient à l'opérateur des décisions qu'il n'avait pas prises — le fil
// affichait « ● vous » sur un message disant « moteur ». Mesuré à l'écran.
func TestActivityAttributesEngineClosuresToEngine(t *testing.T) {
	if got := activityOrigin("decision.moteur", activityPayload{}); got != activityEngine {
		t.Errorf("une fermeture écrite par le moteur doit lui être attribuée, obtenu %q", got)
	}
	if got := activityOrigin("decision", activityPayload{}); got != activityHuman {
		t.Errorf("un acquittement humain reste humain, obtenu %q", got)
	}
	if activityLabel("decision.moteur") == "decision.moteur" {
		t.Error("libellé technique brut affiché tel quel")
	}
	if decisionEventKind("moteur") == decisionEventKind("fpizzi") {
		t.Error("le moteur et un opérateur écrivent le même type : le fil ne peut plus les distinguer")
	}
}
