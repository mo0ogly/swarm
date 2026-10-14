//go:build linux

package engine

import (
	"archive/zip"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestPlanningJudgeImportValidatesWithoutPartialMutation(t *testing.T) {
	for _, kind := range []string{"initial-unlimited", "legacy-no-judge", "decided-continue", "renewed", "zero-cycle", "negative-limit", "closed-without-end", "history-tampered"} {
		t.Run(kind, func(t *testing.T) {
			s, w := planningFixture(t)
			if kind == "decided-continue" || kind == "renewed" || kind == "history-tampered" {
				var err error
				w, err = judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "continue", Reason: "Import an actual attributed cycle verdict"})
				if err != nil {
					t.Fatal(err)
				}
				if kind == "renewed" {
					w, err = judgeDo(t, s, w, "renew", 1, PlanningRequest{})
					if err != nil {
						t.Fatal(err)
					}
				}
			}
			rejected := false
			switch kind {
			case "legacy-no-judge":
				w.Planning.Judge = nil
			case "zero-cycle":
				w.Planning.Judge.Cycle = 0
				rejected = true
			case "negative-limit":
				w.Planning.Judge.MaxCycles = -1
				rejected = true
			case "closed-without-end":
				w.Planning.Judge.Closed = true
				rejected = true
			case "history-tampered":
				w.Planning.Judge.History[0].Reason = "Changed historical verdict"
				rejected = true
			}
			events, err := s.events(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			raw, err := json.Marshal(Bundle{Schema: 2, Work: w, Events: events, Files: map[string]string{}})
			if err != nil {
				t.Fatal(err)
			}
			name := filepath.Join(t.TempDir(), "judge.zip")
			file, err := os.Create(name)
			if err != nil {
				t.Fatal(err)
			}
			writer := zip.NewWriter(file)
			entry, err := writer.Create("manifest.json")
			if err != nil {
				t.Fatal(err)
			}
			if _, err = entry.Write(raw); err != nil {
				t.Fatal(err)
			}
			if err = writer.Close(); err != nil {
				t.Fatal(err)
			}
			if err = file.Close(); err != nil {
				t.Fatal(err)
			}
			other := storeTest(t)
			imported, err := other.importBundle(name)
			if rejected {
				if err == nil {
					t.Fatal("invalid durable judge imported")
				}
				works, e := other.list()
				if e != nil || len(works) != 0 {
					t.Fatal("partial import retained after rejection", e)
				}
			} else if err != nil || imported.ID != w.ID {
				t.Fatal("valid/legacy durable judge import rejected", err)
			}
		})
	}
}

func TestJudgeStateHistoryChecksAreDiscriminating(t *testing.T) {
	s, w := planningFixture(t)
	for cycle := 1; cycle < 3; cycle++ {
		var err error
		w, err = judgeDo(t, s, w, "judge", cycle, PlanningRequest{JudgeVerdict: "continue", Reason: "Produce actual cycle history"})
		if err != nil {
			t.Fatal(err)
		}
		w, err = judgeDo(t, s, w, "renew", cycle, PlanningRequest{})
		if err != nil {
			t.Fatal(err)
		}
	}
	raw, _ := json.Marshal(w.Planning.Judge)
	for _, kind := range []string{"order", "renewal", "timestamp", "quota-below-consumed", "unexpected-live-context"} {
		t.Run(kind, func(t *testing.T) {
			var j JudgeState
			if err := json.Unmarshal(raw, &j); err != nil {
				t.Fatal(err)
			}
			if err := validateJudgeState(&j); err != nil {
				t.Fatal("valid producer state rejected", err)
			}
			switch kind {
			case "order":
				j.History[0], j.History[1] = j.History[1], j.History[0]
			case "renewal":
				j.History[0].RenewedAt = ""
			case "timestamp":
				j.History[0].At = "not-a-time"
			case "quota-below-consumed":
				j.MaxCycles = j.Cycle - 1
			case "unexpected-live-context":
				j.Reason = "Stale live context"
			}
			if err := validateJudgeState(&j); err == nil {
				t.Fatal("single-field invariant not enforced", kind)
			}
		})
	}
	// Matching closure/history is a representation invariant, not acceptance.
	owner, closed := judgeAcceptedFixture(t)
	closed, r := planningClaim(t, owner, closed, "root")
	r.Operations = []PlanningOperation{{Kind: "judge", JudgeCycle: 1, JudgeVerdict: "end", ExpectedCandidate: closed.Planning.Repository.Candidate}}
	var err error
	closed, err = owner.planningChange(closed.ID, "decide", r)
	if err != nil {
		t.Fatal(err)
	}
	if err = validateJudgeState(closed.Planning.Judge); err != nil {
		t.Fatal("valid closure producer rejected", err)
	}
	for _, kind := range []string{"closed-flag", "live-mirror"} {
		t.Run(kind, func(t *testing.T) {
			raw, _ := json.Marshal(closed.Planning.Judge)
			var j JudgeState
			json.Unmarshal(raw, &j)
			if kind == "closed-flag" {
				j.Closed = false
			} else {
				j.Reason = "Changed current explanation"
			}
			if err := validateJudgeState(&j); err == nil {
				t.Fatal("closed state mismatch admitted", kind)
			}
		})
	}
}
