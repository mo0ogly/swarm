//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func controlDiagnosticFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w, a, r := recoveredResultFixture(t)
	var err error
	w, err = s.mutate(w.ID, "fixture", "failed-control-policy", w.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task(a.TaskID)
		task.ValidationPolicy = automaticPolicy("python3", "-c", "print('retained control original');raise SystemExit(7)")
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	raw, _ := json.Marshal(completeDelivery(task, a))
	if err = os.WriteFile(filepath.Join(a.CWD, "docs/first.delivery.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	gitTest(t, a.CWD, "add", "-A")
	r.ResultTree = gitTest(t, a.CWD, "write-tree")
	r.Revision = w.Revision
	w, err = s.planningChange(w.ID, "submit-recovered-result", r)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = w.task(a.TaskID)
	if !strings.Contains(task.Blocker, "contrôle en échec") {
		t.Fatal("not a control failure", task.Blocker)
	}
	return s, w, a
}
func TestControlDiagnosticPublicHTTPPreservesOriginalAndHasNoWrite(t *testing.T) {
	s, w, a := controlDiagnosticFixture(t)
	h := newWebHandler(s, "local.test", "diag-token")
	path := "/api/v1/control-diagnostic?work=" + w.ID + "&task=" + a.TaskID
	rr := runLimitsWebRequest(h, "GET", path, "diag-token", nil)
	if rr.Code != 200 {
		t.Fatalf("original diagnostic unavailable: %d %s", rr.Code, rr.Body.String())
	}
	var got struct {
		Evidence   string                `json:"evidence_sha256"`
		Diagnostic managedControlFailure `json:"diagnostic"`
	}
	if err := json.Unmarshal(rr.Body.Bytes(), &got); err != nil {
		t.Fatal(err)
	}
	if got.Diagnostic.Work != w.ID || got.Diagnostic.Producer != a.ID || got.Diagnostic.Attempt != a.Attempt || got.Diagnostic.Result.ExitCode != 7 || string(got.Diagnostic.Output) != "retained control original\n" || len(got.Evidence) != 64 {
		t.Fatal("provenance or original output lost", rr.Body.String())
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
		t.Fatal("read changed state or spent a call")
	}
	denied := runLimitsWebRequest(h, "GET", path, "wrong-token", nil)
	if denied.Code == 200 {
		t.Fatal("unauthenticated original exposed")
	}
}
func TestControlDiagnosticPublicHTTPRejectsStaleOrCorruptBindings(t *testing.T) {
	for _, mode := range []string{"other-task", "replaced-blocker", "new-attempt", "corrupt-output"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := controlDiagnosticFixture(t)
			task, _ := w.task(a.TaskID)
			if mode == "corrupt-output" {
				parts := controlFailurePattern.FindStringSubmatch(task.Blocker)
				path, err := localFile(s.root, parts[4])
				if err != nil {
					t.Fatal(err)
				}
				raw, err := os.ReadFile(path)
				if err != nil {
					t.Fatal(err)
				}
				var d managedControlFailure
				if err = json.Unmarshal(raw, &d); err != nil {
					t.Fatal(err)
				}
				d.Output = []byte("tampered")
				raw, _ = json.Marshal(d)
				if err = os.WriteFile(path, raw, 0600); err != nil {
					t.Fatal(err)
				}
			}
			if mode == "replaced-blocker" || mode == "new-attempt" {
				var err error
				w, err = s.mutate(w.ID, "fixture", "replace-binding", w.Revision, []byte(`{}`), func(w *Work) error {
					task, _ := w.task(a.TaskID)
					if mode == "replaced-blocker" {
						task.Blocker = "different failure"
					} else {
						task.Attempts = append(task.Attempts, Attempt{ID: "replacement", Status: "interrupted"})
					}
					return nil
				})
				if err != nil {
					t.Fatal(err)
				}
			}
			id := a.TaskID
			if mode == "other-task" {
				id = "unrelated"
			}
			h := newWebHandler(s, "local.test", "diag-token")
			rr := runLimitsWebRequest(h, "GET", "/api/v1/control-diagnostic?work="+w.ID+"&task="+id, "diag-token", nil)
			if rr.Code == 200 || strings.Contains(rr.Body.String(), "retained control original") {
				t.Fatal("stale or corrupt original exposed", rr.Code, rr.Body.String())
			}
		})
	}
}
