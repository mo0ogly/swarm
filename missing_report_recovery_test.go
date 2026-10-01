//go:build linux

package main

import (
	"encoding/json"
	"os"
	"os/exec"
	"reflect"
	"testing"
)

func TestMissingReportRecoveryRequiresCurrentConductorRefusal(t *testing.T) {
	for _, mode := range []string{"valid", "stale", "running", "no-refusal", "mixed", "managed", "review-running"} {
		t.Run(mode, func(t *testing.T) {
			s, w, r := correctiveFixture(t)
			w.Planning.Repository = nil
			w.Tasks[0].IndependentReview = nil
			r.ReviewID = ""
			r.MissingReport = true
			a := Agent{ID: "producer", WorkID: w.ID, TaskID: r.Task, Attempt: r.Attempt, Status: "completed", Relay: "Relais refusé par le conducteur : aucun rapport produit par cette tentative ; 1 rapport(s) antérieur(s) ignoré(s)."}
			switch mode {
			case "stale":
				a.Attempt = "old-2"
			case "running":
				a.Status = "running"
			case "no-refusal":
				a.Relay = ""
			case "mixed":
				r.ReviewID = "invented"
			case "managed":
				w.Planning.Repository = &ManagedRepository{}
			case "review-running":
				w.Tasks[0].IndependentReview = &IndependentReview{State: "running"}
			}
			raw, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			raw, _ = json.Marshal(a)
			if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, r.Task, s.root, a.Status, raw, []byte("{}")); err != nil {
				t.Fatal(err)
			}
			after, err := s.planningChange(w.ID, "authorize-recovery", r)
			if mode != "valid" {
				if err == nil {
					t.Fatal("invalid grant accepted")
				}
				persisted, _ := s.get(w.ID)
				if persisted.Tasks[0].CorrectiveRecovery != nil || persisted.Tasks[0].PlanMaxAttempts != 3 {
					t.Fatal("failed grant was not atomic")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			task := after.Tasks[0]
			if task.PlanMaxAttempts != 4 || !task.CorrectiveRecovery.MissingReport || task.IndependentReview != nil || task.Status != "blocked" || !reflect.DeepEqual(task.Attempts, w.Tasks[0].Attempts) || task.PlanToolLimit != w.Tasks[0].PlanToolLimit {
				t.Fatal("grant changed history, review or limits")
			}
			r.EventID = "again"
			r.Revision = after.Revision
			if _, err = s.planningChange(w.ID, "authorize-recovery", r); err == nil {
				t.Fatal("second exceptional grant accepted")
			}
		})
	}
}

func TestMissingReportRecoveryBrowserRecipe(t *testing.T) {
	binary, out := os.Getenv("SWARM_CORRECTIVE_UI_BINARY"), os.Getenv("SWARM_CORRECTIVE_UI_OUT")
	if binary == "" {
		t.Skip("browser recipe opt-in")
	}
	s, w, r := correctiveFixture(t)
	w.Planning.Repository = nil
	w.Tasks[0].IndependentReview = nil
	w.Tasks[0].Blocker = "Relais refusé par le conducteur : aucun rapport produit par cette tentative"
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	a := Agent{ID: "missing-producer", WorkID: w.ID, TaskID: r.Task, Attempt: r.Attempt, Status: "completed", Relay: w.Tasks[0].Blocker}
	raw, _ = json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, w.ID, r.Task, s.root, a.Status, raw, []byte("{}")); err != nil {
		t.Fatal(err)
	}
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command("node", "tests/corrective_recovery_ui.cjs", binary, out, s.root, w.ID, "missing")
	b, err := cmd.CombinedOutput()
	t.Log(string(b))
	if err != nil {
		t.Fatal(err)
	}
}
