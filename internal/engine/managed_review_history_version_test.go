//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestHistoricalReviewPrefixNeverFreshAcceptance(t *testing.T) {
	s, w, a := managedBatchRuntimeFixture(t)
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task := &w.Tasks[0]
	r := task.IndependentReview
	raw, err := os.ReadFile(filepath.Join(s.root, r.Context))
	if err != nil {
		t.Fatal(err)
	}
	var c managedReviewContext
	if err = json.Unmarshal(raw, &c); err != nil {
		t.Fatal(err)
	}
	owners, err := managedReviewSourceOwners(w, c)
	if err != nil {
		t.Fatal(err)
	}
	workflow, prompt, err := agentWorkflow("reviewer")
	if err != nil {
		t.Fatal(err)
	}
	prefix := legacyManagedReviewPrefix(managedReviewPrefix(prompt))
	batches, err := planManagedReviewBatches(prefix, c, owners)
	if err != nil {
		t.Fatal(err)
	}
	r.BatchPlanDigest = batchPlanDigest(c, prefix, batches, w.Planning.Reviewer, workflow)
	if err = s.independentReviewGuard(&w, task); err == nil {
		t.Fatal("legacy prompt accepted as fresh verdict")
	}
	if err = s.historicalReviewBaselineGuard(w, task); err != nil {
		t.Fatal("exact historical protocol rejected", err)
	}
	r.BatchPlanDigest = "forged"
	if err = s.historicalReviewBaselineGuard(w, task); err == nil {
		t.Fatal("forged protocol accepted")
	}
	r.BatchPlanDigest = batchPlanDigest(c, prefix, batches, w.Planning.Reviewer, workflow)
	if err = os.WriteFile(filepath.Join(s.root, r.Context), []byte("tampered"), 0600); err != nil {
		t.Fatal(err)
	}
	if err = s.historicalReviewBaselineGuard(w, task); err == nil {
		t.Fatal("tampered context accepted")
	}
}
