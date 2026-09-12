package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type LogPage struct {
	Entries      []AgentLog `json:"entries"`
	Next         int64      `json:"next_cursor"`
	RetainedFrom int64      `json:"retained_from"`
	Gap          bool       `json:"retention_gap"`
	More         bool       `json:"more"`
}

func (s *Store) queryLogs(work, agent, query, kind string, after int64, limit int) (LogPage, error) {
	p := LogPage{Entries: []AgentLog{}, Next: after}
	a, e := s.agent(agent)
	if e != nil {
		return p, e
	}
	if a.WorkID != work {
		return p, fmt.Errorf("agent hors travail")
	}
	limit = min(200, max(1, limit))
	var first *int64
	if e = s.db.QueryRow("SELECT min(seq) FROM agent_logs WHERE agent_id=?", agent).Scan(&first); e != nil {
		return p, e
	}
	if first != nil {
		p.RetainedFrom = *first
		p.Gap = after > 0 && after < *first-1
	}
	rows, e := s.db.Query("SELECT seq,agent_id,at,kind,message FROM agent_logs WHERE agent_id=? AND seq>? AND instr(lower(message),lower(?))>0 AND (?='' OR kind=?) ORDER BY seq LIMIT ?", agent, after, query, kind, kind, limit+1)
	if e != nil {
		return p, e
	}
	defer rows.Close()
	for rows.Next() {
		var l AgentLog
		if e = rows.Scan(&l.Seq, &l.AgentID, &l.At, &l.Kind, &l.Message); e != nil {
			return p, e
		}
		if len(p.Entries) == limit {
			p.More = true
			break
		}
		p.Entries = append(p.Entries, l)
		p.Next = l.Seq
	}
	return p, rows.Err()
}
func (s *Store) exportLogPage(work, agent, query, path string) error {
	p, e := s.queryLogs(work, agent, query, "", 0, 200)
	if e != nil {
		return e
	}
	b, e := json.MarshalIndent(p, "", "  ")
	if e != nil {
		return e
	}
	f, e := os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
	if e != nil {
		return e
	}
	defer f.Close()
	_, e = f.Write(b)
	return e
}
