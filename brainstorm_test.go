//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestBrainstormDialoguePersistsAndFeedsAdoptedBrief(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Brainstorm = true
	r.TaskID = ""
	r.Instruction = "Comparer deux options pour préparer les agents."
	a, created, e := s.prepare(w.ID, r)
	if e != nil || !created || a.Role != "planner" {
		t.Fatal(a, e)
	}
	duplicate, created, e := s.prepare(w.ID, r)
	if e != nil || created || duplicate.ID != a.ID {
		t.Fatal("duplicate launch", e)
	}
	w, _ = s.get(w.ID)
	if len(w.Tasks) != 2 {
		t.Fatal("missing planning task")
	}
	if !strings.Contains(a.Prompt, "Analyze et Plan") || !strings.Contains(a.Prompt, "Ne pas attribuer un score arbitraire") {
		t.Fatal("missing APEX contract")
	}
	second := r
	second.EventID = newID("agent-")
	second.Revision = w.Revision
	if _, _, e = s.prepare(w.ID, second); e == nil {
		t.Fatal("parallel conversation allowed")
	}
	unchanged, _ := s.get(w.ID)
	if len(unchanged.Tasks) != 2 {
		t.Fatal("orphan task on refused launch")
	}
	os.Mkdir(filepath.Join(s.root, "docs"), 0700)
	report := "# Options\nA ou B. Quel périmètre souhaitez-vous ?\n"
	path := "docs/" + a.TaskID + "-brainstorm.md"
	os.WriteFile(filepath.Join(s.root, path), []byte(report), 0600)
	zero := 0
	if e = s.finishAgent(a, "completed", "Réponse terminée", &zero); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	if len(task.Answers) != 1 || task.Answers[0].Text != report {
		t.Fatal("answer history missing")
	}
	if task.Response != report || task.Status == "accepted" {
		t.Fatal("response persistence or false acceptance")
	}
	if _, e = s.adoptBrief(w.ID, a.TaskID, path, "wrong", newID("adopt-"), w.Revision); e == nil {
		t.Fatal("unreviewed change adopted")
	}
	w, e = s.adoptBrief(w.ID, a.TaskID, path, hash([]byte(report)), newID("adopt-"), w.Revision)
	if e != nil {
		t.Fatal(e)
	}
	second.Revision = w.Revision
	second.Instruction = "Option B, périmètre limité."
	b, _, e := s.prepare(w.ID, second)
	if e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(b.Prompt, report) || !strings.Contains(b.Prompt, r.Instruction) || !strings.Contains(b.Prompt, second.Instruction) {
		t.Fatal("dialogue context lost")
	}
	os.WriteFile(filepath.Join(s.root, path), []byte("changed after adoption"), 0600)
	w, _ = s.get(w.ID)
	if w.PlanningBrief.Text != report {
		t.Fatal("brief not frozen")
	}
	s.finishAgent(b, "interrupted", "test stop", nil)
	w, _ = s.get(w.ID)
	normal := r
	normal.Brainstorm = false
	normal.EventID = newID("worker-")
	normal.TaskID = "t1"
	normal.Revision = w.Revision
	worker, _, e := s.prepare(w.ID, normal)
	if e != nil || !strings.Contains(worker.Prompt, report) {
		t.Fatal("worker missing adopted brief", e)
	}
	dest := filepath.Join(t.TempDir(), "dialogue.zip")
	if e = s.export(w.ID, dest); e != nil {
		t.Fatal(e)
	}
	other := storeTest(t)
	restored, e := other.importBundle(dest)
	if e != nil {
		t.Fatal(e)
	}
	restoredTask, _ := restored.task(a.TaskID)
	if restored.PlanningBrief.Text != report || restoredTask.Response != report || len(restoredTask.Answers) != 1 {
		t.Fatal("archive lost conversation")
	}

}

func TestPublicProviderReplyOnly(t *testing.T) {
	for _, data := range []map[string]any{
		{"type": "result", "result": "Réponse Claude"},
		{"subtype": "success", "result": "Réponse Claude"},
		{"type": "item.completed", "item": map[string]any{"type": "agent_message", "text": "Réponse Codex"}},
	} {
		if providerReply(data) == "" {
			t.Fatal("final reply lost")
		}
	}
	if providerReply(map[string]any{"type": "item.completed", "item": map[string]any{"type": "reasoning", "text": "private"}}) != "" {
		t.Fatal("reasoning captured")
	}
}
