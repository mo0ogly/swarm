package main

import (
	"bytes"
	"strings"
	"testing"
)

func TestHelpTopicsReadOnlyAndContextReturn(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	before, _ := s.get(w.ID)
	parent := &taskDialog{mode: "budget", row: 2}
	c := &consoleState{dialog: parent}
	s.openTerminalHelp(w.ID, c)
	if !strings.HasPrefix(c.dialog.review, "COMPRENDRE LE BUDGET") {
		t.Fatal(c.dialog.review)
	}
	s.openTerminalHelp(w.ID, c)
	if c.dialog != parent || c.dialog.row != 2 {
		t.Fatal("parent state lost")
	}
	for _, topic := range []string{"mission", "parite", "taches"} {
		if _, e := s.consoleCommand(w.ID, "help "+topic, c); e != nil {
			t.Fatal(e)
		}
		want, _ := cliTopicHelp(topic)
		if c.dialog.review != want {
			t.Fatal("wrong topic")
		}
	}
	after, _ := s.get(w.ID)
	if after.Revision != before.Revision {
		t.Fatal("help mutated work")
	}
	if _, e := cliTopicHelp("inconnu"); e == nil {
		t.Fatal("unknown help silently accepted")
	}
}

func TestHelpWithoutProject(t *testing.T) {
	var out, err bytes.Buffer
	if code := run([]string{"--root", "/absent-project-help-test", "aide", "mission"}, &out, &err); code != 0 {
		t.Fatalf("%d %s", code, err.String())
	}
	if !strings.Contains(out.String(), "mission watch") {
		t.Fatal(out.String())
	}
}
