//go:build linux

package engine

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"strings"
)

// This is an accepted baseline, not a verdict on the new candidate. Original
// contexts/replies remain immutable and are recursively checked through their
// hashes. Every current criterion still needs a NEW independent verdict.
const managedReviewBaselineContractVersion = 2

func supportedManagedReviewBaseline(version int) bool {
	return version == 0 || version == 1 || version == managedReviewBaselineContractVersion
}

type ManagedReviewBaseline struct {
	Version   int                 `json:"version,omitempty"`
	Candidate string              `json:"candidate_commit"`
	Reviews   []IndependentReview `json:"accepted_reviews"`
}

func (s *Store) incrementalManagedReviewFallback(w Work, a Agent, c managedReviewContext) (managedReviewContext, error) {
	_, workflow, e := s.projectAgentWorkflow("reviewer")
	if e != nil {
		return c, e
	}
	prefix := managedReviewPrefixForContext(workflow, c)
	owners, e := managedReviewSourceOwners(w, c)
	if e != nil {
		return c, e
	}
	_, e = planManagedReviewBatches(prefix, c, owners)
	if e != nil && !errors.Is(e, errManagedReviewBatchSize) {
		return c, e
	}
	repo := w.Planning.Repository
	if repo.Candidate == repo.Base {
		return c, nil
	}
	baseline := &ManagedReviewBaseline{Version: managedReviewBaselineContractVersion, Candidate: repo.Candidate}
	seen := map[string]bool{}
	for i := range w.Tasks {
		task := &w.Tasks[i]
		if task.ID == a.TaskID || task.Status != "accepted" {
			continue
		}
		if e = s.historicalReviewBaselineGuard(w, task); e != nil {
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
	delta, e := managedReviewGit(c.CitationContractVersion, bare, "diff", "--no-ext-diff", "--no-textconv", "--full-index", "--unified=40", repo.Candidate, c.Candidate, "--")
	if e != nil {
		return c, e
	}
	next := c
	next.Diff = delta
	next.Baseline = baseline
	// Re-read all declared sources: files previously omitted as full added-file
	// duplicates may no longer occur in an incremental diff.
	next.Sources, e = managedReviewSourcesByTask(w, next.Tasks, next.Candidate)
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
	if c.CitationContractVersion == 2 {
		prefix += "\n" + managedReferencedCitationGuidance + "\n"
	}
	if c.CitationContractVersion == 1 {
		prefix += managedReviewCitationConventions
	}
	if c.InspectionScopeVersion == 1 {
		prefix += managedFragmentScopeDecisionInstructions
	}
	if c.Baseline == nil {
		return prefix
	}
	result := prefix + "\nREVUE INCRÉMENTALE : accepted_baseline désigne uniquement la base précédente déjà acceptée. Le moteur a vérifié les empreintes des contextes, reçus et réponses conservés de ces avis. Le diff contient TOUS les changements de cette base vers candidate_commit ; les rapports, critères et contrôles joints concernent TOUS les résultats sur le nouveau candidat. Examiner les régressions et interactions du delta avec les tâches anciennes, avec les sources courantes jointes. Les avis antérieurs ne valent PAS avis sur le nouveau candidat. Réévaluer CHAQUE critère : pass seulement si les preuves actuelles et la base vérifiée suffisent, sinon unknown ou fail. Ne pas inventer le contenu des fichiers historiques non joints. Les citations doivent provenir du diff actuel, du rapport actuel ou des contrôles actuels, jamais d'un ancien avis seul.\n"
	if c.Baseline.Version == managedReviewBaselineContractVersion {
		result += "\nDIFF CANONIQUE ET TRANSPORT : le delta Git canonique couvre la base accepted_baseline vers candidate_commit. Dans les messages fragmentés de sélection et décision, task_contracts_reports_controls.diff est omis du transport sous la valeur vide : cela ne signifie pas un diff Git vide. Les rows de type diff dans partial_inspections recensent exhaustivement ses sections avec noms et empreintes ; leurs extraits et opinions ne remplacent pas les pièces entières. Seules les pièces demandées dans requested_original_evidence sont fournies intégralement au décideur. changes_since_observations concerne exclusivement les observations partielles réutilisées d’un ancien candidat, pas ce delta depuis la base acceptée ; son absence ne signifie pas absence de changement. Choisir les sections entières nécessaires aux interactions et critères concernés, y compris ceux des tâches anciennes. Ne pas exiger de reconstituer toute la base inchangée par la seule présence du champ diff vidé ; si une section nécessaire manque réellement, préciser laquelle et le lien au critère puis rester unknown. Aucun avis ni inventaire ne vaut preuve d’implémentation ou acceptation.\n"
	}
	return result
}

// Historical evidence may be checked against its original reviewer identity.
// This is only used when constructing a new cumulative review, never by the
// acceptance guard: every current criterion still requires a fresh verdict.
func (s *Store) historicalReviewBaselineGuard(w Work, task *Task) error {
	r := task.IndependentReview
	if r == nil || w.Planning == nil || w.Planning.Reviewer == nil || w.Planning.Reviewer.ModelSelection == nil || len(r.Batches) == 0 {
		return s.independentReviewGuardVersion(&w, task, true)
	}
	provider := strings.TrimPrefix(r.Reviewer, "reviewer://")
	if provider == r.Reviewer || provider == "" {
		return fmt.Errorf("identité du vérificateur historique absente")
	}
	p := *w.Planning
	cfg := *p.Reviewer
	cfg.Provider, cfg.ProviderDigest, cfg.ModelRoute = provider, r.BatchProviderDigest, r.ModelRoute
	if cfg.ModelRoute == nil {
		// Older batch records omitted the model route. A reconstructed default
		// is usable only if the original full batch-plan digest verifies below.
		providers, err := s.providers()
		if err != nil {
			return err
		}
		original, ok := providers.Providers[provider]
		if !ok {
			return fmt.Errorf("fournisseur historique absent")
		}
		_, route, err := resolveModel(original, "auto", "planning")
		if err != nil {
			return err
		}
		cfg.ModelRoute = route
	}
	p.Reviewer = &cfg
	w.Planning = &p
	return s.independentReviewGuardVersion(&w, task, true)
}
