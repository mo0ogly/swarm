//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"time"
)

func migrateDialogueCheckpoints(db *sql.DB) error {
	tx, err := db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	var exists int
	if err = tx.QueryRow("SELECT count(*) FROM pragma_table_info('agent_dialogue_turns') WHERE name='checkpoint'").Scan(&exists); err != nil {
		return err
	}
	if exists == 0 {
		if _, err = tx.Exec("ALTER TABLE agent_dialogue_turns ADD COLUMN checkpoint BLOB NOT NULL DEFAULT ''"); err != nil {
			return err
		}
	}
	if _, err = tx.Exec("PRAGMA user_version=34"); err != nil {
		return err
	}
	return tx.Commit()
}

func (m *dialogueMonitor) persistCheckpoint() error {
	phase := "running"
	if !m.active {
		phase = "confirmed"
		if m.failure != "" {
			phase = "interrupted"
		}
	}
	session := m.session
	if m.outcome.session != "" {
		session = m.outcome.session
	}
	v := dialogueCheckpoint{Schema: 1, Agent: m.agent.ID, Turn: m.turn, Project: hash([]byte(m.sink.s.root)), Launch: dialogueLaunchSHA(m.agent),
		Session: session, Phase: phase, Failure: m.failure, At: now(), Guard: checkpointGuard(m.sink.guard), Usage: m.usage,
		Model: m.sink.reportedModelSnapshot(), Cooldown: m.sink.cooldown}
	v.SHA256 = v.digest()
	if _, err := v.restoreGuard(m.agent, m.sink.s.root); err != nil {
		return err
	}
	raw, err := json.Marshal(v)
	if err != nil {
		return err
	}
	result, err := m.sink.s.db.Exec("UPDATE agent_dialogue_turns SET checkpoint=? WHERE id=? AND agent_id=? AND status='running'", raw, m.turn, m.agent.ID)
	if err != nil {
		return err
	}
	n, err := result.RowsAffected()
	if err != nil {
		return err
	}
	if n != 1 {
		return fmt.Errorf("Tour de dialogue modifié ; état non enregistré")
	}
	return nil
}

// DialogueBoundaryView deliberately excludes private guard details and provider
// tool arguments. It describes stored evidence, never authority to execute.
type DialogueBoundaryView struct {
	Available        bool           `json:"available"`
	Turn             string         `json:"turn,omitempty"`
	Status           string         `json:"status,omitempty"`
	Phase            string         `json:"phase,omitempty"`
	Confirmed        bool           `json:"boundary_confirmed"`
	RestartSupported bool           `json:"restart_supported"`
	Session          string         `json:"session_id,omitempty"`
	At               string         `json:"at,omitempty"`
	SHA256           string         `json:"sha256,omitempty"`
	Scope            string         `json:"scope"`
	Progress         *AgentProgress `json:"progress,omitempty"`
	Usage            *Usage         `json:"usage,omitempty"`
}

func (s *Store) dialogueBoundary(a Agent) (*DialogueBoundaryView, error) {
	v, _, err := s.readDialogueBoundary(a)
	return v, err
}

func (s *Store) readDialogueBoundary(a Agent) (*DialogueBoundaryView, *dialogueCheckpoint, error) {
	return s.readDialogueBoundaryWith(s.db, a)
}

func (s *Store) readDialogueBoundaryWith(reader interface{ QueryRow(string, ...any) *sql.Row }, a Agent) (*DialogueBoundaryView, *dialogueCheckpoint, error) {
	if a.Mode != "dialogue" {
		return nil, nil, nil
	}
	v := &DialogueBoundaryView{Scope: "Aucun état de frontière conservé ; gardes et usages historiques inconnus"}
	var raw []byte
	var session string
	err := reader.QueryRow("SELECT id,status,session_id,checkpoint FROM agent_dialogue_turns WHERE agent_id=? ORDER BY rowid DESC LIMIT 1", a.ID).Scan(&v.Turn, &v.Status, &session, &raw)
	if err == sql.ErrNoRows {
		return v, nil, nil
	}
	if err != nil {
		return nil, nil, err
	}
	if v.Status != "running" && v.Status != "answered" && v.Status != "interrupted" {
		return nil, nil, fmt.Errorf("Statut de tour inconnu ; aucune reprise autorisée")
	}
	if len(raw) == 0 {
		return v, nil, nil
	}
	var c dialogueCheckpoint
	if err = strict(raw, &c); err != nil {
		return nil, nil, fmt.Errorf("État de dialogue illisible ; aucune reprise autorisée")
	}
	if c.Turn != v.Turn {
		return nil, nil, fmt.Errorf("État de dialogue hors tour")
	}
	g, err := c.restoreGuard(a, s.root)
	if err != nil {
		return nil, nil, err
	}
	if v.Status == "answered" && c.Phase != "confirmed" {
		return nil, nil, fmt.Errorf("Tour répondu sans état de fin confirmé")
	}
	if v.Status == "answered" && session != c.Session {
		return nil, nil, fmt.Errorf("Session du tour différente de l’état conservé ; aucune reprise autorisée")
	}
	v.Available, v.Phase, v.Session, v.At, v.SHA256 = true, c.Phase, c.Session, c.At, c.SHA256
	v.Confirmed = v.Status == "answered" && c.Phase == "confirmed"
	at, err := time.Parse(time.RFC3339Nano, c.At)
	if err != nil {
		return nil, nil, fmt.Errorf("Date de frontière invalide")
	}
	progress := g.summaryAt(at)
	v.Progress, v.Usage = &progress, c.Usage
	v.Scope = "État observé à cette frontière ; aucune autorisation de redémarrage"
	if c.Phase == "running" {
		v.Scope = "État au début du tour ; effets et usages ultérieurs inconnus"
	}
	return v, &c, nil
}
