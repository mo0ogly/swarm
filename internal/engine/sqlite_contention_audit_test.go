//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

const sqliteAuditArchive = "docs/benchmarks/billing/analyse-32-exclusions-20261008.json"

type sqliteAuditCase struct {
	Status      string `json:"status"`
	Attribution string `json:"attribution"`
	BusyEvents  []struct {
		Kind string `json:"kind"`
	} `json:"sqlite_busy_events"`
}

func sqliteAuditHistoricalCounts(t *testing.T) (planning, review, timeouts int) {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join(repositoryRoot(t), sqliteAuditArchive))
	if err != nil {
		t.Fatal(err)
	}
	var archive struct {
		Cases []sqliteAuditCase `json:"cases"`
	}
	if err = json.Unmarshal(raw, &archive); err != nil {
		t.Fatal(err)
	}
	if len(archive.Cases) != 32 {
		t.Fatalf("inventaire historique incomplet: %d cas", len(archive.Cases))
	}
	for _, c := range archive.Cases {
		switch c.Attribution {
		case "storage_busy":
			if len(c.BusyEvents) != 1 {
				t.Fatalf("cas storage_busy sans événement SQLite unique: %+v", c)
			}
			switch c.BusyEvents[0].Kind {
			case "planning.failure":
				planning++
			case "review.failure":
				review++
			default:
				t.Fatalf("frontière SQLite inconnue: %s", c.BusyEvents[0].Kind)
			}
		case "planning_progress_unresolved":
			if c.Status != "DÉLAI" || len(c.BusyEvents) != 0 {
				t.Fatalf("délai attribué à tort à SQLite: %+v", c)
			}
			timeouts++
		default:
			t.Fatalf("attribution historique inconnue: %q", c.Attribution)
		}
	}
	return planning, review, timeouts
}

func sqliteAuditRetryPolicy(t *testing.T, s *Store, retries, delayMS, busyMS int) {
	t.Helper()
	raw := []byte(fmt.Sprintf(`{"schema_version":1,"busy_retries":%d,"busy_retry_delay_ms":%d}`, retries, delayMS))
	if err := os.WriteFile(filepath.Join(s.root, ".swarm", "storage-retry.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec(fmt.Sprintf("PRAGMA busy_timeout=%d", busyMS)); err != nil {
		t.Fatal(err)
	}
}

func sqliteAuditWriter(t *testing.T, s *Store, work string) (*Store, *sql.Tx) {
	t.Helper()
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { other.db.Close() })
	if _, err = other.db.Exec("PRAGMA busy_timeout=0"); err != nil {
		t.Fatal(err)
	}
	tx, err := other.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", work); err != nil {
		tx.Rollback()
		t.Fatal(err)
	}
	return other, tx
}

func sqliteAuditPlanningDecision(t *testing.T) (*Store, Work, PlanningRequest) {
	t.Helper()
	s, w := planningFixture(t)
	w, request := planningClaim(t, s, w, "root")
	request.Operations = []PlanningOperation{planningTask("audited-decision")}
	return s, w, request
}

func TestSQLiteAuditPlanning(t *testing.T) {
	planning, review, timeouts := sqliteAuditHistoricalCounts(t)
	if planning != 14 || review != 13 || timeouts != 5 {
		t.Fatalf("classification historique modifiée: planning=%d review=%d délais=%d", planning, review, timeouts)
	}

	t.Run("brief_lock_released", func(t *testing.T) {
		s, w, request := sqliteAuditPlanningDecision(t)
		sqliteAuditRetryPolicy(t, s, 2, 10, 500)
		_, tx := sqliteAuditWriter(t, s, w.ID)
		done := make(chan error, 1)
		go func() {
			_, err := s.planningChange(w.ID, "decide", request)
			done <- err
		}()
		select {
		case err := <-done:
			t.Fatalf("la décision n'a pas attendu le verrou bref: %v", err)
		case <-time.After(40 * time.Millisecond):
		}
		if err := tx.Commit(); err != nil {
			t.Fatal(err)
		}
		if err := <-done; err != nil {
			t.Fatal(err)
		}
		got, err := s.get(w.ID)
		if err != nil || len(got.Tasks) != 1 || got.Planning.Inbox[0].Decision != request.EventID {
			t.Fatalf("décision non durable après libération: tasks=%d err=%v", len(got.Tasks), err)
		}
	})

	t.Run("holder_interrupted", func(t *testing.T) {
		s, w, request := sqliteAuditPlanningDecision(t)
		sqliteAuditRetryPolicy(t, s, 2, 10, 500)
		_, tx := sqliteAuditWriter(t, s, w.ID)
		done := make(chan error, 1)
		go func() {
			_, err := s.planningChange(w.ID, "decide", request)
			done <- err
		}()
		time.Sleep(40 * time.Millisecond)
		if err := tx.Rollback(); err != nil {
			t.Fatal(err)
		}
		if err := <-done; err != nil {
			t.Fatal(err)
		}
		got, _ := s.get(w.ID)
		if len(got.Tasks) != 1 || got.Planning.Inbox[0].Decision != request.EventID {
			t.Fatal("l'interruption du détenteur a perdu la décision")
		}
	})

	t.Run("prolonged_lock", func(t *testing.T) {
		s, w, request := sqliteAuditPlanningDecision(t)
		sqliteAuditRetryPolicy(t, s, 1, 5, 5)
		_, tx := sqliteAuditWriter(t, s, w.ID)
		done := make(chan error, 1)
		go func() {
			_, err := s.planningChange(w.ID, "decide", request)
			done <- err
		}()
		var err error
		select {
		case err = <-done:
		case <-time.After(2 * time.Second):
			t.Fatal("contention persistante non bornée")
		}
		if err == nil || !strings.Contains(err.Error(), "SQLITE_BUSY") {
			t.Fatalf("cause stockage non observable: %v", err)
		}
		if rollbackErr := tx.Rollback(); rollbackErr != nil {
			t.Fatal(rollbackErr)
		}
		got, getErr := s.get(w.ID)
		if getErr != nil || len(got.Tasks) != 0 || got.Planning.Inbox[0].Decision != "" {
			t.Fatalf("effet partiel sous contention: tasks=%d decision=%q err=%v", len(got.Tasks), got.Planning.Inbox[0].Decision, getErr)
		}
	})

	t.Run("revision_conflict_is_not_storage", func(t *testing.T) {
		s, w, request := sqliteAuditPlanningDecision(t)
		current := planningDo(t, s, w, "pause", PlanningRequest{})
		if current.Revision == request.Revision {
			t.Fatal("fixture sans conflit")
		}
		_, err := s.planningChange(w.ID, "decide", request)
		if commandFailure(err).Code != "revision_conflict" || strings.Contains(fmt.Sprint(err), "SQLITE_BUSY") {
			t.Fatalf("conflit de révision mal qualifié: %v", err)
		}
	})
}

func sqliteAuditReviewFixture(t *testing.T, barrier bool) (*Store, Work, string) {
	t.Helper()
	s, p := preparedTeam(t)
	p, err := s.preparationCommand(conversionRequest(t, s, p, "release-plan"))
	if err != nil {
		t.Fatal(err)
	}
	w, err := s.get(p.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	task := &w.Tasks[0]
	task.Status = "blocked"
	task.Attempts = []Attempt{{ID: "sqlite-audit-production", Status: "completed"}}
	task.Criteria = []string{"Le rapport contient la phrase : preuve SQLite auditée"}
	report := "docs/" + task.ID + ".md"
	if err = os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(s.root, report), []byte("preuve SQLite auditée dans le rapport"), 0600); err != nil {
		t.Fatal(err)
	}
	agent := Agent{ID: "sqlite-audit-producer", WorkID: w.ID, TaskID: task.ID, Attempt: task.Attempts[0].ID, Status: "completed", CWD: s.root, Started: time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano), Role: "worker"}
	providers, err := s.providers()
	if err != nil {
		t.Fatal(err)
	}
	provider := providers.Providers[w.Planning.Reviewer.Provider]
	reply := `{"reason":"La preuve auditée est présente dans le rapport","criteria":[{"index":1,"verdict":"pass","evidence":"preuve SQLite auditée"}]}`
	envelope, _ := json.Marshal(map[string]any{"type": "result", "result": reply})
	script := "#!/bin/sh\ncat >\"$0.prompt\"\n"
	if barrier {
		script += ": >\"$0.entered\"\nwhile [ ! -f \"$0.release\" ]; do sleep 0.01; done\n"
	}
	script += "printf '%s\\n' '" + string(envelope) + "'\n"
	if err = os.WriteFile(provider.Command, []byte(script), 0700); err != nil {
		t.Fatal(err)
	}
	body, _ := json.Marshal(w)
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID); err != nil {
		t.Fatal(err)
	}
	body, _ = json.Marshal(agent)
	if _, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,?,?,?)", agent.ID, agent.WorkID, agent.TaskID, agent.CWD, "completed", "run", body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	if _, err = s.webAction(webRequest{Kind: "submit", Work: w.ID, Task: task.ID, Path: report, Revision: w.Revision}); err != nil {
		t.Fatal(err)
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	return s, w, provider.Command
}

func sqliteAuditWaitFile(t *testing.T, path string) {
	t.Helper()
	deadline := time.Now().Add(5 * time.Second)
	for {
		if _, err := os.Stat(path); err == nil {
			return
		}
		if time.Now().After(deadline) {
			t.Fatalf("barrière de revue absente: %s", path)
		}
		time.Sleep(10 * time.Millisecond)
	}
}

func TestSQLiteAuditReview(t *testing.T) {
	planning, review, timeouts := sqliteAuditHistoricalCounts(t)
	if planning != 14 || review != 13 || timeouts != 5 {
		t.Fatalf("classification historique modifiée: planning=%d review=%d délais=%d", planning, review, timeouts)
	}

	for _, tc := range []struct {
		name      string
		interrupt bool
	}{
		{name: "brief_lock_released"},
		{name: "holder_interrupted", interrupt: true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			s, w, command := sqliteAuditReviewFixture(t, true)
			sqliteAuditRetryPolicy(t, s, 2, 10, 500)
			done := make(chan error, 1)
			go func() { done <- s.independentReviewStep(w.ID) }()
			sqliteAuditWaitFile(t, command+".entered")
			_, tx := sqliteAuditWriter(t, s, w.ID)
			if err := os.WriteFile(command+".release", []byte("go"), 0600); err != nil {
				t.Fatal(err)
			}
			time.Sleep(40 * time.Millisecond)
			var err error
			if tc.interrupt {
				err = tx.Rollback()
			} else {
				err = tx.Commit()
			}
			if err != nil {
				t.Fatal(err)
			}
			if err = <-done; err != nil {
				t.Fatal(err)
			}
			got, _ := s.get(w.ID)
			task, _ := got.task(w.Tasks[0].ID)
			if task.IndependentReview == nil || task.IndependentReview.State != "passed" || got.Planning.Reviewer.Calls != 1 {
				t.Fatalf("verdict non durable après libération: %+v calls=%d", task.IndependentReview, got.Planning.Reviewer.Calls)
			}
		})
	}

	t.Run("prolonged_lock", func(t *testing.T) {
		s, w, command := sqliteAuditReviewFixture(t, true)
		sqliteAuditRetryPolicy(t, s, 1, 5, 5)
		done := make(chan error, 1)
		go func() { done <- s.independentReviewStep(w.ID) }()
		sqliteAuditWaitFile(t, command+".entered")
		_, tx := sqliteAuditWriter(t, s, w.ID)
		if err := os.WriteFile(command+".release", []byte("go"), 0600); err != nil {
			t.Fatal(err)
		}
		var err error
		select {
		case err = <-done:
		case <-time.After(2 * time.Second):
			t.Fatal("persistance de revue non bornée")
		}
		if err == nil || !strings.Contains(err.Error(), "SQLITE_BUSY") {
			t.Fatalf("cause stockage non observable: %v", err)
		}
		if rollbackErr := tx.Rollback(); rollbackErr != nil {
			t.Fatal(rollbackErr)
		}
		got, getErr := s.get(w.ID)
		if getErr != nil {
			t.Fatal(getErr)
		}
		task, _ := got.task(w.Tasks[0].ID)
		if task.IndependentReview == nil || task.IndependentReview.State != "running" || got.Planning.Reviewer.Calls != 1 {
			t.Fatalf("état inattendu après échec de persistance: %+v calls=%d", task.IndependentReview, got.Planning.Reviewer.Calls)
		}
		if err = s.independentReviewStep(w.ID); err != nil {
			t.Fatal("reprise d'interruption non persistée", err)
		}
		got, _ = s.get(w.ID)
		task, _ = got.task(w.Tasks[0].ID)
		if task.IndependentReview.State != "passed" || got.Planning.Reviewer.Calls != 1 || !strings.Contains(task.IndependentReview.Reason, "preuve auditée") {
			t.Fatalf("résultat calculé non repris sans nouvel appel: %+v calls=%d", task.IndependentReview, got.Planning.Reviewer.Calls)
		}
	})

	t.Run("provider_timeout_is_not_storage", func(t *testing.T) {
		s, w, command := sqliteAuditReviewFixture(t, false)
		data, err := os.ReadFile(command)
		if err != nil {
			t.Fatal(err)
		}
		data = []byte(strings.Replace(string(data), "cat >\"$0.prompt\"", "cat >\"$0.prompt\"\nsleep 2", 1))
		if err = os.WriteFile(command, data, 0700); err != nil {
			t.Fatal(err)
		}
		w, err = s.planningChange(w.ID, "review-timeout", PlanningRequest{Schema: 1, EventID: "sqlite-audit-timeout", Revision: w.Revision, ReviewTimeoutSeconds: 1, Reason: "Distinguer le délai fournisseur de la contention SQLite"})
		if err != nil {
			t.Fatal(err)
		}
		if err = s.independentReviewStep(w.ID); err != nil {
			t.Fatal(err)
		}
		got, _ := s.get(w.ID)
		task, _ := got.task(w.Tasks[0].ID)
		if task.IndependentReview == nil || task.IndependentReview.State != "error" || !strings.Contains(task.IndependentReview.Reason, "délai") || strings.Contains(task.IndependentReview.Reason, "SQLITE_BUSY") {
			t.Fatalf("timeout mal qualifié: %+v", task.IndependentReview)
		}
	})
}
