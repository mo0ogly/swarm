package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"time"
)

const trackingMigration = `BEGIN;
CREATE TABLE decisions(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), body BLOB NOT NULL);
CREATE INDEX decisions_work ON decisions(work_id);
CREATE TABLE session_visits(work_id TEXT NOT NULL REFERENCES works(id), operator TEXT NOT NULL, revision INTEGER NOT NULL, at TEXT NOT NULL, PRIMARY KEY(work_id,operator));
CREATE TABLE budgets(work_id TEXT PRIMARY KEY REFERENCES works(id), body BLOB NOT NULL);
CREATE TABLE reservations(agent_id TEXT PRIMARY KEY REFERENCES agents(id), work_id TEXT NOT NULL REFERENCES works(id), amount REAL NOT NULL, state TEXT NOT NULL);
PRAGMA user_version=3;
COMMIT;`

type Decision struct {
	ID         string `json:"id"`
	TaskID     string `json:"task_id"`
	AgentID    string `json:"agent_id,omitempty"`
	Kind       string `json:"kind"`
	Summary    string `json:"summary"`
	Evidence   string `json:"evidence"`
	Created    string `json:"created"`
	Author     string `json:"author,omitempty"`
	ResolvedAt string `json:"resolved_at,omitempty"`
	Resolution string `json:"resolution,omitempty"`
}

func operatorIdentity() string {
	host, _ := os.Hostname()
	return fmt.Sprintf("%s/uid:%d", host, os.Getuid())
}
func (s *Store) decisions(work string) ([]Decision, error) {
	w, e := s.get(work)
	if e != nil {
		return nil, e
	}
	agents, e := s.agents(work)
	if e != nil {
		return nil, e
	}
	add := func(t Task, a Agent, kind, summary, proof, version string) error {
		d := Decision{ID: hash([]byte(work + "|" + t.ID + "|" + a.ID + "|" + kind + "|" + version)), TaskID: t.ID, AgentID: a.ID, Kind: kind, Summary: summary, Evidence: proof, Created: now()}
		raw, _ := json.Marshal(d)
		_, e := s.db.Exec("INSERT OR IGNORE INTO decisions(id,work_id,body) VALUES(?,?,?)", d.ID, work, raw)
		return e
	}
	for _, t := range w.Tasks {
		if t.Status == "submitted" {
			if e = add(t, Agent{}, "handoff", "Rapport à examiner", t.Next, fmt.Sprint(len(t.Attempts))); e != nil {
				return nil, e
			}
		}
		if t.Gate != nil && !s.validGate(&t) {
			if e = add(t, Agent{}, "gate", "Gate bloquée ou preuves périmées", t.ID+" : consulter gates et preuves", t.Gate.At); e != nil {
				return nil, e
			}
		}
	}
	for _, a := range agents {
		t, e := w.task(a.TaskID)
		if e != nil {
			continue
		}
		if t.Status == "accepted" || t.Status == "waived" {
			continue
		}
		kind := ""
		if a.Status == "completed" {
			kind = "handoff"
		}
		if a.Status == "failed" || a.Status == "interrupted" {
			kind = "execution"
		}
		last, e := time.Parse(time.RFC3339Nano, a.Heartbeat)
		if activeAgent(a) && e == nil && time.Since(last) > time.Duration(max(30, a.Limits.SilenceSeconds))*time.Second {
			kind = "silence"
		}
		if kind != "" {
			version := ""
			if kind == "silence" {
				version = a.Heartbeat
			}
			if e = add(*t, a, kind, a.Activity, "Journaux de "+a.ID, version); e != nil {
				return nil, e
			}
		}
	}
	rows, e := s.db.Query("SELECT body FROM decisions WHERE work_id=? ORDER BY rowid", work)
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	out := []Decision{}
	for rows.Next() {
		var raw []byte
		var d Decision
		if e = rows.Scan(&raw); e != nil {
			return nil, e
		}
		if e = json.Unmarshal(raw, &d); e != nil {
			return nil, e
		}
		out = append(out, d)
	}
	return out, rows.Err()
}
func (s *Store) resolveDecision(work, id, author, note string) error {
	if strings.TrimSpace(author) == "" || len(strings.TrimSpace(note)) < 5 {
		return fmt.Errorf("auteur et décision motivée requis")
	}
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	var raw []byte
	if e = tx.QueryRow("SELECT body FROM decisions WHERE work_id=? AND id=?", work, id).Scan(&raw); e != nil {
		return e
	}
	var d Decision
	if e = json.Unmarshal(raw, &d); e != nil {
		return e
	}
	if d.ResolvedAt != "" {
		if d.Author == author && d.Resolution == note {
			return nil
		}
		return fmt.Errorf("décision déjà acquittée")
	}
	d.Author = author
	d.Resolution = note
	d.ResolvedAt = now()
	raw, _ = json.Marshal(d)
	if _, e = tx.Exec("UPDATE decisions SET body=? WHERE id=? AND work_id=?", raw, id, work); e != nil {
		return e
	}
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, d.ResolvedAt, "decision", d.TaskID+" : "+author+" : "+note); e != nil {
		return e
	}
	return tx.Commit()
}

type Visit struct {
	Revision int    `json:"revision"`
	At       string `json:"at"`
	Operator string `json:"operator"`
}

func (s *Store) visit(work, operator string) (Visit, error) {
	v := Visit{Operator: operator}
	e := s.db.QueryRow("SELECT revision,at FROM session_visits WHERE work_id=? AND operator=?", work, operator).Scan(&v.Revision, &v.At)
	if e == sql.ErrNoRows {
		return v, nil
	}
	return v, e
}
func (s *Store) markVisit(work, operator string, revision int) error {
	_, e := s.db.Exec("INSERT INTO session_visits(work_id,operator,revision,at) VALUES(?,?,?,?) ON CONFLICT(work_id,operator) DO UPDATE SET revision=excluded.revision,at=excluded.at", work, operator, revision, now())
	return e
}
