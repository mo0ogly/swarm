//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"
)

func TestActionsConfianceT6LaboratoryHelper(t *testing.T) {
	if os.Args[len(os.Args)-1] == "_trust_laboratory_parent" {
		runLaboratoryProcess(context.Background(), laboratoryTestInput(), []string{os.Args[0], "-test.run=^TestActionsConfianceT6LaboratoryHelper$", "--", os.Args[len(os.Args)-2], "_trust_laboratory_wait"})
		os.Exit(0)
	}

	if os.Args[len(os.Args)-1] == "_trust_laboratory_wait" {
		os.WriteFile(os.Args[len(os.Args)-2], []byte(strconv.Itoa(os.Getpid())), 0600)
		time.Sleep(time.Hour)
		os.Exit(0)
	}

	if os.Args[len(os.Args)-1] == "_trust_laboratory" {
		os.Exit(runLaboratoryChild(os.Stdin, os.Stdout))
	}
}
func laboratoryTestArgv() []string {
	return []string{os.Args[0], "-test.run=^TestActionsConfianceT6LaboratoryHelper$", "--", "_trust_laboratory"}
}
func laboratoryTestInput() TrustLaboratoryInput {
	return TrustLaboratoryInput{Version: 1, Digest: hashMust(laboratoryCorpus()), Seed: "18446744073709551615", Scenarios: append([]string{}, laboratoryScenarioIDs...), Connector: "local-ledger-simulation-v1"}
}
func TestActionsConfianceT6LaboratorySealedCorpus(t *testing.T) {
	input := laboratoryTestInput()
	result, err := runLaboratoryProcess(context.Background(), input, laboratoryTestArgv())
	if err != nil {
		t.Fatal("native fixed corpus failed", err)
	}
	if len(result.Cases) != 5 || len(result.Containment.Checks) != 14 {
		t.Fatal("missing native cases or boundary assertions")
	}
	again, err := runLaboratoryProcess(context.Background(), input, laboratoryTestArgv())
	if err != nil || again.DeterministicDigest != result.DeterministicDigest {
		t.Fatal("replay differs", err)
	}
}
func labTestCall(t *testing.T, s *Store, op string, body any, runner func(context.Context, TrustLaboratoryInput) (TrustLaboratoryResult, error)) (any, error) {
	t.Helper()
	raw, _ := json.Marshal(body)
	return s.laboratoryOperationUsing(context.Background(), op, raw, trustAdminToken, runner)
}
func labTestEnable(t *testing.T, s *Store) {
	t.Helper()
	st, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	if _, err = labTestCall(t, s, "policy-apply", map[string]any{"schema_version": 1, "event_id": "lab-enable", "expected_revision": st.Revision, "reason": "enable isolated laboratory", "policy": map[string]any{"enabled": true, "timeout_seconds": 15, "max_concurrent_runs": 1}}, nil); err != nil {
		t.Fatal(err)
	}
}
func TestActionsConfianceT6LaboratoryPersistence(t *testing.T) {
	s, _ := trustFixture(t)
	labTestEnable(t, s)
	st, _ := s.loadTrustState()
	body := map[string]any{"schema_version": 1, "event_id": "lab-run", "expected_revision": st.Revision, "reason": "fixed native corpus fixture", "input": laboratoryTestInput()}
	runner := func(ctx context.Context, in TrustLaboratoryInput) (TrustLaboratoryResult, error) {
		return runLaboratoryProcess(ctx, in, laboratoryTestArgv())
	}
	v, err := labTestCall(t, s, "run", body, runner)
	if err != nil {
		t.Fatal(err)
	}
	run := v.(TrustLaboratoryRun)
	if run.Status != "passed" {
		t.Fatal(run.Status)
	}
	before := secretStateBytes(t, s)
	if _, err = labTestCall(t, s, "run", body, func(context.Context, TrustLaboratoryInput) (TrustLaboratoryResult, error) {
		t.Fatal("replay launched duplicate")
		return TrustLaboratoryResult{}, nil
	}); err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("exact replay changed history")
	}
	reopened := &Store{root: s.root}
	if _, err = labTestCall(t, reopened, "result", map[string]any{"run_id": run.ID}, nil); err != nil {
		t.Fatal("reopen lost report", err)
	}
	replay := map[string]any{"schema_version": 1, "event_id": "lab-replay", "expected_revision": st.Revision, "reason": "explicit versioned scenario replay", "run_id": run.ID}
	v, err = labTestCall(t, reopened, "replay", replay, runner)
	if err != nil {
		t.Fatal(err)
	}
	child := v.(TrustLaboratoryRun)
	if child.Parent != run.ID || child.Result.DeterministicDigest != run.Result.DeterministicDigest {
		t.Fatal("replay lost attribution or determinism")
	}
}

func laboratoryStub(in TrustLaboratoryInput) TrustLaboratoryResult {
	checks := map[string]bool{}
	for _, n := range trustSealedCheckNames {
		checks[n] = true
	}
	v := TrustLaboratoryResult{Input: in, Simulation: true, Containment: trustSealedResult{ABI: 6, Checks: checks}, Cases: []TrustLaboratoryCase{}}
	for _, id := range in.Scenarios {
		v.Cases = append(v.Cases, TrustLaboratoryCase{ID: id, Passed: true, Observed: map[string]any{"test_double": true}})
	}
	v.DeterministicDigest = laboratoryResultDigest(v)
	return v
}
func TestActionsConfianceT6LaboratoryStrictInputs(t *testing.T) {
	for _, raw := range []string{`{"enabled":null,"timeout_seconds":30,"max_concurrent_runs":1}`, `{"enabled":false,"timeout_seconds":0,"max_concurrent_runs":1}`, `{"enabled":false,"timeout_seconds":30,"max_concurrent_runs":0}`, `{"enabled":false,"timeout_seconds":1.5,"max_concurrent_runs":1}`, `{"enabled":false,"timeout_seconds":30}`} {
		t.Run(raw, func(t *testing.T) {
			var p TrustLaboratoryPolicy
			if json.Unmarshal([]byte(raw), &p) == nil {
				t.Fatal("invalid policy accepted")
			}
		})
	}
	s, _ := trustFixture(t)
	labTestEnable(t, s)
	st, _ := s.loadTrustState()
	for _, variant := range []string{"path", "production", "duplicate", "wrong-digest", "numeric-seed"} {
		t.Run(variant, func(t *testing.T) {
			wire, _ := json.Marshal(laboratoryTestInput())
			var input map[string]any
			json.Unmarshal(wire, &input)
			switch variant {
			case "path":
				input["fixture_root"] = s.root
			case "production":
				input["connector_id"] = "production"
			case "duplicate":
				input["scenario_ids"] = []string{"biba-denials", "biba-denials"}
			case "wrong-digest":
				input["corpus_digest"] = "unknown"
			case "numeric-seed":
				input["seed"] = 42
			}
			before := secretStateBytes(t, s)
			_, err := labTestCall(t, s, "run", map[string]any{"schema_version": 1, "event_id": "bad-" + variant, "expected_revision": st.Revision, "reason": "invalid laboratory input must refuse", "input": input}, func(context.Context, TrustLaboratoryInput) (TrustLaboratoryResult, error) {
				t.Fatal("invalid request launched process")
				return TrustLaboratoryResult{}, nil
			})
			if err == nil || !bytes.Equal(before, secretStateBytes(t, s)) {
				t.Fatal("invalid input accepted or mutated state", err)
			}
		})
	}
}
func TestActionsConfianceT6LaboratoryConcurrencyAndInvalidation(t *testing.T) {
	s, _ := trustFixture(t)
	labTestEnable(t, s)
	st, _ := s.loadTrustState()
	entered, release := make(chan struct{}), make(chan struct{})
	done := make(chan error, 1)
	body := map[string]any{"schema_version": 1, "event_id": "lab-blocked", "expected_revision": st.Revision, "reason": "concurrency and current policy fixture", "input": laboratoryTestInput()}
	raw, _ := json.Marshal(body)
	go func() {
		_, err := s.laboratoryOperationUsing(context.Background(), "run", raw, trustAdminToken, func(_ context.Context, in TrustLaboratoryInput) (TrustLaboratoryResult, error) {
			close(entered)
			<-release
			return laboratoryStub(in), nil
		})
		done <- err
	}()
	<-entered
	body["event_id"] = "lab-second"
	if _, err := labTestCall(t, s, "run", body, nil); err == nil {
		t.Fatal("concurrency budget bypassed")
	}
	if _, err := labTestCall(t, s, "policy-apply", map[string]any{"schema_version": 1, "event_id": "disable-during-run", "expected_revision": st.Revision, "reason": "disable policy during running scenario", "policy": map[string]any{"enabled": false, "timeout_seconds": 15, "max_concurrent_runs": 1}}, nil); err != nil {
		t.Fatal(err)
	}
	close(release)
	if err := <-done; err == nil {
		t.Fatal("stale positive result accepted")
	}
	current, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	for _, run := range current.LaboratoryRuns.Runs {
		if run.Status != "invalidated" || run.Result != nil {
			t.Fatal("failed policy recheck became success")
		}
	}
}

func TestActionsConfianceT6LaboratoryTimeoutAndContamination(t *testing.T) {
	marker := filepath.Join(t.TempDir(), "owned-child")
	ctx, cancel := context.WithCancel(context.Background())
	done := make(chan error, 1)
	go func() {
		_, err := runLaboratoryProcess(ctx, laboratoryTestInput(), []string{os.Args[0], "-test.run=^TestActionsConfianceT6LaboratoryHelper$", "--", marker, "_trust_laboratory_wait"})
		done <- err
	}()
	end := time.Now().Add(5 * time.Second)
	var raw []byte
	for time.Now().Before(end) {
		raw, _ = os.ReadFile(marker)
		if len(raw) > 0 {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if len(raw) == 0 {
		cancel()
		<-done
		t.Fatal("owned child never started")
	}
	pid, _ := strconv.Atoi(string(raw))
	start := laboratoryProcessStart(pid)
	cancel()
	if err := <-done; err == nil {
		t.Fatal("cancellation became successful simulation")
	}
	if current, err := laboratoryProcessIdentity(pid); err != nil || current == start {
		t.Fatal("cancelled child still active", err)
	}
	root := t.TempDir()
	os.Mkdir(filepath.Join(root, ".swarm"), 0700)
	before := []byte("owned original")
	os.WriteFile(filepath.Join(root, ".swarm", "sentinel"), before, 0600)
	if _, _, err := laboratoryFixture(root, "42"); err == nil {
		t.Fatal("existing fixture root contaminated")
	}
	after, _ := os.ReadFile(filepath.Join(root, ".swarm", "sentinel"))
	if !bytes.Equal(before, after) {
		t.Fatal("contamination changed original")
	}
}
func TestActionsConfianceT6LaboratoryOldCorpusReadable(t *testing.T) {
	input := laboratoryTestInput()
	input.Version = 9
	corpus, _ := json.Marshal(map[string]any{"version": 9, "historical": true})
	input.Digest = hashMust(json.RawMessage(corpus))
	now := time.Now().UTC()
	result := laboratoryStub(input)
	st := newTrustState()
	st.LaboratoryRuns = &trustLaboratoryState{Runs: map[string]TrustLaboratoryRun{"old": {ID: "old", Input: input, Corpus: corpus, Policy: laboratoryPolicy(nil), Revision: 1, Principal: "old-admin", Authority: "old-authority", Status: "passed", StartedAt: now, FinishedAt: &now, OwnerPID: os.Getpid(), OwnerStart: laboratoryProcessStart(os.Getpid()), Result: &result}}}
	if err := validateTrustLaboratoryState(&st); err != nil {
		t.Fatal("historical corpus prevents loading", err)
	}
	if laboratoryInputValid(&input) {
		t.Fatal("historical corpus silently adopted as current execution")
	}
}
func TestActionsConfianceT6LaboratoryPolicyReplay(t *testing.T) {
	s, _ := trustFixture(t)
	st, _ := s.loadTrustState()
	body := map[string]any{"schema_version": 1, "event_id": "lab-policy-replay", "expected_revision": st.Revision, "reason": "exact settings replay must be idempotent", "policy": map[string]any{"enabled": true, "timeout_seconds": 17, "max_concurrent_runs": 2}}
	first, err := labTestCall(t, s, "policy-apply", body, nil)
	if err != nil {
		t.Fatal(err)
	}
	before := secretStateBytes(t, s)
	again, err := labTestCall(t, s, "policy-apply", body, nil)
	firstRaw, _ := json.Marshal(first)
	againRaw, _ := json.Marshal(again)
	var firstJSON, againJSON any
	json.Unmarshal(firstRaw, &firstJSON)
	json.Unmarshal(againRaw, &againJSON)
	if err != nil || hashMust(firstJSON) != hashMust(againJSON) || !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("settings replay failed or changed history", err)
	}
}

func TestActionsConfianceT6LaboratoryParentDeath(t *testing.T) {
	marker := filepath.Join(t.TempDir(), "owned-lab-child")
	cmd := exec.Command(os.Args[0], "-test.run=^TestActionsConfianceT6LaboratoryHelper$", "--", marker, "_trust_laboratory_parent")
	var output bytes.Buffer
	cmd.Stdout = &output
	cmd.Stderr = &output
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	finished := false
	defer func() {
		if !finished {
			cmd.Process.Kill()
			cmd.Wait()
		}
	}()
	deadline := time.Now().Add(5 * time.Second)
	var raw []byte
	for time.Now().Before(deadline) {
		raw, _ = os.ReadFile(marker)
		if len(raw) > 0 {
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if len(raw) == 0 {
		t.Fatal("owned child did not start")
	}
	pid, _ := strconv.Atoi(string(raw))
	start := laboratoryProcessStart(pid)
	work, err := os.Readlink(filepath.Join("/proc", strconv.Itoa(pid), "cwd"))
	if err != nil {
		t.Fatal(err)
	}
	ownedRoot := filepath.Dir(work)
	if !strings.HasPrefix(filepath.Base(ownedRoot), "swarm-laboratory-") {
		t.Fatal("unexpected owned fixture path")
	}
	defer os.RemoveAll(ownedRoot)
	if err := cmd.Process.Kill(); err != nil {
		t.Fatal(err)
	}
	_ = cmd.Wait()
	finished = true
	deadline = time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		id, e := laboratoryProcessIdentity(pid)
		if e != nil {
			t.Fatal(e)
		}
		if id != start {
			return
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatal("fixed child survived creating parent death")
}
