//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestPlainConsoleBilingualPreservesUserContent(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	for _, lang := range []string{"fr", "en"} {
		t.Setenv("SWARM_LANG", lang)
		state := &consoleState{}
		if _, err := s.consoleCommand(w.ID, "help", state); err != nil {
			t.Fatal(err)
		}
		got := s.renderConsole(w.ID, state, 300, 100, "")
		if !strings.Contains(got, w.Objective) {
			t.Fatal("user objective changed")
		}
		if lang == "en" {
			for _, expected := range []string{"OBJECTIVE:", "TASKS (9 = highest priority)", "No matching agents", "COMMANDS (Enter"} {
				if !strings.Contains(got, expected) {
					t.Fatalf("missing %q: %s", expected, got)
				}
			}
			if strings.Contains(got, "TÂCHES") {
				t.Fatal("French heading in English console")
			}
		} else if !strings.Contains(got, "OBJECTIF :") || !strings.Contains(got, "COMMANDES (Entrée") {
			t.Fatal("French console changed")
		}
	}
}
