//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func actionTurnTest(t *testing.T, s *Store, w Work, op string) AssistTurn {
	t.Helper()
	ctx, e := s.pageContext(w.ID, PageCoordinates{PageID: "tasks", Selected: []string{w.Tasks[0].ID}})
	if e != nil {
		t.Fatal(e)
	}
	var choice PageAction
	for _, a := range ctx.Actions {
		if a.Operation == op && a.Available {
			choice = a
			break
		}
	}
	if choice.ID == "" {
		t.Fatalf("action missing %s %+v", op, ctx.Actions)
	}
	raw := answerFor(ctx, "mission_advice.v1", func(a *AssistantAnswer) {
		a.Interpretation = "Cette tâche prépare le projet.\nIl faut examiner le compte rendu."
		a.NextSteps = []AnswerStep{{ActionID: choice.ID, Why: "Examiner le résultat.", SourceIDs: []string{ctx.Facts[0].ID}}}
	})
	answer, refusal, _ := validateAssistantReply(raw, ctx, "mission_advice.v1")
	if refusal != nil {
		t.Fatal(refusal)
	}
	turn := AssistTurn{ID: newID("assist-"), WorkID: w.ID, Status: "completed", Coordinates: PageCoordinates{PageID: "tasks", Selected: []string{w.Tasks[0].ID}}, Context: ctx, TemplateID: "mission_advice.v1", Answer: answer}
	body, _ := json.Marshal(turn)
	if _, e = s.db.Exec("INSERT INTO assist_turns(id,work_id,created_at,status,body) VALUES(?,?,?,?,?)", turn.ID, w.ID, now(), turn.Status, body); e != nil {
		t.Fatal(e)
	}
	return turn
}
func TestAssistActionSubmitBoundAndReplay(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	path := filepath.Join(s.root, "docs", "t1.md")
	os.MkdirAll(filepath.Dir(path), 0700)
	os.WriteFile(path, []byte("Compte rendu à examiner"), 0600)
	turn := actionTurnTest(t, s, w, "task.submit")
	p, e := s.assistActionPlan(w.ID, turn.ID, 0)
	if e != nil {
		t.Fatal(e)
	}
	os.WriteFile(path, []byte("Rapport changé"), 0600)
	if _, e = s.applyAssistAction(w.ID, turn.ID, 0, p.ID); e == nil {
		t.Fatal("changed report accepted")
	}
	os.WriteFile(path, []byte("Compte rendu à examiner"), 0600)
	r, e := s.applyAssistAction(w.ID, turn.ID, 0, p.ID)
	if e != nil || r.Status != "done" {
		t.Fatal(r, e)
	}
	after, _ := s.get(w.ID)
	if after.Tasks[0].Status != "submitted" {
		t.Fatal(after.Tasks[0])
	}
	replay, e := s.applyAssistAction(w.ID, turn.ID, 0, p.ID)
	if e != nil || replay != r {
		t.Fatal(replay, e)
	}
	last, _ := s.get(w.ID)
	if last.Revision != after.Revision {
		t.Fatal("duplicate mutation")
	}
	if _, e = s.applyAssistAction(w.ID, turn.ID, 1, p.ID); e == nil {
		t.Fatal("receipt used for different step")
	}
	if _, e = s.assistActionPlan("other-work", turn.ID, 0); e == nil {
		t.Fatal("cross work accepted")
	}
}
func TestAssistActionDurablePendingAndImportRefused(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	os.MkdirAll(filepath.Join(s.root, "docs"), 0700)
	os.WriteFile(filepath.Join(s.root, "docs/t1.md"), []byte("rapport"), 0600)
	turn := actionTurnTest(t, s, w, "task.submit")
	p, e := s.assistActionPlan(w.ID, turn.ID, 0)
	if e != nil {
		t.Fatal(e)
	}
	r := AssistActionReceipt{ID: p.ID, Turn: turn.ID, Step: 0, Status: "pending", Message: "À vérifier"}
	raw, _ := json.Marshal(r)
	s.controlEvent(w.ID, "assist-action", string(raw))
	replay, e := s.applyAssistAction(w.ID, turn.ID, 0, p.ID)
	if e != nil || replay.Status != "pending" {
		t.Fatal(replay, e)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision {
		t.Fatal("uncertain execution replayed")
	}
	turn.Imported = true
	body, _ := json.Marshal(turn)
	s.db.Exec("UPDATE assist_turns SET body=? WHERE id=?", body, turn.ID)
	if _, e = s.assistActionPlan(w.ID, turn.ID, 0); e == nil {
		t.Fatal("import accepted")
	}
}
func TestMissionAdviceMustBeShort(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	ctx, _ := s.pageContext(w.ID, PageCoordinates{PageID: "tasks"})
	for _, bad := range []string{"Une seule ligne", strings.Repeat("x", 221) + "\nSuite", "Un\nDeux\nTrois\nQuatre"} {
		raw := answerFor(ctx, "mission_advice.v1", func(a *AssistantAnswer) { a.Interpretation = bad })
		if _, r, _ := validateAssistantReply(raw, ctx, "mission_advice.v1"); r == nil {
			t.Fatal("verbose summary accepted")
		}
	}
}

func TestAssistActionLaunchPreservesProfileAndProvider(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker", Instruction: "Consignes", Timeout: 123, Capture: true, Limits: &RunLimits{MaxToolCalls: 7}}
	if e := s.setProfile(w.ID, "", profile, w.Revision); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	turn := actionTurnTest(t, s, w, "task.start")
	p, e := s.assistActionPlan(w.ID, turn.ID, 0)
	if e != nil {
		t.Fatal(e)
	}
	if p.Launch.Timeout != 123 || !p.Launch.Capture || p.Launch.Limits.MaxToolCalls != 7 || p.Launch.ProviderDigest == "" {
		t.Fatal("profile lost", p.Launch)
	}
	providers, _ := s.providers()
	v := providers.Providers["fixture"]
	v.Args = append(v.Args, "changed")
	providers.Providers["fixture"] = v
	raw, _ := json.Marshal(providers)
	os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600)
	if _, _, e = s.prepareLaunch(w.ID, *p.Launch, true); e == nil {
		t.Fatal("changed provider accepted")
	}
}
