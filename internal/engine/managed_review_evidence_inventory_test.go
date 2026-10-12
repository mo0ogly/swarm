//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestReviewEvidenceInventoryReachesRootAndChildWithoutSourceText(t *testing.T) {
	for _, scope := range []string{"root", "child"} {
		t.Run(scope, func(t *testing.T) {
			s, w := managedFixture(t)
			w.Planning.Scopes = append(w.Planning.Scopes, PlanningScope{ID: "child", Parent: "root"})
			source := ReviewSource{Path: "original.md", Blob: strings.Repeat("a", 40), Content: "original full text\n", Bytes: 19, Digest: hash([]byte("original full text\n"))}
			c := managedReviewContext{Candidate: strings.Repeat("b", 40), Sources: []ReviewSource{source}}
			data, _ := json.Marshal(c)
			name := "review-proof.json"
			if err := os.WriteFile(filepath.Join(s.root, name), data, 0600); err != nil {
				t.Fatal(err)
			}
			r := &IndependentReview{ID: "refusal", Attempt: "attempt", CandidateSHA: c.Candidate, Context: name, ContextDigest: hash(data), State: "changes_requested"}
			w.Tasks[0].ScopeID = scope
			w.Tasks[0].Attempts = []Attempt{{ID: "attempt"}}
			w.Tasks[0].IndependentReview = r
			w.Planning.Inbox = []PlanningEvent{{ID: r.ID, Scope: scope, Kind: "independent_review", Task: w.Tasks[0].ID, Attempt: r.Attempt}}
			payload, delivery, err := s.planningDeliveryContext(w, scope, 64000)
			if err != nil {
				t.Fatal(err)
			}
			var projected struct {
				Inventory      map[string]reviewEvidenceInventory `json:"review_evidence_inventory"`
				SourceContract string                             `json:"review_source_contract"`
			}
			if err := json.Unmarshal(payload, &projected); err != nil {
				t.Fatal(err)
			}
			got, ok := projected.Inventory[w.Tasks[0].ID]
			if !ok || got.Unavailable != "" || got.Review != r.ID || got.Context != r.ContextDigest || len(got.Sources) != 1 || got.Sources[0].Path != source.Path || got.Sources[0].SHA256 != source.Digest || got.Sources[0].Bytes != source.Bytes || strings.Contains(string(payload), source.Content) || projected.SourceContract != reviewSourceBoundary || delivery.SHA256 != hash(payload) {
				t.Fatalf("wrong scope or invented source receipt: %s", payload)
			}
			if err := os.WriteFile(filepath.Join(s.root, name), []byte(`{"sources":[]}`), 0600); err != nil {
				t.Fatal(err)
			}
			payload, _, err = s.planningDeliveryContext(w, scope, 64000)
			if err != nil {
				t.Fatal(err)
			}
			if err := json.Unmarshal(payload, &projected); err != nil {
				t.Fatal(err)
			}
			got = projected.Inventory[w.Tasks[0].ID]
			if got.Unavailable == "" || len(got.Sources) != 0 {
				t.Fatal("tampered context attested", got)
			}
			w.Planning.Inbox[0].ID = "older-refusal"
			projected.Inventory = nil
			payload, _, err = s.planningDeliveryContext(w, scope, 64000)
			if err != nil {
				t.Fatal(err)
			}
			if err := json.Unmarshal(payload, &projected); err != nil {
				t.Fatal(err)
			}
			if len(projected.Inventory) != 0 {
				t.Fatal("latest context attributed to old refusal")
			}
		})
	}
}

func TestReviewEvidenceInventoryReadsWholeAdmittedContext(t *testing.T) {
	for _, scope := range []string{"root", "child"} {
		t.Run(scope, func(t *testing.T) {
			s, w := managedFixture(t)
			w.Planning.Scopes = append(w.Planning.Scopes, PlanningScope{ID: "child", Parent: "root"})
			content := strings.Repeat("complete admitted source\n", 3000)
			c := managedReviewContext{Candidate: strings.Repeat("b", 40)}
			for i := 0; i < 11; i++ {
				c.Sources = append(c.Sources, ReviewSource{Path: "source-" + string(rune('a'+i)) + ".md", Blob: strings.Repeat("a", 40), Content: content, Bytes: len(content), Digest: hash([]byte(content))})
			}
			data, err := json.Marshal(c)
			if err != nil || len(data) <= maxManagedRecoveryBytes {
				t.Fatal("fixture must exceed unrelated recovery handoff limit", len(data), err)
			}
			if err := os.WriteFile(filepath.Join(s.root, "context.json"), data, 0600); err != nil {
				t.Fatal(err)
			}
			r := &IndependentReview{ID: "current-review", ResultEvent: "current-result", State: "unknown", Attempt: "current-attempt", CandidateSHA: c.Candidate, Context: "context.json", ContextDigest: hash(data)}
			w.Tasks[0].ScopeID = scope
			w.Tasks[0].Attempts = []Attempt{{ID: r.Attempt}}
			w.Tasks[0].IndependentReview = r
			w.Planning.Inbox = []PlanningEvent{{ID: r.ResultEvent, Scope: scope, Kind: "independent_review", Task: w.Tasks[0].ID, Attempt: r.Attempt}}
			payload, delivery, err := s.planningDeliveryContext(w, scope, 64000)
			if err != nil {
				t.Fatal(err)
			}
			var got struct {
				Inventory map[string]reviewEvidenceInventory `json:"review_evidence_inventory"`
			}
			if err := json.Unmarshal(payload, &got); err != nil {
				t.Fatal(err)
			}
			inventory := got.Inventory[w.Tasks[0].ID]
			if inventory.Unavailable != "" || len(inventory.Sources) != len(c.Sources) || inventory.Context != hash(data) || delivery.SHA256 != hash(payload) || strings.Contains(string(payload), content) {
				t.Fatal("full admitted originals hidden or source text leaked", inventory)
			}
			if _, err := readManagedRecoveryFile(filepath.Join(s.root, "context.json")); err == nil {
				t.Fatal("unrelated recovery handoff limit was relaxed")
			}
		})
	}
}

func TestReviewEvidenceInventoryRejectsUnattestedOriginals(t *testing.T) {
	for _, mode := range []string{"tampered", "foreign-candidate", "incoherent-source", "extra-json", "trailing-garbage", "non-regular", "missing", "historical"} {
		t.Run(mode, func(t *testing.T) {
			s, _ := managedFixture(t)
			content := "complete original\n"
			c := managedReviewContext{Candidate: strings.Repeat("b", 40), Sources: []ReviewSource{{Path: "original.md", Blob: strings.Repeat("a", 40), Content: content, Bytes: len(content), Digest: hash([]byte(content))}}}
			r := &IndependentReview{ID: "review", CandidateSHA: c.Candidate, Context: "context.json"}
			if mode == "foreign-candidate" {
				c.Candidate = strings.Repeat("c", 40)
			}
			if mode == "incoherent-source" {
				c.Sources[0].Digest = strings.Repeat("0", 64)
			}
			data, err := json.Marshal(c)
			if err != nil {
				t.Fatal(err)
			}
			if mode == "extra-json" {
				data = append(data, []byte(` {"unexpected":true}`)...)
			}
			if mode == "trailing-garbage" {
				data = append(data, []byte(" trailing garbage")...)
			}
			r.ContextDigest = hash(data)
			if mode == "tampered" {
				data = append(data, ' ')
			}
			path := filepath.Join(s.root, r.Context)
			if mode == "non-regular" {
				err = os.Mkdir(path, 0700)
			} else if mode != "missing" && mode != "historical" {
				err = os.WriteFile(path, data, 0600)
			}
			if err != nil {
				t.Fatal(err)
			}
			if mode == "historical" {
				r.Context = ""
				r.ContextDigest = ""
			}
			got := s.reviewEvidenceInventory(r)
			if got.Unavailable == "" || len(got.Sources) != 0 {
				t.Fatal("unattested sources exposed", mode, got)
			}
		})
	}
}

func TestReviewEvidenceInventoryRetainsAcknowledgedCurrentVerdictAsReadOnly(t *testing.T) {
	for _, scope := range []string{"root", "child"} {
		for _, mode := range []string{"unknown", "changes_requested", "error", "foreign-scope", "old-result", "other-attempt", "accepted", "no-fresh-input", "historical-only"} {
			t.Run(scope+"/"+mode, func(t *testing.T) {
				s, w := managedFixture(t)
				w.Planning.Scopes = append(w.Planning.Scopes, PlanningScope{ID: "child", Parent: "root"})
				c := managedReviewContext{Candidate: strings.Repeat("b", 40), Sources: []ReviewSource{{Path: "original.md", Content: "original", Bytes: 8, Digest: hash([]byte("original"))}}}
				data, _ := json.Marshal(c)
				if err := os.WriteFile(filepath.Join(s.root, "context.json"), data, 0600); err != nil {
					t.Fatal(err)
				}
				r := &IndependentReview{ID: "review", ResultEvent: "result", Attempt: "attempt", State: "unknown", Context: "context.json", ContextDigest: hash(data), CandidateSHA: c.Candidate}
				if mode == "changes_requested" || mode == "error" {
					r.State = mode
				}
				task := &w.Tasks[0]
				task.ScopeID = scope
				task.Status = "blocked"
				task.Attempts = []Attempt{{ID: r.Attempt}}
				task.IndependentReview = r
				event := PlanningEvent{ID: r.ResultEvent, Scope: scope, Task: task.ID, Attempt: r.Attempt, Kind: "independent_review", Decision: "original-ack"}
				switch mode {
				case "foreign-scope":
					event.Scope = "elsewhere"
				case "old-result":
					event.ID = "historical-result"
				case "other-attempt":
					event.Attempt = "historical-attempt"
				case "accepted":
					task.Status = "accepted"
				}
				w.Planning.Inbox = []PlanningEvent{event}
				if mode == "historical-only" {
					w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "older-pending", Scope: scope, Task: task.ID, Attempt: "old", Kind: "independent_review"})
				} else if mode != "no-fresh-input" {
					w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "fresh-input", Scope: scope, Kind: "operator_resume"})
				}
				before, _ := json.Marshal(w)
				payload, delivery, err := s.planningDeliveryContext(w, scope, 64000)
				if err != nil {
					t.Fatal(err)
				}
				var got struct {
					Inventory map[string]reviewEvidenceInventory `json:"review_evidence_inventory"`
					Origins   []PlanningEvent                    `json:"acknowledged_current_review_evidence"`
				}
				if err := json.Unmarshal(payload, &got); err != nil {
					t.Fatal(err)
				}
				want := mode == "unknown" || mode == "changes_requested" || mode == "error"
				if (len(got.Inventory) > 0) != want || (len(got.Origins) > 0) != want {
					t.Fatal("acknowledged current evidence hidden or foreign originals reused", mode, got)
				}
				if want && (got.Inventory[task.ID].Unavailable != "" || len(got.Inventory[task.ID].Sources) != 1 || !reflect.DeepEqual(got.Origins, []PlanningEvent{event})) {
					t.Fatal("original attribution changed", got)
				}
				if containsString(delivery.Events, event.ID) || len(delivery.UnresolvedReviews) > 0 || !delivery.ReviewRetryUnavailable {
					t.Fatal("read-only evidence authorized protocol retry or re-acknowledgement", delivery)
				}
				after, _ := json.Marshal(w)
				if string(before) != string(after) {
					t.Fatal("projection mutated original verdict/ack/attempt")
				}
			})
		}
	}
}
