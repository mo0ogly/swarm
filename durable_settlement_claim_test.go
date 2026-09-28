//go:build linux

package main

import (
	"sync/atomic"
	"testing"
	"time"
)

func TestManagedSettlementClaimBeforeDispatchAcrossStores(t *testing.T) {
	root := t.TempDir()
	s := &Store{root: root}
	other := &Store{root: root}
	a := Agent{ID: "agent", WorkID: "work", Attempt: "attempt"}
	started, release, finished := make(chan struct{}), make(chan struct{}), make(chan struct{})
	var calls atomic.Int32
	ok, e := s.startManagedSettlement(a, func() { calls.Add(1); close(started); <-release; close(finished) })
	if e != nil || !ok {
		t.Fatal(ok, e)
	}
	<-started
	for i := 0; i < 100; i++ {
		ok, e = other.startManagedSettlement(a, func() { calls.Add(1) })
		if e != nil || ok {
			t.Fatal("duplicate admitted", ok, e)
		}
	}
	if calls.Load() != 1 {
		t.Fatal(calls.Load())
	}
	close(release)
	<-finished
	deadline := time.Now().Add(time.Second)
	done := make(chan struct{})
	for {
		ok, e = other.startManagedSettlement(a, func() { close(done) })
		if e != nil {
			t.Fatal(e)
		}
		if ok {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("claim not released")
		}
		time.Sleep(time.Millisecond)
	}
	<-done
}
