//go:build linux

package main

import (
	"github.com/rivo/uniseg"
	"os"
	"strings"
)

// Cell width, not rune count: wide East Asian and emoji runes occupy two
// terminal cells, combining marks occupy none. Used by every clipping,
// padding and wrapping helper so columns stay aligned for non-ASCII data.
func runeCellWidth(r rune) int { return uniseg.StringWidth(string(r)) }
func textWidth(s string) int   { return uniseg.StringWidth(s) }
func truncateCells(s string, width int) string {
	if width < 1 {
		return ""
	}
	g := uniseg.NewGraphemes(s)
	end, cells := 0, 0
	for g.Next() {
		if cells+g.Width() > width {
			break
		}
		cells += g.Width()
		_, end = g.Positions()
	}
	return s[:end]
}
func firstCluster(s string) string {
	g := uniseg.NewGraphemes(s)
	if g.Next() {
		return g.Str()
	}
	return ""
}

func padCells(s string, n int) string {
	s = truncateCells(s, n)
	return s + strings.Repeat(" ", max(0, n-textWidth(s)))
}

// Word wrapping keeps ordinary words intact; unbroken paths still have a bounded fallback.
func styleTerminalFrame(frame string) string {
	if _, off := os.LookupEnv("NO_COLOR"); off || os.Getenv("TERM") == "dumb" {
		return frame
	}
	light := terminalLightTheme()
	accent, muted, warning, success, selected := "38;5;117", "38;5;247", "38;5;221", "38;5;114", "48;5;24;38;5;231;1"
	if light {
		accent, muted, warning, success, selected = "38;5;24", "38;5;240", "38;5;130", "38;5;28", "48;5;153;38;5;17;1"
	}
	paint := func(s, code string) string { return "\x1b[" + code + "m" + s + "\x1b[0m" }
	style := func(line string) string {
		t := strings.TrimSpace(line)
		switch {
		case strings.HasPrefix(t, ">──"):
			return paint(line, "1;"+accent)
		case strings.HasPrefix(t, ">") || strings.HasPrefix(t, "│ >"):
			return paint(line, selected)
		case strings.HasPrefix(t, "SWARM") || strings.HasPrefix(t, "┌"):
			return paint(line, "1;"+accent)
		case strings.Contains(t, "──") || strings.HasPrefix(t, "└"):
			return paint(line, muted)
		case strings.Contains(t, "Erreur") || (strings.Contains(strings.ToLower(t), "bloquée") && !strings.Contains(t, "0 bloquées")) || strings.Contains(t, "Mise à jour"):
			return paint(line, warning)
		case strings.Contains(t, "acceptée") || strings.Contains(t, "Demande enregistrée"):
			return paint(line, success)
		case strings.HasPrefix(t, "↑") || strings.HasPrefix(t, "Tab ") || strings.HasPrefix(t, "Travail "):
			return paint(line, muted)
		default:
			return line
		}
	}
	lines := strings.Split(frame, "\r\n")
	for i, line := range lines {
		if parts := strings.SplitN(line, " │ ", 2); len(parts) == 2 && !strings.HasPrefix(strings.TrimSpace(line), "│") {
			lines[i] = style(parts[0]) + paint(" │ ", muted) + style(parts[1])
		} else {
			lines[i] = style(line)
		}
	}
	return strings.Join(lines, "\r\n")
}
func uiStatus(status string) string {
	if strings.HasPrefix(status, "unknown/") {
		return "À vérifier"
	}
	switch status {
	case "todo":
		return "À faire"
	case "running":
		return "En cours"
	case "blocked":
		return "Bloquée"
	case "submitted":
		return "À vérifier"
	case "waived":
		return "Dérogation"
	case "accepted":
		return "Acceptée"
	case "completed":
		return "Terminé"
	case "interrupted":
		return "Interrompu"
	case "failed":
		return "Échec"
	case "stopping":
		return "Arrêt…"
	case "queued":
		return "En attente"
	case "starting":
		return "Démarrage"
	}
	return status
}

// Word wrapping keeps ordinary words intact; unbroken paths still have a bounded fallback.
func readableWrap(text string, width int) []string {
	width = max(1, width)
	rows := []string{}
	current := ""
	for _, word := range strings.Fields(terminalText(text)) {
		if current != "" && textWidth(current)+1+textWidth(word) <= width {
			current += " " + word
			continue
		}
		if current != "" {
			rows = append(rows, current)
			current = ""
		}
		for textWidth(word) > width {
			fit := truncateCells(word, width)
			if fit == "" {
				rows = append(rows, "…")
				word = word[len(firstCluster(word)):]
				continue
			}
			rows = append(rows, fit)
			word = word[len(fit):]
		}
		current = word
	}
	if current != "" {
		rows = append(rows, current)
	}
	return rows
}

func terminalLightTheme() bool {
	light := os.Getenv("SWARM_THEME") == "light"
	if os.Getenv("SWARM_THEME") == "" {
		parts := strings.Split(os.Getenv("COLORFGBG"), ";")
		if len(parts) > 1 {
			switch parts[len(parts)-1] {
			case "7", "15":
				light = true
			}
		}
	}
	return light
}
func toggleTerminalTheme() {
	if terminalLightTheme() {
		_ = os.Setenv("SWARM_THEME", "dark")
	} else {
		_ = os.Setenv("SWARM_THEME", "light")
	}
}
