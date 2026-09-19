//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func exchangeFixture(t *testing.T) (*Store, Work, Agent, Agent, ExchangeArtifact) {
	t.Helper()
	s := storeTest(t)
	w := createTest(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "producer", Title: "Produire", Deliverable: "proof.txt", Criteria: []string{"empreinte"}})
	w = applyTest(t, s, w, "task.add", Request{ID: "consumer", Title: "Consommer", Deliverable: "result.txt", Criteria: []string{"remise"}, Depends: []string{"producer"}})
	request := Request{Schema: 1, EventID: newID("fixture-"), Revision: w.Revision}
	raw, _ := json.Marshal(request)
	w, err := s.mutate(w.ID, "checkpoint", request.EventID, request.Revision, raw, func(current *Work) error {
		producer, _ := current.task("producer")
		consumer, _ := current.task("consumer")
		producer.Status = "running"
		consumer.Status = "running"
		producer.Attempts = []Attempt{{ID: "attempt-producer", Status: "recorded", Started: now()}}
		consumer.Attempts = []Attempt{{ID: "attempt-consumer", Status: "recorded", Started: now()}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	producer := Agent{ID: "agent-producer", WorkID: w.ID, TaskID: "producer", Attempt: "attempt-producer", Role: "worker", CWD: filepath.Join(s.root, "producer"), Status: "completed", Started: now()}
	consumer := Agent{ID: "agent-consumer", WorkID: w.ID, TaskID: "consumer", Attempt: "attempt-consumer", Role: "worker", CWD: filepath.Join(s.root, "consumer"), Status: "completed", Started: now()}
	for _, agent := range []Agent{producer, consumer} {
		body, _ := json.Marshal(agent)
		if _, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", agent.ID, agent.WorkID, agent.TaskID, agent.CWD, agent.Status, body, []byte(`{}`)); err != nil {
			t.Fatal(err)
		}
	}
	if err = os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("proof"), 0600); err != nil {
		t.Fatal(err)
	}
	return s, w, producer, consumer, ExchangeArtifact{Path: "proof.txt", SHA256: hash([]byte("proof"))}
}

func TestExchangeHandoffConsumedOnceAndRejectsStaleAttempt(t *testing.T) {
	s, w, producer, consumer, artifact := exchangeFixture(t)
	request := ExchangeSend{Schema: 1, EventID: "handoff-once", Kind: "handoff", AgentID: producer.ID, TaskID: producer.TaskID,
		AttemptID: producer.Attempt, RecipientTask: consumer.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{artifact}}
	exchange, created, err := s.sendExchange(w.ID, request)
	if err != nil || !created || exchange.State != "pending" {
		t.Fatal(exchange, created, err)
	}
	if duplicate, duplicated, err := s.sendExchange(w.ID, request); err != nil || duplicated || duplicate.ID != exchange.ID {
		t.Fatal("duplicate had an effect", duplicate, duplicated, err)
	}
	consume := ExchangeConsume{Schema: 1, EventID: "consume-once", Exchange: exchange.ID, AgentID: consumer.ID,
		TaskID: consumer.TaskID, AttemptID: consumer.Attempt, Note: "empreinte vérifiée"}
	consumed, changed, err := s.consumeExchange(w.ID, consume)
	if err != nil || !changed || consumed.State != "consumed" || consumed.RecipientAttempt != consumer.Attempt {
		t.Fatal(consumed, changed, err)
	}
	if replay, changed, err := s.consumeExchange(w.ID, consume); err != nil || changed || replay.State != "consumed" {
		t.Fatal("second consumption had an effect", replay, changed, err)
	}
	current, _ := s.get(w.ID)
	mutation := Request{Schema: 1, EventID: newID("new-attempt-"), Revision: current.Revision}
	raw, _ := json.Marshal(mutation)
	_, err = s.mutate(w.ID, "checkpoint", mutation.EventID, mutation.Revision, raw, func(next *Work) error {
		task, _ := next.task(producer.TaskID)
		task.Attempts = append(task.Attempts, Attempt{ID: "attempt-new", Status: "recorded", Started: now()})
		return nil
	})
	request.EventID = "handoff-stale"
	if _, _, err = s.sendExchange(w.ID, request); err == nil {
		t.Fatal("stale producer attempt accepted")
	}
}

func TestExchangeChangedArtifactBecomesStale(t *testing.T) {
	s, w, producer, consumer, artifact := exchangeFixture(t)
	exchange, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "handoff-changed", Kind: "handoff", AgentID: producer.ID,
		TaskID: producer.TaskID, AttemptID: producer.Attempt, RecipientTask: consumer.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{artifact}})
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600); err != nil {
		t.Fatal(err)
	}
	_, _, err = s.consumeExchange(w.ID, ExchangeConsume{Schema: 1, EventID: "consume-changed", Exchange: exchange.ID,
		AgentID: consumer.ID, TaskID: consumer.TaskID, AttemptID: consumer.Attempt})
	if err == nil {
		t.Fatal("changed artifact consumed")
	}
	stored, _ := s.agentExchange(exchange.ID)
	if stored.State != "stale" {
		t.Fatal(stored)
	}
}

func TestExchangeHelpAnswerAndEscalationAreBounded(t *testing.T) {
	s, w, producer, consumer, _ := exchangeFixture(t)
	question := ExchangeSend{Schema: 1, EventID: "help-question", Kind: "help_request", AgentID: producer.ID, TaskID: producer.TaskID,
		AttemptID: producer.Attempt, RecipientTask: consumer.TaskID, RecipientRole: "worker", Need: "Quel format faut-il ?", Timeout: 60, Artifacts: []ExchangeArtifact{}}
	exchange, _, err := s.sendExchange(w.ID, question)
	if err != nil {
		t.Fatal(err)
	}
	ack, changed, err := s.consumeExchange(w.ID, ExchangeConsume{Schema: 1, EventID: "ack-help", Exchange: exchange.ID,
		AgentID: consumer.ID, TaskID: consumer.TaskID, AttemptID: consumer.Attempt, Note: "prise en compte"})
	if err != nil || !changed || ack.State != "acknowledged" {
		t.Fatal(ack, changed, err)
	}
	answer, created, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "help-answer", Kind: "help_answer", AgentID: consumer.ID,
		TaskID: consumer.TaskID, AttemptID: consumer.Attempt, RecipientTask: producer.TaskID, RecipientRole: "worker", ReplyTo: exchange.ID,
		Need: "JSON versionné.", Artifacts: []ExchangeArtifact{}})
	if err != nil || !created || answer.Kind != "help_answer" {
		t.Fatal(answer, created, err)
	}
	answered, _ := s.agentExchange(exchange.ID)
	if answered.State != "answered" {
		t.Fatal(answered)
	}
	late := question
	late.EventID = "help-late"
	late.Timeout = 1
	lateExchange, _, err := s.sendExchange(w.ID, late)
	if err != nil {
		t.Fatal(err)
	}
	count, err := s.expireAgentExchanges(w.ID, "conductor-test", time.Now().Add(2*time.Second))
	if err != nil || count != 1 {
		t.Fatal(count, err)
	}
	lateExchange, _ = s.agentExchange(lateExchange.ID)
	if lateExchange.State != "escalated" || lateExchange.AcknowledgedBy != "conductor-test" {
		t.Fatal(lateExchange)
	}
	wrong := question
	wrong.EventID = "help-wrong-role"
	wrong.RecipientRole = "planner"
	if _, _, err = s.sendExchange(w.ID, wrong); err == nil {
		t.Fatal("agent message granted an unplanned role")
	}
	current, _ := s.get(w.ID)
	if len(current.Tasks) != 2 {
		t.Fatal("exchange created a task outside the plan")
	}
}

func TestExchangeIsObservableThroughCLIAndWeb(t *testing.T) {
	s, w, producer, consumer, artifact := exchangeFixture(t)
	_, _, err := s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "visible-handoff", Kind: "handoff", AgentID: producer.ID,
		TaskID: producer.TaskID, AttemptID: producer.Attempt, RecipientTask: consumer.TaskID, RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{artifact}})
	if err != nil {
		t.Fatal(err)
	}
	var stdout, stderr bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "exchange", "list", w.ID}, &stdout, &stderr); code != 0 || !bytes.Contains(stdout.Bytes(), []byte("visible-handoff")) {
		t.Fatal(code, stdout.String(), stderr.String())
	}
	h := newWebHandler(s, "local.test", "exchange-capability")
	req := httptest.NewRequest(http.MethodGet, "http://local.test/api/v1/exchanges?work="+w.ID, nil)
	req.Host = "local.test"
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "exchange-capability"})
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, req)
	if rr.Code != http.StatusOK || !bytes.Contains(rr.Body.Bytes(), []byte("visible-handoff")) {
		t.Fatal(rr.Code, rr.Body.String())
	}
}

func TestExchangeInstructionsExposeBoundIdentityToAgent(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	agent, created, err := s.prepare(w.ID, request)
	if err != nil || !created {
		t.Fatal(agent, created, err)
	}
	for _, expected := range []string{w.ID, request.TaskID, agent.ID, agent.Attempt, "exchange list", "exchange send", "exchange consume"} {
		if !strings.Contains(agent.Prompt, expected) {
			t.Fatalf("instruction d’échange absente : %s", expected)
		}
	}
}
