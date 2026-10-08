//go:build linux

package main

import "testing"

func TestValidationRecoveryRequiresObjectiveVerdict(t *testing.T) {
	a := Agent{Status: "completed", Attempt: "attempt"}
	task := Task{Status: "blocked", PlanMaxAttempts: 2, AutoValidation: &AutomaticValidation{
		Attempt: a.Attempt, State: "blocked", PolicyDigest: "policy", Receipt: "first",
		Controls: []ValidationControlResult{{ID: "check", Executed: true, ExitCode: 7, OutputHash: "output"}},
	}}
	_, cause, ok := automaticCorrection(task, a)
	if !ok {
		t.Fatal("objective failure cannot be corrected")
	}
	task.AutoValidation.Receipt = "second"
	a.Recovery.CauseFingerprint = cause
	if _, _, ok := automaticCorrection(task, a); ok {
		t.Fatal("new receipt hid unchanged failure")
	}
	a.Recovery.CauseFingerprint = ""
	for _, result := range []ValidationControlResult{
		{ID: "check", Executed: true, ExitCode: -1},
		{ID: "check", Executed: false, ExitCode: 1},
	} {
		task.AutoValidation.Controls = []ValidationControlResult{result}
		if _, _, ok := automaticCorrection(task, a); ok {
			t.Fatal("verification incident launched worker")
		}
	}
}
