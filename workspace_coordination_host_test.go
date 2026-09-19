//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestWorkspaceHostReleasedTurnRejoinsAtEnd(t *testing.T) {
	s := storeTest(t)
	a, _ := setupAgent(t, s)
	b, _ := setupAgent(t, s)
	shared := filepath.Join(s.root, "shared")
	if err := s.enqueueWorkspaceTurn(a.ID, "t1", shared); err != nil {
		t.Fatal(err)
	}
	if err := s.enqueueWorkspaceTurn(b.ID, "t1", shared); err != nil {
		t.Fatal(err)
	}
	if err := s.releaseObsoleteWorkspaceTurns(a.ID, map[string]bool{}); err != nil {
		t.Fatal(err)
	}
	if err := s.enqueueWorkspaceTurn(a.ID, "t1", shared); err != nil {
		t.Fatal(err)
	}
	turns, err := s.workspaceTurns()
	if err != nil {
		t.Fatal(err)
	}
	if blocker := workspaceTurnBlockers(turns, b.ID)["t1"]; blocker != "" {
		t.Fatal("ancien tour relancé passe devant celui qui attendait", blocker)
	}
	if blocker := workspaceTurnBlockers(turns, a.ID)["t1"]; blocker == "" {
		t.Fatal("reprise sans attente derrière le tour existant")
	}
}

func TestWorkspaceHostIntegrationRefusesActiveAgent(t *testing.T) {
	s, w, p, c, a := exchangeFixture(t)
	x, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "host-active-handoff", Kind: "handoff", AgentID: p.ID, TaskID: p.TaskID, AttemptID: p.Attempt, RecipientTask: c.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{a}})
	if err != nil {
		t.Fatal(err)
	}
	canonical := filepath.Join(s.root, "canonical")
	if err = os.Mkdir(canonical, 0700); err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("UPDATE agents SET status='running',cwd=? WHERE id=?", canonical, c.ID); err != nil {
		t.Fatal(err)
	}
	_, _, err = s.integrateWorkspace(w.ID, IntegrationRequest{Schema: 1, EventID: "host-active-integrate", ExchangeID: x.ID, CanonicalWorkspace: canonical, Artifacts: []IntegrationArtifact{{Source: a.Path, Target: "result.txt", ResultSHA256: a.SHA256}}})
	if err == nil || !strings.Contains(err.Error(), "espace canonique occupé") {
		t.Fatal("exclusion intégrateur absente", err)
	}
	if _, err = os.Stat(filepath.Join(canonical, "result.txt")); !os.IsNotExist(err) {
		t.Fatal("cible modifiée", err)
	}
}
func TestWorkspaceHostAgentRefusesActiveIntegration(t *testing.T) {
	s, w, p, c, a := exchangeFixture(t)
	x, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "host-reserved-handoff", Kind: "handoff", AgentID: p.ID, TaskID: p.TaskID, AttemptID: p.Attempt, RecipientTask: c.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{a}})
	if err != nil {
		t.Fatal(err)
	}
	_, launch := setupAgent(t, s)
	receipt := IntegrationReceipt{ID: "host-reserved", WorkID: w.ID, ExchangeID: x.ID, CanonicalWorkspace: s.root, State: "running", RequestedAt: now()}
	body, _ := json.Marshal(receipt)
	_, err = s.db.Exec("INSERT INTO workspace_integrations(id,request_hash,work_id,exchange_id,canonical_workspace,state,requested_at,finished_at,detail) VALUES(?,?,?,?,?,'running',?,'',?)", receipt.ID, "test", w.ID, x.ID, s.root, receipt.RequestedAt, body)
	if err != nil {
		t.Fatal(err)
	}
	// Launch belongs to its own fixture work, sharing the same canonical root.
	var workID string
	if err = s.db.QueryRow("SELECT id FROM works WHERE id!=? ORDER BY rowid DESC LIMIT 1", w.ID).Scan(&workID); err != nil {
		t.Fatal(err)
	}
	if _, _, err = s.prepare(workID, launch); err == nil || !strings.Contains(err.Error(), "espace occupé par l’intégration") {
		t.Fatal("exclusion agent absente", err)
	}
}

func TestWorkspaceHostWaitingIntegrationRejectsReplacedSource(t *testing.T) {
	s, w, p, c, a := exchangeFixture(t)
	x, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "wait-source", Kind: "handoff", AgentID: p.ID, TaskID: p.TaskID, AttemptID: p.Attempt, RecipientTask: c.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{a}})
	if err != nil {
		t.Fatal(err)
	}
	canonical := filepath.Join(s.root, "canonical")
	if err = os.Mkdir(canonical, 0700); err != nil {
		t.Fatal(err)
	}
	_, err = s.db.Exec("INSERT INTO workspace_integrations(id,request_hash,work_id,exchange_id,canonical_workspace,state,requested_at,finished_at,detail) VALUES('holder','test',?,?,?,'running',?,'','{}')", w.ID, x.ID, canonical, now())
	if err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() {
		_, _, e := s.integrateWorkspace(w.ID, IntegrationRequest{Schema: 1, EventID: "wait-request", ExchangeID: x.ID, CanonicalWorkspace: canonical, Artifacts: []IntegrationArtifact{{Source: a.Path, Target: "result.txt", ResultSHA256: a.SHA256}}})
		done <- e
	}()
	// Keep the slot unavailable while the source becomes obsolete.
	time.Sleep(40 * time.Millisecond)
	_, err = s.mutate(w.ID, "checkpoint", "replace-source", w.Revision, []byte(`{}`), func(next *Work) error {
		task, e := next.task(p.TaskID)
		if e != nil {
			return e
		}
		task.Attempts = append(task.Attempts, Attempt{ID: "replacement", Status: "recorded", Started: now()})
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("UPDATE workspace_integrations SET state='integrated' WHERE id='holder'"); err != nil {
		t.Fatal(err)
	}
	select {
	case err = <-done:
		if err == nil {
			t.Fatal("source obsolète intégrée")
		}
	case <-time.After(6 * time.Second):
		t.Fatal("attente non bornée")
	}
	if _, err = os.Stat(filepath.Join(canonical, "result.txt")); !os.IsNotExist(err) {
		t.Fatal("cible modifiée", err)
	}
}
