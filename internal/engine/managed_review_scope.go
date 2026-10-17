//go:build linux

package engine

import "strings"

type managedReviewScopeTask struct {
	Task        string   `json:"task"`
	Title       string   `json:"title"`
	Deliverable string   `json:"deliverable"`
	Criteria    []string `json:"criteria"`
}

type managedReviewInspectionScope struct {
	Version           int                      `json:"version"`
	Tasks             []managedReviewScopeTask `json:"tasks"`
	BaselineCandidate string                   `json:"accepted_baseline_candidate,omitempty"`
	BaselineVersion   int                      `json:"accepted_baseline_version,omitempty"`
	DiffDigest        string                   `json:"canonical_diff_sha256"`
	ChangedSections   []string                 `json:"changed_sections"`
}

func managedFragmentScope(c managedReviewContext) *managedReviewInspectionScope {
	if c.InspectionScopeVersion != 1 {
		return nil
	}
	out := &managedReviewInspectionScope{Version: 1, DiffDigest: hash([]byte(c.Diff)), ChangedSections: []string{}}
	for _, task := range c.Tasks {
		out.Tasks = append(out.Tasks, managedReviewScopeTask{task.Task, task.Title, task.Deliverable, append([]string(nil), task.Criteria...)})
	}
	if c.Baseline != nil {
		out.BaselineCandidate, out.BaselineVersion = c.Baseline.Candidate, c.Baseline.Version
	}
	for _, line := range strings.Split(c.Diff, "\n") {
		if strings.HasPrefix(line, "diff --git ") {
			out.ChangedSections = append(out.ChangedSections, line)
		}
	}
	return out
}

const managedFragmentScopeInstructions = `
PÉRIMÈTRE CANONIQUE : scope reproduit les tâches, critères et sections modifiées du contexte dont context_sha256 fixe l'identité ; ces données de cadrage sont vérifiées par le moteur, même sans la pièce context dans ce paquet. Elles ne constituent aucune preuve d'implémentation. Inspecter chaque pièce et ses interactions avec ces critères et changements. Rattacher toute demande manquante à un critère ou une interaction concrète, sans réauditer un composant inchangé pour sa seule présence. La base indique une acceptation antérieure, jamais un verdict sur ce candidat ; ses avis originaux restent dans le contexte complet. Une incertitude réelle reste unknown. Le décideur final réexamine toutes les questions avec les originaux nécessaires.
`

const managedFragmentScopeDecisionInstructions = `
PERTINENCE DES QUESTIONS : les demandes d'inspection sont des questions attribuées à résoudre, pas de nouvelles exigences ajoutées au contrat. Pour chacune, décider si les critères et interactions courants nécessitent la preuve demandée. Une demande hors périmètre ou circulaire (exiger la décision finale courante avant de la rendre) peut être résolue avec une citation littérale du contrat courant et une justification précise ; conserver sa résolution et son identité. Cela ne prouve aucun critère d'implémentation. Sélectionner et examiner tout original réellement nécessaire, y compris une dépendance inchangée affectée ; conserver unknown si cette preuve manque, fail si le défaut est démontré. Aucune question ne peut être omise ni un défaut écarté par simple déclaration de périmètre.
`
