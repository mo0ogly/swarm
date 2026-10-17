//go:build linux

package engine

import (
	"bufio"
	"bytes"
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"syscall"
	"time"
	"unicode/utf8"
)

const kernelCheckProtocol = "swarm-kernel-fixture-v1"
const kernelCheckFixture = "fixed main-thread fork/openat/unix-connect fixture v1"

var kernelCheckExitArgs = func() []string { return []string{"_kernel_fixture_exit"} }

type kernelCheckBootstrap struct {
	Protocol    string                 `json:"protocol"`
	Nonce       string                 `json:"nonce"`
	Socket      string                 `json:"socket"`
	Parent      int                    `json:"parent"`
	ParentStart string                 `json:"parent_start"`
	Policy      TrustKernelCheckPolicy `json:"policy"`
}

func kernelCheckLease(pid int, start, nonce string) kernelObservationLease {
	return kernelObservationLease{AgentID: "owned-kernel-fixture", Mission: "diagnostic", Task: "kernel-check", Attempt: nonce, PID: pid, Start: start, Host: hostIdentity(), ResourceDigest: hash([]byte(kernelCheckFixture))}
}

func kernelCheckRead(r *bufio.Reader, out any) error {
	b, err := r.ReadSlice('\n')
	if err != nil {
		return err
	}
	if len(b)-1 > kernelObservationFrameBytes {
		return errors.New("kernel diagnostic protocol exceeds 16384 bytes")
	}
	if !utf8.Valid(b) {
		return errors.New("invalid kernel diagnostic UTF-8")
	}
	d := json.NewDecoder(bytes.NewReader(b[:len(b)-1]))
	if err = kernelCheckJSONValue(d); err != nil {
		return err
	}
	if _, err = d.Token(); err != io.EOF {
		return errors.New("trailing kernel diagnostic data")
	}
	return strict(b[:len(b)-1], out)
}

func kernelCheckJSONValue(d *json.Decoder) error {
	t, err := d.Token()
	if err != nil {
		return err
	}
	if delim, ok := t.(json.Delim); ok {
		switch delim {
		case '{':
			seen := map[string]bool{}
			for d.More() {
				k, e := d.Token()
				if e != nil {
					return e
				}
				s, ok := k.(string)
				if !ok || seen[s] {
					return errors.New("duplicate kernel diagnostic key")
				}
				seen[s] = true
				if e = kernelCheckJSONValue(d); e != nil {
					return e
				}
			}
			end, e := d.Token()
			if e != nil || end != json.Delim('}') {
				return errors.New("invalid kernel diagnostic object")
			}
		case '[':
			for d.More() {
				if err = kernelCheckJSONValue(d); err != nil {
					return err
				}
			}
			end, e := d.Token()
			if e != nil || end != json.Delim(']') {
				return errors.New("invalid kernel diagnostic array")
			}
		default:
			return errors.New("unexpected kernel diagnostic delimiter")
		}
	}
	return nil
}

func kernelCheckFixedResult() TrustKernelCheckResult {
	return TrustKernelCheckResult{SchemaVersion: 1, Source: "ebpf", Scope: "owned_fixture_only", Events: []kernelObservation{}, OwnedPIDs: []int{}}
}

func (s *Store) trustKernelCheck(ctx context.Context, raw []byte, token string) (any, error) {
	return s.trustKernelCheckUsing(ctx, raw, token, func(ctx context.Context, p TrustKernelCheckPolicy) (TrustKernelCheckResult, error) {
		return runTrustKernelCheckProcess(ctx, p, nil)
	})
}

func runTrustKernelCheckProcess(ctx context.Context, p TrustKernelCheckPolicy, argv []string) (result TrustKernelCheckResult, err error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	result = kernelCheckFixedResult()
	if err = validateTrustKernelPolicy(&p); err != nil {
		return result, err
	}
	if !bool(p.Enabled) {
		return result, trustKernelDenied()
	}
	ctx, cancel := context.WithTimeout(ctx, time.Duration(p.TimeoutSeconds)*time.Second)
	defer cancel()
	defer func() {
		if ctx.Err() != nil {
			result = kernelCheckFixedResult()
			err = trustKernelInterrupted()
		}
	}()
	root, err := os.MkdirTemp("", "swarm-kernel-check-")
	if err != nil {
		return result, err
	}
	defer func() {
		if e := os.RemoveAll(root); e != nil {
			result = kernelCheckFixedResult()
			err = fmt.Errorf("nettoyage du diagnostic noyau refusé : %w", e)
		}
	}()
	l, err := net.ListenUnix("unix", &net.UnixAddr{Name: filepath.Join(root, "control"), Net: "unix"})
	if err != nil {
		return result, err
	}
	defer l.Close()
	var nonce [16]byte
	if _, err = rand.Read(nonce[:]); err != nil {
		return result, err
	}
	stamp, err := kernelObservationProcessStart(os.Getpid())
	if err != nil {
		return result, err
	}
	b := kernelCheckBootstrap{kernelCheckProtocol, hex.EncodeToString(nonce[:]), l.Addr().String(), os.Getpid(), stamp, p}
	if argv == nil {
		exe, e := os.Executable()
		if e != nil {
			return result, e
		}
		argv = []string{exe, "_kernel_check"}
	}
	raw, _ := json.Marshal(b)
	cmd := exec.CommandContext(ctx, argv[0], argv[1:]...)
	cmd.Stdin = bytes.NewReader(raw)
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard
	cmd.Env = []string{"LANG=C", "TZ=UTC"}
	cmd.Dir = root
	image, err := pinTrustFixedHelperImage(cmd)
	if err != nil {
		return result, err
	}
	defer image.Close()
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true, Pdeathsig: syscall.SIGKILL}
	cmd.Cancel = func() error {
		if cmd.Process == nil {
			return os.ErrProcessDone
		}
		e := syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		if errors.Is(e, syscall.ESRCH) {
			return os.ErrProcessDone
		}
		return e
	}
	if err = cmd.Start(); err != nil {
		return result, err
	}
	defer func() {
		if cmd.ProcessState == nil {
			_ = cmd.Cancel()
			_ = cmd.Wait()
		}
	}()
	start, err := kernelObservationProcessStart(cmd.Process.Pid)
	if err != nil {
		return result, err
	}
	stopAccept := context.AfterFunc(ctx, func() { _ = l.Close() })
	defer stopAccept()
	if deadline, ok := ctx.Deadline(); ok {
		_ = l.SetDeadline(deadline)
	}
	conn, err := l.AcceptUnix()
	if err != nil {
		if ctx.Err() != nil {
			return result, trustKernelInterrupted()
		}
		return result, err
	}
	defer conn.Close()
	stopRead := context.AfterFunc(ctx, func() { _ = conn.Close() })
	defer stopRead()
	peer := kernelObservationPeer{PID: cmd.Process.Pid, UID: uint32(os.Getuid()), Start: start}
	if err = authenticateKernelObservationPeer(conn, peer); err != nil {
		return result, err
	}
	reader := bufio.NewReaderSize(conn, kernelObservationFrameBytes+1)
	var ready struct {
		Protocol string `json:"protocol"`
		Nonce    string `json:"nonce"`
	}
	if err = kernelCheckRead(reader, &ready); err != nil {
		return result, err
	}
	if ready.Protocol != b.Protocol || ready.Nonce != b.Nonce {
		return result, errors.New("kernel helper readiness mismatch")
	}
	if err = json.NewEncoder(conn).Encode(map[string]bool{"run": true}); err != nil {
		return result, err
	}
	if err = kernelCheckRead(reader, &result); err != nil {
		return result, err
	}
	if err = authenticateKernelObservationPeer(conn, peer); err != nil {
		return result, err
	}
	if result.SchemaVersion != 1 || result.EffectivePolicy != p || result.Source != "ebpf" || result.Scope != "owned_fixture_only" || result.GlobalQualification || result.Events == nil || len(result.OwnedPIDs) != 1 || result.OwnedPIDs[0] != cmd.Process.Pid {
		return result, errors.New("kernel helper scope mismatch")
	}
	lease := kernelCheckLease(cmd.Process.Pid, start, b.Nonce)
	stream := newKernelObservationStream(lease, b.Nonce, "swarm-minimal-v1")
	for _, event := range result.Events {
		if err = stream.accept(event, lease); err != nil {
			return kernelCheckFixedResult(), err
		}
	}
	if result.Lost < stream.snapshot().Lost || (!result.Available && (len(result.Events) != 0 || result.FixtureVerified)) || (result.FixtureVerified && (result.Lost != 0 || len(result.Events) == 0)) {
		return kernelCheckFixedResult(), errors.New("kernel helper verdict mismatch")
	}
	if err = json.NewEncoder(conn).Encode(map[string]bool{"done": true}); err != nil {
		return result, err
	}
	if err = cmd.Wait(); err != nil {
		return kernelCheckFixedResult(), err
	}
	if ctx.Err() != nil {
		return kernelCheckFixedResult(), trustKernelInterrupted()
	}
	if !result.CleanupVerified {
		return kernelCheckFixedResult(), errors.New("kernel fixture cleanup not verified")
	}
	return result, nil
}
