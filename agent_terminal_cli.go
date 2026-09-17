//go:build linux

package main

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"
	"time"

	"golang.org/x/sys/unix"
)

// Attach never starts a provider or a new attempt. Ctrl+] only detaches the view.
func (s *Store) attachTerminal(id string, in *os.File, out io.Writer) error {
	a, e := s.agent(id)
	if e != nil {
		return e
	}
	if !interactiveMode(a.Mode) {
		return fmt.Errorf("Cette tentative n’est pas interactive. Utiliser agent logs pour ses journaux.")
	}
	old, e := unix.IoctlGetTermios(int(in.Fd()), unix.TCGETS)
	if e != nil {
		return fmt.Errorf("agent attach exige un terminal")
	}
	fmt.Fprintln(out, "Session "+a.ID+" — Ctrl+] ferme la vue ; Ctrl+C est transmis à l’agent.")
	request := terminalRequest{Client: newID("cli-"), Kind: "claim"}
	var lease terminalReply
	if activeAgent(a) {
		lease, e = s.terminalRPC(a, request)
		if e != nil {
			return e
		}
		if !lease.Writable {
			return fmt.Errorf("Une autre vue détient la saisie. Fermer cette vue ou attendre l’expiration de sa connexion.")
		}
		request.Lease = lease.Lease
		request.Seq = lease.Seq
		defer func() { request.Kind = "release"; _, _ = s.terminalRPC(a, request) }()
	}
	raw := *old
	raw.Lflag &^= unix.ICANON | unix.ECHO | unix.ISIG | unix.IEXTEN
	raw.Iflag &^= unix.ICRNL | unix.IXON | unix.BRKINT | unix.INPCK | unix.ISTRIP
	raw.Oflag &^= unix.OPOST
	raw.Cflag |= unix.CS8
	raw.Cc[unix.VMIN] = 1
	raw.Cc[unix.VTIME] = 0
	if e = unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, &raw); e != nil {
		return e
	}
	defer unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, old)
	defer fmt.Fprint(out, "\x1b[0m\x1b[?1049l\x1b[?25h\x1b[?2004l\r\nVue fermée.\r\n")
	signals := make(chan os.Signal, 2)
	signal.Notify(signals, syscall.SIGTERM, syscall.SIGHUP)
	defer signal.Stop(signals)
	var cursor int64
	lastCheck, lastLease := time.Time{}, time.Now()
	cols, rows := 0, 0
	buf := make([]byte, 4096)
	for {
		select {
		case <-signals:
			return nil
		default:
		}
		events, err := s.terminalEvents(id, cursor)
		if err != nil {
			return err
		}
		for _, v := range events {
			if len(v.Data) > 0 {
				if _, err = out.Write(v.Data); err != nil {
					return err
				}
			}
			cursor = v.Seq
		}
		if len(events) == 16 {
			continue
		}
		if time.Since(lastCheck) > time.Second {
			lastCheck = time.Now()
			a, err = s.agent(id)
			if err != nil {
				return err
			}
			if !activeAgent(a) {
				return nil
			}
			if size, err := unix.IoctlGetWinsize(int(in.Fd()), unix.TIOCGWINSZ); err == nil && size.Col >= 20 && size.Row >= 5 && (int(size.Col) != cols || int(size.Row) != rows) {
				cols = min(300, int(size.Col))
				rows = min(120, int(size.Row))
				request.Kind = "resize"
				request.Cols = cols
				request.Rows = rows
				if _, err = s.terminalRPC(a, request); err != nil {
					return err
				}
			}
		}
		if time.Since(lastLease) > 5*time.Second {
			request.Kind = "claim"
			renewed, err := s.terminalRPC(a, request)
			if err != nil {
				return err
			}
			if renewed.Lease != request.Lease {
				return fmt.Errorf("La saisie a expiré ; rattacher la session")
			}
			lastLease = time.Now()
		}
		fds := []unix.PollFd{{Fd: int32(in.Fd()), Events: unix.POLLIN}}
		n, err := unix.Poll(fds, 100)
		if err == unix.EINTR {
			continue
		}
		if err != nil {
			return err
		}
		if n == 0 {
			continue
		}
		if fds[0].Revents&(unix.POLLHUP|unix.POLLERR) != 0 {
			return nil
		}
		n, err = in.Read(buf)
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
		if bytes.IndexByte(buf[:n], 0x1d) >= 0 {
			return nil
		}
		if n > 0 {
			request.Kind = "input"
			request.Seq++
			request.Data = buf[:n]
			if _, err = s.terminalRPC(a, request); err != nil {
				return fmt.Errorf("Saisie non confirmée, aucune retransmission automatique : %w", err)
			}
		}
	}
}
