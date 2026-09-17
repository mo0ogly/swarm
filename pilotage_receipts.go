//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
)

type agentCommandRecord struct {
	ID      string `json:"id"`
	Digest  string `json:"digest"`
	Status  string `json:"status"`
	Message string `json:"message"`
	Error   string `json:"error,omitempty"`
}

// Claim before executing: a lost response never repeats an agent command.
// An interrupted claimant remains explicitly uncertain and requires inspection.
func (s *Store) agentCommandReceipt(r webRequest) (any, error) {
	if !safeName(r.Event) {
		return nil, fmt.Errorf("identifiant de commande invalide")
	}
	raw, _ := json.Marshal(r)
	digest := hash(raw)
	var saved string
	err := s.db.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='agent-command' AND json_extract(message,'$.id')=? ORDER BY seq DESC LIMIT 1", r.Work, r.Event).Scan(&saved)
	if err == nil {
		var record agentCommandRecord
		if err = json.Unmarshal([]byte(saved), &record); err != nil {
			return nil, err
		}
		if record.Digest != digest {
			return nil, fmt.Errorf("identifiant de commande déjà utilisé pour une autre demande")
		}
		if record.Status == "pending" {
			return nil, fmt.Errorf("commande déjà enregistrée ; résultat non confirmé, examinez l’état de cette tentative")
		}
		if record.Error != "" {
			return nil, &CommandError{Code: "agent_command_refused", Message: record.Error}
		}
		return map[string]string{"message": record.Message}, nil
	}
	if err != sql.ErrNoRows {
		return nil, err
	}
	a, err := s.agent(r.Agent)
	if err != nil {
		return nil, err
	}
	if a.WorkID != r.Work || (r.Task != "" && a.TaskID != r.Task) {
		return nil, fmt.Errorf("tentative incompatible avec le travail ou la tâche affichée")
	}
	w, err := s.get(r.Work)
	if err != nil {
		return nil, err
	}
	if w.Revision != r.Revision {
		return nil, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	record := agentCommandRecord{ID: r.Event, Digest: digest, Status: "pending"}
	raw, _ = json.Marshal(record)
	result, err := s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) SELECT ?,?,'agent-command',? WHERE NOT EXISTS (SELECT 1 FROM cockpit_events WHERE work_id=? AND kind='agent-command' AND json_extract(message,'$.id')=?)", r.Work, now(), string(raw), r.Work, r.Event)
	if err != nil {
		return nil, err
	}
	n, err := result.RowsAffected()
	if err != nil {
		return nil, err
	}
	if n == 0 {
		return s.agentCommandReceipt(r)
	}
	seq, err := result.LastInsertId()
	if err != nil {
		return nil, err
	}
	// Reuse all current guards; the receipt is not permission to bypass them.
	r.Event = ""
	value, actionErr := s.webAction(r)
	record.Status = "done"
	record.Message = "Demande enregistrée ; vérifier l’état observé."
	if actionErr != nil {
		record.Error = actionErr.Error()
	}
	raw, _ = json.Marshal(record)
	if _, err = s.db.Exec("UPDATE cockpit_events SET message=? WHERE seq=?", string(raw), seq); err != nil {
		return nil, err
	}
	if actionErr != nil {
		return nil, &CommandError{Code: "agent_command_refused", Message: actionErr.Error()}
	}
	return value, nil
}
