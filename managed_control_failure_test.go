//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestManagedFailedControlPreservesDiagnosticWithoutPublication(t *testing.T) {
	s, w := managedFixture(t)
	w, e := s.mutate(w.ID, "test.policy", "failed-diagnostic", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].ValidationPolicy = automaticPolicy("git", "diff", "--exit-code", "--no-index", "value.txt", "missing.txt")
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	a := managedCompleted(t, s, w, "first", "diagnostic\n")
	if e = s.integrateManagedAttempt(a); e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	if after.Tasks[0].Status != "blocked" || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate || managedReviewCalls(t, s) != 0 {
		t.Fatal("failure published or reviewed")
	}
	paths, e := filepath.Glob(filepath.Join(w.Planning.Repository.Storage, "diagnostics", "control-failure-*.json"))
	if e != nil || len(paths) != 1 {
		t.Fatal(paths, e)
	}
	raw, e := os.ReadFile(paths[0])
	if e != nil {
		t.Fatal(e)
	}
	var d managedControlFailure
	if e = json.Unmarshal(raw, &d); e != nil {
		t.Fatal(e)
	}
	if d.Work != w.ID || d.Task != "first" || d.Attempt != a.Attempt || d.Producer != a.ID || len(d.Candidate) != 40 || d.Previous != w.Planning.Repository.Candidate || d.Result.Passed || !d.Result.Executed || d.Result.ExitCode == 0 || len(d.Output) == 0 || hash(d.Output) != d.Result.OutputHash || d.OutputBytes != len(d.Output) || d.Truncated {
		t.Fatalf("incomplete diagnostic: %+v", d)
	}
	if got := gitTest(t, filepath.Join(w.Planning.Repository.Storage, "repository.git"), "rev-parse", "refs/swarm/failed-controls/"+d.Candidate); got != d.Candidate {
		t.Fatal("rejected candidate not retained", got)
	}
	if !strings.Contains(after.Tasks[0].Blocker, filepath.Base(paths[0])) {
		t.Fatal("diagnostic not discoverable")
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	persisted, e := os.ReadFile(paths[0])
	if e != nil || string(raw) != string(persisted) {
		t.Fatal("diagnostic lost on reopen", e)
	}
}

func TestControlCaptureKeepsBoundAndFailure(t *testing.T) {
	r, out, total := runValidationControlCaptured(t.TempDir(), ValidationControl{ID: "large", Command: []string{"python3", "-c", "import sys;sys.stdout.buffer.write(b'x'*70000);sys.stdout.flush();sys.exit(3)"}, Timeout: 10})
	if r.Passed || r.ExitCode != 3 || len(out) != maxValidationOutput || total != 70000 || hash(out) != r.OutputHash || !strings.Contains(r.Summary, "tronquée") || r.Finished == "" {
		t.Fatal(r, len(out), total)
	}
}
