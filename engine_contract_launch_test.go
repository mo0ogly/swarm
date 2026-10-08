//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// A passing delivery gate cannot substitute for the independent review, even
// through the generic task mutation API or a legacy reviewer_required=false.
// These are deterministic engine tests, not observations of an AI reviewer.
func TestEngineContractLaunchAcceptanceCannotBypassReview(t *testing.T) {
	for _, condition := range []string{"missing-config-legacy", "missing-verdict", "review-error", "review-refused"} {
		for _, entry := range []string{"web", "cli", "console"} {
			t.Run(condition+"/"+entry, func(t *testing.T) {
				s := storeTest(t)
				w := taskTest(t, s, createTest(t, s))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
				w = gateTest(t, s, w, fixture(t, s.root))
				organizedFixtureStore(t, s)
				w, err := s.get(w.ID)
				if err != nil {
					t.Fatal(err)
				}
				switch condition {
				case "missing-config-legacy":
					w.Planning.Reviewer = nil
					w.Planning.ReviewerRequired = false
				case "review-error":
					w.Tasks[0].IndependentReview = &IndependentReview{State: "error"}
				case "review-refused":
					w.Tasks[0].IndependentReview = &IndependentReview{State: "changes_requested"}
				}
				// Fixture-only historical/corrupt state; no production mutation.
				raw, _ := json.Marshal(w)
				if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
					t.Fatal(err)
				}
				if !s.validGate(&w.Tasks[0]) {
					t.Fatal("precondition: delivery gate must pass independently of review")
				}
				r := Request{Schema: 1, EventID: newID("refused-accept-"), Revision: w.Revision, ID: "t1", Status: "accepted"}
				var diagnostic string
				switch entry {
				case "web":
					_, err = s.webAction(webRequest{Kind: "task", Work: w.ID, Task: "t1", Event: r.EventID, Revision: w.Revision, Request: r})
				case "console":
					err = s.acceptReviewedTask(w.ID, "t1")
				case "cli":
					input := filepath.Join(t.TempDir(), "accept.json")
					raw, _ = json.Marshal(r)
					if err = os.WriteFile(input, raw, 0600); err != nil {
						t.Fatal(err)
					}
					var out, errors bytes.Buffer
					if code := run([]string{"--root", s.root, "--json", "task", "update", w.ID, "--input", input}, &out, &errors); code == 0 {
						t.Fatal("CLI accepted an unreviewed result", out.String())
					}
					diagnostic = out.String() + errors.String()
				}
				if entry != "cli" {
					if err == nil {
						t.Fatal("accepted a passing gate without favorable independent review")
					}
					diagnostic = err.Error()
				}
				if !strings.Contains(diagnostic, "indépendant") {
					t.Fatalf("refusal did not identify missing independent review: %s", diagnostic)
				}
				if err = s.db.Close(); err != nil {
					t.Fatal(err)
				}
				reopened, err := openStore(s.root, false)
				if err != nil {
					t.Fatal(err)
				}
				defer reopened.db.Close()
				after, err := reopened.get(w.ID)
				if err != nil || after.Revision != w.Revision || after.Tasks[0].Status != "submitted" || len(after.Tasks[0].Attempts) != 1 {
					t.Fatalf("refusal changed durable state: %+v %v", after, err)
				}
				if reopened.acceptedFresh(&after, &after.Tasks[0], map[string]bool{}) {
					t.Fatal("refused result became an accepted dependency after restart")
				}
			})
		}
	}
}
