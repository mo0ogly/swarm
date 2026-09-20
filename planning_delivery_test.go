package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestPlanningDeliveryRejectsInvalidReportBeforeClaim(t *testing.T) {
	for _, mode := range []string{"missing", "changed", "outside", "oversize", "binary"} {
		t.Run(mode, func(t *testing.T) {
			s := storeTest(t)
			w := createTest(t, s)
			if err := s.applyPlanning(&w, "enable", PlanningRequest{EventID: "enable", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 10}, time.Now()); err != nil {
				t.Fatal(err)
			}
			body := []byte("proof")
			ref := ExchangeArtifact{Path: "report.md", SHA256: hash(body)}
			switch mode {
			case "changed":
				body = []byte("changed")
			case "outside":
				ref.Path = filepath.Join(t.TempDir(), "report.md")
			case "oversize":
				body = []byte(strings.Repeat("x", 64001))
				ref.SHA256 = hash(body)
			case "binary":
				body = []byte{0xff, 0, 1}
				ref.SHA256 = hash(body)
			}
			if mode != "missing" {
				if err := os.WriteFile(filepath.Join(s.root, "report.md"), body, 0600); err != nil {
					t.Fatal(err)
				}
			}
			w.Planning.Inbox = []PlanningEvent{{ID: "handoff", Scope: "root", Kind: "handoff", Handoff: &ref}}
			if err := s.applyPlanning(&w, "claim", PlanningRequest{Scope: "root", ScopeRevision: 1, Holder: "owner", LeaseSeconds: 60}, time.Now()); err == nil {
				t.Fatal("invalid handoff admitted")
			}
			if w.Planning.Activations != 0 || w.Planning.Scopes[0].Holder != "" {
				t.Fatal("invalid report consumed a planning activation")
			}
		})
	}
}

func TestPlanningDeliveryBatchesWholeReportsAndRejectsUnseenInput(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	if err := s.applyPlanning(&w, "enable", PlanningRequest{EventID: "enable", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 10}, time.Now()); err != nil {
		t.Fatal(err)
	}
	w.Planning.Inbox = nil
	for i := 0; i < 3; i++ {
		text := strings.Repeat(fmt.Sprintf("report-%d ", i), 1700)
		ref := ExchangeArtifact{Path: fmt.Sprintf("report-%d.md", i), SHA256: hash([]byte(text))}
		if err := os.WriteFile(filepath.Join(s.root, ref.Path), []byte(text), 0600); err != nil {
			t.Fatal(err)
		}
		w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: fmt.Sprintf("event-%d", i), Scope: "root", Kind: "handoff", Handoff: &ref, Artifacts: []ExchangeArtifact{ref}})
	}
	if err := s.applyPlanning(&w, "claim", PlanningRequest{Scope: "root", ScopeRevision: 1, Holder: "owner", LeaseSeconds: 60}, time.Now()); err != nil {
		t.Fatal(err)
	}
	scope := &w.Planning.Scopes[0]
	if scope.Delivery == nil || len(scope.Delivery.Events) == 0 || len(scope.Delivery.Events) >= 3 {
		t.Fatalf("report batch was not bounded: %+v", scope.Delivery)
	}
	r := PlanningRequest{Scope: "root", ScopeRevision: 1, Holder: "owner", Generation: scope.Generation, Inputs: []string{"event-2"}, Reason: "Prétendre avoir traité un événement non fourni"}
	if err := s.applyPlanning(&w, "decide", r, time.Now()); err == nil {
		t.Fatal("unseen event acknowledged")
	}
	for _, event := range w.Planning.Inbox {
		if event.Decision != "" {
			t.Fatal("pending event lost after a rejected decision")
		}
	}
}
