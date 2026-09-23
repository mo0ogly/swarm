//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"path/filepath"
	"strings"
)

// External repair is explicit and is never reported as autonomous production.
// The stopped process, consumed attempt and tool budget stay unchanged.
type RecoveredResult struct {
	ProofKey       string `json:"proof_key,omitempty"`
	ReplacesResult string `json:"replaces_result,omitempty"`
	PriorReview    string `json:"prior_review_id,omitempty"`
	Event          string `json:"event_id"`
	RequestDigest  string `json:"request_sha256"`
	Agent          string `json:"agent_id"`
	Attempt        string `json:"attempt_id"`
	Result         string `json:"result_commit"`
	Tree           string `json:"tree"`
	Actor          string `json:"actor"`
	At             string `json:"at"`
	Reason         string `json:"reason"`
	ProcessStatus  string `json:"original_process_status"`
}

func recoveredResultMatches(t *Task, a Agent, item ManagedAttempt) bool {
	r := t.RecoveredResult
	return r != nil && r.Agent == a.ID && r.Attempt == a.Attempt && r.Result != "" && r.Result == item.Result && r.ProcessStatus == a.Status && (a.Status == "interrupted" || a.Status == "failed" || a.Status == "completed")
}

func (s *Store) submitRecoveredResult(work string, r PlanningRequest) (Work, error) {
	return s.recoverResult(work, r, false)
}
func managedProofKey(t *Task, a Agent) string {
	if len(t.Requalifications) > 0 {
		r := t.Requalifications[len(t.Requalifications)-1]
		if r.Agent == a.ID && r.Attempt == a.Attempt {
			return r.ProofKey
		}
	}
	if t.RecoveredResult != nil && t.RecoveredResult.Agent == a.ID && t.RecoveredResult.Attempt == a.Attempt && t.RecoveredResult.ProofKey != "" {
		return t.RecoveredResult.ProofKey
	}
	return a.ID
}
func (s *Store) recoverResult(work string, r PlanningRequest, revise bool) (Work, error) {
	if !r.ConfirmRecovery || r.Agent == "" || r.Attempt == "" || (len(r.ResultTree) != 40 && len(r.ResultTree) != 64) || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return Work{}, fmt.Errorf("confirmation, agent, tentative, arbre Git examiné et motif de réparation requis")
	}
	if err := s.storageGuard(); err != nil {
		return Work{}, err
	}
	unlock, err := managedLock(s.root, work)
	if err != nil {
		return Work{}, err
	}
	defer func() {
		if unlock != nil {
			unlock()
		}
	}()
	w, err := s.get(work)
	if err != nil {
		return Work{}, err
	}
	t, err := w.task(r.Task)
	if err != nil {
		return Work{}, err
	}
	raw, _ := json.Marshal(r)
	if prior := t.RecoveredResult; prior != nil {
		if prior.Event == r.EventID && prior.RequestDigest == hash(raw) {
			a, e := s.agent(r.Agent)
			if e != nil {
				return Work{}, e
			}
			unlock()
			unlock = nil
			if e = s.integrateManagedAttempt(a); e != nil {
				return Work{}, e
			}
			return s.get(work)
		}
		if !revise {
			return Work{}, fmt.Errorf("un résultat réparé est déjà enregistré ; aucune nouvelle soumission implicite")
		}
	}
	if revise && (t.IndependentReview == nil || t.IndependentReview.ID != r.ReviewID || t.IndependentReview.Attempt != r.Attempt || t.IndependentReview.State != "changes_requested") {
		return Work{}, fmt.Errorf("révision corrective liée au dernier refus indépendant requise")
	}
	if w.Revision != r.Revision || t.Status != "blocked" || !currentTaskAttempt(t, r.Attempt) {
		return Work{}, fmt.Errorf("révision ou tentative modifiée ; relire le résultat à soumettre")
	}
	a, err := s.agent(r.Agent)
	if err != nil {
		return Work{}, err
	}
	if a.WorkID != work || a.TaskID != t.ID || a.Attempt != r.Attempt || (a.Status != "interrupted" && a.Status != "failed" && !(revise && a.Status == "completed")) || a.Ended == "" {
		return Work{}, fmt.Errorf("une tentative arrêtée et attribuable est requise")
	}
	if !recoveryProcessEnded(a, hostIdentity(), processStamp(a.Child)) {
		return Work{}, fmt.Errorf("fin du processus non confirmée")
	}
	if err = reviewerLaunchGuard(w); err != nil {
		return Work{}, err
	}
	if ok, reason := s.automaticValidationAuthorized(work); !ok {
		return Work{}, fmt.Errorf("soumission suspendue : %s", reason)
	}
	repo, err := s.managedRepository(w)
	if err != nil {
		return Work{}, err
	}
	item, err := s.managedAttempt(a.ID)
	if err != nil {
		return Work{}, err
	}
	if item.Work != work || item.Task != t.ID || item.Agent != a.ID || (!revise && item.Result != "") || (revise && (item.Result == "" || item.State != "conflict")) || a.CWD != filepath.Join(item.Path, repo.Subdir) {
		return Work{}, fmt.Errorf("copie ou résultat déjà remis incompatible avec cette réparation")
	}
	if err = verifyManagedCopy(repo, item.Path); err != nil {
		return Work{}, err
	}
	if _, err = managedGit(item.Path, "add", "-A", "--", "."); err != nil {
		return Work{}, err
	}
	tree, err := managedGit(item.Path, "write-tree")
	if err != nil {
		return Work{}, err
	}
	if tree != r.ResultTree {
		return Work{}, fmt.Errorf("les fichiers ont changé depuis l’examen de la réparation")
	}
	if revise {
		oldTree, e := managedGit(filepath.Join(repo.Storage, "repository.git"), "rev-parse", item.Result+"^{tree}")
		if e != nil {
			return Work{}, e
		}
		if oldTree == tree {
			return Work{}, fmt.Errorf("aucune correction depuis le résultat refusé")
		}
	}
	result, err := managedGit(item.Path, "commit-tree", tree, "-p", item.Base, "-m", "Réparation externe de "+a.ID)
	if err != nil {
		return Work{}, err
	}
	bare := filepath.Join(repo.Storage, "repository.git")
	if _, err = managedGit(bare, "fetch", "--no-tags", item.Path, result); err != nil {
		return Work{}, err
	}
	// Mandatory even for a legacy worker: external repair has its own complete
	// declaration and passes the ordinary candidate controls and review below.
	a.DeliveryVersion = 1
	if _, err = s.managedDelivery(w, t, a, result); err != nil {
		return Work{}, err
	}
	reportPath := filepath.ToSlash(filepath.Join(repo.Subdir, "docs", t.ID+".md"))
	report, err := candidateReviewSource(bare, result, reportPath, 1<<20)
	if err != nil || strings.TrimSpace(report.Content) == "" {
		return Work{}, fmt.Errorf("rapport de réparation absent ou illisible")
	}
	proofKey := a.ID
	ref := "refs/swarm/attempts/" + a.ID
	if revise {
		proofKey = a.ID + "-repair-" + hash(raw)[:16]
		ref = "refs/swarm/repairs/" + proofKey
	}
	if _, err = managedGit(bare, "update-ref", ref, result); err != nil {
		return Work{}, err
	}
	eventRaw := raw
	if revise {
		eventRaw, _ = json.Marshal(map[string]any{"request": r, "previous_review": t.IndependentReview, "previous_repair": t.RecoveredResult, "previous_result": item.Result})
	}
	_, err = s.mutateWithHook(work, "managed.recovered-result", r.EventID, r.Revision, eventRaw, func(current *Work) error {
		task, e := current.task(t.ID)
		if e != nil {
			return e
		}
		if (!revise && task.RecoveredResult != nil) || (revise && (task.IndependentReview == nil || task.IndependentReview.ID != r.ReviewID || task.IndependentReview.State != "changes_requested")) || !currentTaskAttempt(task, a.Attempt) || task.Status != "blocked" {
			return fmt.Errorf("tentative remplacée ou réparation déjà soumise")
		}
		task.RecoveredResult = &RecoveredResult{Event: r.EventID, RequestDigest: hash(raw), Agent: a.ID, Attempt: a.Attempt, Result: result, Tree: tree, Actor: operatorIdentity(), At: now(), Reason: r.Reason, ProcessStatus: a.Status}
		if revise {
			task.RecoveredResult.ProofKey = proofKey
			task.RecoveredResult.ReplacesResult = item.Result
			task.RecoveredResult.PriorReview = r.ReviewID
			task.IndependentReview = nil
		}
		task.Next = "Réparation externe remise ; contrôles et revue indépendante requis."
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
		var active int
		if e := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')", work, t.ID).Scan(&active); e != nil {
			return e
		}
		if active != 0 {
			return fmt.Errorf("agent encore actif sur cette tâche")
		}
		res, e := tx.Exec("UPDATE managed_attempts SET result_commit=?,state='integrating',detail='' WHERE agent_id=? AND result_commit=?", result, a.ID, item.Result)
		if e != nil {
			return e
		}
		n, e := res.RowsAffected()
		if e != nil {
			return e
		}
		if n != 1 {
			return fmt.Errorf("résultat remplacé pendant la soumission")
		}
		return nil
	})
	if err != nil {
		return Work{}, err
	}
	unlock()
	unlock = nil
	if err = s.integrateManagedAttempt(a); err != nil {
		return Work{}, err
	}
	return s.get(work)
}

// A durable terminal record predating a reboot cannot refer to a live process
// from that old boot. Keep different hosts/namespaces and active states closed.
func recoveryProcessEnded(a Agent, currentHost, stamp string) bool {
	if a.Ended == "" || (a.Status != "completed" && a.Status != "interrupted" && a.Status != "failed") {
		return false
	}
	if a.Child == 0 {
		return true
	}
	if a.Host == currentHost {
		return a.ChildStamp != "" && stamp != a.ChildStamp
	}
	old := strings.SplitN(a.Host, ":", 3)
	current := strings.SplitN(currentHost, ":", 3)
	return len(old) == 3 && len(current) == 3 && old[0] != "" && old[0] == current[0] && old[1] != "" && current[1] != "" && old[1] != current[1] && old[2] != "" && old[2] == current[2]
}
