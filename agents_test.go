//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestProviderHelper(t *testing.T) {
	if os.Getenv("SWARM_TEST_PROVIDER") != "1" {
		return
	}
	raw, _ := io.ReadAll(os.Stdin)
	fmt.Println(`{"type":"item.started","item":{"type":"command_execution"}}`)
	if strings.Contains(string(raw), "TEST_SLEEP") {
		time.Sleep(60 * time.Second)
	}
	if strings.Contains(string(raw), "TEST_FAIL") {
		os.Exit(7)
	}
	fmt.Println("HANDOFF: observed fixture only")
	os.Exit(0)
}
func setupAgent(t *testing.T, s *Store) (Work, Launch) {
	t.Helper()
	w := taskTest(t, s, createTest(t, s))
	exe, e := os.Executable()
	if e != nil {
		t.Fatal(e)
	}
	t.Setenv("SWARM_TEST_PROVIDER", "1")
	p := Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: exe, Args: []string{"-test.run=^TestProviderHelper$"}, Env: []string{"SWARM_TEST_PROVIDER"}}}}
	b, _ := json.Marshal(p)
	if e = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), b, 0600); e != nil {
		t.Fatal(e)
	}
	return w, Launch{Schema: 1, EventID: newID("agent-"), Revision: w.Revision, TaskID: "t1", Provider: "fixture", Capture: true}
}
func waitAgent(t *testing.T, s *Store, id string, want func(Agent) bool) Agent {
	t.Helper()
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		a, e := s.agent(id)
		if e != nil {
			t.Fatal(e)
		}
		if want(a) {
			return a
		}
		time.Sleep(20 * time.Millisecond)
	}
	a, _ := s.agent(id)
	t.Fatalf("timeout état agent %+v", a)
	return a
}
func TestLaunchAtomicIdempotentAndWorkspaceExclusive(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, created, e := s.prepare(w.ID, r)
	if e != nil || !created {
		t.Fatal(e)
	}
	again, created, e := s.prepare(w.ID, r)
	if e != nil || created || again.ID != a.ID {
		t.Fatalf("retry duplicate %v", e)
	}
	r.Instruction = "changed"
	if _, _, e = s.prepare(w.ID, r); e == nil {
		t.Fatal("idempotency conflict accepted")
	}
	w, _ = s.get(w.ID)
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "second", Deliverable: "report", Criteria: []string{"proof"}})
	r.EventID = newID("agent-")
	r.TaskID = "t2"
	r.Revision = w.Revision
	if _, _, e = s.prepare(w.ID, r); e == nil {
		t.Fatal("workspace concurrency accepted")
	}
	after, _ := s.get(w.ID)
	task, _ := after.task("t2")
	if after.Revision != w.Revision || task.Status != "todo" {
		t.Fatal("partial launch mutation")
	}
	if e = s.reconcile(a.ID); e != nil {
		t.Fatal(e)
	}
}
func TestSupervisorSuccessIsNotAcceptance(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.supervise(a.ID); e != nil {
		t.Fatal(e)
	}
	a, _ = s.agent(a.ID)
	if a.Status != "completed" || a.ExitCode == nil || *a.ExitCode != 0 {
		t.Fatalf("unexpected result %+v", a)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "blocked" {
		t.Fatal("success must await handoff")
	}
	logs, e := s.logs(a.ID, 0)
	if e != nil {
		t.Fatal(e)
	}
	found := false
	for _, l := range logs {
		if strings.Contains(l.Message, "HANDOFF") {
			found = true
		}
	}
	if !found {
		t.Fatal("stdout not logged")
	}
}
func TestStopAndTimeoutConfirmExit(t *testing.T) {
	for _, mode := range []string{"stop", "timeout"} {
		t.Run(mode, func(t *testing.T) {
			s := storeTest(t)
			w, r := setupAgent(t, s)
			r.Instruction = "TEST_SLEEP"
			if mode == "timeout" {
				r.Timeout = 1
			}
			a, _, e := s.prepare(w.ID, r)
			if e != nil {
				t.Fatal(e)
			}
			done := make(chan error, 1)
			go func() { done <- s.supervise(a.ID) }()
			waitAgent(t, s, a.ID, func(a Agent) bool { return a.Status == "running" })
			if mode == "stop" {
				if e = s.stopAgent(a.ID); e != nil {
					t.Fatal(e)
				}
			}
			select {
			case e = <-done:
				if e != nil {
					t.Fatal(e)
				}
			case <-time.After(10 * time.Second):
				t.Fatal("supervisor did not stop")
			}
			a, _ = s.agent(a.ID)
			if a.Status != "interrupted" || processStamp(a.Child) != "" {
				t.Fatalf("not stopped %+v", a)
			}
		})
	}
}
func TestProviderFailureAndNoOutputCapture(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Instruction = "TEST_FAIL"
	r.Capture = false
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.supervise(a.ID); e != nil {
		t.Fatal(e)
	}
	a, _ = s.agent(a.ID)
	if a.Status != "failed" || *a.ExitCode != 7 {
		t.Fatal(a.Status)
	}
	logs, _ := s.logs(a.ID, 0)
	for _, l := range logs {
		if l.Kind == "output" {
			t.Fatal("capture without consent")
		}
	}
}
func TestPausedUnknownReconcileAndRestart(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if e := s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	if _, _, e := s.prepare(w.ID, r); e == nil {
		t.Fatal("launch while paused")
	}
	_ = s.pause(w.ID, false)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	a.Heartbeat = time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano)
	_ = s.saveAgent(a)
	if observedAgent(a) != "unknown/no-heartbeat" {
		t.Fatal("false active status")
	}
	if e = s.reconcile(a.ID); e != nil {
		t.Fatal(e)
	}
	state := &consoleState{}
	if _, e = s.consoleCommand(w.ID, "note "+a.ID+" reproduce issue", state); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	r.EventID = newID("agent-")
	r.Revision = w.Revision
	r.Previous = a.ID
	next, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if next.Attempt == a.Attempt || !strings.Contains(next.Prompt, a.ID) {
		t.Fatal("attempt not renewed")
	}
}
func TestControlScopeHandoffAndGatePreserved(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	other := createTest(t, s)
	state := &consoleState{}
	if _, e = s.consoleCommand(other.ID, "stop "+a.ID, state); e == nil {
		t.Fatal("cross work stop")
	}
	if _, e = s.consoleCommand(w.ID, "ready t1", state); e == nil {
		t.Fatal("live task modified")
	}
	_ = s.reconcile(a.ID)
	if e = os.WriteFile(filepath.Join(s.root, "handoff.md"), []byte("Tests observed, needs review"), 0600); e != nil {
		t.Fatal(e)
	}
	if _, e = s.consoleCommand(w.ID, "submit t1 handoff.md", state); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	if w.Tasks[0].Status != "submitted" || s.validGate(&w.Tasks[0]) {
		t.Fatal("gate bypass")
	}
	if _, e = s.consoleCommand(w.ID, "priority t1 9", state); e != nil {
		t.Fatal(e)
	}
	if s.priorities(w.ID)["t1"] != 9 {
		t.Fatal("priority missing")
	}
}
func TestTerminalLogBoundsAndNonTTY(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.log(a.ID, "output", "\x1b]52;c;AAAA\a\u202e"+strings.Repeat("x", 5000)); e != nil {
		t.Fatal(e)
	}
	logs, _ := s.logs(a.ID, 0)
	last := logs[len(logs)-1]
	if strings.ContainsAny(last.Message, "\x1b\a\u202e") || len(last.Message) > 2100 {
		t.Fatal("unsafe/unbounded log")
	}
	before, _ := s.get(w.ID)
	var out bytes.Buffer
	f, e := os.Open(os.DevNull)
	if e != nil {
		t.Fatal(e)
	}
	defer f.Close()
	if e = s.console(w.ID, f, &out, false); e != nil {
		t.Fatal(e)
	}
	if !strings.Contains(out.String(), "unconfirmed") {
		t.Fatal("snapshot missing observed status")
	}
	after, _ := s.get(w.ID)
	if before.Revision != after.Revision {
		t.Fatal("read console mutated work")
	}
}
func TestMigrationOnePreservesWork(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	_, e := s.db.Exec("DROP TABLE assist_previews; DROP TABLE assist_reservations; DROP TABLE assist_turns; DROP TABLE decisions; DROP TABLE session_visits; DROP TABLE budgets; DROP TABLE reservations; DROP TABLE agent_logs; DROP TABLE agents; DROP TABLE cockpit_events; DROP TABLE cockpit_controls; DROP TABLE cockpit_tasks; PRAGMA user_version=1;")
	if e != nil {
		t.Fatal(e)
	}
	s.db.Close()
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	got, e := reopened.get(w.ID)
	if e != nil || got.Revision != w.Revision {
		t.Fatal("migration lost work", e)
	}
	var version int
	_ = reopened.db.QueryRow("PRAGMA user_version").Scan(&version)
	if version != 4 {
		t.Fatal(version)
	}
}
func TestLaunchRejectsEscapeAndUnsatisfiedDependencies(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Workspace = t.TempDir()
	if _, _, e := s.prepare(w.ID, r); e == nil {
		t.Fatal("external cwd accepted")
	}
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "dependent", Deliverable: "proof", Criteria: []string{"pass"}, Depends: []string{"t1"}})
	r.Workspace = "."
	r.Revision = w.Revision
	r.TaskID = "t2"
	if _, _, e := s.prepare(w.ID, r); e == nil {
		t.Fatal("dependency gate bypass")
	}
}

func TestArchiveTelemetryNeverReactivatesAgents(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	_ = s.log(a.ID, "operator-note", "historical note")
	dest := filepath.Join(t.TempDir(), "bundle.zip")
	if e = s.export(w.ID, dest); e != nil {
		t.Fatal(e)
	}
	other := storeTest(t)
	if _, e = other.importBundle(dest); e != nil {
		t.Fatal(e)
	}
	agents, e := other.agents(w.ID)
	if e != nil || len(agents) != 0 {
		t.Fatal("historical session activated")
	}
	b, e := os.ReadFile(filepath.Join(other.root, ".swarm/imports", w.ID, "manifest.json"))
	if e != nil {
		t.Fatal(e)
	}
	var bundle Bundle
	if e = json.Unmarshal(b, &bundle); e != nil {
		t.Fatal(e)
	}
	if bundle.Cockpit == nil || len(bundle.Cockpit.Agents) != 1 {
		t.Fatal("telemetry lost")
	}
}
func TestLogCursorAndOverlappingWorkspace(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	for i := 0; i < 205; i++ {
		if e = s.log(a.ID, "fixture", fmt.Sprint(i)); e != nil {
			t.Fatal(e)
		}
	}
	first, e := s.logs(a.ID, -1)
	if e != nil || len(first) != 200 {
		t.Fatal("first cursor page", e)
	}
	next, e := s.logs(a.ID, first[len(first)-1].Seq)
	if e != nil || len(next) != 6 {
		t.Fatal("cursor skipped records", e, len(next))
	}
	w, _ = s.get(w.ID)
	w = applyTest(t, s, w, "task.add", Request{ID: "nested", Title: "nested", Deliverable: "report", Criteria: []string{"ok"}})
	_ = os.Mkdir(filepath.Join(s.root, "nested"), 0700)
	r.EventID = newID("agent-")
	r.TaskID = "nested"
	r.Workspace = "nested"
	r.Revision = w.Revision
	if _, _, e = s.prepare(w.ID, r); e == nil {
		t.Fatal("overlapping workspace accepted")
	}
}

func TestUnknownProcessAndOutputFloodRemainBounded(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	a.Supervisor = os.Getpid()
	a.SupervisorStamp = "not-this-process"
	a.Heartbeat = now()
	if observedAgent(a) != "unknown/supervisor-lost" {
		t.Fatal("PID reuse shown as live")
	}
	sink := &outputSink{s: s, id: a.ID, capture: true}
	for i := 0; i < 1010; i++ {
		_, _ = sink.Write([]byte("\n"))
	}
	sink.flush()
	if !sink.truncated || sink.lines != 1000 {
		t.Fatal("blank output flood unbounded")
	}
	rendered := s.renderConsole(w.ID, &consoleState{}, 80, 32, "")
	for _, line := range strings.Split(rendered, "\n") {
		if len([]rune(line)) > 80 {
			t.Fatal("terminal width exceeded")
		}
	}
}

func TestPlainConsoleAndZeroDimensions(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	input, writer, e := os.Pipe()
	if e != nil {
		t.Fatal(e)
	}
	defer input.Close()
	go func() { defer writer.Close(); _, _ = writer.Write([]byte("help\nq\n")) }()
	var output bytes.Buffer
	if e = s.plainConsole(w.ID, input, &output, false); e != nil {
		t.Fatal(e)
	}
	if strings.Contains(output.String(), "\x1b") || !strings.Contains(output.String(), "COMMANDES") {
		t.Fatal("plain mode unusable")
	}
	width, height := consoleDimensions(0, 0)
	if width != 80 || height != 24 {
		t.Fatal("zero terminal size")
	}
	t.Setenv("TERM", "dumb")
	file, e := os.Open(os.DevNull)
	if e != nil {
		t.Fatal(e)
	}
	defer file.Close()
	output.Reset()
	if e = s.console(w.ID, file, &output, false); e != nil {
		t.Fatal(e)
	}
	var snapshot map[string]any
	if e = json.Unmarshal(output.Bytes(), &snapshot); e != nil {
		t.Fatal("non-TTY must remain JSON", e)
	}
}

func TestDashboardBoundsAndActionReview(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	c := &consoleState{}
	for _, size := range [][2]int{{80, 24}, {140, 40}, {64, 20}, {20, 8}} {
		frame := s.renderDashboard(w.ID, c, size[0], size[1], "")
		lines := strings.Split(frame, "\r\n")
		if len(lines) > size[1] {
			t.Fatalf("frame exceeds height: %d", len(lines))
		}
		for _, line := range lines {
			if len([]rune(line)) > size[0] {
				t.Fatal("frame exceeds width")
			}
		}
	}
	s.openTaskDialog(w.ID, c)
	agents, e := s.agents(w.ID)
	if e != nil || len(agents) != 0 {
		t.Fatal("menu launched a process")
	}
	c.focus = 2
	c.move(-1)
	if c.logOffset != 1 {
		t.Fatal("log scrolling")
	}
	if !strings.Contains(s.renderDashboard(w.ID, c, 100, 30, ""), "Lancer avec un fournisseur") {
		t.Fatal("missing action menu")
	}
}
