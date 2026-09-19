//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"golang.org/x/sys/unix"
)

func openAgentPTY() (*os.File, *os.File, error) {
	fd, e := unix.Open("/dev/ptmx", unix.O_RDWR|unix.O_NOCTTY|unix.O_CLOEXEC|unix.O_NONBLOCK, 0)
	if e != nil {
		return nil, nil, e
	}
	master := os.NewFile(uintptr(fd), "swarm-pty")
	fail := func(e error) (*os.File, *os.File, error) { master.Close(); return nil, nil, e }
	if e = unix.IoctlSetPointerInt(fd, unix.TIOCSPTLCK, 0); e != nil {
		return fail(e)
	}
	n, e := unix.IoctlGetInt(fd, unix.TIOCGPTN)
	if e != nil {
		return fail(e)
	}
	slave, e := os.OpenFile(fmt.Sprintf("/dev/pts/%d", n), os.O_RDWR|syscall.O_NOCTTY, 0)
	if e != nil {
		return fail(e)
	}
	if e = unix.IoctlSetWinsize(fd, unix.TIOCSWINSZ, &unix.Winsize{Col: 96, Row: 28}); e != nil {
		slave.Close()
		return fail(e)
	}
	return master, slave, nil
}

type terminalControl struct {
	mu            sync.Mutex
	pty           *os.File
	client, lease string
	seq           int64
	lastData      []byte
	expires       time.Time
	stopped       bool
}

func (c *terminalControl) command(r terminalRequest) terminalReply {
	c.mu.Lock()
	defer c.mu.Unlock()
	reply := terminalReply{Seq: c.seq}
	fail := func(s string) terminalReply { reply.Error = s; return reply }
	if c.stopped {
		return fail("Terminal arrêté ; consultation seule.")
	}
	if !safeName(r.Client) || len(r.Client) > 80 {
		return fail("Identité de vue invalide.")
	}
	owned := c.client == r.Client && c.lease != "" && time.Now().Before(c.expires)
	if r.Kind == "claim" {
		if c.lease != "" && time.Now().Before(c.expires) && !owned {
			reply.Busy = true
			return reply
		}
		if !owned {
			c.client = r.Client
			c.lease = newID("lease-")
			c.seq = 0
			c.lastData = nil
		}
		c.expires = time.Now().Add(20 * time.Second)
		return terminalReply{Lease: c.lease, Seq: c.seq, Writable: true}
	}
	if !owned || r.Lease != c.lease {
		return fail("Saisie détenue par une autre vue ou expirée ; reprendre la saisie.")
	}
	c.expires = time.Now().Add(20 * time.Second)
	switch r.Kind {
	case "release":
		c.lease = ""
		c.client = ""
		return terminalReply{}
	case "input":
		if len(r.Data) == 0 || len(r.Data) > 4096 {
			return fail("Saisie limitée à 4096 octets.")
		}
		if r.Seq == c.seq && string(r.Data) == string(c.lastData) {
			return terminalReply{Lease: c.lease, Seq: c.seq, Writable: true}
		}
		if r.Seq != c.seq+1 {
			return fail("Ordre de saisie différent ; aucune retransmission automatique.")
		}
		// Consume before writing. Even a partial/uncertain write is never replayed.
		c.seq = r.Seq
		c.lastData = append([]byte{}, r.Data...)
		_ = c.pty.SetWriteDeadline(time.Now().Add(time.Second))
		n, e := c.pty.Write(r.Data)
		if e != nil || n != len(r.Data) {
			c.lease = ""
			return fail("Saisie partiellement reçue ou non confirmée ; vérifier le terminal avant de continuer.")
		}
	default:
		return fail("Commande de terminal inconnue.")
	}
	return terminalReply{Lease: c.lease, Seq: c.seq, Writable: true}
}
func terminalPeerAllowed(c *net.UnixConn) bool {
	raw, e := c.SyscallConn()
	if e != nil {
		return false
	}
	allowed := false
	e = raw.Control(func(fd uintptr) {
		cred, err := unix.GetsockoptUcred(int(fd), unix.SOL_SOCKET, unix.SO_PEERCRED)
		allowed = err == nil && cred.Uid == uint32(os.Getuid())
	})
	return e == nil && allowed
}
func (s *Store) superviseTerminal(a Agent) error {
	master, slave, e := openAgentPTY()
	if e != nil {
		return s.finishAgent(a, "failed", "Pseudo-terminal indisponible : "+e.Error(), nil)
	}
	defer master.Close()
	defer slave.Close()
	listener, e := net.ListenUnix("unix", &net.UnixAddr{Name: s.terminalSocket(a.ID), Net: "unix"})
	if e != nil {
		return s.finishAgent(a, "failed", "Canal terminal indisponible : "+e.Error(), nil)
	}
	defer listener.Close()
	if e = os.Chmod(s.terminalSocket(a.ID), 0600); e != nil {
		return s.finishAgent(a, "failed", e.Error(), nil)
	}
	control := &terminalControl{pty: master}
	defer func() { control.mu.Lock(); control.stopped = true; control.mu.Unlock() }()
	// One connection handled at a time, with a bounded read/write deadline.
	go func() {
		for {
			c, err := listener.AcceptUnix()
			if err != nil {
				return
			}
			func() {
				defer c.Close()
				_ = c.SetDeadline(time.Now().Add(2 * time.Second))
				if !terminalPeerAllowed(c) {
					return
				}
				var r terminalRequest
				if err = json.NewDecoder(io.LimitReader(c, 10000)).Decode(&r); err != nil {
					return
				}
				if r.Kind == "resize" {
					control.mu.Lock()
					defer control.mu.Unlock()
					reply := terminalReply{}
					if control.stopped || r.Client != control.client || r.Lease != control.lease || control.lease == "" || time.Now().After(control.expires) {
						reply.Error = "Reprendre la saisie avant de redimensionner."
					} else if r.Cols < 20 || r.Cols > 300 || r.Rows < 5 || r.Rows > 120 {
						reply.Error = "Dimensions du terminal hors limites."
					} else {
						_, err = s.db.Exec("INSERT INTO terminal_events(agent_id,data,cols,rows) VALUES(?,?,?,?)", a.ID, []byte{}, r.Cols, r.Rows)
						if err == nil {
							raw, rawErr := master.SyscallConn()
							err = rawErr
							if err == nil {
								err = raw.Control(func(fd uintptr) {
									rawErr = unix.IoctlSetWinsize(int(fd), unix.TIOCSWINSZ, &unix.Winsize{Col: uint16(r.Cols), Row: uint16(r.Rows)})
								})
								if err == nil {
									err = rawErr
								}
							}
						}
						if err != nil {
							reply.Error = "Redimensionnement non confirmé."
						}
					}
					_ = json.NewEncoder(c).Encode(reply)
					return
				}
				reply := control.command(r)
				_ = json.NewEncoder(c).Encode(reply)
			}()
		}
	}()
	signals := make(chan os.Signal, 2)
	signal.Notify(signals, syscall.SIGTERM, syscall.SIGINT)
	defer signal.Stop(signals)
	var monitor *dialogueMonitor
	var monitorDone chan struct{}
	var dialogueReader, dialogueWriter *os.File
	var authReader, authWriter *os.File
	cmd := exec.Command(a.Command, a.Args...)
	if a.Mode == "dialogue" {
		exe, err := os.Executable()
		if err != nil {
			return err
		}
		cmd = exec.Command(exe, "--root", s.root, "_dialogue_agent", a.ID)
		dialogueReader, dialogueWriter, e = os.Pipe()
		if e != nil {
			return e
		}
		defer dialogueReader.Close()
		defer dialogueWriter.Close()
		authReader, authWriter, e = os.Pipe()
		if e != nil {
			return e
		}
		defer authReader.Close()
		defer authWriter.Close()
		cmd.ExtraFiles = []*os.File{dialogueWriter, authReader}
		monitor = newDialogueMonitor(s, a)
		monitor.authorize = authWriter
		monitorDone = make(chan struct{})
		go func() { monitor.consume(dialogueReader); close(monitorDone) }()
	}
	cmd.Dir = a.CWD
	cmd.Env = providerEnvironment(a.Env)
	// Replace rather than duplicate TERM: native providers need terminal capabilities.
	for i := len(cmd.Env) - 1; i >= 0; i-- {
		if len(cmd.Env[i]) >= 5 && cmd.Env[i][:5] == "TERM=" {
			cmd.Env = append(cmd.Env[:i], cmd.Env[i+1:]...)
		}
	}
	cmd.Env = append(cmd.Env, "TERM=xterm-256color")
	cmd.Stdin = slave
	cmd.Stdout = slave
	cmd.Stderr = slave
	cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true, Setctty: true, Ctty: 0}
	if e = cmd.Start(); e != nil {
		return s.finishAgent(a, "failed", "Échec du terminal : "+e.Error(), nil)
	}
	_ = slave.Close()
	if dialogueWriter != nil {
		dialogueWriter.Close()
		authReader.Close()
	}
	a.Child = cmd.Process.Pid
	a.ChildStamp = processStamp(a.Child)
	a.Status = "running"
	a.Heartbeat = now()
	a.Activity = "Terminal ouvert ; progression métier non mesurée"
	a.Progress.Degraded = terminalMonitoring
	if monitor != nil {
		a.Progress.Degraded = ""
		a.Activity = "Dialogue contrôlé démarré"
	}
	if e = s.saveAgent(a); e != nil {
		_ = syscall.Kill(-a.Child, syscall.SIGKILL)
		_ = cmd.Wait()
		return e
	}
	_ = s.log(a.ID, "lifecycle", "Terminal interactif démarré. Fermer la vue ne l’arrête pas.")
	_ = s.log(a.ID, "monitoring", agentMonitoring(a))
	readDone := make(chan error, 1)
	outputStop := make(chan string, 1)
	go func() {
		total := 0
		buf := make([]byte, 4096)
		for {
			n, err := master.Read(buf)
			if n > 0 {
				if total+n > terminalOutputLimit {
					outputStop <- "Historique terminal atteint (4 Mio)"
					readDone <- nil
					return
				}
				_, saveErr := s.db.Exec("INSERT INTO terminal_events(agent_id,data) VALUES(?,?)", a.ID, append([]byte{}, buf[:n]...))
				if saveErr != nil {
					outputStop <- "Persistance du terminal indisponible"
					readDone <- saveErr
					return
				}
				total += n
			}
			if err != nil {
				readDone <- err
				return
			}
		}
	}()
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	ticker := time.NewTicker(time.Second)
	defer ticker.Stop()
	deadline := time.Now().Add(time.Duration(a.Timeout) * time.Second)
	var stopping time.Time
	reason := ""
	stop := func(message, cause string) {
		if !stopping.IsZero() {
			return
		}
		stopping = time.Now()
		reason = message
		a.StopKind = cause
		a.Status = "stopping"
		a.Activity = message + " ; arrêt en attente"
		control.mu.Lock()
		control.stopped = true
		control.mu.Unlock()
		_ = syscall.Kill(-a.Child, syscall.SIGTERM)
		_ = s.saveAgent(a)
	}
	for {
		select {
		case <-signals:
			stop("Interruption du superviseur", "superviseur")
		case message := <-outputStop:
			stop(message, "garde")
		case err := <-done:
			if monitor != nil {
				select {
				case <-monitorDone:
				case <-time.After(2 * time.Second):
					dialogueReader.Close()
				}
				var guardReason string
				a.Progress, a.Usage, guardReason, a.Activity = monitor.snapshot()
				if guardReason != "" {
					stop(guardReason, "garde")
				}
			}
			_ = syscall.Kill(-a.Child, syscall.SIGKILL)
			select {
			case <-readDone:
			case <-time.After(time.Second):
				_ = master.Close()
			}
			state := "completed"
			message := "Session terminée ; résultat à examiner, tâche non acceptée"
			if err != nil {
				state = "failed"
				message = "Session en échec ; examiner le terminal"
			}
			if !stopping.IsZero() {
				state = "interrupted"
				message = reason + " ; arrêt confirmé"
			}
			code := cmd.ProcessState.ExitCode()
			return s.finishAgent(a, state, message, &code)
		case <-ticker.C:
			if monitor != nil {
				var guardReason string
				a.Progress, a.Usage, guardReason, a.Activity = monitor.snapshot()
				if guardReason != "" {
					stop(guardReason, "garde")
				}
			}
			desired, err := s.desired(a.ID)
			if err != nil {
				stop("Stockage indisponible", "superviseur")
			}
			if desired == "stop" {
				stop("Arrêt demandé par opérateur", "operateur")
			}
			if time.Now().After(deadline) {
				stop("Durée maximale atteinte", "delai")
			}
			if !stopping.IsZero() && time.Since(stopping) > 3*time.Second {
				_ = syscall.Kill(-a.Child, syscall.SIGKILL)
			}
			a.Heartbeat = now()
			if err = s.saveAgent(a); err != nil {
				stop("Persistance indisponible", "superviseur")
			}
		}
	}
}
