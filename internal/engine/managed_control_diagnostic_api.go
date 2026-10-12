//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
)

type ManagedControlDiagnosticView struct {
	Work       string                `json:"work_id"`
	Task       string                `json:"task_id"`
	Revision   int                   `json:"revision"`
	Evidence   string                `json:"evidence_sha256"`
	Diagnostic managedControlFailure `json:"diagnostic"`
}

// Only expose the engine-owned original linked to the current blocked result.
// The client supplies task identity, never an arbitrary private file path.
func (s *Store) managedControlDiagnostic(work, taskID string) (ManagedControlDiagnosticView, error) {
	w, err := s.get(work)
	if err != nil {
		return ManagedControlDiagnosticView{}, err
	}
	task, err := w.task(taskID)
	if err != nil || w.Planning == nil || w.Planning.Repository == nil || task.Status != "blocked" {
		return ManagedControlDiagnosticView{}, fmt.Errorf("résultat bloqué avec diagnostic de contrôle attribué requis")
	}
	events, err := s.events(work)
	if err != nil {
		return ManagedControlDiagnosticView{}, err
	}
	for i := len(events) - 1; i >= 0; i-- {
		event := events[i]
		if event.Kind != "managed.conflict" {
			continue
		}
		var failure struct {
			Agent  string `json:"agent"`
			Reason string `json:"reason"`
		}
		if json.Unmarshal(event.Payload, &failure) != nil || failure.Reason != task.Blocker {
			continue
		}
		a, err := s.agent(failure.Agent)
		if err != nil || a.WorkID != work || a.TaskID != taskID || !currentTaskAttempt(task, a.Attempt) || activeAgent(a) || a.Ended == "" {
			continue
		}
		item, err := s.managedAttempt(a.ID)
		if err != nil || item.Work != work || item.Task != taskID || item.State != "conflict" || item.Detail != task.Blocker {
			continue
		}
		_, evidence, digest, legacy, err := s.integrationFailureBase(w, a, item)
		if err != nil {
			return ManagedControlDiagnosticView{}, err
		}
		if legacy {
			return ManagedControlDiagnosticView{}, fmt.Errorf("sortie originale historique absente ; aucune trace inventée")
		}
		path, err := localFile(s.root, evidence)
		if err != nil {
			return ManagedControlDiagnosticView{}, err
		}
		raw, err := os.ReadFile(path)
		if err != nil {
			return ManagedControlDiagnosticView{}, err
		}
		if hash(raw) != digest {
			return ManagedControlDiagnosticView{}, fmt.Errorf("diagnostic modifié pendant la lecture")
		}
		var d managedControlFailure
		if err = json.Unmarshal(raw, &d); err != nil {
			return ManagedControlDiagnosticView{}, err
		}
		if d.OutputBytes < len(d.Output) || d.Truncated != (d.OutputBytes > len(d.Output)) {
			return ManagedControlDiagnosticView{}, fmt.Errorf("bornes de sortie incohérentes")
		}
		current, err := s.get(work)
		if err != nil {
			return ManagedControlDiagnosticView{}, err
		}
		if current.Revision != w.Revision {
			return ManagedControlDiagnosticView{}, fmt.Errorf("travail modifié pendant la lecture ; relire le diagnostic courant")
		}
		return ManagedControlDiagnosticView{Work: work, Task: taskID, Revision: w.Revision, Evidence: digest, Diagnostic: d}, nil
	}
	return ManagedControlDiagnosticView{}, fmt.Errorf("aucun diagnostic original lié au résultat courant")
}
