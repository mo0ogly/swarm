//go:build linux

package engine

import (
	"path"
	"sort"
	"strings"
)

// This is evidence for a planner, never an approved schedule or a review verdict.
// Path-based grouping is deliberately labelled heuristic: dependencies require
// examination of the actual contracts and code before lots can run separately.
type ReviewCutOption struct {
	Kind       string `json:"kind"`
	When       string `json:"when"`
	Constraint string `json:"constraint"`
}
type ReviewLotSuggestion struct {
	Component    string   `json:"component"`
	Files        []string `json:"files"`
	Dependencies string   `json:"dependencies"`
}
type ReviewDecomposition struct {
	Version     int                   `json:"version"`
	Candidate   string                `json:"candidate_commit"`
	Required    bool                  `json:"planning_required"`
	Reasons     []string              `json:"reasons"`
	DiffBytes   int                   `json:"diff_bytes"`
	Options     []ReviewCutOption     `json:"cut_options"`
	Suggestions []ReviewLotSuggestion `json:"suggested_lots"`
	State       string                `json:"state"`
	FinalReview string                `json:"final_review"`
}

func reviewComponent(file string) string {
	switch {
	case strings.HasPrefix(file, "docs/") || strings.HasSuffix(file, ".md"):
		return "documentation"
	case strings.HasPrefix(file, "web/") || strings.HasPrefix(file, "locales/"):
		return "interface"
	case strings.HasPrefix(file, ".claude/") || strings.HasPrefix(file, ".agents/") || strings.HasPrefix(file, "tools/agent-workflows/"):
		return "methodes-agents"
	case strings.HasPrefix(file, "tests/"):
		return "recette-transversale"
	}
	name := path.Base(file)
	switch {
	case strings.HasPrefix(name, "managed_review") || strings.HasPrefix(name, "independent_review"):
		return "revue"
	case strings.HasPrefix(name, "planning") || strings.HasPrefix(name, "conductor") || strings.HasPrefix(name, "dispatcher"):
		return "coordination"
	case strings.HasPrefix(name, "managed_") || strings.HasPrefix(name, "recovered_"):
		return "integration-reprise"
	case strings.HasPrefix(name, "console_") || strings.HasSuffix(name, "_cli.go"):
		return "cli"
	case strings.HasPrefix(name, "agent") || strings.HasPrefix(name, "provider"):
		return "execution"
	default:
		return "contrats-transversaux"
	}
}
func detectReviewDecomposition(candidate string, files []string, diffBytes int) ReviewDecomposition {
	d := ReviewDecomposition{Version: 1, Candidate: candidate, DiffBytes: diffBytes, State: "proposal_only", FinalReview: "Revue des interactions et contrôles globaux sur le même candidat ; couverture de chaque fichier et critère obligatoire. Aucun lot ne vaut acceptation globale."}
	d.Options = []ReviewCutOption{
		{"requirement", "Plusieurs résultats fonctionnels dans la même remise", "Chaque critère doit avoir un responsable et des preuves."},
		{"component", "Responsabilités techniques distinctes", "Les dossiers suggèrent des lots, sans démontrer leur indépendance."},
		{"dependency", "Contrats, migrations ou consommateurs couplés", "Examiner les dépendances ; regrouper les cycles et ordonner les autres lots."},
		{"specialty", "Sécurité, concurrence ou ergonomie exigent des examens distincts", "Le recouvrement est permis ; chaque avis garde son périmètre explicite."},
		{"volume", "Un lot cohérent dépasse encore la capacité du vérificateur", "Conserver tout le contexte commun et prévoir une synthèse ; ne rien tronquer."},
	}
	if len(files) > managedScopeFiles {
		d.Reasons = append(d.Reasons, "Plus de 100 fichiers modifiés depuis le candidat accepté.")
	}
	if diffBytes > managedReviewPromptLimit {
		d.Reasons = append(d.Reasons, "Le diff seul dépasse la capacité du contexte de revue ; preuves et consignes augmentent encore ce volume.")
	}
	groups := map[string][]string{}
	for _, f := range files {
		c := reviewComponent(f)
		groups[c] = append(groups[c], f)
	}
	// Mixed components alone are common in small fixes: do not force delegation.
	if len(groups) >= 4 && len(files) >= 20 {
		d.Reasons = append(d.Reasons, "Au moins quatre responsabilités techniques dans une remise de vingt fichiers ou plus.")
	}
	d.Required = len(d.Reasons) > 0
	keys := []string{}
	for k := range groups {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		sort.Strings(groups[k])
		d.Suggestions = append(d.Suggestions, ReviewLotSuggestion{k, groups[k], "non examinées : proposition à vérifier par le sous-planificateur"})
	}
	return d
}
