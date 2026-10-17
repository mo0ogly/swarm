//go:build linux

package engine

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
)

func TestKernelObserveHelper(t *testing.T) {
	mode := os.Args[len(os.Args)-1]
	if strings.HasPrefix(mode, "_kernel_observe_parent:") {
		runtime.LockOSThread()
		path := strings.TrimPrefix(mode, "_kernel_observe_parent:")
		l := kernelObservationLease{AgentID: "fixture", Mission: "fixture", Task: "fixture", Attempt: "fixture", PID: os.Getpid(), Start: processStamp(os.Getpid()), Host: hostIdentity(), ResourceDigest: strings.Repeat("a", 64)}
		p := kernelObservePolicy(nil)
		p.Enabled = true
		process, err := startKernelObservedHelper(context.Background(), p, l, []string{"-test.run=^TestKernelObserveHelper$", "--", "_kernel_observe_wait:" + path})
		if err != nil {
			os.Exit(2)
		}
		defer process.close()
		time.Sleep(time.Hour)
		os.Exit(0)
	}
	if !strings.HasPrefix(mode, "_kernel_observe_") {
		t.Skip("owned protocol fixture; no BPF")
	}
	runtime.LockOSThread()
	raw, err := io.ReadAll(os.Stdin)
	if err != nil {
		os.Exit(2)
	}
	var b kernelObserveBootstrap
	if strict(raw, &b) != nil || kernelFixedHelperSecurity(b.Parent, b.ParentStart) != nil {
		os.Exit(2)
	}
	c, err := net.DialUnix("unix", nil, &net.UnixAddr{Name: b.Socket, Net: "unix"})
	if err != nil {
		os.Exit(2)
	}
	defer c.Close()
	if kernelObserveSend(c, kernelObservePacket{Type: "ready", Protocol: b.Protocol, Nonce: b.Nonce}) != nil {
		os.Exit(2)
	}
	reader := bufio.NewReaderSize(c, kernelObservationFrameBytes+1)
	if strings.HasPrefix(mode, "_kernel_observe_wait:") {
		wd, _ := os.Getwd()
		stamp, _ := kernelObservationProcessStart(os.Getpid())
		body, _ := json.Marshal(map[string]any{"pid": os.Getpid(), "start": stamp, "parent": os.Getppid(), "workdir": wd})
		if os.WriteFile(strings.TrimPrefix(mode, "_kernel_observe_wait:"), body, 0600) != nil {
			os.Exit(2)
		}
		_, _ = io.Copy(io.Discard, c)
		os.Exit(0)
	}
	for i := 0; i < 2; i++ {
		if _, err = kernelObserveAuthorize(c, reader, b.Source); err != nil {
			os.Exit(2)
		}
	}
	result := kernelObserveFixed(b.Policy, b.Source)
	result.CleanupVerified = true
	result.StopReason = "unavailable"
	result.Diagnostic = "Go protocol fixture, no BPF"
	if mode == "_kernel_observe_event" || mode == "_kernel_observe_bad" {
		event := kernelObservation{CollectorBootID: b.Nonce, Sequence: 1, KtimeNS: 1, EventType: "file", PID: b.Source.PID, PIDStartTime: b.Source.Start, MissionID: b.Source.Mission, TaskID: b.Source.Task, AttemptID: b.Source.Attempt, ResourceDigest: b.Source.ResourceDigest, ProbeSet: "swarm-minimal-v1"}
		if mode == "_kernel_observe_bad" {
			event.ResourceDigest = strings.Repeat("d", 64)
		}
		body, _ := json.Marshal(event)
		if kernelObserveSend(c, kernelObservePacket{Type: "event", Event: body}) != nil {
			os.Exit(2)
		}
		result.Available = true
		result.Attempts = 1
		result.Emitted = 1
		result.StopReason = "window"
	}
	body, _ := json.Marshal(result)
	if kernelObserveSend(c, kernelObservePacket{Type: "result", Result: body}) != nil {
		os.Exit(2)
	}
	var done struct {
		Done bool `json:"done"`
	}
	if kernelCheckRead(reader, &done) != nil || !done.Done {
		os.Exit(2)
	}
	os.Exit(0)
}

func TestKernelObserveOwnedProtocol(t *testing.T) {
	for _, mode := range []string{"unavailable", "event", "bad"} {
		t.Run(mode, func(t *testing.T) {
			s, a := kernelAttemptFixture(t)
			c := kernelObserveChange(t, s, true)
			if _, err := s.trustConfig(c, false); err != nil {
				t.Fatal(err)
			}
			before := kernelObserveStateBytes(t, s)
			raw, _ := json.Marshal(kernelObserveCommand{AgentID: a.ID})
			v, err := s.trustKernelObserveUsing(context.Background(), raw, trustAdminToken, func(ctx context.Context, p TrustKernelObservePolicy, l kernelObservationLease, fresh func() error) (TrustKernelObserveResult, error) {
				return s.runKernelObserveProcess(ctx, p, l, fresh, []string{"-test.run=^TestKernelObserveHelper$", "--", "_kernel_observe_" + mode})
			})
			if mode == "bad" {
				if err == nil {
					t.Fatal("wrong digest accepted")
				}
			} else {
				if err != nil {
					t.Fatal(err)
				}
				result := v.(TrustKernelObserveResult)
				if !result.CleanupVerified || result.Complete || result.GlobalQualification {
					t.Fatal("invalid observation qualification")
				}
				if mode == "event" && (!result.Available || len(result.Events) != 1) {
					t.Fatal("fixture event lost")
				}
				if mode == "unavailable" && (result.Available || len(result.Events) != 0) {
					t.Fatal("unavailable fixture invented events")
				}
			}
			if !bytes.Equal(before, kernelObserveStateBytes(t, s)) {
				t.Fatal("observation changed business state")
			}
		})
	}
}
func TestKernelObserveCancellation(t *testing.T) {
	s, a := kernelAttemptFixture(t)
	source, err := s.kernelAttemptObservationSource(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(t.TempDir(), "child.json")
	p := kernelObservePolicy(nil)
	p.Enabled = true
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	done := make(chan error, 1)
	go func() {
		_, err := s.runKernelObserveProcess(ctx, p, source, func() error { return nil }, []string{"-test.run=^TestKernelObserveHelper$", "--", "_kernel_observe_wait:" + path})
		done <- err
	}()
	receipt := kernelCheckWaitReceipt(t, path)
	cancel()
	select {
	case err = <-done:
		if err == nil {
			t.Fatal("cancelled observer succeeded")
		}
	case <-time.After(3 * time.Second):
		t.Fatal("observer cancellation blocked")
	}
	kernelCheckAssertStopped(t, receipt)
}
func TestKernelObserveParentDeath(t *testing.T) {
	path := filepath.Join(t.TempDir(), "child.json")
	cmd := exec.Command(os.Args[0], "-test.run=^TestKernelObserveHelper$", "--", "_kernel_observe_parent:"+path)
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if cmd.ProcessState == nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})
	receipt := kernelCheckWaitReceipt(t, path)
	if receipt.Parent != cmd.Process.Pid {
		t.Fatal("wrong owned parent")
	}
	if err := cmd.Process.Kill(); err != nil {
		t.Fatal(err)
	}
	_ = cmd.Wait()
	kernelCheckAssertStopped(t, receipt)
	if !strings.HasPrefix(filepath.Base(receipt.Workdir), "swarm-kernel-observer-") {
		t.Fatal("unknown fixture directory")
	}
	if err := os.RemoveAll(receipt.Workdir); err != nil {
		t.Fatal(err)
	}
}
