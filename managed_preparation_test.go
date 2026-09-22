//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

func TestManagedPreparedLaunchExplainsExistingOperation(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-original", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete the assigned task", Timeout: 60}
	if _, e := s.ensureManagedAttempt(w, r); e != nil {
		t.Fatal(e)
	}
	r.EventID = "prepared-new-click"
	_, created, e := s.prepare(w.ID, r)
	if e == nil || created || commandFailure(e).Code != "prepared_launch_exists" {
		t.Fatalf("prepared operation must be identified without overwriting its copy: created=%v error=%v code=%s", created, e, commandFailure(e).Code)
	}
}

func TestManagedPreparedLaunchResumeRetainsCopyAndDoesNotDuplicate(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-resume", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete the assigned task", Timeout: 60, Limits: &RunLimits{MaxToolCalls: 12}}
	path, e := s.ensureManagedAttempt(w, r)
	if e != nil {
		t.Fatal(e)
	}
	marker := filepath.Join(path, "keep.txt")
	if e = os.WriteFile(marker, []byte("retained\n"), 0600); e != nil {
		t.Fatal(e)
	}
	reopened, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer reopened.db.Close()
	a, created, e := reopened.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
	if e != nil || !created {
		t.Fatalf("resume: %v created=%v", e, created)
	}
	if a.ID != r.EventID || a.CWD != path || a.Limits.MaxToolCalls != 12 {
		t.Fatalf("identity/settings lost: %+v", a)
	}
	if got, e := os.ReadFile(marker); e != nil || string(got) != "retained\n" {
		t.Fatal("prepared work lost", e)
	}
	for range 2 {
		got, again, e := reopened.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
		if e != nil || again || got.ID != a.ID {
			t.Fatalf("replay duplicated launch: %v %v", again, e)
		}
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks[0].Attempts) != 1 {
		t.Fatal("duplicate attempt", after.Tasks[0].Attempts)
	}
}

func TestManagedPreparedLaunchRefusesChangedContract(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-stale", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete the assigned task", Timeout: 60}
	path, e := s.ensureManagedAttempt(w, r)
	if e != nil {
		t.Fatal(e)
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "first", Next: "A changed recovery instruction"})
	if _, created, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision); e == nil || created {
		t.Fatal("changed task resumed", e)
	}
	if _, e := os.Stat(path); e != nil {
		t.Fatal("rejected preparation deleted", e)
	}
	if agents, _ := s.agents(w.ID); len(agents) != 0 {
		t.Fatal("provider reserved despite stale contract")
	}
}

func TestManagedPreparedLaunchRecoversInsertFailure(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-insert-fault", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete and prove the task", Timeout: 60}
	// Fault injection is confined to this test's temporary database.
	if _, e := s.db.Exec("CREATE TRIGGER fail_agent_insert BEFORE INSERT ON agents BEGIN SELECT RAISE(ABORT, 'injected persistence failure'); END"); e != nil {
		t.Fatal(e)
	}
	if _, created, e := s.prepare(w.ID, r); e == nil || created {
		t.Fatal("expected persistence failure", e)
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks[0].Attempts) != 0 {
		t.Fatal("failed transaction consumed an attempt")
	}
	task, _ := after.task("first")
	actions := s.taskActions(&after, task, nil)
	found := false
	for _, action := range actions {
		if action.Kind == "resume-launch" {
			found = action.Disponible && action.Conseillee && action.Prepared.ID == r.EventID
		}
	}
	if !found {
		t.Fatal("no public recovery action", actions)
	}
	status, e := s.missionStatus(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if status.Tasks[0].State != "intervention" || status.Tasks[0].Label != "Reprendre le lancement préparé" {
		t.Fatal("mission hides pending launch", status.Tasks[0])
	}

	if _, e := s.db.Exec("DROP TRIGGER fail_agent_insert"); e != nil {
		t.Fatal(e)
	}
	a, created, e := s.resumePreparedLaunch(w.ID, r.EventID, after.Revision)
	if e != nil || !created {
		t.Fatal("resume", e)
	}
	if a.DeliveryVersion != 1 || !strings.Contains(a.Prompt, "docs/first.delivery.json") || !strings.Contains(a.Prompt, a.Attempt) {
		t.Fatal("missing producer delivery contract")
	}
}

func TestManagedPreparedLaunchConcurrentResume(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-concurrent", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete", Timeout: 60}
	if _, e := s.ensureManagedAttempt(w, r); e != nil {
		t.Fatal(e)
	}
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	var wg sync.WaitGroup
	for _, store := range []*Store{s, other} {
		wg.Add(1)
		go func(s *Store) { defer wg.Done(); _, _, _ = s.resumePreparedLaunch(w.ID, r.EventID, w.Revision) }(store)
	}
	wg.Wait()
	a, _, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
	if e != nil || a.ID != r.EventID {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	agents, _ := s.agents(w.ID)
	if len(after.Tasks[0].Attempts) != 1 || len(agents) != 1 {
		t.Fatal("duplicate reservation", len(agents), after.Tasks[0].Attempts)
	}
}

func TestManagedPreparedLaunchRejectsChangedProvider(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-provider", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete", Timeout: 60}
	stale := r
	stale.ProviderDigest = "outdated-provider"
	if _, e := s.ensureManagedAttempt(w, stale); e == nil {
		t.Fatal("stale provider proposal produced a recoverable copy")
	}

	if _, e := s.ensureManagedAttempt(w, r); e != nil {
		t.Fatal(e)
	}
	ps, e := s.providers()
	if e != nil {
		t.Fatal(e)
	}
	p := ps.Providers[r.Provider]
	p.Args = []string{"changed"}
	ps.Providers[r.Provider] = p
	b, _ := json.Marshal(ps)
	if e = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), b, 0600); e != nil {
		t.Fatal(e)
	}
	if _, created, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision); e == nil || created {
		t.Fatal("changed provider resumed", e)
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 0 {
		t.Fatal("created despite changed provider")
	}
}

func TestManagedPreparedLaunchRecoversFilesystemCheckpoint(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-checkpoint", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete", Timeout: 60}
	if _, e := s.ensureManagedAttempt(w, r); e != nil {
		t.Fatal(e)
	}
	// Reproduce rename completed, attribution not yet committed to SQLite.
	if _, e := s.db.Exec("DELETE FROM managed_attempts WHERE agent_id=?", r.EventID); e != nil {
		t.Fatal(e)
	}
	task, _ := w.task("first")
	p, e := s.preparedLaunchForTask(w, task)
	if e != nil || p == nil || !p.Ready || p.ID != r.EventID {
		t.Fatal("checkpoint undiscoverable", p, e)
	}
	if _, created, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision); e != nil || !created {
		t.Fatal("checkpoint resume", e)
	}
}

func TestManagedPreparedLaunchStopsAutomaticRepeat(t *testing.T) {
	s, w := managedFixture(t)
	if e := s.setProfile(w.ID, "", LaunchProfile{Provider: "managed-review-fixture", Role: "worker", Workspace: filepath.Join(s.root, "project"), Instruction: "Complete"}, -1); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	r := Launch{Schema: 1, EventID: "prepared-no-loop", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete", Timeout: 60}
	if _, e := s.ensureManagedAttempt(w, r); e != nil {
		t.Fatal(e)
	}
	second := r
	second.TaskID, second.EventID = "second", "prepared-no-loop-second"
	if _, e := s.ensureManagedAttempt(w, second); e != nil {
		t.Fatal(e)
	}
	for range 2 {
		if started, e := s.dispatch(w.ID); e != nil || len(started) != 0 {
			t.Fatal("automatic duplicate", started, e)
		}
	}
	agents, _ := s.agents(w.ID)
	if len(agents) != 0 {
		t.Fatal("prepared launch repeated automatically")
	}
	var count int
	if e := s.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='dispatch'", w.ID).Scan(&count); e != nil || count != 1 {
		t.Fatal("repeated recovery notices", count, e)
	}
}

func TestManagedPreparedLaunchCapsProviderBudget(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-budget", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete the assigned task", Timeout: 60, Limits: &RunLimits{MaxToolCalls: 120}}
	path, e := s.ensureManagedAttempt(w, r)
	if e != nil {
		t.Fatal(e)
	}
	a, created, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
	if e != nil || !created {
		t.Fatalf("resume: created=%v error=%v", created, e)
	}
	if a.ID != r.EventID || a.CWD != path || a.Limits.MaxToolCalls != 100 {
		t.Fatalf("wrong identity or budget: %+v", a)
	}
	record, e := s.readPreparedLaunch(w, r.EventID)
	if e != nil || record.Request.Limits.MaxToolCalls != 120 {
		t.Fatal("original budget was rewritten", e)
	}
	again, created, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
	if e != nil || created || again.ID != a.ID {
		t.Fatal("replay duplicated launch", e)
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks[0].Attempts) != 1 {
		t.Fatal("duplicate attempt")
	}
}

func TestPreparedBudgetCompatibilityRejectsOtherChanges(t *testing.T) {
	saved := Launch{Instruction: "original", Provider: "fixture", Limits: &RunLimits{MaxToolCalls: 120, ToolSeconds: 300}}
	for _, tc := range []struct {
		name        string
		calls       int
		instruction string
		seconds     int
		ok          bool
	}{
		{"lower", 100, "original", 300, true}, {"same", 120, "original", 300, true},
		{"raise", 121, "original", 300, false}, {"inherit", 0, "original", 300, false},
		{"negative", -1, "original", 300, false}, {"instruction", 100, "changed", 300, false},
		{"other limit", 100, "original", 600, false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			request := saved
			request.Instruction = tc.instruction
			request.Limits = &RunLimits{MaxToolCalls: tc.calls, ToolSeconds: tc.seconds}
			if got := preparedRequestCompatible(saved, request); got != tc.ok {
				t.Fatalf("compatible=%v", got)
			}
		})
	}
	request := saved
	request.Limits = nil
	if preparedRequestCompatible(saved, request) {
		t.Fatal("removed limits accepted")
	}
}
