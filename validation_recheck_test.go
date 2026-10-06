//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestValidationRecheckPreservesAttemptAndRequiresCompletedProducer(t *testing.T) {
	for _, completed := range []bool{false, true} {
		s, w := validationConfigFixture(t)
		w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
		w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Outcome: "completed", Blocker: "control timeout"})
		attempt := w.Tasks[0].Attempts[0].ID
		w.Tasks[0].AutoValidation = &AutomaticValidation{State: "blocked", Attempt: attempt, PolicyDigest: "old"}
		raw, _ := json.Marshal(w)
		if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
			t.Fatal(err)
		}
		if completed {
			a := Agent{ID: "producer", TaskID: "t1", WorkID: w.ID, Attempt: attempt, Status: "completed"}
			body, _ := json.Marshal(a)
			if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, body, []byte(`{}`)); err != nil {
				t.Fatal(err)
			}
		}
		change := automaticChange(w, []string{"go", "test", "./..."})
		change.RecheckCompleted = true
		preview, err := s.previewValidationPolicy(w.ID, change)
		if !completed {
			if err == nil {
				t.Fatal("recheck preview without completed producer")
			}
			unchanged, getErr := s.get(w.ID)
			if getErr != nil || unchanged.Revision != w.Revision || unchanged.Tasks[0].Status != "blocked" {
				t.Fatal("refused preview mutated the task", getErr)
			}
			continue
		}
		if err != nil {
			t.Fatal(err)
		}
		if dir := os.Getenv("SWARM_T2_RECHECK_EVIDENCE"); dir != "" {
			exportRoot := filepath.Join(dir, "base-store", ".swarm")
			if err = os.MkdirAll(exportRoot, 0700); err != nil {
				t.Fatal(err)
			}
			exportDB := filepath.Join(exportRoot, "state.db")
			if err = os.Remove(exportDB); err != nil && !os.IsNotExist(err) {
				t.Fatal(err)
			}
			if _, err = s.db.Exec("VACUUM INTO ?", exportDB); err != nil {
				t.Fatal(err)
			}
			if err = os.WriteFile(filepath.Join(dir, "work-id.txt"), []byte(w.ID), 0600); err != nil {
				t.Fatal(err)
			}
		}
		change.EventID = "recheck"
		change.PreviewToken = preview.Token
		got, err := s.applyValidationPolicy(w.ID, change)
		if err != nil {
			t.Fatal(err)
		}
		if got.Tasks[0].Status != "submitted" || len(got.Tasks[0].Attempts) != 1 || got.Tasks[0].Attempts[0].ID != attempt || got.Tasks[0].AutoValidation != nil || got.Tasks[0].Gate != nil {
			t.Fatal("recheck altered attempts or accepted stale proof")
		}
	}
}

func TestValidationRecheckCLIUsesSharedPreviewApplyContract(t *testing.T) {
	s, w := validationConfigFixture(t)
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Outcome: "completed", Blocker: "control failed"})
	attempt := latestAttemptID(&w.Tasks[0])
	w.Tasks[0].AutoValidation = &AutomaticValidation{State: "blocked", Attempt: attempt, PolicyDigest: "old-policy"}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	producer := Agent{ID: "producer-cli", TaskID: "t1", WorkID: w.ID, Attempt: attempt, Status: "completed"}
	body, _ := json.Marshal(producer)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", producer.ID, w.ID, producer.TaskID, s.root, producer.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	change := automaticChange(w, []string{"go", "version"})
	raw, _ = json.Marshal(change)
	input := validationInput(t, s.root, raw)
	var out, errs bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "validation", "recheck-preview", w.ID, "--task", "t1", "--input", input}, &out, &errs); code != 0 {
		t.Fatalf("recheck preview CLI: %d %s", code, errs.String())
	}
	var preview ValidationPolicyPreview
	if err := json.Unmarshal(out.Bytes(), &preview); err != nil || preview.Token == "" {
		t.Fatalf("invalid recheck preview: %s %v", out.String(), err)
	}
	change.PreviewToken, change.EventID = preview.Token, "cli-recheck"
	raw, _ = json.Marshal(change)
	input = validationInput(t, s.root, raw)
	out.Reset()
	errs.Reset()
	if code := run([]string{"--root", s.root, "--json", "validation", "recheck-apply", w.ID, "--task", "t1", "--input", input}, &out, &errs); code != 0 {
		t.Fatalf("recheck apply CLI: %d %s", code, errs.String())
	}
	got, err := s.get(w.ID)
	if err != nil || got.Tasks[0].Status != "submitted" || len(got.Tasks[0].Attempts) != 1 || got.Tasks[0].AutoValidation != nil {
		t.Fatalf("CLI did not recheck existing result: %+v %v", got.Tasks[0], err)
	}
}

func TestValidationRecheckPreviewRefusesUnchangedPrecondition(t *testing.T) {
	s, w := validationConfigFixture(t)
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Outcome: "completed", Blocker: "control failed"})
	attempt := w.Tasks[0].Attempts[0].ID
	change := automaticChange(w, []string{"go", "version"})
	w.Tasks[0].ValidationPolicy = change.Policy
	w.Tasks[0].AutoValidation = &AutomaticValidation{State: "blocked", Attempt: attempt, PolicyDigest: validationPolicyDigest(*change.Policy)}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	a := Agent{ID: "producer", TaskID: "t1", WorkID: w.ID, Attempt: attempt, Status: "completed"}
	body, _ := json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	change.RecheckCompleted = true
	if _, err := s.previewValidationPolicy(w.ID, change); err == nil || !strings.Contains(err.Error(), "corrigée différente") {
		t.Fatalf("unchanged precondition received a preview: %v", err)
	}
}

func TestValidationRecheckHasExplicitWebAndCLIEntrypoints(t *testing.T) {
	for _, check := range []struct {
		path string
		want []string
	}{
		{"web/mission.js", []string{"Recontrôler le résultat", "recheck_completed"}},
		{"main.go", []string{"recheck-preview", "recheck-apply", "RecheckCompleted = true"}},
	} {
		data, err := os.ReadFile(check.path)
		if err != nil {
			t.Fatal(err)
		}
		for _, want := range check.want {
			if !strings.Contains(string(data), want) {
				t.Fatalf("%s does not expose %q", check.path, want)
			}
		}
	}
}

func TestValidationRecheckWebActionUsesSharedPreviewApplyContract(t *testing.T) {
	s, w := validationConfigFixture(t)
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Outcome: "completed", Blocker: "control failed"})
	attempt := latestAttemptID(&w.Tasks[0])
	w.Tasks[0].AutoValidation = &AutomaticValidation{State: "blocked", Attempt: attempt, PolicyDigest: "old-policy"}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	producer := Agent{ID: "producer-web", TaskID: "t1", WorkID: w.ID, Attempt: attempt, Status: "completed"}
	body, _ := json.Marshal(producer)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", producer.ID, w.ID, producer.TaskID, s.root, producer.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	change := automaticChange(w, []string{"go", "version"})
	change.RecheckCompleted = true
	value, err := s.webAction(webRequest{Kind: "validation-policy-preview", Work: w.ID, Task: "t1", Revision: w.Revision, ValidationPolicy: change})
	preview, ok := value.(ValidationPolicyPreview)
	if err != nil || !ok || preview.Token == "" || !strings.Contains(strings.Join(preview.Effects, " "), "Recontrôle explicitement demandé") {
		t.Fatalf("aperçu web recheck invalide: %#v %v", value, err)
	}
	change.PreviewToken = preview.Token
	value, err = s.webAction(webRequest{Kind: "validation-policy-apply", Work: w.ID, Task: "t1", Revision: w.Revision, Event: "web-recheck", ValidationPolicy: change})
	updated, ok := value.(Work)
	if err != nil || !ok {
		t.Fatalf("application web recheck: %#v %v", value, err)
	}
	task := updated.Tasks[0]
	if task.Status != "submitted" || len(task.Attempts) != 1 || task.Attempts[0].ID != attempt || task.AutoValidation != nil || task.Gate != nil {
		t.Fatalf("web recheck did not preserve producer result or invalidate stale proof: %+v", task)
	}
	var producers int
	if err = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=?", w.ID, "t1").Scan(&producers); err != nil || producers != 1 {
		t.Fatalf("web recheck created a producer: count=%d err=%v", producers, err)
	}
}
