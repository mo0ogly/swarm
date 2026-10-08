//go:build linux

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestManagedReviewDoesNotBlockConductorOrOtherMissions(t *testing.T) {
	s, w := managedFixture(t)
	fixture := filepath.Join(s.root, "review-fixture", "claude")
	data, err := os.ReadFile(fixture)
	if err != nil {
		t.Fatal(err)
	}
	data = bytes.Replace(data, []byte("mode='pass'"), []byte("import time\nwhile not os.path.exists(os.path.join(folder,'release')): time.sleep(.02)\nmode='pass'"), 1)
	if err = os.WriteFile(fixture, data, 0700); err != nil {
		t.Fatal(err)
	}
	release := filepath.Join(s.root, "review-fixture", "release")
	defer os.WriteFile(release, []byte("release"), 0600)
	a := managedCompleted(t, s, w, "first", "review in progress\n")
	other := createTest(t, s)
	previous, registered := map[string]string{}, map[string]bool{}
	cycle := func() {
		t.Helper()
		done := make(chan struct{})
		go func() {
			s.missionOwnedCycle(previous, registered, "responsive-conductor", "recette")
			close(done)
		}()
		select {
		case <-done:
		case <-time.After(2 * time.Second):
			os.WriteFile(release, []byte("release"), 0600)
			<-done
			t.Fatal("independent review blocked conductor cycle")
		}
	}
	cycle()
	deadline := time.Now().Add(3 * time.Second)
	for {
		if _, err = os.Stat(filepath.Join(s.root, "review-fixture", "observed.json")); err == nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("review process not started")
		}
		time.Sleep(10 * time.Millisecond)
	}
	before, err := s.missionSupervision(w.ID, time.Now())
	if err != nil {
		t.Fatal(err)
	}
	cycle()
	after, err := s.missionSupervision(w.ID, time.Now())
	if err != nil || after.State != "active" || after.LastCheckAt == before.LastCheckAt {
		t.Fatalf("conductor heartbeat stalled during review: %+v %v", after, err)
	}
	otherStatus, err := s.missionSupervision(other.ID, time.Now())
	if err != nil || otherStatus.State != "active" || otherStatus.LastCheckAt == "" {
		t.Fatalf("other mission starved: %+v %v", otherStatus, err)
	}
	w, _ = s.get(w.ID)
	task, _ := w.task(a.TaskID)
	if task.IndependentReview == nil || task.IndependentReview.State != "running" || w.Planning.Reviewer.Calls != 1 || task.Status == "accepted" {
		t.Fatalf("running review duplicated or prematurely accepted: %+v", task)
	}
	os.WriteFile(release, []byte("release"), 0600)
	deadline = time.Now().Add(3 * time.Second)
	for {
		w, _ = s.get(w.ID)
		task, _ = w.task(a.TaskID)
		if task.Status == "accepted" {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("released review not integrated: %+v", task)
		}
		time.Sleep(10 * time.Millisecond)
	}
	// Acceptance is persisted before the integration goroutine finishes its Git
	// housekeeping. Wait for ownership release before TempDir removes its files.
	deadline = time.Now().Add(3 * time.Second)
	for {
		releaseReview, lockErr := managedReviewOwnershipLock(s.root, w.ID, a.ID)
		if lockErr == nil {
			releaseReview()
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("integration still owns its files after acceptance", lockErr)
		}
		time.Sleep(10 * time.Millisecond)
	}
	if w.Planning.Reviewer.Calls != 1 || managedReviewCalls(t, s) != 1 {
		t.Fatal("conductor polling duplicated paid review")
	}
}
