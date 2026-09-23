//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func batchFixture() (managedReviewContext, map[string][]string) {
	c := managedReviewContext{Candidate: strings.Repeat("a", 40), Previous: strings.Repeat("b", 40), Diff: strings.Repeat("diff evidence\n", 8000), Receipt: json.RawMessage(`{"controls":"global receipt"}`)}
	owners := map[string][]string{}
	for _, id := range []string{"first", "second", "third"} {
		c.Tasks = append(c.Tasks, managedReviewTaskContext{Task: id, Binding: ManagedTaskReview{Task: id, Attempt: "attempt-" + id, Producer: "producer-" + id}, Criteria: []string{"criterion one", "criterion two"}, Report: strings.Repeat(id+" report\n", 180)})
		content := strings.Repeat(id+" source\n", 2600)
		c.Sources = append(c.Sources, ReviewSource{Path: id + ".go", Blob: strings.Repeat("c", 40), Content: content, Bytes: len(content), Digest: hash([]byte(content))})
		owners[id] = []string{id + ".go"}
	}
	return c, owners
}

func TestManagedBatchPreflightPreservesGlobalEvidenceAndCoverage(t *testing.T) {
	c, owners := batchFixture()
	before, _ := json.Marshal(c)
	batches, err := planManagedReviewBatches("review instruction", c, owners)
	if err != nil {
		t.Fatal(err)
	}
	if len(batches) < 2 {
		t.Fatal("fixture did not exercise split", len(batches))
	}
	seen := map[string]bool{}
	sources := map[string]bool{}
	for _, b := range batches {
		if len(b.Prompt) > managedReviewPromptLimit || b.Context.Candidate != c.Candidate || b.Context.Previous != c.Previous || b.Context.Diff != c.Diff || string(b.Context.Receipt) != string(c.Receipt) || !reflect.DeepEqual(b.Context.Tasks, c.Tasks) {
			t.Fatal("global proof changed or limit exceeded")
		}
		for _, id := range b.Tasks {
			if seen[id] {
				t.Fatal("duplicate coverage")
			}
			seen[id] = true
			for _, p := range owners[id] {
				found := false
				for _, s := range b.Context.Sources {
					if s.Path == p {
						found = true
						sources[p] = true
					}
				}
				if !found {
					t.Fatal("assigned source missing", p)
				}
			}
		}
	}
	if len(seen) != len(c.Tasks) || len(sources) != len(c.Sources) {
		t.Fatal("incomplete coverage")
	}
	after, _ := json.Marshal(c)
	if string(before) != string(after) {
		t.Fatal("canonical evidence mutated")
	}
	again, err := planManagedReviewBatches("review instruction", c, owners)
	if err != nil || !reflect.DeepEqual(batches, again) {
		t.Fatal("nondeterministic plan", err)
	}
}

func TestManagedBatchPreflightFailsBeforeAnyUsablePlan(t *testing.T) {
	for _, kind := range []string{"global-too-large", "single-too-large", "unassigned", "unknown-task", "unknown-source", "changed-source", "duplicate-source", "task-source-limit", "empty-criteria"} {
		t.Run(kind, func(t *testing.T) {
			c, owners := batchFixture()
			switch kind {
			case "global-too-large":
				c.Diff = strings.Repeat("x", managedReviewPromptLimit)
			case "single-too-large":
				c.Diff = strings.Repeat("x", 180*1024)
			case "unassigned":
				delete(owners, "first")
			case "unknown-task":
				owners["alien"] = []string{"first.go"}
			case "unknown-source":
				owners["first"] = []string{"alien.go"}
			case "changed-source":
				c.Sources[0].Content += "changed"
			case "duplicate-source":
				c.Sources = append(c.Sources, c.Sources[0])
			case "task-source-limit":
				owners["first"] = []string{"first.go", "second.go", "third.go"}
				for i := range c.Sources {
					c.Sources[i].Content = strings.Repeat("x", 50*1024)
					c.Sources[i].Bytes = len(c.Sources[i].Content)
					c.Sources[i].Digest = hash([]byte(c.Sources[i].Content))
				}
			case "empty-criteria":
				c.Tasks[0].Criteria = nil
			}
			plan, err := planManagedReviewBatches("review", c, owners)
			if err == nil || plan != nil {
				t.Fatal("invalid plan partially usable", kind, len(plan), err)
			}
		})
	}
}

func TestManagedBatchSingleContextKeepsLegacyPrompt(t *testing.T) {
	c, owners := batchFixture()
	c.Diff = "small diff"
	plan, err := planManagedReviewBatches("prefix", c, owners)
	if err != nil || len(plan) != 1 || plan[0].Prompt != boundedManagedReviewPrompt("prefix", c) || !reflect.DeepEqual(plan[0].Context, c) {
		t.Fatal("single-context compatibility lost", err)
	}
}

func TestManagedBatchSourceOwnershipUsesImmutableManifests(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	os.MkdirAll(filepath.Join(dir, "docs"), 0700)
	for name, body := range map[string]string{"shared.go": "shared source", "first.go": "first source", "docs/first.review-context.json": `{"version":1,"files":["shared.go","first.go"]}`, "docs/second.review-context.json": `{"version":1,"files":["shared.go"]}`} {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(body), 0600); err != nil {
			t.Fatal(err)
		}
	}
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "source ownership")
	storage := t.TempDir()
	gitTest(t, storage, "clone", "--bare", dir, "repository.git")
	w := Work{Planning: &PlanningState{Repository: &ManagedRepository{Storage: storage}}}
	c := managedReviewContext{Candidate: gitTest(t, dir, "rev-parse", "HEAD"), Tasks: []managedReviewTaskContext{{Task: "first"}, {Task: "second"}}}
	var err error
	c.Sources, err = managedReviewSources(w, c.Tasks, c.Candidate)
	if err != nil {
		t.Fatal(err)
	}
	owners, err := managedReviewSourceOwners(w, c)
	if err != nil || !reflect.DeepEqual(owners, map[string][]string{"first": {"shared.go", "first.go"}, "second": {"shared.go"}}) {
		t.Fatal(owners, err)
	}
	c.Sources = c.Sources[1:]
	if _, err = managedReviewSourceOwners(w, c); err == nil {
		t.Fatal("missing shared source silently dropped")
	}
}

func batchReplies(t *testing.T, batches []managedReviewBatch, verdict string) []string {
	t.Helper()
	replies := []string{}
	for _, b := range batches {
		tasks := []any{}
		for _, id := range b.Tasks {
			for _, tc := range b.Context.Tasks {
				if tc.Task != id {
					continue
				}
				criteria := []ReviewCriterion{}
				for i := range tc.Criteria {
					criteria = append(criteria, ReviewCriterion{Index: i + 1, Verdict: verdict, Evidence: tc.Report[:40]})
				}
				tasks = append(tasks, map[string]any{"task": id, "reason": "Independent fixture assessment", "criteria": criteria})
			}
		}
		raw, _ := json.Marshal(map[string]any{"candidate_commit": b.Context.Candidate, "tasks": tasks})
		replies = append(replies, string(raw))
	}
	return replies
}

func TestManagedBatchVerdictsRequireCompleteExactCoverage(t *testing.T) {
	c, owners := batchFixture()
	batches, err := planManagedReviewBatches("review", c, owners)
	if err != nil {
		t.Fatal(err)
	}
	state, records, err := parseManagedReviewBatchReplies(c, batches, batchReplies(t, batches, "pass"))
	if err != nil || state != "passed" || len(records) != len(c.Tasks) {
		t.Fatal(state, len(records), err)
	}
	for i, r := range records {
		if r.Task != c.Tasks[i].Task || len(r.Criteria) != len(c.Tasks[i].Criteria) {
			t.Fatal("lost binding or criterion")
		}
	}
	for _, verdict := range []string{"fail", "unknown"} {
		replies := batchReplies(t, batches, "pass")
		replies[len(replies)-1] = batchReplies(t, batches, verdict)[len(replies)-1]
		state, _, err = parseManagedReviewBatchReplies(c, batches, replies)
		if err != nil || state != "changes_requested" {
			t.Fatal(verdict, state, err)
		}
	}
}

func TestManagedBatchVerdictsRejectMissingAndTamperedEvidence(t *testing.T) {
	for _, kind := range []string{"missing-reply", "missing-batch", "duplicate-task", "foreign-sha", "changed-diff", "changed-report", "changed-source", "missing-criteria", "extra-task"} {
		t.Run(kind, func(t *testing.T) {
			c, owners := batchFixture()
			batches, err := planManagedReviewBatches("review", c, owners)
			if err != nil {
				t.Fatal(err)
			}
			// Independent copy: mutations must not alter the canonical evidence too.
			raw, _ := json.Marshal(batches)
			batches = nil
			json.Unmarshal(raw, &batches)
			replies := batchReplies(t, batches, "pass")
			switch kind {
			case "missing-reply":
				replies = replies[:len(replies)-1]
			case "missing-batch":
				batches = batches[:len(batches)-1]
				replies = replies[:len(replies)-1]
			case "duplicate-task":
				batches[len(batches)-1].Tasks = []string{batches[0].Tasks[0]}
			case "foreign-sha":
				replies[0] = strings.ReplaceAll(replies[0], c.Candidate, strings.Repeat("d", 40))
			case "changed-diff":
				batches[0].Context.Diff += "tampered"
			case "changed-report":
				batches[0].Context.Tasks[0].Report += "tampered"
			case "changed-source":
				batches[0].Context.Sources[0].Content += "tampered"
			case "missing-criteria":
				var response map[string]any
				json.Unmarshal([]byte(replies[0]), &response)
				response["tasks"].([]any)[0].(map[string]any)["criteria"] = []any{}
				r, _ := json.Marshal(response)
				replies[0] = string(r)
			case "extra-task":
				var response map[string]any
				json.Unmarshal([]byte(replies[0]), &response)
				ts := response["tasks"].([]any)
				response["tasks"] = append(ts, ts[0])
				r, _ := json.Marshal(response)
				replies[0] = string(r)
			}
			state, records, err := parseManagedReviewBatchReplies(c, batches, replies)
			if err == nil || state == "passed" || records != nil {
				t.Fatal("invalid bundle produced usable aggregate", kind, state, err)
			}
		})
	}
}

// Cumulative sources can exceed one task's allowance, but each immutable task
// manifest and each actual provider prompt remain bounded independently.
func TestManagedBatchSourcesBoundPerTask(t *testing.T) {
	c, owners := batchFixture()
	c.Diff = strings.Repeat("diff evidence\n", 4000)
	for i := range c.Sources {
		c.Sources[i].Content = strings.Repeat("s", 80*1024)
		c.Sources[i].Bytes = len(c.Sources[i].Content)
		c.Sources[i].Digest = hash([]byte(c.Sources[i].Content))
	}
	batches, err := planManagedReviewBatches("review", c, owners)
	if err != nil {
		t.Fatal(err)
	}
	if len(batches) != 3 {
		t.Fatalf("expected three bounded batches, got %d", len(batches))
	}
	seen := map[string]bool{}
	for _, b := range batches {
		if len(b.Prompt) > managedReviewPromptLimit {
			t.Fatal("oversized prompt")
		}
		for _, source := range b.Context.Sources {
			seen[source.Path] = true
		}
	}
	if len(seen) != len(c.Sources) {
		t.Fatal("source lost")
	}
	owners["first"] = []string{"first.go", "second.go"}
	if _, err := planManagedReviewBatches("review", c, owners); err == nil {
		t.Fatal("oversized task accepted")
	}
}
