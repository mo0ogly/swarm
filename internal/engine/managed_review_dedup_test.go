//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestAddedReviewSourceDedupIsLosslessAndConservative(t *testing.T) {
	content := "first line\n\n+literal plus\nlast line\n"
	source := ReviewSource{Path: "new.go", Blob: strings.Repeat("a", 40), Content: content}
	diff := "diff --git a/new.go b/new.go\nnew file mode 100644\nindex " + strings.Repeat("0", 40) + ".." + source.Blob + "\n--- /dev/null\n+++ b/new.go\n@@ -0,0 +1,4 @@\n+first line\n+\n++literal plus\n+last line"
	if !addedSourceInDiff(diff, source) {
		t.Fatal("complete added source not recognized")
	}
	for _, bad := range []string{strings.Replace(diff, "new file mode", "old file mode", 1), strings.Replace(diff, source.Blob, strings.Repeat("b", 40), 1), strings.Replace(diff, "+last line", "+wrong line", 1), diff + "\n\\ No newline at end of file"} {
		if addedSourceInDiff(bad, source) {
			t.Fatal("non-equivalent source removed")
		}
	}
}

func TestManagedReviewCompactionPreservesDeclaredOriginals(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	gitTest(t, dir, "config", "user.email", "fixture@example.invalid")
	gitTest(t, dir, "config", "user.name", "Fixture")
	gitTest(t, dir, "commit", "--allow-empty", "-m", "base")
	base := gitTest(t, dir, "rev-parse", "HEAD")
	quote := "func exactOriginal() {\n\tvalue := \"é + original\"\n\t_ = value\n}\n"
	content := quote + strings.Repeat("padding line\n", 7254)
	os.WriteFile(filepath.Join(dir, "new.txt"), []byte(content), 0600)
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "candidate")
	candidate := gitTest(t, dir, "rev-parse", "HEAD")
	diff := gitTest(t, dir, "diff", "--full-index", "--unified=3", base, candidate)
	c := managedReviewContext{Candidate: candidate, Diff: diff, Sources: []ReviewSource{{Path: "new.txt", Blob: gitTest(t, dir, "rev-parse", "HEAD:new.txt"), Content: content}}}
	raw, _ := json.Marshal(c)
	if len(raw) <= 160*1024 {
		t.Fatal("fixture outside instruction-reserve window", len(raw))
	}
	if err := compactManagedReviewContext(&c, dir, base); err != nil {
		t.Fatal(err)
	}
	if len(c.Sources) != 1 || c.Sources[0].Content != content || c.Diff != diff {
		t.Fatal("declared original lost or altered during compaction")
	}
	c.Tasks = []managedReviewTaskContext{{Task: "original-source", Criteria: []string{"Inspect complete physical original"}}}
	for _, tc := range []struct {
		name, evidence string
		pass           bool
	}{
		{"literal", quote, true},
		{"mutated", strings.Replace(quote, "original", "changed", 1), false},
		{"flattened", strings.ReplaceAll(quote, "\n", " "), false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			reply, err := json.Marshal(map[string]any{"candidate_commit": candidate, "tasks": []any{map[string]any{"task": c.Tasks[0].Task, "reason": "Exact declared physical source examined", "criteria": []any{map[string]any{"index": 1, "verdict": "pass", "evidence": tc.evidence}}}}})
			if err != nil {
				t.Fatal(err)
			}
			state, _, err := parseManagedReview(string(reply), c)
			if tc.pass && (err != nil || state != "passed") {
				t.Fatal("literal original rejected", state, err)
			}
			if !tc.pass && err == nil {
				t.Fatal("altered original accepted")
			}
		})
	}
}
