//go:build linux

package main

import (
	"encoding/json"
	"time"
)

// A report submission is not production. Old versions recorded a synthetic
// attempt when an operator submitted a completed worker's report. Only the
// audited signature of that bug permits moving it out of production attempts.
func (s *Store) legacyReportSubmission(work string, t *Task, agents []Agent) (string, bool) {
	if t.Status != "submitted" || len(t.Attempts) < 2 || t.IndependentReview != nil || t.Gate != nil {
		return "", false
	}
	last := t.Attempts[len(t.Attempts)-1]
	prior := t.Attempts[len(t.Attempts)-2]
	if last.Status != "completed" || prior.Status != "completed" {
		return "", false
	}
	started, e := time.Parse(time.RFC3339Nano, last.Started)
	if e != nil {
		return "", false
	}
	ended, e := time.Parse(time.RFC3339Nano, last.Ended)
	if e != nil || ended.Before(started) || ended.Sub(started) > time.Second {
		return "", false
	}
	var producer *Agent
	for i := range agents {
		a := &agents[i]
		if a.TaskID != t.ID {
			continue
		}
		if a.Attempt == last.ID || a.Status == "running" || a.Status == "starting" || a.Status == "queued" || a.Status == "stopping" {
			return "", false
		}
		if a.Attempt == prior.ID && a.Status == "completed" {
			producer = a
		}
	}
	if producer == nil {
		return "", false
	}
	report, _ := s.provenAttemptReport(*producer)
	if report == "" {
		return "", false
	}
	var at string
	var raw []byte
	if e = s.db.QueryRow("SELECT at,payload FROM events WHERE work_id=? AND kind='task.submit' ORDER BY revision DESC LIMIT 1", work).Scan(&at, &raw); e != nil {
		return "", false
	}
	var r Request
	when, e := time.Parse(time.RFC3339Nano, at)
	if e != nil || when.Before(started) || when.Sub(ended) > time.Second || json.Unmarshal(raw, &r) != nil || r.ID != t.ID || r.Origin != "" || r.Next != "Évaluer les preuves et la gate delivery ; handoff : "+report {
		return "", false
	}
	return report, true
}
