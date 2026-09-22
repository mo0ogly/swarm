//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

// IntegrationRetry retains the original failure and consumes a single retry
// for a result/base pair. It never grants another producer or a review verdict.
type IntegrationRetry struct {
	Event          string `json:"event_id"`
	RequestDigest  string `json:"request_sha256"`
	Agent          string `json:"agent_id"`
	Attempt        string `json:"attempt_id"`
	Result         string `json:"result_commit"`
	Previous       string `json:"failed_base"`
	Candidate      string `json:"expected_candidate"`
	Contract       string `json:"contract_sha256"`
	Failure        string `json:"original_failure"`
	Evidence       string `json:"failure_evidence"`
	EvidenceDigest string `json:"evidence_sha256"`
	Legacy         bool   `json:"legacy_missing_output"`
	Reason         string `json:"reason"`
	At             string `json:"at"`
}

var controlFailurePattern = regexp.MustCompile(`^Contrôle ([^ ]+) de ([^ ]+) en échec : .+ \(empreinte ([0-9a-f]{64})\)(?: ; diagnostic : (.+))?$`)

// Failure provenance is engine-owned. Historical failures have no output
// artifact: bind them to the durable conflict and preceding publication event,
// explicitly retaining that limitation instead of inventing lost evidence.
func (s *Store) integrationFailureBase(w Work, a Agent, item ManagedAttempt) (string, string, string, bool, error) {
	parts := controlFailurePattern.FindStringSubmatch(item.Detail)
	if parts == nil {
		return "", "", "", false, fmt.Errorf("seul un contrôle d’intégration en échec peut être repris")
	}
	checked, e := w.task(parts[2])
	if e != nil || checked.ValidationPolicy == nil {
		return "", "", "", false, fmt.Errorf("contrôle d’origine absent")
	}
	var control *ValidationControl
	for i := range checked.ValidationPolicy.Controls {
		if checked.ValidationPolicy.Controls[i].ID == parts[1] {
			control = &checked.ValidationPolicy.Controls[i]
		}
	}
	if control == nil {
		return "", "", "", false, fmt.Errorf("contrôle d’origine remplacé")
	}
	if parts[4] != "" {
		path, e := localFile(s.root, parts[4])
		if e != nil {
			return "", "", "", false, e
		}
		raw, e := os.ReadFile(path)
		if e != nil {
			return "", "", "", false, e
		}
		var d managedControlFailure
		if e = json.Unmarshal(raw, &d); e != nil {
			return "", "", "", false, e
		}
		expected, _ := json.Marshal(control)
		actual, _ := json.Marshal(d.Control)
		if d.Version != 1 || d.Work != w.ID || d.Task != checked.ID || d.Producer != a.ID || d.Attempt != a.Attempt || d.Result.Passed || !d.Result.Executed || d.Result.OutputHash != parts[3] || hash(d.Output) != parts[3] || string(actual) != string(expected) || d.Previous == "" {
			return "", "", "", false, fmt.Errorf("diagnostic d’échec incohérent ou contrôle modifié")
		}
		bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
		parent, err := managedGit(bare, "rev-parse", d.Candidate+"^")
		if err != nil || parent != d.Previous {
			return "", "", "", false, fmt.Errorf("base du diagnostic différente du candidat conservé")
		}
		retained, err := managedGit(bare, "rev-parse", "refs/swarm/failed-controls/"+d.Candidate)
		if err != nil || retained != d.Candidate {
			return "", "", "", false, fmt.Errorf("candidat refusé non conservé")
		}
		return d.Previous, parts[4], hash(raw), false, nil
	}
	events, e := s.events(w.ID)
	if e != nil {
		return "", "", "", true, e
	}
	previous := ""
	for _, event := range events {
		if event.Kind == "managed.integrated" {
			var receipt struct {
				Candidate string `json:"candidate_commit"`
			}
			if json.Unmarshal(event.Payload, &receipt) == nil {
				previous = receipt.Candidate
			}
		}
		if event.Kind == "managed.conflict" {
			var failure struct {
				Agent  string `json:"agent"`
				Reason string `json:"reason"`
			}
			if json.Unmarshal(event.Payload, &failure) == nil && failure.Agent == a.ID && failure.Reason == item.Detail && previous != "" {
				return previous, "event:" + event.ID, hash(event.Payload), true, nil
			}
		}
	}
	return "", "", "", true, fmt.Errorf("base du contrôle historique non démontrée ; aucune reprise implicite")
}

func (s *Store) retryManagedIntegration(work string, r PlanningRequest) (Work, error) {
	if len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 || r.Agent == "" || r.Attempt == "" || r.ResultCommit == "" || r.ExpectedCandidate == "" {
		return Work{}, fmt.Errorf("agent, tentative, résultat, base attendue et motif explicite requis")
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
	for _, prior := range t.IntegrationRetries {
		if prior.Event == r.EventID {
			if prior.RequestDigest != hash(raw) {
				return Work{}, fmt.Errorf("événement de reprise réutilisé avec une autre demande")
			}
			return w, nil
		}
	}
	if w.Revision != r.Revision || w.Planning == nil || w.Planning.Repository == nil || w.Planning.Reviewer == nil || t.Status != "blocked" || !currentTaskAttempt(t, r.Attempt) || t.IndependentReview != nil || t.BatchReviewResume != nil || t.RecoveredResult != nil {
		return Work{}, fmt.Errorf("reprise réservée au dernier résultat terminé, sans avis ni réparation acquis")
	}
	if w.Planning.Reviewer.Calls >= w.Planning.Reviewer.MaxCalls || w.Planning.Reviewer.Failure != "" {
		return Work{}, fmt.Errorf("vérificateur indisponible ou budget épuisé")
	}
	if e = s.providerCooldownGuard(w.Planning.Reviewer.Provider); e != nil {
		return Work{}, e
	}
	a, e := s.agent(r.Agent)
	if e != nil {
		return Work{}, e
	}
	if a.WorkID != work || a.TaskID != t.ID || a.Attempt != r.Attempt || a.Status != "completed" || a.Ended == "" || (a.Child != 0 && (a.Host != hostIdentity() || processStamp(a.Child) == a.ChildStamp)) {
		return Work{}, fmt.Errorf("production terminée et fin du processus confirmée requises")
	}
	item, e := s.managedAttempt(a.ID)
	if e != nil {
		return Work{}, e
	}
	if item.Work != work || item.Task != t.ID || item.State != "conflict" || item.Result != r.ResultCommit || item.Detail != t.Blocker {
		return Work{}, fmt.Errorf("résultat ou échec remplacé")
	}
	repo := w.Planning.Repository
	if repo.Candidate != r.ExpectedCandidate {
		return Work{}, fmt.Errorf("base attendue remplacée")
	}
	for _, prior := range t.IntegrationRetries {
		if prior.Result == item.Result && prior.Candidate == repo.Candidate {
			return Work{}, fmt.Errorf("reprise déjà consommée pour ce résultat et cette base")
		}
	}
	// No old review checkpoint/receipt may be overwritten by this recovery.
	for _, name := range []string{"candidate.json", "receipt.json"} {
		_, err := os.Stat(filepath.Join(repo.Storage, "proofs", managedProofKey(t, a), name))
		if err == nil {
			return Work{}, fmt.Errorf("dossier de revue existant ; reprise d’intégration refusée")
		}
		if !os.IsNotExist(err) {
			return Work{}, err
		}
	}
	previous, evidence, digest, legacy, e := s.integrationFailureBase(w, a, item)
	if e != nil {
		return Work{}, e
	}
	if previous == repo.Candidate {
		return Work{}, fmt.Errorf("base validée inchangée ; aucune répétition du contrôle")
	}
	bare := filepath.Join(repo.Storage, "repository.git")
	if _, e = managedGit(bare, "merge-base", "--is-ancestor", previous, repo.Candidate); e != nil {
		return Work{}, fmt.Errorf("nouvelle base non descendante de la base en échec")
	}
	if _, e = managedGit(bare, "cat-file", "-e", item.Result+"^{commit}"); e != nil {
		return Work{}, e
	}
	retry := IntegrationRetry{r.EventID, hash(raw), a.ID, a.Attempt, item.Result, previous, repo.Candidate, managedReviewContract(w, t.ID), item.Detail, evidence, digest, legacy, r.Reason, now()}
	return s.mutateWithHook(work, "managed.retry-integration", r.EventID, r.Revision, raw, func(current *Work) error {
		task, err := current.task(r.Task)
		if err != nil {
			return err
		}
		if current.Planning.Repository.Candidate != retry.Candidate || !currentTaskAttempt(task, a.Attempt) || managedReviewContract(*current, t.ID) != retry.Contract {
			return fmt.Errorf("base, tentative ou contrat modifié")
		}
		task.IntegrationRetries = append(task.IntegrationRetries, retry)
		task.Next = "Reprise du résultat conservé : contrôles cumulés puis nouvelle revue indépendante."
		return nil
	}, func(tx *sql.Tx, _ *Work) error {
		if err := automaticValidationGuard(tx, work); err != nil {
			return err
		}
		var status string
		if err := tx.QueryRow("SELECT status FROM agents WHERE id=?", a.ID).Scan(&status); err != nil {
			return err
		}
		if status != "completed" {
			return fmt.Errorf("état producteur modifié")
		}
		result, err := tx.Exec("UPDATE managed_attempts SET state='integrating',detail='' WHERE agent_id=? AND work_id=? AND task_id=? AND state='conflict' AND result_commit=? AND detail=?", a.ID, work, t.ID, item.Result, item.Detail)
		if err != nil {
			return err
		}
		n, err := result.RowsAffected()
		if err != nil {
			return err
		}
		if n != 1 {
			return fmt.Errorf("reprise déjà réservée ou résultat remplacé")
		}
		return nil
	})
}

func integrationRetryGuard(w Work, t *Task, a Agent, item ManagedAttempt) error {
	for i := len(t.IntegrationRetries) - 1; i >= 0; i-- {
		r := t.IntegrationRetries[i]
		if r.Agent == a.ID && r.Attempt == a.Attempt && r.Result == item.Result {
			if r.Candidate != w.Planning.Repository.Candidate || r.Contract != managedReviewContract(w, t.ID) {
				return fmt.Errorf("base ou contrat modifié depuis la reprise réservée ; résultat conservé")
			}
			return nil
		}
	}
	return nil
}
