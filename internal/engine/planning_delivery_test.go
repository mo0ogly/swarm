package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestPlanningDeliveryInvalidReportAllowsAcknowledgementOnly(t *testing.T) {
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
			if err := s.applyPlanning(&w, "claim", PlanningRequest{Scope: "root", ScopeRevision: 1, Holder: "owner", LeaseSeconds: 60}, time.Now()); err != nil {
				t.Fatal(err)
			}
			delivery := w.Planning.Scopes[0].Delivery
			if delivery.Unavailable["handoff"] == "" || len(delivery.Reports) != 0 || len(delivery.Events) != 1 {
				t.Fatalf("invalid bytes supplied as evidence: %+v", delivery)
			}
			r := PlanningRequest{EventID: "ack", Scope: "root", ScopeRevision: 1, Holder: "owner", Generation: w.Planning.Scopes[0].Generation, Inputs: []string{"handoff"}, Reason: "Unavailable report observed, no result validated", Operations: []PlanningOperation{planningTask("unsafe")}}
			if err := s.applyPlanning(&w, "decide", r, time.Now()); err == nil {
				t.Fatal("operation authorized from unavailable report")
			}
			if len(w.Tasks) != 0 || w.Planning.Inbox[0].Decision != "" {
				t.Fatal("rejected decision changed work")
			}
			r.Operations = nil
			if err := s.applyPlanning(&w, "decide", r, time.Now()); err != nil {
				t.Fatal(err)
			}
			if w.Planning.Inbox[0].Decision != "ack" || len(w.Tasks) != 0 {
				t.Fatal("acknowledgement lost or production invented")
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

func TestPlanningDeliverySingleValidReportExceedsRemainingContextBudget(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	if e := s.applyPlanning(&w, "enable", PlanningRequest{EventID: "enable", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 10}, time.Now()); e != nil {
		t.Fatal(e)
	}
	text := strings.Repeat("complete-report-with-no-truncation ", 1200)
	ref := ExchangeArtifact{Path: "large-valid.md", SHA256: hash([]byte(text))}
	if e := os.WriteFile(filepath.Join(s.root, ref.Path), []byte(text), 0600); e != nil {
		t.Fatal(e)
	}
	w.Planning.Inbox = []PlanningEvent{{ID: "handoff", Scope: "root", Kind: "handoff", Handoff: &ref}}
	raw, d, e := s.planningDeliveryContext(w, "root", 10000)
	if e != nil {
		t.Fatal(e)
	}
	if len(raw) > 10000 || len(d.Reports) != 0 || d.Unavailable["handoff"] == "" || len(d.Events) != 1 || strings.Contains(string(raw), "complete-report-with-no-truncation") {
		t.Fatal("oversize report hidden, truncated or emitted as proof", string(raw))
	}
	var ctx struct {
		Reports []planningReportContent `json:"handoff_contents"`
	}
	if e = json.Unmarshal(raw, &ctx); e != nil || len(ctx.Reports) != 0 {
		t.Fatal(e)
	}
	if w.Planning.Inbox[0].Decision != "" {
		t.Fatal("diagnostic silently consumed event")
	}
	if e = s.applyPlanning(&w, "claim", PlanningRequest{Scope: "root", ScopeRevision: 1, Holder: "owner", LeaseSeconds: 60}, time.Now()); e != nil {
		t.Fatal(e)
	}
	w.Planning.Scopes[0].Delivery = &d
	r := PlanningRequest{EventID: "ack-large", Scope: "root", ScopeRevision: 1, Holder: "owner", Generation: w.Planning.Scopes[0].Generation, Inputs: d.Events, Reason: "Report is unavailable within this activation; acknowledge diagnostic only", Operations: []PlanningOperation{planningTask("unsafe")}}
	if e = s.applyPlanning(&w, "decide", r, time.Now()); e == nil {
		t.Fatal("diagnostic authorized production")
	}
	r.Operations = nil
	if e = s.applyPlanning(&w, "decide", r, time.Now()); e != nil {
		t.Fatal(e)
	}
	b, e := os.ReadFile(filepath.Join(s.root, ref.Path))
	if e != nil || string(b) != text {
		t.Fatal("historical report changed")
	}
}
