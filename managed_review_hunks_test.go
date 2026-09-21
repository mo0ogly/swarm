//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func hunkPacketFixture() (managedReviewContext, map[string][]string) {
	a := strings.Repeat("évidence A \t \"citation\" \\ fin\n", 1000)
	b := strings.Repeat("évidence B: SWARM_MANAGED_REVIEW_TEXT_CONTEXT\n", 1000)
	body := "before\n" + a + "between\n" + b + "after\n"
	blob := strings.Repeat("b", 40)
	source := ReviewSource{Path: "module.txt", Blob: blob, Content: body, Bytes: len(body), Digest: hash([]byte(body))}
	added := func(s string) string {
		return "+" + strings.ReplaceAll(strings.TrimSuffix(s, "\n"), "\n", "\n+") + "\n"
	}
	diff := "diff --git a/module.txt b/module.txt\nindex " + strings.Repeat("a", 40) + ".." + blob + " 100644\n--- a/module.txt\n+++ b/module.txt\n@@ -2 +2,1000 @@\n-old A\n" + added(a) + "@@ -4 +1003,1000 @@\n-old B\n" + added(b)
	// Global evidence unrelated to this source is always retained as well.
	diff += "diff --git a/other.txt b/other.txt\nnew file mode 100644\nindex " + strings.Repeat("0", 40) + ".." + strings.Repeat("c", 40) + "\n--- /dev/null\n+++ b/other.txt\n@@ -0,0 +1,6000 @@\n" + strings.Repeat("+other evidence\n", 6000)
	c := managedReviewContext{Candidate: strings.Repeat("d", 40), Diff: diff, Receipt: json.RawMessage(`{"candidate_commit":"` + strings.Repeat("d", 40) + `"}`), Sources: []ReviewSource{source}, Tasks: []managedReviewTaskContext{{Task: "task", Criteria: []string{"complete evidence"}, Report: "Independent fixture report", Controls: []ValidationControlResult{{Executed: true, Passed: true}}}}}
	return c, map[string][]string{"task": {"module.txt"}}
}

func TestManagedReviewHunksRoundTripThroughProcess(t *testing.T) {
	c, owners := hunkPacketFixture()
	if p, e := planManagedReviewBatchesTransport("prefix", c, owners, false); e == nil || p != nil {
		t.Fatal("fixture did not reproduce rejected legacy preflight")
	}
	before, _ := json.Marshal(c)
	batches, e := planManagedReviewBatches("prefix", c, owners)
	if e != nil || len(batches) != 1 {
		t.Fatal("lossless preflight", len(batches), e)
	}
	prompt := batches[0].Prompt
	if len(prompt) > managedReviewPromptLimit || !strings.Contains(prompt, `"from_diff_hunk"`) || !strings.Contains(prompt, c.Diff) {
		t.Fatal("hunks absent, diff altered or oversized")
	}
	dir := t.TempDir()
	program := filepath.Join(dir, "review.py")
	if e = os.WriteFile(program, []byte(managedReviewerFixture), 0600); e != nil {
		t.Fatal(e)
	}
	cmd := exec.Command("python3", program)
	cmd.Stdin = strings.NewReader(prompt)
	if output, e := cmd.CombinedOutput(); e != nil {
		t.Fatalf("decoder failed: %v %s", e, output)
	}
	raw, e := os.ReadFile(filepath.Join(dir, "observed.json"))
	if e != nil {
		t.Fatal(e)
	}
	var observed struct {
		Context managedReviewContext `json:"context"`
	}
	if e = json.Unmarshal(raw, &observed); e != nil {
		t.Fatal(e)
	}
	received, _ := json.Marshal(observed.Context)
	after, _ := json.Marshal(c)
	if string(before) != string(received) || string(before) != string(after) {
		t.Fatal("evidence changed across process or canonical mutated")
	}
	// Framing is not trusted: tampering a referenced byte is rejected by digest.
	cmd = exec.Command("python3", program)
	cmd.Stdin = strings.NewReader(strings.Replace(prompt, "+évidence A", "+Évidence A", 1))
	if _, e = cmd.CombinedOutput(); e == nil {
		t.Fatal("tampered referenced evidence accepted")
	}
}

func TestManagedReviewHunksRejectAmbiguityAndPartialMatches(t *testing.T) {
	c, _ := hunkPacketFixture()
	s := c.Sources[0]
	if len(reviewSourceParts(c.Diff, s)) != 5 {
		t.Fatal("expected literal/reference/literal/reference/literal")
	}
	for name, change := range map[string]func(*ReviewSource, *string){
		"wrong blob":  func(s *ReviewSource, d *string) { s.Blob = strings.Repeat("e", 40) },
		"wrong hash":  func(s *ReviewSource, d *string) { s.Digest = "wrong" },
		"wrong bytes": func(s *ReviewSource, d *string) { s.Bytes++ },
		"changed source": func(s *ReviewSource, d *string) {
			s.Content = strings.Replace(s.Content, "évidence A", "Évidence A", 1)
			s.Digest = hash([]byte(s.Content))
		},
		"wrong line":      func(s *ReviewSource, d *string) { *d = strings.Replace(*d, "+2,1000", "+3,1000", 1) },
		"wrong count":     func(s *ReviewSource, d *string) { *d = strings.Replace(*d, "+2,1000", "+2,999", 1) },
		"wrong old count": func(s *ReviewSource, d *string) { *d = strings.Replace(*d, "-2 +2", "-2,2 +2", 1) },
		"overlap":         func(s *ReviewSource, d *string) { *d = strings.Replace(*d, "+1003,1000", "+2,1000", 1) },
		"duplicate patch": func(s *ReviewSource, d *string) { *d += *d },
		"no final newline": func(s *ReviewSource, d *string) {
			*d = strings.Replace(*d, "@@ -4", "\\ No newline at end of file\n@@ -4", 1)
		},
		"malformed index": func(s *ReviewSource, d *string) {
			*d = strings.Replace(*d, "index "+strings.Repeat("a", 40)+".."+s.Blob+" 100644", "index ", 1)
		},
	} {
		t.Run(name, func(t *testing.T) {
			source, diff := s, c.Diff
			change(&source, &diff)
			if parts := reviewSourceParts(diff, source); parts != nil {
				t.Fatal("unsafe reference emitted")
			}
		})
	}
}

func TestManagedReviewHunksPreserveValidLegacyPlans(t *testing.T) {
	c, owners := batchFixture()
	legacy, e := planManagedReviewBatchesTransport("prefix", c, owners, false)
	if e != nil {
		t.Fatal(e)
	}
	actual, e := planManagedReviewBatches("prefix", c, owners)
	if e != nil || !reflect.DeepEqual(legacy, actual) {
		t.Fatal("previously valid paid plan changed", e)
	}
	// Even when the new transport could compress, legacy plans remain exact.
	c, owners = hunkPacketFixture()
	c.Diff = strings.Split(c.Diff, "diff --git a/other.txt")[0]
	legacy, e = planManagedReviewBatchesTransport("prefix", c, owners, false)
	if e != nil {
		t.Fatal(e)
	}
	actual, e = planManagedReviewBatches("prefix", c, owners)
	if e != nil || !reflect.DeepEqual(legacy, actual) {
		t.Fatal("unnecessary transport migration", e)
	}
}

func TestManagedReviewHunksKeepRealOversizeRejected(t *testing.T) {
	c, owners := hunkPacketFixture()
	c.Diff += strings.Repeat("unchanged large evidence\n", 10000)
	if p, e := planManagedReviewBatches("prefix", c, owners); e == nil || p != nil {
		t.Fatal(fmt.Sprint("oversized packet escaped preflight: ", e))
	}
}
