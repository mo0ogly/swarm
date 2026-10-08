//go:build linux

package main

import (
	"testing"
	"time"
)

func TestLaunchWaitsForConcurrentWriterWithoutDuplicate(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	tx, err := other.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", w.ID); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { _, _, err := s.prepare(w.ID, r); done <- err }()
	select {
	case err := <-done:
		t.Fatalf("launch failed instead of waiting for writer: %v", err)
	case <-time.After(200 * time.Millisecond):
	}
	if err = tx.Commit(); err != nil {
		t.Fatal(err)
	}
	select {
	case err = <-done:
		if err != nil {
			t.Fatal(err)
		}
	case <-time.After(6 * time.Second):
		t.Fatal("launch did not resume after writer committed")
	}
	a, created, err := s.prepare(w.ID, r)
	if err != nil || created || a.ID == "" {
		t.Fatal("same request was not idempotent", err)
	}
	agents, err := s.agents(w.ID)
	if err != nil || len(agents) != 1 {
		t.Fatal("expected exactly one reservation", err)
	}
}
