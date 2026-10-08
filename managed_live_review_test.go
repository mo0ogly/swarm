//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// The real reviewer subprocess waits at a barrier while a second Store polls.
// A free Git lock must never let that poll overwrite a still-live verdict.
func TestManagedLiveReviewSurvivesConcurrentConductor(t *testing.T) {
	testManagedReviewWindow(t, false)
}
func TestManagedLiveReviewKeepsVerdictWhenGitLockIsRetaken(t *testing.T) {
	testManagedReviewWindow(t, true)
}
func testManagedReviewWindow(t *testing.T, retainGitLock bool) {
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "result\n")
	folder := filepath.Join(s.root, "review-fixture")
	script := strings.Replace(managedReviewerFixture, "if mode=='exit':", `if mode=='blocked':
 import time
 with open(os.path.join(folder,'entered'),'w') as f: f.write('ready')
 deadline=time.monotonic()+15
 while not os.path.exists(os.path.join(folder,'release')):
  if time.monotonic()>deadline: sys.exit(8)
  time.sleep(.01)
if mode=='exit':`, 1)
	if err := os.WriteFile(filepath.Join(folder, "claude"), []byte(script), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(folder, "mode"), []byte("blocked"), 0600); err != nil {
		t.Fatal(err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	done := make(chan error, 1)
	go func() { done <- s.integrateManagedAttempt(a) }()
	released := false
	defer func() {
		if !released {
			_ = os.WriteFile(filepath.Join(folder, "release"), []byte("go"), 0600)
			select {
			case <-done:
			case <-time.After(20 * time.Second):
			}
		}
	}()
	deadline := time.Now().Add(10 * time.Second)
	for {
		if _, err := os.Stat(filepath.Join(folder, "entered")); err == nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("reviewer never reached barrier")
		}
		time.Sleep(10 * time.Millisecond)
	}
	// The subprocess has entered inference: this must be a real unlocked window.
	releaseGit, lockErr := managedLock(s.root, w.ID)
	if lockErr != nil {
		t.Fatalf("Git lock held during reviewer inference: %v", lockErr)
	}
	gitHeld := true
	defer func() {
		if gitHeld {
			releaseGit()
		}
	}()
	if !retainGitLock {
		releaseGit()
		gitHeld = false
	}
	other.conduct(a, "completed")
	mid, err := other.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := mid.task(a.TaskID)
	if task.IndependentReview == nil || task.IndependentReview.State != "running" {
		t.Fatalf("live reviewer overwritten: %+v", task.IndependentReview)
	}
	if err = os.WriteFile(filepath.Join(folder, "release"), []byte("go"), 0600); err != nil {
		t.Fatal(err)
	}
	select {
	case err = <-done:
		released = true
		if retainGitLock {
			if err == nil || !strings.Contains(err.Error(), "déjà en cours") {
				t.Fatalf("missing transient Git contention: %v", err)
			}
			retained, e := other.get(w.ID)
			if e != nil {
				t.Fatal(e)
			}
			task, _ := retained.task(a.TaskID)
			if task.Status == "accepted" || task.IndependentReview == nil || task.IndependentReview.State != "passed" || managedReviewCalls(t, s) != 1 {
				t.Fatalf("verdict not retained: %+v", task)
			}
			releaseGit()
			gitHeld = false
			if e = other.integrateManagedAttempt(a); e != nil {
				t.Fatal(e)
			}
		} else if err != nil {
			t.Fatal(err)
		}
	case <-time.After(20 * time.Second):
		t.Fatal("review did not finish")
	}
	final, err := other.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = final.task(a.TaskID)
	if task.Status != "accepted" || task.IndependentReview.State != "passed" || managedReviewCalls(t, s) != 1 {
		t.Fatalf("verdict lost or duplicated: status=%s review=%+v calls=%d", task.Status, task.IndependentReview, managedReviewCalls(t, s))
	}
	other.conduct(a, "completed")
	stable, _ := other.get(w.ID)
	if stable.Revision != final.Revision || managedReviewCalls(t, s) != 1 {
		t.Fatal("replay changed accepted verdict")
	}
}
