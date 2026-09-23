package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"io"
)

// TaskModel is an operator authorization, separate from execution history and
// planner/reviewer routes. The policy fingerprint is checked again at launch.
type TaskModel struct {
	Provider string     `json:"provider"`
	Route    ModelRoute `json:"route"`
	Actor    string     `json:"actor"`
	At       string     `json:"at"`
}
type TaskModelRequest struct {
	Schema     int    `json:"schema_version"`
	EventID    string `json:"event_id"`
	Revision   int    `json:"expected_revision"`
	Task       string `json:"task_id"`
	Provider   string `json:"provider"`
	Level      string `json:"level"`
	PolicyHash string `json:"model_policy_hash"`
	Inherit    bool   `json:"inherit"`
}

func (s *Store) prepareTaskModel(w *Work, r TaskModelRequest) error {
	if r.Schema != 1 || r.Revision < 1 {
		return fmt.Errorf("Version de demande ou révision invalide.")
	}
	t, err := w.task(r.Task)
	if err != nil {
		return err
	}
	if t.Status == "running" || (t.IndependentReview != nil && t.IndependentReview.State == "running") {
		return fmt.Errorf("Attendez la fin de la tentative ou de la revue avant de changer le modèle.")
	}
	if r.Inherit {
		t.ModelSelection = nil
		return nil
	}
	providers, err := s.providers()
	if err != nil {
		return err
	}
	p, ok := providers.Providers[r.Provider]
	if !ok {
		return fmt.Errorf("fournisseur inconnu : %s", r.Provider)
	}
	if p.APIConnectionID != "" {
		return fmt.Errorf("Choisissez un fournisseur avec outils pour cette tâche.")
	}
	_, route, err := resolveModel(p, r.Level, "work")
	if err != nil {
		return err
	}
	if route == nil {
		return fmt.Errorf("Adaptateur de modèles indisponible pour cet exécutable.")
	}
	if r.PolicyHash == "" || r.PolicyHash != route.PolicyHash {
		return fmt.Errorf("Politique de modèle modifiée ; examiner à nouveau le choix.")
	}
	t.ModelSelection = &TaskModel{Provider: r.Provider, Route: *route, Actor: operatorIdentity(), At: now()}
	return nil
}
func (s *Store) previewTaskModel(work string, r TaskModelRequest) (any, error) {
	w, err := s.get(work)
	if err != nil {
		return nil, err
	}
	if w.Revision != r.Revision {
		return nil, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	t, err := w.task(r.Task)
	if err != nil {
		return nil, err
	}
	if err = taskModelIdle(s.db, w.ID, r.Task); err != nil {
		return nil, err
	}
	before := t.ModelSelection
	if err = s.prepareTaskModel(&w, r); err != nil {
		return nil, err
	}
	return map[string]any{"before": before, "after": t.ModelSelection, "revision": w.Revision}, nil
}
func (s *Store) configureTaskModel(work string, r TaskModelRequest) (Work, error) {
	raw, err := json.Marshal(r)
	if err != nil {
		return Work{}, err
	}
	return s.mutateWithHook(work, "task.model", r.EventID, r.Revision, raw, func(w *Work) error { return s.prepareTaskModel(w, r) }, func(tx *sql.Tx, w *Work) error { return taskModelIdle(tx, w.ID, r.Task) })
}
func (s *Store) taskModelCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 3 {
		return fmt.Errorf("swarm task-model show|preview|apply WORK --input request.json")
	}
	if pos[1] == "show" {
		w, e := s.get(pos[2])
		if e != nil {
			return e
		}
		raw, _ := json.Marshal(w)
		var result map[string]any
		json.Unmarshal(raw, &result)
		providers, e := s.providers()
		if e != nil {
			return e
		}
		options := map[string]map[string]*ModelRoute{}
		for id, p := range providers.Providers {
			if p.APIConnectionID != "" {
				continue
			}
			options[id] = map[string]*ModelRoute{}
			for _, level := range []string{"auto", "simple", "standard", "exigeant"} {
				_, route, err := resolveModel(p, level, "work")
				if err == nil && route != nil {
					options[id][level] = route
				}
			}
		}
		result["available_models"] = options
		return printJSON(out, result)
	}
	if pos[1] != "preview" && pos[1] != "apply" {
		return fmt.Errorf("action inconnue")
	}
	raw, e := readInput(input)
	if e != nil {
		return e
	}
	var r TaskModelRequest
	if e = strict(raw, &r); e != nil {
		return e
	}
	if pos[1] == "preview" {
		v, e := s.previewTaskModel(pos[2], r)
		if e != nil {
			return e
		}
		return printJSON(out, v)
	}
	v, e := s.configureTaskModel(pos[2], r)
	if e != nil {
		return e
	}
	return printJSON(out, v)
}

func taskModelIdle(db interface{ QueryRow(string, ...any) *sql.Row }, work, task string) error {
	var count int
	if err := db.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')", work, task).Scan(&count); err != nil {
		return err
	}
	if count > 0 {
		return fmt.Errorf("Attendez la fin de la tentative ou de la revue avant de changer le modèle.")
	}
	return nil
}
