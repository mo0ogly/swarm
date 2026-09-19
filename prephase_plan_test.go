//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func prepAdopt(t *testing.T, s *Store, p Preparation) Preparation {
	t.Helper()
	p = prepSave(t, s, p, "brief", "Objectif : cadrer le besoin puis vérifier le plan.")
	r := prepRequest(p, "adopt-brief")
	r.Hash = p.Documents["brief"].Hash
	p, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	return p
}
func TestPreparationPlanProposalLifecycle(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	r.Target = "plan"
	if _, e := s.sendPreparation(r); e == nil {
		t.Fatal("plan without adopted brief")
	}
	p = prepAdopt(t, s, p)
	r.Revision = p.Revision
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(turn.Prompt, "CADRE OBLIGATOIRE DU PLAN") {
		t.Fatal("missing contract")
	}
	plan := fixturePlan()
	plan.Questions = []PlanQuestion{{Question: "Quel périmètre ?", Answer: ""}}
	raw, _ := json.Marshal(plan)
	answer, _ := json.Marshal(PreparationAnswer{Message: "Deux missions proposées.", Plan: string(raw)})
	if e = s.finishPreparationTurn(turn, string(answer), nil, ""); e != nil {
		t.Fatal(e)
	}
	turn, _ = s.preparationTurn(turn.ID)
	if turn.Status != "answered" || turn.PlanSummary.Dependencies != 1 || turn.PlanSummary.OpenQuestions != 1 {
		t.Fatal(turn)
	}
	unchanged, _ := s.preparation(p.ID)
	if unchanged.Revision != p.Revision || unchanged.PlanReady {
		t.Fatal("automatic adoption")
	}
	req := prepRequest(p, "use-proposal")
	req.Turn = turn.ID
	p, e = s.preparationCommand(req)
	if e != nil {
		t.Fatal(e)
	}
	if p.Documents["plan"].SourceTurn != turn.ID || p.Documents["plan"].BriefHash != p.Brief.Hash || p.PlanReady {
		t.Fatal(p)
	}
	req = prepRequest(p, "validate-plan")
	req.Hash = p.Documents["plan"].Hash
	if _, e = s.preparationCommand(req); e == nil {
		t.Fatal("unresolved question allowed")
	}
	plan.Questions[0].Answer = "docs uniquement"
	raw, _ = json.Marshal(plan)
	p = prepSave(t, s, p, "plan", string(raw))
	req = prepRequest(p, "validate-plan")
	req.Hash = p.Documents["plan"].Hash
	p, e = s.preparationCommand(req)
	if e != nil || !p.PlanReady {
		t.Fatal(p, e)
	}
	var count int
	s.db.QueryRow("SELECT count(*) FROM agents").Scan(&count)
	if count != 0 {
		t.Fatal("created agents")
	}
}
func TestPreparationPlanRejectsCycleAndLateReply(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	p = prepAdopt(t, s, p)
	r.Revision = p.Revision
	r.Target = "plan"
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	plan := fixturePlan()
	plan.Tasks[0].Depends = []string{"T2"}
	b, _ := json.Marshal(plan)
	a, _ := json.Marshal(PreparationAnswer{Message: "Plan", Plan: string(b)})
	s.finishPreparationTurn(turn, string(a), nil, "")
	turn, _ = s.preparationTurn(turn.ID)
	if turn.Status != "failed" || !strings.Contains(turn.Error, "Cycle") {
		t.Fatal(turn)
	}
	r.Event = "next"
	turn, e = s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	plan = fixturePlan()
	b, _ = json.Marshal(plan)
	a, _ = json.Marshal(PreparationAnswer{Message: "Plan", Plan: string(b)})
	p = prepSave(t, s, p, "brief", "Brief modifié pendant la génération.")
	s.finishPreparationTurn(turn, string(a), nil, "")
	req := prepRequest(p, "use-proposal")
	req.Turn = turn.ID
	if _, e = s.preparationCommand(req); e == nil {
		t.Fatal("late plan applied")
	}
}
func TestPreparationTerminalConfirmationPasteAndUnknownCommand(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	s.runPreparationTurn(turn)
	var out bytes.Buffer
	tty := &preparationTerminal{s: s, p: p, out: &out, width: 79}
	if _, e = tty.line("/appliquer "+turn.ID, false); e != nil {
		t.Fatal(e)
	}
	if _, e = tty.line("/confirmer", true); e == nil {
		t.Fatal("paste confirmed action")
	}
	current, _ := s.preparation(p.ID)
	if current.Revision != p.Revision {
		t.Fatal("early write")
	}
	if _, e = tty.line("/confirmer", false); e != nil {
		t.Fatal(e)
	}
	if tty.p.Brief != nil || tty.p.Documents["brief"].Text == "" {
		t.Fatal("adopted implicitly")
	}
	if _, e = tty.line("/shell touch unexpected", false); e == nil {
		t.Fatal("unknown command accepted")
	}
	if _, e = tty.line("/multiligne besoin", false); e != nil {
		t.Fatal(e)
	}
	tty.line("Un besoin avec accents éà et /plan.", true)
	tty.line("/envoyer", false)
	if tty.p.Documents["besoin"].Text != "Un besoin avec accents éà et /plan." {
		t.Fatal(tty.p)
	}
	out.Reset()
	tty.say("texte\x1b[31m hostile\x07")
	if strings.Contains(out.String(), "\x1b") || strings.Contains(out.String(), "\x07") {
		t.Fatal("terminal injection")
	}
}

func TestPreparationTerminalResumeCreatesNewAttempt(t *testing.T) {
	s, p, r := prepDialogueFixture(t, prepReplyScript)
	turn, e := s.sendPreparation(r)
	if e != nil {
		t.Fatal(e)
	}
	turn, e = s.cancelPreparation(p.ID, turn.ID)
	if e != nil || turn.Status != "interrupted" {
		t.Fatal(turn, e)
	}
	capabilities := s.preparationCapabilities()
	if len(capabilities) != 1 || !capabilities[0].Available {
		t.Fatal(capabilities)
	}
	var out bytes.Buffer
	tty := &preparationTerminal{s: s, p: p, provider: &capabilities[0], out: &out, width: 79}
	if _, e = tty.line("/reprendre", false); e != nil {
		t.Fatal(e)
	}
	turns, e := s.preparationDialogue(p.ID)
	if e != nil || len(turns) != 2 || turns[1].Question != turn.Question || turns[1].ID == turn.ID {
		t.Fatal(turns, e)
	}
}
func TestPreparationExternalEditorPrivateFileAndInvalidDraft(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	var out bytes.Buffer
	tty := &preparationTerminal{s: s, p: p, out: &out, width: 79}
	dir := t.TempDir()
	editor := filepath.Join(dir, "editor with spaces")
	os.WriteFile(editor, []byte("#!/bin/sh\nprintf '%s' '{invalid' > \"$1\"\n"), 0700)
	t.Setenv("VISUAL", editor)
	in, e := os.Open(os.DevNull)
	if e != nil {
		t.Fatal(e)
	}
	defer in.Close()
	if e = tty.editExternal("plan", in); e != nil {
		t.Fatal(e)
	}
	if tty.p.PlanReady || tty.p.Documents["plan"].Text != "{invalid" || !strings.Contains(out.String(), "JSON invalide") {
		t.Fatal(tty.p, out.String())
	}
	args, e := preparationEditorArgs(`"/path with spaces/editor" --wait 'literal $(secret)'`)
	if e != nil || len(args) != 3 || args[2] != "literal $(secret)" {
		t.Fatal(args, e)
	}
}

func TestPreparationExternalEditorConflictKeepsLocalFile(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	var out bytes.Buffer
	tty := &preparationTerminal{s: s, p: p, out: &out, width: 79}
	marker := filepath.Join(t.TempDir(), "marker")
	editor := filepath.Join(t.TempDir(), "editor")
	os.WriteFile(editor, []byte("#!/bin/sh\nprintf '%s' \"$1\" > \"$SWARM_EDITOR_MARKER\"\nsleep 0.3\nprintf '%s' 'Version locale' > \"$1\"\n"), 0700)
	t.Setenv("VISUAL", editor)
	t.Setenv("SWARM_EDITOR_MARKER", marker)
	in, e := os.Open(os.DevNull)
	if e != nil {
		t.Fatal(e)
	}
	defer in.Close()
	done := make(chan error, 1)
	go func() { done <- tty.editExternal("brief", in) }()
	deadline := time.Now().Add(2 * time.Second)
	var local []byte
	for time.Now().Before(deadline) {
		local, _ = os.ReadFile(marker)
		if len(local) > 0 {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if len(local) == 0 {
		t.Fatal("editor did not start")
	}
	p = prepSave(t, s, p, "brief", "Version web concurrente")
	if e = <-done; e == nil {
		t.Fatal("editor overwrote newer revision")
	}
	b, e := os.ReadFile(string(local))
	if e != nil || string(b) != "Version locale" {
		t.Fatal("local draft lost", e)
	}
	defer os.RemoveAll(filepath.Dir(string(local)))
	current, _ := s.preparation(p.ID)
	if current.Documents["brief"].Text != "Version web concurrente" {
		t.Fatal(current)
	}
}
