//go:build linux

package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
)

// This is an accepted baseline, not a verdict on the new candidate. Original
// contexts/replies remain immutable and are recursively checked through their
// hashes. Every current criterion still needs a NEW independent verdict.
type ManagedReviewBaseline struct {
	Candidate string              `json:"candidate_commit"`
	Reviews   []IndependentReview `json:"accepted_reviews"`
}

func (s *Store) incrementalManagedReviewFallback(w Work, a Agent, c managedReviewContext) (managedReviewContext, error) {
	_, workflow, e := agentWorkflow("reviewer")
	if e != nil {
		return c, e
	}
	prefix := managedReviewPrefix(workflow)
	owners, e := managedReviewSourceOwners(w, c)
	if e != nil {
		return c, e
	}
	_, e = planManagedReviewBatches(prefix, c, owners)
	// Keep every already-fitting historical context/plan byte-for-byte.
	if !errors.Is(e, errManagedReviewBatchSize) {
		return c, e
	}
	repo := w.Planning.Repository
	if repo.Candidate == repo.Base {
		return c, nil
	}
	baseline := &ManagedReviewBaseline{Candidate: repo.Candidate}
	seen := map[string]bool{}
	for i := range w.Tasks {
		task := &w.Tasks[i]
		if task.ID == a.TaskID || task.Status != "accepted" {
			continue
		}
		if e = s.independentReviewGuard(&w, task); e != nil {
			return c, fmt.Errorf("base de revue non vérifiée : %s : %w", task.ID, e)
		}
		r := task.IndependentReview
		if r == nil || r.State != "passed" || r.CandidateSHA != repo.Candidate || task.AutoValidation == nil || task.AutoValidation.PolicyDigest != validationPolicyDigest(*task.ValidationPolicy) {
			return c, fmt.Errorf("avis de base périmé : %s", task.ID)
		}
		found := false
		for _, binding := range r.ManagedTasks {
			if binding.Task == task.ID && binding.Attempt == task.Attempts[len(task.Attempts)-1].ID && binding.Contract == reviewContract(task) && binding.Policy == validationPolicyDigest(*task.ValidationPolicy) {
				found = true
			}
		}
		if !found {
			return c, fmt.Errorf("critères de base non couverts : %s", task.ID)
		}
		if !seen[r.ID] {
			baseline.Reviews = append(baseline.Reviews, *r)
			seen[r.ID] = true
		}
	}
	if len(baseline.Reviews) == 0 {
		return c, nil
	}
	bare := filepath.Join(repo.Storage, "repository.git")
	if _, e = managedGit(bare, "merge-base", "--is-ancestor", repo.Candidate, c.Candidate); e != nil {
		return c, fmt.Errorf("candidat non descendant de la base acceptée")
	}
	delta, e := managedGit(bare, "diff", "--no-ext-diff", "--no-textconv", "--full-index", "--unified=40", repo.Candidate, c.Candidate, "--")
	if e != nil {
		return c, e
	}
	next := c
	next.Diff = delta
	next.Baseline = baseline
	// Re-read all declared sources: files previously omitted as full added-file
	// duplicates may no longer occur in an incremental diff.
	next.Sources, e = managedReviewSources(w, next.Tasks, next.Candidate)
	if e != nil {
		return c, e
	}
	if e = compactManagedReviewContext(&next, bare, repo.Candidate); e != nil {
		return c, e
	}
	return next, nil
}

func sameManagedBaseline(a, b *ManagedReviewBaseline) bool {
	x, _ := json.Marshal(a)
	y, _ := json.Marshal(b)
	return string(x) == string(y)
}

func incrementalReviewPrefix(prefix string, c managedReviewContext) string {
	if c.Baseline == nil {
		return prefix
	}
	return prefix + "\nREVUE INCRÉMENTALE : accepted_baseline désigne uniquement la base précédente déjà acceptée. Le moteur a vérifié les empreintes des contextes, reçus et réponses conservés de ces avis. Le diff contient TOUS les changements de cette base vers candidate_commit ; les rapports, critères et contrôles joints concernent TOUS les résultats sur le nouveau candidat. Examiner les régressions et interactions du delta avec les tâches anciennes, avec les sources courantes jointes. Les avis antérieurs ne valent PAS avis sur le nouveau candidat. Réévaluer CHAQUE critère : pass seulement si les preuves actuelles et la base vérifiée suffisent, sinon unknown ou fail. Ne pas inventer le contenu des fichiers historiques non joints. Les citations doivent provenir du diff actuel, du rapport actuel ou des contrôles actuels, jamais d'un ancien avis seul.\n"
}
