//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"net"
	"os"
	"os/exec"
	"reflect"
	"strings"
	"testing"

	"golang.org/x/sys/unix"
	"swarm.local/companion/internal/enrollmentconfig"
	"swarm.local/companion/internal/identitycontract"
)

func configClientProcess(t *testing.T, pid int) *os.File {
	t.Helper()
	fd, err := unix.PidfdOpen(pid, 0)
	if err != nil {
		t.Fatal(err)
	}
	f := os.NewFile(uintptr(fd), "independent-fixture-server-pidfd")
	t.Cleanup(func() { f.Close() })
	return f
}

func configClientFixture(t *testing.T, s *agentIdentityConfigService) (*agentIdentityConfigClient, *agentIdentityConfigChannel) {
	t.Helper()
	server, conn := configServicePair(t)
	channel, err := newAgentIdentityConfigChannel(server)
	if err != nil {
		t.Fatal(err)
	}
	client, err := newAgentIdentityConfigClient(conn, configClientProcess(t, os.Getpid()), uint32(os.Getuid()), s.pins)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { client.close() })
	return client, channel
}

func TestAgentIdentityConfigClientRoundTripAndSingleUse(t *testing.T) {
	s := configServiceFixture(t)
	var head enrollmentconfig.Head
	for _, op := range []string{"preview", "create", "read", "apply"} {
		client, server := configClientFixture(t, s)
		done := make(chan error, 1)
		go func() { done <- server.serve(context.Background(), make([]byte, 65536), s) }()
		r := configServiceRequest(op, "initial", head)
		if op == "apply" {
			r.Change.EventID = "next"
		}
		reply, err := client.exchange(context.Background(), op, r.Change, make([]byte, 65536))
		if serveErr := <-done; err != nil || serveErr != nil {
			t.Fatalf("%s: %v %v", op, err, serveErr)
		}
		if e := server.serve(context.Background(), make([]byte, 65536), s); !errors.Is(e, errAgentIdentityConfigUsed) {
			t.Fatal("server channel reused")
		}
		if op == "preview" {
			if reply.Projection == nil || reply.View != nil {
				t.Fatal("preview lost projection")
			}
		} else {
			if reply.View == nil || reply.Projection != nil {
				t.Fatal("saved view missing")
			}
			head = reply.View.Head
		}
		if again, err := client.exchange(context.Background(), op, r.Change, make([]byte, 65536)); err == nil || !reflect.DeepEqual(again, agentIdentityConfigReply{}) {
			t.Fatal("channel reused")
		}
	}
	if head.Revision != 2 {
		t.Fatal("wrong persisted revision")
	}
}

func TestAgentIdentityConfigClientRejectsReplySubstitution(t *testing.T) {
	s := configServiceFixture(t)
	r := configServiceRequest("create", "initial", enrollmentconfig.Head{})
	reply, err := configServiceCall(t, s, r)
	if err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"schema", "request", "operation", "head", "pins", "both", "empty", "unknown-error", "error-with-data", "noncanonical", "duplicate", "different-change"} {
		t.Run(name, func(t *testing.T) {
			raw, _ := json.Marshal(reply)
			var candidate agentIdentityConfigReply
			if err := json.Unmarshal(raw, &candidate); err != nil {
				t.Fatal(err)
			}
			switch name {
			case "schema":
				candidate.Schema = 1
			case "request":
				candidate.RequestDigest = strings.Repeat("f", 64)
			case "operation":
				candidate.Operation = "read"
			case "head":
				candidate.View.Head.Digest = strings.Repeat("f", 64)
			case "pins":
				candidate.View.Snapshot.Pins.Principal.Subject = "other"
			case "both":
				candidate.Projection = &enrollmentconfig.Projection{}
			case "empty":
				candidate.View = nil
			case "unknown-error":
				candidate.View = nil
				candidate.Error = "private/raw error"
			case "error-with-data":
				candidate.Error = "denied"
			case "different-change":
				other := configServiceRequest("apply", "different", candidate.View.Head)
				got, e := configServiceCall(t, s, other)
				if e != nil {
					t.Fatal(e)
				}
				candidate.View = got.View
			}
			raw, _ = json.Marshal(candidate)
			if name == "noncanonical" {
				raw = append(raw, '\n')
			}
			if name == "duplicate" {
				raw = bytes.Replace(raw, []byte(`"schema_version":2`), []byte(`"schema_version":2,"schema_version":2`), 1)
			}
			got, e := decodeAgentIdentityConfigReply(raw, r, s.pins)
			if e == nil || !reflect.DeepEqual(got, agentIdentityConfigReply{}) {
				t.Fatalf("accepted %s", name)
			}
		})
	}
	for _, code := range []string{"denied", "conflict", "busy", "pending", "not_configured", "indeterminate", "unavailable", "invalid_request"} {
		candidate := agentIdentityConfigReply{Schema: 2, Operation: r.Operation, RequestDigest: agentIdentityConfigRequestDigest(mustConfigClientEncode(t, r)), Error: code}
		raw, _ := json.Marshal(candidate)
		got, e := decodeAgentIdentityConfigReply(raw, r, s.pins)
		if e != nil || got.Error != code {
			t.Fatalf("explicit failure envelope: %s %v", code, e)
		}
	}
}

func mustConfigClientEncode(t *testing.T, r agentIdentityConfigRequest) []byte {
	t.Helper()
	raw, e := encodeAgentIdentityConfigRequest(r)
	if e != nil {
		t.Fatal(e)
	}
	return raw
}

func TestAgentIdentityConfigClientCancellationAndDamage(t *testing.T) {
	for _, name := range []string{"before", "after-send", "truncated", "wrong-uid", "bad-pidfd"} {
		t.Run(name, func(t *testing.T) {
			s := configServiceFixture(t)
			if name == "bad-pidfd" {
				_, conn := configServicePair(t)
				file, e := os.Open(os.DevNull)
				if e != nil {
					t.Fatal(e)
				}
				defer file.Close()
				if _, e = newAgentIdentityConfigClient(conn, file, uint32(os.Getuid()), s.pins); e == nil {
					t.Fatal("regular file accepted as pidfd")
				}
				return
			}
			client, server := configClientFixture(t, s)
			if name == "wrong-uid" {
				client.uid = ^uint32(os.Getuid())
			}
			ctx, cancel := context.WithCancel(context.Background())
			defer cancel()
			var done chan error
			if name == "before" {
				cancel()
			} else {
				done = make(chan error, 1)
				go func() {
					if name == "after-send" {
						m, e := receiveAgentIdentityMessage(server.conn, make([]byte, 65536))
						if e == nil {
							e = m.Close()
						}
						cancel()
						done <- e
						return
					}
					done <- server.serve(context.Background(), make([]byte, 65536), s)
				}()
			}
			buffer := make([]byte, 65536)
			if name == "truncated" {
				buffer = buffer[:8]
			}
			r := configServiceRequest("create", "initial", enrollmentconfig.Head{})
			got, e := client.exchange(ctx, r.Operation, r.Change, buffer)
			if done != nil {
				if err := <-done; err != nil {
					t.Fatal(err)
				}
			}
			if e == nil || !reflect.DeepEqual(got, agentIdentityConfigReply{}) {
				t.Fatal("failure produced result")
			}
			if name == "before" {
				if errors.Is(e, enrollmentconfig.ErrIndeterminate) || !errors.Is(e, context.Canceled) {
					t.Fatal("unsent request wrongly uncertain")
				}
			} else if !errors.Is(e, enrollmentconfig.ErrIndeterminate) {
				t.Fatal("post-send mutation uncertainty lost")
			}
		})
	}
}

func TestAgentIdentityConfigClientSocketLimitAndExactRetry(t *testing.T) {
	s := configServiceFixture(t)
	client, server := configClientFixture(t, s)
	raw, e := server.conn.SyscallConn()
	if e != nil {
		t.Fatal(e)
	}
	var limit int
	var setup error
	if e = raw.Control(func(fd uintptr) {
		setup = unix.SetsockoptInt(int(fd), unix.SOL_SOCKET, unix.SO_SNDBUF, 4096)
		if setup == nil {
			limit, setup = unix.GetsockoptInt(int(fd), unix.SOL_SOCKET, unix.SO_SNDBUF)
		}
	}); e != nil || setup != nil {
		t.Fatalf("socket fixture: %v %v", e, setup)
	}
	// Derive the payload from the observed kernel buffer, not a product ceiling.
	r := configServiceRequest("create", "large", enrollmentconfig.Head{})
	r.Change.Reason = strings.Repeat("x", limit*2)
	done := make(chan error, 1)
	go func() {
		e := server.serve(context.Background(), make([]byte, len(r.Change.Reason)+65536), s)
		done <- e
	}()
	got, e := client.exchange(context.Background(), r.Operation, r.Change, make([]byte, len(r.Change.Reason)+65536))
	serveErr := <-done
	if !errors.Is(serveErr, unix.EMSGSIZE) || !errors.Is(serveErr, enrollmentconfig.ErrIndeterminate) || !errors.Is(e, enrollmentconfig.ErrIndeterminate) || !reflect.DeepEqual(got, agentIdentityConfigReply{}) {
		t.Fatalf("not actual EMSGSIZE: %v / %v", e, serveErr)
	}
	before := configServiceState(t, s)
	if before[0] == nil {
		t.Fatal("did not exercise committed effect")
	}
	client2, server2 := configClientFixture(t, s)
	go func() { done <- server2.serve(context.Background(), make([]byte, len(r.Change.Reason)+65536), s) }()
	reply, e := client2.exchange(context.Background(), r.Operation, r.Change, make([]byte, len(r.Change.Reason)+65536))
	if serveErr = <-done; e != nil || serveErr != nil || reply.View == nil || reply.View.Head.Revision != 1 || !reflect.DeepEqual(before, configServiceState(t, s)) {
		t.Fatal("large reply retry duplicated or lost effect")
	}
	t.Logf("observed SO_SNDBUF=%d bytes; reason=%d bytes; first response EMSGSIZE; retry unchanged", limit, len(r.Change.Reason))
}

func TestAgentIdentityConfigClientServerChild(t *testing.T) {
	if os.Getenv("SWARM_CONFIG_SERVER_CHILD") != "1" {
		return
	}
	var paths [2]string
	if e := json.Unmarshal([]byte(os.Getenv("SWARM_CONFIG_SERVER_PATHS")), &paths); e != nil {
		t.Fatal(e)
	}
	s := configServiceFixture(t)
	s.configPath, s.witnessPath = paths[0], paths[1]
	s.withAuthority = func(ctx context.Context, m *agentIdentityMessage, r agentIdentityConfigRequest, run func(identitycontract.Principal) error) error {
		if m.pid != int32(os.Getppid()) || m.uid != uint32(os.Getuid()) {
			return enrollmentconfig.ErrUnauthorized
		}
		return run(s.pins.Principal)
	}
	f := os.NewFile(3, "child-server-socket")
	conn, e := net.FileConn(f)
	f.Close()
	if e != nil {
		t.Fatal(e)
	}
	defer conn.Close()
	channel, e := newAgentIdentityConfigChannel(conn.(*net.UnixConn))
	if e != nil {
		t.Fatal(e)
	}
	if e = channel.serve(context.Background(), make([]byte, 65536), s); e != nil {
		t.Fatal(e)
	}
}

func TestAgentIdentityConfigClientAuthenticatesServerProcess(t *testing.T) {
	for _, correct := range []bool{true, false} {
		t.Run(map[bool]string{true: "pinned-child", false: "rogue-same-uid"}[correct], func(t *testing.T) {
			s := configServiceFixture(t)
			server, conn := configServicePair(t)
			if _, e := newAgentIdentityConfigChannel(server); e != nil {
				t.Fatal(e)
			}
			if e := configureAgentIdentityChannel(conn); e != nil {
				t.Fatal(e)
			}
			f, e := server.File()
			if e != nil {
				t.Fatal(e)
			}
			defer f.Close()
			paths, _ := json.Marshal([2]string{s.configPath, s.witnessPath})
			cmd := exec.Command(os.Args[0], "-test.run=^TestAgentIdentityConfigClientServerChild$", "-test.timeout=0")
			cmd.Env = append(os.Environ(), "SWARM_CONFIG_SERVER_CHILD=1", "SWARM_CONFIG_SERVER_PATHS="+string(paths))
			cmd.ExtraFiles = []*os.File{f}
			var out bytes.Buffer
			cmd.Stdout, cmd.Stderr = &out, &out
			if e = cmd.Start(); e != nil {
				t.Fatal(e)
			}
			pid := cmd.Process.Pid
			if !correct {
				pid = os.Getpid()
			}
			client, e := newAgentIdentityConfigClient(conn, configClientProcess(t, pid), uint32(os.Getuid()), s.pins)
			if e != nil {
				t.Fatal(e)
			}
			r := configServiceRequest("create", "child-initial", enrollmentconfig.Head{})
			reply, e := client.exchange(context.Background(), r.Operation, r.Change, make([]byte, 65536))
			if waitErr := cmd.Wait(); waitErr != nil {
				t.Fatalf("child: %v %s", waitErr, out.String())
			}
			if correct {
				if e != nil || reply.View == nil || reply.View.Head.Revision != 1 {
					t.Fatalf("genuine server: %v", e)
				}
			} else if !errors.Is(e, enrollmentconfig.ErrIndeterminate) || !reflect.DeepEqual(reply, agentIdentityConfigReply{}) {
				t.Fatal("rogue same UID accepted")
			}
		})
	}
}

func TestAgentIdentityConfigClientFailedSendClosesServerChannel(t *testing.T) {
	s := configServiceFixture(t)
	client, server := configClientFixture(t, s)
	raw, e := server.conn.SyscallConn()
	if e != nil {
		t.Fatal(e)
	}
	var setup error
	var limit int
	if e = raw.Control(func(fd uintptr) {
		setup = unix.SetsockoptInt(int(fd), unix.SOL_SOCKET, unix.SO_SNDBUF, 4096)
		if setup == nil {
			limit, setup = unix.GetsockoptInt(int(fd), unix.SOL_SOCKET, unix.SO_SNDBUF)
		}
	}); e != nil || setup != nil {
		t.Fatalf("socket setup: %v %v", e, setup)
	}
	r := configServiceRequest("create", "closure", enrollmentconfig.Head{})
	r.Change.Reason = strings.Repeat("x", limit*2)
	if _, e = client.conn.Write(mustConfigClientEncode(t, r)); e != nil {
		t.Fatal(e)
	}
	if e = server.serve(context.Background(), make([]byte, len(r.Change.Reason)+65536), s); !errors.Is(e, unix.EMSGSIZE) {
		t.Fatalf("wrong send failure: %v", e)
	}
	peer, e := client.conn.SyscallConn()
	if e != nil {
		t.Fatal(e)
	}
	var events int16
	var pollErr error
	if e = peer.Control(func(fd uintptr) {
		fds := []unix.PollFd{{Fd: int32(fd), Events: unix.POLLIN}}
		_, pollErr = unix.Poll(fds, 0) // Nonblocking state observation, not a watchdog.
		events = fds[0].Revents
	}); e != nil || pollErr != nil {
		t.Fatalf("peer inspection: %v %v", e, pollErr)
	}
	if events&unix.POLLHUP == 0 {
		t.Fatal("failed server exchange leaves client waiting on an open channel")
	}
}

func TestAgentIdentityConfigClientUnavailableDoesNotInventStorageCause(t *testing.T) {
	err := agentIdentityConfigReplyError("unavailable")
	if err == nil || errors.Is(err, enrollmentconfig.ErrStorage) {
		t.Fatalf("general unavailability becomes a storage diagnosis: %v", err)
	}
}

func TestAgentIdentityConfigClientFreshChallengeAndDescriptorOwnership(t *testing.T) {
	s := configServiceFixture(t)
	gate := s.withAuthority
	var challenges []string
	s.withAuthority = func(ctx context.Context, m *agentIdentityMessage, r agentIdentityConfigRequest, run func(identitycontract.Principal) error) error {
		challenges = append(challenges, r.Challenge)
		return gate(ctx, m, r, run)
	}
	for i := 0; i < 2; i++ {
		server, conn := configServicePair(t)
		channel, e := newAgentIdentityConfigChannel(server)
		if e != nil {
			t.Fatal(e)
		}
		original := configClientProcess(t, os.Getpid())
		client, e := newAgentIdentityConfigClient(conn, original, uint32(os.Getuid()), s.pins)
		if e != nil {
			t.Fatal(e)
		}
		flags, e := unix.FcntlInt(client.process.Fd(), unix.F_GETFD, 0)
		if e != nil || flags&unix.FD_CLOEXEC == 0 {
			t.Fatal("duplicate inherited across exec")
		}
		done := make(chan error, 1)
		go func() { done <- channel.serve(context.Background(), make([]byte, 65536), s) }()
		r := configServiceRequest("create", "same-event", enrollmentconfig.Head{})
		if _, e = client.exchange(context.Background(), r.Operation, r.Change, make([]byte, 65536)); e != nil {
			t.Fatal(e)
		}
		if e = <-done; e != nil {
			t.Fatal(e)
		}
		if _, e = original.Stat(); e != nil {
			t.Fatal("client closed launcher's descriptor")
		}
		if _, e = client.process.Stat(); !errors.Is(e, os.ErrClosed) {
			t.Fatal("client duplicate remains open")
		}
		if _, e = client.conn.Write([]byte("late")); !errors.Is(e, net.ErrClosed) {
			t.Fatal("client connection remains open")
		}
	}
	if len(challenges) != 2 || challenges[0] == challenges[1] {
		t.Fatal("retry did not create a fresh challenge")
	}
}

func TestAgentIdentityConfigClientRejectsStaleChallenge(t *testing.T) {
	s := configServiceFixture(t)
	r := configServiceRequest("create", "initial", enrollmentconfig.Head{})
	reply, e := configServiceCall(t, s, r)
	if e != nil {
		t.Fatal(e)
	}
	r.Challenge = strings.Repeat("A", 43)
	raw, e := json.Marshal(reply)
	if e != nil {
		t.Fatal(e)
	}
	if result, e := decodeAgentIdentityConfigReply(raw, r, s.pins); e == nil || !reflect.DeepEqual(result, agentIdentityConfigReply{}) {
		t.Fatal("old reply accepted for a fresh request challenge")
	}
}

func TestAgentIdentityConfigClientRejectsPinsBeforeAuthority(t *testing.T) {
	s := configServiceFixture(t)
	s.withAuthority = func(context.Context, *agentIdentityMessage, agentIdentityConfigRequest, func(identitycontract.Principal) error) error {
		t.Fatal("foreign pins reached authority")
		return nil
	}
	r := configServiceRequest("create", "initial", enrollmentconfig.Head{})
	r.Pins.Principal.Subject = "foreign-installation-human"
	reply, e := configServiceCall(t, s, r)
	state := configServiceState(t, s)
	if !errors.Is(e, enrollmentconfig.ErrPins) || reply.View != nil || state[0] != nil || state[1] != nil {
		t.Fatal("foreign pins mutated or succeeded")
	}
}
