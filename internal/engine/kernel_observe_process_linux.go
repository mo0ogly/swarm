//go:build linux

package engine

import (
	"bytes"
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"syscall"
	"time"
)

type kernelObserveProcess struct {
	cmd      *exec.Cmd
	listener *net.UnixListener
	conn     *net.UnixConn
	root     string
	peer     kernelObservationPeer
	nonce    string
}

func (p *kernelObserveProcess) close() error {
	var err error
	if p.conn != nil {
		if e := p.conn.Close(); !errors.Is(e, net.ErrClosed) {
			err = errors.Join(err, e)
		}
	}
	if p.listener != nil {
		if e := p.listener.Close(); !errors.Is(e, net.ErrClosed) {
			err = errors.Join(err, e)
		}
	}
	if p.cmd != nil && p.cmd.Process != nil && p.cmd.ProcessState == nil {
		if e := p.cmd.Cancel(); !errors.Is(e, os.ErrProcessDone) {
			err = errors.Join(err, e)
		}
		_ = p.cmd.Wait()
	}
	if p.root != "" {
		err = errors.Join(err, os.RemoveAll(p.root))
	}
	return err
}
func startKernelObservedHelper(ctx context.Context, policy TrustKernelObservePolicy, l kernelObservationLease, args []string) (p *kernelObserveProcess, err error) {
	p = &kernelObserveProcess{}
	defer func() {
		if err != nil {
			err = errors.Join(err, p.close())
		}
	}()
	p.root, err = os.MkdirTemp("", "swarm-kernel-observer-")
	if err != nil {
		return p, err
	}
	p.listener, err = net.ListenUnix("unix", &net.UnixAddr{Name: filepath.Join(p.root, "control"), Net: "unix"})
	if err != nil {
		return p, err
	}
	var nonce [16]byte
	if _, err = rand.Read(nonce[:]); err != nil {
		return p, err
	}
	p.nonce = hex.EncodeToString(nonce[:])
	stamp, err := kernelObservationProcessStart(os.Getpid())
	if err != nil {
		return p, err
	}
	bootstrap := kernelObserveBootstrap{kernelObserveProtocol, p.nonce, p.listener.Addr().String(), os.Getpid(), stamp, policy, l}
	raw, err := json.Marshal(bootstrap)
	if err != nil {
		return p, err
	}
	exe, err := os.Open("/proc/self/exe")
	if err != nil {
		return p, err
	}
	defer exe.Close()
	if args == nil {
		args = []string{"_kernel_observe"}
	}
	p.cmd = exec.CommandContext(ctx, "/proc/self/fd/3", args...)
	p.cmd.ExtraFiles = []*os.File{exe}
	p.cmd.Env = []string{"LANG=C", "TZ=UTC"}
	p.cmd.Dir = p.root
	p.cmd.Stdin = bytes.NewReader(raw)
	p.cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true, Pdeathsig: syscall.SIGKILL}
	p.cmd.Cancel = func() error {
		if p.cmd.Process == nil {
			return os.ErrProcessDone
		}
		e := syscall.Kill(-p.cmd.Process.Pid, syscall.SIGKILL)
		if errors.Is(e, syscall.ESRCH) {
			return os.ErrProcessDone
		}
		return e
	}
	if err = p.cmd.Start(); err != nil {
		return p, err
	}
	start, err := kernelObservationProcessStart(p.cmd.Process.Pid)
	if err != nil {
		return p, err
	}
	p.peer = kernelObservationPeer{p.cmd.Process.Pid, uint32(os.Getuid()), start}
	stop := context.AfterFunc(ctx, func() { _ = p.listener.Close() })
	defer stop()
	if deadline, ok := ctx.Deadline(); ok {
		_ = p.listener.SetDeadline(deadline)
	}
	p.conn, err = p.listener.AcceptUnix()
	if err != nil {
		return p, err
	}
	if err = authenticateKernelObservationPeer(p.conn, p.peer); err != nil {
		return p, err
	}
	return p, nil
}

func (s *Store) runKernelObserveProcess(ctx context.Context, policy TrustKernelObservePolicy, l kernelObservationLease, fresh func() error, args []string) (result TrustKernelObserveResult, err error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	result = kernelObserveFixed(policy, l)
	ctx, cancel := context.WithTimeout(ctx, time.Duration(policy.TimeoutSeconds)*time.Second)
	defer cancel()
	p, err := startKernelObservedHelper(ctx, policy, l, args)
	if err != nil {
		return result, err
	}
	defer func() {
		err = errors.Join(err, p.close())
		if ctx.Err() != nil {
			err = trustKernelInterrupted()
		}
	}()
	stop := context.AfterFunc(ctx, func() { _ = p.conn.Close() })
	defer stop()
	receiver, err := s.newKernelAttemptReceiver(p.conn, p.peer, l.AgentID, p.nonce)
	if err != nil {
		return result, err
	}
	if receiver.stream.lease != l {
		return result, fmt.Errorf("source d’observation modifiée avant départ")
	}
	ready, authorized := false, false
	events := []kernelObservation{}
	for {
		if err = authenticateKernelObservationPeer(p.conn, p.peer); err != nil {
			return result, err
		}
		var packet kernelObservePacket
		if err = kernelCheckRead(receiver.reader, &packet); err != nil {
			return result, err
		}
		if err = authenticateKernelObservationPeer(p.conn, p.peer); err != nil {
			return result, err
		}
		if packet.Type != "ready" && (packet.Protocol != "" || packet.Nonce != "") {
			return result, fmt.Errorf("contrôle d’observation inattendu")
		}
		switch packet.Type {
		case "ready":
			if ready || packet.Protocol != kernelObserveProtocol || packet.Nonce != p.nonce || len(packet.Event) != 0 || len(packet.Result) != 0 {
				return result, fmt.Errorf("identité du collecteur invalide")
			}
			ready = true
		case "authorize":
			if !ready || len(packet.Event) != 0 || len(packet.Result) != 0 {
				return result, fmt.Errorf("autorisation de collecteur invalide")
			}
			if err = fresh(); err != nil {
				return result, err
			}
			if err = json.NewEncoder(p.conn).Encode(map[string]bool{"authorized": true}); err != nil {
				return result, err
			}
			authorized = true
		case "event":
			if !ready || !authorized || len(packet.Result) != 0 || len(events) >= policy.MaxEvents {
				return result, fmt.Errorf("événement collecteur inattendu")
			}
			event, e := decodeKernelObservation(packet.Event)
			if e != nil {
				return result, e
			}
			if err = fresh(); err != nil {
				return result, err
			}
			current, e := receiver.fresh()
			if e != nil {
				return result, e
			}
			if err = receiver.stream.accept(event, current); err != nil {
				return result, err
			}
			events = append(events, event)
		case "result":
			if !ready || len(packet.Event) != 0 {
				return result, fmt.Errorf("résultat collecteur inattendu")
			}
			if err = strict(packet.Result, &result); err != nil {
				return result, err
			}
			if len(result.Events) != 0 || result.Events == nil {
				return result, fmt.Errorf("événements collecteur dupliqués")
			}
			if result.Available && !authorized {
				return result, fmt.Errorf("collecteur non autorisé")
			}
			if err = fresh(); err != nil {
				return result, err
			}
			result.Events = events
			if result.StopReason == "collector_failed" {
				return result, fmt.Errorf("collecteur noyau : %s", result.Diagnostic)
			}
			if err = validateKernelObserveResult(result, policy, l); err != nil {
				return result, err
			}
			if err = json.NewEncoder(p.conn).Encode(map[string]bool{"done": true}); err != nil {
				return result, err
			}
			if err = p.cmd.Wait(); err != nil {
				return result, err
			}
			return result, nil
		default:
			return result, fmt.Errorf("message collecteur inconnu")
		}
	}
}
func (s *Store) trustKernelObserve(ctx context.Context, raw []byte, token string) (any, error) {
	return s.trustKernelObserveUsing(ctx, raw, token, func(ctx context.Context, p TrustKernelObservePolicy, l kernelObservationLease, fresh func() error) (TrustKernelObserveResult, error) {
		return s.runKernelObserveProcess(ctx, p, l, fresh, nil)
	})
}
