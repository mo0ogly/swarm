package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestTaskDefinitionEditPersistsAndReplays(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "two", Deliverable: "report", Criteria: []string{"old"}, Depends: []string{"t1"}})
	r := Request{Schema: 1, EventID: newID("edit-"), Revision: w.Revision, ID: "t2", Title: "new", Deliverable: "new report", Criteria: []string{"a", "b"}, Depends: []string{}}
	got, e := s.executeRequest(w.ID, "task.update", r)
	if e != nil {
		t.Fatal(e)
	}
	again, e := s.executeRequest(w.ID, "task.update", r)
	if e != nil {
		t.Fatal(e)
	}
	if got.Revision != again.Revision {
		t.Fatal("replay changed revision")
	}
	task, _ := again.task("t2")
	if task.Title != "new" || task.Deliverable != "new report" || len(task.Depends) != 0 || len(task.Criteria) != 2 || task.Status != "todo" {
		t.Fatalf("%+v", task)
	}
	ev, e := s.events(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	var recorded Request
	if e = json.Unmarshal(ev[len(ev)-1].Payload, &recorded); e != nil {
		t.Fatal(e)
	}
	if recorded.Depends == nil {
		t.Fatal("explicit dependency removal lost from audit")
	}
	r.EventID = newID("stale-")
	if _, e = s.executeRequest(w.ID, "task.update", r); e == nil {
		t.Fatal("stale revision accepted")
	}
}
func TestTaskDefinitionRejectsInvalidGraphAtomically(t *testing.T) {
	for _, tc := range []Request{{Depends: []string{"t1"}}, {Depends: []string{"missing"}}, {Depends: []string{"t2", "t2"}}, {Depends: []string{"t2"}}, {Criteria: []string{}}, {Criteria: []string{" "}}, {Title: " "}} {
		s := storeTest(t)
		w := taskTest(t, s, createTest(t, s))
		w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "two", Deliverable: "report", Criteria: []string{"ok"}, Depends: []string{"t1"}})
		tc.Schema = 1
		tc.EventID = newID("invalid-")
		tc.Revision = w.Revision
		tc.ID = "t1"
		if _, e := s.executeRequest(w.ID, "task.update", tc); e == nil {
			t.Fatalf("accepted %+v", tc)
		}
		got, _ := s.get(w.ID)
		if !reflect.DeepEqual(w, got) {
			t.Fatal("failed edit changed persisted state")
		}
	}
}
func TestTaskDefinitionInvalidatesGateAndRebuildsChecks(t *testing.T) {
	w := Work{Tasks: []Task{{ID: "t1", Status: "todo", Title: "one", Deliverable: "r", Criteria: []string{"old"}, Gate: &GateRecord{}, Override: &ManualOverride{}, Revalidation: &Revalidation{}, PlanChecks: map[string]string{"plan-criterion-9": "validation"}}}}
	if e := updateTaskDefinition(&w, &w.Tasks[0], Request{Criteria: []string{"new"}}); e != nil {
		t.Fatal(e)
	}
	got := w.Tasks[0]
	if got.Gate != nil || got.Override != nil || got.Revalidation != nil || len(got.PlanChecks) != 4 || got.PlanChecks["plan-criterion-1"] != "validation" {
		t.Fatalf("stale validation %+v", got)
	}
	for _, status := range []string{"running", "submitted", "accepted", "waived", "abandoned"} {
		w.Tasks[0].Status = status
		if e := updateTaskDefinition(&w, &w.Tasks[0], Request{Title: "changed"}); e == nil {
			t.Fatalf("edited %s", status)
		}
	}
}

func TestTaskDefinitionActiveAgentGuard(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	a := Agent{ID: "agent-queued", TaskID: "t1", Status: "queued"}
	raw, _ := json.Marshal(a)
	if _, e := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, raw, []byte("{}")); e != nil {
		t.Fatal(e)
	}
	r := Request{Schema: 1, EventID: newID("edit-"), Revision: w.Revision, ID: "t1", Title: "changed"}
	if _, e := s.executeRequest(w.ID, "task.update", r); e == nil {
		t.Fatal("active agent contract overwritten")
	}
	// Owner-only metadata remains editable without invalidating the definition.
	r.Title = ""
	r.Owner = "reviewer"
	r.EventID = newID("owner-")
	if _, e := s.executeRequest(w.ID, "task.update", r); e != nil {
		t.Fatal(e)
	}
}
func TestTaskDefinitionLimitsAndMetadata(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", MaxAttempts: 2, MaxToolCalls: 70})
	if w.Tasks[0].PlanMaxAttempts != 2 || w.Tasks[0].PlanToolLimit != 70 {
		t.Fatal("limits not stored")
	}
	for _, r := range []Request{{MaxAttempts: 4}, {MaxToolCalls: 101}, {MaxAttempts: -1}, {MaxToolCalls: -1}, {Title: "changed", Status: "running"}} {
		r.Schema = 1
		r.EventID = newID("invalid-")
		r.Revision = w.Revision
		r.ID = "t1"
		if _, e := s.executeRequest(w.ID, "task.update", r); e == nil {
			t.Fatalf("accepted %+v", r)
		}
	}
}

func TestWorkDefinitionRevision(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	r := Request{Schema: 1, EventID: newID("work-edit-"), Revision: w.Revision, Objective: "Corrected objective", Criteria: []string{"new criterion"}}
	got, e := s.executeRequest(w.ID, "work.update", r)
	if e != nil {
		t.Fatal(e)
	}
	if got.Objective != r.Objective || got.Tasks[0].ID != "t1" || got.Revision != w.Revision+1 {
		t.Fatal("bad work edit")
	}
	got = applyTest(t, s, got, "task.update", Request{ID: "t1", Status: "running"})
	r.Revision = got.Revision
	r.EventID = newID("blocked-")
	if _, e = s.executeRequest(w.ID, "work.update", r); e == nil {
		t.Fatal("edited active work")
	}
}

func TestTaskDefinitionCLIAndWebAction(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	input := filepath.Join(s.root, "edit.json")
	raw, _ := json.Marshal(Request{Schema: 1, EventID: newID("cli-"), Revision: w.Revision, ID: "t1", Title: "CLI corrected", Criteria: []string{"proof"}, Depends: []string{}})
	if e := os.WriteFile(input, raw, 0600); e != nil {
		t.Fatal(e)
	}
	var out, errOut bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "task", "update", w.ID, "--input", input}, &out, &errOut); code != 0 {
		t.Fatalf("%d %s", code, errOut.String())
	}
	got, _ := s.get(w.ID)
	if got.Tasks[0].Title != "CLI corrected" {
		t.Fatal("CLI edit lost")
	}
	if _, e := s.webAction(webRequest{Kind: "task", Work: w.ID, Task: "t1", Event: newID("web-"), Revision: got.Revision, Request: Request{Title: "Web corrected"}}); e != nil {
		t.Fatal(e)
	}
	got, _ = s.get(w.ID)
	if got.Tasks[0].Title != "Web corrected" {
		t.Fatal("web edit lost")
	}
	raw, _ = json.Marshal(Request{Schema: 1, EventID: newID("work-cli-"), Revision: got.Revision, Objective: "New work objective"})
	if e := os.WriteFile(input, raw, 0600); e != nil {
		t.Fatal(e)
	}
	if code := run([]string{"--root", s.root, "--json", "work", "update", w.ID, "--input", input}, &out, &errOut); code != 0 {
		t.Fatalf("%d %s", code, errOut.String())
	}
	got, _ = s.get(w.ID)
	if got.Objective != "New work objective" {
		t.Fatal("work CLI edit lost")
	}
}

func TestTaskDefinitionRequestEncoding(t *testing.T) {
	for _, deps := range [][]string{nil, {"t1"}, {}} {
		r := Request{Schema: 1, EventID: "event", Revision: 1, Depends: deps}
		b, e := json.Marshal(r)
		if e != nil {
			t.Fatal(e)
		}
		var decoded Request
		if e = json.Unmarshal(b, &decoded); e != nil {
			t.Fatal(e)
		}
		if !reflect.DeepEqual(deps, decoded.Depends) {
			t.Fatalf("dependency presence lost: %s", b)
		}
		if deps == nil {
			want := `{"schema_version":1,"event_id":"event","expected_revision":1}`
			if string(b) != want {
				t.Fatalf("legacy envelope changed: %s", b)
			}
		}
	}
	w := Work{Tasks: []Task{{ID: "t1", Status: "todo", Title: "one", Deliverable: "r", Criteria: []string{"old"}, PlanChecks: map[string]string{"security-review": "delivery", "plan-criterion-9": "validation"}}}}
	if e := updateTaskDefinition(&w, &w.Tasks[0], Request{Criteria: []string{"new"}}); e != nil {
		t.Fatal(e)
	}
	if w.Tasks[0].PlanChecks["security-review"] != "delivery" {
		t.Fatal("custom mandatory gate lost")
	}
}
