//go:build linux

package main

import (
	"os"
	"os/exec"
	"testing"
)

func TestHistoricalRequalificationBrowser(t *testing.T) {
	binary := os.Getenv("SWARM_REQUALIFY_UI_BINARY")
	if binary == "" {
		t.Skip("explicit browser recipe")
	}
	s, w, _, _ := historicalFixture(t)
	cmd := exec.Command("node", "tests/requalification_ui.cjs", binary, s.root, w.ID, os.Getenv("SWARM_REQUALIFY_UI_OUTPUT"))
	if out, e := cmd.CombinedOutput(); e != nil {
		t.Fatalf("%v\n%s", e, out)
	}
}
func TestHistoricalRequalificationIdentityPreviewReadOnly(t *testing.T) {
	s, w, _, expected := historicalFixture(t)
	r, e := s.historicalRequalificationRequest(w.ID, "first")
	if e != nil {
		t.Fatal(e)
	}
	if r.Agent != expected.Agent || r.Attempt != expected.Attempt || r.ResultCommit != expected.ResultCommit || r.ExpectedCandidate != expected.ExpectedCandidate {
		t.Fatal("identity mismatch")
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision || after.Tasks[0].Status != "accepted" {
		t.Fatal("preview changed state")
	}
}
