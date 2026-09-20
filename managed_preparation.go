//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// A preparation is a durable operation, not a new attempt. It is never adopted
// by another request. An explicit resume reuses its identity and saved settings.
func taskForPreparation(w Work, id string) *Task {
	t, _ := w.task(id)
	return t
}

func managedPreparationContract(w Work, t *Task) string {
	if t == nil || w.Planning == nil || w.Planning.Repository == nil {
		return ""
	}
	data, _ := json.Marshal([]any{w.Objective, w.Scope, t.ID, t.Title, t.Deliverable,
		t.Criteria, t.Depends, t.Next, t.ScopeID, t.Requirements, t.PlanRole,
		t.PlanMaxAttempts, t.PlanToolLimit, t.ValidationPolicy, t.Profile,
		len(t.Attempts), w.Planning.Repository.Candidate})
	return hash(data)
}

func preparedRequestDigest(r Launch) string {
	r.Revision, r.EventID, r.Origin, r.ConductorID, r.ProviderDigest = 0, "", "", "", ""
	if r.Role == "" {
		r.Role = "worker"
	}
	if r.Timeout == 0 {
		r.Timeout = 1800
	}
	b, _ := json.Marshal(r)
	return hash(b)
}

func (s *Store) readPreparedLaunch(w Work, id string) (ManagedAttempt, error) {
	var record ManagedAttempt
	if !safeName(id) || w.Planning == nil || w.Planning.Repository == nil {
		return record, fmt.Errorf("préparation de lancement inconnue")
	}
	repo := w.Planning.Repository
	rel, e := filepath.Rel(s.root, filepath.Join(repo.Storage, "copy-"+id+".json"))
	if e != nil {
		return record, e
	}
	path, e := localFile(s.root, rel)
	if e != nil {
		return record, e
	}
	st, e := os.Stat(path)
	if e != nil {
		return record, e
	}
	if st.Size() > 128*1024 {
		return record, fmt.Errorf("préparation de lancement trop grande")
	}
	b, e := os.ReadFile(path)
	if e != nil {
		return record, e
	}
	if e = strict(b, &record); e != nil {
		return record, e
	}
	if record.Agent != id || record.Work != w.ID || record.Task == "" || record.State != "ready" {
		return record, fmt.Errorf("attribution de préparation incohérente")
	}
	return record, nil
}

func (s *Store) preparedLaunchForTask(w Work, t *Task) (*PreparedLaunch, error) {
	if w.Planning == nil || w.Planning.Repository == nil {
		return nil, nil
	}
	var id string
	e := s.db.QueryRow(`SELECT m.agent_id FROM managed_attempts m
	 WHERE m.work_id=? AND m.task_id=? AND m.state='ready'
	 AND NOT EXISTS(SELECT 1 FROM agents a WHERE a.id=m.agent_id)
	 ORDER BY m.rowid DESC LIMIT 1`, w.ID, t.ID).Scan(&id)
	if e == sql.ErrNoRows {
		// A crash can occur after the filesystem checkpoint but before SQL.
		id, e = s.preparedManifestForTask(w, t)
		if e != nil || id == "" {
			return nil, e
		}
	}
	if e != nil {
		return nil, e
	}
	p := &PreparedLaunch{ID: id, Task: t.ID}
	record, e := s.readPreparedLaunch(w, id)
	if e != nil {
		p.Reason = "Préparation illisible ; copie conservée : " + e.Error()
		return p, nil
	}
	if e = s.preparedLaunchGuard(w, t, record); e != nil {
		p.Reason = e.Error()
		return p, nil
	}
	r := record.Request
	p.Provider, p.Workspace, p.Instruction, p.Timeout, p.Limits = r.Provider, filepath.Join(record.Path, w.Planning.Repository.Subdir), r.Instruction, r.Timeout, r.Limits
	p.Ready = true
	return p, nil
}

func (s *Store) preparedLaunchGuard(w Work, t *Task, record ManagedAttempt) error {
	if record.Request == nil || record.PreparationContract == "" {
		return &CommandError{Code: "prepared_launch_legacy", Message: "Préparation ancienne sans paramètres sauvegardés : conserver la copie et reprendre uniquement avec la demande initiale identifiée."}
	}
	r := record.Request
	if r.EventID != record.Agent || r.TaskID != t.ID || record.Task != t.ID || record.Work != w.ID || r.Schema != 1 {
		return fmt.Errorf("paramètres de préparation incohérents")
	}
	if record.PreparationContract != managedPreparationContract(w, t) || record.Base != w.Planning.Repository.Candidate || record.Path != managedCopyRoot(w.Planning.Repository, t.ID, len(t.Attempts)+1) {
		return &CommandError{Code: "prepared_launch_changed", Message: "La tâche ou sa révision Git a changé depuis la préparation. Reprise refusée ; copie conservée pour examen."}
	}
	return verifyManagedCopy(w.Planning.Repository, record.Path)
}

func (s *Store) resumePreparedLaunch(work, id string, revision int) (Agent, bool, error) {
	if !safeName(id) {
		return Agent{}, false, fmt.Errorf("identifiant de préparation invalide")
	}
	// A lost response or a double click can only retrieve the original agent.
	if a, e := s.agent(id); e == nil {
		if a.WorkID != work {
			return Agent{}, false, fmt.Errorf("préparation hors mission")
		}
		return a, false, nil
	} else if e != sql.ErrNoRows {
		return Agent{}, false, e
	}
	w, e := s.get(work)
	if e != nil {
		return Agent{}, false, e
	}
	if w.Revision != revision {
		return Agent{}, false, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	record, e := s.readPreparedLaunch(w, id)
	if e != nil {
		return Agent{}, false, e
	}
	t, e := w.task(record.Task)
	if e != nil {
		return Agent{}, false, e
	}
	if e = s.preparedLaunchGuard(w, t, record); e != nil {
		return Agent{}, false, e
	}
	r := *record.Request
	r.Revision, r.Origin, r.ConductorID = revision, originOperator, ""
	// All provider, dependency, review, budget and concurrency guards run again.
	return s.prepare(work, r)
}

func (s *Store) preparedManifestForTask(w Work, t *Task) (string, error) {
	entries, e := os.ReadDir(w.Planning.Repository.Storage)
	if e != nil {
		return "", e
	}
	found := ""
	for _, entry := range entries {
		name := entry.Name()
		if !strings.HasPrefix(name, "copy-") || !strings.HasSuffix(name, ".json") {
			continue
		}
		id := strings.TrimSuffix(strings.TrimPrefix(name, "copy-"), ".json")
		record, e := s.readPreparedLaunch(w, id)
		if e != nil || record.Task != t.ID || record.Path != managedCopyRoot(w.Planning.Repository, t.ID, len(t.Attempts)+1) {
			continue
		}
		if _, e = s.agent(id); e == nil {
			continue
		} else if e != sql.ErrNoRows {
			return "", e
		}
		if found != "" {
			return "", fmt.Errorf("plusieurs préparations attribuées à la même copie ; examen requis")
		}
		found = id
	}
	return found, nil
}
