//go:build linux

package engine

import (
	"bytes"
	"database/sql"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestDialogueRestorePublicOperationExists(t *testing.T) {
	s, a, _ := dialogueRestoreFixture(t, "codex")
	p, err := s.recoveryPreview(a.WorkID, a.TaskID, a.ID)
	if err != nil {
		t.Fatal(err)
	}
	request := map[string]any{"schema_version": 1, "event_id": "restore-public", "expected_revision": p.Revision, "checkpoint_sha256": p.Dialogue.Boundary.SHA256, "facts_sha256": p.Dialogue.FactsSHA256, "confirm": true, "reason": "Examine the same retained session", "instruction": "Continue the retained conversation after this confirmed boundary"}
	raw, _ := json.Marshal(request)
	file := filepath.Join(t.TempDir(), "request.json")
	if err := os.WriteFile(file, raw, 0600); err != nil {
		t.Fatal(err)
	}
	var out, stderr bytes.Buffer
	code := run([]string{"--root", s.root, "--json", "agent", "resume-dialogue-preview", a.ID, "--input", file}, &out, &stderr)
	if code != 0 {
		t.Fatalf("explicit dialogue restoration preview is unavailable: %s%s", out.String(), stderr.String())
	}
}

func dialogueRestoreFixture(t *testing.T, provider string) (*Store, Agent, string) {
	t.Helper()
	events := []string{`{"type":"thread.started","thread_id":"restore-session"}`, `{"type":"item.started","item":{"id":"tool_0","type":"command_execution","command":"fixture-command"}}`, `{"type":"item.completed","item":{"id":"tool_0","type":"command_execution","status":"completed","exit_code":0}}`, `{"type":"turn.completed","total_cost_usd":0.1,"usage":{"input_tokens":10,"output_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":1,"cached_input_tokens":2}}`}
	if provider == "claude" {
		events = []string{`{"type":"system","session_id":"restore-session"}`, `{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool_0","name":"Read","input":{"file_path":"fixture.txt"}}]}}`, `{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tool_0","content":"fixture value"}]}}`, `{"type":"result","subtype":"success","is_error":false,"session_id":"restore-session","total_cost_usd":0.1,"usage":{"input_tokens":10,"output_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":1,"cached_input_tokens":2}}`}
	}
	s, a, script := dialogueReceiptFixture(t, provider, events)
	a.Limits, _ = (RunLimits{MaxToolCalls: 4, MaxRepeatedCalls: 8, MaxConsecutiveErrors: 3}).normalized()
	if err := s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	providers, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	p := providers.Providers[a.Provider]
	p.Limits = a.Limits
	p.PreflightArgs = []string{"--swarm-preflight"}
	p.PreflightKind = "provider-context"
	p.PreflightCaps = append([]string{}, requiredProviderCapabilities...)
	p.PreflightTimeout = 5
	providers.Providers[a.Provider] = p
	raw, _ := json.Marshal(providers)
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	body, err := os.ReadFile(script)
	if err != nil {
		t.Fatal(err)
	}
	prefix := `#!/bin/sh
if [ "$1" = "--swarm-preflight" ]; then
 if [ -f "$0.block" ]; then touch "$0.ready"; while [ ! -f "$0.release" ]; do sleep 0.01; done; fi
 printf '%s\n' '{"schema_version":1,"capabilities":{"process":"verified","workspace_read":"verified","workspace_write":"verified"}}'
 exit 0
fi
`
	if err = os.WriteFile(script, []byte(prefix+strings.TrimPrefix(string(body), "#!/bin/sh\n")), 0700); err != nil {
		t.Fatal(err)
	}
	m, err := runDialogueReceiptWrapper(t, s, a, "/fin\n")
	if err != nil {
		t.Fatal(err)
	}
	a.Progress, a.Usage, _, _ = m.executionSnapshot()
	if err = s.finishAgent(a, "completed", "local source completed without acceptance", nil); err != nil {
		t.Fatal(err)
	}
	a, err = s.agent(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	return s, a, script
}

func dialogueRestoreRequestTest(t *testing.T, s *Store, a Agent) DialogueRestoreRequest {
	t.Helper()
	p, err := s.recoveryPreview(a.WorkID, a.TaskID, a.ID)
	if err != nil {
		t.Fatal(err)
	}
	return DialogueRestoreRequest{Schema: 1, Event: newID("restore-"), Revision: p.Revision, Checkpoint: p.Dialogue.Boundary.SHA256, Facts: p.Dialogue.FactsSHA256, Confirm: true, Reason: "Continue after reviewing this confirmed boundary", Instruction: "Continue the same conversation using the retained boundary and current task contract"}
}

func TestDialogueRestoreKeepsSessionGuardsAndSeparatesExecutionUsage(t *testing.T) {
	for _, provider := range []string{"codex", "claude"} {
		t.Run(provider, func(t *testing.T) {
			s, source, script := dialogueRestoreFixture(t, provider)
			original := loadDialogueCheckpointTest(t, s, source).SHA256
			for step := 1; step <= 2; step++ {
				r := dialogueRestoreRequestTest(t, s, source)
				before := recoverySnapshotTest(t, s)
				preview, created, err := s.resumeDialogue(source.ID, r, true)
				if err != nil || created || preview.DialogueRestore == nil || preview.DialogueRestore.DecisionRecorded || before != recoverySnapshotTest(t, s) {
					t.Fatal("preview mutated or granted a decision", err)
				}
				a, created, err := s.resumeDialogue(source.ID, r, false)
				if err != nil || !created {
					t.Fatal("restore not prepared", err)
				}
				if a.Attempt == source.Attempt || a.ID == source.ID || a.DialogueRestore.SourceAgent != source.ID || !a.DialogueRestore.DecisionRecorded || a.DialogueRestore.PriorTurns != step {
					t.Fatal("execution identity or decision lost", a.DialogueRestore)
				}
				replay, again, err := s.resumeDialogue(source.ID, r, false)
				if err != nil || again || replay.ID != a.ID || replay.DialogueRestore.At != a.DialogueRestore.At {
					t.Fatal("replay duplicated execution or decision", err)
				}
				m, err := runDialogueReceiptWrapper(t, s, a, "/fin\n")
				if err != nil {
					t.Fatal("same session not resumed", err)
				}
				a.Progress, a.Usage, _, _ = m.executionSnapshot()
				if a.Usage == nil || a.Usage.Input != 10 || a.Usage.CacheRead == nil || *a.Usage.CacheRead != 3 || a.Progress.ToolCalls != 1 || a.Progress.ToolResults != 1 {
					t.Fatal("history counted twice", a.Usage, a.Progress)
				}
				if err = s.finishAgent(a, "completed", "local restored execution ended", nil); err != nil {
					t.Fatal(err)
				}
				c := loadDialogueCheckpointTest(t, s, a)
				if c.Session != "restore-session" || c.Guard.Calls != step+1 || c.Usage.Input != int64((step+1)*10) || c.Usage.CacheRead == nil || *c.Usage.CacheRead != int64((step+1)*3) {
					t.Fatal("conversation state reset", c.Guard.Calls, c.Usage)
				}
				cost, err := s.costSummary(a.WorkID)
				if err != nil || cost.Reported < float64(step+1)*.1-.00001 || cost.Reported > float64(step+1)*.1+.00001 {
					t.Fatal("cost double counted", cost, err)
				}
				if step == 1 && loadDialogueCheckpointTest(t, s, source).SHA256 != original {
					t.Fatal("source checkpoint rewritten")
				}
				source, err = s.agent(a.ID)
				if err != nil {
					t.Fatal(err)
				}
			}
			args, err := os.ReadFile(script + ".args")
			if err != nil || strings.Count(string(args), "resume restore-session") != 2 {
				t.Fatal("provider resume arguments lost", string(args), err)
			}
		})
	}
}

func TestDialogueRestoreRejectsStaleUnconfirmedAndUnverifiedWithoutMutation(t *testing.T) {
	for _, variant := range []string{"confirmation", "facts", "checkpoint", "revision", "unconfirmed", "provider_preflight", "source_status", "unknown_field"} {
		t.Run(variant, func(t *testing.T) {
			s, a, script := dialogueRestoreFixture(t, "codex")
			r := dialogueRestoreRequestTest(t, s, a)
			switch variant {
			case "confirmation":
				r.Confirm = false
			case "facts":
				r.Facts = strings.Repeat("0", 64)
			case "checkpoint":
				r.Checkpoint = strings.Repeat("0", 64)
			case "revision":
				r.Revision--
			case "unconfirmed":
				if _, err := s.db.Exec("UPDATE agent_dialogue_turns SET status='interrupted' WHERE agent_id=?", a.ID); err != nil {
					t.Fatal(err)
				}
			case "source_status":
				a.Status = "running"
				a.Ended = ""
				if err := s.saveAgent(a); err != nil {
					t.Fatal(err)
				}
			case "provider_preflight":
				providers, _ := s.providers()
				p := providers.Providers[a.Provider]
				p.PreflightArgs = nil
				providers.Providers[a.Provider] = p
				raw, _ := json.Marshal(providers)
				if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
					t.Fatal(err)
				}
				r = dialogueRestoreRequestTest(t, s, a)
			case "unknown_field":
				raw, _ := json.Marshal(r)
				raw = append(raw[:len(raw)-1], []byte(`,"actor":"engine"}`)...)
				var request DialogueRestoreRequest
				if strict(raw, &request) == nil {
					t.Fatal("unknown authority accepted")
				}
				return
			}
			before := recoverySnapshotTest(t, s)
			calls, _ := os.ReadFile(script + ".calls")
			if _, _, err := s.resumeDialogue(a.ID, r, false); err == nil {
				t.Fatal("invalid restore accepted")
			}
			afterCalls, _ := os.ReadFile(script + ".calls")
			if before != recoverySnapshotTest(t, s) || !bytes.Equal(calls, afterCalls) {
				t.Fatal("refusal mutated state or executed provider")
			}
		})
	}
}

func TestDialogueRestoreRechecksSourceAfterPreflight(t *testing.T) {
	s, a, script := dialogueRestoreFixture(t, "codex")
	r := dialogueRestoreRequestTest(t, s, a)
	if err := os.WriteFile(script+".block", []byte("wait"), 0600); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { _, _, err := s.resumeDialogue(a.ID, r, false); done <- err }()
	until := time.Now().Add(3 * time.Second)
	for {
		if _, err := os.Stat(script + ".ready"); err == nil {
			break
		}
		if time.Now().After(until) {
			t.Fatal("preflight did not reach owned barrier")
		}
		time.Sleep(10 * time.Millisecond)
	}
	if _, err := s.db.Exec("UPDATE agent_dialogue_turns SET session_id='changed-during-probe' WHERE agent_id=?", a.ID); err != nil {
		t.Fatal(err)
	}
	before := recoverySnapshotTest(t, s)
	if err := os.WriteFile(script+".release", []byte("release"), 0600); err != nil {
		t.Fatal(err)
	}
	select {
	case err := <-done:
		if err == nil {
			t.Fatal("stale source accepted after probe")
		}
	case <-time.After(6 * time.Second):
		t.Fatal("resume blocked after probe release")
	}
	if before != recoverySnapshotTest(t, s) {
		t.Fatal("failed freshness check wrote a launch")
	}
}

func TestDialogueRestoreConcurrentRequestsCreateOneExecution(t *testing.T) {
	s, a, _ := dialogueRestoreFixture(t, "codex")
	r := dialogueRestoreRequestTest(t, s, a)
	var wg sync.WaitGroup
	results := make(chan bool, 2)
	for i := 0; i < 2; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			other, err := openStore(s.root, false)
			if err != nil {
				t.Error(err)
				return
			}
			defer other.db.Close()
			q := r
			q.Event = fmt.Sprintf("restore-concurrent-%d", i)
			_, created, err := other.resumeDialogue(a.ID, q, false)
			results <- err == nil && created
		}(i)
	}
	wg.Wait()
	close(results)
	count := 0
	for success := range results {
		if success {
			count++
		}
	}
	if count != 1 {
		t.Fatal("concurrent restores created wrong execution count", count)
	}
	w, err := s.get(a.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	if len(task.Attempts) != 2 {
		t.Fatal("attempt history overwritten or duplicated", len(task.Attempts))
	}
}

func TestDialogueRestoreCumulativeToolLimitCannotReset(t *testing.T) {
	s, source, script := dialogueRestoreFixture(t, "codex")
	for step := 2; step <= 4; step++ {
		r := dialogueRestoreRequestTest(t, s, source)
		a, created, err := s.resumeDialogue(source.ID, r, false)
		if err != nil || !created {
			t.Fatal(err)
		}
		input := "/fin\n"
		if step == 4 {
			input = "must not bypass a guard\n/fin\n"
		}
		m, runErr := runDialogueReceiptWrapper(t, s, a, input)
		if (step < 4 && runErr != nil) || (step == 4 && runErr == nil) {
			t.Fatal("unexpected cumulative guard result", step, runErr)
		}
		a.Progress, a.Usage, _, _ = m.executionSnapshot()
		state := "completed"
		if runErr != nil {
			state = "interrupted"
		}
		if err = s.finishAgent(a, state, "local cumulative guard result", nil); err != nil {
			t.Fatal(err)
		}
		c := loadDialogueCheckpointTest(t, s, a)
		if c.Guard.Calls != step {
			t.Fatal("tool counter reset", c.Guard.Calls, step)
		}
		if step == 4 && c.Phase != "interrupted" {
			t.Fatal("exhausted cumulative guard became confirmed")
		}
		source, err = s.agent(a.ID)
		if err != nil {
			t.Fatal(err)
		}
	}
	before := recoverySnapshotTest(t, s)
	if _, _, err := s.resumeDialogue(source.ID, dialogueRestoreRequestTest(t, s, source), false); err == nil {
		t.Fatal("exhausted guard restarted")
	}
	calls, err := os.ReadFile(script + ".calls")
	if err != nil || strings.Count(string(calls), "call\n") != 4 || before != recoverySnapshotTest(t, s) {
		t.Fatal("guard failure replayed provider or mutated", err)
	}
}

func TestDialogueRestoreWrongSessionIsNotSuccessful(t *testing.T) {
	s, source, script := dialogueRestoreFixture(t, "codex")
	r := dialogueRestoreRequestTest(t, s, source)
	a, _, err := s.resumeDialogue(source.ID, r, false)
	if err != nil {
		t.Fatal(err)
	}
	body, err := os.ReadFile(script)
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(script, []byte(strings.ReplaceAll(string(body), "restore-session", "foreign-session")), 0700); err != nil {
		t.Fatal(err)
	}
	m, err := runDialogueReceiptWrapper(t, s, a, "must not execute again\n/fin\n")
	if err == nil {
		t.Fatal("new provider session advertised as restored")
	}
	a.Progress, a.Usage, _, _ = m.executionSnapshot()
	if err = s.finishAgent(a, "interrupted", "foreign session refused", nil); err != nil {
		t.Fatal(err)
	}
	b, err := s.dialogueBoundary(a)
	if err != nil || b.Confirmed {
		t.Fatal("foreign session confirmed", b, err)
	}
	calls, _ := os.ReadFile(script + ".calls")
	if strings.Count(string(calls), "call\n") != 2 {
		t.Fatal("failed resume automatically retried", string(calls))
	}
}

func TestDialogueRestorePreservesAttemptAndBudgetGuards(t *testing.T) {
	for _, kind := range []string{"attempt_limit", "budget"} {
		t.Run(kind, func(t *testing.T) {
			s, a, _ := dialogueRestoreFixture(t, "codex")
			w, err := s.get(a.WorkID)
			if err != nil {
				t.Fatal(err)
			}
			if kind == "attempt_limit" {
				_, err = s.mutate(w.ID, "fixture.attempt-limit", newID("fixture-"), w.Revision, []byte("{}"), func(w *Work) error { w.Tasks[0].PlanMaxAttempts = 1; return nil })
			} else {
				_, err = s.configureBudget(w.ID, BudgetChange{Schema: 1, EventID: newID("fixture-budget-"), Revision: w.Revision, Budget: Budget{Limit: 1, Reserve: 2, Source: "local fixture estimate", PriceDate: "2026-10-16"}})
			}
			if err != nil {
				t.Fatal(err)
			}
			r := dialogueRestoreRequestTest(t, s, a)
			before := recoverySnapshotTest(t, s)
			if _, _, err = s.resumeDialogue(a.ID, r, false); err == nil {
				t.Fatal("existing guard bypassed", kind)
			}
			if before != recoverySnapshotTest(t, s) {
				t.Fatal("guard refusal partially mutated", kind)
			}
		})
	}
}

func TestDialogueRestoreVersionFencePreservesCheckpointAndBackup(t *testing.T) {
	s, a, _ := dialogueRestoreFixture(t, "codex")
	original := loadDialogueCheckpointTest(t, s, a).SHA256
	if _, err := s.db.Exec("PRAGMA user_version=34"); err != nil {
		t.Fatal(err)
	}
	if old, err := openStoreWithMigration(s.root, false, false); err == nil {
		old.db.Close()
		t.Fatal("inspection silently migrated")
	}
	upgraded, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer upgraded.db.Close()
	var version int
	if err = upgraded.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != 35 {
		t.Fatal("restoration fence absent", version, err)
	}
	if loadDialogueCheckpointTest(t, upgraded, a).SHA256 != original {
		t.Fatal("migration rewrote checkpoint")
	}
	backups, err := filepath.Glob(filepath.Join(s.root, ".swarm/state-pre-v35-*.db"))
	if err != nil || len(backups) != 1 {
		t.Fatal("migration backup absent", backups, err)
	}
	info, err := os.Stat(backups[0])
	if err != nil || info.Mode().Perm() != 0600 {
		t.Fatal("backup is not private", err)
	}
	backup, err := sql.Open("sqlite", backups[0])
	if err != nil {
		t.Fatal(err)
	}
	defer backup.Close()
	if err = backup.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != 34 {
		t.Fatal("backup not original version", version, err)
	}
	var raw []byte
	if err = backup.QueryRow("SELECT checkpoint FROM agent_dialogue_turns WHERE agent_id=?", a.ID).Scan(&raw); err != nil {
		t.Fatal(err)
	}
	var c dialogueCheckpoint
	if json.Unmarshal(raw, &c) != nil || c.SHA256 != original {
		t.Fatal("backup lost original checkpoint")
	}
}

func TestDialogueRestoreStartupRejectsChangedContractAndReceipt(t *testing.T) {
	for _, kind := range []string{"provider", "receipt", "preflight_expired"} {
		t.Run(kind, func(t *testing.T) {
			s, source, _ := dialogueRestoreFixture(t, "codex")
			r := dialogueRestoreRequestTest(t, s, source)
			a, _, err := s.resumeDialogue(source.ID, r, false)
			if err != nil {
				t.Fatal(err)
			}
			if err = s.dialogueRestoreStartGuard(a); err != nil {
				t.Fatal("fresh restoration fails startup", err)
			}
			switch kind {
			case "provider":
				providers, _ := s.providers()
				p := providers.Providers[a.Provider]
				p.Env = append(p.Env, "SWARM_DIFFERENT_CONTEXT")
				providers.Providers[a.Provider] = p
				raw, _ := json.Marshal(providers)
				if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
					t.Fatal(err)
				}
			case "receipt":
				a.DialogueRestore.PriorTurns++
			case "preflight_expired":
				a.Preflight.ValidUntil = time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano)
			}
			if err = s.dialogueRestoreStartGuard(a); err == nil {
				t.Fatal("changed contract accepted at startup", kind)
			}
		})
	}
}

func TestDialogueRestoreHTTPPreviewAuthenticationScopeAndNoMutation(t *testing.T) {
	s, a, script := dialogueRestoreFixture(t, "codex")
	request := dialogueRestoreRequestTest(t, s, a)
	raw, _ := json.Marshal(request)
	handler := newWebHandler(s, "localhost:18787", "restore-http-token")
	before := recoverySnapshotTest(t, s)
	calls, _ := os.ReadFile(script + ".calls")
	for _, kind := range []string{"unauthenticated", "csrf", "foreign_work", "malformed", "valid", "escaped"} {
		body := raw
		if kind == "escaped" {
			large := request
			large.Instruction = "Continue " + strings.Repeat("\x01", 15980)
			body, _ = json.Marshal(large)
			if len(body) <= 65536 {
				t.Fatal("escaped transport fixture is too small")
			}
		}
		if kind == "malformed" {
			body = []byte(`{"schema_version":1,"actor":"engine"}`)
		}
		work := a.WorkID
		if kind == "foreign_work" {
			work = "foreign-work"
		}
		req := httptest.NewRequest(http.MethodPost, "http://localhost:18787/api/v1/dialogue-resume-preview?work="+work+"&agent="+a.ID, bytes.NewReader(body))
		if kind != "unauthenticated" {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "restore-http-token"})
		}
		if kind != "csrf" {
			req.Header.Set("Origin", "http://localhost:18787")
			req.Header.Set("X-Swarm-CSRF", "restore-http-token")
		}
		response := httptest.NewRecorder()
		handler.ServeHTTP(response, req)
		want := 403
		if kind == "malformed" {
			want = 400
		}
		if kind == "valid" || kind == "escaped" {
			want = 200
		}
		if response.Code != want {
			t.Fatal("wrong API status", kind, response.Code, response.Body.String())
		}
		if kind == "valid" || kind == "escaped" {
			var result struct {
				Agent   Agent `json:"agent"`
				Created bool  `json:"created"`
				Preview bool  `json:"preview"`
			}
			if json.Unmarshal(response.Body.Bytes(), &result) != nil || result.Created || !result.Preview || result.Agent.DialogueRestore == nil || result.Agent.DialogueRestore.DecisionRecorded {
				t.Fatal("preview invented decision or launch", response.Body.String())
			}
		}
	}
	if before != recoverySnapshotTest(t, s) {
		t.Fatal("HTTP previews or refusals mutated state")
	}
	afterCalls, _ := os.ReadFile(script + ".calls")
	if !bytes.Equal(calls, afterCalls) {
		t.Fatal("HTTP preview invoked business provider")
	}
}

func TestDialogueRestoreRetainsExistingConversationTurnLimit(t *testing.T) {
	s, a, script := dialogueReceiptFixture(t, "codex", []string{`{"type":"thread.started","thread_id":"twenty-turn-session"}`, `{"type":"turn.completed"}`})
	m, err := runDialogueReceiptWrapper(t, s, a, strings.Repeat("follow-up\n", 19))
	if err == nil {
		t.Fatal("existing twenty-turn boundary unexpectedly disappeared")
	}
	a.Progress, a.Usage, _, _ = m.executionSnapshot()
	if err = s.finishAgent(a, "failed", "existing exchange limit reached", nil); err != nil {
		t.Fatal(err)
	}
	p, err := s.recoveryPreview(a.WorkID, a.TaskID, a.ID)
	if err != nil || !p.Dialogue.Boundary.Confirmed || p.Dialogue.ResumeSupported || p.Dialogue.Path != "review_current_contract" {
		t.Fatal("exhausted conversation offered for restoration", p.Dialogue, err)
	}
	calls, _ := os.ReadFile(script + ".calls")
	if strings.Count(string(calls), "call\n") != 20 {
		t.Fatal("fixture did not run twenty exchanges")
	}
	before := recoverySnapshotTest(t, s)
	r := dialogueRestoreRequestTest(t, s, a)
	if _, _, err = s.resumeDialogue(a.ID, r, false); err == nil || before != recoverySnapshotTest(t, s) {
		t.Fatal("conversation exchange limit reset or refusal mutated")
	}
}
