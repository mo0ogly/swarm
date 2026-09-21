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

func TestManagedReviewPacketPreservesAllEvidenceThroughProcess(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "reviewed\n")
	if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.review-context.json"), []byte(`{"version":1,"files":["value.txt"]}`), 0600); err != nil {
		t.Fatal(err)
	}
	// JSON escaping alone pushes this under-limit evidence beyond the cap.
	content := "Report\n" + strings.Repeat("\t<é>\"\\\n", 7000) + "SWARM_MANAGED_REVIEW_TEXT_CONTEXT\n{\"field\":\"diff\",\"bytes\":0}\n"
	if err := os.WriteFile(filepath.Join(a.CWD, "docs/first.md"), []byte(content), 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task("first")
	if task.Status != "accepted" || managedReviewCalls(t, s) != 1 {
		t.Fatalf("packet not reviewed: %+v", task)
	}
	data, err := os.ReadFile(filepath.Join(s.root, "review-fixture/observed.json"))
	if err != nil {
		t.Fatal(err)
	}
	var observed struct {
		Context managedReviewContext `json:"context"`
		Prompt  string               `json:"prompt"`
	}
	if err = json.Unmarshal(data, &observed); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(observed.Prompt, "\nSWARM_MANAGED_REVIEW_TEXT_CONTEXT\n") || len(observed.Prompt) > 192*1024 {
		t.Fatal("not testing bounded text transport")
	}
	if !strings.Contains(observed.Prompt, `"content_from_added_diff":"docs/first.md"`) {
		t.Fatal("duplicate report not reused")
	}
	stored, err := os.ReadFile(filepath.Join(s.root, task.IndependentReview.Context))
	if err != nil {
		t.Fatal(err)
	}
	decoded, _ := json.Marshal(observed.Context)
	var expected, received any
	json.Unmarshal(stored, &expected)
	json.Unmarshal(decoded, &received)
	if !reflect.DeepEqual(expected, received) || hash(stored) != task.IndependentReview.ContextDigest {
		t.Fatal("process received different evidence from canonical stored context")
	}
	if len(observed.Context.Sources) != 1 || observed.Context.Sources[0].Content != "reviewed\n" {
		t.Fatal("source bytes missing")
	}
	if !strings.Contains(observed.Context.Tasks[0].Report, content[:100]) {
		t.Fatal("report missing")
	}
	if err := s.integrateManagedAttempt(a); err != nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("replay duplicated paid review", err)
	}
}

func TestManagedReviewPacketDoesNotMutateCanonicalContext(t *testing.T) {
	c := managedReviewContext{Diff: "diff\n", Receipt: json.RawMessage(`{"candidate_commit":"sha"}`), Sources: []ReviewSource{{Content: "source\n"}}, Tasks: []managedReviewTaskContext{{Report: "report\n"}}}
	before, _ := json.Marshal(c)
	packet := managedReviewTextPacket(c)
	after, _ := json.Marshal(c)
	if string(before) != string(after) || !strings.Contains(packet, "source\n") || !strings.Contains(packet, "report\n") {
		t.Fatal("canonical evidence mutated or absent")
	}
}

func TestReportDiffReferenceRequiresExactCompleteNewFile(t *testing.T) {
	blob := strings.Repeat("a", 40)
	diff := "diff --git a/docs/t.md b/docs/t.md\nnew file mode 100644\nindex " + strings.Repeat("0", 40) + ".." + blob + "\n--- /dev/null\n+++ b/docs/t.md\n@@ -0,0 +1,2 @@\n+rapport é\n+preuve"
	if !reportInAddedDiff(diff, "docs/t.md", "rapport é\npreuve") {
		t.Fatal("full new report not recognized")
	}
	for _, changed := range []string{strings.Replace(diff, "+preuve", "+autre", 1), strings.Replace(diff, "new file mode", "old file mode", 1), diff + "\n\\ No newline at end of file"} {
		if reportInAddedDiff(changed, "docs/t.md", "rapport é\npreuve") {
			t.Fatal("nonidentical report reference")
		}
	}
	if reportInAddedDiff(diff, "docs/other.md", "rapport é\npreuve") || reportInAddedDiff(diff, "docs/t.md", "rapport é") {
		t.Fatal("partial or wrong report reference")
	}
}
