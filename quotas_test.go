//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func quotaInt(n int) *int { return &n }
func TestQuotasPublicParityPersistenceAndNoRefund(t *testing.T) {
	s, w := planningFixture(t)
	var err error
	w, err = s.mutate(w.ID, "test.quotas", "quota-fixture", w.Revision, []byte(`{}`), func(w *Work) error {
		p := w.Planning
		p.Activations = 4
		p.Decisions = 3
		p.Scopes[0].Activations = 4
		p.Scopes[0].ActivationLimit = 5
		p.Reviewer = &ReviewerConfig{Provider: "fixture", MaxCalls: 7, Calls: 2}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	r := QuotaChange{Schema: 1, EventID: "quota-change", Revision: w.Revision, Limits: QuotaValues{Planning: 20, Decisions: 15, Reviews: quotaInt(9)}, Reason: "Autorisation de recette isolée"}
	raw, _ := json.Marshal(r)
	path := filepath.Join(t.TempDir(), "quota.json")
	os.WriteFile(path, raw, 0600)
	var out bytes.Buffer
	if err = s.quotasCLI([]string{"quotas", "preview", w.ID}, path, &out); err != nil {
		t.Fatal(err)
	}
	before, _ := s.get(w.ID)
	if before.Revision != w.Revision {
		t.Fatal("preview mutated")
	}
	if _, err = s.webAction(webRequest{Kind: "quotas", Work: w.ID, Revision: r.Revision, Event: r.EventID, Quotas: r}); err != nil {
		t.Fatal(err)
	}
	current, _ := s.get(w.ID)
	if current.Planning.Activations != 4 || current.Planning.Decisions != 3 || current.Planning.Reviewer.Calls != 2 || current.Planning.Scopes[0].ActivationLimit != 5 {
		t.Fatal("history/child allocation altered")
	}
	out.Reset()
	if err = s.quotasCLI([]string{"quotas", "apply", w.ID}, path, &out); err != nil {
		t.Fatal(err)
	}
	replayed, _ := s.get(w.ID)
	if replayed.Revision != current.Revision {
		t.Fatal("replay mutated")
	}
	r.EventID = "stale"
	if _, err = s.configureQuotas(w.ID, r); err == nil || commandFailure(err).Code != "revision_conflict" {
		t.Fatal(err)
	}
	r.Revision = current.Revision
	r.Limits.Planning = 3
	r.EventID = "below-used"
	if _, err = s.configureQuotas(w.ID, r); err == nil {
		t.Fatal("below consumption allowed")
	}
	r.Limits.Planning = 4
	r.Limits.Decisions = 3
	r.Limits.Reviews = quotaInt(2)
	r.EventID = "at-used"
	if _, err = s.configureQuotas(w.ID, r); err != nil {
		t.Fatal(err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	saved, _ := other.get(w.ID)
	v, err := quotaView(saved)
	if err != nil {
		t.Fatal(err)
	}
	if v.Remaining.Planning != 0 || *v.Remaining.Reviews != 0 || v.Authorization.Actor == "" || v.Authorization.Reason != r.Reason {
		t.Fatal(v)
	}
	root, _ := saved.Planning.scope("root")
	claim := PlanningRequest{Schema: 1, EventID: "claim-at-cap", Revision: saved.Revision, Scope: "root", ScopeRevision: root.Revision, Holder: "quota-test", LeaseSeconds: 60}
	if _, err = s.planningChange(w.ID, "claim", claim); err == nil {
		t.Fatal("exhausted activation permitted")
	}
	r.Revision = saved.Revision
	r.EventID = "raise-for-claim"
	r.Limits.Planning = 6
	r.Limits.Decisions = 4
	raised, err := s.configureQuotas(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	claim.Revision = raised.Revision
	claim.EventID = "claim-after-grant"
	claimed, err := s.planningChange(w.ID, "claim", claim)
	if err != nil {
		t.Fatal(err)
	}
	if claimed.Planning.Activations != 5 {
		t.Fatal("activation was not charged")
	}
	// Local activation allocation is still enforced after changing global quotas.
	saved.Planning.Scopes[0].Activations = 5
	if checkScopeActivation(saved.Planning, "root") == nil {
		t.Fatal("local allocation bypassed")
	}
}
func TestQuotasRejectActivePlannerAndInvalidBudgets(t *testing.T) {
	s, w := planningFixture(t)
	r := QuotaChange{Schema: 1, EventID: "q", Revision: w.Revision, Limits: QuotaValues{Planning: 20, Decisions: 20}, Reason: "Budget de recette"}
	for _, n := range []int{0, -1, 201} {
		r.Limits.Planning = n
		if _, err := s.previewQuotas(w.ID, r); err == nil {
			t.Fatal(n)
		}
	}
	r.Limits.Planning = 20
	r.Limits.Reviews = quotaInt(10)
	if _, err := s.configureQuotas(w.ID, r); err == nil {
		t.Fatal("invented reviewer")
	}
	r.Limits.Reviews = nil
	claimed, _ := planningClaim(t, s, w, "root")
	r.Revision = claimed.Revision
	if _, err := s.configureQuotas(w.ID, r); err == nil {
		t.Fatal("live planner quota changed")
	}
}
func TestQuotasConcurrentAuthorization(t *testing.T) {
	s, w := planningFixture(t)
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	results := make(chan error, 2)
	for i, store := range []*Store{s, other} {
		go func(i int, store *Store) {
			_, err := store.configureQuotas(w.ID, QuotaChange{Schema: 1, EventID: []string{"a", "b"}[i], Revision: w.Revision, Limits: QuotaValues{Planning: 20 + i, Decisions: 20}, Reason: "Concurrent fixture"})
			results <- err
		}(i, store)
	}
	success, conflict := 0, 0
	for range 2 {
		err := <-results
		if err == nil {
			success++
		} else if commandFailure(err).Code == "revision_conflict" {
			conflict++
		} else {
			t.Fatal(err)
		}
	}
	if success != 1 || conflict != 1 {
		t.Fatal(success, conflict)
	}
}
