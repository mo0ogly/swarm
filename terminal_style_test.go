//go:build linux

package main

import (
	"os"
	"regexp"
	"strings"
	"testing"
)

func TestTerminalStyleDoesNotChangeText(t *testing.T) {
	t.Setenv("TERM", "xterm-256color")
	// Force NO_COLOR absent only within this test; cleanup restores caller environment.
	t.Setenv("NO_COLOR", "")
	t.Setenv("SWARM_THEME", "dark")
	frame := "SWARM / VOS TRAVAUX\r\n> Travail sélectionné\r\n1 bloquées\r\n↑↓ choisir"
	if got := styleTerminalFrame(frame); got != frame {
		t.Fatal("NO_COLOR ignored")
	}
	os.Unsetenv("NO_COLOR")
	dark := styleTerminalFrame(frame)
	t.Setenv("SWARM_THEME", "light")
	light := styleTerminalFrame(frame)
	if dark == light || dark == frame {
		t.Fatal("theme or selection styling missing")
	}
	for _, styled := range []string{dark, light} {
		if ansiStylePattern.ReplaceAllString(styled, "") != frame {
			t.Fatal("style changed layout or labels")
		}
	}
	toggleTerminalTheme()
	if terminalLightTheme() {
		t.Fatal("theme toggle failed")
	}
	t.Setenv("TERM", "dumb")
	if styleTerminalFrame(frame) != frame {
		t.Fatal("dumb terminal styled")
	}

}
func TestReadableWrapPreservesWords(t *testing.T) {
	got := readableWrap("Un objectif clair et lisible", 16)
	if strings.Join(got, " ") != "Un objectif clair et lisible" {
		t.Fatal(got)
	}
	for _, line := range got {
		if len([]rune(line)) > 16 {
			t.Fatal("width")
		}
	}
}

var ansiStylePattern = regexp.MustCompile(`\x1b\[[0-9;]*m`)
