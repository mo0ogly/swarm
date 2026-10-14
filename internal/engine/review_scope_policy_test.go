//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestReviewScopePolicyPublicQuotaContract(t *testing.T) {
	s, w := planningFixture(t)
	w, err := s.mutate(w.ID, "test.reviewer-policy", "review-policy-setup", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Planning.Reviewer = &ReviewerConfig{Provider: "fixture", MaxCalls: 10, Calls: 2}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	view, err := quotaView(w)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(view)
	var public map[string]any
	json.Unmarshal(raw, &public)
	limits := public["limits"].(map[string]any)
	for key, value := range map[string]float64{"review_context_files": 24, "review_source_bytes": 98304, "review_context_bytes": 131072} {
		if limits[key] != value {
			t.Fatalf("source policy absent from Admin: %s", key)
		}
	}
	if limits["review_delivery_files"] != float64(100) {
		t.Fatalf("Admin cannot inspect the effective legacy file policy: %s", raw)
	}
}

func TestConfiguredReviewFilePolicyPreservesCheckpointAndAllowsExplicitRecovery(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "repaired\n")
	a.DeliveryVersion = 1
	a.Ended = now()
	task, _ := w.task(a.TaskID)
	delivery, _ := json.Marshal(completeDelivery(task, a))
	if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), delivery, 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 101; i++ {
		if err := os.WriteFile(filepath.Join(a.CWD, fmt.Sprintf("extra-%03d.txt", i)), []byte("original\n"), 0600); err != nil {
			t.Fatal(err)
		}
	}
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ = w.task(a.TaskID)
	if task.Status != "blocked" || managedReviewCalls(t, s) != 0 {
		t.Fatal("legacy policy not enforced before paid review", task.Status)
	}
	checkpoint, _ := s.managedAttempt(a.ID)
	beforeAttempts := len(task.Attempts)
	limits, _ := quotaView(w)
	apply := QuotaChange{Schema: 1, EventID: "files-300", Revision: w.Revision, Limits: limits.Limits, Reason: "Expand explicit file policy while retaining the original checkpoint"}
	apply.Limits.ReviewFiles = quotaInt(300)
	raw, _ := json.Marshal(apply)
	request := filepath.Join(t.TempDir(), "quota.json")
	os.WriteFile(request, raw, 0600)
	var output bytes.Buffer
	if err := s.quotasCLI([]string{"quotas", "preview", w.ID}, request, &output); err != nil {
		t.Fatal(err)
	}
	previewed, _ := s.get(w.ID)
	if !reflect.DeepEqual(w, previewed) {
		t.Fatal("quota preview mutated work")
	}
	changed, err := s.configureQuotas(w.ID, apply)
	if err != nil {
		t.Fatal(err)
	}
	p, err := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
	if err != nil {
		t.Fatal(err)
	}
	if p.Limit != 300 || p.RequiresReduction || len(p.Files) <= 100 {
		t.Fatal("hidden old ceiling remained", p.Limit, p.RequiresReduction, len(p.Files))
	}
	if _, err = s.prepareManagedPreflightRetry(w.ID, a.TaskID); err != nil {
		t.Fatal("new configured policy not used by actual recovery consumer", err)
	}
	same, _ := s.managedAttempt(a.ID)
	currentTask, _ := changed.task(a.TaskID)
	if !reflect.DeepEqual(checkpoint, same) || len(currentTask.Attempts) != beforeAttempts || managedReviewCalls(t, s) != 0 || currentTask.Status != "blocked" {
		t.Fatal("policy application fabricated recovery/consumption")
	}
	apply.Revision = changed.Revision
	apply.EventID = "files-unlimited"
	apply.Limits.ReviewFiles = quotaInt(0)
	changed, err = s.configureQuotas(w.ID, apply)
	if err != nil {
		t.Fatal(err)
	}
	p, err = s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
	if err != nil || p.Limit != 0 || p.RequiresReduction {
		t.Fatal("unlimited file policy not applied", p.Limit, err)
	}
	selection, err := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID, Revision: changed.Revision, ExpectedCandidate: p.Candidate, ScopeFiles: p.Files}, true)
	if err != nil || len(selection.Selected) != len(p.Files) || len(selection.Deferred) != 0 {
		t.Fatal("scope patch silently limited to100", err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	saved, err := other.get(w.ID)
	if err != nil || effectiveReviewFileLimit(saved.Planning.Reviewer) != 0 {
		t.Fatal("unlimited policy lost on reopen", err)
	}
	beforeRejected, _ := s.get(w.ID)
	for _, invalid := range []struct {
		id         string
		rev, files int
	}{{"negative", changed.Revision, -1}, {"stale", changed.Revision - 1, 300}} {
		bad := apply
		bad.Revision = invalid.rev
		bad.EventID = invalid.id
		bad.Limits.ReviewFiles = quotaInt(invalid.files)
		if _, err = s.configureQuotas(w.ID, bad); err == nil {
			t.Fatal("invalid policy accepted", invalid.id)
		}
	}
	after, _ := s.get(w.ID)
	if !reflect.DeepEqual(after, beforeRejected) {
		t.Fatal("rejected config mutated persisted work")
	}
}

func TestReviewLotPolicyUsesSameLimitWithoutTransportRecapping(t *testing.T) {
	files := []string{}
	for i := 0; i < 101; i++ {
		files = append(files, fmt.Sprintf("file-%d", i))
	}
	p := ReviewCoordinationProposal{Candidate: "candidate", Evidence: "proof", FinalReview: "Review all interactions and global exact-candidate checks", Lots: []ReviewCoordinationLot{{ID: "all", Kind: "component", Objective: "Inspect every retained source", Files: files, Criteria: []string{"task#1"}}}}
	if _, err := validateReviewCoordination(p, "candidate", "proof", files, []string{"task#1"}); err != nil {
		t.Fatal("transport coverage still applies an implicit file cap", err)
	}
	if err := validateReviewLotFilePolicy(p, 100); err == nil {
		t.Fatal("configured limit ignored")
	}
	for _, limit := range []int{101, 300, 0} {
		if err := validateReviewLotFilePolicy(p, limit); err != nil {
			t.Fatal("configured/unlimited lot policy refused", limit, err)
		}
	}
}

func TestReviewSourcePolicySeparatesValidManifestFromCapacityAndPreservesAllBytes(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	os.MkdirAll(filepath.Join(dir, "docs"), 0700)
	files := []string{}
	for i := 0; i < 28; i++ {
		name := fmt.Sprintf("source-%02d", i)
		files = append(files, name)
		data := []byte("original\n")
		if i >= 26 {
			data = bytes.Repeat([]byte("a"), 70*1024)
		}
		if err := os.WriteFile(filepath.Join(dir, name), data, 0600); err != nil {
			t.Fatal(err)
		}
	}
	manifest, _ := json.Marshal(map[string]any{"version": 1, "files": files})
	os.WriteFile(filepath.Join(dir, "docs/task.review-context.json"), manifest, 0600)
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "Source capacity recipe baseline")
	storage := t.TempDir()
	gitTest(t, storage, "clone", "--bare", dir, "repository.git")
	candidate := gitTest(t, dir, "rev-parse", "HEAD")
	w := Work{Planning: &PlanningState{Repository: &ManagedRepository{Storage: storage}, Reviewer: &ReviewerConfig{}}}
	tasks := []managedReviewTaskContext{{Task: "task"}}
	_, err := managedReviewSources(w, tasks, candidate)
	if err == nil || !managedReviewSourceCapacityRefusal(err) || strings.Contains(err.Error(), "manifeste") {
		t.Fatal("valid28-source manifest misclassified as malformed", err)
	}
	w.Planning.Reviewer.MaxContextFiles = quotaInt(32)
	_, err = managedReviewSources(w, tasks, candidate)
	if err == nil || !managedReviewSourceCapacityRefusal(err) || !strings.Contains(err.Error(), "128") {
		t.Fatal("source byte policy not enforced separately", err)
	}
	w.Planning.Reviewer.MaxContextBytes = quotaInt(200000)
	sources, err := managedReviewSources(w, tasks, candidate)
	if err != nil || len(sources) != 28 {
		t.Fatal("configured sources still capped or lost", len(sources), err)
	}
	for _, source := range sources {
		original, _ := os.ReadFile(filepath.Join(dir, source.Path))
		if !bytes.Equal(original, []byte(source.Content)) || source.Bytes != len(original) || source.Digest != hash(original) {
			t.Fatal("source truncated or altered", source.Path)
		}
	}
	context := managedReviewContext{Candidate: candidate, Tasks: []managedReviewTaskContext{{Task: "task", Criteria: []string{"all evidence"}}}, Sources: sources}
	// Paid contexts must remain verifiable after an operator lowers current policy.
	w.Planning.Reviewer.MaxContextFiles = quotaInt(1)
	owners, ownerErr := managedReviewSourceOwners(w, context)
	if ownerErr != nil {
		t.Fatal("current policy invalidated immutable source attribution", ownerErr)
	}
	if _, packetErr := planManagedReviewBatchesTransport("review", context, owners, false); packetErr != nil {
		t.Fatal("packet reintroduced hidden source policy", packetErr)
	}
	w.Planning.Reviewer.MaxSourceBytes = quotaInt(1000)
	if _, err = managedReviewSources(w, tasks, candidate); err == nil {
		t.Fatal("individual source cap ignored")
	}
	w.Planning.Reviewer.MaxContextFiles = quotaInt(0)
	w.Planning.Reviewer.MaxSourceBytes = quotaInt(0)
	w.Planning.Reviewer.MaxContextBytes = quotaInt(0)
	sources, err = managedReviewSources(w, tasks, candidate)
	if err != nil || len(sources) != 28 {
		t.Fatal("unlimited source policy retained hidden ceilings", len(sources), err)
	}
	if gitTest(t, dir, "rev-parse", "HEAD") != candidate || gitTest(t, dir, "status", "--porcelain") != "" {
		t.Fatal("source collection changed original")
	}
}

func TestReviewSourcePolicyPublicRetryPreservesAttributedUnpaidCheckpoint(t *testing.T) {
	for _, kind := range []string{"file-count", "individual-bytes"} {
		t.Run(kind, func(t *testing.T) {
			s, w := managedFixture(t)
			a := managedCompleted(t, s, w, "first", "retained source\n")
			a.Ended = now()
			if err := s.saveAgent(a); err != nil {
				t.Fatal(err)
			}
			files := []string{}
			count, size := 28, 20
			if kind == "individual-bytes" {
				count, size = 1, defaultReviewSourceBytes+1
			}
			for i := 0; i < count; i++ {
				name := fmt.Sprintf("source-%02d.txt", i)
				files = append(files, name)
				if err := os.WriteFile(filepath.Join(a.CWD, name), []byte(strings.Repeat("a", size)), 0600); err != nil {
					t.Fatal(err)
				}
			}
			manifest, _ := json.Marshal(map[string]any{"version": 1, "files": files})
			if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.review-context.json"), manifest, 0600); err != nil {
				t.Fatal(err)
			}
			if err := s.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			task, _ := w.task(a.TaskID)
			before, _ := s.managedAttempt(a.ID)
			if task.Status != "blocked" || !reviewSourceCapacityReason(before.Detail) || managedReviewCalls(t, s) != 0 {
				t.Fatal("capacity not attributed before paid call", task.Status, before.Detail)
			}
			if _, err := s.prepareManagedPreflightRetry(w.ID, a.TaskID); err == nil {
				t.Fatal("unchanged policy allowed retry")
			}
			q, _ := quotaView(w)
			change := QuotaChange{Schema: 1, EventID: "source-policy-correction", Revision: w.Revision, Limits: q.Limits, Reason: "Relax measured source capacity without changing producer or evidence"}
			change.Limits.ReviewContextFiles = quotaInt(32)
			change.Limits.ReviewSourceBytes = quotaInt(196608)
			change.Limits.ReviewContextBytes = quotaInt(524288)
			changed, err := s.configureQuotas(w.ID, change)
			if err != nil {
				t.Fatal(err)
			}
			if _, err = s.prepareManagedPreflightRetry(w.ID, a.TaskID); err != nil {
				t.Fatal("corrected policy did not reach packet/recovery consumer", err)
			}
			after, err := s.planningChange(w.ID, "retry-review", PlanningRequest{Schema: 1, EventID: "retry-source-checkpoint", Revision: changed.Revision, Task: a.TaskID, Reason: "Resume identical retained evidence after public policy correction"})
			if err != nil {
				t.Fatal(err)
			}
			item, _ := s.managedAttempt(a.ID)
			task, _ = after.task(a.TaskID)
			if item.Result != before.Result || item.State != "integrating" || len(task.Attempts) != 1 || managedReviewCalls(t, s) != 0 {
				t.Fatal("retry rewrote result or consumed attempt/review")
			}
		})
	}
}
