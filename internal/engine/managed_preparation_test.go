//go:build linux

package engine

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
	type result struct {
		agent   Agent
		created bool
		err     error
	}
	results := make(chan result, 2)
	var wg sync.WaitGroup
	for _, store := range []*Store{s, other} {
		wg.Add(1)
		go func(s *Store) {
			defer wg.Done()
			a, created, err := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
			results <- result{a, created, err}
		}(store)
	}
	wg.Wait()
	close(results)
	createdCount := 0
	for outcome := range results {
		if outcome.err != nil {
			conflict, ok := outcome.err.(*CommandError)
			if outcome.created || outcome.agent.ID != "" || !(outcome.err.Error() == "une opération Git est déjà en cours pour cette mission" || (ok && conflict.Code == "revision_conflict")) {
				t.Fatalf("unexpected concurrent resume failure: %v", outcome.err)
			}
			continue // Explicit contention is allowed, but another concurrent call must create.
		}
		if outcome.agent.ID != r.EventID {
			t.Fatal("concurrent resume returned a different agent")
		}
		if outcome.created {
			createdCount++
		}
	}
	if createdCount != 1 {
		t.Fatalf("concurrent calls created %d agents, want exactly one", createdCount)
	}
	a, created, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
	if created {
		t.Fatal("sequential replay created the agent instead of the concurrent calls")
	}
	if e != nil || a.ID != r.EventID {
		t.Fatal(e)
	}
	after, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	agents, e := s.agents(w.ID)
	if e != nil {
		t.Fatal(e)
	}
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

func TestManagedPreparedReplayRejectsUnboundIdentity(t *testing.T) {
	for _, kind := range []string{"missing-record", "wrong-task", "wrong-attempt", "missing-manifest"} {
		t.Run(kind, func(t *testing.T) {
			s, w := managedFixture(t)
			req := Launch{Schema: 1, EventID: "bound-preparation", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete task", Timeout: 60}
			if _, err := s.ensureManagedAttempt(w, req); err != nil {
				t.Fatal(err)
			}
			a, created, err := s.resumePreparedLaunch(w.ID, req.EventID, w.Revision)
			if err != nil || !created {
				t.Fatal(err)
			}
			switch kind {
			case "missing-record":
				_, err = s.db.Exec("DELETE FROM managed_attempts WHERE agent_id=?", a.ID)
			case "wrong-task":
				_, err = s.db.Exec("UPDATE managed_attempts SET task_id='other-task' WHERE agent_id=?", a.ID)
			case "wrong-attempt":
				a.Attempt = "unrelated-attempt"
				err = s.saveAgent(a)
			case "missing-manifest":
				err = os.Remove(filepath.Join(w.Planning.Repository.Storage, "copy-"+a.ID+".json"))
			}
			if err != nil {
				t.Fatal(err)
			}
			if _, created, err = s.resumePreparedLaunch(w.ID, a.ID, w.Revision); err == nil || created {
				t.Fatal("unbound agent returned as resumed preparation", kind)
			}
		})
	}
}

func TestManagedPreparedResumeAfterIdenticalTreeRequalification(t *testing.T) {
	for _, mode := range []string{"identical-tree", "changed-task", "changed-tree"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			r := Launch{Schema: 1, EventID: "prepared-before-requalification", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete the assigned task", Timeout: 60}
			path, e := s.ensureManagedAttempt(w, r)
			if e != nil {
				t.Fatal(e)
			}
			bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
			base := w.Planning.Repository.Candidate
			tree, e := managedGit(bare, "rev-parse", base+"^{tree}")
			if e != nil {
				t.Fatal(e)
			}
			if mode == "changed-tree" {
				tree, e = managedGit(bare, "mktree")
				if e != nil {
					t.Fatal(e)
				}
			}
			candidate, e := managedGit(bare, "commit-tree", tree, "-p", base, "-m", "Requalification, unchanged tree")
			if e != nil {
				t.Fatal(e)
			}
			w.Planning.Repository.Candidate = candidate
			if mode == "changed-task" {
				w.Tasks[0].Next = "Different task contract"
			}
			raw, _ := json.Marshal(w)
			if _, e = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
				t.Fatal(e)
			}
			a, created, e := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision)
			if mode != "identical-tree" {
				if e == nil || created {
					t.Fatal("changed task accepted")
				}
				return
			}
			if e != nil || !created || a.CWD != path {
				t.Fatal("identical tree restart refused", created, e)
			}
		})
	}
}

func TestManagedInvalidLimitsDoNotCreatePendingLaunch(t *testing.T) {
	s, w := managedFixture(t)
	ps, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	p := ps.Providers["managed-review-fixture"]
	p.Limits.SilenceSeconds = 120
	ps.Providers["limited-worker"] = p
	raw, _ := json.Marshal(ps)
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	r := Launch{Schema: 1, EventID: "invalid-limits", Revision: w.Revision, TaskID: "first", Provider: "limited-worker", Role: "worker", Timeout: 60, Limits: &RunLimits{SilenceSeconds: 180}}
	if _, created, err := s.prepare(w.ID, r); err == nil || created || !strings.Contains(err.Error(), "limite de mission") {
		t.Fatalf("expected limits refusal: %v %v", created, err)
	}
	pending, err := s.preparedLaunchForTask(w, &w.Tasks[0])
	if err != nil || pending != nil {
		t.Fatalf("invalid launch left preparation: %+v %v", pending, err)
	}
	if _, err = os.Stat(managedCopyRoot(w.Planning.Repository, "first", 1)); !os.IsNotExist(err) {
		t.Fatalf("invalid launch created a copy: %v", err)
	}
}
