//go:build linux

package main

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
	changed := source
	changed.Content += "extra\n"
	existing := source
	existing.Path = "existing.go"
	c := managedReviewContext{Diff: diff, Sources: []ReviewSource{source, changed, existing}}
	deduplicateAddedReviewSources(&c)
	if c.Diff != diff || len(c.Sources) != 2 || c.Sources[0].Content != changed.Content || c.Sources[1].Path != "existing.go" {
		t.Fatal("evidence removed", c)
	}
	for _, bad := range []string{strings.Replace(diff, "new file mode", "old file mode", 1), strings.Replace(diff, source.Blob, strings.Repeat("b", 40), 1), strings.Replace(diff, "+last line", "+wrong line", 1), diff + "\n\\ No newline at end of file"} {
		if addedSourceInDiff(bad, source) {
			t.Fatal("non-equivalent source removed")
		}
	}
}

func TestAddedReviewDedupReservesInstructionBudget(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	gitTest(t, dir, "config", "user.email", "fixture@example.invalid")
	gitTest(t, dir, "config", "user.name", "Fixture")
	gitTest(t, dir, "commit", "--allow-empty", "-m", "base")
	base := gitTest(t, dir, "rev-parse", "HEAD")
	content := strings.Repeat("x", 85*1024) + "\n"
	os.WriteFile(filepath.Join(dir, "new.txt"), []byte(content), 0600)
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "candidate")
	candidate := gitTest(t, dir, "rev-parse", "HEAD")
	diff := gitTest(t, dir, "diff", "--full-index", "--unified=3", base, candidate)
	c := managedReviewContext{Candidate: candidate, Diff: diff, Sources: []ReviewSource{{Path: "new.txt", Blob: gitTest(t, dir, "rev-parse", "HEAD:new.txt"), Content: content}}}
	raw, _ := json.Marshal(c)
	if len(raw) <= 160*1024 || len(raw) >= 192*1024 {
		t.Fatal("fixture outside instruction-reserve window", len(raw))
	}
	if err := compactManagedReviewContext(&c, dir, base); err != nil {
		t.Fatal(err)
	}
	if len(c.Sources) != 0 || c.Diff != diff {
		t.Fatal("duplicate left no room for instructions or diff changed")
	}
}
