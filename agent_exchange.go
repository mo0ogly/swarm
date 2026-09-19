package main

import (
	"crypto/sha256"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const agentExchangeMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS agent_exchanges(
 id TEXT PRIMARY KEY,
 request_hash TEXT NOT NULL,
 work_id TEXT NOT NULL REFERENCES works(id),
 kind TEXT NOT NULL,
 source_agent_id TEXT NOT NULL REFERENCES agents(id),
 source_task_id TEXT NOT NULL,
 source_attempt_id TEXT NOT NULL,
 recipient_task_id TEXT NOT NULL,
 recipient_role TEXT NOT NULL,
 recipient_attempt_id TEXT NOT NULL DEFAULT '',
 reply_to TEXT NOT NULL DEFAULT '',
 state TEXT NOT NULL,
 result_state TEXT NOT NULL DEFAULT '',
 need TEXT NOT NULL DEFAULT '',
 artifacts BLOB NOT NULL,
 created_at TEXT NOT NULL,
 deadline_at TEXT NOT NULL DEFAULT '',
 acknowledged_at TEXT NOT NULL DEFAULT '',
 acknowledged_by TEXT NOT NULL DEFAULT '',
 acknowledgement TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS agent_exchange_work ON agent_exchanges(work_id,created_at);
CREATE INDEX IF NOT EXISTS agent_exchange_deadline ON agent_exchanges(work_id,state,deadline_at);
PRAGMA user_version=15;
COMMIT;`

func migrateAgentExchanges(db *sql.DB) error {
	_, err := db.Exec(agentExchangeMigration)
	return err
}

func migrateExchangeAcknowledgements(db *sql.DB) error {
	_, err := db.Exec(`BEGIN IMMEDIATE;
 CREATE TABLE IF NOT EXISTS exchange_acknowledgements(
 id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id),
 exchange_id TEXT NOT NULL REFERENCES agent_exchanges(id),
 request_hash TEXT NOT NULL, created_at TEXT NOT NULL);
 PRAGMA user_version=17; COMMIT;`)
	return err
}

type ExchangeArtifact struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
}

type ExchangeSend struct {
	Schema        int                `json:"schema_version"`
	EventID       string             `json:"event_id"`
	Kind          string             `json:"kind"`
	AgentID       string             `json:"agent_id"`
	TaskID        string             `json:"task_id"`
	AttemptID     string             `json:"attempt_id"`
	RecipientTask string             `json:"recipient_task_id"`
	RecipientRole string             `json:"recipient_role"`
	ReplyTo       string             `json:"reply_to,omitempty"`
	ResultState   string             `json:"result_state,omitempty"`
	Need          string             `json:"need,omitempty"`
	Artifacts     []ExchangeArtifact `json:"artifacts"`
	Timeout       int                `json:"timeout_seconds,omitempty"`
}

type ExchangeConsume struct {
	Schema    int    `json:"schema_version"`
	EventID   string `json:"event_id"`
	Exchange  string `json:"exchange_id"`
	AgentID   string `json:"agent_id"`
	TaskID    string `json:"task_id"`
	AttemptID string `json:"attempt_id"`
	Note      string `json:"acknowledgement"`
}

type AgentExchange struct {
	ID               string             `json:"id"`
	WorkID           string             `json:"work_id"`
	Kind             string             `json:"kind"`
	SourceAgent      string             `json:"source_agent_id"`
	SourceTask       string             `json:"source_task_id"`
	SourceAttempt    string             `json:"source_attempt_id"`
	RecipientTask    string             `json:"recipient_task_id"`
	RecipientRole    string             `json:"recipient_role"`
	RecipientAttempt string             `json:"recipient_attempt_id,omitempty"`
	ReplyTo          string             `json:"reply_to,omitempty"`
	State            string             `json:"state"`
	ResultState      string             `json:"result_state,omitempty"`
	Need             string             `json:"need,omitempty"`
	Artifacts        []ExchangeArtifact `json:"artifacts"`
	CreatedAt        string             `json:"created_at"`
	DeadlineAt       string             `json:"deadline_at,omitempty"`
	AcknowledgedAt   string             `json:"acknowledged_at,omitempty"`
	AcknowledgedBy   string             `json:"acknowledged_by,omitempty"`
	Acknowledgement  string             `json:"acknowledgement,omitempty"`
}

func plannedTaskRole(t *Task) string {
	if t.PlanRole != "" {
		return t.PlanRole
	}
	if t.Profile != nil && t.Profile.Role != "" {
		return t.Profile.Role
	}
	return "worker"
}

func currentTaskAttempt(t *Task, attempt string) bool {
	return len(t.Attempts) > 0 && t.Attempts[len(t.Attempts)-1].ID == attempt
}

func containsString(values []string, wanted string) bool {
	for _, value := range values {
		if value == wanted {
			return true
		}
	}
	return false
}

func (s *Store) validateExchangeAgent(work string, w *Work, agentID, taskID, attemptID string) (Agent, *Task, error) {
	if !safeName(agentID) || !safeName(taskID) || !safeName(attemptID) {
		return Agent{}, nil, fmt.Errorf("agent_id/task_id/attempt_id invalide")
	}
	agent, err := s.agent(agentID)
	if err != nil {
		return Agent{}, nil, fmt.Errorf("agent source inconnu : %w", err)
	}
	if agent.WorkID != work || agent.TaskID != taskID || agent.Attempt != attemptID {
		return Agent{}, nil, fmt.Errorf("identité mission/tâche/tentative incohérente")
	}
	task, err := w.task(taskID)
	if err != nil {
		return Agent{}, nil, err
	}
	if !currentTaskAttempt(task, attemptID) {
		return Agent{}, nil, fmt.Errorf("résultat obsolète : la tentative %s n’est plus courante", attemptID)
	}
	return agent, task, nil
}

func (s *Store) artifactDigest(artifact ExchangeArtifact) (string, error) {
	if artifact.Path == "" || filepath.IsAbs(artifact.Path) || strings.ContainsRune(artifact.Path, '\x00') {
		return "", fmt.Errorf("chemin d’artefact relatif requis")
	}
	wanted, err := hex.DecodeString(artifact.SHA256)
	if err != nil || len(wanted) != sha256.Size || strings.ToLower(artifact.SHA256) != artifact.SHA256 {
		return "", fmt.Errorf("empreinte SHA-256 invalide pour %s", artifact.Path)
	}
	path, err := filepath.EvalSymlinks(filepath.Join(s.root, artifact.Path))
	if err != nil {
		return "", err
	}
	rel, err := filepath.Rel(s.root, path)
	if err != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("artefact hors racine : %s", artifact.Path)
	}
	f, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer f.Close()
	info, err := f.Stat()
	if err != nil || !info.Mode().IsRegular() {
		return "", fmt.Errorf("artefact régulier requis : %s", artifact.Path)
	}
	h := sha256.New()
	if _, err = io.Copy(h, io.LimitReader(f, 64<<20)); err != nil {
		return "", err
	}
	if info.Size() > 64<<20 {
		return "", fmt.Errorf("artefact supérieur à 64 Mio : %s", artifact.Path)
	}
	got := hex.EncodeToString(h.Sum(nil))
	if got != artifact.SHA256 {
		return got, fmt.Errorf("empreinte obsolète pour %s", artifact.Path)
	}
	return got, nil
}

func (s *Store) verifyExchangeArtifacts(artifacts []ExchangeArtifact) error {
	seen := map[string]bool{}
	for _, artifact := range artifacts {
		if seen[artifact.Path] {
			return fmt.Errorf("artefact dupliqué : %s", artifact.Path)
		}
		seen[artifact.Path] = true
		if _, err := s.artifactDigest(artifact); err != nil {
			return err
		}
	}
	return nil
}

func (s *Store) sendExchange(work string, request ExchangeSend) (AgentExchange, bool, error) {
	return retryExchange(func() (AgentExchange, bool, error) { return s.sendExchangeOnce(work, request) })
}

func retryExchange(operation func() (AgentExchange, bool, error)) (AgentExchange, bool, error) {
	for attempt := 0; ; attempt++ {
		exchange, changed, err := operation()
		if err == nil || attempt >= 4 || !strings.Contains(err.Error(), "SQLITE_BUSY") {
			return exchange, changed, err
		}
		// The failed transaction has rolled back. Re-read the same persisted
		// operation identity instead of creating a new request or relaxing guards.
		time.Sleep(time.Duration(attempt+1) * 25 * time.Millisecond)
	}
}

func (s *Store) sendExchangeOnce(work string, request ExchangeSend) (AgentExchange, bool, error) {
	var empty AgentExchange
	if request.Schema != 1 || !safeName(request.EventID) {
		return empty, false, fmt.Errorf("schema_version/event_id invalide")
	}
	if request.Kind != "handoff" && request.Kind != "help_request" && request.Kind != "help_answer" {
		return empty, false, fmt.Errorf("kind doit valoir handoff, help_request ou help_answer")
	}
	if len(request.Need) > 4000 || len(request.Artifacts) > 32 {
		return empty, false, fmt.Errorf("besoin limité à 4000 octets et 32 artefacts")
	}
	raw, _ := json.Marshal(request)
	requestHash := hash(raw)
	// L'identité de commande est examinée avant la fraîcheur : le rejeu exact
	// d'une remise déjà traitée doit rester sans effet, même si une nouvelle
	// tentative a commencé depuis.
	if existing, lookupErr := s.agentExchange(request.EventID); lookupErr == nil {
		if existing.WorkID != work {
			return empty, false, fmt.Errorf("event_id déjà utilisé dans une autre mission")
		}
		var storedHash string
		if err := s.db.QueryRow("SELECT request_hash FROM agent_exchanges WHERE id=?", request.EventID).Scan(&storedHash); err != nil {
			return empty, false, err
		}
		if storedHash != requestHash {
			return empty, false, fmt.Errorf("event_id déjà utilisé avec un autre échange")
		}
		return existing, false, nil
	} else if lookupErr != sql.ErrNoRows {
		return empty, false, lookupErr
	}
	w, err := s.get(work)
	if err != nil {
		return empty, false, err
	}
	if w.Planning != nil {
		return empty, false, fmt.Errorf("mission hiérarchique : remettre le résultat au responsable via planning handoff")
	}
	_, sourceTask, err := s.validateExchangeAgent(work, &w, request.AgentID, request.TaskID, request.AttemptID)
	if err != nil {
		return empty, false, err
	}
	recipient, err := w.task(request.RecipientTask)
	if err != nil {
		return empty, false, fmt.Errorf("destinataire hors plan : %w", err)
	}
	expectedRole := plannedTaskRole(recipient)
	if request.RecipientRole != expectedRole {
		return empty, false, fmt.Errorf("rôle destinataire non autorisé : attendu %s", expectedRole)
	}
	if request.Kind == "handoff" {
		if !containsString(recipient.Depends, sourceTask.ID) {
			return empty, false, fmt.Errorf("remise refusée : %s ne dépend pas de %s", recipient.ID, sourceTask.ID)
		}
		if !containsString([]string{"completed", "blocked", "failed", "needs-review"}, request.ResultState) || len(request.Artifacts) == 0 {
			return empty, false, fmt.Errorf("remise : état explicite et au moins un artefact requis")
		}
	}
	if request.Kind == "help_request" {
		if !nonempty(request.Need) || request.RecipientTask == request.TaskID || request.Timeout < 1 || request.Timeout > 3600 {
			return empty, false, fmt.Errorf("demande d’aide : besoin, autre tâche et timeout_seconds entre 1 et 3600 requis")
		}
	}
	var replied AgentExchange
	if request.Kind == "help_answer" {
		if !nonempty(request.Need) || request.ReplyTo == "" {
			return empty, false, fmt.Errorf("réponse : reply_to et réponse explicite requis")
		}
		replied, err = s.agentExchange(request.ReplyTo)
		if err != nil || replied.WorkID != work || replied.Kind != "help_request" {
			return empty, false, fmt.Errorf("demande d’aide source invalide")
		}
		requester, requesterErr := w.task(replied.SourceTask)
		if requesterErr != nil || !currentTaskAttempt(requester, replied.SourceAttempt) {
			return empty, false, fmt.Errorf("demande d’aide obsolète : tentative demandeuse remplacée")
		}
		deadline, parseErr := time.Parse(time.RFC3339Nano, replied.DeadlineAt)
		if parseErr != nil || !time.Now().Before(deadline) || (replied.State != "pending" && replied.State != "acknowledged") {
			return empty, false, fmt.Errorf("demande d’aide expirée ou déjà traitée")
		}
		if request.TaskID != replied.RecipientTask || request.RecipientTask != replied.SourceTask ||
			(replied.RecipientAttempt != "" && replied.RecipientAttempt != request.AttemptID) {
			return empty, false, fmt.Errorf("réponse hors route autorisée")
		}
	}
	if err = s.verifyExchangeArtifacts(request.Artifacts); err != nil {
		return empty, false, err
	}
	created := now()
	deadline := ""
	if request.Kind == "help_request" {
		deadline = time.Now().UTC().Add(time.Duration(request.Timeout) * time.Second).Format(time.RFC3339Nano)
	}
	artifacts, _ := json.Marshal(request.Artifacts)
	tx, err := s.db.Begin()
	if err != nil {
		return empty, false, err
	}
	defer tx.Rollback()
	// A concurrent sender may have committed after the initial lookup.
	var concurrentHash, concurrentWork string
	lookupErr := tx.QueryRow("SELECT request_hash,work_id FROM agent_exchanges WHERE id=?", request.EventID).Scan(&concurrentHash, &concurrentWork)
	if lookupErr == nil {
		tx.Rollback()
		if concurrentHash != requestHash || concurrentWork != work {
			return empty, false, fmt.Errorf("event_id déjà utilisé avec un autre échange")
		}
		existing, readErr := s.agentExchange(request.EventID)
		return existing, false, readErr
	}
	if lookupErr != sql.ErrNoRows {
		return empty, false, lookupErr
	}
	var currentRevision int
	if err = tx.QueryRow("SELECT revision FROM works WHERE id=?", work).Scan(&currentRevision); err != nil {
		return empty, false, err
	}
	if currentRevision != w.Revision {
		return empty, false, fmt.Errorf("mission modifiée pendant le routage ; relire le plan")
	}
	if request.Kind == "help_answer" {
		result, updateErr := tx.Exec("UPDATE agent_exchanges SET state='answered',acknowledged_at=?,acknowledged_by=? WHERE id=? AND state IN ('pending','acknowledged') AND deadline_at>?", created, request.AgentID, request.ReplyTo, created)
		if updateErr != nil {
			return empty, false, updateErr
		}
		if affected, _ := result.RowsAffected(); affected != 1 {
			return empty, false, fmt.Errorf("demande d’aide expirée ou déjà traitée")
		}
	}
	_, err = tx.Exec(`INSERT INTO agent_exchanges
		(id,request_hash,work_id,kind,source_agent_id,source_task_id,source_attempt_id,recipient_task_id,recipient_role,reply_to,state,result_state,need,artifacts,created_at,deadline_at)
		VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)`, request.EventID, requestHash, work, request.Kind, request.AgentID, request.TaskID, request.AttemptID,
		request.RecipientTask, request.RecipientRole, request.ReplyTo, "pending", request.ResultState, terminalText(request.Need), artifacts, created, deadline)
	if err != nil {
		return empty, false, err
	}
	if err = tx.Commit(); err != nil {
		return empty, false, err
	}
	exchange, err := s.agentExchange(request.EventID)
	if err == nil {
		_ = s.controlEvent(work, "agent-exchange", fmt.Sprintf("%s · %s → %s (%s)", request.Kind, request.TaskID, request.RecipientTask, request.EventID))
	}
	return exchange, true, err
}

func scanAgentExchange(scanner interface{ Scan(...any) error }) (AgentExchange, error) {
	var exchange AgentExchange
	var artifacts []byte
	err := scanner.Scan(&exchange.ID, &exchange.WorkID, &exchange.Kind, &exchange.SourceAgent, &exchange.SourceTask, &exchange.SourceAttempt,
		&exchange.RecipientTask, &exchange.RecipientRole, &exchange.RecipientAttempt, &exchange.ReplyTo, &exchange.State, &exchange.ResultState,
		&exchange.Need, &artifacts, &exchange.CreatedAt, &exchange.DeadlineAt, &exchange.AcknowledgedAt, &exchange.AcknowledgedBy, &exchange.Acknowledgement)
	if err == nil {
		err = json.Unmarshal(artifacts, &exchange.Artifacts)
	}
	return exchange, err
}

const exchangeSelect = `SELECT id,work_id,kind,source_agent_id,source_task_id,source_attempt_id,recipient_task_id,recipient_role,
 recipient_attempt_id,reply_to,state,result_state,need,artifacts,created_at,deadline_at,acknowledged_at,acknowledged_by,acknowledgement FROM agent_exchanges`

func (s *Store) agentExchange(id string) (AgentExchange, error) {
	return scanAgentExchange(s.db.QueryRow(exchangeSelect+" WHERE id=?", id))
}

func (s *Store) agentExchanges(work string) ([]AgentExchange, error) {
	if _, err := s.get(work); err != nil {
		return nil, err
	}
	rows, err := s.db.Query(exchangeSelect+" WHERE work_id=? ORDER BY created_at,id", work)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	out := []AgentExchange{}
	for rows.Next() {
		exchange, scanErr := scanAgentExchange(rows)
		if scanErr != nil {
			return nil, scanErr
		}
		out = append(out, exchange)
	}
	return out, rows.Err()
}

func (s *Store) markExchangeStale(id, reason string) {
	_, _ = s.db.Exec("UPDATE agent_exchanges SET state='stale',acknowledgement=? WHERE id=? AND state IN ('pending','acknowledged')", terminalText(reason), id)
}

func (s *Store) consumeExchange(work string, request ExchangeConsume) (AgentExchange, bool, error) {
	return retryExchange(func() (AgentExchange, bool, error) { return s.consumeExchangeOnce(work, request) })
}

func (s *Store) consumeExchangeOnce(work string, request ExchangeConsume) (AgentExchange, bool, error) {
	var empty AgentExchange
	if request.Schema != 1 || !safeName(request.EventID) || !safeName(request.Exchange) || len(request.Note) > 2000 {
		return empty, false, fmt.Errorf("schema_version/event_id/exchange_id invalide ou accusé trop long")
	}
	exchange, err := s.agentExchange(request.Exchange)
	if err != nil || exchange.WorkID != work {
		return empty, false, fmt.Errorf("échange inconnu dans cette mission")
	}
	w, err := s.get(work)
	if err != nil {
		return empty, false, err
	}
	_, _, err = s.validateExchangeAgent(work, &w, request.AgentID, request.TaskID, request.AttemptID)
	if err != nil {
		return empty, false, err
	}
	if request.TaskID != exchange.RecipientTask {
		return empty, false, fmt.Errorf("échange destiné à une autre tâche")
	}
	raw, _ := json.Marshal(request)
	requestHash := hash(raw)
	var recordedHash, recordedWork string
	receiptErr := s.db.QueryRow("SELECT request_hash,work_id FROM exchange_acknowledgements WHERE id=?", request.EventID).Scan(&recordedHash, &recordedWork)
	if receiptErr == nil {
		if recordedHash != requestHash || recordedWork != work {
			return empty, false, fmt.Errorf("event_id d’acquittement déjà utilisé avec un autre contenu")
		}
		return exchange, false, nil
	}
	if receiptErr != sql.ErrNoRows {
		return empty, false, receiptErr
	}
	if exchange.State != "pending" {
		return empty, false, fmt.Errorf("échange non consommable : %s", exchange.State)
	}
	if exchange.Kind == "help_request" {
		deadline, parseErr := time.Parse(time.RFC3339Nano, exchange.DeadlineAt)
		if parseErr != nil || !time.Now().Before(deadline) {
			return empty, false, fmt.Errorf("demande d’aide expirée")
		}
	}
	source, err := w.task(exchange.SourceTask)
	if err != nil || !currentTaskAttempt(source, exchange.SourceAttempt) {
		s.markExchangeStale(exchange.ID, "tentative source remplacée avant consommation")
		return empty, false, fmt.Errorf("résultat obsolète : tentative source remplacée")
	}
	if exchange.Kind == "help_answer" {
		requestExchange, requestErr := s.agentExchange(exchange.ReplyTo)
		if requestErr != nil || requestExchange.SourceAttempt != request.AttemptID {
			s.markExchangeStale(exchange.ID, "tentative demandeuse remplacée avant réponse")
			return empty, false, fmt.Errorf("réponse obsolète pour cette tentative")
		}
	}
	if err = s.verifyExchangeArtifacts(exchange.Artifacts); err != nil {
		s.markExchangeStale(exchange.ID, err.Error())
		return empty, false, fmt.Errorf("remise obsolète : %w", err)
	}
	state := "consumed"
	if exchange.Kind == "help_request" {
		state = "acknowledged"
	}
	tx, err := s.db.Begin()
	if err != nil {
		return empty, false, err
	}
	defer tx.Rollback()
	receiptErr = tx.QueryRow("SELECT request_hash,work_id FROM exchange_acknowledgements WHERE id=?", request.EventID).Scan(&recordedHash, &recordedWork)
	if receiptErr == nil {
		tx.Rollback()
		if recordedHash != requestHash || recordedWork != work {
			return empty, false, fmt.Errorf("event_id d’acquittement déjà utilisé avec un autre contenu")
		}
		replay, replayErr := s.agentExchange(request.Exchange)
		return replay, false, replayErr
	}
	if receiptErr != sql.ErrNoRows {
		return empty, false, receiptErr
	}
	result, err := tx.Exec(`UPDATE agent_exchanges SET state=?,recipient_attempt_id=?,acknowledged_at=?,acknowledged_by=?,acknowledgement=?
		WHERE id=? AND state='pending' AND (kind!='help_request' OR deadline_at>?) AND EXISTS(SELECT 1 FROM works WHERE id=? AND revision=?)`, state, request.AttemptID, now(), request.AgentID, terminalText(request.Note), exchange.ID, now(), work, w.Revision)
	if err != nil {
		return empty, false, err
	}
	affected, _ := result.RowsAffected()
	if affected != 1 {
		return empty, false, fmt.Errorf("échange déjà traité, expiré ou mission modifiée ; relire son état")
	}
	if _, err = tx.Exec("INSERT INTO exchange_acknowledgements(id,work_id,exchange_id,request_hash,created_at) VALUES(?,?,?,?,?)", request.EventID, work, exchange.ID, requestHash, now()); err != nil {
		return empty, false, err
	}
	if err = tx.Commit(); err != nil {
		return empty, false, err
	}
	exchange, err = s.agentExchange(exchange.ID)
	return exchange, true, err
}

func (s *Store) expireAgentExchanges(work, conductor string, at time.Time) (int64, error) {
	result, err := s.db.Exec(`UPDATE agent_exchanges SET state='escalated',acknowledged_at=?,acknowledged_by=?,
	 acknowledgement='Délai de réponse dépassé ; intervention requise' WHERE work_id=? AND kind='help_request'
	 AND state IN ('pending','acknowledged') AND deadline_at<>'' AND deadline_at<=?`, at.UTC().Format(time.RFC3339Nano), conductor, work, at.UTC().Format(time.RFC3339Nano))
	if err != nil {
		return 0, err
	}
	count, _ := result.RowsAffected()
	if count > 0 {
		_ = s.controlEvent(work, "agent-exchange-escalated", fmt.Sprintf("%d demande(s) d’aide sans réponse arrivée(s) à échéance ; intervention requise", count))
	}
	return count, nil
}
