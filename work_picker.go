//go:build linux

package main

import (
	"fmt"
	"io"
	"os"
	"os/signal"
	"strings"
	"syscall"

	"golang.org/x/sys/unix"
)

func renderWorkPicker(works []Work, selected, width, height int) string {
	width, height = consoleDimensions(width, height)
	width = max(1, width-1)
	if len(works) == 0 {
		return "Aucun travail disponible."
	}
	if height < 14 || width < 48 {
		return clip("Agrandir le terminal ; Entrée ouvre le travail, q quitte.", width)
	}
	selected = min(max(0, selected), len(works)-1)
	lines := []string{"SWARM  /  VOS TRAVAUX", "Choisissez un travail avec les flèches, puis Entrée.", ""}
	count := min(len(works), max(2, (height-11)/2))
	start := max(0, selected-count+1)
	for i := start; i < min(len(works), start+count); i++ {
		prefix := "  "
		if i == selected {
			prefix = "> "
		}
		lines = append(lines, pad(prefix+works[i].Title, width))
	}
	w := works[selected]
	running, blocked, accepted, waived := 0, 0, 0, 0
	for _, t := range w.Tasks {
		switch t.Status {
		case "running":
			running++
		case "blocked":
			blocked++
		case "waived":
			waived++
		case "accepted":
			accepted++
		}
	}
	lines = append(lines, "", strings.Repeat("─", width), fmt.Sprintf("%d tâches · %d en cours · %d bloquées · %d acceptées · %d dérogations", len(w.Tasks), running, blocked, accepted, waived))
	if running == 0 {
		lines = append(lines, "AUCUNE TÂCHE EN COURS — ouvrir un travail ne lance pas d’agent.")
	}
	for _, t := range w.Tasks {
		if t.Status == "blocked" {
			reason := t.Blocker
			if reason == "" {
				reason = "motif non renseigné"
			}
			rows := readableWrap("BLOCAGE "+t.ID+" : "+reason, width)
			if len(rows) > 2 {
				rows = rows[:2]
			}
			lines = append(lines, rows...)
			lines = append(lines, "Entrée : ouvrir le travail, sélectionner la tâche puis Entrée : actions.")
			break
		}
	}
	for _, entry := range []struct{ label, value string }{{"Objectif : ", w.Objective}, {"Prochaine action : ", w.Next}} {
		rows := readableWrap(entry.label+entry.value, width)
		if len(rows) > 2 {
			rows = rows[:2]
			rows[1] = clip(rows[1], max(1, width-1)) + "…"
		}
		lines = append(lines, rows...)
	}
	if len(lines) > height-2 {
		lines = lines[:height-2]
	}
	for len(lines) < height-2 {
		lines = append(lines, "")
	}
	lines = append(lines, fmt.Sprintf("Travail %d sur %d", selected+1, len(works)), "↑↓ choisir   Entrée ouvrir   t thème   q quitter")
	for i := range lines {
		lines[i] = clip(lines[i], width)
	}
	return strings.Join(lines, "\r\n")
}

// Read one byte at a time without read-ahead: the selected console owns all subsequent input.
func pickConsoleWork(works []Work, in *os.File, out io.Writer) (string, error) {
	old, e := unix.IoctlGetTermios(int(in.Fd()), unix.TCGETS)
	if e != nil {
		return "", e
	}
	raw := *old
	raw.Lflag &^= unix.ICANON | unix.ECHO | unix.ISIG | unix.IEXTEN
	raw.Iflag &^= unix.ICRNL | unix.IXON
	raw.Cc[unix.VMIN] = 1
	raw.Cc[unix.VTIME] = 0
	if e = unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, &raw); e != nil {
		return "", e
	}
	defer unix.IoctlSetTermios(int(in.Fd()), unix.TCSETS, old)
	fmt.Fprint(out, "\x1b[?1049h\x1b[?25l")
	defer fmt.Fprint(out, "\x1b[?25h\x1b[?1049l")
	sig := make(chan os.Signal, 2)
	signal.Notify(sig, syscall.SIGTERM, syscall.SIGHUP, syscall.SIGWINCH)
	defer signal.Stop(sig)
	selected := 0
	draw := func() {
		width, height := 80, 24
		if size, e := unix.IoctlGetWinsize(int(in.Fd()), unix.TIOCGWINSZ); e == nil {
			width, height = int(size.Col), int(size.Row)
		}
		fmt.Fprint(out, "\x1b[H\x1b[2J", styleTerminalFrame(renderWorkPicker(works, selected, width, height)))
	}
	draw()
	escape := false
	for {
		select {
		case event := <-sig:
			if event == syscall.SIGWINCH {
				draw()
			} else {
				return "", nil
			}
		default:
		}
		fds := []unix.PollFd{{Fd: int32(in.Fd()), Events: unix.POLLIN}}
		n, e := unix.Poll(fds, 100)
		if e == syscall.EINTR {
			continue
		}
		if e != nil {
			return "", e
		}
		if n == 0 {
			continue
		}
		if fds[0].Revents&(unix.POLLHUP|unix.POLLERR|unix.POLLNVAL) != 0 {
			return "", nil
		}
		var one [1]byte
		n, e = in.Read(one[:])
		if e != nil {
			return "", e
		}
		if n == 0 {
			return "", nil
		}
		key := one[0]
		if key == 3 || key == 4 || key == 'q' {
			return "", nil
		}
		if key == 27 {
			escape = true
			continue
		}
		if escape {
			if key == '[' || key == 'O' {
				continue
			}
			escape = false
			if key == 'A' {
				selected = (selected + len(works) - 1) % len(works)
			}
			if key == 'B' {
				selected = (selected + 1) % len(works)
			}
			draw()
			continue
		}
		switch key {
		case 't':
			toggleTerminalTheme()
		case '\r', '\n':
			return works[selected].ID, nil
		case 'k':
			selected = (selected + len(works) - 1) % len(works)
		case 'j', '\t':
			selected = (selected + 1) % len(works)
		}
		draw()
	}
}
