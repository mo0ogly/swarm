//go:build linux

package main

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

// Opt-in real HTTP/CLI/browser journey against an isolated fixture only.
func TestManagedRecoveryBrowserRecipe(t *testing.T) {
	binary := os.Getenv("SWARM_RECOVERY_UI_BINARY")
	if binary == "" {
		t.Skip("set SWARM_RECOVERY_UI_BINARY and SWARM_RECOVERY_UI_OUT")
	}
	out := os.Getenv("SWARM_RECOVERY_UI_OUT")
	if out == "" {
		t.Fatal("output directory required")
	}
	s, w := managedFixture(t)
	partial := managedCompleted(t, s, w, "second", "partial fixture result\n")
	partial.DeliveryVersion = 1
	if e := s.saveAgent(partial); e != nil {
		t.Fatal(e)
	}
	if e := s.integrateManagedAttempt(partial); e != nil {
		t.Fatal(e)
	}
	if e := s.setMission(w.ID, false); e != nil {
		t.Fatal(e)
	}
	if e := s.setAutonomy(w.ID, autonomyManual, 2); e != nil {
		t.Fatal(e)
	}
	w, _ = s.get(w.ID)
	// A fixture provider is actually spawned; no external model or paid call.
	script := `#!/usr/bin/env python3
import os,sys,json
if '--version' in sys.argv:
 print('fixture 1.0');sys.exit(0)
sys.stdin.read()
with open(os.path.join(os.path.dirname(__file__),'worker-starts'),'a') as f:f.write('start\n')
print(json.dumps({'type':'result','result':'Fixture run finished without a delivery; not accepted.'}))
`
	if e := os.WriteFile(filepath.Join(s.root, "review-fixture/claude"), []byte(script), 0700); e != nil {
		t.Fatal(e)
	}
	r := Launch{Schema: 1, EventID: "prepared-browser", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Verify prepared launch recovery", Timeout: 60}
	if _, e := s.ensureManagedAttempt(w, r); e != nil {
		t.Fatal(e)
	}
	cmd := exec.Command("node", "tests/managed_recovery_ui.cjs", binary, out, s.root, w.ID)
	b, e := cmd.CombinedOutput()
	t.Log(string(b))
	if e != nil {
		t.Fatal(e)
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks[0].Attempts) != 1 || after.Tasks[0].Status == "accepted" || after.Planning.Reviewer.Calls != 0 {
		t.Fatal("unexpected mutation", after.Tasks[0])
	}
}
