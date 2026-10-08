//go:build linux

package main

import (
	"encoding/json"
	"os"
	"os/exec"
	"testing"
)

func TestRecoveryHealthBrowserRecipe(t *testing.T) {
	binary, out := os.Getenv("SWARM_HEALTH_UI_BINARY"), os.Getenv("SWARM_HEALTH_UI_OUT")
	if binary == "" {
		t.Skip("set SWARM_HEALTH_UI_BINARY and SWARM_HEALTH_UI_OUT")
	}
	if out == "" {
		t.Fatal("output directory required")
	}
	s, w := exhaustedTaskFixture(t)
	w.Tasks[0].PlanMaxAttempts = 3
	w.Tasks[0].Attempts = append(w.Tasks[0].Attempts, Attempt{ID: "old-3", Status: "completed"})
	w.Tasks[0].IndependentReview.Attempt = "old-3"
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, false); err != nil {
		t.Fatal(err)
	}
	if err := s.setAutonomy(w.ID, autonomyManual, 2); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command("node", "tests/runtime_health_ui.cjs", binary, out, s.root, w.ID)
	b, err := cmd.CombinedOutput()
	t.Log(string(b))
	if err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks[0].Attempts) != 3 || after.Tasks[0].Status != "blocked" || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls {
		t.Fatal("read-only diagnosis changed mission")
	}
}
