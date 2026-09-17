//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPreparationBudgetReserveReplayCancelAndSpend(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	req := prepRequest(p, "budget")
	req.Budget = &Budget{Limit: 1, Reserve: 1, Source: "test forfait", PriceDate: "2026-09-16"}
	p, e := s.preparationCommand(req)
	if e != nil {
		t.Fatal(e)
	}
	r.Revision = p.Revision
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.sendPreparation(r); e != nil {
		t.Fatal(e)
	}
	view, e := s.preparationBudget(p.ID)
	if e != nil || view.Reserved != 1 {
		t.Fatal(view, e)
	}
	if _, e = s.cancelPreparation(p.ID, turn.ID); e != nil {
		t.Fatal(e)
	}
	view, e = s.preparationBudget(p.ID)
	if e != nil || view.Reserved != 0 || view.Estimated != 0 {
		t.Fatal(view, e)
	}
	r.Event = "after-cancel"
	turn, e = s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	s.runPreparationTurn(turn)
	view, e = s.preparationBudget(p.ID)
	if e != nil || view.Reserved != 0 || view.Estimated != 1 || view.ActualCost != nil {
		t.Fatal(view, e)
	}
	r.Event = "exhausted"
	if _, e = s.sendPreparation(r); e == nil {
		t.Fatal("budget exceeded")
	}
}

func TestPreparationSharedBudgetBlocksAgentAndOtherPreparation(t *testing.T) {
	s, _, r := prepDialogueFixture(t, prepReplyScript)
	// setupAgent installs a fixture provider; preserve the preparation provider too.
	ps, _ := s.providers()
	w, launch := setupAgent(t, s)
	merged, _ := s.providers()
	merged.Providers["test"] = ps.Providers["test"]
	raw, _ := json.Marshal(merged)
	os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600)
	if e := s.setBudget(w.ID, Budget{Limit: 1, Reserve: 1, Source: "Forfait de recette", PriceDate: "2026-09-16"}); e != nil {
		t.Fatal(e)
	}
	create := prepRequest(Preparation{}, "create")
	create.WorkID = w.ID
	p, e := s.preparationCommand(create)
	if e != nil {
		t.Fatal(e)
	}
	r.ID = p.ID
	r.Revision = p.Revision
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	if _, _, e = s.prepare(w.ID, launch); e == nil {
		t.Fatal("agent ignored preparation reservation")
	}
	create = prepRequest(Preparation{}, "create")
	create.WorkID = w.ID
	p2, e := s.preparationCommand(create)
	if e != nil {
		t.Fatal(e)
	}
	r.ID = p2.ID
	r.Revision = p2.Revision
	r.Event = "other-preparation"
	if _, e = s.sendPreparation(r); e == nil {
		t.Fatal("second preparation exceeded shared envelope")
	}
	if _, e = s.cancelPreparation(p.ID, turn.ID); e != nil {
		t.Fatal(e)
	}
	if _, e = s.sendPreparation(r); e != nil {
		t.Fatal("cancel did not release shared budget", e)
	}
}

func TestPreparationDisabledPreservesReadExport(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	t.Setenv("SWARM_PREPARATION_DISABLED", "1")
	req := prepRequest(p, "save")
	req.Document = "besoin"
	req.Text = "ne doit pas remplacer"
	if _, e := s.preparationCommand(req); e == nil {
		t.Fatal("write allowed during fallback")
	}
	if _, e := s.sendPreparation(r); e == nil {
		t.Fatal("provider allowed during fallback")
	}
	current, e := s.preparation(p.ID)
	if e != nil || current.Revision != p.Revision {
		t.Fatal(current, e)
	}
	if e = exportPreparation(current, filepath.Join(t.TempDir(), "export")); e != nil {
		t.Fatal(e)
	}
}

func TestPreparationDiskFullKeepsConfirmedRevision(t *testing.T) {
	s, _, _ := prepDialogueFixture(t, prepReplyScript)
	// SQLite enforces the actual page allocation limit (SQLITE_FULL), without
	// filling the developer's filesystem or touching another application's disk.
	var pages int
	if e := s.db.QueryRow("PRAGMA page_count").Scan(&pages); e != nil {
		t.Fatal(e)
	}
	if _, e := s.db.Exec(fmt.Sprintf("PRAGMA max_page_count=%d", pages)); e != nil {
		t.Fatal(e)
	}
	p := prepCreate(t, s)
	initial := p.Revision
	failed := false
	for i := 0; i < 40; i++ {
		req := prepRequest(p, "save")
		req.Document = "besoin"
		req.Text = strings.Repeat(fmt.Sprintf("ligne-%03d ", i), 1500)
		next, e := s.preparationCommand(req)
		if e != nil {
			if !strings.Contains(e.Error(), "full") {
				t.Fatal("not a disk allocation failure", e)
			}
			failed = true
			break
		}
		p = next
	}
	if !failed {
		t.Fatal("allocation limit did not produce failure")
	}
	got, e := s.preparation(p.ID)
	if e != nil || got.Revision != p.Revision || got.Revision < initial {
		t.Fatal(got, e)
	}
	var integrity string
	if e = s.db.QueryRow("PRAGMA integrity_check").Scan(&integrity); e != nil || integrity != "ok" {
		t.Fatal(integrity, e)
	}
}
