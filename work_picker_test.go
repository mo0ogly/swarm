//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestWorkPickerBoundsAndUsefulContext(t *testing.T) {
	works := []Work{{ID: "private-id-not-needed", Title: "Mon travail", Objective: strings.Repeat("Objectif long ", 30), Next: strings.Repeat("Action ", 30), Tasks: []Task{{Status: "blocked"}}}}
	for _, size := range [][2]int{{80, 24}, {64, 20}, {140, 40}, {10, 4}} {
		frame := renderWorkPicker(works, 0, size[0], size[1])
		rows := strings.Split(frame, "\r\n")
		if len(rows) > size[1] {
			t.Fatal("height overflow")
		}
		for _, r := range rows {
			if len([]rune(r)) > size[0] {
				t.Fatal("width overflow")
			}
		}
		if strings.Contains(frame, "private-id") {
			t.Fatal("unnecessary ID displayed")
		}
		if size[0] > 48 && !strings.Contains(frame, "1 bloquées") {
			t.Fatal("missing task state")
		}
	}
}
