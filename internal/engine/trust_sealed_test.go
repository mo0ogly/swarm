//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"
)

func sealedChange(t *testing.T, s *Store, policy any) TrustConfigChange {
	t.Helper()
	v, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	c := lifecycleChange(t, "alice", false, v.(map[string]any)["revision"].(int))
	raw, _ := json.Marshal(c)
	var fields map[string]json.RawMessage
	json.Unmarshal(raw, &fields)
	fields["sealed_execution"], _ = json.Marshal(policy)
	raw, _ = json.Marshal(fields)
	if err = strict(raw, &c); err != nil {
		t.Fatal("sealed execution contract missing", err)
	}
	return c
}
func TestActionsConfianceT4SealedPublicPolicy(t *testing.T) {
	s, _ := trustFixture(t)
	p, _, _ := s.trustPaths()
	before, _ := os.ReadFile(p)
	c := sealedChange(t, s, map[string]any{"enabled": false, "timeout_seconds": 15})
	v, err := s.trustConfig(c, true)
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := v.(map[string]any)["sealed_execution"]; !ok {
		t.Fatal("preview omits sealed policy")
	}
	after, _ := os.ReadFile(p)
	if !bytes.Equal(before, after) {
		t.Fatal("preview mutated")
	}
}

func sealedHelperArgv() []string {
	return []string{os.Args[0], "-test.run=^TestActionsConfianceT4SealedHelper$", "--", "_trust_sealed"}
}
func TestActionsConfianceT4SealedHelper(t *testing.T) {
	mode := os.Args[len(os.Args)-1]
	if mode == "_trust_sealed" {
		os.Exit(runTrustSealedChild(os.Stdin, os.Stdout))
	}
	if mode == "_trust_sealed_wait_child" {
		time.Sleep(time.Hour)
		os.Exit(0)
	}
	if mode == "_trust_sealed_wait" {
		child := exec.Command(os.Args[0], "-test.run=^TestActionsConfianceT4SealedHelper$", "--", "_trust_sealed_wait_child")
		child.Stdout = os.Stdout
		child.Stderr = os.Stderr
		if child.Start() != nil {
			os.Exit(9)
		}
		if os.WriteFile(os.Args[len(os.Args)-2], []byte(strconv.Itoa(child.Process.Pid)), 0600) != nil {
			os.Exit(9)
		}
		time.Sleep(time.Hour)
		os.Exit(0)
	}
}
func TestActionsConfianceT4SealedBoundary(t *testing.T) {
	result, err := runTrustSealedProbe(context.Background(), TrustSealedPolicy{Enabled: true, TimeoutSeconds: 15}, sealedHelperArgv())
	if err != nil {
		t.Fatal("sealed helper failed", err)
	}
	if result.ABI < 6 || len(result.Checks) != 14 {
		t.Fatal("incomplete boundary proof", result)
	}
	for _, name := range trustSealedCheckNames {
		if !result.Checks[name] {
			t.Fatal("boundary failed", name)
		}
	}
}
func TestActionsConfianceT4SealedValidationAndPersistence(t *testing.T) {
	s, _ := trustFixture(t)
	c := sealedChange(t, s, map[string]any{"enabled": true, "timeout_seconds": 15})
	p, _, _ := s.trustPaths()
	for _, raw := range []string{`{"enabled":null,"timeout_seconds":15}`, `{"enabled":true,"timeout_seconds":null}`, `{"enabled":true}`, `{"timeout_seconds":15}`, `{"enabled":1,"timeout_seconds":15}`, `{"enabled":true,"timeout_seconds":1.5}`, `{"enabled":true,"timeout_seconds":"15"}`} {
		var value TrustSealedPolicy
		if strict([]byte(raw), &value) == nil {
			t.Fatal("malformed policy accepted", raw)
		}
	}
	for _, seconds := range []int{0, -1} {
		before, _ := os.ReadFile(p)
		c.SealedExecution.TimeoutSeconds = seconds
		if _, err := s.trustConfig(c, false); err == nil {
			t.Fatal("invalid duration accepted", seconds)
		}
		after, _ := os.ReadFile(p)
		if !bytes.Equal(before, after) {
			t.Fatal("invalid configuration mutated")
		}
	}
	c.SealedExecution.TimeoutSeconds = 15
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	v, _ := s.trustView()
	c2 := lifecycleChange(t, "alice", false, v.(map[string]any)["revision"].(int))
	if _, err := s.trustConfig(c2, false); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	v, err = reopened.trustView()
	if err != nil {
		t.Fatal(err)
	}
	policy := v.(map[string]any)["sealed_execution"].(TrustSealedPolicy)
	if !bool(policy.Enabled) || policy.TimeoutSeconds != 15 {
		t.Fatal("policy did not persist")
	}
}
func TestActionsConfianceT4SealedDisabledAndInvalidRequest(t *testing.T) {
	s, _ := trustFixture(t)
	p, _, _ := s.trustPaths()
	before, _ := os.ReadFile(p)
	for _, raw := range []string{`{}`, `null`, `{"actor":"admin"}`, `{"work":"/tmp"}`} {
		_, err := s.trustSealedCheck(context.Background(), []byte(raw))
		if err == nil {
			t.Fatal("invalid or disabled check ran")
		}
		after, _ := os.ReadFile(p)
		if !bytes.Equal(before, after) {
			t.Fatal("check mutated business state")
		}
	}
}
func TestActionsConfianceT4SealedCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	_, err := runTrustSealedProbe(ctx, TrustSealedPolicy{Enabled: true, TimeoutSeconds: 15}, sealedHelperArgv())
	if commandFailure(err).Code != "containment_timeout" {
		t.Fatal("cancellation not attributed", err)
	}
}

func TestActionsConfianceT4SealedTimeoutClosesDescendants(t *testing.T) {
	marker := filepath.Join(t.TempDir(), "child.pid")
	argv := []string{os.Args[0], "-test.run=^TestActionsConfianceT4SealedHelper$", "--", marker, "_trust_sealed_wait"}
	start := time.Now()
	_, err := runTrustSealedProbe(context.Background(), TrustSealedPolicy{Enabled: true, TimeoutSeconds: 2}, argv)
	if commandFailure(err).Code != "containment_timeout" || time.Since(start) > 5*time.Second {
		t.Fatal("timeout not enforced", err)
	}
	raw, err := os.ReadFile(marker)
	if err != nil {
		t.Fatal("descendant was not observed", err)
	}
	pid, err := strconv.Atoi(string(raw))
	if err != nil {
		t.Fatal(err)
	}
	stat, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", pid))
	if os.IsNotExist(err) || errors.Is(err, syscall.ESRCH) {
		return
	}
	if err != nil {
		t.Fatal(err)
	}
	parts := strings.SplitN(string(stat), ") ", 2)
	if len(parts) != 2 || !strings.HasPrefix(parts[1], "Z ") {
		t.Fatal("descendant still running")
	}
}
func TestActionsConfianceT4SealedStaleConfiguration(t *testing.T) {
	s, _ := trustFixture(t)
	if _, err := s.trustConfig(sealedChange(t, s, map[string]any{"enabled": true, "timeout_seconds": 15}), false); err != nil {
		t.Fatal(err)
	}
	p, _, _ := s.trustPaths()
	var afterConfig []byte
	_, err := s.trustSealedCheckUsing(context.Background(), []byte(`{}`), func(ctx context.Context, policy TrustSealedPolicy) (trustSealedResult, error) {
		if _, err := s.trustConfig(sealedChange(t, s, map[string]any{"enabled": false, "timeout_seconds": 15}), false); err != nil {
			t.Fatal(err)
		}
		afterConfig, _ = os.ReadFile(p)
		return trustSealedResult{ABI: 7, Checks: map[string]bool{}}, nil
	})
	if commandFailure(err).Code != "revision_conflict" {
		t.Fatal("stale result returned", err)
	}
	after, _ := os.ReadFile(p)
	if !bytes.Equal(afterConfig, after) {
		t.Fatal("probe mutated after reconfiguration")
	}
}

func TestActionsConfianceT4SealedBusyStateRefusesPromptly(t *testing.T) {
	s, _ := trustFixture(t)
	_, lockPath, _ := s.trustPaths()
	lock, err := os.OpenFile(lockPath, os.O_RDWR, 0600)
	if err != nil {
		t.Fatal(err)
	}
	defer lock.Close()
	if syscall.Flock(int(lock.Fd()), syscall.LOCK_EX) != nil {
		t.Fatal("fixture lock unavailable")
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	start := time.Now()
	_, err = s.trustSealedCheck(context.Background(), []byte(`{}`))
	if commandFailure(err).Code != "workspace_wait" || time.Since(start) > time.Second {
		t.Fatal("read waited behind occupied state", err)
	}
}
