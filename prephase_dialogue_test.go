//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func prepDialogueFixture(t *testing.T, script string) (*Store, Preparation, PreparationSend) {
	t.Helper()
	s := storeTest(t)
	prepMethods(t, s)
	p := prepCreate(t, s)
	p = prepSave(t, s, p, "besoin", "Un cockpit simple.")
	dir := t.TempDir()
	command := filepath.Join(dir, "claude")
	if e := os.WriteFile(command, []byte("#!/bin/sh\n"+script), 0700); e != nil {
		t.Fatal(e)
	}
	ps := Providers{Schema: 1, Providers: map[string]Provider{"test": {Command: command}}}
	b, _ := json.Marshal(ps)
	if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), b, 0600); e != nil {
		t.Fatal(e)
	}
	cap := s.preparationCapabilities()[0]
	r := PreparationSend{Version: 1, ID: p.ID, Event: "message-1", Revision: p.Revision, Provider: "test", Capability: cap.Hash, Message: "Propose un brief."}
	return s, p, r
}

const prepReplyScript = "cat >/dev/null\nprintf '%s\\n' '{\"type\":\"result\",\"result\":\"{\\\"message\\\":\\\"Voici une proposition.\\\",\\\"brief\\\":\\\"# Brief\\\\nObjectif explicite.\\\"}\"}'\n"

func TestPreparationDialogueReplyReplayAndExplicitProposal(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	replay, e := s.sendPreparation(r)
	if e != nil || replay.ID != turn.ID {
		t.Fatal(replay, e)
	}
	other := r
	other.Event = "message-2"
	if _, e = s.sendPreparation(other); e == nil {
		t.Fatal("second active turn allowed")
	}
	s.runPreparationTurn(turn)
	got, e := s.preparationTurn(turn.ID)
	if e != nil || got.Answer == nil || got.Status != "answered" {
		t.Fatalf("%+v %v", got, e)
	}
	unchanged, _ := s.preparation(p.ID)
	if unchanged.Revision != p.Revision || unchanged.Documents["brief"].Text != "" {
		t.Fatal("reply mutated document")
	}
	req := prepRequest(p, "use-proposal")
	req.Turn = turn.ID
	applied, e := s.preparationCommand(req)
	if e != nil || applied.Documents["brief"].Text != got.Answer.Brief || applied.Brief != nil {
		t.Fatalf("%+v %v", applied, e)
	}
	replayP, e := s.preparationCommand(req)
	if e != nil || replayP.Revision != applied.Revision {
		t.Fatal(e)
	}
	ts, e := s.preparationDialogue(p.ID)
	if e != nil || len(ts) != 1 || !ts[0].Stale || ts[0].Prompt != "" {
		t.Fatal(ts, e)
	}
	var agents int
	_ = s.db.QueryRow("SELECT count(*) FROM agents").Scan(&agents)
	if agents != 0 {
		t.Fatal("agents created")
	}
}
func TestPreparationDialogueLateProposalAndMethodDrift(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	p = prepSave(t, s, p, "besoin", "Nouveau besoin.")
	s.runPreparationTurn(turn)
	req := prepRequest(p, "use-proposal")
	req.Turn = turn.ID
	if _, e = s.preparationCommand(req); e == nil {
		t.Fatal("stale proposal applied")
	}
	ts, _ := s.preparationDialogue(p.ID)
	if !ts[0].Stale {
		t.Fatal("not stale")
	}
	r.Event = "next"
	r.Revision = p.Revision
	os.WriteFile(filepath.Join(s.root, ".claude/skills/apex/SKILL.md"), []byte("Méthode modifiée"), 0600)
	if _, e = s.sendPreparation(r); e == nil {
		t.Fatal("method drift allowed")
	}
}
func TestPreparationDialoguePendingRunningStopAndDeadline(t *testing.T) {
	s, _, r := prepDialogueFixture(t, "cat >/dev/null\nsleep 10\n"+prepReplyScript)
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	stopped, e := s.cancelPreparation(r.ID, turn.ID)
	if e != nil || stopped.Status != "interrupted" {
		t.Fatal(stopped, e)
	}
	s.runPreparationTurn(turn)
	got, _ := s.preparationTurn(turn.ID)
	if got.Status != "interrupted" || got.Answer != nil {
		t.Fatal(got)
	}
	r.Event = "run"
	turn, e = s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	done := make(chan struct{})
	go func() { s.runPreparationTurn(turn); close(done) }()
	deadline := time.Now().Add(3 * time.Second)
	for time.Now().Before(deadline) {
		got, _ = s.preparationTurn(turn.ID)
		if got.PID > 0 {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if got.PID == 0 {
		t.Fatal("not running")
	}
	// Simulate legacy JSON lagging behind SQL; cancel must use SQL status.
	stale := got
	stale.Status = "pending"
	raw, _ := json.Marshal(stale)
	s.db.Exec("UPDATE preparation_turns SET body=? WHERE id=?", raw, turn.ID)
	stopped, e = s.cancelPreparation(r.ID, turn.ID)
	if e != nil || stopped.Status != "stopping" {
		t.Fatal(stopped, e)
	}
	select {
	case <-done:
	case <-time.After(3 * time.Second):
		t.Fatal("stop timed out")
	}
	got, _ = s.preparationTurn(turn.ID)
	if got.Status != "interrupted" || got.Answer != nil {
		t.Fatal(got)
	}
	r.Event = "timeout"
	turn, e = s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	s.runPreparationTurnWithin(turn, 100*time.Millisecond)
	got, _ = s.preparationTurn(turn.ID)
	if got.Status != "failed" || !strings.Contains(got.Error, "Délai") {
		t.Fatal(got)
	}
}
func TestPreparationDialogueProviderChangeAndOrphan(t *testing.T) {
	s, _, r := prepDialogueFixture(t, prepReplyScript)
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	ps, _ := s.providers()
	p := ps.Providers["test"]
	p.Args = []string{"--model", "changed"}
	ps.Providers["test"] = p
	b, _ := json.Marshal(ps)
	os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), b, 0600)
	s.runPreparationTurn(turn)
	got, _ := s.preparationTurn(turn.ID)
	if got.Status != "failed" || !strings.Contains(got.Error, "modifié") {
		t.Fatal(got)
	}
	r.Event = "changed"
	if _, e = s.sendPreparation(r); e == nil {
		t.Fatal("stale capability accepted")
	}
	r.Capability = s.preparationCapabilities()[0].Hash
	turn, e = s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	turn.CreatedAt = time.Now().Add(-time.Minute).Format(time.RFC3339Nano)
	b, _ = json.Marshal(turn)
	s.db.Exec("UPDATE preparation_turns SET body=? WHERE id=?", b, turn.ID)
	ts, e := s.preparationDialogue(r.ID)
	if e != nil || ts[1].active() {
		t.Fatal(ts, e)
	}
}

func TestPreparationDialogueLimitsAndSingleClaim(t *testing.T) {
	s, _, r := prepDialogueFixture(t, prepReplyScript)
	r.Message = strings.Repeat("x", 8001)
	if _, e := s.sendPreparation(r); e == nil {
		t.Fatal("message size unchecked")
	}
	r.Message = "Un message"
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	done := make(chan struct{}, 2)
	for i := 0; i < 2; i++ {
		go func() { s.runPreparationTurn(turn); done <- struct{}{} }()
	}
	<-done
	<-done
	got, e := s.preparationTurn(turn.ID)
	if e != nil || got.Status != "answered" {
		t.Fatal(got, e)
	}
	r.Event = "invalid"
	turn, e = s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.finishPreparationTurn(turn, `{"message":"réponse","brief":"","extra":"injection"}`, nil, ""); e != nil {
		t.Fatal(e)
	}
	got, _ = s.preparationTurn(turn.ID)
	if got.Status != "failed" || got.Answer != nil {
		t.Fatal("unexpected field accepted")
	}
	p, _ := s.preparation(r.ID)
	m, _ := s.preparationMethod(p.Method)
	turns := make([]PreparationTurn, 20)
	for i := range turns {
		turns[i].Question = strings.Repeat("q", 8000)
	}
	if _, e = s.preparationPrompt(p, m, turns, "message"); e == nil {
		t.Fatal("context silently truncated")
	}
}

func TestPreparationDeterministicActionsDoNotInvokeProvider(t *testing.T) {
	s := storeTest(t)
	prepMethods(t, s)
	dir := t.TempDir()
	counter := filepath.Join(dir, "provider-called")
	command := filepath.Join(dir, "claude")
	if e := os.WriteFile(command, []byte("#!/bin/sh\nprintf called >>\"$SWARM_TEST_COUNTER\"\n"), 0700); e != nil {
		t.Fatal(e)
	}
	t.Setenv("SWARM_TEST_COUNTER", counter)
	providers := Providers{Schema: 1, Providers: map[string]Provider{
		"counter": {Command: command, Env: []string{"SWARM_TEST_COUNTER"}},
	}}
	raw, _ := json.Marshal(providers)
	if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); e != nil {
		t.Fatal(e)
	}

	// Catalogue and capability inspection, document mutations, adoption and
	// validation are local operations. Only sendPreparation followed by the
	// runtime is allowed to execute the configured provider.
	if len(s.preparationMethods()) != 3 || len(s.preparationCapabilities()) != 1 {
		t.Fatal("catalogue or capability unavailable")
	}
	p := prepCreate(t, s)
	p = prepSave(t, s, p, "besoin", "Besoin déterministe")
	p = prepSave(t, s, p, "brief", "Brief déterministe")
	req := prepRequest(p, "adopt-brief")
	req.Hash = p.Documents["brief"].Hash
	var e error
	p, e = s.preparationCommand(req)
	if e != nil {
		t.Fatal(e)
	}
	plan, _ := json.Marshal(fixturePlan())
	p = prepSave(t, s, p, "plan", string(plan))
	req = prepRequest(p, "validate-plan")
	req.Hash = p.Documents["plan"].Hash
	if _, e = s.preparationCommand(req); e != nil {
		t.Fatal(e)
	}
	if _, e = s.preparationDialogue(p.ID); e != nil {
		t.Fatal(e)
	}
	if _, e = os.Stat(counter); !os.IsNotExist(e) {
		t.Fatalf("deterministic action invoked provider: %v", e)
	}
}

func TestPreparationMethodCatalogueStaysInPrephase(t *testing.T) {
	s := storeTest(t)
	prepMethods(t, s)
	want := map[string]string{
		"apex":       "Analyze,Plan",
		"ks-feature": "Cadrage,Plan",
		"audit-pdca": "Plan",
	}
	methods := s.preparationMethods()
	if len(methods) != len(want) {
		t.Fatalf("unexpected method catalogue: %+v", methods)
	}
	for _, method := range methods {
		if !method.Available || strings.Join(method.Phases, ",") != want[method.ID] {
			t.Fatalf("method escaped preparation phases: %+v", method)
		}
		if strings.Contains(strings.Join(method.Phases, ","), "Execute") || strings.Contains(strings.Join(method.Phases, ","), "Do") || strings.Contains(strings.Join(method.Phases, ","), "Check") || strings.Contains(strings.Join(method.Phases, ","), "Act") {
			t.Fatalf("execution phase exposed by catalogue: %+v", method)
		}
	}
}
func TestPreparationDialogueV7Backup(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	if _, e := s.db.Exec("DROP TRIGGER preparation_launch_insert; DROP TRIGGER preparation_launch_update; DROP TABLE preparation_launch_locks; DROP TABLE preparation_turns; PRAGMA user_version=7"); e != nil {
		t.Fatal(e)
	}
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	got, e := other.preparation(p.ID)
	if e != nil || got.Revision != p.Revision {
		t.Fatal(got, e)
	}
	backups, _ := filepath.Glob(filepath.Join(s.root, ".swarm/state-pre-v8-*.db"))
	if len(backups) != 1 {
		t.Fatal(backups)
	}
}
