//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The reference file is unchanged by the producer. A path or copied quotation
// in a handoff cannot substitute for the actual candidate blob in the context.
func TestManagedDeliveryEvidenceJoinsUnchangedOriginalWithoutManifest(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "initial\n")
	a.DeliveryVersion = 1
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	d := completeDelivery(task, a)
	d.Criteria[0].Evidence = []string{"value.txt", "docs/first.md", "value.txt"}
	raw, _ := json.Marshal(d)
	if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	managedReviewMode(t, s, "unknown")
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(filepath.Join(s.root, "review-fixture/observed.json"))
	if err != nil {
		t.Fatal(err)
	}
	var observed struct {
		Context managedReviewContext `json:"context"`
	}
	if err := json.Unmarshal(raw, &observed); err != nil {
		t.Fatal(err)
	}
	sources := observed.Context.Sources
	if len(sources) != 1 || sources[0].Path != "value.txt" || sources[0].Content != "initial\n" || sources[0].Digest != hash([]byte("initial\n")) || len(sources[0].Blob) != 40 {
		t.Fatalf("unchanged declared evidence not supplied completely: %+v", sources)
	}
	if err := os.WriteFile(filepath.Join(a.CWD, "value.txt"), []byte("mutated live copy"), 0600); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	again, err := managedReviewSourcesByTask(w, observed.Context.Tasks, observed.Context.Candidate)
	if err != nil || len(again) != 1 || again[0] != sources[0] {
		t.Fatalf("evidence followed mutable producer: %+v %v", again, err)
	}
	handoff, err := s.managedRecoveryHandoff(w, &w.Tasks[0])
	if err != nil || handoff == nil || handoff.ReviewEvidence == nil || len(handoff.ReviewEvidence.Sources) != 1 || handoff.ReviewEvidence.Sources[0].Path != "value.txt" {
		t.Fatalf("recovery lost supplied evidence inventory: %+v %v", handoff, err)
	}
	owners, err := managedReviewSourceOwners(w, observed.Context)
	if err != nil || len(owners["first"]) != 1 || owners["first"][0] != "value.txt" {
		t.Fatalf("evidence lost in batched ownership: %+v %v", owners, err)
	}
	if w.Tasks[0].Status == "accepted" || len(w.Tasks[0].Attempts) != 1 || managedReviewCalls(t, s) != 1 {
		t.Fatal("source attachment bypassed review or changed attempts")
	}
}

func TestManagedDeliveryEvidenceInvalidSourcesStopBeforePaidReview(t *testing.T) {
	for _, mode := range []string{"oversize", "non-text", "overflow"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			a := managedCompleted(t, s, w, "first", "candidate\n")
			a.DeliveryVersion = 1
			if err := s.saveAgent(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task(a.TaskID)
			d := completeDelivery(task, a)
			text := []byte(strings.Repeat("a", 96*1024+1))
			if mode == "non-text" {
				text = []byte{0, 1, 2}
			}
			if mode == "overflow" {
				text = []byte(strings.Repeat("a", 70*1024))
			}
			if err := os.WriteFile(filepath.Join(a.CWD, "evidence.txt"), text, 0600); err != nil {
				t.Fatal(err)
			}
			d.Criteria[0].Evidence = []string{"evidence.txt"}
			if mode == "overflow" {
				if err := os.WriteFile(filepath.Join(a.CWD, "other.txt"), text, 0600); err != nil {
					t.Fatal(err)
				}
				d.Criteria[0].Evidence = append(d.Criteria[0].Evidence, "other.txt")
			}
			raw, _ := json.Marshal(d)
			if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), raw, 0600); err != nil {
				t.Fatal(err)
			}
			if err := s.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			after, _ := s.get(w.ID)
			if after.Tasks[0].Status != "blocked" || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate || managedReviewCalls(t, s) != 0 || after.Planning.Reviewer.Calls != 0 || len(after.Tasks[0].Attempts) != 1 {
				t.Fatal("invalid evidence reached reviewer/publication or spent another attempt")
			}
			if !strings.Contains(after.Tasks[0].Blocker, "evidence.txt") && mode == "oversize" {
				t.Fatal("source diagnosis lost", after.Tasks[0].Blocker)
			}
		})
	}
}

func TestManagedDeliveryEvidenceAndExplicitSourcesKeepTaskOwnership(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	if err := os.MkdirAll(filepath.Join(dir, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"one", "two", "extra"} {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(name+"\n"), 0600); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.WriteFile(filepath.Join(dir, "docs/first.review-context.json"), []byte(`{"version":1,"files":["extra","one"]}`), 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "immutable proof fixture")
	storage := t.TempDir()
	gitTest(t, storage, "clone", "--bare", dir, "repository.git")
	w := Work{Planning: &PlanningState{Repository: &ManagedRepository{Storage: storage}}}
	tasks := []managedReviewTaskContext{
		{Task: "first", Delivery: &ManagedDelivery{Criteria: []DeliveryCriterion{{Evidence: []string{"one"}}}}},
		{Task: "second", Delivery: &ManagedDelivery{Criteria: []DeliveryCriterion{{Evidence: []string{"two", "one"}}}}},
	}
	candidate := gitTest(t, dir, "rev-parse", "HEAD")
	sources, err := managedReviewSourcesByTask(w, tasks, candidate)
	if err != nil || len(sources) != 3 {
		t.Fatalf("sources lost or duplicated: %+v %v", sources, err)
	}
	owners, err := managedReviewSourceOwners(w, managedReviewContext{Candidate: candidate, Tasks: tasks, Sources: sources})
	if err != nil || len(owners["first"]) != 2 || len(owners["second"]) != 2 || owners["first"][0] != "extra" || owners["second"][0] != "two" {
		t.Fatalf("foreign or missing task evidence: %+v %v", owners, err)
	}
}
