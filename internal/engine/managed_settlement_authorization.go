//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
)

// Local consent is never exported as Work authority. A public retry binds one
// current retained result; it grants neither production nor planner execution.
type managedSettlementConsent struct {
	Event         string `json:"event"`
	Actor         string `json:"actor"`
	Task          string `json:"task"`
	Agent         string `json:"agent"`
	Attempt       string `json:"attempt"`
	Result        string `json:"result"`
	Previous      string `json:"previous"`
	Contract      string `json:"contract"`
	MissionDigest string `json:"mission_sha256"`
}

type settlementQuery interface{ QueryRow(string, ...any) *sql.Row }

func recordManagedSettlementConsent(tx *sql.Tx, w Work, producer, event string) error {
	var task, result, state, body, status, desired string
	if err := tx.QueryRow(`SELECT m.task_id,m.result_commit,m.state,a.body,a.status,a.desired FROM managed_attempts m JOIN agents a ON a.id=m.agent_id WHERE m.agent_id=? AND m.work_id=?`, producer, w.ID).Scan(&task, &result, &state, &body, &status, &desired); err != nil {
		return err
	}
	a, err := decodeAgentRow([]byte(body), status, desired)
	if err != nil {
		return err
	}
	t, err := w.task(task)
	if err != nil || !currentTaskAttempt(t, a.Attempt) || state != "integrating" || result == "" || (a.Status != "completed" && !recoveredResultMatches(t, a, ManagedAttempt{Agent: producer, Result: result})) {
		return fmt.Errorf("résultat courant attribuable requis pour autorisation de règlement")
	}
	var missionRaw string
	if err := tx.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1", w.ID).Scan(&missionRaw); err != nil {
		return err
	}
	var mission MissionPolicy
	if err := json.Unmarshal([]byte(missionRaw), &mission); err != nil || !mission.Enabled {
		return fmt.Errorf("mission locale autorisée requise pour consentement de règlement")
	}
	consent := managedSettlementConsent{Event: event, Actor: operatorIdentity(), Task: task, Agent: producer, Attempt: a.Attempt, Result: result, Previous: w.Planning.Repository.Candidate, Contract: managedReviewContract(w, task), MissionDigest: hash([]byte(missionRaw))}
	raw, err := json.Marshal(consent)
	if err != nil {
		return err
	}
	_, err = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", w.ID, now(), "managed-settlement-consent:"+task, string(raw))
	return err
}

func managedSettlementLiveAuthority(q settlementQuery, w Work) (string, string, error) {
	var mode string
	var paused int
	if err := q.QueryRow("SELECT autonomy,paused FROM cockpit_controls WHERE work_id=?", w.ID).Scan(&mode, &paused); err != nil {
		return "", "", err
	}
	var body []byte
	if err := q.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&body); err != nil {
		return "", "", err
	}
	var live struct {
		Planning *struct {
			Paused bool `json:"paused"`
		} `json:"planning"`
	}
	if err := json.Unmarshal(body, &live); err != nil {
		return "", "", err
	}
	if paused != 0 || live.Planning == nil || live.Planning.Paused {
		return "", "", fmt.Errorf("règlement suspendu : mission ou planification en pause")
	}
	if !validAutonomy(mode) {
		return "", "", fmt.Errorf("mode de règlement invalide")
	}
	var raw string
	if err := q.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1", w.ID).Scan(&raw); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return "", "", &CommandError{Code: "authorization_required", Message: "autorisation de mission absente ou révoquée"}
		}
		return "", "", err
	}
	var mission MissionPolicy
	if err := json.Unmarshal([]byte(raw), &mission); err != nil {
		return "", "", fmt.Errorf("autorisation de mission illisible : %w", err)
	}
	if !mission.Enabled {
		return "", "", &CommandError{Code: "authorization_required", Message: "autorisation de mission absente ou révoquée"}
	}
	return mode, hash([]byte(raw)), nil
}

// A public recovery request still needs local mission permission and no pause;
// result-specific execution consent is recorded only after its own preflight.
func managedSettlementRequestGuard(q settlementQuery, w Work) error {
	_, _, err := managedSettlementLiveAuthority(q, w)
	return err
}

func managedSettlementGuard(q settlementQuery, w Work, a Agent, item ManagedAttempt) error {
	mode, missionDigest, err := managedSettlementLiveAuthority(q, w)
	if err != nil {
		return err
	}
	var raw string
	if mode == autonomyAuto {
		return nil
	}
	if mode != autonomyManual && mode != autonomyAssisted {
		return fmt.Errorf("mode de règlement invalide")
	}
	if item.State != "integrating" || w.Planning.Repository == nil {
		return fmt.Errorf("intention de règlement absente")
	}
	if err := q.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind=? ORDER BY rowid DESC LIMIT 1", w.ID, "managed-settlement-consent:"+a.TaskID).Scan(&raw); err != nil {
		return fmt.Errorf("autorisation explicite du résultat requise : %w", err)
	}
	var consent managedSettlementConsent
	if err := json.Unmarshal([]byte(raw), &consent); err != nil || consent.Event == "" || consent.Actor == "" || consent.MissionDigest != missionDigest {
		return fmt.Errorf("autorisation locale de règlement invalide")
	}
	t, err := w.task(a.TaskID)
	if err != nil || !currentTaskAttempt(t, a.Attempt) || consent.Task != a.TaskID || consent.Agent != a.ID || consent.Attempt != a.Attempt || consent.Result != item.Result || consent.Previous != w.Planning.Repository.Candidate || consent.Contract != managedReviewContract(w, a.TaskID) {
		return fmt.Errorf("autorisation de règlement périmée ou hors résultat")
	}
	return nil
}
