//go:build linux

package main

import "testing"

func TestExchangeHostConsumedRejectsWrongIdentity(t *testing.T) {
	s, w, p, c, a := exchangeFixture(t)
	x, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "host-handoff", Kind: "handoff", AgentID: p.ID, TaskID: p.TaskID, AttemptID: p.Attempt, RecipientTask: c.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{a}})
	if err != nil {
		t.Fatal(err)
	}
	req := ExchangeConsume{Schema: 1, EventID: "host-consume", Exchange: x.ID, AgentID: c.ID, TaskID: c.TaskID, AttemptID: c.Attempt}
	if _, _, err = s.consumeExchange(w.ID, req); err != nil {
		t.Fatal(err)
	}
	req.AgentID = "agent-inconnu"
	if _, _, err = s.consumeExchange(w.ID, req); err == nil {
		t.Fatal("identité inconnue admise au rejeu")
	}
}

func TestExchangeHostExpiredHelpCannotBeAcknowledged(t *testing.T) {
	s, w, p, c, _ := exchangeFixture(t)
	x, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "host-question", Kind: "help_request", AgentID: p.ID, TaskID: p.TaskID, AttemptID: p.Attempt, RecipientTask: c.TaskID, RecipientRole: "worker", Need: "format?", Timeout: 60})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("UPDATE agent_exchanges SET deadline_at='2000-01-01T00:00:00Z' WHERE id=?", x.ID); err != nil {
		t.Fatal(err)
	}
	if _, _, err = s.consumeExchange(w.ID, ExchangeConsume{Schema: 1, EventID: "host-late-ack", Exchange: x.ID, AgentID: c.ID, TaskID: c.TaskID, AttemptID: c.Attempt}); err == nil {
		t.Fatal("demande expirée acquittée avant passage du conducteur")
	}
}

func TestExchangeHostDeleteRestorePreservesExchange(t *testing.T) {
	s, w, p, c, a := exchangeFixture(t)
	x, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "host-lifecycle", Kind: "handoff", AgentID: p.ID, TaskID: p.TaskID, AttemptID: p.Attempt, RecipientTask: c.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{a}})
	if err != nil {
		t.Fatal(err)
	}
	preview, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "delete"})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(preview, "host-delete")); err != nil {
		t.Fatal(err)
	}
	preview, err = s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "restore"})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(preview, "host-restore")); err != nil {
		t.Fatal(err)
	}
	restored, err := s.agentExchange(x.ID)
	if err != nil || restored.ID != x.ID {
		t.Fatal("échange perdu à la restauration", restored, err)
	}
}

func TestExchangeHostConcurrentSendHasOneEffect(t *testing.T) {
	s, w, p, c, a := exchangeFixture(t)
	req := ExchangeSend{Schema: 1, EventID: "host-concurrent", Kind: "handoff", AgentID: p.ID, TaskID: p.TaskID, AttemptID: p.Attempt, RecipientTask: c.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{a}}
	start := make(chan struct{})
	done := make(chan error, 2)
	created := make(chan bool, 2)
	for i := 0; i < 2; i++ {
		go func() { <-start; _, yes, err := s.sendExchange(w.ID, req); created <- yes; done <- err }()
	}
	close(start)
	effects := 0
	for i := 0; i < 2; i++ {
		if err := <-done; err != nil {
			t.Fatal(err)
		}
		if <-created {
			effects++
		}
	}
	if effects != 1 {
		t.Fatal("nombre d’effets", effects)
	}
}

func TestExchangeHostAckReplayBoundToContentAfterRestart(t *testing.T) {
	s, w, p, c, a := exchangeFixture(t)
	x, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "ack-source", Kind: "handoff", AgentID: p.ID, TaskID: p.TaskID, AttemptID: p.Attempt, RecipientTask: c.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{a}})
	if err != nil {
		t.Fatal(err)
	}
	req := ExchangeConsume{Schema: 1, EventID: "ack-durable", Exchange: x.ID, AgentID: c.ID, TaskID: c.TaskID, AttemptID: c.Attempt, Note: "premier contenu"}
	if _, changed, err := s.consumeExchange(w.ID, req); err != nil || !changed {
		t.Fatal(changed, err)
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	if _, changed, err := other.consumeExchange(w.ID, req); err != nil || changed {
		t.Fatal("rejeu", changed, err)
	}
	req.Note = "contenu modifié"
	if _, _, err := other.consumeExchange(w.ID, req); err == nil {
		t.Fatal("rejeu modifié accepté")
	}
	req.Note = "premier contenu"
	preview, err := other.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "delete"})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = other.lifecycleApply(w.ID, lifecycleRequest(preview, "ack-delete")); err != nil {
		t.Fatal(err)
	}
	preview, err = other.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "restore"})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = other.lifecycleApply(w.ID, lifecycleRequest(preview, "ack-restore")); err != nil {
		t.Fatal(err)
	}
	if _, changed, err := other.consumeExchange(w.ID, req); err != nil || changed {
		t.Fatal("reçu perdu à la restauration", changed, err)
	}
}
