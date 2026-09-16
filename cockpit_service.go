package main

import (
	"encoding/json"
	"errors"
)

// Machine-readable code is stable; the French message is shared by terminal and CLI.
type CommandError struct {
	Code      string `json:"code"`
	Message   string `json:"message"`
	Retryable bool   `json:"retryable"`
}

func (e *CommandError) Error() string { return e.Message }
func commandFailure(err error) *CommandError {
	var e *CommandError
	if errors.As(err, &e) {
		return e
	}
	return &CommandError{Code: "command_failed", Message: err.Error()}
}
func (s *Store) executeRequest(work, kind string, r Request) (Work, error) {
	if r.Schema != 1 {
		return Work{}, &CommandError{Code: "invalid_schema", Message: "schema_version doit valoir 1"}
	}
	switch kind {
	case "work.create", "work.update", "task.add", "task.update", "checkpoint", "ooda":
	default:
		return Work{}, &CommandError{Code: "unknown_command", Message: "opération inconnue"}
	}
	raw, e := json.Marshal(r)
	if e != nil {
		return Work{}, e
	}
	return s.mutate(work, kind, r.EventID, r.Revision, raw, func(w *Work) error { return s.apply(w, kind, r) })
}
