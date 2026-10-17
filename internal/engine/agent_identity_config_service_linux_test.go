//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"errors"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"

	"golang.org/x/sys/unix"
	"swarm.local/companion/internal/enrollmentconfig"
	"swarm.local/companion/internal/identitycontract"
)

func configServicePins() enrollmentconfig.Pins {
	return enrollmentconfig.Pins{InstallationDigest: strings.Repeat("a", 64), PublicKey: base64.RawURLEncoding.EncodeToString(bytes.Repeat([]byte{7}, 32)),
		Principal: identitycontract.Principal{ID: "operator", Kind: "human", Issuer: "https://issuer.example", Subject: "initial-human"}}
}

func configServiceFixture(t *testing.T) *agentIdentityConfigService {
	t.Helper()
	dir, witness := t.TempDir(), t.TempDir()
	for _, path := range []string{dir, witness} {
		if err := os.Chmod(path, 0700); err != nil {
			t.Fatal(err)
		}
	}
	s := &agentIdentityConfigService{configPath: dir, witnessPath: witness, owner: uint32(os.Geteuid()),
		pins: configServicePins(),
		now:  func() time.Time { return time.Date(2026, 10, 17, 14, 0, 0, 0, time.UTC) }}
	s.withAuthority = func(ctx context.Context, message *agentIdentityMessage, request agentIdentityConfigRequest, run func(identitycontract.Principal) error) error {
		if message == nil || message.process == nil || message.pid != int32(os.Getpid()) || message.uid != uint32(os.Getuid()) {
			return enrollmentconfig.ErrUnauthorized
		}
		return run(s.pins.Principal)
	}
	return s
}

func configServiceRequest(op, event string, head enrollmentconfig.Head) agentIdentityConfigRequest {
	r := agentIdentityConfigRequest{Schema: 2, Challenge: base64.RawURLEncoding.EncodeToString(bytes.Repeat([]byte{3}, 32)), Pins: configServicePins(), Operation: op}
	if op != "read" {
		r.Change = &enrollmentconfig.Change{Schema: 1, EventID: event, Expected: head, Values: enrollmentconfig.Values{Mode: "local", SocketPath: "/run/swarm-authority/control.sock"}, Reason: "fixture operator decision"}
	}
	return r
}

func configServicePair(t *testing.T) (*net.UnixConn, *net.UnixConn) {
	t.Helper()
	fds, err := unix.Socketpair(unix.AF_UNIX, unix.SOCK_SEQPACKET|unix.SOCK_CLOEXEC, 0)
	if err != nil {
		t.Fatal(err)
	}
	var pair [2]*net.UnixConn
	for i, fd := range fds {
		file := os.NewFile(uintptr(fd), "config-service-test-socket")
		conn, err := net.FileConn(file)
		file.Close()
		if err != nil {
			t.Fatal(err)
		}
		pair[i] = conn.(*net.UnixConn)
		t.Cleanup(func() { conn.Close() })
	}
	return pair[0], pair[1]
}

func configServiceExchange(t *testing.T, s *agentIdentityConfigService, raw []byte) (agentIdentityConfigReply, error) {
	t.Helper()
	server, client := configServicePair(t)
	channel, err := newAgentIdentityConfigChannel(server)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = client.Write(raw); err != nil {
		t.Fatal(err)
	}
	serveErr := channel.serve(context.Background(), make([]byte, 65536), s)
	buffer := make([]byte, 65536)
	n, _, flags, _, err := client.ReadMsgUnix(buffer, nil)
	if err != nil || flags&unix.MSG_TRUNC != 0 {
		t.Fatalf("reply packet: %d %v", flags, err)
	}
	var reply agentIdentityConfigReply
	if err = json.Unmarshal(buffer[:n], &reply); err != nil {
		t.Fatal(err)
	}
	canonical, err := json.Marshal(reply)
	if err != nil || !bytes.Equal(canonical, buffer[:n]) {
		t.Fatal("noncanonical reply")
	}
	if reply.Schema != 2 {
		t.Fatal("missing reply schema")
	}
	if serveErr != nil && (reply.Error == "" || reply.View != nil || reply.Projection != nil) {
		t.Fatal("failure exposed success data")
	}
	return reply, serveErr
}

func configServiceCall(t *testing.T, s *agentIdentityConfigService, r agentIdentityConfigRequest) (agentIdentityConfigReply, error) {
	t.Helper()
	raw, err := encodeAgentIdentityConfigRequest(r)
	if err != nil {
		t.Fatal(err)
	}
	return configServiceExchange(t, s, raw)
}

func configServiceState(t *testing.T, s *agentIdentityConfigService) [2][]byte {
	t.Helper()
	var result [2][]byte
	for i, path := range []string{filepath.Join(s.configPath, "config.json"), filepath.Join(s.witnessPath, "witness.json")} {
		raw, err := os.ReadFile(path)
		if err != nil && !os.IsNotExist(err) {
			t.Fatal(err)
		}
		result[i] = raw
	}
	return result
}

func TestAgentIdentityConfigServiceCanonicalRequests(t *testing.T) {
	r := configServiceRequest("create", "initial", enrollmentconfig.Head{})
	raw, err := encodeAgentIdentityConfigRequest(r)
	if err != nil {
		t.Fatal(err)
	}
	for _, op := range []string{"read", "preview", "create", "apply"} {
		t.Run(op, func(t *testing.T) {
			want := configServiceRequest(op, "event", enrollmentconfig.Head{})
			encoded, err := encodeAgentIdentityConfigRequest(want)
			if err != nil {
				t.Fatal(err)
			}
			got, err := decodeAgentIdentityConfigRequest(encoded)
			if err != nil || !reflect.DeepEqual(got, want) {
				t.Fatalf("round trip: %#v %v", got, err)
			}
		})
	}
	bad := map[string][]byte{
		"duplicate":         bytes.Replace(raw, []byte(`"schema_version":2`), []byte(`"schema_version":2,"schema_version":2`), 1),
		"folded":            bytes.Replace(raw, []byte(`"operation"`), []byte(`"Operation"`), 1),
		"actor":             append(append([]byte{}, raw[:len(raw)-1]...), []byte(`,"actor":"admin"}`)...),
		"trailing":          append(append([]byte{}, raw...), '\n'),
		"missing-change":    []byte(`{"schema_version":1,"operation":"read"}`),
		"null-change":       []byte(`{"schema_version":1,"operation":"create","change":null}`),
		"unknown-op":        []byte(`{"schema_version":1,"operation":"recover","change":null}`),
		"unknown-schema":    bytes.Replace(raw, []byte(`"schema_version":2`), []byte(`"schema_version":3`), 1),
		"legacy-version":    bytes.Replace(raw, []byte(`"schema_version":2`), []byte(`"schema_version":1`), 1),
		"missing-context":   []byte(`{"schema_version":2,"operation":"read","change":null}`),
		"invalid-challenge": bytes.Replace(raw, []byte(r.Challenge), []byte("bad"), 1),
		"null":              []byte(`null`),
		"nested-extra":      bytes.Replace(raw, []byte(`"mode":"local"`), []byte(`"mode":"local","admin":true`), 1),
	}
	for name, packet := range bad {
		t.Run(name, func(t *testing.T) {
			got, err := decodeAgentIdentityConfigRequest(packet)
			if err == nil || !reflect.DeepEqual(got, agentIdentityConfigRequest{}) {
				t.Fatalf("accepted %s: %#v %v", name, got, err)
			}
			s := configServiceFixture(t)
			s.withAuthority = func(context.Context, *agentIdentityMessage, agentIdentityConfigRequest, func(identitycontract.Principal) error) error {
				t.Fatal("malformed request reached authority")
				return nil
			}
			reply, err := configServiceExchange(t, s, packet)
			state := configServiceState(t, s)
			if err == nil || reply.Error != "invalid_request" || state[0] != nil || state[1] != nil {
				t.Fatal("malformed request mutated or succeeded")
			}
		})
	}
	readWithChange := r
	readWithChange.Operation = "read"
	if _, err := encodeAgentIdentityConfigRequest(readWithChange); err == nil {
		t.Fatal("read accepts change")
	}
}

func TestAgentIdentityConfigServicePreviewSaveReadCASAndReplay(t *testing.T) {
	s := configServiceFixture(t)
	initial := configServiceRequest("create", "initial", enrollmentconfig.Head{})
	preview := initial
	preview.Operation = "preview"
	p, err := configServiceCall(t, s, preview)
	if err != nil || p.Projection == nil || p.View != nil || p.Projection.Next.Head.Revision != 1 {
		t.Fatalf("preview: %#v %v", p, err)
	}
	if got := configServiceState(t, s); got[0] != nil || got[1] != nil {
		t.Fatal("preview persisted state")
	}
	first, err := configServiceCall(t, s, initial)
	if err != nil || first.View == nil || !reflect.DeepEqual(*first.View, p.Projection.Next) {
		t.Fatalf("create: %#v %v", first, err)
	}
	second, err := configServiceCall(t, s, configServiceRequest("apply", "next", first.View.Head))
	if err != nil || second.View == nil || second.View.Head.Revision != 2 {
		t.Fatalf("apply: %#v %v", second, err)
	}
	state := configServiceState(t, s)
	stale, err := configServiceCall(t, s, configServiceRequest("apply", "stale", first.View.Head))
	if !errors.Is(err, enrollmentconfig.ErrConflict) || stale.Error != "conflict" || !reflect.DeepEqual(state, configServiceState(t, s)) {
		t.Fatal("CAS accepted or mutated")
	}
	replay, err := configServiceCall(t, s, configServiceRequest("apply", "next", first.View.Head))
	if err != nil || !reflect.DeepEqual(replay.View, second.View) || !reflect.DeepEqual(state, configServiceState(t, s)) {
		t.Fatal("retry duplicated or rewrote event")
	}
	read, err := configServiceCall(t, s, configServiceRequest("read", "", enrollmentconfig.Head{}))
	if err != nil || !reflect.DeepEqual(read.View, second.View) {
		t.Fatal("read lost saved state")
	}
}

func TestAgentIdentityConfigServiceRequiresScopedAuthority(t *testing.T) {
	for _, name := range []string{"absent", "deny", "wrong-human", "no-execute", "swallowed-error", "double-execute", "after-execute", "request-mutation"} {
		t.Run(name, func(t *testing.T) {
			s := configServiceFixture(t)
			cause := errors.New("private fixture authority failure")
			var retained func(identitycontract.Principal) error
			s.withAuthority = func(ctx context.Context, m *agentIdentityMessage, r agentIdentityConfigRequest, run func(identitycontract.Principal) error) error {
				retained = run
				switch name {
				case "deny":
					return cause
				case "wrong-human":
					p := s.pins.Principal
					p.Subject = "different"
					return run(p)
				case "no-execute":
					return nil
				case "swallowed-error":
					_ = run(identitycontract.Principal{})
					return nil
				case "double-execute":
					_ = run(s.pins.Principal)
					_ = run(s.pins.Principal)
					return nil
				case "after-execute":
					if err := run(s.pins.Principal); err != nil {
						return err
					}
					return cause
				case "request-mutation":
					r.Change.Reason = "mutated callback copy"
					return run(s.pins.Principal)
				}
				return nil
			}
			if name == "absent" {
				s.withAuthority = nil
			}
			reply, err := configServiceCall(t, s, configServiceRequest("create", "initial", enrollmentconfig.Head{}))
			if name == "request-mutation" {
				if err != nil || reply.View == nil || reply.View.Snapshot.History[0].Change.Reason != "fixture operator decision" {
					t.Fatal("authority callback altered request")
				}
			} else {
				if err == nil || reply.Error == "" {
					t.Fatal("invalid authority succeeded")
				}
				if strings.Contains(reply.Error, "private") {
					t.Fatal("private cause leaked")
				}
				if name == "deny" || name == "after-execute" {
					if !errors.Is(err, cause) {
						t.Fatal("original cause lost")
					}
				}
				state := configServiceState(t, s)
				if name != "double-execute" && name != "after-execute" && (state[0] != nil || state[1] != nil) {
					t.Fatal("denial mutated state")
				}
				if name == "double-execute" || name == "after-execute" {
					if !errors.Is(err, enrollmentconfig.ErrIndeterminate) || state[0] == nil {
						t.Fatal("post-effect failure hid uncertainty")
					}
				}
			}
			if retained != nil {
				before := configServiceState(t, s)
				if retained(s.pins.Principal) == nil || !reflect.DeepEqual(before, configServiceState(t, s)) {
					t.Fatal("late execution accepted")
				}
			}
		})
	}
}

func TestAgentIdentityConfigServiceReadIsAuthorized(t *testing.T) {
	s := configServiceFixture(t)
	if _, err := configServiceCall(t, s, configServiceRequest("create", "initial", enrollmentconfig.Head{})); err != nil {
		t.Fatal(err)
	}
	before := configServiceState(t, s)
	s.withAuthority = func(context.Context, *agentIdentityMessage, agentIdentityConfigRequest, func(identitycontract.Principal) error) error {
		return enrollmentconfig.ErrUnauthorized
	}
	reply, err := configServiceCall(t, s, configServiceRequest("read", "", enrollmentconfig.Head{}))
	if !errors.Is(err, enrollmentconfig.ErrUnauthorized) || reply.View != nil || !reflect.DeepEqual(before, configServiceState(t, s)) {
		t.Fatal("unauthorized read exposed state")
	}
}

func TestAgentIdentityConfigServiceRejectsTransportDamage(t *testing.T) {
	for _, name := range []string{"truncated", "rights"} {
		t.Run(name, func(t *testing.T) {
			s := configServiceFixture(t)
			server, client := configServicePair(t)
			channel, err := newAgentIdentityConfigChannel(server)
			if err != nil {
				t.Fatal(err)
			}
			raw, err := encodeAgentIdentityConfigRequest(configServiceRequest("create", "initial", enrollmentconfig.Head{}))
			if err != nil {
				t.Fatal(err)
			}
			var oob []byte
			buffer := make([]byte, 65536)
			if name == "truncated" {
				buffer = buffer[:8]
			} else {
				f, err := os.Open(os.DevNull)
				if err != nil {
					t.Fatal(err)
				}
				defer f.Close()
				oob = unix.UnixRights(int(f.Fd()))
			}
			if _, _, err = client.WriteMsgUnix(raw, oob, nil); err != nil {
				t.Fatal(err)
			}
			if err = channel.serve(context.Background(), buffer, s); err == nil {
				t.Fatal("damaged transport accepted")
			}
			state := configServiceState(t, s)
			if state[0] != nil || state[1] != nil {
				t.Fatal("transport rejection mutated")
			}
		})
	}
}

func TestAgentIdentityConfigServiceLostReplyReopensWithoutDuplicate(t *testing.T) {
	s := configServiceFixture(t)
	server, client := configServicePair(t)
	channel, err := newAgentIdentityConfigChannel(server)
	if err != nil {
		t.Fatal(err)
	}
	r := configServiceRequest("create", "initial", enrollmentconfig.Head{})
	raw, err := encodeAgentIdentityConfigRequest(r)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = client.Write(raw); err != nil {
		t.Fatal(err)
	}
	if err = client.Close(); err != nil {
		t.Fatal(err)
	}
	err = channel.serve(context.Background(), make([]byte, 65536), s)
	if !errors.Is(err, enrollmentconfig.ErrIndeterminate) {
		t.Fatalf("lost reply not uncertain: %v", err)
	}
	before := configServiceState(t, s)
	if before[0] == nil || before[1] == nil {
		t.Fatal("did not exercise committed mutation")
	}
	reply, err := configServiceCall(t, s, r)
	if err != nil || reply.View == nil || reply.View.Head.Revision != 1 || !reflect.DeepEqual(before, configServiceState(t, s)) {
		t.Fatal("exact retry rewrote event")
	}
}

func TestAgentIdentityConfigServiceExactApplyRecoversPending(t *testing.T) {
	s := configServiceFixture(t)
	r := configServiceRequest("create", "pending-event", enrollmentconfig.Head{})
	initial, err := configServiceCall(t, s, r)
	if err != nil {
		t.Fatal(err)
	}
	// Isolated fixture of the durable pre-config window, not a real process crash.
	intent := struct {
		Schema    int                   `json:"schema_version"`
		Pins      enrollmentconfig.Pins `json:"pins"`
		Committed enrollmentconfig.Head `json:"committed"`
		Pending   *struct {
			Entry  enrollmentconfig.Entry `json:"entry"`
			Target enrollmentconfig.Head  `json:"target"`
		} `json:"pending"`
	}{Schema: 1, Pins: s.pins}
	intent.Pending = &struct {
		Entry  enrollmentconfig.Entry `json:"entry"`
		Target enrollmentconfig.Head  `json:"target"`
	}{initial.View.Snapshot.History[0], initial.View.Head}
	raw, err := json.Marshal(intent)
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(s.witnessPath, "witness.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	if err = os.Remove(filepath.Join(s.configPath, "config.json")); err != nil {
		t.Fatal(err)
	}
	pending := configServiceState(t, s)
	reply, err := configServiceCall(t, s, configServiceRequest("read", "", enrollmentconfig.Head{}))
	if !errors.Is(err, enrollmentconfig.ErrPending) || reply.Error != "pending" {
		t.Fatal("pending exposed as confirmed")
	}
	reply, err = configServiceCall(t, s, configServiceRequest("apply", "different", enrollmentconfig.Head{}))
	if !errors.Is(err, enrollmentconfig.ErrPending) || !reflect.DeepEqual(pending, configServiceState(t, s)) {
		t.Fatal("different recovery mutated")
	}
	authorize := s.withAuthority
	s.withAuthority = func(context.Context, *agentIdentityMessage, agentIdentityConfigRequest, func(identitycontract.Principal) error) error {
		return enrollmentconfig.ErrUnauthorized
	}
	r.Operation = "apply"
	if _, err = configServiceCall(t, s, r); !errors.Is(err, enrollmentconfig.ErrUnauthorized) || !reflect.DeepEqual(pending, configServiceState(t, s)) {
		t.Fatal("denied recovery mutated")
	}
	s.withAuthority = authorize
	s.now = func() time.Time { return time.Date(2026, 10, 18, 0, 0, 0, 0, time.UTC) }
	reply, err = configServiceCall(t, s, r)
	if err != nil || !reflect.DeepEqual(reply.View, initial.View) {
		t.Fatalf("exact recovery changed entry or timestamp: %#v %v", reply, err)
	}
}

func TestAgentIdentityConfigServiceCancellationAndMissingConfig(t *testing.T) {
	s := configServiceFixture(t)
	reply, err := configServiceCall(t, s, configServiceRequest("read", "", enrollmentconfig.Head{}))
	if !errors.Is(err, enrollmentconfig.ErrNotConfigured) || reply.Error != "not_configured" {
		t.Fatal("missing config reported success")
	}
	server, _ := configServicePair(t)
	channel, err := newAgentIdentityConfigChannel(server)
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if err = channel.serve(ctx, make([]byte, 65536), s); !errors.Is(err, context.Canceled) {
		t.Fatal("cancelled request attempted receive")
	}
	state := configServiceState(t, s)
	if state[0] != nil || state[1] != nil {
		t.Fatal("cancel or missing read created state")
	}
}

func TestAgentIdentityConfigServiceChild(t *testing.T) {
	if os.Getenv("SWARM_IDENTITY_CONFIG_FIXTURE_CHILD") != "1" {
		return
	}
	file := os.NewFile(3, "inherited-config-client")
	conn, err := net.FileConn(file)
	file.Close()
	if err != nil {
		t.Fatal(err)
	}
	defer conn.Close()
	raw, err := encodeAgentIdentityConfigRequest(configServiceRequest("create", "child-event", enrollmentconfig.Head{}))
	if err != nil {
		t.Fatal(err)
	}
	if _, err = conn.Write(raw); err != nil {
		t.Fatal(err)
	}
	buffer := make([]byte, 65536)
	n, err := conn.Read(buffer)
	if err != nil {
		t.Fatal(err)
	}
	var reply agentIdentityConfigReply
	if err = json.Unmarshal(buffer[:n], &reply); err != nil {
		t.Fatal(err)
	}
	if os.Getenv("SWARM_IDENTITY_CONFIG_EXPECT_DENY") == "1" {
		if reply.Error != "denied" || reply.View != nil || reply.Projection != nil {
			t.Fatalf("child denial: %#v", reply)
		}
		return
	}
	if reply.Error != "" || reply.View == nil || reply.View.Head.Revision != 1 {
		t.Fatalf("child reply: %#v %v", reply, err)
	}
}

func TestAgentIdentityConfigServiceAttributesSendingChild(t *testing.T) {
	for _, allow := range []bool{true, false} {
		t.Run(map[bool]string{true: "allowed-sender", false: "same-uid-denied"}[allow], func(t *testing.T) {
			s := configServiceFixture(t)
			server, client := configServicePair(t)
			channel, err := newAgentIdentityConfigChannel(server)
			if err != nil {
				t.Fatal(err)
			}
			file, err := client.File()
			if err != nil {
				t.Fatal(err)
			}
			defer file.Close()
			cmd := exec.Command(os.Args[0], "-test.run=^TestAgentIdentityConfigServiceChild$", "-test.timeout=0")
			cmd.Env = append(os.Environ(), "SWARM_IDENTITY_CONFIG_FIXTURE_CHILD=1")
			if !allow {
				cmd.Env = append(cmd.Env, "SWARM_IDENTITY_CONFIG_EXPECT_DENY=1")
			}
			cmd.ExtraFiles = []*os.File{file}
			var output bytes.Buffer
			cmd.Stdout, cmd.Stderr = &output, &output
			if err = cmd.Start(); err != nil {
				t.Fatal(err)
			}
			observed := int32(0)
			s.withAuthority = func(ctx context.Context, m *agentIdentityMessage, r agentIdentityConfigRequest, run func(identitycontract.Principal) error) error {
				observed = m.pid
				if m.process == nil || m.pid != int32(cmd.Process.Pid) || m.pid == int32(os.Getpid()) || m.uid != uint32(os.Getuid()) {
					return enrollmentconfig.ErrUnauthorized
				}
				if !allow {
					return enrollmentconfig.ErrUnauthorized
				}
				return run(s.pins.Principal)
			}
			err = channel.serve(context.Background(), make([]byte, 65536), s)
			if waitErr := cmd.Wait(); (allow && err != nil) || (!allow && !errors.Is(err, enrollmentconfig.ErrUnauthorized)) || waitErr != nil {
				t.Fatalf("IPC: %v %v %s", err, waitErr, output.String())
			}
			if observed != int32(cmd.Process.Pid) || (configServiceState(t, s)[0] != nil) != allow {
				t.Fatal("no child attribution or persisted effect")
			}
		})
	}
}
