//go:build linux

package engine

import (
	"encoding/json"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func kernelAttemptFixture(t *testing.T) (*Store, Agent) {
	t.Helper()
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, created, err := s.prepare(w.ID, r)
	if err != nil || !created {
		t.Fatalf("prepare actual isolated producer: %v", err)
	}
	a.Status = "running"
	a.Child = os.Getpid()
	a.ChildStamp = processStamp(a.Child)
	a.Host = hostIdentity()
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	return s, a
}
func TestKernelAttemptSourceRegisteredInput(t *testing.T) {
	s, a := kernelAttemptFixture(t)
	before, err := s.agent(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	l, err := s.kernelAttemptObservationSource(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	if l.AgentID != a.ID || l.Attempt != a.Attempt || l.PID != a.Child || !kernelObservationHex(l.ResourceDigest, 32) {
		t.Fatal("bad protected source")
	}
	after, err := s.agent(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	b, _ := json.Marshal(before)
	c, _ := json.Marshal(after)
	if string(b) != string(c) {
		t.Fatal("source read mutated registered agent")
	}
	a.Heartbeat = now()
	a.Activity = "new progress"
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	stable, err := s.kernelAttemptObservationSource(a.ID)
	if err != nil || stable != l {
		t.Fatalf("progress changed content source: %v", err)
	}
	a.Prompt += "\nchanged registered input"
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	changed, err := s.kernelAttemptObservationSource(a.ID)
	if err != nil || changed.ResourceDigest == l.ResourceDigest {
		t.Fatal("changed input retained digest")
	}
}

func TestKernelAttemptSourceInvalid(t *testing.T) {
	for _, field := range []string{"mode", "role", "command", "directory", "provider", "prompt", "context", "workflow", "status", "desired", "host", "pid", "start", "agent", "work", "task", "attempt", "paused", "missing"} {
		t.Run(field, func(t *testing.T) {
			s, a := kernelAttemptFixture(t)
			switch field {
			case "mode":
				a.Mode = "dialogue"
			case "role":
				a.Role = ""
			case "command":
				a.Command = ""
			case "directory":
				a.CWD = ""
			case "provider":
				a.Provider = ""
			case "prompt":
				a.Prompt = " "
			case "context":
				a.Context = &ContextManifest{SHA256: "bad"}
			case "workflow":
				a.Workflow = &AgentWorkflow{SHA256: "bad"}
			case "status":
				a.Status = "completed"
			case "host":
				a.Host = "foreign"
			case "pid":
				a.Child = 0
			case "start":
				a.ChildStamp = "1"
			case "agent":
				a.ID = "body-other"
			case "work":
				a.WorkID = "body-other"
			case "task":
				a.TaskID = "body-other"
			case "attempt":
				a.Attempt = "old-attempt"
			}
			id := a.ID
			if field == "agent" {
				id = "body-other"
				a.ID = id
			}
			if field == "agent" {
				b, _ := json.Marshal(a)
				if _, err := s.db.Exec("UPDATE agents SET body=? WHERE work_id=?", b, a.WorkID); err != nil {
					t.Fatal(err)
				}
				var actual string
				if err := s.db.QueryRow("SELECT id FROM agents WHERE work_id=?", a.WorkID).Scan(&actual); err != nil {
					t.Fatal(err)
				}
				id = actual
			} else if err := s.saveAgent(a); err != nil {
				t.Fatal(err)
			}
			if field == "desired" {
				if err := s.stopAgent(id); err != nil {
					t.Fatal(err)
				}
			}
			if field == "paused" {
				if err := s.pause(a.WorkID, true); err != nil {
					t.Fatal(err)
				}
			}
			if field == "missing" {
				id = "missing"
			}
			if _, err := s.kernelAttemptObservationSource(id); err == nil {
				t.Fatal("invalid registry source accepted")
			}
		})
	}
}

func TestKernelAttemptSourceActualSupervisor(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Instruction = "TEST_SLEEP"
	a, created, err := s.prepare(w.ID, r)
	if err != nil || !created {
		t.Fatalf("prepare: %v", err)
	}
	done := make(chan error, 1)
	go func() { done <- s.supervise(a.ID) }()
	finished := false
	defer func() {
		if !finished {
			_ = s.stopAgent(a.ID)
			select {
			case <-done:
			case <-time.After(8 * time.Second):
				t.Error("owned supervisor cleanup timed out")
			}
		}
	}()
	running := waitAgent(t, s, a.ID, func(a Agent) bool { return a.Status == "running" && a.Child > 0 })
	source, err := s.kernelAttemptObservationSource(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	if source.PID != running.Child || source.PID == os.Getpid() || source.Start != running.ChildStamp {
		t.Fatal("source does not identify actual owned child")
	}
	if err = s.stopAgent(a.ID); err != nil {
		t.Fatal(err)
	}
	if _, err = s.kernelAttemptObservationSource(a.ID); err == nil {
		t.Fatal("accepted source after public stop")
	}
	select {
	case err = <-done:
		finished = true
		if err != nil {
			t.Fatal(err)
		}
	case <-time.After(8 * time.Second):
		t.Fatal("owned supervisor did not stop")
	}
}

func kernelAttemptSocket(t *testing.T) (*net.UnixConn, *net.UnixConn, kernelObservationPeer) {
	t.Helper()
	name := filepath.Join(t.TempDir(), "owned.sock")
	l, err := net.ListenUnix("unix", &net.UnixAddr{Name: name, Net: "unix"})
	if err != nil {
		t.Fatal(err)
	}
	defer l.Close()
	client, err := net.DialUnix("unix", nil, &net.UnixAddr{Name: name, Net: "unix"})
	if err != nil {
		t.Fatal(err)
	}
	server, err := l.AcceptUnix()
	if err != nil {
		client.Close()
		t.Fatal(err)
	}
	t.Cleanup(func() { client.Close(); server.Close() })
	return server, client, kernelObservationPeer{PID: os.Getpid(), UID: uint32(os.Getuid()), Start: processStamp(os.Getpid())}
}

func TestKernelAttemptReceiverFreshSource(t *testing.T) {
	for _, change := range []string{"none", "prompt", "stop", "pause", "superseded", "wrong-digest"} {
		t.Run(change, func(t *testing.T) {
			s, a := kernelAttemptFixture(t)
			source, err := s.kernelAttemptObservationSource(a.ID)
			if err != nil {
				t.Fatal(err)
			}
			server, client, peer := kernelAttemptSocket(t)
			boot := strings.Repeat("c", 32)
			receiver, err := s.newKernelAttemptReceiver(server, peer, a.ID, boot)
			if err != nil {
				t.Fatal(err)
			}
			result := make(chan error, 1)
			go func() { _, err := receiver.read(); result <- err }()
			switch change {
			case "prompt":
				a.Prompt += " changed"
				err = s.saveAgent(a)
			case "stop":
				err = s.stopAgent(a.ID)
			case "pause":
				err = s.pause(a.WorkID, true)
			case "superseded":
				w, e := s.get(a.WorkID)
				if e != nil {
					t.Fatal(e)
				}
				task, _ := w.task(a.TaskID)
				task.Attempts = append(task.Attempts, Attempt{ID: "new-attempt", Status: "recorded"})
				body, _ := json.Marshal(w)
				_, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID)
			}
			if err != nil {
				t.Fatal(err)
			}
			event := kernelObservation{CollectorBootID: boot, Sequence: 1, KtimeNS: 1, EventType: "file", PID: source.PID, PIDStartTime: source.Start, ParentPID: os.Getppid(), MissionID: source.Mission, TaskID: source.Task, AttemptID: source.Attempt, ResourceDigest: source.ResourceDigest, ProbeSet: "swarm-minimal-v1"}
			if change == "wrong-digest" {
				event.ResourceDigest = strings.Repeat("d", 64)
			}
			if err = json.NewEncoder(client).Encode(event); err != nil {
				t.Fatal(err)
			}
			select {
			case err = <-result:
			case <-time.After(3 * time.Second):
				t.Fatal("owned receiver did not finish")
			}
			status := receiver.stream.snapshot()
			if change == "none" {
				if err != nil || status.Accepted != 1 || status.Complete {
					t.Fatalf("valid registered source: %v %+v", err, status)
				}
			} else if err == nil || status.Accepted != 0 || !status.Disconnected {
				t.Fatalf("withdrawal or changed input accepted: %v %+v", err, status)
			}
		})
	}
}

func TestKernelAttemptDigestBindsRegisteredFields(t *testing.T) {
	_, a := kernelAttemptFixture(t)
	original, err := kernelAttemptResourceDigest(a)
	if err != nil {
		t.Fatal(err)
	}
	for _, field := range []string{"prompt", "command", "args", "directory", "provider", "role", "context", "workflow"} {
		t.Run(field, func(t *testing.T) {
			b := a
			switch field {
			case "prompt":
				b.Prompt += " changed"
			case "command":
				b.Command += "-changed"
			case "args":
				b.Args = append(append([]string{}, a.Args...), "changed")
			case "directory":
				b.CWD += "-changed"
			case "provider":
				b.Provider += "-changed"
			case "role":
				b.Role += "-changed"
			case "context":
				c := *a.Context
				c.SHA256 = strings.Repeat("d", 64)
				b.Context = &c
			case "workflow":
				w := *a.Workflow
				w.SHA256 = strings.Repeat("e", 64)
				b.Workflow = &w
			}
			digest, err := kernelAttemptResourceDigest(b)
			if err != nil || digest == original {
				t.Fatalf("registered change lost: %v", err)
			}
		})
	}
}
