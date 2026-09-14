//go:build linux

package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestDecisionDedupResolveAndArchive(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	one, e := s.decisions(w.ID)
	if e != nil || len(one) != 1 {
		t.Fatal(one, e)
	}
	two, e := s.decisions(w.ID)
	if e != nil || len(two) != 1 || two[0].ID != one[0].ID {
		t.Fatal("duplicate", e)
	}
	if e = s.resolveDecision(w.ID, one[0].ID, "reviewer", "Rapport lu, preuves à contrôler"); e != nil {
		t.Fatal(e)
	}
	current, _ := s.get(w.ID)
	if current.Tasks[0].Status != "submitted" {
		t.Fatal("acknowledgement accepted task")
	}
	if e = s.markVisit(w.ID, "operator", w.Revision); e != nil {
		t.Fatal(e)
	}
	zip := filepath.Join(t.TempDir(), "archive.zip")
	if e = s.export(w.ID, zip); e != nil {
		t.Fatal(e)
	}
	dest := storeTest(t)
	if _, e = dest.importBundle(zip); e != nil {
		t.Fatal(e)
	}
	got, e := dest.decisions(w.ID)
	if e != nil || len(got) != 1 || got[0].Author != "reviewer" || got[0].ResolvedAt == "" {
		t.Fatal(got, e)
	}
	v, e := dest.visit(w.ID, "operator")
	if e != nil || v.Revision != w.Revision {
		t.Fatal(v, e)
	}
}
func TestLogCursorLiteralSearchAndBounds(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	for i := 0; i < 31; i++ {
		if e = s.log(a.ID, "test", fmt.Sprintf("trace %% %02d", i)); e != nil {
			t.Fatal(e)
		}
	}
	p, e := s.queryLogs(w.ID, a.ID, "trace %", "", 0, 7)
	if e != nil || len(p.Entries) != 7 || !p.More {
		t.Fatal(p, e)
	}
	ids := map[int64]bool{}
	for {
		for _, l := range p.Entries {
			if ids[l.Seq] {
				t.Fatal("duplicate cursor")
			}
			ids[l.Seq] = true
		}
		if !p.More {
			break
		}
		p, e = s.queryLogs(w.ID, a.ID, "trace %", "", p.Next, 7)
		if e != nil {
			t.Fatal(e)
		}
	}
	if len(ids) != 31 {
		t.Fatal("lost records", len(ids))
	}
	p, e = s.queryLogs(w.ID, a.ID, "not found", "", 0, 7)
	if e != nil || len(p.Entries) != 0 {
		t.Fatal(p, e)
	}
}
func TestHierarchyRoleAndOODARestart(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Role = "planner"
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	tree := s.hierarchyText(w.ID)
	for _, x := range []string{w.Title, "t1", a.ID, "planner", "processus"} {
		if !strings.Contains(tree, x) {
			t.Fatal("missing", x)
		}
	}
	w, _ = s.get(w.ID)
	w, e = s.executeRequest(w.ID, "ooda", Request{Schema: 1, EventID: newID("test-"), Revision: w.Revision, Observation: "Échec constaté", Orientation: "Corriger la cause", Decision: "Revoir le test", Result: "Diagnostic enregistré", Next: "Rejouer le test corrigé", Owner: "reviewer"})
	if e != nil {
		t.Fatal(e)
	}
	if w.Summary != "Diagnostic enregistré" || !strings.Contains(s.resumeSinceText(w.ID, Visit{Operator: "test"}), "Revoir le test") {
		t.Fatal("OODA missing")
	}
}

// Une gate réenregistrée crée une carte par version. Tant que la précédente
// était rouverte de force à chaque lecture, le même sujet s'empilait : mesuré
// sur un travail réel, 86 cartes ouvertes pour 16 tâches et un seul texte.
// C'est l'inverse de ce que cet écran doit faire.
func TestDecisionsKeepOneCardPerGateSubject(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})

	ouvertes := func() []Decision {
		list, e := s.decisions(w.ID)
		if e != nil {
			t.Fatal(e)
		}
		out := []Decision{}
		for _, d := range list {
			if d.Kind == "gate" && d.ResolvedAt == "" {
				out = append(out, d)
			}
		}
		return out
	}

	// Trois évaluations successives de la même tâche, par le chemin réel du
	// moteur : chacune porte un horodatage propre, donc une version propre.
	// Évaluation en échec : c'est elle qui ouvre une demande de gate.
	raw := []byte(fmt.Sprintf(`{"method_version":"2","scope_id":"t1","artifacts":{"proof.txt":"%s"},"domains":{"quality":100},"checks":[{"id":"q","domain":"quality","mandatory":true,"gate":"delivery","penalty":100,"max_penalty":100,"severity":"major"}],"results":[{"id":"q","status":"FAIL","count":1,"evidence":["proof.txt"]}]}`, hash([]byte("proof"))))
	if e := os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("proof"), 0600); e != nil {
		t.Fatal(e)
	}
	for i := 0; i < 3; i++ {
		w = gateTest(t, s, w, raw)
		if n := len(ouvertes()); n > 1 {
			t.Fatalf("évaluation %d : %d cartes ouvertes pour un seul sujet", i+1, n)
		}
	}
	list := ouvertes()
	if len(list) != 1 {
		t.Fatalf("un sujet, une carte : %d cartes ouvertes", len(list))
	}
	// La carte qui subsiste doit porter le blocage : refermer les doublons ne
	// doit pas revenir à faire disparaître la demande de l'écran.
	if list[0].TaskID != "t1" || strings.TrimSpace(list[0].Evidence) == "" {
		t.Fatalf("la carte restante ne porte pas le blocage : %+v", list[0])
	}
	// Et les versions précédentes doivent être closes explicitement, pas
	// simplement absentes : l'historique dit pourquoi elles ont disparu.
	toutes, e := s.decisions(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	var remplacees int
	for _, d := range toutes {
		if d.Kind == "gate" && strings.Contains(d.Resolution, "Sujet remplacé") {
			remplacees++
		}
	}
	if remplacees == 0 {
		t.Fatal("aucune carte close comme remplacée : la supersession n'est pas tracée")
	}

	// Non-résurrection. C'est le défaut mesuré sur un travail réel : tant que
	// toute carte de gate d'une tâche bloquée était rouverte à l'affichage, des
	// décisions déjà closes — dont certaines acquittées par un humain —
	// revenaient à chaque lecture. Le blocage justifie une carte, pas la
	// réouverture de toutes les précédentes.
	closes := map[string]string{}
	for _, d := range toutes {
		if d.ResolvedAt != "" {
			closes[d.ID] = d.Resolution
		}
	}
	if len(closes) == 0 {
		t.Fatal("aucune carte close : le scénario ne prouve rien de la résurrection")
	}
	for essai := 0; essai < 2; essai++ {
		relu, e := s.decisions(w.ID)
		if e != nil {
			t.Fatal(e)
		}
		for _, d := range relu {
			if _, etaitClose := closes[d.ID]; etaitClose && d.ResolvedAt == "" {
				t.Fatalf("lecture %d : une décision close est réapparue ouverte : %+v", essai+1, d)
			}
		}
	}
}
