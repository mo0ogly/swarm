//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestManagedFragmentNativeDecisionDiagnostic(t *testing.T) {
	manifest := os.Getenv("SWARM_NATIVE_REVIEW_DIAGNOSTIC")
	if manifest == "" {
		t.Skip("explicit immutable evidence manifest required")
	}
	var paths struct{ Context, Plan, Journal, FinalJournal, Reply string }
	raw, err := os.ReadFile(manifest)
	if err != nil {
		t.Fatal(err)
	}
	if err = json.Unmarshal(raw, &paths); err != nil {
		t.Fatal(err)
	}
	read := func(path string, value any) {
		t.Helper()
		raw, err := os.ReadFile(filepath.Clean(path))
		if err != nil {
			t.Fatal(err)
		}
		if err = strict(raw, value); err != nil {
			t.Fatal(err)
		}
	}
	var c managedReviewContext
	var p managedReviewFragmentPlan
	var j managedFragmentJournal
	var f managedFragmentFinalJournal
	read(paths.Context, &c)
	read(paths.Plan, &p)
	read(paths.Journal, &j)
	read(paths.FinalJournal, &f)
	replies := make([]string, len(p.Packets))
	for _, e := range j.Entries {
		if e.State != "interrupted" && e.State != "reserved" {
			replies[e.Packet] = e.Reply
		}
	}
	var selection managedFragmentFinalRequest
	if err = strict([]byte(f.Calls[0].Reply), &selection); err != nil {
		t.Fatal(err)
	}
	started := time.Now()
	_, visible, err := managedFragmentDecisionPrompt("", c, p, replies, selection.References)
	t.Log("render decision", time.Since(started))
	if err != nil {
		t.Fatal(err)
	}
	visibleRaw, err := json.Marshal(visible)
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(filepath.Dir(manifest), "native-visible-context.json"), visibleRaw, 0600); err != nil {
		t.Fatal(err)
	}
	raw, err = os.ReadFile(paths.Reply)
	if err != nil {
		t.Fatal(err)
	}
	started = time.Now()
	state, records, err := parseManagedFragmentDecision(string(raw), visible, c, p, replies)
	t.Log("parse decision", time.Since(started), "state", state, "tasks", len(records))
	if err != nil {
		t.Fatal(err)
	}
	if state == "" || len(records) != len(c.Tasks) {
		t.Fatal("incomplete decision")
	}
}
