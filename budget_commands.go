package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"io"
)

// BudgetChange is shared by HTTP and CLI. A replay preserves counters and revision.
type BudgetChange struct {
	Schema   int    `json:"schema_version"`
	EventID  string `json:"event_id"`
	Revision int    `json:"expected_revision"`
	Budget   Budget `json:"budget"`
}
type BudgetPreview struct {
	Revision         int        `json:"revision"`
	Current          BudgetView `json:"current"`
	Proposed         Budget     `json:"proposed"`
	Remaining        float64    `json:"proposed_remaining_usd"`
	Unlimited        bool       `json:"proposed_unlimited"`
	NewLaunchAllowed bool       `json:"new_launch_budget_allowed"`
	Coverage         string     `json:"coverage"`
}

const budgetCoverage = "Estimated envelope: execution, planning, page assistance and preparation. Independent reviews have a separate call limit; their monetary cost is not included. Other launch conditions still apply. Active agents are not stopped."

func validateBudgetChange(r BudgetChange) error {
	if r.Schema != 1 {
		return fmt.Errorf("schema_version must be 1")
	}
	if r.Revision < 1 {
		return fmt.Errorf("expected_revision must be positive")
	}
	return validateBudget(r.Budget)
}
func (s *Store) previewBudget(work string, r BudgetChange) (BudgetPreview, error) {
	var p BudgetPreview
	if err := validateBudgetChange(r); err != nil {
		return p, err
	}
	w, err := s.get(work)
	if err != nil {
		return p, err
	}
	if w.Revision != r.Revision {
		return p, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	current, err := s.budget(work)
	if err != nil {
		return p, err
	}
	// Preview is advisory. Apply rechecks the revision atomically and launch
	// reservations are always recalculated inside their own transaction.
	r.Budget.Actor, r.Budget.Updated = "", ""
	remaining := r.Budget.Limit - current.Reserved - current.Estimated
	return BudgetPreview{Revision: w.Revision, Current: current, Proposed: r.Budget, Remaining: max(0, remaining), Unlimited: r.Budget.Limit == 0, NewLaunchAllowed: r.Budget.Limit == 0 || remaining >= r.Budget.Reserve, Coverage: budgetCoverage}, nil
}
func (s *Store) configureBudget(work string, r BudgetChange) (Work, error) {
	if err := validateBudgetChange(r); err != nil {
		return Work{}, err
	}
	// Identity and time belong to the engine, never to submitted metadata.
	r.Budget.Actor, r.Budget.Updated = "", ""
	raw, err := json.Marshal(r)
	if err != nil {
		return Work{}, err
	}
	return s.mutateWithHook(work, "budget.configure", r.EventID, r.Revision, raw, func(w *Work) error { return nil }, func(tx *sql.Tx, w *Work) error { return writeBudget(tx, w.ID, r.Budget) })
}
func (s *Store) budgetCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 3 {
		return fmt.Errorf("swarm budget show|preview|apply <work> [--input budget.json]")
	}
	if pos[1] == "show" {
		w, err := s.get(pos[2])
		if err != nil {
			return err
		}
		b, err := s.budget(pos[2])
		if err != nil {
			return err
		}
		return printJSON(out, map[string]any{"revision": w.Revision, "budget": b, "coverage": budgetCoverage})
	}
	if pos[1] != "preview" && pos[1] != "apply" {
		return fmt.Errorf("unknown budget action")
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	var r BudgetChange
	if err = strict(raw, &r); err != nil {
		return err
	}
	if pos[1] == "preview" {
		p, err := s.previewBudget(pos[2], r)
		if err != nil {
			return err
		}
		return printJSON(out, p)
	}
	w, err := s.configureBudget(pos[2], r)
	if err != nil {
		return err
	}
	return printJSON(out, w)
}
