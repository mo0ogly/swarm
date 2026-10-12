//go:build linux

package engine

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"testing"
)

func TestControlRuntimeBudgetAppliesToGoCommandDeadlineAndKeepsApprovedPolicy(t *testing.T) {
	s, w, _, _ := recoveredResultFixture(t)
	cfg := validationBudgetFixture(t, s, w.ID, 12, "explicit-go-deadline")
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "explicit-go-inner-deadline", Scope: ScopeRole, Mission: w.ID, Key: "validation-go-test", Revision: 0, Values: RunLimits{ToolSeconds: 10}, Reason: "Explicit internal Go deadline below the process budget"}); err != nil {
		t.Fatal(err)
	}
	root := t.TempDir()
	files := map[string]string{
		"go.mod":           "module deadline.fixture\n\ngo 1.23\n",
		"deadline_test.go": "package deadline\nimport (\"testing\";\"time\")\nfunc TestBounded(t *testing.T){time.Sleep(150*time.Millisecond)}\n",
	}
	for name, body := range files {
		if err := os.WriteFile(filepath.Join(root, name), []byte(body), 0600); err != nil {
			t.Fatal(err)
		}
	}
	// Warm only this tiny fixture so compilation cannot obscure the inner deadline.
	warm := exec.Command("go", "test", "-c", "-o", filepath.Join(root, "fixture.test"))
	warm.Dir = root
	if out, err := warm.CombinedOutput(); err != nil {
		t.Fatalf("fixture compilation: %v %s", err, out)
	}
	control := ValidationControl{ID: "go-control", Command: []string{"go", "test", ".", "-run", "^TestBounded$", "-count=1", "-timeout=1ms"}, Timeout: 1, Criteria: []int{1}, Justification: "Bounded original control"}
	original := append([]string(nil), control.Command...)
	result, output, _ := s.runConfiguredValidationControl(w.ID, root, control)
	if !result.Passed {
		t.Fatalf("configured budget left inner deadline unchanged: %s %s", result.Summary, output)
	}
	if !reflect.DeepEqual(control.Command, original) || control.Timeout != 1 {
		t.Fatal("approved historical control changed")
	}
	expected := append([]string(nil), original...)
	expected[len(expected)-1] = "-timeout=10s"
	if !reflect.DeepEqual(result.Command, expected) || result.RuntimeLimits == nil || result.RuntimeLimits.Revision != cfg.Revision || result.CommandRuntimeLimits == nil || result.CommandRuntimeLimits.Values.ToolSeconds != 10 {
		t.Fatalf("effective command/budget not recorded: %+v", result)
	}
}

func TestConfiguredGoTestDeadlinePreservesOtherArgumentsAndOriginal(t *testing.T) {
	for _, row := range []struct {
		name          string
		command, want []string
		seconds       int
	}{
		{"separate", []string{"go", "test", "./...", "-run", "TestBudget", "-timeout", "25s"}, []string{"go", "test", "./...", "-run", "TestBudget", "-timeout", "10s"}, 10},
		{"attached", []string{"go", "test", "./...", "-timeout=25s"}, []string{"go", "test", "./...", "-timeout=10s"}, 10},
		{"forwarded", []string{"go", "test", ".", "-args", "-test.timeout=25s", "-custom=value"}, []string{"go", "test", ".", "-args", "-test.timeout=10s", "-custom=value"}, 10},
		{"default", []string{"go", "test", "."}, []string{"go", "test", ".", "-timeout=10s"}, 10},
		{"before-forwarding", []string{"go", "test", ".", "-args", "-custom=value"}, []string{"go", "test", ".", "-timeout=10s", "-args", "-custom=value"}, 10},
		{"other-program", []string{"python3", "-c", "print('-timeout=25s')"}, []string{"python3", "-c", "print('-timeout=25s')"}, 10},
		{"other-go-command", []string{"go", "build", "."}, []string{"go", "build", "."}, 10},
		{"no-explicit-budget", []string{"go", "test", ".", "-timeout=25s"}, []string{"go", "test", ".", "-timeout=25s"}, 0},
	} {
		t.Run(row.name, func(t *testing.T) {
			original := append([]string(nil), row.command...)
			got := configuredGoTestDeadline(row.command, row.seconds)
			if !reflect.DeepEqual(got, row.want) || !reflect.DeepEqual(row.command, original) {
				t.Fatalf("argument corruption: got %v want %v original %v", got, row.want, row.command)
			}
		})
	}
}
func TestControlGoBudgetRejectsAbsentOrInsufficientOuterAllowance(t *testing.T) {
	for _, seconds := range []int{0, 10, 9} {
		t.Run(fmt.Sprint(seconds), func(t *testing.T) {
			s, w, _, _ := recoveredResultFixture(t)
			if seconds > 0 {
				validationBudgetFixture(t, s, w.ID, seconds, "outer-budget")
			}
			_, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "inner-budget", Scope: ScopeRole, Mission: w.ID, Key: validationGoRuntimeRole, Revision: 0, Values: RunLimits{ToolSeconds: 10}, Reason: "Test invalid internal deadline bound"})
			if err != nil {
				t.Fatal(err)
			}
			if _, _, err = s.validationRuntimeLimits(w.ID); err == nil {
				t.Fatal("internal budget has no strictly larger explicit process allowance")
			}
		})
	}
}
func TestControlGoBudgetChangeInvalidatesReservedCombinedDigest(t *testing.T) {
	s, w, a, _ := recoveredResultFixture(t)
	validationBudgetFixture(t, s, w.ID, 12, "outer-budget")
	inner, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "inner-budget", Scope: ScopeRole, Mission: w.ID, Key: validationGoRuntimeRole, Revision: 0, Values: RunLimits{ToolSeconds: 10}, Reason: "Explicit attributed internal deadline"})
	if err != nil {
		t.Fatal(err)
	}
	_, digest, err := s.validationRuntimeLimits(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	item, _ := s.managedAttempt(a.ID)
	task.IntegrationRetries = []IntegrationRetry{{Agent: a.ID, Attempt: a.Attempt, Result: item.Result, Candidate: w.Planning.Repository.Candidate, Contract: managedReviewContract(w, task.ID), RuntimeBudgetDigest: digest}}
	if err = s.integrationRetryGuard(w, task, a, item); err != nil {
		t.Fatal(err)
	}
	_, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "inner-replaced", Scope: ScopeRole, Mission: w.ID, Key: validationGoRuntimeRole, Revision: inner.Revision, Values: RunLimits{ToolSeconds: 11}, Reason: "Replace only internal command allowance"})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.integrationRetryGuard(w, task, a, item); err == nil {
		t.Fatal("internal budget changed without invalidating reservation")
	}
}
func TestControlGoBudgetStorageUpgradeKeepsOriginalReceiptAndRefusesOldWriter(t *testing.T) {
	s, w, _, _ := recoveredResultFixture(t)
	inner := RunLimitsConfigEntry{Scope: ScopeRole, MissionID: w.ID, ScopeKey: validationGoRuntimeRole, Revision: 1, Values: RunLimits{ToolSeconds: 10}}
	var err error
	w, err = s.mutate(w.ID, "fixture", "retain-inner-budget", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].AutoValidation = &AutomaticValidation{Controls: []ValidationControlResult{{ID: "go-control", Command: []string{"go", "test", ".", "-timeout=10s"}, CommandRuntimeLimits: &inner}}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	var before []byte
	if err = s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&before); err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("PRAGMA user_version=30"); err != nil {
		t.Fatal(err)
	}
	if _, err = s.mutate(w.ID, "old-writer", "forbidden-old-write", w.Revision, []byte(`{}`), func(*Work) error { return nil }); commandFailure(err).Code != "storage_version_conflict" {
		t.Fatal("old writer allowed", err)
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	if old, err := openStoreWithMigration(s.root, false, false); err == nil {
		old.db.Close()
		t.Fatal("old storage opened without upgrade")
	}
	up, err := openStoreWithMigration(s.root, false, true)
	if err != nil {
		t.Fatal(err)
	}
	defer up.db.Close()
	var after []byte
	if err = up.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&after); err != nil || string(before) != string(after) {
		t.Fatal("migration changed original receipt", err)
	}
	got, err := up.get(w.ID)
	if err != nil || got.Tasks[0].AutoValidation.Controls[0].CommandRuntimeLimits == nil {
		t.Fatal("attributed budget lost", err)
	}
	files, err := filepath.Glob(filepath.Join(s.root, ".swarm/state-pre-v31-*.db"))
	if err != nil || len(files) != 1 {
		t.Fatal("migration backup absent", err)
	}
	info, err := os.Stat(files[0])
	if err != nil || info.Mode().Perm() != 0600 {
		t.Fatal("backup not private", err)
	}
}
