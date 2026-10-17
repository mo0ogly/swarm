//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func summarySourceFixture(t *testing.T) (*Store, Work, map[string]any) {
	t.Helper()
	s := storeTest(t)
	w := memorySeed(t, s, taskTest(t, s, createTest(t, s)))
	q := map[string]any{"schema_version": 1, "work": w.ID, "task": "t1", "expected_revision": w.Revision, "dialogues": []DialogueRef{dialogueHits(w)[0].Ref}}
	return s, w, q
}

func TestMemorySummarySourceWrongMethodDoesNotReturnCatalog(t *testing.T) {
	s, _, _ := summarySourceFixture(t)
	req := httptest.NewRequest(http.MethodGet, "http://localhost:18787/api/v1/project-memory?action=summary-source", nil)
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "fixture-token"})
	response := httptest.NewRecorder()
	newWebHandler(s, "localhost:18787", "fixture-token").ServeHTTP(response, req)
	if response.Code != http.StatusMethodNotAllowed {
		t.Fatalf("wrong method became a successful catalogue: %d %s", response.Code, response.Body.String())
	}
}

func TestMemorySummarySourceInspectionNeverMigratesOldStore(t *testing.T) {
	s, _, q := summarySourceFixture(t)
	old := schemaVersion - 1
	if _, err := s.db.Exec("PRAGMA user_version=" + fmt.Sprint(old)); err != nil {
		t.Fatal(err)
	}
	code, out, stderr := summarySourceCLI(t, s, q)
	var version int
	if err := s.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil {
		t.Fatal(err)
	}
	if code != 2 || !strings.Contains(string(out)+stderr, "storage_upgrade_required") || version != old {
		t.Fatalf("source inspection migrated old store: code=%d version=%d want=%d out=%s stderr=%s", code, version, old, out, stderr)
	}
}

func TestMemorySummarySourceCLIAPIParityAndSessionProtection(t *testing.T) {
	s, _, q := summarySourceFixture(t)
	before := recoverySnapshotTest(t, s)
	code, cli, stderr := summarySourceCLI(t, s, q)
	if code != 0 {
		t.Fatal(code, stderr)
	}
	api := projectMemoryHTTP(t, s, "summary-source", q, 200)
	var value map[string]any
	if err := json.Unmarshal(cli, &value); err != nil {
		t.Fatal(err)
	}
	a, _ := json.Marshal(api)
	b, _ := json.Marshal(value)
	if !bytes.Equal(a, b) {
		t.Fatal("CLI/API source packets differ")
	}
	for _, auth := range []bool{false, true} {
		raw, _ := json.Marshal(q)
		req := httptest.NewRequest(http.MethodPost, "http://localhost:18787/api/v1/project-memory?action=summary-source", bytes.NewReader(raw))
		if auth {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "fixture-token"})
		}
		response := httptest.NewRecorder()
		newWebHandler(s, "localhost:18787", "fixture-token").ServeHTTP(response, req)
		if response.Code != 403 || strings.Contains(response.Body.String(), "Réponse 0") {
			t.Fatal("session/origin protection lost", response.Code)
		}
	}
	if recoverySnapshotTest(t, s) != before {
		t.Fatal("API source read mutated state")
	}
}

func TestMemorySummarySourceRejectsInvalidStaleAndIncompleteSelection(t *testing.T) {
	for _, scenario := range []string{"schema", "revision", "stale", "task", "brainstorm-target", "empty", "duplicate", "changed-ref", "foreign-work", "unknown-field"} {
		t.Run(scenario, func(t *testing.T) {
			s, w, q := summarySourceFixture(t)
			switch scenario {
			case "schema":
				q["schema_version"] = 2
			case "revision":
				q["expected_revision"] = 0
			case "stale":
				q["expected_revision"] = w.Revision - 1
			case "task":
				q["task"] = "absent"
			case "brainstorm-target":
				q["task"] = "old-0"
			case "empty":
				q["dialogues"] = []DialogueRef{}
			case "duplicate":
				q["dialogues"] = []DialogueRef{dialogueHits(w)[0].Ref, dialogueHits(w)[0].Ref}
			case "changed-ref":
				ref := dialogueHits(w)[0].Ref
				ref.SHA256 = strings.Repeat("0", 64)
				q["dialogues"] = []DialogueRef{ref}
			case "foreign-work":
				q["work"] = "foreign-work"
			case "unknown-field":
				q["automatic"] = true
			}
			before := recoverySnapshotTest(t, s)
			code, out, _ := summarySourceCLI(t, s, q)
			if code == 0 || len(out) != 0 {
				t.Fatal("invalid selection became source packet", code)
			}
			if recoverySnapshotTest(t, s) != before {
				t.Fatal("rejected request mutated state")
			}
		})
	}
}

func TestMemorySummarySourceRejectsChangedStoredAnswerAndInvalidWork(t *testing.T) {
	for _, scenario := range []string{"changed-answer", "empty-answer", "identity", "revision-column", "invalid-utf8"} {
		t.Run(scenario, func(t *testing.T) {
			s, w, q := summarySourceFixture(t)
			if scenario == "changed-answer" {
				w.Tasks[1].Answers[0].Text = "changed"
			}
			if scenario == "empty-answer" {
				w.Tasks[1].Answers[0].Text = ""
				q["dialogues"] = []DialogueRef{dialogueHits(w)[0].Ref}
			}
			if scenario == "identity" {
				w.ID = "other-work"
			}
			if scenario == "revision-column" {
				w.Revision++
			}
			raw, _ := json.Marshal(w)
			if scenario == "invalid-utf8" {
				// The current identity projection trigger rejects this malformed
				// write before an export can occur. Keep that earlier protection;
				// the source reader guard has its own isolated SQL fixture below.
				before := recoverySnapshotTest(t, s)
				if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", []byte{0xff}, q["work"]); err == nil {
					t.Fatal("invalid UTF-8 stored despite the identity projection boundary")
				}
				if before != recoverySnapshotTest(t, s) {
					t.Fatal("rejected malformed write mutated storage")
				}
				t.Log("invalid UTF-8 refused at storage boundary; source CLI not invoked")
				return
			}
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, q["work"]); err != nil {
				t.Fatal(err)
			}
			before := recoverySnapshotTest(t, s)
			code, out, _ := summarySourceCLI(t, s, q)
			if code == 0 || len(out) != 0 {
				t.Fatal("invalid stored source exported", code)
			}
			if recoverySnapshotTest(t, s) != before {
				t.Fatal("source rejection mutated state")
			}
		})
	}
}

func summarySourceCLI(t *testing.T, s *Store, q map[string]any) (int, []byte, string) {
	t.Helper()
	raw, err := json.Marshal(q)
	if err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(t.TempDir(), "request.json")
	if err := os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	var out, stderr bytes.Buffer
	code := run([]string{"--root", s.root, "--json", "memory", "summary-source", "--input", path}, &out, &stderr)
	return code, out.Bytes(), stderr.String()
}

func TestMemorySummarySourceCLIExportsExactSelectedSourcesWithoutMutation(t *testing.T) {
	s, w, q := summarySourceFixture(t)
	before := recoverySnapshotTest(t, s)
	code, raw, stderr := summarySourceCLI(t, s, q)
	if code != 0 {
		t.Fatalf("source export unavailable: code=%d stderr=%s body=%s", code, stderr, raw)
	}
	var packet struct {
		Schema  int    `json:"schema_version"`
		Kind    string `json:"kind"`
		Payload string `json:"payload"`
		SHA256  string `json:"sha256"`
	}
	if err := json.Unmarshal(raw, &packet); err != nil {
		t.Fatal(err)
	}
	if packet.Schema != 1 || packet.Kind != "swarm_summary_source_v1" || packet.SHA256 != hash([]byte(packet.Payload)) {
		t.Fatal("source packet lacks exact-byte binding")
	}
	var value struct {
		Scope struct {
			Work string `json:"work"`
			Task string `json:"task"`
		} `json:"scope"`
		Revision int    `json:"work_revision"`
		Project  string `json:"project_sha256"`
		Complete bool   `json:"provider_history_complete"`
		Audience string `json:"audience"`
		Sources  []struct {
			ID     string `json:"id"`
			Role   string `json:"role"`
			Kind   string `json:"kind"`
			Text   string `json:"text"`
			SHA256 string `json:"sha256"`
		} `json:"sources"`
	}
	if err := json.Unmarshal([]byte(packet.Payload), &value); err != nil {
		t.Fatal(err)
	}
	if value.Scope.Work != w.ID || value.Scope.Task != "t1" || value.Revision != w.Revision || value.Project != hash([]byte(s.root)) || value.Complete || value.Audience != "local_operator" || len(value.Sources) != 3 {
		t.Fatalf("wrong scope or completeness: %+v", value)
	}
	for _, source := range value.Sources {
		if source.SHA256 != hash([]byte(source.Text)) {
			t.Fatal("source text hash mismatch")
		}
	}
	if value.Sources[0].ID != "current-task-snapshot" || value.Sources[0].Kind != "task_snapshot" || !strings.Contains(value.Sources[0].Text, "tests PASS") {
		t.Fatal("current task snapshot missing")
	}
	if value.Sources[1].Text != dialogueHits(w)[0].Question || value.Sources[2].Text != dialogueHits(w)[0].Text || value.Sources[2].Role != "assistant" {
		t.Fatal("selected question/answer not copied exactly")
	}
	if strings.Contains(packet.Payload, "Réponse 5") || strings.Contains(packet.Payload, "old-5") {
		t.Fatal("unselected sibling exchange leaked")
	}
	if recoverySnapshotTest(t, s) != before {
		t.Fatal("read-only source export mutated state")
	}
}
