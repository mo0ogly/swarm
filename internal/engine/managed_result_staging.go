//go:build linux

package engine

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// Staging transports a stopped worker's immutable result while another candidate
// is reviewed. It does not prepare controls, call a reviewer or publish a task.
func (s *Store) stageManagedResult(a Agent) error {
	unlock, err := managedLock(s.root, a.WorkID)
	if err != nil {
		return err
	}
	defer unlock()
	releaseAttempt, err := managedReviewOwnershipLock(s.root, a.WorkID, a.ID)
	if err != nil {
		return err
	}
	defer releaseAttempt()
	w, t, item, proceed, err := s.managedIntegrationInput(a)
	if err != nil || !proceed || item.Result != "" {
		return err
	}
	repo, err := s.managedRepository(w)
	if err != nil {
		return err
	}
	_, _, err = s.retainManagedResult(a, t, item, repo)
	return err
}

// Call under both Git and attempt ownership locks. Eligibility and authorization
// are identical whether the lane is available or only result staging is allowed.
func (s *Store) managedIntegrationInput(a Agent) (Work, *Task, ManagedAttempt, bool, error) {
	item, err := s.managedAttempt(a.ID)
	if err != nil {
		return Work{}, nil, item, false, err
	}
	stored, err := s.agent(a.ID)
	if err != nil {
		return Work{}, nil, item, false, err
	}
	if stored.WorkID != a.WorkID || stored.TaskID != a.TaskID || stored.Attempt != a.Attempt || stored.Status != a.Status || item.Work != a.WorkID || item.Task != a.TaskID || item.Path != stored.CWD {
		return Work{}, nil, item, false, fmt.Errorf("identité ou copie de tentative remplacée")
	}
	if item.State == "conflict" && item.Detail != managedReviewContextTooLarge {
		return Work{}, nil, item, false, s.managedFailure(a, item.Detail)
	}
	if item.State == "integrated" {
		return Work{}, nil, item, false, nil
	}
	w, err := s.get(a.WorkID)
	if err != nil {
		return Work{}, nil, item, false, err
	}
	// A public retry may be armed during maintenance. Preserve that intent
	// until both pause controls are clear; a deliberate pause is neither a
	// candidate defect nor permission to start controls or a paid review.
	if s.paused(a.WorkID) || (w.Planning != nil && w.Planning.Paused) {
		return Work{}, nil, item, false, nil
	}
	t, err := w.task(a.TaskID)
	if err != nil {
		return Work{}, nil, item, false, err
	}
	if err = historicalRequalificationGuard(w, t, a); err != nil {
		return Work{}, nil, item, false, s.managedFailure(a, err.Error())
	}
	if err = s.integrationRetryGuard(w, t, a, item); err != nil {
		return Work{}, nil, item, false, s.managedFailure(a, err.Error())
	}
	// An explicit replay may resume an unpaid preflight, or finish publication
	// after a persisted pass. reviewManagedCandidate rechecks every binding and
	// proof before reusing a pass; it never makes a second paid call in that case.
	if item.State == "conflict" && (!recoveredResultMatches(t, a, item) || (t.IndependentReview != nil && t.IndependentReview.Attempt == a.Attempt && t.IndependentReview.State != "passed")) {
		return Work{}, nil, item, false, s.managedFailure(a, item.Detail)
	}
	if !currentTaskAttempt(t, a.Attempt) || (a.Status != "completed" && !recoveredResultMatches(t, a, item)) {
		return Work{}, nil, item, false, fmt.Errorf("tentative non intégrable")
	}
	if ok, reason := s.automaticValidationAuthorized(w.ID); !ok {
		return Work{}, nil, item, false, fmt.Errorf("intégration suspendue : %s", reason)
	}
	if t.ValidationPolicy == nil || t.ValidationPolicy.Mode != "automatic" {
		return Work{}, nil, item, false, s.managedFailure(a, "Contrôles non autorisés pour cette tâche ; aucune validation supposée")
	}
	if err = validationPolicyCoversTask(*t.ValidationPolicy, t); err != nil {
		return Work{}, nil, item, false, err
	}
	return w, t, item, true, nil
}

// Persist the same immutable result in both paths. Once retained, always read
// its report from Git; workspace changes on replay cannot replace the result.
func (s *Store) retainManagedResult(a Agent, t *Task, item ManagedAttempt, repo *ManagedRepository) (ManagedAttempt, []byte, error) {
	bare := filepath.Join(repo.Storage, "repository.git")
	reportName := filepath.ToSlash(filepath.Join(repo.Subdir, "docs", t.ID+".md"))
	var report []byte
	if item.Result == "" {
		reportPath, err := localFile(item.Path, reportName)
		if err != nil {
			return item, nil, s.managedFailure(a, "Rapport de tentative absent : "+err.Error())
		}
		report, err = os.ReadFile(reportPath)
		if err != nil || len(report) == 0 || len(report) > 1<<20 {
			return item, nil, s.managedFailure(a, "Rapport de tentative vide, illisible ou trop grand")
		}
		if old, e := managedGit(item.Path, "show", item.Base+":"+reportName); e == nil && hash([]byte(old)) == hash([]byte(strings.TrimSpace(string(report)))) {
			return item, nil, s.managedFailure(a, "Rapport identique à la base ; nouvelle preuve de tentative requise")
		}
		if _, err = managedGit(item.Path, "add", "-A", "--", "."); err != nil {
			return item, nil, err
		}
		tree, err := managedGit(item.Path, "write-tree")
		if err != nil {
			return item, nil, err
		}
		result, err := managedGit(item.Path, "commit-tree", tree, "-p", item.Base, "-m", "Résultat "+a.ID)
		if err != nil {
			return item, nil, err
		}
		if _, err = managedGit(bare, "fetch", "--no-tags", item.Path, result); err != nil {
			return item, nil, err
		}
		if _, err = managedGit(bare, "update-ref", "refs/swarm/attempts/"+a.ID, result); err != nil {
			return item, nil, err
		}
		if _, err = s.db.Exec("UPDATE managed_attempts SET state='integrating',result_commit=? WHERE agent_id=?", result, a.ID); err != nil {
			return item, nil, err
		}
		item.Result = result
		item.State = "integrating"
		if committed, e := managedGit(bare, "show", result+":"+reportName); e != nil || committed != strings.TrimSpace(string(report)) {
			return item, nil, s.managedFailure(a, "Rapport absent de la révision Git ; vérifier les fichiers ignorés")
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
			return item, nil, s.managedFailure(a, "Intégration échouée : révision "+item.Result+" absente de la copie gérée ("+e.Error()+")")
		}
		report = []byte(committed)
	}
	return item, report, nil
}
