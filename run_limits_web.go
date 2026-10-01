package main

import (
	"io"
	"net/http"
)

// runLimitsFieldBounds mirrors the exact bounds validRunLimitsOverride
// (run_limits_admin.go) enforces for each field. It exists only to label the
// unit and ceiling of each field in the Administration screen (REQ-ADM-04);
// it never decides what is accepted — every write still goes through
// validRunLimitsOverride via configureRunLimits/rollbackRunLimits, the one
// source of truth. TestRunLimitsFieldBoundsMatchValidator keeps this table
// honest against that function.
var runLimitsFieldBounds = []struct {
	Name string `json:"name"`
	Unit string `json:"unit"`
	Max  int    `json:"max"`
}{
	{"silence_seconds", "secondes", 86400},
	{"tool_seconds", "secondes", 86400},
	{"max_tool_calls", "appels", 10000},
	{"max_repeated_calls", "répétitions identiques", 100},
	{"max_consecutive_errors", "erreurs consécutives", 100},
}

// runLimitsPreviewRequest is the payload for a non-writing preview
// (REQ-ADM-06): it never calls configureRunLimits, only the same validators
// that function calls before it opens a transaction.
type runLimitsPreviewRequest struct {
	Scope    string    `json:"scope"`
	Mission  string    `json:"mission_id,omitempty"`
	Key      string    `json:"scope_key,omitempty"`
	Revision int       `json:"expected_revision"`
	Values   RunLimits `json:"values"`
}

type runLimitsRollbackRequest struct {
	Scope      string `json:"scope"`
	Mission    string `json:"mission_id,omitempty"`
	Key        string `json:"scope_key,omitempty"`
	ToRevision int    `json:"to_revision"`
	EventID    string `json:"event_id"`
	Reason     string `json:"reason"`
	Revision   int    `json:"expected_revision"`
}

func (s *Store) registerRunLimitsAdmin(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/run-limits", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET requis", 405)
			return
		}
		scope, mission, key := r.URL.Query().Get("scope"), r.URL.Query().Get("mission_id"), r.URL.Query().Get("scope_key")
		if e := validRunLimitsScope(scope, mission, key); e != nil {
			fail(w, e)
			return
		}
		entry, e := s.currentRunLimitsConfig(scope, mission, key)
		if e != nil {
			fail(w, e)
			return
		}
		hist, e := s.runLimitsHistory(scope, mission, key)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]any{
			"scope": scope, "mission_id": mission, "scope_key": key,
			"entry": entry, "history": hist, "bounds": runLimitsFieldBounds,
			"note": "Portée projet/mission/rôle/tâche ; une valeur à 0 est héritée du niveau englobant. Aucun secret n'est exposé par cet écran.",
		})
	})
	mux.HandleFunc("/api/v1/run-limits/effective", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET requis", 405)
			return
		}
		mission, role, task := r.URL.Query().Get("mission_id"), r.URL.Query().Get("role"), r.URL.Query().Get("task")
		l, e := s.effectiveRunLimits(mission, role, task)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]any{"mission_id": mission, "role": role, "task": task, "effective": l,
			"note": "Valeur qu'obtiendrait un NOUVEAU départ maintenant. Les tentatives déjà lancées gardent leurs limites gelées (REQ-ADM-05)."})
	})
	mux.HandleFunc("/api/v1/run-limits/preview", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		raw, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 4096))
		if e != nil {
			fail(w, e)
			return
		}
		var req runLimitsPreviewRequest
		if e = strict(raw, &req); e != nil {
			fail(w, e)
			return
		}
		// Same validators configureRunLimits calls before writing anything;
		// this handler never opens a transaction and never persists.
		if e = validRunLimitsScope(req.Scope, req.Mission, req.Key); e != nil {
			fail(w, e)
			return
		}
		if e = validRunLimitsOverride(req.Values); e != nil {
			fail(w, e)
			return
		}
		current, e := s.currentRunLimitsConfig(req.Scope, req.Mission, req.Key)
		if e != nil {
			fail(w, e)
			return
		}
		revisionOK := current.Revision == req.Revision
		result := map[string]any{
			"preview": true, "applied": false, "scope": req.Scope, "mission_id": req.Mission, "scope_key": req.Key,
			"current": current, "proposed": req.Values, "revision_ok": revisionOK,
		}
		if !revisionOK {
			result["warning"] = "La configuration a changé depuis le chargement ; relire avant de confirmer."
		}
		result["note"] = "Aperçu sans écriture. Effet réservé aux prochains départs (REQ-ADM-05) ; aucune tentative en cours n'est modifiée."
		send(w, result)
	})
	mux.HandleFunc("/api/v1/run-limits/apply", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		raw, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 4096))
		if e != nil {
			fail(w, e)
			return
		}
		var change RunLimitsConfigChange
		if e = strict(raw, &change); e != nil {
			fail(w, e)
			return
		}
		change.Schema = 1
		entry, e := s.configureRunLimits(change)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, entry)
	})
	mux.HandleFunc("/api/v1/run-limits/rollback", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		raw, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 4096))
		if e != nil {
			fail(w, e)
			return
		}
		var req runLimitsRollbackRequest
		if e = strict(raw, &req); e != nil {
			fail(w, e)
			return
		}
		entry, e := s.rollbackRunLimits(req.Scope, req.Mission, req.Key, req.ToRevision, req.EventID, req.Reason, req.Revision)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, entry)
	})
}
