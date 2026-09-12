package main

import (
	"fmt"
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
