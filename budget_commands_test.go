package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestBudgetPublicCommands(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	r := BudgetChange{Schema: 1, EventID: "budget-one", Revision: w.Revision, Budget: Budget{Limit: 10, Reserve: 2, Source: "fixture", PriceDate: "2026-09-23"}}
	input := filepath.Join(t.TempDir(), "budget.json")
	raw, _ := json.Marshal(r)
	if err := os.WriteFile(input, raw, 0600); err != nil {
		t.Fatal(err)
	}
	var out bytes.Buffer
	if err := s.budgetCLI([]string{"budget", "preview", w.ID}, input, &out); err != nil {
		t.Fatal(err)
	}
	var preview BudgetPreview
	if err := json.Unmarshal(out.Bytes(), &preview); err != nil {
		t.Fatal(err)
	}
	if preview.Current.Budget.Limit != 0 || preview.Proposed.Limit != 10 || !preview.NewLaunchAllowed {
		t.Fatal(preview)
	}
	unchanged, _ := s.get(w.ID)
	if unchanged.Revision != w.Revision {
		t.Fatal("preview mutated")
	}
	out.Reset()
	if err := s.budgetCLI([]string{"budget", "apply", w.ID}, input, &out); err != nil {
		t.Fatal(err)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision+1 {
		t.Fatal(current.Revision)
	}
	// HTTP exact replay of CLI request is idempotent.
	if _, err := s.webAction(webRequest{Kind: "budget", Work: w.ID, Event: r.EventID, Revision: r.Revision, Budget: r.Budget}); err != nil {
		t.Fatal(err)
	}
	replay, _ := s.get(w.ID)
	if replay.Revision != current.Revision {
		t.Fatal("replay mutated")
	}
	r.EventID = "stale"
	if _, err := s.configureBudget(w.ID, r); err == nil || commandFailure(err).Code != "revision_conflict" {
		t.Fatal("stale edit", err)
	}
	// Lowering a ceiling retains commitments; restoring it does not refund them.
	if _, err := s.db.Exec("INSERT INTO planning_calls(id,work_id,scope_id,estimate,state,created_at) VALUES(?,?,'root',?,'finished','2026-09-23')", "fixture-reservation", w.ID, 4); err != nil {
		t.Fatal(err)
	}
	r.Revision = current.Revision
	r.Budget.Limit = 3
	r.EventID = "lower"
	if _, err := s.webAction(webRequest{Kind: "budget", Work: w.ID, Event: r.EventID, Revision: r.Revision, Budget: r.Budget}); err != nil {
		t.Fatal(err)
	}
	current, _ = s.get(w.ID)
	r.Revision = current.Revision
	r.EventID = "raise"
	r.Budget.Limit = 10
	if _, err := s.configureBudget(w.ID, r); err != nil {
		t.Fatal(err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	b, err := other.budget(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if b.Estimated != 4 || b.Remaining != 6 || b.Budget.Actor == "" || b.Budget.Updated == "" {
		t.Fatal(b)
	}
	r.Budget.Limit = -1
	current, _ = s.get(w.ID)
	r.Revision = current.Revision
	r.EventID = "invalid"
	if _, err := s.configureBudget(w.ID, r); err == nil {
		t.Fatal("negative accepted")
	}
	after, _ := s.get(w.ID)
	if after.Revision != current.Revision {
		t.Fatal("failed write mutated")
	}
}

func TestBudgetConcurrentAuthorization(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	results := make(chan error, 2)
	for i, store := range []*Store{s, other} {
		go func(i int, store *Store) {
			_, err := store.configureBudget(w.ID, BudgetChange{Schema: 1, EventID: []string{"first", "second"}[i], Revision: w.Revision, Budget: Budget{Limit: float64(10 + i), Reserve: 1, Source: "fixture", PriceDate: "2026-09-23"}})
			results <- err
		}(i, store)
	}
	success, conflicts := 0, 0
	for range 2 {
		err := <-results
		if err == nil {
			success++
		} else if commandFailure(err).Code == "revision_conflict" {
			conflicts++
		} else {
			t.Fatal(err)
		}
	}
	if success != 1 || conflicts != 1 {
		t.Fatal(success, conflicts)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision+1 {
		t.Fatal("lost update")
	}
}
