//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func memorySeed(t *testing.T, s *Store, w Work) Work {
	t.Helper()
	v, e := s.mutate(w.ID, "fixture", newID("e-"), w.Revision, []byte(`{}`), func(w *Work) error {
		for i := 0; i < 6; i++ {
			w.Tasks = append(w.Tasks, Task{ID: fmt.Sprintf("old-%d", i), Title: "Dialogue", Brainstorm: true, Question: fmt.Sprintf("Décision numéro %d", i), Response: fmt.Sprintf("Réponse %d", i), Answers: []BrainstormAnswer{{Attempt: fmt.Sprintf("a%d", i), Text: fmt.Sprintf("Réponse %d", i), At: now()}}, Status: "blocked"})
		}
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	return v
}
func TestContextPreviewFreezesAndRejectsDrift(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	w = memorySeed(t, s, w)
	r.Brainstorm = true
	r.TaskID = ""
	r.Revision = w.Revision
	r.Instruction = "Reprendre notre décision"
	r.References = []DialogueRef{dialogueHits(w)[0].Ref}
	a, created, e := s.prepareLaunch(w.ID, r, true)
	if e != nil || created {
		t.Fatal(e)
	}
	if len(a.Context.Included) != 4 || len(a.Context.Excluded) != 2 || !strings.Contains(a.Prompt, "Réponse 0") {
		t.Fatal(a.Context)
	}
	untouched, _ := s.get(w.ID)
	if untouched.Revision != w.Revision || len(untouched.Tasks) != len(w.Tasks) {
		t.Fatal("preview mutated state")
	}
	bad := r
	bad.ContextHash = "bad"
	if _, _, e = s.prepare(w.ID, bad); e == nil {
		t.Fatal("stale hash accepted")
	}
	r.ContextHash = a.Context.SHA256
	sent, created, e := s.prepare(w.ID, r)
	if e != nil || !created || sent.Prompt != a.Prompt || sent.Context.SHA256 != hash([]byte(sent.Prompt)) {
		t.Fatal(e)
	}
	stored, e := s.agent(sent.ID)
	if e != nil || stored.Context.SHA256 != r.ContextHash {
		t.Fatal("manifest lost", e)
	}
	if e = s.finishAgent(sent, "interrupted", "Fin de fixture", nil); e != nil {
		t.Fatal(e)
	}
	bundle := filepath.Join(t.TempDir(), "memory.zip")
	if e = s.export(w.ID, bundle); e != nil {
		t.Fatal(e)
	}
	other := storeTest(t)
	restored, e := other.importBundle(bundle)
	if e != nil {
		t.Fatal(e)
	}
	importedTask, e := restored.task(sent.TaskID)
	if e != nil || len(importedTask.Contexts) != 1 || importedTask.Contexts[0].Manifest.SHA256 != sent.Context.SHA256 || importedTask.Contexts[0].Prompt != sent.Prompt {
		t.Fatal("context archive mismatch", e)
	}

}
func TestDialogueSearchScopeVersionsAndBounds(t *testing.T) {
	s := storeTest(t)
	w := memorySeed(t, s, createTest(t, s))
	w.Tasks[0].Answers = append(w.Tasks[0].Answers, BrainstormAnswer{Attempt: "retry", Text: "Ancienne résolution spéciale"})
	hits := searchDialogue(w, "decision", 0)
	if hits["total"].(int) != 7 {
		t.Fatal(hits)
	}
	if len(searchDialogue(w, "", 0)["hits"].([]DialogueHit)) != 0 {
		t.Fatal("empty query")
	}
	if len(searchDialogue(w, "introuvable", 0)["hits"].([]DialogueHit)) != 0 {
		t.Fatal("false match")
	}
	if len(searchDialogue(w, "decision", 100)["hits"].([]DialogueHit)) != 0 {
		t.Fatal("pagination")
	}
	ref := dialogueHits(w)[1].Ref
	if _, e := resolveDialogue(Work{}, ref); e == nil {
		t.Fatal("foreign work accepted")
	}
	ref.SHA256 = "changed"
	if _, e := resolveDialogue(w, ref); e == nil {
		t.Fatal("wrong source accepted")
	}
	if _, e := dialogueAttachments(w, make([]DialogueRef, 9)); e == nil {
		t.Fatal("unbounded context")
	}
}
func retexTestAction(t *testing.T, s *Store, w Work, kind string, r Retex, note string) Work {
	t.Helper()
	_, e := s.retexAction(webRequest{Kind: kind, Work: w.ID, Task: r.ID, Event: newID("e-"), Revision: w.Revision, Retex: r, Note: note})
	if e != nil {
		t.Fatal(kind, e)
	}
	w, e = s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	return w
}
func TestRetexTaskExportAndVerifiedGuide(t *testing.T) {
	s := storeTest(t)
	w := memorySeed(t, s, createTest(t, s))
	r := Retex{ID: "r-test", Source: dialogueHits(w)[0].Ref, Problem: "Comprendre la reprise", Nature: "hypothese", Proposal: "Afficher le contexte", Effort: "faible", Risk: "Contexte erroné", Criteria: "Aperçu identique au prompt", Proof: "proof.txt"}
	w = retexTestAction(t, s, w, "retex-save", r, "")
	w = retexTestAction(t, s, w, "retex-task", r, "")
	n := len(w.Tasks)
	w = retexTestAction(t, s, w, "retex-task", r, "")
	if len(w.Tasks) != n {
		t.Fatal("duplicate task")
	}
	if _, e := s.retexAction(webRequest{Kind: "retex-status", Work: w.ID, Task: r.ID, Event: newID("e-"), Revision: w.Revision, Note: "verifie"}); e == nil {
		t.Fatal("unproven lesson")
	}
	w = retexTestAction(t, s, w, "retex-export", r, "")
	path := filepath.Join(s.root, "docs/retex/swarm/r-test.md")
	if _, e := os.Stat(path); e != nil {
		t.Fatal(e)
	}
	os.WriteFile(path, []byte("external edits"), 0600)
	w = retexTestAction(t, s, w, "retex-export", r, "")
	if w.Retex[0].ExportError == "" {
		t.Fatal("overwrite not detected")
	}
	r.Nature = "fait"
	w = retexTestAction(t, s, w, "retex-qualify", r, "")
	id := w.Retex[0].Task
	raw := []byte(strings.ReplaceAll(string(fixture(t, s.root)), `"t1"`, `"`+id+`"`))
	ev, e := evaluate(raw, s.root, "delivery")
	if e != nil {
		t.Fatal(e)
	}
	w, e = s.mutate(w.ID, "fixture-gate", newID("e-"), w.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task(id)
		task.Status = "accepted"
		task.Gate = &GateRecord{Document: raw, Evaluation: ev, At: now()}
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	w = retexTestAction(t, s, w, "retex-status", r, "verifie")
	q := webRequest{Kind: "retex-guide-preview", Work: w.ID, Task: r.ID, Event: newID("e-"), Revision: w.Revision, Note: "Vérifier les sources avant reprise."}
	v, e := s.retexAction(q)
	if e != nil {
		t.Fatal(e)
	}
	digest := v.(map[string]any)["digest"].(string)
	os.WriteFile(filepath.Join(s.root, ".claude/field-guide/index.md"), []byte("# Modifié\n"), 0600)
	q.Kind = "retex-guide"
	q.ContextHash = digest
	if _, e = s.retexAction(q); e == nil {
		t.Fatal("guide drift accepted")
	}
	q.Kind = "retex-guide-preview"
	v, e = s.retexAction(q)
	if e != nil {
		t.Fatal(e)
	}
	q.ContextHash = v.(map[string]any)["digest"].(string)
	q.Kind = "retex-guide"
	if _, e = s.retexAction(q); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Retex[0].Lesson == "" {
		t.Fatal("lesson not tracked")
	}

	bundle := filepath.Join(t.TempDir(), "retex.zip")
	if e = s.export(w.ID, bundle); e != nil {
		t.Fatal(e)
	}
	other := storeTest(t)
	restoredWork, e := other.importBundle(bundle)
	if e != nil || len(restoredWork.Retex) != 1 || restoredWork.Retex[0].Source != r.Source {
		t.Fatal("RETEX archive mismatch", e)
	}
	os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600)
	q.Revision = w.Revision
	q.Kind = "retex-guide-preview"
	if _, e = s.retexAction(q); e == nil {
		t.Fatal("stale gate accepted")
	}
	// Portable work data preserves structured RETEX independently of derived Markdown.
	data, _ := json.Marshal(w)
	var restored Work
	if e = json.Unmarshal(data, &restored); e != nil || restored.Retex[0].Source != r.Source {
		t.Fatal(e)
	}
}
func BenchmarkDialogueSearch(b *testing.B) {
	w := Work{}
	for i := 0; i < 1000; i++ {
		w.Tasks = append(w.Tasks, Task{ID: fmt.Sprint(i), Brainstorm: true, Question: "Décision", Response: strings.Repeat("réponse ", 100)})
	}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		searchDialogue(w, "decision", 0)
	}
}

func TestGuideContractNestedBudgetLinksAndSymlinks(t *testing.T) {
	root := t.TempDir()
	dir, e := generatedDir(root, ".claude/field-guide")
	if e != nil {
		t.Fatal(e)
	}
	pending := map[string]string{"index.md": "# Guide\n\n- [Note](new.md)\n", "new.md": "# Leçon\n"}
	if _, e = validateGuide(root, dir, pending); e != nil {
		t.Fatal(e)
	}
	pending["new.md"] = "[Absent](missing.md)"
	if _, e = validateGuide(root, dir, pending); e == nil {
		t.Fatal("missing link accepted")
	}
	pending["new.md"] = "# Leçon\n"
	sub := filepath.Join(dir, "nested")
	os.Mkdir(sub, 0700)
	os.WriteFile(filepath.Join(sub, "long.md"), []byte(strings.Repeat("note\n", 201)), 0600)
	if _, e = validateGuide(root, dir, pending); e == nil {
		t.Fatal("nested budget ignored")
	}
	os.Remove(filepath.Join(sub, "long.md"))
	os.Symlink(root, filepath.Join(dir, "outside"))
	if _, e = validateGuide(root, dir, pending); e == nil {
		t.Fatal("symlink accepted")
	}
	if _, e = findRetex(&Work{Retex: []Retex{{ID: "../escape"}}}, "../escape"); e == nil {
		t.Fatal("imported traversal id accepted")
	}
}
