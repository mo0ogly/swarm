//go:build linux

package engine

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func kernelCheckChange(t *testing.T, s *Store, p any) TrustConfigChange {
	t.Helper()
	v, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	c := lifecycleChange(t, "alice", false, v.(map[string]any)["revision"].(int))
	raw, _ := json.Marshal(c)
	var fields map[string]json.RawMessage
	_ = json.Unmarshal(raw, &fields)
	fields["kernel_check"], _ = json.Marshal(p)
	raw, _ = json.Marshal(fields)
	if err = strict(raw, &c); err != nil {
		t.Fatal(err)
	}
	return c
}

func TestKernelCheckHelper(t *testing.T) {
	mode := os.Args[len(os.Args)-1]
	if strings.HasPrefix(mode, "_kernel_check_block:") {
		path := strings.TrimPrefix(mode, "_kernel_check_block:")
		start, _ := kernelObservationProcessStart(os.Getpid())
		wd, _ := os.Getwd()
		raw, _ := json.Marshal(map[string]any{"pid": os.Getpid(), "start": start, "parent": os.Getppid(), "workdir": wd})
		if os.WriteFile(path, raw, 0600) != nil {
			os.Exit(2)
		}
		time.Sleep(time.Hour)
		os.Exit(0)
	}
	if strings.HasPrefix(mode, "_kernel_check_parent:") {
		path := strings.TrimPrefix(mode, "_kernel_check_parent:")
		_, err := runTrustKernelCheckProcess(context.Background(), TrustKernelCheckPolicy{Enabled: true, TimeoutSeconds: 30, RingBytes: 65536}, []string{os.Args[0], "-test.run=^TestKernelCheckHelper$", "--", "_kernel_check_block:" + path})
		if err != nil {
			os.Exit(2)
		}
		os.Exit(0)
	}
	if mode == "_kernel_fixture_exit" {
		os.Exit(0)
	}
	if mode != "_kernel_check" {
		t.Skip("owned helper fixture only")
	}
	kernelCheckExitArgs = func() []string { return []string{"-test.run=^TestKernelCheckHelper$", "--", "_kernel_fixture_exit"} }
	os.Exit(runTrustKernelCheckChild(os.Stdin))
}

type kernelCheckProcessReceipt struct {
	PID     int    `json:"pid"`
	Start   string `json:"start"`
	Parent  int    `json:"parent"`
	Workdir string `json:"workdir"`
}

func kernelCheckWaitReceipt(t *testing.T, path string) kernelCheckProcessReceipt {
	t.Helper()
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		raw, err := os.ReadFile(path)
		if err == nil {
			var v kernelCheckProcessReceipt
			if json.Unmarshal(raw, &v) == nil && v.PID > 0 && v.Start != "" {
				return v
			}
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatal("owned helper did not start")
	return kernelCheckProcessReceipt{}
}
func kernelCheckAssertStopped(t *testing.T, v kernelCheckProcessReceipt) {
	t.Helper()
	deadline := time.Now().Add(3 * time.Second)
	for time.Now().Before(deadline) {
		raw, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", v.PID))
		if os.IsNotExist(err) {
			return
		}
		if err != nil {
			t.Fatal(err)
		}
		fields := strings.Fields(string(raw[strings.LastIndex(string(raw), ")")+1:]))
		if len(fields) > 0 && (fields[0] == "Z" || fields[0] == "X") {
			return
		}
		start, err := parseKernelObservationProcessStart(raw, v.PID)
		if err != nil {
			t.Fatal(err)
		}
		if start != v.Start {
			return
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatal("owned helper stayed alive")
}

func TestKernelCheckProcessCancellation(t *testing.T) {
	path := filepath.Join(t.TempDir(), "child.json")
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	done := make(chan error, 1)
	go func() {
		_, err := runTrustKernelCheckProcess(ctx, TrustKernelCheckPolicy{Enabled: true, TimeoutSeconds: 10, RingBytes: 65536}, []string{os.Args[0], "-test.run=^TestKernelCheckHelper$", "--", "_kernel_check_block:" + path})
		done <- err
	}()
	v := kernelCheckWaitReceipt(t, path)
	cancel()
	select {
	case err := <-done:
		if err == nil {
			t.Fatal("cancelled diagnostic succeeded")
		}
	case <-time.After(3 * time.Second):
		t.Fatal("cancelled launcher stayed blocked")
	}
	kernelCheckAssertStopped(t, v)
}

func TestKernelCheckParentDeathStopsHelper(t *testing.T) {
	path := filepath.Join(t.TempDir(), "child.json")
	cmd := exec.Command(os.Args[0], "-test.run=^TestKernelCheckHelper$", "--", "_kernel_check_parent:"+path)
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if cmd.ProcessState == nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})
	v := kernelCheckWaitReceipt(t, path)
	if v.Parent != cmd.Process.Pid {
		t.Fatal("unexpected helper parent")
	}
	if err := cmd.Process.Kill(); err != nil {
		t.Fatal(err)
	}
	_ = cmd.Wait()
	kernelCheckAssertStopped(t, v)
	if !strings.HasPrefix(filepath.Base(v.Workdir), "swarm-kernel-check-") {
		t.Fatal("unexpected fixture ownership path")
	}
	if err := os.RemoveAll(v.Workdir); err != nil {
		t.Fatal(err)
	}
}

func TestKernelCheckRealProcessProtocol(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	p := TrustKernelCheckPolicy{Enabled: true, TimeoutSeconds: 5, RingBytes: 65536}
	r, err := runTrustKernelCheckProcess(ctx, p, []string{os.Args[0], "-test.run=^TestKernelCheckHelper$", "--", "_kernel_check"})
	if err != nil {
		t.Fatal(err)
	}
	if r.Source != "ebpf" || r.Scope != "owned_fixture_only" || r.GlobalQualification || !r.CleanupVerified || len(r.OwnedPIDs) != 1 {
		t.Fatal("invalid owned process result")
	}
	if !r.Available && (r.FixtureVerified || len(r.Events) != 0) {
		t.Fatal("unavailable collector fabricated events")
	}
}

func TestKernelCheckPolicyInvalid(t *testing.T) {
	for _, raw := range []string{
		`{"enabled":null,"timeout_seconds":15,"ring_bytes":65536}`,
		`{"enabled":false,"timeout_seconds":null,"ring_bytes":65536}`,
		`{"enabled":false,"timeout_seconds":0,"ring_bytes":65536}`,
		`{"enabled":false,"timeout_seconds":1.5,"ring_bytes":65536}`,
		`{"enabled":false,"timeout_seconds":15,"ring_bytes":65537}`,
		`{"enabled":false,"timeout_seconds":15,"ring_bytes":0}`,
		`{"enabled":false,"timeout_seconds":15,"ring_bytes":4294967296}`,
		`{"enabled":false,"timeout_seconds":15}`,
	} {
		var p TrustKernelCheckPolicy
		err := json.Unmarshal([]byte(raw), &p)
		if err == nil {
			err = validateTrustKernelPolicy(&p)
		}
		if err == nil {
			t.Fatalf("invalid inactive policy admitted: %s", raw)
		}
	}
}

func TestKernelCheckFreshAuthorityAndRevision(t *testing.T) {
	s, _ := trustFixture(t)
	c := kernelCheckChange(t, s, map[string]any{"enabled": true, "timeout_seconds": 15, "ring_bytes": 65536})
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	called := false
	runner := func(context.Context, TrustKernelCheckPolicy) (TrustKernelCheckResult, error) {
		called = true
		return kernelCheckFixedResult(), nil
	}
	if _, err := s.trustKernelCheckUsing(context.Background(), []byte("{}"), "invalid-token", runner); err == nil || called {
		t.Fatal("unauthorized helper launched")
	}
	if _, err := s.trustKernelCheckUsing(context.Background(), []byte(`{"pid":1}`), trustAdminToken, runner); err == nil || called {
		t.Fatal("arbitrary target admitted")
	}
	_, err := s.trustKernelCheckUsing(context.Background(), []byte("{}"), trustAdminToken, func(context.Context, TrustKernelCheckPolicy) (TrustKernelCheckResult, error) {
		if _, e := s.withTrustReadAvailable(func(*trustState) (any, error) { return nil, nil }); e != nil {
			t.Fatal("runner held workspace lock", e)
		}
		c2 := kernelCheckChange(t, s, map[string]any{"enabled": true, "timeout_seconds": 17, "ring_bytes": 65536})
		_, e := s.trustConfig(c2, false)
		if e != nil {
			t.Fatal(e)
		}
		return kernelCheckFixedResult(), nil
	})
	if err == nil || !strings.Contains(err.Error(), "modifiée") {
		t.Fatal("stale result accepted", err)
	}
}

func TestKernelCheckCLIRejectsEmptyVerdict(t *testing.T) {
	r := kernelCheckFixedResult()
	r.EffectivePolicy = TrustKernelCheckPolicy{Enabled: true, TimeoutSeconds: 15, RingBytes: 65536}
	r.Available = true
	r.FixtureVerified = true
	r.CleanupVerified = true
	r.OwnedPIDs = []int{os.Getpid()}
	var out bytes.Buffer
	if err := printTrustKernelCheck(&out, r); err == nil {
		t.Fatal("positive empty verdict accepted as CLI success")
	}
}

func TestKernelCheckProtocolInvalid(t *testing.T) {
	for _, raw := range []string{`{"x":1,"x":2}` + "\n", `{"events":[{"pid":1,"pid":2}]}` + "\n", string([]byte{0xff, '\n'}), strings.Repeat("x", kernelObservationFrameBytes+1) + "\n", `{} {}` + "\n"} {
		var v map[string]any
		if err := kernelCheckRead(bufio.NewReaderSize(strings.NewReader(raw), kernelObservationFrameBytes+1), &v); err == nil {
			t.Fatal("malformed private protocol accepted")
		}
	}
}

func TestKernelCheckUnavailableCLI(t *testing.T) {
	r := kernelCheckFixedResult()
	r.EffectivePolicy = TrustKernelCheckPolicy{Enabled: true, TimeoutSeconds: 15, RingBytes: 65536}
	r.CleanupVerified = true
	r.OwnedPIDs = []int{os.Getpid()}
	r.Reason = "fixture_unavailable"
	var out bytes.Buffer
	err := printTrustKernelCheck(&out, r)
	if err == nil || commandFailure(err).Code != "kernel_unavailable" {
		t.Fatal("unavailable CLI outcome lost", err)
	}
	var got TrustKernelCheckResult
	if json.Unmarshal(out.Bytes(), &got) != nil || got.Available || got.FixtureVerified || len(got.Events) != 0 {
		t.Fatal("unavailable output fabricated coverage")
	}
}

func TestKernelCheckPolicy(t *testing.T) {
	s, _ := trustFixture(t)
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	c := kernelCheckChange(t, s, map[string]any{"enabled": false, "timeout_seconds": 15, "ring_bytes": 65536})
	v, err := s.trustConfig(c, true)
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := v.(map[string]any)["kernel_check"]; !ok {
		t.Fatal("policy absent from preview")
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("preview mutated state")
	}
	if _, err = s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	v, err = s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	if v.(map[string]any)["kernel_check"].(TrustKernelCheckPolicy).RingBytes != 65536 {
		t.Fatal("policy lost")
	}
}

func TestKernelCheckInactiveRefusesBeforeRunner(t *testing.T) {
	s, _ := trustFixture(t)
	called := false
	_, err := s.trustKernelCheckUsing(context.Background(), []byte("{}"), trustAdminToken, func(context.Context, TrustKernelCheckPolicy) (TrustKernelCheckResult, error) {
		called = true
		return TrustKernelCheckResult{}, nil
	})
	if err == nil || called {
		t.Fatal("inactive policy launched helper")
	}
}
