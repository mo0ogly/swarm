//go:build linux

package main

import (
	"fmt"
	"golang.org/x/sys/unix"
	"io"
	"os"
	"strings"
	"time"
	"unicode/utf8"
)

func preparationIsTTY(in *os.File) bool {
	_, e := unix.IoctlGetTermios(int(in.Fd()), unix.TCGETS)
	return e == nil
}
func (s *Store) preparationEntry(args []string, input, output string, asJSON bool, out io.Writer) error {
	plain := false
	pos := []string{}
	for _, arg := range args {
		if arg == "--plain" {
			plain = true
		} else {
			pos = append(pos, arg)
		}
	}
	explicit := len(pos) > 0 && pos[0] == "chat"
	interactive := explicit || (!asJSON && input == "" && preparationIsTTY(os.Stdin) && (len(pos) == 0 || len(pos) == 2 && pos[0] == "resume"))
	if interactive {
		if asJSON || input != "" || !preparationIsTTY(os.Stdin) {
			return fmt.Errorf("Le dialogue interactif exige un terminal. Pour les scripts : prepare send/dialogue en JSON.")
		}
		if len(pos) > 2 {
			return fmt.Errorf("prepare chat [ID]")
		}
		id := ""
		if len(pos) == 2 {
			id = pos[1]
		}
		return s.preparationREPL(id, os.Stdin, out, plain)
	}
	return s.preparationCLI(pos, input, output, out)
}
func (s *Store) preparationREPL(id string, in *os.File, out io.Writer, plain bool) error {
	old, e := unix.IoctlGetTermios(int(in.Fd()), unix.TCGETS)
	if e != nil {
		return e
	}
	raw := *old
	raw.Lflag &^= unix.ICANON | unix.ECHO | unix.ISIG | unix.IEXTEN
	raw.Iflag &^= unix.ICRNL | unix.IXON
	raw.Cc[unix.VMIN] = 1
	raw.Cc[unix.VTIME] = 0
	if e = unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, &raw); e != nil {
		return e
	}
	defer unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, old)
	if !plain {
		fmt.Fprint(out, "\x1b[?2004h")
		defer fmt.Fprint(out, "\x1b[?2004l")
	}
	t := &preparationTerminal{s: s, out: out, width: 80, seen: map[string]string{}}
	size := func() {
		if ws, err := unix.IoctlGetWinsize(int(in.Fd()), unix.TIOCGWINSZ); err == nil && ws.Col > 0 {
			t.width = max(20, int(ws.Col)-1)
		}
	}
	size()
	t.editor = func(kind string) error {
		if !plain {
			fmt.Fprint(out, "\x1b[?2004l")
			defer fmt.Fprint(out, "\x1b[?2004h")
		}
		unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, old)
		defer unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, &raw)
		return t.editExternal(kind, in)
	}
	t.say("SWARM — PRÉPARER · /aide pour les commandes")
	if id != "" {
		if e = t.load(id); e != nil {
			return e
		}
		_ = t.history(true)
	} else {
		t.say("Décrivez votre besoin pour l’enregistrer, ou /sessions puis /ouvrir ID.")
	}
	t.prompt()
	var buffer, sequence []byte
	paste, literal, overflow := false, false, false
	lastPoll := time.Now()
	redraw := func() {
		if plain {
			fmt.Fprintln(out)
		} else {
			fmt.Fprint(out, "\r\x1b[2K")
		}
		t.prompt()
		fmt.Fprint(out, terminalText(string(buffer)))
	}
	for {
		fds := []unix.PollFd{{Fd: int32(in.Fd()), Events: unix.POLLIN}}
		n, err := unix.Poll(fds, 250)
		if err == unix.EINTR {
			continue
		}
		if err != nil {
			return err
		}
		size()
		if n == 0 {
			if len(buffer) == 0 && t.multi == "" && t.confirmation == nil && time.Since(lastPoll) > time.Second {
				lastPoll = time.Now()
				var log strings.Builder
				save := t.out
				t.out = &log
				err = t.history(false)
				t.out = save
				if err != nil {
					t.say(err.Error())
				}
				if log.Len() > 0 {
					fmt.Fprintln(out)
					fmt.Fprint(out, log.String())
					t.prompt()
				}
			}
			continue
		}
		b := []byte{0}
		read, err := in.Read(b)
		if err != nil {
			return nil
		}
		if read == 0 {
			return nil
		}
		c := b[0]
		if sequence != nil {
			sequence = append(sequence, c)
			done, key := decodeSequence(sequence, paste)
			if done {
				sequence = nil
				if key == "paste-start" {
					paste = true
					literal = true
				}
				if key == "paste-end" {
					paste = false
					redraw()
				}
			}
			continue
		}
		if c == 27 {
			sequence = []byte{}
			continue
		}
		if paste {
			if len(buffer) < 16000 {
				buffer = append(buffer, c)
			} else {
				overflow = true
			}
			continue
		}
		switch c {
		case 3:
			buffer = nil
			literal = false
			overflow = false
			t.multi = ""
			t.lines = nil
			t.confirmation = nil
			fmt.Fprintln(out)
			t.say("Saisie abandonnée.")
			if t.p.ID != "" {
				if e = t.stop(); e != nil {
					t.say(e.Error())
				}
			}
			t.prompt()
		case 4:
			if len(buffer) == 0 {
				fmt.Fprintln(out)
				t.say("Vue fermée ; échanges enregistrés conservés.")
				return nil
			}
		case '\r', '\n':
			fmt.Fprintln(out)
			if overflow {
				t.say("Saisie trop longue, aucun envoi. Utilisez /edit pour un document.")
			} else {
				quit, err := t.line(string(buffer), literal)
				if err != nil {
					t.say("Action refusée : " + err.Error())
				}
				if quit {
					return nil
				}
			}
			buffer = nil
			literal = false
			overflow = false
			t.prompt()
		case 127, 8:
			if len(buffer) > 0 {
				_, n := utf8.DecodeLastRune(buffer)
				buffer = buffer[:len(buffer)-n]
				redraw()
			}
		case 21:
			buffer = nil
			overflow = false
			redraw()
		default:
			if c >= 32 {
				if len(buffer) < 16000 {
					buffer = append(buffer, c)
					fmt.Fprint(out, string([]byte{c}))
				} else {
					overflow = true
				}
			}
		}
	}
}
