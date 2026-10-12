//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func acceptedRecipeFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	raw := fixture(t, s.root)
	ev, e := evaluate(raw, s.root, "delivery")
	if e != nil {
		t.Fatal(e)
	}
	w, e = s.mutate(w.ID, "fixture-accepted-recipe", newID("fixture-"), w.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task("t1")
		task.Status = "accepted"
		task.Gate = &GateRecord{Document: raw, Evaluation: ev, At: now()}
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	return s, w
}
func TestPostDeliveryFreshRecipeRetexAndEvolution(t *testing.T) {
	s, w := acceptedRecipeFixture(t)
	p := s.postDelivery(&w)
	if !p.Ready || len(p.Sources) != 1 {
		t.Fatal("fresh recipe not offered", p)
	}
	record := Retex{ID: "r-recipe", SourceKind: "acceptance", Source: p.Sources[0], Problem: "Améliorer le parcours", Nature: "hypothese", OpportunityKind: "quick_win", Impact: "Gagner du temps à chaque reprise", Proposal: "Présenter les consignes ajoutées", Effort: "Faible, à confirmer", Risk: "Confusion avec la recette", Criteria: "Rejet persisté et original conservé", Proof: "Recette source à examiner"}
	request := webRequest{Kind: "retex-save", Work: w.ID, Event: "retex-once", Revision: w.Revision, Retex: record}
	if _, err := s.retexAction(request); err != nil {
		t.Fatal(err)
	}
	got, _ := s.get(w.ID)
	if len(got.Retex) != 1 || len(got.Tasks) != len(w.Tasks) || got.Tasks[0].Status != "accepted" || got.Retex[0].Status != "propose" {
		t.Fatal("retrospective launched or accepted an evolution")
	}
	if _, err := s.retexAction(request); err != nil {
		t.Fatal("replay", err)
	}
	seed, err := s.retexEvolution(w.ID, record.ID)
	if err != nil || !strings.Contains(seed["need"], record.Proposal) || !strings.Contains(seed["need"], record.Source.SHA256) {
		t.Fatal("source not received by next preparation", seed, err)
	}
	var cli bytes.Buffer
	if err = s.planningCLI([]string{"planning", "post-delivery", w.ID}, "", &cli); err != nil {
		t.Fatal(err)
	}
	var view PostDelivery
	if json.Unmarshal(cli.Bytes(), &view) != nil || !view.Ready {
		t.Fatal("public phase absent")
	}
	record.Source.SHA256 = strings.Repeat("0", 64)
	record.ID = "r-forged"
	request.Retex = record
	request.Event = "forged"
	request.Revision = got.Revision
	if _, err = s.retexAction(request); err == nil {
		t.Fatal("forged recipe accepted")
	}
	if _, err = s.retexAction(webRequest{Kind: "retex-task", Work: w.ID, Task: "r-recipe", Event: "launch-evolution", Revision: got.Revision}); err == nil {
		t.Fatal("recipe modified without adopting evolution")
	}
	if err = os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed source"), 0600); err != nil {
		t.Fatal(err)
	}
	stale, _ := s.get(w.ID)
	if s.postDelivery(&stale).Ready {
		t.Fatal("stale recipe offered")
	}
	if _, err = s.retexEvolution(w.ID, "r-recipe"); err == nil {
		t.Fatal("stale source reused")
	}
}
func TestPostDeliveryWaitingEmptyAndRootDecision(t *testing.T) {
	s, w := acceptedRecipeFixture(t)
	if s.postDelivery(&Work{}).Ready {
		t.Fatal("empty work recipe")
	}
	w.Tasks[0].Status = "todo"
	if s.postDelivery(&w).Ready {
		t.Fatal("unfinished work recipe")
	}
	w.Tasks[0].Status = "accepted"
	w.Planning = &PlanningState{Scopes: []PlanningScope{{ID: "root", State: "waiting"}}, HumanReviewAuthorized: now()}
	if s.postDelivery(&w).Ready {
		t.Fatal("engine ended supervision")
	}
	w.Planning.Scopes[0].State = "closed"
	if s.postDelivery(&w).Ready {
		t.Fatal("root closure bypassed required independent review")
	}
	p := fixtureProductPlan()
	p.Tasks[1].Phase = "retex"
	p.Tasks[1].Depends = []string{p.Tasks[0].ID}
	if err := validateProductPlan(p); err != nil {
		t.Fatal("planned retrospective phase refused", err)
	}
}
