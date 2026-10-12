//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
	"time"
)

func TestManagedAutomaticPreparedDispatchResumesSameIdentity(t *testing.T) {
	s, w := managedFixture(t)
	w, err := s.mutate(w.ID, "test.hold-independent-task", "hold-second", w.Revision, []byte(`{}`), func(current *Work) error {
		task, err := current.task("second")
		if err != nil {
			return err
		}
		task.LaunchHeld = true
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err := s.setProfile(w.ID, "", LaunchProfile{Provider: "managed-review-fixture", Role: "worker", Workspace: filepath.Join(s.root, "project"), Instruction: "Complete"}, w.Revision); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	conductor := "prepared-dispatch-conductor"
	owned, err := s.claimMissionSupervision(w.ID, conductor, "test", time.Now())
	if err != nil || !owned {
		t.Fatal("isolated conductor reservation", err)
	}
	r := Launch{Schema: 1, EventID: "automatic-prepared-original", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete", Timeout: 60, Origin: originConductor, ConductorID: conductor}
	copy, err := s.ensureManagedAttempt(w, r)
	if err != nil {
		t.Fatal(err)
	}
	marker := filepath.Join(copy, "retained.txt")
	if err = os.WriteFile(marker, []byte("original preparation\n"), 0600); err != nil {
		t.Fatal(err)
	}
	started, err := s.dispatch(w.ID, conductor)
	if err != nil || len(started) != 1 {
		t.Fatalf("authorized preparation did not resume: starts=%d error=%v", len(started), err)
	}
	a, err := s.agent(r.EventID)
	if err != nil || a.Origin != originConductor || a.CWD != copy {
		t.Fatalf("original identity/copy/conductor attribution lost: %+v %v", a, err)
	}
	var request []byte
	if err = s.db.QueryRow("SELECT request FROM agents WHERE id=?", a.ID).Scan(&request); err != nil {
		t.Fatal(err)
	}
	var saved Launch
	if err = json.Unmarshal(request, &saved); err != nil || saved.ConductorID != conductor {
		t.Fatal("conductor attribution missing from launch request", err)
	}
	if data, err := os.ReadFile(marker); err != nil || string(data) != "original preparation\n" {
		t.Fatal("prepared files lost", err)
	}
	after, _ := s.get(w.ID)
	task, _ := after.task("first")
	if len(task.Attempts) != 1 {
		t.Fatal("prepared resume duplicated the production attempt")
	}
	if again, err := s.dispatch(w.ID, conductor); err != nil || len(again) != 0 {
		t.Fatal("repeated dispatch duplicated the launch", again, err)
	}
}

func TestManagedAutomaticPreparedResumeRefusesMissingAuthority(t *testing.T) {
	for _, variant := range []string{"operator", "unattributed", "missing-lease", "foreign-lease", "stale-lease", "paused", "manual", "revoked", "changed-contract"} {
		t.Run(variant, func(t *testing.T) {
			s, w := managedFixture(t)
			conductor := "current-conductor"
			r := Launch{Schema: 1, EventID: "retained-automatic", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Complete", Timeout: 60, Origin: originConductor, ConductorID: "prior-conductor"}
			if variant == "operator" {
				r.Origin, r.ConductorID = originOperator, ""
			}
			if variant == "unattributed" {
				r.ConductorID = ""
			}
			copy, err := s.ensureManagedAttempt(w, r)
			if err != nil {
				t.Fatal(err)
			}
			if variant != "missing-lease" {
				owner, at := conductor, time.Now()
				if variant == "foreign-lease" {
					owner = "other-conductor"
				}
				if variant == "stale-lease" {
					at = at.Add(-2 * missionConductorStaleAfter)
				}
				if owned, err := s.claimMissionSupervision(w.ID, owner, "test", at); err != nil || !owned {
					t.Fatal("isolated conductor setup", err)
				}
			}
			switch variant {
			case "paused":
				err = s.pause(w.ID, true)
			case "manual":
				err = s.setAutonomy(w.ID, autonomyManual, 2)
			case "revoked":
				err = s.setMission(w.ID, false)
			case "changed-contract":
				w = applyTest(t, s, w, "task.update", Request{ID: "first", Next: "Changed instructions"})
			}
			if err != nil {
				t.Fatal(err)
			}
			before, _ := s.get(w.ID)
			if _, created, err := s.resumeAutomaticPreparedLaunch(w.ID, r.EventID, before.Revision, conductor); err == nil || created {
				t.Fatal("automatic resume bypassed authority or freshness", variant, err)
			}
			after, _ := s.get(w.ID)
			if !reflect.DeepEqual(before, after) {
				t.Fatal("refusal changed task, attempts or shared budgets")
			}
			if agents, err := s.agents(w.ID); err != nil || len(agents) != 0 {
				t.Fatal("refusal reserved a provider call", agents, err)
			}
			if _, err := os.Stat(copy); err != nil {
				t.Fatal("retained preparation lost", err)
			}
		})
	}
}
