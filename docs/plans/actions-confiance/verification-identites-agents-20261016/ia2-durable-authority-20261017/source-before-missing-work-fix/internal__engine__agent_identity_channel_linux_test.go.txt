//go:build linux

package engine

import (
	"encoding/json"
	"net"
	"os"
	"strings"
	"testing"
	"time"

	"golang.org/x/sys/unix"
)

func agentIdentitySocketPair(t *testing.T, kind int) (*net.UnixConn, *net.UnixConn) {
	t.Helper()
	fds, err := unix.Socketpair(unix.AF_UNIX, kind|unix.SOCK_CLOEXEC, 0)
	if err != nil {
		t.Fatal(err)
	}
	var connections [2]*net.UnixConn
	for i, fd := range fds {
		file := os.NewFile(uintptr(fd), "identity-test-socket")
		conn, e := net.FileConn(file)
		file.Close()
		if e != nil {
			t.Fatal(e)
		}
		connections[i] = conn.(*net.UnixConn)
		t.Cleanup(func() { conn.Close() })
		connections[i].SetDeadline(time.Now().Add(5 * time.Second))
	}
	return connections[0], connections[1]
}

func TestAgentIdentityChannelRequiresPacketSocket(t *testing.T) {
	stream, _ := agentIdentitySocketPair(t, unix.SOCK_STREAM)
	if err := configureAgentIdentityChannel(stream); err == nil {
		t.Fatal("stream channel accepted")
	}
	if err := configureAgentIdentityChannel(nil); err == nil {
		t.Fatal("nil channel accepted")
	}
}

func TestAgentIdentityChannelReceivesKernelProcessHandle(t *testing.T) {
	server, client := agentIdentitySocketPair(t, unix.SOCK_SEQPACKET)
	if err := configureAgentIdentityChannel(server); err != nil {
		t.Fatal(err)
	}
	body, _ := json.Marshal(agentIdentityTestRequest())
	if _, err := client.Write(body); err != nil {
		t.Fatal(err)
	}
	buffer := make([]byte, 65536)
	message, err := receiveAgentIdentityMessage(server, buffer)
	if err != nil {
		t.Fatal(err)
	}
	defer message.Close()
	if message.pid != int32(os.Getpid()) || message.uid != uint32(os.Getuid()) || string(message.raw) != string(body) {
		t.Fatalf("lost message attribution: pid=%d uid=%d raw=%s", message.pid, message.uid, message.raw)
	}
	buffer[0] = 'x'
	if string(message.raw) != string(body) {
		t.Fatal("message aliases caller's reused transport buffer")
	}
	flags, err := unix.FcntlInt(message.process.Fd(), unix.F_GETFD, 0)
	if err != nil || flags&unix.FD_CLOEXEC == 0 {
		t.Fatalf("received process FD is inheritable: %d %v", flags, err)
	}
	pins, err := agentIdentityProcessNamespaces(message.process)
	if err != nil {
		t.Fatal(err)
	}
	for i, name := range []string{"pid", "mnt"} {
		actual, e := agentIdentityNamespaceOf(pins[i])
		pins[i].Close()
		expected, e2 := os.Open("/proc/self/ns/" + name)
		if e != nil || e2 != nil {
			t.Fatalf("namespace comparison unavailable: %v %v", e, e2)
		}
		want, e := agentIdentityNamespaceOf(expected)
		expected.Close()
		if e != nil || actual != want {
			t.Fatalf("wrong %s namespace: %+v != %+v (%v)", name, actual, want, e)
		}
	}
}

func TestAgentIdentityChannelRejectsMissingTruncatedAndInjectedMetadata(t *testing.T) {
	for _, name := range []string{"unconfigured", "payload-truncated", "rights-injected", "many-rights", "empty"} {
		t.Run(name, func(t *testing.T) {
			server, client := agentIdentitySocketPair(t, unix.SOCK_SEQPACKET)
			if name != "unconfigured" {
				if err := configureAgentIdentityChannel(server); err != nil {
					t.Fatal(err)
				}
			}
			body := []byte(strings.Repeat("x", 100))
			var oob []byte
			if name == "rights-injected" || name == "many-rights" {
				file, err := os.Open(os.DevNull)
				if err != nil {
					t.Fatal(err)
				}
				defer file.Close()
				fds := []int{int(file.Fd())}
				if name == "many-rights" {
					for i := 0; i < 30; i++ {
						fds = append(fds, int(file.Fd()))
					}
				}
				oob = unix.UnixRights(fds...)
			}
			if name == "empty" {
				body = nil
			}
			before, err := os.ReadDir("/proc/self/fd")
			if err != nil {
				t.Fatal("descriptor inventory unavailable: ", err)
			}
			for attempt := 0; attempt < 3; attempt++ {
				if _, _, err := client.WriteMsgUnix(body, oob, nil); err != nil {
					t.Fatal(err)
				}
				buffer := make([]byte, 256)
				if name == "payload-truncated" {
					buffer = buffer[:8]
				}
				message, err := receiveAgentIdentityMessage(server, buffer)
				if err == nil || message != nil {
					if message != nil {
						message.Close()
					}
					t.Fatal("incomplete or injected message accepted")
				}
			}
			after, err := os.ReadDir("/proc/self/fd")
			if err != nil {
				t.Fatal("descriptor inventory unavailable after rejection: ", err)
			}
			if len(after) != len(before) {
				t.Fatalf("rejected ancillary descriptors leaked: before=%d after=%d", len(before), len(after))
			}
		})
	}
}
