//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

func kernelObservationFixture(t *testing.T) (kernelObservationLease, kernelObservation) {
	t.Helper()
	stamp, err := kernelObservationProcessStart(os.Getpid())
	if err != nil {
		t.Fatal(err)
	}
	a := Agent{ID: "agent", WorkID: "mission", TaskID: "task", Attempt: "attempt", Status: "running", Child: os.Getpid(), ChildStamp: stamp, Host: hostIdentity()}
	l, err := newKernelObservationLease(a, strings.Repeat("a", 64))
	if err != nil {
		t.Fatal(err)
	}
	e := kernelObservation{CollectorBootID: strings.Repeat("b", 32), Sequence: 1, KtimeNS: 1, EventType: "process", PID: l.PID, PIDStartTime: l.Start, ParentPID: os.Getppid(), CgroupID: 1, MissionID: l.Mission, TaskID: l.Task, AttemptID: l.Attempt, ResourceDigest: l.ResourceDigest, ProbeSet: "swarm-minimal-v1"}
	return l, e
}

func TestKernelObservationStrictFrame(t *testing.T) {
	_, e := kernelObservationFixture(t)
	b, err := json.Marshal(e)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = decodeKernelObservation(b); err != nil {
		t.Fatal(err)
	}
	for name, raw := range map[string]string{
		"duplicate":    strings.Replace(string(b), `"sequence":`, `"pid":1,"sequence":`, 1),
		"unknown":      strings.TrimSuffix(string(b), "}") + `,"path":"secret"}`,
		"missing":      strings.Replace(string(b), `"lost_before":"0",`, "", 1),
		"null":         strings.Replace(string(b), `"pid":`, `"pid":null,"extra":`, 1),
		"trailing":     string(b) + ` {}`,
		"oversize":     strings.Repeat(" ", kernelObservationFrameBytes+1),
		"invalid_utf8": string(append([]byte{0xff}, b...)),
		"boolean":      strings.Replace(string(b), `"sequence":"1"`, `"sequence":true`, 1),
		"noncanonical": strings.Replace(string(b), `"sequence":"1"`, `"sequence":"01"`, 1),
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := decodeKernelObservation([]byte(raw)); err == nil {
				t.Fatal("malformed frame accepted")
			}
		})
	}
}

func TestKernelObservationAttributionAndLoss(t *testing.T) {
	l, e := kernelObservationFixture(t)
	s := newKernelObservationStream(l, e.CollectorBootID, e.ProbeSet)
	if err := s.accept(e, l); err != nil {
		t.Fatal(err)
	}
	if err := s.accept(e, l); err == nil {
		t.Fatal("replay accepted")
	}
	e.Sequence = 4
	e.LostBefore = 2
	e.KtimeNS = 2
	if err := s.accept(e, l); err != nil {
		t.Fatal(err)
	}
	if s.snapshot().Lost != 2 || s.snapshot().Complete {
		t.Fatal("loss/complete misreported")
	}
	e.Sequence = 5
	e.LostBefore = 0
	e.MissionID = "foreign"
	if err := s.accept(e, l); err == nil {
		t.Fatal("foreign attribution accepted")
	}
	s.disconnect()
	if err := s.accept(e, l); err == nil {
		t.Fatal("disconnected stream accepted")
	}
}

func TestKernelObservationPeerHelper(t *testing.T) {
	if os.Getenv("SWARM_TEST_OBSERVATION_PEER") != "1" {
		t.Skip("owned subprocess fixture only")
	}
	c, err := net.DialUnix("unix", nil, &net.UnixAddr{Name: os.Getenv("SWARM_TEST_OBSERVATION_SOCKET"), Net: "unix"})
	if err != nil {
		t.Fatal(err)
	}
	defer c.Close()
	if _, err = io.WriteString(c, os.Getenv("SWARM_TEST_OBSERVATION_FRAME")+"\n"); err != nil {
		t.Fatal(err)
	}
	_, _ = io.Copy(io.Discard, os.Stdin)
}

func kernelObservationSocketFixture(t *testing.T, frame string) (*net.UnixConn, kernelObservationPeer) {
	t.Helper()
	path := filepath.Join(t.TempDir(), "collector.sock")
	l, err := net.ListenUnix("unix", &net.UnixAddr{Name: path, Net: "unix"})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = l.Close() })
	_ = l.SetDeadline(time.Now().Add(10 * time.Second))
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	cmd := exec.CommandContext(ctx, os.Args[0], "-test.run=^TestKernelObservationPeerHelper$", "-test.v")
	cmd.Env = append(os.Environ(), "SWARM_TEST_OBSERVATION_PEER=1", "SWARM_TEST_OBSERVATION_SOCKET="+path, "SWARM_TEST_OBSERVATION_FRAME="+frame)
	stdin, err := cmd.StdinPipe()
	if err != nil {
		cancel()
		t.Fatal(err)
	}
	var out bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &out
	if err = cmd.Start(); err != nil {
		cancel()
		t.Fatal(err)
	}
	t.Cleanup(func() {
		_ = stdin.Close()
		err := cmd.Wait()
		cancel()
		if err != nil {
			t.Errorf("owned peer: %v: %s", err, out.String())
		}
	})
	start, err := kernelObservationProcessStart(cmd.Process.Pid)
	if err != nil {
		t.Fatal(err)
	}
	c, err := l.AcceptUnix()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = c.Close() })
	return c, kernelObservationPeer{PID: cmd.Process.Pid, UID: uint32(os.Getuid()), Start: start}
}

func TestKernelObservationUnixPeer(t *testing.T) {
	l, e := kernelObservationFixture(t)
	b, _ := json.Marshal(e)
	c, p := kernelObservationSocketFixture(t, string(b))
	for name, bad := range map[string]kernelObservationPeer{
		"same_uid_other_pid": {PID: os.Getpid(), UID: p.UID, Start: processStamp(os.Getpid())},
		"wrong_uid":          {PID: p.PID, UID: p.UID + 1, Start: p.Start},
		"reused_pid":         {PID: p.PID, UID: p.UID, Start: "1"},
	} {
		t.Run(name, func(t *testing.T) {
			if err := authenticateKernelObservationPeer(c, bad); err == nil {
				t.Fatal("foreign peer accepted")
			}
		})
	}
	s := newKernelObservationStream(l, e.CollectorBootID, e.ProbeSet)
	r, err := newKernelObservationReceiver(c, p, s, func() (kernelObservationLease, error) { return l, nil })
	if err != nil {
		t.Fatal(err)
	}
	got, err := r.read()
	if err != nil || got != e {
		t.Fatalf("owned frame: %v", err)
	}
	if s.snapshot().Accepted != 1 {
		t.Fatal("owned frame missing")
	}
	_ = c.Close()
	if _, err = r.read(); err == nil || !s.snapshot().Disconnected {
		t.Fatal("disconnect not reported")
	}
}

func TestKernelObservationRefusesStaleSource(t *testing.T) {
	l, e := kernelObservationFixture(t)
	for _, field := range []string{"mission", "task", "attempt", "digest", "pid", "start", "boot", "probe", "clock", "gap", "overflow"} {
		t.Run(field, func(t *testing.T) {
			s := newKernelObservationStream(l, e.CollectorBootID, e.ProbeSet)
			bad := e
			switch field {
			case "mission":
				bad.MissionID = "other"
			case "task":
				bad.TaskID = "other"
			case "attempt":
				bad.AttemptID = "other"
			case "digest":
				bad.ResourceDigest = strings.Repeat("c", 64)
			case "pid":
				bad.PID++
			case "start":
				bad.PIDStartTime = "1"
			case "boot":
				bad.CollectorBootID = strings.Repeat("c", 32)
			case "probe":
				bad.ProbeSet = "all-processes"
			case "clock":
				bad.KtimeNS = 0
			case "gap":
				bad.Sequence = 2
			case "overflow":
				bad.LostBefore = ^uint64(0)
			}
			if err := s.accept(bad, l); err == nil || s.snapshot().Accepted != 0 {
				t.Fatal("invalid event accepted or mutated stream")
			}
		})
	}
	for _, kind := range []string{"changed", "read_error"} {
		t.Run(kind, func(t *testing.T) {
			b, _ := json.Marshal(e)
			c, p := kernelObservationSocketFixture(t, string(b))
			s := newKernelObservationStream(l, e.CollectorBootID, e.ProbeSet)
			r, err := newKernelObservationReceiver(c, p, s, func() (kernelObservationLease, error) {
				if kind == "read_error" {
					return kernelObservationLease{}, os.ErrPermission
				}
				fresh := l
				fresh.Attempt = "new"
				return fresh, nil
			})
			if err != nil {
				t.Fatal(err)
			}
			if got, err := r.read(); err == nil || got.PID != 0 || s.snapshot().Accepted != 0 {
				t.Fatal("stale source disclosed an event")
			}
		})
	}
}

func TestKernelObservationConcurrentReplay(t *testing.T) {
	l, e := kernelObservationFixture(t)
	s := newKernelObservationStream(l, e.CollectorBootID, e.ProbeSet)
	var wg sync.WaitGroup
	for i := 0; i < 8; i++ {
		wg.Add(1)
		go func() { defer wg.Done(); _ = s.accept(e, l) }()
	}
	wg.Wait()
	if s.snapshot().Accepted != 1 {
		t.Fatal("concurrent replay duplicated event")
	}
}

func TestKernelObservationProcessIdentity(t *testing.T) {
	fields := make([]string, 20)
	for i := range fields {
		fields[i] = "1"
	}
	fields[0] = "S"
	fields[19] = "123456"
	raw := []byte("42 (odd ) name) " + strings.Join(fields, " "))
	if got, err := parseKernelObservationProcessStart(raw, 42); err != nil || got != "123456" {
		t.Fatalf("comm parsing: %v", err)
	}
	for _, raw := range []string{"", "42 (bad) Z", "43 (wrong) " + strings.Join(fields, " ")} {
		if _, err := parseKernelObservationProcessStart([]byte(raw), 42); err == nil {
			t.Fatal("invalid identity accepted")
		}
	}
	if _, err := kernelObservationProcessStart(0); err == nil {
		t.Fatal("invalid pid accepted")
	}
	t.Log("owned PID identity checked; no kernel collection")
}

func TestKernelObservationAgentLifecycle(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	a, created, err := s.prepare(w.ID, launch)
	if err != nil || !created {
		t.Fatalf("prepare isolated agent: %v", err)
	}
	if a.Desired != "" {
		t.Fatal("unexpected producer desired state")
	}
	a.Status = "running"
	a.Child = os.Getpid()
	a.ChildStamp = processStamp(a.Child)
	a.Host = hostIdentity()
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	a, err = s.agent(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.kernelObservationSource(a.ID, strings.Repeat("a", 64)); err != nil {
		t.Fatalf("active producer rejected: %v", err)
	}
	for _, state := range []string{"stop", "invalid", "running"} {
		t.Run(state, func(t *testing.T) {
			bad := a
			bad.Desired = state
			if _, err := newKernelObservationLease(bad, strings.Repeat("a", 64)); err == nil {
				t.Fatal("noncanonical desired state accepted")
			}
		})
	}
	if _, err = s.db.Exec("UPDATE agents SET desired='stop' WHERE id=?", a.ID); err != nil {
		t.Fatal(err)
	}
	if _, err = s.kernelObservationSource(a.ID, strings.Repeat("a", 64)); err == nil {
		t.Fatal("persisted stop not observed")
	}
	if _, err = s.kernelObservationSource("missing", strings.Repeat("a", 64)); err == nil {
		t.Fatal("missing agent admitted")
	}
}

func TestKernelObservationUnixLossAndCancellation(t *testing.T) {
	l, e := kernelObservationFixture(t)
	first, _ := json.Marshal(e)
	e.Sequence = 4
	e.LostBefore = 2
	e.KtimeNS = 2
	second, _ := json.Marshal(e)
	c, p := kernelObservationSocketFixture(t, string(first)+"\n"+string(second))
	s := newKernelObservationStream(l, e.CollectorBootID, e.ProbeSet)
	r, err := newKernelObservationReceiver(c, p, s, func() (kernelObservationLease, error) { return l, nil })
	if err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 2; i++ {
		if _, err = r.read(); err != nil {
			t.Fatal(err)
		}
	}
	if got := s.snapshot(); got.Accepted != 2 || got.Lost != 2 || got.Complete {
		t.Fatal("loss hidden or buffered frame lost")
	}
	done := make(chan error, 1)
	go func() { _, err := r.read(); done <- err }()
	_ = c.Close()
	select {
	case err := <-done:
		if err == nil {
			t.Fatal("cancelled read succeeded")
		}
	case <-time.After(time.Second):
		t.Fatal("cancelled read stayed blocked")
	}
	if !s.snapshot().Disconnected {
		t.Fatal("cancellation hidden")
	}
}

func TestKernelObservationUnixMalformedFrame(t *testing.T) {
	for _, frame := range []string{`{}`, strings.Repeat("x", kernelObservationFrameBytes+1)} {
		l, e := kernelObservationFixture(t)
		c, p := kernelObservationSocketFixture(t, frame)
		s := newKernelObservationStream(l, e.CollectorBootID, e.ProbeSet)
		r, err := newKernelObservationReceiver(c, p, s, func() (kernelObservationLease, error) { return l, nil })
		if err != nil {
			t.Fatal(err)
		}
		if got, err := r.read(); err == nil || got.PID != 0 || !s.snapshot().Disconnected || s.snapshot().Accepted != 0 {
			t.Fatal("invalid frame accepted or connection left active")
		}
	}
}
