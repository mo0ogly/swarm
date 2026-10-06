package main

import (
	"crypto/sha256"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"time"
)

const automationRequestMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS automation_requests(
 request_id TEXT PRIMARY KEY,
 idempotency_key TEXT NOT NULL UNIQUE,
 content_digest TEXT NOT NULL,
 target_work_id TEXT NOT NULL REFERENCES works(id),
 source TEXT NOT NULL,
 action TEXT NOT NULL CHECK(action='request_resume'),
 occurrence_id TEXT NOT NULL UNIQUE,
 state TEXT NOT NULL CHECK(state IN ('received','waiting','claimed','executed','rejected','uncertain_effect')),
 revision INTEGER NOT NULL,
 reason TEXT NOT NULL,
 actor TEXT NOT NULL,
 next_action TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS automation_occurrences(
 occurrence_id TEXT PRIMARY KEY,
 request_id TEXT NOT NULL UNIQUE REFERENCES automation_requests(request_id),
 state TEXT NOT NULL CHECK(state IN ('received','waiting','claimed','executed','rejected','uncertain_effect')),
 revision INTEGER NOT NULL,
 claimed_by TEXT NOT NULL,
 lease_until TEXT NOT NULL,
 effect_id TEXT NOT NULL,
 reason TEXT NOT NULL,
 actor TEXT NOT NULL,
 next_action TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS automation_effects(
 effect_id TEXT PRIMARY KEY,
 request_id TEXT NOT NULL UNIQUE REFERENCES automation_requests(request_id),
 target_work_id TEXT NOT NULL REFERENCES works(id),
 action TEXT NOT NULL CHECK(action='request_resume'),
 created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS automation_occurrences_claimable ON automation_occurrences(state,lease_until);
PRAGMA user_version=25;
COMMIT;`

// AutomationRequestConfig is versioned so later public adapters can expose the
// same operational policy. It is never inferred from request content.
type AutomationRequestConfig struct {
	Version           int `json:"version"`
	ClaimLeaseSeconds int `json:"claim_lease_seconds"`
}

func defaultAutomationRequestConfig() AutomationRequestConfig {
	return AutomationRequestConfig{Version: 1, ClaimLeaseSeconds: 30}
}

func (c AutomationRequestConfig) validate() error {
	if c.Version != 1 || c.ClaimLeaseSeconds < 5 || c.ClaimLeaseSeconds > 300 {
		return fmt.Errorf("configuration automation invalide : version 1 et bail de 5 à 300 secondes requis")
	}
	return nil
}

type AutomationResumeRequest struct {
	Schema         int    `json:"schema_version"`
	IdempotencyKey string `json:"idempotency_key"`
	ContentDigest  string `json:"content_digest"`
	Source         string `json:"source"`
	TargetWorkID   string `json:"target_work_id"`
	Action         string `json:"action"`
}

type AutomationRequestRecord struct {
	RequestID      string `json:"request_id"`
	OccurrenceID   string `json:"occurrence_id"`
	IdempotencyKey string `json:"idempotency_key"`
	ContentDigest  string `json:"content_digest"`
	TargetWorkID   string `json:"target_work_id"`
	Source         string `json:"source"`
	Action         string `json:"action"`
	State          string `json:"state"`
	Revision       int    `json:"revision"`
	ClaimedBy      string `json:"claimed_by,omitempty"`
	LeaseUntil     string `json:"lease_until,omitempty"`
	EffectID       string `json:"effect_id,omitempty"`
	Reason         string `json:"reason,omitempty"`
	Actor          string `json:"actor,omitempty"`
	NextAction     string `json:"next_action,omitempty"`
	CreatedAt      string `json:"created_at"`
	UpdatedAt      string `json:"updated_at"`
}

func automationResumeDigest(r AutomationResumeRequest) string {
	canonical, _ := json.Marshal(struct {
		Schema       int    `json:"schema_version"`
		Source       string `json:"source"`
		TargetWorkID string `json:"target_work_id"`
		Action       string `json:"action"`
	}{r.Schema, r.Source, r.TargetWorkID, r.Action})
	sum := sha256.Sum256(canonical)
	return hex.EncodeToString(sum[:])
}

func validateAutomationResumeRequest(r AutomationResumeRequest) error {
	if r.Schema != 1 || !safeName(r.IdempotencyKey) || !safeName(r.TargetWorkID) || r.Action != "request_resume" {
		return &CommandError{Code: "invalid_input", Message: "schema_version, idempotency_key, cible et action request_resume requis"}
	}
	if strings.TrimSpace(r.Source) == "" || len([]byte(r.Source)) > 200 {
		return &CommandError{Code: "invalid_input", Message: "source requise et limitée à 200 octets"}
	}
	if r.ContentDigest != automationResumeDigest(r) {
		return &CommandError{Code: "invalid_input", Message: "content_digest ne correspond pas au contenu canonique"}
	}
	return nil
}

func scanAutomationRequest(row interface{ Scan(...any) error }) (AutomationRequestRecord, error) {
	var r AutomationRequestRecord
	err := row.Scan(&r.RequestID, &r.OccurrenceID, &r.IdempotencyKey, &r.ContentDigest,
		&r.TargetWorkID, &r.Source, &r.Action, &r.State, &r.Revision, &r.ClaimedBy,
		&r.LeaseUntil, &r.EffectID, &r.Reason, &r.Actor, &r.NextAction, &r.CreatedAt, &r.UpdatedAt)
	return r, err
}

const automationRequestSelect = `SELECT r.request_id,r.occurrence_id,r.idempotency_key,r.content_digest,
 r.target_work_id,r.source,r.action,r.state,r.revision,o.claimed_by,o.lease_until,o.effect_id,
 r.reason,r.actor,r.next_action,r.created_at,r.updated_at
 FROM automation_requests r JOIN automation_occurrences o ON o.request_id=r.request_id`

func (s *Store) automationRequest(id string) (AutomationRequestRecord, error) {
	return scanAutomationRequest(s.db.QueryRow(automationRequestSelect+" WHERE r.request_id=?", id))
}

func automationTerminal(w Work) bool {
	if w.Planning == nil {
		return false
	}
	root, err := w.Planning.scope("root")
	return err == nil && root.State == "closed"
}

func automationPolicyTx(tx *sql.Tx, work string) (bool, string, error) {
	var body []byte
	if err := tx.QueryRow("SELECT body FROM works WHERE id=?", work).Scan(&body); err != nil {
		return false, "unknown_target", err
	}
	var w Work
	if err := json.Unmarshal(body, &w); err != nil {
		return false, "invalid_target", err
	}
	var archived int
	if err := tx.QueryRow("SELECT count(*) FROM mission_lifecycle WHERE work_id=?", work).Scan(&archived); err != nil {
		return false, "storage_error", err
	}
	if archived > 0 || automationTerminal(w) {
		return false, "terminal_target", nil
	}
	if err := organizationGuard(w); err != nil {
		return false, "authorization_required", nil
	}
	var policyRaw string
	if err := tx.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1", work).Scan(&policyRaw); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return false, "authorization_required", nil
		}
		return false, "storage_error", err
	}
	var policy MissionPolicy
	if err := json.Unmarshal([]byte(policyRaw), &policy); err != nil {
		return false, "authorization_required", nil
	}
	var autonomy string
	var paused int
	if err := tx.QueryRow("SELECT autonomy,paused FROM cockpit_controls WHERE work_id=?", work).Scan(&autonomy, &paused); err != nil {
		return false, "authorization_required", nil
	}
	if !policy.Enabled || autonomy != autonomyAuto || paused != 0 {
		return false, "authorization_required", nil
	}
	if w.Planning != nil && (w.Planning.Paused || w.Planning.Failure != "" || w.Planning.Activations >= w.Planning.MaxActivations || w.Planning.Decisions >= w.Planning.MaxDecisions) {
		return false, "budget_or_planning_blocked", nil
	}
	return true, "", nil
}

func (s *Store) submitAutomationResume(r AutomationResumeRequest) (AutomationRequestRecord, bool, error) {
	var zero AutomationRequestRecord
	if err := validateAutomationResumeRequest(r); err != nil {
		return zero, false, err
	}
	tx, err := s.db.Begin()
	if err != nil {
		return zero, false, err
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", r.TargetWorkID); err != nil {
		return zero, false, err
	}
	existing, err := scanAutomationRequest(tx.QueryRow(automationRequestSelect+" WHERE r.idempotency_key=?", r.IdempotencyKey))
	if err == nil {
		if existing.ContentDigest != r.ContentDigest || existing.TargetWorkID != r.TargetWorkID || existing.Source != r.Source || existing.Action != r.Action {
			return zero, false, &CommandError{Code: "idempotency_conflict", Message: "clé déjà utilisée pour un contenu différent"}
		}
		return existing, false, nil
	}
	if !errors.Is(err, sql.ErrNoRows) {
		return zero, false, err
	}
	requestID := "request-" + hash([]byte(r.IdempotencyKey))[:24]
	occurrenceID := "occurrence-" + hash([]byte(requestID))[:24]
	stamp := now()
	state, reason, actor, next := "received", "", "request-service", "claim"
	authorized, code, policyErr := automationPolicyTx(tx, r.TargetWorkID)
	if policyErr != nil {
		if errors.Is(policyErr, sql.ErrNoRows) {
			return zero, false, &CommandError{Code: "unknown_target", Message: "cible inconnue"}
		}
		return zero, false, policyErr
	}
	if !authorized {
		state, reason, next = "rejected", code, "none"
	}
	if _, err = tx.Exec(`INSERT INTO automation_requests(request_id,idempotency_key,content_digest,target_work_id,source,action,occurrence_id,state,revision,reason,actor,next_action,created_at,updated_at)
 VALUES(?,?,?,?,?,?,?,?,1,?,?,?,?,?)`, requestID, r.IdempotencyKey, r.ContentDigest, r.TargetWorkID, r.Source, r.Action, occurrenceID, state, reason, actor, next, stamp, stamp); err != nil {
		return zero, false, err
	}
	if _, err = tx.Exec(`INSERT INTO automation_occurrences(occurrence_id,request_id,state,revision,claimed_by,lease_until,effect_id,reason,actor,next_action,created_at,updated_at)
 VALUES(?,?,?,1,'','','',?,?,?, ?,?)`, occurrenceID, requestID, state, reason, actor, next, stamp, stamp); err != nil {
		return zero, false, err
	}
	if err = tx.Commit(); err != nil {
		return zero, false, err
	}
	got, err := s.automationRequest(requestID)
	return got, err == nil, err
}

func (s *Store) automationWorkspaceBusyTx(tx *sql.Tx, work string) (bool, error) {
	var body []byte
	if err := tx.QueryRow("SELECT body FROM works WHERE id=?", work).Scan(&body); err != nil {
		return false, err
	}
	var w Work
	if err := json.Unmarshal(body, &w); err != nil {
		return false, err
	}
	workspaces := []string{}
	if w.Profile != nil && w.Profile.Workspace != "" {
		workspaces = append(workspaces, w.Profile.Workspace)
	}
	for _, task := range w.Tasks {
		if task.Profile != nil && task.Profile.Workspace != "" {
			workspaces = append(workspaces, task.Profile.Workspace)
		}
	}
	for _, workspace := range workspaces {
		var count int
		if err := tx.QueryRow(`SELECT count(*) FROM agents WHERE status IN ('queued','starting','running','stopping')
 AND (cwd=? OR instr(cwd, ? || '/')=1 OR instr(?, cwd || '/')=1)`, workspace, workspace, workspace).Scan(&count); err != nil {
			return false, err
		}
		if count > 0 {
			return true, nil
		}
	}
	return false, nil
}

func updateAutomationStateTx(tx *sql.Tx, id, state, reason, actor, next string) error {
	stamp := now()
	result, err := tx.Exec(`UPDATE automation_requests SET state=?,revision=revision+1,reason=?,actor=?,next_action=?,updated_at=? WHERE request_id=?`, state, reason, actor, next, stamp, id)
	if err != nil {
		return err
	}
	if n, _ := result.RowsAffected(); n != 1 {
		return sql.ErrNoRows
	}
	_, err = tx.Exec(`UPDATE automation_occurrences SET state=?,revision=revision+1,reason=?,actor=?,next_action=?,updated_at=? WHERE request_id=?`, state, reason, actor, next, stamp, id)
	return err
}

func (s *Store) claimAutomationRequest(id, conductor string, at time.Time, config AutomationRequestConfig) (AutomationRequestRecord, bool, error) {
	var zero AutomationRequestRecord
	if !safeName(id) || !safeName(conductor) {
		return zero, false, &CommandError{Code: "invalid_input", Message: "identités de demande et conducteur invalides"}
	}
	if err := config.validate(); err != nil {
		return zero, false, err
	}
	tx, err := s.db.Begin()
	if err != nil {
		return zero, false, err
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE automation_requests SET revision=revision WHERE request_id=?", id); err != nil {
		return zero, false, err
	}
	record, err := scanAutomationRequest(tx.QueryRow(automationRequestSelect+" WHERE r.request_id=?", id))
	if err != nil {
		return zero, false, err
	}
	if record.State == "executed" || record.State == "rejected" || record.State == "uncertain_effect" {
		return record, false, nil
	}
	if record.State == "claimed" && record.ClaimedBy != conductor {
		until, parseErr := time.Parse(time.RFC3339Nano, record.LeaseUntil)
		if parseErr == nil && at.Before(until) {
			return record, false, nil
		}
	}
	if authorized, code, externalErr := s.automationExternalAuthorized(record.Source, record.TargetWorkID, record.Action); externalErr != nil {
		return zero, false, externalErr
	} else if !authorized {
		if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
			return zero, false, err
		}
		if err = tx.Commit(); err != nil {
			return zero, false, err
		}
		got, readErr := s.automationRequest(id)
		return got, false, readErr
	}
	authorized, code, err := automationPolicyTx(tx, record.TargetWorkID)
	if err != nil {
		return zero, false, err
	}
	if !authorized {
		if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
			return zero, false, err
		}
		if err = tx.Commit(); err != nil {
			return zero, false, err
		}
		got, readErr := s.automationRequest(id)
		return got, false, readErr
	}
	busy, err := s.automationWorkspaceBusyTx(tx, record.TargetWorkID)
	if err != nil {
		return zero, false, err
	}
	if busy {
		if err = updateAutomationStateTx(tx, id, "waiting", "workspace_wait", conductor, "retry_after_resource_release"); err != nil {
			return zero, false, err
		}
		if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", id); err != nil {
			return zero, false, err
		}
		if err = tx.Commit(); err != nil {
			return zero, false, err
		}
		got, readErr := s.automationRequest(id)
		return got, false, readErr
	}
	lease := at.Add(time.Duration(config.ClaimLeaseSeconds) * time.Second).UTC().Format(time.RFC3339Nano)
	if err = updateAutomationStateTx(tx, id, "claimed", "", conductor, "apply_request_resume"); err != nil {
		return zero, false, err
	}
	result, err := tx.Exec(`UPDATE automation_occurrences SET claimed_by=?,lease_until=? WHERE request_id=? AND (claimed_by='' OR claimed_by=? OR lease_until<=?)`, conductor, lease, id, conductor, at.UTC().Format(time.RFC3339Nano))
	if err != nil {
		return zero, false, err
	}
	if n, _ := result.RowsAffected(); n != 1 {
		return record, false, nil
	}
	if err = tx.Commit(); err != nil {
		return zero, false, err
	}
	got, err := s.automationRequest(id)
	return got, err == nil, err
}

func (s *Store) applyAutomationResumeEffect(id, conductor string) (string, error) {
	tx, err := s.db.Begin()
	if err != nil {
		return "", err
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE automation_requests SET revision=revision WHERE request_id=?", id); err != nil {
		return "", err
	}
	record, err := scanAutomationRequest(tx.QueryRow(automationRequestSelect+" WHERE r.request_id=?", id))
	if err != nil {
		return "", err
	}
	effectID := "effect-" + hash([]byte(id + "|request_resume"))[:24]
	var existing string
	if err = tx.QueryRow("SELECT effect_id FROM automation_effects WHERE request_id=?", id).Scan(&existing); err == nil {
		return existing, nil
	} else if !errors.Is(err, sql.ErrNoRows) {
		return "", err
	}
	if record.State != "claimed" || record.ClaimedBy != conductor {
		return "", &CommandError{Code: "stale_lease", Message: "bail de demande absent ou remplacé"}
	}
	until, parseErr := time.Parse(time.RFC3339Nano, record.LeaseUntil)
	if parseErr != nil || !time.Now().Before(until) {
		return "", &CommandError{Code: "stale_lease", Message: "bail de demande expiré"}
	}
	if authorized, code, externalErr := s.automationExternalAuthorized(record.Source, record.TargetWorkID, record.Action); externalErr != nil {
		return "", externalErr
	} else if !authorized {
		if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
			return "", err
		}
		if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", id); err != nil {
			return "", err
		}
		if err = tx.Commit(); err != nil {
			return "", err
		}
		return "", &CommandError{Code: code, Message: "portée externe retirée avant effet"}
	}
	authorized, code, err := automationPolicyTx(tx, record.TargetWorkID)
	if err != nil {
		return "", err
	}
	if !authorized {
		if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
			return "", err
		}
		if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", id); err != nil {
			return "", err
		}
		if err = tx.Commit(); err != nil {
			return "", err
		}
		return "", &CommandError{Code: code, Message: "autorisation ou condition retirée avant effet"}
	}
	if _, err = tx.Exec("INSERT INTO automation_effects(effect_id,request_id,target_work_id,action,created_at) VALUES(?,?,?,?,?)", effectID, id, record.TargetWorkID, record.Action, now()); err != nil {
		return "", err
	}
	message, _ := json.Marshal(map[string]string{"request_id": id, "effect_id": effectID, "action": record.Action, "source": record.Source})
	if _, err = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", record.TargetWorkID, now(), "automation-request", string(message)); err != nil {
		return "", err
	}
	if err = tx.Commit(); err != nil {
		return "", err
	}
	return effectID, nil
}

func (s *Store) markAutomationRequestUncertain(id, conductor, reason string) error {
	if strings.TrimSpace(reason) == "" || len([]byte(reason)) > 500 {
		return &CommandError{Code: "invalid_input", Message: "motif d’incertitude requis et limité à 500 octets"}
	}
	tx, err := s.db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	var state, owner string
	if err = tx.QueryRow(`SELECT r.state,o.claimed_by FROM automation_requests r
 JOIN automation_occurrences o ON o.request_id=r.request_id WHERE r.request_id=?`, id).Scan(&state, &owner); err != nil {
		return err
	}
	if state != "claimed" || owner != conductor {
		return &CommandError{Code: "stale_lease", Message: "seul le conducteur attribué peut déclarer un effet incertain"}
	}
	if err = updateAutomationStateTx(tx, id, "uncertain_effect", reason, conductor, "operator_reconciliation"); err != nil {
		return err
	}
	if _, err = tx.Exec("UPDATE automation_occurrences SET lease_until='' WHERE request_id=?", id); err != nil {
		return err
	}
	return tx.Commit()
}

func (s *Store) completeAutomationRequest(id, conductor, effectID string) error {
	tx, err := s.db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	var known string
	if err = tx.QueryRow("SELECT effect_id FROM automation_effects WHERE request_id=?", id).Scan(&known); err != nil {
		return err
	}
	if effectID != known {
		return &CommandError{Code: "uncertain_effect", Message: "identité de l’effet non démontrée"}
	}
	if err = updateAutomationStateTx(tx, id, "executed", "", conductor, "none"); err != nil {
		return err
	}
	if _, err = tx.Exec("UPDATE automation_occurrences SET effect_id=?,lease_until='' WHERE request_id=?", effectID, id); err != nil {
		return err
	}
	return tx.Commit()
}

func (s *Store) processAutomationRequest(id, conductor string, at time.Time, config AutomationRequestConfig) (AutomationRequestRecord, error) {
	if record, err := s.automationRequest(id); err == nil && record.State != "executed" {
		var effectID string
		if lookupErr := s.db.QueryRow("SELECT effect_id FROM automation_effects WHERE request_id=?", id).Scan(&effectID); lookupErr == nil {
			if err = s.completeAutomationRequest(id, conductor, effectID); err != nil {
				return AutomationRequestRecord{}, err
			}
			return s.automationRequest(id)
		} else if !errors.Is(lookupErr, sql.ErrNoRows) {
			return AutomationRequestRecord{}, lookupErr
		}
	}
	record, claimed, err := s.claimAutomationRequest(id, conductor, at, config)
	if err != nil || !claimed {
		return record, err
	}
	effectID, err := s.applyAutomationResumeEffect(id, conductor)
	if err != nil {
		// An absent effect is retryable after lease expiry. If its presence cannot
		// be established, retain an explicit uncertain terminal state.
		var count int
		lookupErr := s.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", id).Scan(&count)
		if lookupErr != nil {
			tx, beginErr := s.db.Begin()
			if beginErr == nil {
				_ = updateAutomationStateTx(tx, id, "uncertain_effect", "effect_lookup_failed", conductor, "operator_reconciliation")
				_ = tx.Commit()
			}
		}
		return AutomationRequestRecord{}, err
	}
	if err = s.completeAutomationRequest(id, conductor, effectID); err != nil {
		return AutomationRequestRecord{}, err
	}
	return s.automationRequest(id)
}
