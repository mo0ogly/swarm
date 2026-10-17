//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"math"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
)

func TestDialogueBoundaryRejectsInconsistentTurnAcrossConsumers(t *testing.T) {
	for _, variant := range []string{"different_session", "missing_session", "unknown_status"} {
		t.Run(variant, func(t *testing.T) {
			s, a, script := dialogueRecoveryFixture(t, []string{`{"type":"thread.started","thread_id":"integrity-session"}`, `{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}`})
			if variant == "unknown_status" {
				if _, err := s.db.Exec("UPDATE agent_dialogue_turns SET status='unexpected' WHERE agent_id=?", a.ID); err != nil {
					t.Fatal(err)
				}
			} else {
				session := "foreign-session"
				if variant == "missing_session" {
					session = ""
				}
				if _, err := s.db.Exec("UPDATE agent_dialogue_turns SET session_id=? WHERE agent_id=?", session, a.ID); err != nil {
					t.Fatal(err)
				}
			}
			before := recoverySnapshotTest(t, s)
			calls, err := os.ReadFile(script + ".calls")
			if err != nil {
				t.Fatal(err)
			}
			if _, err := s.dialogueBoundary(a); err == nil {
				t.Error("inconsistent stored turn became a public boundary")
			}
			if _, err := s.recoveryPreview(a.WorkID, a.TaskID, a.ID); err == nil {
				t.Error("recovery diagnosis accepted an inconsistent stored turn")
			}
			for _, args := range [][]string{{"agent", "show", a.ID}, {"mission", "recovery", a.WorkID, a.TaskID, a.ID}} {
				var out, stderr bytes.Buffer
				argv := append([]string{"--root", s.root, "--json"}, args...)
				if code := run(argv, &out, &stderr); code == 0 || strings.Contains(out.String(), `"boundary_confirmed": true`) {
					t.Errorf("CLI accepted inconsistent turn: %v; code=%d", args, code)
				}
			}
			handler := newWebHandler(s, "localhost:18787", "integrity-fixture-token")
			for _, endpoint := range []string{"agent-detail", "recovery-preview"} {
				req := httptest.NewRequest(http.MethodGet, "http://localhost:18787/api/v1/"+endpoint+"?work="+a.WorkID+"&task="+a.TaskID+"&agent="+a.ID, nil)
				req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "integrity-fixture-token"})
				response := httptest.NewRecorder()
				handler.ServeHTTP(response, req)
				if response.Code != http.StatusBadRequest || strings.Contains(response.Body.String(), "integrity-session") || strings.Contains(response.Body.String(), "foreign-session") {
					t.Errorf("API accepted or exposed inconsistent state: %s; status=%d", endpoint, response.Code)
				}
			}
			afterCalls, err := os.ReadFile(script + ".calls")
			if err != nil || !bytes.Equal(calls, afterCalls) || recoverySnapshotTest(t, s) != before {
				t.Fatal("rejection changed retained state or invoked the provider", err)
			}
		})
	}
}

func TestDialogueCheckpointRejectsUnresumableSessionAndInvalidUsage(t *testing.T) {
	s, a, _ := dialogueRecoveryFixture(t, []string{`{"type":"thread.started","thread_id":"integrity-session"}`, `{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}`})
	original := loadDialogueCheckpointTest(t, s, a)
	for _, variant := range []string{"option_session", "space_session", "cache_read", "cache_creation", "cached_input", "reported_cost", "nan_cost", "infinite_cost"} {
		t.Run(variant, func(t *testing.T) {
			raw, _ := json.Marshal(original)
			var c dialogueCheckpoint
			if err := json.Unmarshal(raw, &c); err != nil {
				t.Fatal(err)
			}
			negative := int64(-1)
			switch variant {
			case "option_session":
				c.Session = "--resume"
			case "space_session":
				c.Session = "invalid session"
			case "cache_read":
				c.Usage.CacheRead = &negative
			case "cache_creation":
				c.Usage.CacheCreation = &negative
			case "cached_input":
				c.Usage.CachedInput = &negative
			case "reported_cost":
				cost := -0.01
				c.Usage.ReportedCost = &cost
			case "nan_cost":
				cost := math.NaN()
				c.Usage.ReportedCost = &cost
			case "infinite_cost":
				cost := math.Inf(1)
				c.Usage.ReportedCost = &cost
			}
			c.SHA256 = c.digest() // A valid digest does not establish semantic validity.
			if _, err := c.restoreGuard(a, s.root); err == nil {
				t.Error("invalid checkpoint payload restored its guards")
			}
		})
	}
}

func TestDialogueConfirmationDoesNotOverwriteChangedSession(t *testing.T) {
	for _, provider := range []string{"codex", "claude"} {
		for _, target := range []int{1, 2} {
			t.Run(fmt.Sprintf("%s/turn_%d", provider, target), func(t *testing.T) {
				events := []string{`{"type":"thread.started","thread_id":"integrity-session"}`, `{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}`}
				if provider == "claude" {
					events = []string{`{"type":"system","subtype":"init","session_id":"integrity-session"}`, `{"type":"result","subtype":"success","session_id":"integrity-session","is_error":false,"usage":{"input_tokens":10,"output_tokens":2}}`}
				}
				s, a, script := dialogueReceiptFixture(t, provider, events)
				// Reproduce the window after durable end, before the answer CAS.
				trigger := fmt.Sprintf(`CREATE TRIGGER change_dialogue_session AFTER UPDATE OF checkpoint ON agent_dialogue_turns WHEN json_extract(NEW.checkpoint,'$.phase')='confirmed' AND (SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=NEW.agent_id)=%d BEGIN UPDATE agent_dialogue_turns SET session_id='concurrent-session' WHERE id=NEW.id; END`, target)
				if _, err := s.db.Exec(trigger); err != nil {
					t.Fatal(err)
				}
				input := strings.Repeat("follow-up\n", target-1) + "must not run\n/fin\n"
				_, wrapperErr := runDialogueReceiptWrapper(t, s, a, input)
				var status, session string
				if err := s.db.QueryRow("SELECT status,session_id FROM agent_dialogue_turns WHERE agent_id=? ORDER BY rowid DESC LIMIT 1", a.ID).Scan(&status, &session); err != nil {
					t.Fatal(err)
				}
				calls, err := os.ReadFile(script + ".calls")
				if wrapperErr == nil || err != nil || status == "answered" || session != "concurrent-session" || string(calls) != strings.Repeat("call\n", target) {
					t.Fatal(fmt.Sprintf("changed session overwritten or provider continued: error=%v status=%s session=%s calls=%q read=%v", wrapperErr, status, session, calls, err))
				}
			})
		}
	}
}

func TestDialogueBoundaryAllowsUnconfirmedEndBeforeAnswer(t *testing.T) {
	s, a, _ := dialogueRecoveryFixture(t, []string{`{"type":"thread.started","thread_id":"integrity-session"}`, `{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}`})
	// The durable end precedes answer confirmation; the row can still carry
	// the previous session, which is empty during the first turn.
	for _, status := range []string{"running", "interrupted"} {
		if _, err := s.db.Exec("UPDATE agent_dialogue_turns SET status=?,session_id='' WHERE agent_id=?", status, a.ID); err != nil {
			t.Fatal(err)
		}
		v, err := s.dialogueBoundary(a)
		if err != nil || !v.Available || v.Confirmed || v.Phase != "confirmed" || v.Status != status || v.RestartSupported {
			t.Fatal("valid transition was rejected or advertised as confirmed", v, err)
		}
	}
}
