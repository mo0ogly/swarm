//go:build linux

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	"golang.org/x/sys/unix"
)

func (s *Store) chooseConsoleWork(in *os.File, out io.Writer, asJSON bool, plain bool) (string, error) {
	works, e := s.list()
	if e != nil {
		return "", e
	}
	if asJSON {
		return "", printJSON(out, map[string]any{"works": works, "selection_required": true})
	}
	if len(works) == 0 {
		fmt.Fprintln(out, "Aucun travail. Créer un travail avec swarm work create --input fichier.json.")
		return "", nil
	}
	if _, err := unix.IoctlGetTermios(int(in.Fd()), unix.TCGETS); err == nil && !plain && os.Getenv("TERM") != "dumb" {
		return pickConsoleWork(works, in, out)
	}
	for i, w := range works {
		fmt.Fprintf(out, "%d. %s | %s\n", i+1, terminalText(w.ID), terminalText(w.Title))
	}
	if _, e = unix.IoctlGetTermios(int(in.Fd()), unix.TCGETS); e != nil {
		fmt.Fprintln(out, "Sans terminal interactif : swarm console IDENTIFIANT --json")
		return "", nil
	}
	for {
		fmt.Fprint(out, "Choisir un numéro ou un identifiant (q pour quitter) : ")
		// No buffered read-ahead: the console must receive subsequent keyboard input.
		var line strings.Builder
		one := make([]byte, 1)
		for {
			n, e := in.Read(one)
			if e != nil {
				return "", e
			}
			if n == 0 {
				return "", io.EOF
			}
			if one[0] == '\n' || one[0] == '\r' {
				break
			}
			if line.Len() < 200 {
				line.WriteByte(one[0])
			}
		}
		answer := strings.TrimSpace(line.String())
		if answer == "q" {
			return "", nil
		}
		if n, e := strconv.Atoi(answer); e == nil && n >= 1 && n <= len(works) {
			return works[n-1].ID, nil
		}
		for _, w := range works {
			if answer == w.ID {
				return w.ID, nil
			}
		}
		fmt.Fprintln(out, "Choix invalide. Utiliser un numéro ou l’identifiant affiché.")
	}
}
func (s *Store) plainConsole(work string, in *os.File, out io.Writer, asJSON bool) error {
	if work == "" {
		var e error
		work, e = s.chooseConsoleWork(in, out, asJSON, true)
		if e != nil || work == "" {
			return e
		}
	}
	if asJSON {
		snapshot, e := s.cockpitSnapshot(work)
		if e != nil {
			return e
		}
		return printJSON(out, snapshot)
	}
	if _, e := s.get(work); e != nil {
		return e
	}
	state := &consoleState{message: "Mode texte : Entrée actualise ; help affiche les commandes ; q quitte."}
	scanner := bufio.NewScanner(in)
	scanner.Buffer(make([]byte, 1024), 16000)
	for {
		fmt.Fprint(out, s.renderConsole(work, state, 120, 40, ""))
		if !scanner.Scan() {
			return scanner.Err()
		}
		state.message = ""
		quit, e := s.consoleCommand(work, scanner.Text(), state)
		if e != nil {
			state.message = "Erreur : " + e.Error()
		}
		if quit {
			return nil
		}
	}
}
func consoleDimensions(width, height int) (int, int) {
	if width <= 0 {
		width = 80
	}
	if height <= 0 {
		height = 24
	}
	return width, height
}
