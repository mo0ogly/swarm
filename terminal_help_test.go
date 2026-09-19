//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestHelpPreservesPendingFormAndRestoresParent(t *testing.T) {
	s := storeTest(t)
	parent := &taskDialog{mode: "start", instruction: "Consigne en cours ?", row: 2}
	c := &consoleState{dialog: parent}
	s.dialogKey("", c, "help")
	if c.dialog.mode != "help" {
		t.Fatal("F1 did not open help")
	}
	s.dialogKey("", c, "down")
	s.dialogKey("", c, "escape")
	if c.dialog != parent || c.dialog.instruction != "Consigne en cours ?" || c.dialog.row != 2 {
		t.Fatal("Help lost form or selection")
	}
	for _, seq := range []string{"OP", "[11~"} {
		_, key := decodeSequence([]byte(seq), false)
		if key != "help" {
			t.Fatalf("F1 %q decoded as %q", seq, key)
		}
	}
	for _, size := range [][2]int{{80, 24}, {100, 30}} {
		s.dialogKey("", c, "help")
		base := strings.Repeat(strings.Repeat(" ", size[0])+"\r\n", size[1])
		frame := renderTaskDialog(base, c.dialog, size[0], size[1])
		if !strings.Contains(frame, "F1 fermer l’aide") || !strings.Contains(frame, "└") {
			t.Fatal("Help footer clipped")
		}
		for _, line := range strings.Split(frame, "\r\n") {
			if textWidth(line) > size[0] {
				t.Fatal("Help overflow")
			}
		}
		s.dialogKey("", c, "enter")
	}
}
