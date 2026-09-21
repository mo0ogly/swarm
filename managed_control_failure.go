//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

// Failure evidence never grants acceptance and is never reused as a passing
// receipt. Keep exact bounded bytes even for non-UTF8 output (JSON base64).
type managedControlFailure struct {
	Version     int                     `json:"version"`
	Work        string                  `json:"work"`
	Task        string                  `json:"checked_task"`
	Producer    string                  `json:"producer"`
	Attempt     string                  `json:"attempt"`
	Candidate   string                  `json:"candidate_commit"`
	Previous    string                  `json:"previous_candidate"`
	Control     ValidationControl       `json:"control"`
	Result      ValidationControlResult `json:"result"`
	Output      []byte                  `json:"output_base64"`
	OutputBytes int                     `json:"output_bytes"`
	Truncated   bool                    `json:"truncated"`
}

func (s *Store) preserveManagedControlFailure(w Work, a Agent, task, candidate string, c ValidationControl, r ValidationControlResult, output []byte, total int) (string, error) {
	if r.Passed || len(output) > maxValidationOutput || total < len(output) || (r.Executed && hash(output) != r.OutputHash) {
		return "", fmt.Errorf("diagnostic de contrôle incohérent")
	}
	record := managedControlFailure{1, w.ID, task, a.ID, a.Attempt, candidate, w.Planning.Repository.Candidate, c, r, output, total, total > len(output)}
	data, err := json.MarshalIndent(record, "", "  ")
	if err != nil {
		return "", err
	}
	dir := filepath.Join(s.root, ".swarm", "managed", w.ID, "diagnostics")
	if err = os.MkdirAll(dir, 0700); err != nil {
		return "", err
	}
	// A fresh immutable artifact keeps every explicit retry, including identical
	// output. Atomic publication avoids exposing a partially written diagnostic.
	name := filepath.Join(dir, newID("control-failure-")+".json")
	if err = atomicWrite(name, data); err != nil {
		return "", err
	}
	rel, err := filepath.Rel(s.root, name)
	return filepath.ToSlash(rel), err
}
