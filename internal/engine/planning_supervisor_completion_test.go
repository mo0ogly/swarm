//go:build linux

package engine

import "testing"

func TestAcceptanceKeepsSupervisorScopeOpenAndEventsPending(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	s.conduct(a, "completed")
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	root, err := current.Planning.scope("root")
	if err != nil {
		t.Fatal(err)
	}
	if current.Tasks[0].Status != "accepted" {
		t.Fatal("valid candidate was not accepted", current.Tasks[0].Blocker)
	}
	if root.State == "closed" {
		t.Fatal("engine closed the objective without the supervisor's decision")
	}
	pending := false
	for _, event := range current.Planning.Inbox {
		if event.Scope == "root" && event.Decision == "" {
			pending = true
		}
	}
	if !pending {
		t.Fatal("engine acknowledged the supervisor's events implicitly")
	}
}
