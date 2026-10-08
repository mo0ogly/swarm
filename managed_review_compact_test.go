//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedReviewCompactPreservesPatchAndSources(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	gitTest(t, dir, "config", "user.email", "fixture@example.invalid")
	gitTest(t, dir, "config", "user.name", "Fixture")
	lines := make([]string, 100)
	for i := range lines {
		lines[i] = strings.TrimSpace(strings.Repeat("unchanged context ", 150))
	}
	path := filepath.Join(dir, "large.txt")
	os.WriteFile(path, []byte(strings.Join(lines, "\n")+"\n"), 0600)
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "base")
	base := gitTest(t, dir, "rev-parse", "HEAD")
	lines[50] = "changed fact"
	os.WriteFile(path, []byte(strings.Join(lines, "\n")+"\n"), 0600)
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "candidate")
	candidate := gitTest(t, dir, "rev-parse", "HEAD")
	c := managedReviewContext{Candidate: candidate, Diff: gitTest(t, dir, "diff", "--unified=40", base, candidate), Receipt: json.RawMessage(`{"proof":"unchanged"}`)}
	original := c.Diff
	receipt := string(c.Receipt)
	if e := compactManagedReviewContext(&c, dir, base); e != nil {
		t.Fatal(e)
	}
	if len(c.Diff) >= len(original) || !strings.Contains(c.Diff, "+changed fact") || string(c.Receipt) != receipt {
		t.Fatal("lost change/proof or no compaction")
	}
	patch := filepath.Join(t.TempDir(), "review.patch")
	os.WriteFile(patch, []byte(c.Diff+"\n"), 0600)
	gitTest(t, dir, "apply", "--reverse", "--check", patch)
	prior := c.Diff
	if e := compactManagedReviewContext(&c, dir, base); e != nil || prior != c.Diff {
		t.Fatal("unstable context")
	}
}
