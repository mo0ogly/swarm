package main

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

func TestFreshnessSharedDAGMemoIsLocalAndRejectsCycles(t *testing.T) {
	s := storeTest(t)
	w := Work{}
	for i := 0; i < 24; i++ {
		task := Task{ID: fmt.Sprintf("t%d", i), Status: "waived", Override: &ManualOverride{Reason: "explicit test waiver"}}
		for j := 0; j < i; j++ {
			task.Depends = append(task.Depends, fmt.Sprintf("t%d", j))
		}
		w.Tasks = append(w.Tasks, task)
	}
	memo := map[string]bool{}
	if !s.acceptedFreshMemo(&w, &w.Tasks[23], map[string]bool{}, memo) || len(memo) != 24 {
		t.Fatal("shared DAG not traversed exactly once", len(memo))
	}
	w.Tasks[0].Status = "todo"
	if s.acceptedFresh(&w, &w.Tasks[23], map[string]bool{}) {
		t.Fatal("prior traversal hid reopened dependency")
	}
	w.Tasks[0].Status = "waived"
	w.Tasks[0].Depends = []string{"t23"}
	if s.acceptedFresh(&w, &w.Tasks[23], map[string]bool{}) {
		t.Fatal("cycle accepted")
	}
}

func TestReadScopeFingerprintsNeverSurviveTheNextRead(t *testing.T) {
	s := storeTest(t)
	raw := fixture(t, s.root)
	ev, e := evaluate(raw, s.root, "delivery")
	if e != nil {
		t.Fatal(e)
	}
	task := Task{ID: "t1", Status: "accepted", Gate: &GateRecord{Document: raw, Evaluation: ev}}
	scope := s.readScope()
	if !scope.validGate(&task) || len(scope.readDigests) == 0 {
		t.Fatal("proofs not captured")
	}
	if s.readDigests != nil {
		t.Fatal("cache retained by live store")
	}
	if e = os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("modified after observation"), 0600); e != nil {
		t.Fatal(e)
	}
	if !scope.validGate(&task) {
		t.Fatal("same read did not reuse its observation")
	}
	if s.readScope().validGate(&task) || s.validGate(&task) {
		t.Fatal("new read or mutation reused stale fingerprints")
	}
}
