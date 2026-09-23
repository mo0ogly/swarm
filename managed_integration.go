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

// Publication is the SQLite candidate pointer to an immutable Git commit.
// Preparing files, merging and running checks happen before that transaction.
// A crash can leave a retained candidate, never half of a canonical checkout.
func (s *Store) integrateManagedAttempt(a Agent) error {
	unlock, err := managedLock(s.root, a.WorkID)
	if err != nil {
		return err
	}
	defer func() {
		if unlock != nil {
			unlock()
		}
	}()
	// Keep ownership of this attempt while releasing the mission Git lock for
	// inference. Another conductor must not rewrite its proofs or orphan it.
	releaseAttempt, err := managedReviewOwnershipLock(s.root, a.WorkID, a.ID)
	if err != nil {
		return err
	}
	defer releaseAttempt()
	item, err := s.managedAttempt(a.ID)
	if err != nil {
		return err
	}
	if item.State == "conflict" && item.Detail != managedReviewContextTooLarge {
		return s.managedFailure(a, item.Detail)
	}
	if item.State == "integrated" {
		return nil
	}
	w, err := s.get(a.WorkID)
	if err != nil {
		return err
	}
	t, err := w.task(a.TaskID)
	if err != nil {
		return err
	}
	if err = historicalRequalificationGuard(w, t, a); err != nil {
		return s.managedFailure(a, err.Error())
	}
	if err = integrationRetryGuard(w, t, a, item); err != nil {
		return s.managedFailure(a, err.Error())
	}
	// An explicit replay may resume an unpaid preflight, or finish publication
	// after a persisted pass. reviewManagedCandidate rechecks every binding and
	// proof before reusing a pass; it never makes a second paid call in that case.
	if item.State == "conflict" && (!recoveredResultMatches(t, a, item) || (t.IndependentReview != nil && t.IndependentReview.Attempt == a.Attempt && t.IndependentReview.State != "passed")) {
		return s.managedFailure(a, item.Detail)
	}
	if !currentTaskAttempt(t, a.Attempt) || (a.Status != "completed" && !recoveredResultMatches(t, a, item)) {
		return fmt.Errorf("tentative non intégrable")
	}
	if ok, reason := s.automaticValidationAuthorized(w.ID); !ok {
		return fmt.Errorf("intégration suspendue : %s", reason)
	}
	if t.ValidationPolicy == nil || t.ValidationPolicy.Mode != "automatic" {
		return s.managedFailure(a, "Contrôles non autorisés pour cette tâche ; aucune validation supposée")
	}
	if err = validationPolicyCoversTask(*t.ValidationPolicy, t); err != nil {
		return err
	}
	repo, err := s.managedRepository(w)
	if err != nil {
		return err
	}
	bare := filepath.Join(repo.Storage, "repository.git")
	reportName := filepath.ToSlash(filepath.Join(repo.Subdir, "docs", t.ID+".md"))
	var report []byte
	if item.Result == "" {
		reportPath, err := localFile(item.Path, reportName)
		if err != nil {
			return s.managedFailure(a, "Rapport de tentative absent : "+err.Error())
		}
		report, err = os.ReadFile(reportPath)
		if err != nil || len(report) == 0 || len(report) > 1<<20 {
			return s.managedFailure(a, "Rapport de tentative vide, illisible ou trop grand")
		}
		if old, e := managedGit(item.Path, "show", item.Base+":"+reportName); e == nil && hash([]byte(old)) == hash([]byte(strings.TrimSpace(string(report)))) {
			return s.managedFailure(a, "Rapport identique à la base ; nouvelle preuve de tentative requise")
		}
		if _, err = managedGit(item.Path, "add", "-A", "--", "."); err != nil {
			return err
		}
		tree, err := managedGit(item.Path, "write-tree")
		if err != nil {
			return err
		}
		result, err := managedGit(item.Path, "commit-tree", tree, "-p", item.Base, "-m", "Résultat "+a.ID)
		if err != nil {
			return err
		}
		if _, err = managedGit(bare, "fetch", "--no-tags", item.Path, result); err != nil {
			return err
		}
		if _, err = managedGit(bare, "update-ref", "refs/swarm/attempts/"+a.ID, result); err != nil {
			return err
		}
		if _, err = s.db.Exec("UPDATE managed_attempts SET state='integrating',result_commit=? WHERE agent_id=?", result, a.ID); err != nil {
			return err
		}
		item.Result = result
		if committed, e := managedGit(bare, "show", result+":"+reportName); e != nil || committed != strings.TrimSpace(string(report)) {
			return s.managedFailure(a, "Rapport absent de la révision Git ; vérifier les fichiers ignorés")
		}
	} else {
		// A prior pass already committed and verified this report into the bare
		// repository (state left "integrating", e.g. after a crash before the
		// merge/checks completed). Trust that managed copy instead of
		// re-deriving from the ephemeral attempt workspace, which can churn
		// between retries: past this point, any failure is a real integration
		// failure, never a spurious "report missing".
		committed, e := managedGit(bare, "show", item.Result+":"+reportName)
		if e != nil {
			return s.managedFailure(a, "Intégration échouée : révision "+item.Result+" absente de la copie gérée ("+e.Error()+")")
		}
		report = []byte(committed)
	}
	proofDir := filepath.Join(repo.Storage, "proofs", managedProofKey(t, a))
	if err = os.MkdirAll(proofDir, 0700); err != nil {
		return err
	}
	relReport, _ := filepath.Rel(s.root, filepath.Join(proofDir, "report.md"))
	if err = atomicWrite(filepath.Join(s.root, relReport), report); err != nil {
		return err
	}
	if _, e := s.managedDelivery(w, t, a, item.Result); e != nil {
		return s.managedFailure(a, e.Error())
	}
	candidate, results, err := s.preparedManagedCandidate(w, t, a, item, repo)
	if err != nil {
		return s.managedFailure(a, err.Error())
	}
	receipt := map[string]any{"agent_id": a.ID, "attempt_id": a.Attempt, "base_commit": item.Base, "previous_candidate": repo.Candidate, "candidate_commit": candidate, "controls": results, "controller": validationController}
	if recoveredResultMatches(t, a, item) {
		receipt["external_repair"] = t.RecoveredResult
	}
	raw, _ := json.MarshalIndent(receipt, "", "  ")
	relReceipt, _ := filepath.Rel(s.root, filepath.Join(proofDir, "receipt.json"))
	if err = atomicWrite(filepath.Join(s.root, relReceipt), raw); err != nil {
		return err
	}
	// All reviewed Git objects are immutable. Publication still requires the
	// Git lock and rechecks the candidate/attempt and all review bindings.
	unlock()
	unlock = nil
	reviewErr := s.reviewManagedCandidate(w, a, candidate, filepath.ToSlash(relReceipt), raw)
	unlock, err = managedLock(s.root, a.WorkID)
	if err != nil {
		return err
	} // Durable verdict remains reusable; no failure mutation.
	if err = reviewErr; err != nil {
		if commandFailure(err).Code == "provider_cooldown" {
			return err
		}
		return s.managedFailure(a, err.Error())
	}
	artifacts := map[string]string{filepath.ToSlash(relReceipt): hash(raw), filepath.ToSlash(relReport): hash(report)}
	// Fetch current revision to tolerate unrelated planning activity. The candidate
	// and exact attempt are still compared inside the transaction.
	current, err := s.get(w.ID)
	if err != nil {
		return err
	}
	_, err = s.mutateWithHook(w.ID, "managed.integrated", "integrated-"+managedProofKey(t, a), current.Revision, raw, func(current *Work) error {
		task, e := current.task(a.TaskID)
		if e != nil {
			return e
		}
		if !currentTaskAttempt(task, a.Attempt) || current.Planning.Repository.Candidate != repo.Candidate {
			return fmt.Errorf("révision candidate ou tentative remplacée")
		}
		reviews, e := s.managedReviewsForPublication(current, a, candidate, filepath.ToSlash(relReceipt), raw)
		if e != nil {
			return e
		}
		for id, controls := range results {
			target, e := current.task(id)
			if e != nil {
				return e
			}
			original, _ := w.task(id)
			if target.ValidationPolicy == nil || validationPolicyDigest(*target.ValidationPolicy) != validationPolicyDigest(*original.ValidationPolicy) {
				return fmt.Errorf("politique modifiée pendant les contrôles")
			}
			checks := []map[string]any{}
			observations := []map[string]any{}
			for _, control := range controls {
				checks = append(checks, map[string]any{"id": control.ID, "domain": "automation", "mandatory": true, "gate": "delivery", "penalty": 100, "max_penalty": 100, "severity": "major"})
				observations = append(observations, map[string]any{"id": control.ID, "status": "PASS", "count": 0, "evidence": []string{filepath.ToSlash(relReceipt), filepath.ToSlash(relReport)}})
			}
			doc, _ := json.Marshal(map[string]any{"method_version": "2", "scope_id": id, "artifacts": artifacts, "domains": map[string]int{"automation": 1}, "checks": checks, "results": observations})
			evaluation, e := evaluate(doc, s.root, "delivery")
			if e != nil {
				return e
			}
			if !evaluation.Ship {
				return fmt.Errorf("preuve d’intégration refusée")
			}
			target.Gate = &GateRecord{Name: "Contrôles sur révision intégrée", Document: doc, Evaluation: evaluation, At: now()}
			attempt := a.Attempt
			if len(target.Attempts) > 0 {
				attempt = target.Attempts[len(target.Attempts)-1].ID
			}
			producer := a.ID
			deliveryVersion := a.DeliveryVersion
			if id != a.TaskID && target.AutoValidation != nil {
				producer = target.AutoValidation.Producer
				deliveryVersion = target.AutoValidation.DeliveryVersion
			}
			target.AutoValidation = &AutomaticValidation{DeliveryVersion: deliveryVersion, Attempt: attempt, CandidateSHA: candidate, Producer: producer, Controller: validationController, PolicyDigest: validationPolicyDigest(*target.ValidationPolicy), Policy: *target.ValidationPolicy, Artifacts: artifacts, Controls: controls, Receipt: filepath.ToSlash(relReceipt), State: "accepted", Reason: "Révision intégrée vérifiée : " + candidate, At: now()}
			if review, ok := reviews[id]; ok {
				target.IndependentReview = &review
			}
			target.Status = "accepted"
			target.Blocker = ""
			target.Next = "Résultat intégré et vérifié : " + candidate
		}
		current.Planning.Repository.Candidate = candidate
		scope, _ := current.Planning.scope(task.ScopeID)
		ref := ExchangeArtifact{Path: filepath.ToSlash(relReport), SHA256: hash(report)}
		current.Planning.Inbox = append(current.Planning.Inbox, PlanningEvent{Handoff: &ref, ID: planningEventID(managedProofKey(t, a), "integrated"), Scope: scope.ID, Kind: "integrated", Task: task.ID, Attempt: a.Attempt, Message: "Résultat intégré ; consulter la remise complète, ses constats, écarts et limites.", Artifacts: []ExchangeArtifact{{Path: filepath.ToSlash(relReceipt), SHA256: hash(raw)}, ref}, At: now()})
		scope.State = "ready"
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
		if err := automaticValidationGuard(tx, w.ID); err != nil {
			return err
		}
		var active int
		if err := tx.QueryRow("SELECT count(*) FROM agents WHERE id=? AND status IN ('queued','starting','running','stopping')", a.ID).Scan(&active); err != nil {
			return err
		}
		if active > 0 {
			return fmt.Errorf("processus encore actif")
		}
		_, err := tx.Exec("UPDATE managed_attempts SET state='integrated',detail=? WHERE agent_id=?", candidate, a.ID)
		return err
	})
	if err != nil {
		return err
	}
	// Advisory ref is repaired from the durable pointer by every future operation.
	_, err = managedGit(bare, "update-ref", "refs/heads/swarm-result", candidate)
	return err
}

// Do not ask a planner to retry between process completion and its controller verdict.
func (s *Store) managedIntegrationPending(w Work, scope string) (bool, error) {
	rows, e := s.db.Query("SELECT m.task_id FROM managed_attempts m JOIN agents a ON a.id=m.agent_id WHERE m.work_id=? AND m.state IN ('ready','integrating') AND a.status IN ('queued','starting','running','stopping','completed')", w.ID)
	if e != nil {
		return false, e
	}
	defer rows.Close()
	for rows.Next() {
		var id string
		if e = rows.Scan(&id); e != nil {
			return false, e
		}
		t, e := w.task(id)
		if e == nil && t.ScopeID == scope {
			return true, nil
		}
	}
	return false, rows.Err()
}

func (s *Store) prepareManagedCandidate(w Work, t *Task, a Agent, item ManagedAttempt, repo *ManagedRepository) (string, map[string][]ValidationControlResult, error) {
	bare := filepath.Join(repo.Storage, "repository.git")
	temp, err := os.MkdirTemp(repo.Storage, "integration-")
	if err != nil {
		return "", nil, err
	}
	defer os.RemoveAll(temp)
	candidateDir := filepath.Join(temp, "candidate")
	if _, err = managedGit(repo.Storage, "clone", "--no-local", bare, candidateDir); err != nil {
		return "", nil, err
	}
	if _, err = managedGit(candidateDir, "checkout", "--detach", repo.Candidate); err != nil {
		return "", nil, err
	}
	if _, err = managedGit(candidateDir, "fetch", "--no-tags", bare, item.Result); err != nil {
		return "", nil, err
	}
	if _, err = managedGit(candidateDir, "merge", "--no-commit", "--no-ff", item.Result); err != nil {
		return "", nil, fmt.Errorf("%s", "Conflit Git à résoudre sur la révision courante : "+err.Error())
	}
	tree, err := managedGit(candidateDir, "write-tree")
	if err != nil {
		return "", nil, err
	}
	// Commit before checks, so any tracked mutation by a control is detected.
	candidate, err := managedGit(candidateDir, "commit-tree", tree, "-p", repo.Candidate, "-m", "Candidat vérifié "+a.ID)
	if err != nil {
		return "", nil, err
	}
	if _, err = managedGit(candidateDir, "reset", "--hard", candidate); err != nil {
		return "", nil, err
	}
	results := map[string][]ValidationControlResult{}
	for _, task := range w.Tasks {
		if task.ID != t.ID && task.Status != "accepted" {
			continue
		}
		if task.ValidationPolicy == nil || task.ValidationPolicy.Mode != "automatic" {
			return "", nil, fmt.Errorf("%s", "Un résultat antérieur exige une revue humaine avant nouvelle intégration")
		}
		for _, control := range task.ValidationPolicy.Controls {
			if ok, reason := s.automaticValidationAuthorized(w.ID); !ok {
				return "", nil, fmt.Errorf("contrôles suspendus : %s", reason)
			}
			result, output, outputBytes := runValidationControlCaptured(filepath.Join(candidateDir, repo.Subdir), control)
			results[task.ID] = append(results[task.ID], result)
			if !result.Passed {
				// Keep the exact rejected commit reachable after the temporary
				// checkout is removed. This private ref never publishes it.
				if _, err = managedGit(bare, "fetch", "--no-tags", candidateDir, candidate); err != nil {
					return "", nil, fmt.Errorf("contrôle refusé ; conservation du candidat impossible : %w", err)
				}
				if _, err = managedGit(bare, "update-ref", "refs/swarm/failed-controls/"+candidate, candidate); err != nil {
					return "", nil, err
				}
				diagnostic, err := s.preserveManagedControlFailure(w, a, task.ID, candidate, control, result, output, outputBytes)
				if err != nil {
					return "", nil, fmt.Errorf("contrôle %s de %s refusé ; conservation du diagnostic impossible : %w", control.ID, task.ID, err)
				}
				return "", nil, fmt.Errorf("Contrôle %s de %s en échec : %s (empreinte %s) ; diagnostic : %s", control.ID, task.ID, result.Summary, result.OutputHash, diagnostic)
			}
		}
	}
	if _, err = managedGit(candidateDir, "diff", "--exit-code", "HEAD", "--"); err != nil {
		return "", nil, fmt.Errorf("%s", "Un contrôle a modifié les sources suivies ; résultat non publié")
	}
	if _, err = managedGit(bare, "fetch", "--no-tags", candidateDir, candidate); err != nil {
		return "", nil, err
	}
	if _, err = managedGit(bare, "update-ref", "refs/swarm/candidates/"+managedProofKey(t, a), candidate); err != nil {
		return "", nil, err
	}
	return candidate, results, nil
}
