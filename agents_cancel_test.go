//go:build linux

package main

import "testing"

func TestCancelUnclaimedLaunchAcrossHostsAndRetry(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Host = "ancienne-session"
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("UPDATE agents SET body=CAST(body AS TEXT) WHERE id=?", a.ID); err != nil {
		t.Fatal(err)
	}
	if err = s.reconcile(a.ID); err != nil {
		t.Fatal(err)
	}
	ended, err := s.agent(a.ID)
	if err != nil || ended.Status != "interrupted" || ended.Child != 0 || ended.StopKind != originOperator {
		t.Fatal(ended, err)
	}
	var reservations int
	if err = s.db.QueryRow("SELECT count(*) FROM reservations WHERE agent_id=? AND state='reserved'", a.ID).Scan(&reservations); err != nil || reservations != 0 {
		t.Fatal(reservations, err)
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := w.task(a.TaskID)
	if task.Status != "blocked" {
		t.Fatal(task.Status)
	}
	if err = s.supervise(a.ID); err == nil {
		t.Fatal("Le superviseur tardif a repris une intention annulée")
	}
	revision := w.Revision
	if err = s.reconcile(a.ID); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	if w.Revision != revision {
		t.Fatal("Rejeu non idempotent")
	}
	r.EventID = newID("retry-")
	r.Revision = w.Revision
	r.Previous = a.ID
	if _, created, err := s.prepare(w.ID, r); err != nil || !created {
		t.Fatal("Reprise refusée après libération", created, err)
	}
}

func TestCancelCannotWinAfterSupervisorClaim(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	// Simulate the exact claim window: SQL says starting, body still says queued.
	if _, err = s.db.Exec("UPDATE agents SET status='starting' WHERE id=?", a.ID); err != nil {
		t.Fatal(err)
	}
	if err = s.reconcile(a.ID); err == nil {
		t.Fatal("Annulation autorisée après prise en charge")
	}
	var status string
	s.db.QueryRow("SELECT status FROM agents WHERE id=?", a.ID).Scan(&status)
	if status != "starting" {
		t.Fatal(status)
	}
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	if task.Status != "running" {
		t.Fatal(task.Status)
	}
}

func TestCancelRefusesForeignStartedAttempt(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Status = "starting"
	a.Host = "autre-session"
	a.Supervisor = 123
	a.Heartbeat = now()
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if err = s.reconcile(a.ID); err == nil {
		t.Fatal("Libération d’une exécution étrangère non vérifiée")
	}
	current, _ := s.agent(a.ID)
	if current.Status != "starting" {
		t.Fatal(current.Status)
	}
}

func TestCancelLegacyUnregisteredClaimWithStopRequested(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Host = "ancienne-session"
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("UPDATE agents SET status='starting',desired='stop' WHERE id=?", a.ID); err != nil {
		t.Fatal(err)
	}
	read, err := s.agent(a.ID)
	if err != nil || read.Status != "starting" || !read.UnregisteredClaim || !cancellableQueuedAgent(read) {
		t.Fatal(read, err)
	}
	listed, err := s.pilotAgents(w.ID)
	if err != nil || len(listed) != 1 || listed[0].Status != "starting" {
		t.Fatal(listed, err)
	}
	if err = s.reconcile(a.ID); err != nil {
		t.Fatal(err)
	}
	ended, err := s.agent(a.ID)
	if err != nil || ended.Status != "interrupted" || ended.StopKind != originOperator {
		t.Fatal(ended, err)
	}
	desired, err := s.desired(a.ID)
	if err != nil || desired != "stop" {
		t.Fatal("Un ancien superviseur doit encore lire stop avant tout lancement", desired, err)
	}
	if err = s.supervise(a.ID); err == nil {
		t.Fatal("Lancement tardif autorisé")
	}
	var reserved int
	s.db.QueryRow("SELECT count(*) FROM reservations WHERE agent_id=? AND state='reserved'", a.ID).Scan(&reserved)
	if reserved != 0 {
		t.Fatal("Réservation non libérée")
	}
	current, _ := s.get(w.ID)
	task, _ := current.task(a.TaskID)
	if task.Status != "blocked" {
		t.Fatal(task.Status)
	}
}
