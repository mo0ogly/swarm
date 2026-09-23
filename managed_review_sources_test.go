//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedReviewSourcesAreCandidateBoundAndComplete(t *testing.T) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "candidate\n")
	content := "unchanged source, including its final newline\n"
	os.WriteFile(filepath.Join(a.CWD, "helper.txt"), []byte(content), 0600)
	os.WriteFile(filepath.Join(a.CWD, "docs/first.review-context.json"), []byte(`{"version":1,"files":["helper.txt","value.txt","helper.txt"]}`), 0600)
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(filepath.Join(s.root, "review-fixture/observed.json"))
	if err != nil {
		t.Fatal(err)
	}
	var observed struct {
		Context managedReviewContext `json:"context"`
	}
	if err = json.Unmarshal(data, &observed); err != nil {
		t.Fatal(err)
	}
	sources := observed.Context.Sources
	if len(sources) != 2 || sources[0].Content != content || sources[0].Bytes != len(content) || sources[0].Digest != hash([]byte(content)) || len(sources[0].Blob) != 40 {
		t.Fatalf("incomplete sources: %+v", sources)
	}
	os.WriteFile(filepath.Join(a.CWD, "helper.txt"), []byte("tampered workspace"), 0600)
	w, _ = s.get(w.ID)
	again, err := managedReviewSources(w, observed.Context.Tasks, observed.Context.Candidate)
	if err != nil || len(again) != 2 || again[0] != sources[0] {
		t.Fatalf("context followed mutable workspace: %+v %v", again, err)
	}
}

func TestCandidateReviewSourceRejectsUnsafeOrOversizedContent(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	os.WriteFile(filepath.Join(dir, "good.txt"), []byte("all bytes\n"), 0600)
	os.WriteFile(filepath.Join(dir, "binary"), []byte{0, 1, 2}, 0600)
	os.WriteFile(filepath.Join(dir, "invalid-utf8"), []byte{255}, 0600)
	os.Symlink("/etc/passwd", filepath.Join(dir, "link"))
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "fixture")
	sha := gitTest(t, dir, "rev-parse", "HEAD")
	bare := filepath.Join(dir, ".git")
	for _, name := range []string{"../good.txt", "/etc/passwd", "./good.txt", ".git/config", "good.txt:other", "link", "binary", "invalid-utf8", "missing", "good.txt\n"} {
		t.Run(strings.ReplaceAll(name, "/", "_"), func(t *testing.T) {
			if _, err := candidateReviewSource(bare, sha, name, 100); err == nil {
				t.Fatal("unsafe source accepted")
			}
		})
	}
	if _, err := candidateReviewSource(bare, sha, "good.txt", 2); err == nil {
		t.Fatal("oversized content truncated")
	}
	if source, err := candidateReviewSource(bare, sha, "good.txt", 100); err != nil || source.Content != "all bytes\n" {
		t.Fatalf("regular text refused: %+v %v", source, err)
	}
}

func TestManagedReviewSourcesInvalidManifestCannotLaunchReviewer(t *testing.T) {
	for _, manifest := range []string{`{"version":1,"files":["missing"]}`, `{"version":2,"files":["value.txt"]}`, `{"version":1,"files":[],"unknown":true}`} {
		t.Run(manifest, func(t *testing.T) {
			s, w := managedFixture(t)
			a := managedCompleted(t, s, w, "first", "candidate\n")
			os.WriteFile(filepath.Join(a.CWD, "docs/first.review-context.json"), []byte(manifest), 0600)
			_ = s.integrateManagedAttempt(a)
			after, _ := s.get(w.ID)
			if after.Tasks[0].Status == "accepted" || managedReviewCalls(t, s) != 0 {
				t.Fatal("invalid source manifest reached paid reviewer or acceptance")
			}
		})
	}
}

func TestManagedReviewSourcesRejectsCumulativeOverflowWithoutTruncation(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	os.MkdirAll(filepath.Join(dir, "docs"), 0700)
	os.WriteFile(filepath.Join(dir, "one"), []byte(strings.Repeat("a", 70*1024)), 0600)
	os.WriteFile(filepath.Join(dir, "two"), []byte(strings.Repeat("b", 70*1024)), 0600)
	os.WriteFile(filepath.Join(dir, "docs/first.review-context.json"), []byte(`{"version":1,"files":["one","two"]}`), 0600)
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "oversized context")
	storage := t.TempDir()
	gitTest(t, storage, "clone", "--bare", dir, "repository.git")
	w := Work{Planning: &PlanningState{Repository: &ManagedRepository{Storage: storage}}}
	if sources, err := managedReviewSources(w, []managedReviewTaskContext{{Task: "first"}}, gitTest(t, dir, "rev-parse", "HEAD")); err == nil || len(sources) != 0 || !strings.Contains(err.Error(), "128") {
		t.Fatalf("oversized context not rejected: %d %v", len(sources), err)
	}
}

func TestManagedReviewSourcesPerTaskBound(t *testing.T) {
	dir := t.TempDir()
	gitTest(t, dir, "init")
	os.MkdirAll(filepath.Join(dir, "docs"), 0700)
	for _, name := range []string{"one", "two"} {
		os.WriteFile(filepath.Join(dir, name), []byte(strings.Repeat(name+"\n", 20000)), 0600)
	}
	os.WriteFile(filepath.Join(dir, "docs/first.review-context.json"), []byte(`{"version":1,"files":["one"]}`), 0600)
	os.WriteFile(filepath.Join(dir, "docs/second.review-context.json"), []byte(`{"version":1,"files":["two"]}`), 0600)
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "separate bounded task sources")
	storage := t.TempDir()
	gitTest(t, storage, "clone", "--bare", dir, "repository.git")
	w := Work{Planning: &PlanningState{Repository: &ManagedRepository{Storage: storage}}}
	sha := gitTest(t, dir, "rev-parse", "HEAD")
	sources, e := managedReviewSourcesByTask(w, []managedReviewTaskContext{{Task: "first"}, {Task: "second"}}, sha)
	if e != nil || len(sources) != 2 || len(sources[0].Content)+len(sources[1].Content) != 160000 {
		t.Fatal("lost bounded sources", e)
	}
	// The original bound still rejects the same data assigned to a single task.
	os.WriteFile(filepath.Join(dir, "docs/first.review-context.json"), []byte(`{"version":1,"files":["one","two"]}`), 0600)
	gitTest(t, dir, "add", ".")
	gitTest(t, dir, "commit", "-m", "single oversized task")
	sha = gitTest(t, dir, "rev-parse", "HEAD")
	gitTest(t, filepath.Join(storage, "repository.git"), "fetch", dir, sha)
	if _, e = managedReviewSourcesByTask(w, []managedReviewTaskContext{{Task: "first"}}, sha); e == nil || !strings.Contains(e.Error(), "128") {
		t.Fatal("single-task bound weakened", e)
	}
}
