//go:build linux

package main

import (
	"fmt"
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
		if got := activityOrigin(kind); got != activityHuman {
			t.Errorf("%s est un geste d'opérateur, classé %q", kind, got)
		}
		if activityLabel(kind) == kind {
			t.Errorf("%s sans libellé français : la chaîne technique serait affichée telle quelle", kind)
		}
	}
	// Les départs et relais restent au moteur : c'est ce que le fil doit rendre visible.
	for _, kind := range []string{"dispatch", "conductor"} {
		if got := activityOrigin(kind); got != activityEngine {
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
