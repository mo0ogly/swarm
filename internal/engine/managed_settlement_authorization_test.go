//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestManagedReviewRetryIsExplicitlyAuthorizedInManualMode(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	if err := s.setAutonomy(w.ID, autonomyManual, 1); err != nil {
		t.Fatal(err)
	}
	request := PlanningRequest{Schema: 1, EventID: "manual-result-review", Revision: w.Revision, Task: a.TaskID, Reason: "Authorize review of this retained immutable result without production dispatch"}
	if _, err := s.planningChange(w.ID, "retry-review", request); err != nil {
		t.Fatal(err)
	}
	if err := s.reconcileKnownMissionResult(a, "manual-conductor"); err != nil {
		t.Fatal("explicit result review rejected in manual mode", err)
	}
	after, _ := s.get(w.ID)
	task, _ := after.task(a.TaskID)
	if task.Status != "accepted" || managedReviewCalls(t, s) != 1 || s.autonomy(w.ID) != autonomyManual || len(task.Attempts) != 1 {
		t.Fatal("manual consent changed launch mode or failed settlement", task.Status)
	}
	if err := s.reconcileKnownMissionResult(a, "manual-conductor"); err != nil || managedReviewCalls(t, s) != 1 {
		t.Fatal("manual replay duplicated paid effects", err)
	}
}

func TestManagedManualSettlementRequiresFreshLocalResultSpecificConsent(t *testing.T) {
	for _, kind := range []string{"absent", "paused", "revoked", "restarted-mission", "changed-result", "changed-candidate", "changed-contract", "foreign-task"} {
		t.Run(kind, func(t *testing.T) {
			s, w, a := unpaidReviewFixture(t)
			if err := s.setAutonomy(w.ID, autonomyManual, 1); err != nil {
				t.Fatal(err)
			}
			if kind != "absent" {
				request := PlanningRequest{Schema: 1, EventID: "manual-bound-consent", Revision: w.Revision, Task: a.TaskID, Reason: "Bound permission to review exactly the retained result"}
				var err error
				w, err = s.planningChange(w.ID, "retry-review", request)
				if err != nil {
					t.Fatal(err)
				}
			}
			item, _ := s.managedAttempt(a.ID)
			if kind != "absent" {
				if err := managedSettlementGuard(s.db, w, a, item); err != nil {
					t.Fatal("valid consent rejected before negative case", err)
				}
			}
			switch kind {
			case "paused":
				s.pause(w.ID, true)
			case "revoked":
				s.setMission(w.ID, false)
			case "restarted-mission":
				s.setMission(w.ID, false)
				s.setMission(w.ID, true)
			case "changed-result":
				item.Result = "another-result"
			case "changed-candidate":
				w.Planning.Repository.Candidate = "another-candidate"
			case "changed-contract":
				w.Tasks[0].Criteria = append(w.Tasks[0].Criteria, "changed requirement")
			case "foreign-task":
				a.TaskID = "second"
			}
			if err := managedSettlementGuard(s.db, w, a, item); err == nil {
				t.Fatal("unattributed or stale manual settlement permitted", kind)
			}
			if managedReviewCalls(t, s) != 0 {
				t.Fatal("refusal consumed a model call")
			}
		})
	}
}

func TestManagedPendingReviewRetryCanReceiveFreshManualConsent(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	request := PlanningRequest{Schema: 1, EventID: "queue-auto-result", Revision: w.Revision, Task: a.TaskID, Reason: "Queue retained result under existing authorization"}
	w, err := s.planningChange(w.ID, "retry-review", request)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.setAutonomy(w.ID, autonomyManual, 1); err != nil {
		t.Fatal(err)
	}
	request.EventID = "fresh-manual-pending-consent"
	request.Revision = w.Revision
	w, err = s.planningChange(w.ID, "retry-review", request)
	if err != nil {
		t.Fatal("pending result lost recoverability", err)
	}
	if err = s.reconcileKnownMissionResult(a, "manual-pending-conductor"); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	if after.Tasks[0].Status != "accepted" || managedReviewCalls(t, s) != 1 || len(after.Tasks[0].Attempts) != 1 || s.autonomy(w.ID) != autonomyManual {
		t.Fatal("pending retry lost single-result scope")
	}
}

func TestManagedPlanningPauseDuringReviewPreventsPublication(t *testing.T) {
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
	done := make(chan error, 1)
	go func() { done <- s.integrateManagedAttempt(a) }()
	defer os.WriteFile(filepath.Join(folder, "release"), []byte("go"), 0600)
	deadline := time.Now().Add(10 * time.Second)
	for {
		if _, err := os.Stat(filepath.Join(folder, "entered")); err == nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("review never reached barrier")
		}
		time.Sleep(10 * time.Millisecond)
	}
	current, _ := s.get(w.ID)
	_, err := s.planningChange(w.ID, "pause", PlanningRequest{Schema: 1, EventID: "pause-during-review", Revision: current.Revision, Reason: "Suspend planning before provider reply and publication"})
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(folder, "release"), []byte("go"), 0600); err != nil {
		t.Fatal(err)
	}
	select {
	case <-done:
	case <-time.After(20 * time.Second):
		t.Fatal("review failed to settle")
	}
	after, _ := s.get(w.ID)
	if after.Tasks[0].Status == "accepted" || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
		t.Fatal("planning pause ignored at publication")
	}
	if managedReviewCalls(t, s) != 1 {
		t.Fatal("pause changed consumed call history")
	}
}

func TestManagedManualRepairAndIntegrationUseBoundedConsent(t *testing.T) {
	t.Run("interrupted-repair", func(t *testing.T) {
		s, w, a, r := recoveredResultFixture(t)
		if err := s.setAutonomy(w.ID, autonomyManual, 1); err != nil {
			t.Fatal(err)
		}
		after, err := s.planningChange(w.ID, "submit-recovered-result", r)
		if err != nil {
			t.Fatal(err)
		}
		saved, _ := s.agent(a.ID)
		if after.Tasks[0].Status != "accepted" || managedReviewCalls(t, s) != 1 || saved.Status != "interrupted" || len(after.Tasks[0].Attempts) != 1 || s.autonomy(w.ID) != autonomyManual {
			t.Fatal("manual repair lost bounded permission or history")
		}
	})
	t.Run("integration-retry", func(t *testing.T) {
		s, w, a, r := integrationRetryFixture(t)
		if err := s.setAutonomy(w.ID, autonomyManual, 1); err != nil {
			t.Fatal(err)
		}
		if _, err := s.planningChange(w.ID, "retry-integration", r); err != nil {
			t.Fatal(err)
		}
		before := managedReviewCalls(t, s)
		if err := s.reconcileKnownMissionResult(a, "manual-integration"); err != nil {
			t.Fatal(err)
		}
		after, _ := s.get(w.ID)
		if after.Tasks[0].Status != "accepted" || managedReviewCalls(t, s) != before+1 || len(after.Tasks[0].Attempts) != 1 || s.autonomy(w.ID) != autonomyManual {
			t.Fatal("manual integration did not apply exact authorized result", after.Tasks[0].Status, after.Tasks[0].Blocker, managedReviewCalls(t, s), before)
		}
	})
}

func TestManagedSettlementAuthorityRefusalRetainsItsCause(t *testing.T) {
	for _, kind := range []string{"absent", "revoked", "malformed"} {
		t.Run(kind, func(t *testing.T) {
			s, w := managedFixture(t)
			switch kind {
			case "absent":
				if _, err := s.db.Exec("DELETE FROM cockpit_events WHERE work_id=? AND kind='mission-policy'", w.ID); err != nil {
					t.Fatal(err)
				}
			case "revoked":
				if err := s.setMission(w.ID, false); err != nil {
					t.Fatal(err)
				}
			case "malformed":
				if err := s.controlEvent(w.ID, "mission-policy", "not-json"); err != nil {
					t.Fatal(err)
				}
			}
			before, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			calls := managedReviewCalls(t, s)
			_, _, err = managedSettlementLiveAuthority(s.db, w)
			if err == nil {
				t.Fatal("absent, revoked or malformed authority became permission")
			}
			if kind == "malformed" {
				if commandFailure(err).Code == "authorization_required" || !strings.Contains(err.Error(), "illisible") {
					t.Fatal("malformed authority lost its parse cause", err)
				}
			} else if commandFailure(err).Code != "authorization_required" {
				t.Fatal("authorization refusal became a generic error", err)
			}
			after, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			a, _ := json.Marshal(before)
			b, _ := json.Marshal(after)
			if string(a) != string(b) || managedReviewCalls(t, s) != calls {
				t.Fatal("refused authority mutated work or consumed a call")
			}
		})
	}
}
