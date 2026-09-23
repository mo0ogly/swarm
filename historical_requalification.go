//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"path/filepath"
	"strings"
)

// Previous evidence remains attached, even when current validity is withdrawn.
type HistoricalRequalification struct {
	Event      string               `json:"event_id"`
	Digest     string               `json:"request_sha256"`
	ProofKey   string               `json:"proof_key"`
	Agent      string               `json:"agent_id"`
	Attempt    string               `json:"attempt_id"`
	Result     string               `json:"result_commit"`
	Candidate  string               `json:"previous_candidate"`
	Contract   string               `json:"contract_sha256"`
	Status     string               `json:"previous_status"`
	Gate       *GateRecord          `json:"previous_gate,omitempty"`
	Validation *AutomaticValidation `json:"previous_validation,omitempty"`
	Review     *IndependentReview   `json:"previous_review,omitempty"`
	Reason     string               `json:"reason"`
	At         string               `json:"at"`
}

func historicalRequalificationGuard(w Work, t *Task, a Agent) error {
	if len(t.Requalifications) == 0 {
		return nil
	}
	r := t.Requalifications[len(t.Requalifications)-1]
	if r.Agent != a.ID || r.Attempt != a.Attempt {
		return nil
	}
	if w.Planning.Repository.Candidate != r.Candidate || managedReviewContract(w, t.ID) != r.Contract {
		return fmt.Errorf("candidat ou contrat modifié depuis la demande de requalification")
	}
	return nil
}

func (s *Store) requalifyHistorical(work string, r PlanningRequest) (Work, error) {
	if r.Agent == "" || r.Attempt == "" || r.ResultCommit == "" || r.ExpectedCandidate == "" || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return Work{}, fmt.Errorf("agent, tentative, résultat, candidat et motif explicite requis")
	}
	unlock, e := managedLock(s.root, work)
	if e != nil {
		return Work{}, e
	}
	defer unlock()
	w, e := s.get(work)
	if e != nil {
		return Work{}, e
	}
	t, e := w.task(r.Task)
	if e != nil {
		return Work{}, e
	}
	raw, _ := json.Marshal(r)
	for _, old := range t.Requalifications {
		if old.Event == r.EventID {
			if old.Digest != hash(raw) {
				return Work{}, fmt.Errorf("événement réutilisé avec une demande différente")
			}
			return w, nil
		}
	}
	if w.Revision != r.Revision || w.Planning == nil || w.Planning.Repository == nil || t.Status != "accepted" || t.IndependentReview != nil || t.BatchReviewResume != nil || t.RecoveredResult != nil || !currentTaskAttempt(t, r.Attempt) || len(t.Requalifications) > 0 {
		return Work{}, fmt.Errorf("ancienne acceptation sans avis indépendant et tentative inchangée requises")
	}
	for _, scope := range w.Planning.Scopes {
		if scope.Holder != "" {
			return Work{}, fmt.Errorf("décision de planification en cours")
		}
	}
	for _, task := range w.Tasks {
		if task.IndependentReview != nil && task.IndependentReview.State == "running" {
			return Work{}, fmt.Errorf("revue indépendante en cours")
		}
	}
	if e = s.reviewerAvailable(w); e != nil {
		return Work{}, e
	}
	cfg := w.Planning.Reviewer
	if cfg.Calls >= cfg.MaxCalls || cfg.Failure != "" {
		return Work{}, fmt.Errorf("vérificateur indisponible ou budget épuisé")
	}
	if e = s.providerCooldownGuard(cfg.Provider); e != nil {
		return Work{}, e
	}
	if t.ValidationPolicy == nil || t.ValidationPolicy.Mode != "automatic" {
		return Work{}, fmt.Errorf("contrôles automatiques explicites requis")
	}
	if e = validationPolicyCoversTask(*t.ValidationPolicy, t); e != nil {
		return Work{}, e
	}
	a, e := s.agent(r.Agent)
	if e != nil {
		return Work{}, e
	}
	if a.WorkID != work || a.TaskID != t.ID || a.Attempt != r.Attempt || a.Status != "completed" || a.Ended == "" || (a.Child != 0 && (a.Host != hostIdentity() || processStamp(a.Child) == a.ChildStamp)) {
		return Work{}, fmt.Errorf("production terminée attribuable requise")
	}
	item, e := s.managedAttempt(a.ID)
	if e != nil {
		return Work{}, e
	}
	repo := w.Planning.Repository
	if item.Work != work || item.Task != t.ID || item.State != "integrated" || item.Result != r.ResultCommit || repo.Candidate != r.ExpectedCandidate {
		return Work{}, fmt.Errorf("résultat intégré ou candidat remplacé")
	}
	bare := filepath.Join(repo.Storage, "repository.git")
	if _, e = managedGit(bare, "cat-file", "-e", item.Result+"^{commit}"); e != nil {
		return Work{}, e
	}
	if _, e = managedGit(bare, "cat-file", "-e", repo.Candidate+"^{commit}"); e != nil {
		return Work{}, e
	}
	record := HistoricalRequalification{Event: r.EventID, Digest: hash(raw), ProofKey: "requalification-" + hash(raw)[:32], Agent: a.ID, Attempt: a.Attempt, Result: item.Result, Candidate: repo.Candidate, Status: t.Status, Gate: t.Gate, Validation: t.AutoValidation, Review: t.IndependentReview, Reason: r.Reason, At: now()}
	return s.mutateWithHook(work, "managed.requalify", r.EventID, r.Revision, raw, func(current *Work) error {
		task, err := current.task(r.Task)
		if err != nil {
			return err
		}
		task.Status = "blocked"
		task.Blocker = "Requalification demandée : nouveaux contrôles et revue indépendante requis"
		task.Next = "Le conducteur reprend le candidat conservé sans nouvel exécutant."
		task.Gate = nil
		task.AutoValidation = nil
		record.Contract = managedReviewContract(*current, task.ID)
		task.Requalifications = append(task.Requalifications, record)
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
		if e := automaticValidationGuard(tx, work); e != nil {
			return e
		}
		var active int
		if e := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')", work).Scan(&active); e != nil {
			return e
		}
		if active != 0 {
			return fmt.Errorf("agents encore actifs")
		}
		res, e := tx.Exec("UPDATE managed_attempts SET state='integrating',detail='' WHERE agent_id=? AND state='integrated' AND result_commit=?", a.ID, item.Result)
		if e != nil {
			return e
		}
		n, e := res.RowsAffected()
		if e != nil {
			return e
		}
		if n != 1 {
			return fmt.Errorf("résultat déjà réservé ou remplacé")
		}
		return nil
	})
}

// Read-only identity resolution; all eligibility checks run again on submission.
func (s *Store) historicalRequalificationRequest(work, task string) (PlanningRequest, error) {
	w, e := s.get(work)
	if e != nil {
		return PlanningRequest{}, e
	}
	t, e := w.task(task)
	if e != nil {
		return PlanningRequest{}, e
	}
	if w.Planning == nil || w.Planning.Repository == nil || t.Status != "accepted" || t.IndependentReview != nil || len(t.Attempts) == 0 {
		return PlanningRequest{}, fmt.Errorf("ancienne acceptation Git sans avis indépendant requise")
	}
	agents, e := s.agents(work)
	if e != nil {
		return PlanningRequest{}, e
	}
	for _, a := range agents {
		if a.TaskID != task || !currentTaskAttempt(t, a.Attempt) {
			continue
		}
		item, e := s.managedAttempt(a.ID)
		if e != nil {
			return PlanningRequest{}, e
		}
		if item.State != "integrated" || item.Result == "" {
			continue
		}
		return PlanningRequest{Schema: 1, Revision: w.Revision, Task: task, Agent: a.ID, Attempt: a.Attempt, ResultCommit: item.Result, ExpectedCandidate: w.Planning.Repository.Candidate}, nil
	}
	return PlanningRequest{}, fmt.Errorf("résultat intégré attribuable introuvable ; aucune preuve inventée")
}
