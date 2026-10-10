//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestPreparationTimeoutAdminFreezesNextExchange(t *testing.T) {
	s, _, r := prepDialogueFixture(t, prepReplyScript)
	before := s.preparationCapabilities()[0]
	policy, err := s.preparationTimeoutConfig()
	if err != nil {
		t.Fatal(err)
	}
	change := PreparationTimeoutChange{Schema: 1, EventID: "long-preparation", Revision: policy.Revision, Values: PreparationTimeoutValues{Timeout: 1800, Maximum: 2400}, Reason: "Préparation complexe autorisée"}
	preview, err := s.changePreparationTimeout(change, true)
	if err != nil || preview.Values.Timeout != 1800 {
		t.Fatal(preview, err)
	}
	unchanged, _ := s.preparationTimeoutConfig()
	if unchanged.Revision != policy.Revision {
		t.Fatal("preview wrote config")
	}
	if _, err = s.changePreparationTimeout(change, false); err != nil {
		t.Fatal(err)
	}
	if _, err = s.sendPreparation(r); err == nil {
		t.Fatal("stale capability admitted")
	}
	cap := s.preparationCapabilities()[0]
	if cap.Timeout != 1800 || cap.Maximum != 2400 || cap.Hash == before.Hash {
		t.Fatal(cap)
	}
	r.Capability = cap.Hash
	turn, err := s.sendPreparation(r)
	if err != nil {
		t.Fatal(err)
	}
	if turn.TimeoutSeconds != 1800 || turn.TimeoutMaximum != 2400 {
		t.Fatal(turn)
	}
	// Administration changes only future calls, not this persisted exchange.
	change.EventID = "lower-preparation"
	change.Revision = 1
	change.Values = PreparationTimeoutValues{Timeout: 30, Maximum: 60}
	if _, err = s.changePreparationTimeout(change, false); err != nil {
		t.Fatal(err)
	}
	stored, _ := s.preparationTurn(turn.ID)
	if stored.TimeoutSeconds != 1800 || stored.TimeoutMaximum != 2400 {
		t.Fatal(stored)
	}
	s.runPreparationTurn(stored)
	got, _ := s.preparationTurn(turn.ID)
	if got.Status != "answered" {
		t.Fatal(got)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	saved, err := reopened.preparationTimeoutConfig()
	if err != nil || saved.Revision != 2 || saved.Values.Timeout != 30 || len(saved.History) != 2 {
		t.Fatal(saved, err)
	}
	if _, err = reopened.changePreparationTimeout(change, false); err != nil {
		t.Fatal("replay", err)
	}
	change.EventID = "restore"
	change.Revision = 2
	change.Values = saved.History[0].Values
	if _, err = reopened.changePreparationTimeout(change, false); err != nil {
		t.Fatal(err)
	}
	restored, _ := reopened.preparationTimeoutConfig()
	if restored.Revision != 3 || restored.Values.Timeout != 1800 {
		t.Fatal(restored)
	}
}
func TestPreparationTimeoutRejectsInvalidPolicy(t *testing.T) {
	s := storeTest(t)
	for _, v := range []PreparationTimeoutValues{{0, 600}, {-1, 600}, {601, 600}, {1, -1}} {
		if _, err := s.changePreparationTimeout(PreparationTimeoutChange{Schema: 1, EventID: "invalid", Values: v, Reason: "Invalid configuration"}, false); err == nil {
			t.Fatal(v)
		}
	}
	c, err := s.preparationTimeoutConfig()
	if err != nil || c.Revision != 0 {
		t.Fatal(c, err)
	}
}
func TestPreparationRuntimeRejectsInvalidDeadlineBeforeProvider(t *testing.T) {
	for _, deadline := range []time.Duration{0, -time.Second, 601 * time.Second} {
		t.Run(deadline.String(), func(t *testing.T) {
			marker := filepath.Join(t.TempDir(), "started")
			s, _, r := prepDialogueFixture(t, "touch '"+marker+"'\n"+prepReplyScript)
			turn, err := s.sendPreparation(r)
			if err != nil {
				t.Fatal(err)
			}
			s.runPreparationTurnWithin(turn, deadline)
			got, _ := s.preparationTurn(turn.ID)
			if got.Status != "failed" || !strings.Contains(got.Error, "Délai de préparation invalide") {
				t.Fatal(got)
			}
			if _, err = os.Stat(marker); !os.IsNotExist(err) {
				t.Fatal("provider invoked", err)
			}
		})
	}
}
