package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestConductorContextUnknownSelection(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	for _, page := range []string{"tasks", "agents", "decisions", "logs", "brainstorm"} {
		if _, e := s.pageContext(w.ID, PageCoordinates{PageID: page, Selected: []string{"not-in-this-work"}}); e == nil {
			t.Errorf("%s accepted nonexistent selection", page)
		}
	}
}
func TestConductorContextUniqueActions(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Other target", Deliverable: "report", Criteria: []string{"verified"}})
	c, e := s.pageContext(w.ID, PageCoordinates{PageID: "tasks"})
	if e != nil {
		t.Fatal(e)
	}
	seen := map[string]bool{}
	for _, a := range c.Actions {
		if seen[a.ID] {
			t.Errorf("ambiguous action %s", a.ID)
		}
		seen[a.ID] = true
	}
}
func TestConductorEmptyPagesHaveGroundableState(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	for _, p := range assistPages {
		c, e := s.pageContext(w.ID, PageCoordinates{PageID: p.ID})
		if e != nil {
			t.Fatal(e)
		}
		if len(c.Facts) == 0 {
			t.Errorf("%s has no facts so every answer refused", p.ID)
		}
	}
}
func TestConductorNoUnsupportedSteps(t *testing.T) {
	c := PageContext{Hash: "hash", Facts: []PageFact{{ID: "f1"}}, Actions: []PageAction{{ID: "do", Available: true}}}
	a := AssistantAnswer{Version: 1, TemplateID: "next_action.v1", ContextHash: "hash", Facts: []AnswerFact{{Text: "Known fact", SourceIDs: []string{"f1"}}}, NextSteps: []AnswerStep{{ActionID: "do", Why: "why"}}}
	if r := checkAnswer(&a, c, a.TemplateID); r == nil {
		t.Error("unsupported action justification accepted without references")
	}
	a.NextSteps[0].SourceIDs = make([]string, 50)
	for i := range a.NextSteps[0].SourceIDs {
		a.NextSteps[0].SourceIDs[i] = "f1"
	}
	if r := checkAnswer(&a, c, a.TemplateID); r == nil {
		t.Error("unbounded action references")
	}
}
func TestConductorSecretsExcluded(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w = applyTest(t, s, w, "checkpoint", Request{Summary: "Authorization: Bearer test-private-token-1234567890123456\npassword=very-private-value", Next: "review"})
	c, e := s.pageContext(w.ID, PageCoordinates{PageID: "resume"})
	if e != nil {
		t.Fatal(e)
	}
	b, _ := json.Marshal(c)
	for _, secret := range []string{"test-private-token", "very-private-value"} {
		if strings.Contains(string(b), secret) {
			t.Errorf("context leaks %s", secret)
		}
	}
}
func TestConductorExactPreviewAndIdempotencyIsolation(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	other := createTest(t, s)
	if e := s.initProviders(); e != nil {
		t.Fatal(e)
	}
	ps, _ := s.providers()
	name := ""
	for id := range ps.Providers {
		name = id
		break
	}
	r := AssistRequest{EventID: newID("question-"), Revision: w.Revision, Coordinates: PageCoordinates{PageID: "tasks"}, TemplateID: "understand_page.v1", Provider: name}
	p, e := s.assistPreview(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	r.ContextHash = p.Turn.Context.Hash
	turn, e := s.assistAsk(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if turn.Prompt != p.Prompt {
		t.Fatal("prompt sent differs from exact preview")
	}
	if _, e = s.assistAsk(other.ID, r); e == nil {
		t.Fatal("cross-work replay disclosed turn")
	}
	r.Question = "changed after preview"
	if _, e = s.assistAsk(w.ID, r); e == nil {
		t.Fatal("event ID reused for modified question")
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision || len(after.Tasks) != len(w.Tasks) {
		t.Fatal("question changed workflow progress")
	}
	if e = s.cancelAssist(w.ID, turn.ID); e != nil {
		t.Fatal(e)
	}
	if e = s.settleAssistTurn(turn, "{}", nil, nil); e != nil {
		t.Fatal(e)
	}
	end, _ := s.assistTurn(turn.ID)
	if end.Status != "interrupted" {
		t.Fatal("late result overwrote cancellation")
	}
}
func TestConductorOwnershipDoesNotClearBlocker(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Blocker: "Provider quota exhausted"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Owner: "codex"})
	if w.Tasks[0].Status != "blocked" || w.Tasks[0].Blocker != "Provider quota exhausted" || w.Tasks[0].Owner != "codex" {
		t.Fatal(w.Tasks[0])
	}
}

func TestConductorDeadlineSurvivesClosedStdout(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	command := filepath.Join(t.TempDir(), "claude")
	if e := os.WriteFile(command, []byte("#!/bin/sh\nexec 1>&-\nsleep 15\n"), 0700); e != nil {
		t.Fatal(e)
	}
	ps := Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: command}}}
	raw, _ := json.Marshal(ps)
	if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); e != nil {
		t.Fatal(e)
	}
	r := AssistRequest{EventID: newID("question-"), Revision: w.Revision, Coordinates: PageCoordinates{PageID: "tasks"}, TemplateID: "understand_page.v1", Provider: "fixture"}
	p, e := s.assistPreview(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	r.ContextHash = p.Turn.Context.Hash
	turn, e := s.assistAsk(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	start := time.Now()
	s.runAssistTurnWithin(turn, 80*time.Millisecond)
	if time.Since(start) > 3*time.Second {
		t.Fatal("stdout closure disabled process deadline")
	}
	after, e := s.assistTurn(turn.ID)
	if e != nil || after.Refusal == nil || after.Refusal.Code != "timeout" {
		t.Fatal(after, e)
	}
}
func TestConductorProviderArgumentsCannotEnableCoding(t *testing.T) {
	command := filepath.Join(t.TempDir(), "codex")
	if e := os.WriteFile(command, []byte("#!/bin/sh\nexit 0\n"), 0700); e != nil {
		t.Fatal(e)
	}
	p, e := assistantProvider(Provider{Command: command, Args: []string{"exec", "--dangerously-bypass-approvals-and-sandbox", "--model", "model-selected", "--add-dir", "/"}})
	if e != nil {
		t.Fatal(e)
	}
	args := strings.Join(p.Args, " ")
	if strings.Contains(args, "dangerously") || strings.Contains(args, "add-dir") || !strings.Contains(args, "--sandbox read-only") || !strings.Contains(args, "--disable shell_tool") || !strings.Contains(args, "--ignore-user-config") || !strings.Contains(args, "model-selected") {
		t.Fatal(args)
	}
}
func TestConductorArchivePreservesAssistantHistory(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if e := s.initProviders(); e != nil {
		t.Fatal(e)
	}
	ps, _ := s.providers()
	name := ""
	for id := range ps.Providers {
		name = id
		break
	}
	r := AssistRequest{EventID: newID("question-"), Revision: w.Revision, Coordinates: PageCoordinates{PageID: "tasks"}, TemplateID: "understand_page.v1", Provider: name}
	p, e := s.assistPreview(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	r.ContextHash = p.Turn.Context.Hash
	turn, e := s.assistAsk(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.settleAssistTurn(turn, answerFor(turn.Context, turn.TemplateID, nil), nil, nil); e != nil {
		t.Fatal(e)
	}
	path := filepath.Join(t.TempDir(), "work.zip")
	if e = s.export(w.ID, path); e != nil {
		t.Fatal(e)
	}
	dest := storeTest(t)
	if _, e = dest.importBundle(path); e != nil {
		t.Fatal(e)
	}
	turns, e := dest.assistCurrentTurns(w.ID)
	if e != nil || len(turns) != 1 || turns[0].Answer == nil || turns[0].Prompt != p.Prompt || !turns[0].Imported || !turns[0].Stale {
		t.Fatal(turns, e)
	}
}
func TestConductorAtomicActiveQuestion(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if e := s.initProviders(); e != nil {
		t.Fatal(e)
	}
	ps, _ := s.providers()
	name := ""
	for id := range ps.Providers {
		name = id
		break
	}
	rs := make([]AssistRequest, 2)
	for i := range rs {
		rs[i] = AssistRequest{EventID: newID("question-"), Revision: w.Revision, Coordinates: PageCoordinates{PageID: "tasks"}, TemplateID: "understand_page.v1", Provider: name}
		p, e := s.assistPreview(w.ID, rs[i])
		if e != nil {
			t.Fatal(e)
		}
		rs[i].ContextHash = p.Turn.Context.Hash
	}
	done := make(chan error, 2)
	start := make(chan struct{})
	for _, r := range rs {
		go func(r AssistRequest) { <-start; _, e := s.assistAsk(w.ID, r); done <- e }(r)
	}
	close(start)
	success := 0
	for range rs {
		if <-done == nil {
			success++
		}
	}
	if success != 1 {
		t.Fatalf("%d simultaneous questions accepted", success)
	}
}

func TestConductorDeadSupervisorDoesNotLeaveInfiniteWait(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if e := s.initProviders(); e != nil {
		t.Fatal(e)
	}
	ps, _ := s.providers()
	name := ""
	for id := range ps.Providers {
		name = id
		break
	}
	r := AssistRequest{EventID: newID("question-"), Revision: w.Revision, Coordinates: PageCoordinates{PageID: "tasks"}, TemplateID: "understand_page.v1", Provider: name}
	p, e := s.assistPreview(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	r.ContextHash = p.Turn.Context.Hash
	turn, e := s.assistAsk(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	turn.Status = "running"
	turn.Host = hostIdentity()
	turn.SupervisorPID = 99999999
	turn.SupervisorStamp = "missing"
	if e = s.saveAssistTurn(turn); e != nil {
		t.Fatal(e)
	}
	ts, e := s.assistCurrentTurns(w.ID)
	if e != nil || len(ts) != 1 || ts[0].Status != "interrupted" || ts[0].Refusal == nil {
		t.Fatal(ts, e)
	}
}

func TestConductorStructuredProviderNarrowsReferences(t *testing.T) {
	turn := AssistTurn{TemplateID: "understand_page.v1", Context: PageContext{Hash: strings.Repeat("a", 64), Facts: []PageFact{{ID: "f1"}}, Actions: []PageAction{{ID: "task.review:t1", Available: true}, {ID: "task.start:t1", Available: false}}}}
	for _, name := range []string{"codex", "claude"} {
		p := Provider{Command: "/bin/" + name, Args: []string{"-"}}
		got, e := assistantStructuredProvider(p, turn, t.TempDir())
		if e != nil {
			t.Fatal(e)
		}
		var raw []byte
		if name == "codex" {
			raw, e = os.ReadFile(got.Args[len(got.Args)-2])
			if e != nil {
				t.Fatal(e)
			}
		} else {
			raw = []byte(got.Args[len(got.Args)-1])
		}
		var schema map[string]any
		if e = json.Unmarshal(raw, &schema); e != nil {
			t.Fatal(e)
		}
		if !strings.Contains(string(raw), "task.review:t1") || strings.Contains(string(raw), "task.start:t1") {
			t.Fatal(string(raw))
		}
		props := schema["properties"].(map[string]any)
		for _, k := range []string{"facts", "next_steps"} {
			refs := props[k].(map[string]any)["items"].(map[string]any)["properties"].(map[string]any)["source_ids"].(map[string]any)
			if refs["minItems"] != float64(1) || refs["items"].(map[string]any)["enum"].([]any)[0] != "f1" {
				t.Fatal(refs)
			}
		}
	}
	out := readAssistOutput(strings.NewReader(`{"type":"result","result":"","structured_output":{"version":1,"facts":[]}}`))
	if out.err != nil || !strings.Contains(out.reply, `"version":1`) {
		t.Fatal(out)
	}
}

func TestConductorLogPublicTextBeforeTruncation(t *testing.T) {
	raw := `{"is_error":true,"usage":"` + strings.Repeat("x", 2000) + `","result":"session limit"}`
	if got := assistLogMessage(raw); !strings.Contains(got, "session limit") || strings.Contains(got, "xxx") {
		t.Fatal(got)
	}
	raw = `{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"PRIVATE"},{"type":"text","text":"Public error"}]}}`
	if got := assistLogMessage(raw); got != "Public error" {
		t.Fatal(got)
	}
	if got := assistLogMessage(`{"type":"item.completed","item":{"type":"reasoning","text":"PRIVATE"}}`); strings.Contains(got, "PRIVATE") {
		t.Fatal(got)
	}
}
