//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
)

// This phase is an offer after a fresh recipe. It grants no execution budget.
type PostDelivery struct {
	Ready   bool          `json:"ready"`
	Sources []DialogueRef `json:"sources"`
}

func acceptanceSource(w Work, t Task) DialogueRef {
	body, _ := json.Marshal(struct {
		Work, Task, Contract string
		Gate                 *GateRecord
		Review               *IndependentReview
		Automatic            *AutomaticValidation
	}{w.ID, t.ID, reviewContract(&t), t.Gate, t.IndependentReview, t.AutoValidation})
	attempt := ""
	if len(t.Attempts) > 0 {
		attempt = t.Attempts[len(t.Attempts)-1].ID
	}
	return DialogueRef{Task: t.ID, Attempt: attempt, SHA256: hash(body)}
}
func (s *Store) postDelivery(w *Work) PostDelivery {
	out := PostDelivery{Sources: []DialogueRef{}}
	// A retained RETEX task belongs to the next increment, not the recipe source.
	improvement := map[string]bool{}
	for _, r := range w.Retex {
		if r.Task != "" {
			improvement[r.Task] = true
		}
	}
	memo := map[string]bool{}
	for i := range w.Tasks {
		t := &w.Tasks[i]
		if improvement[t.ID] {
			continue
		}
		if t.Status != "accepted" || !s.acceptedFreshMemo(w, t, map[string]bool{}, memo) {
			return PostDelivery{Sources: []DialogueRef{}}
		}
		out.Sources = append(out.Sources, acceptanceSource(*w, *t))
	}
	if len(out.Sources) == 0 {
		return out
	}
	if w.Planning != nil {
		root, err := w.Planning.scope("root")
		if err != nil || root.State != "closed" {
			return PostDelivery{Sources: []DialogueRef{}}
		}
	}
	out.Ready = true
	return out
}
func (s *Store) verifyAcceptanceRetexSource(w *Work, ref DialogueRef) error {
	p := s.postDelivery(w)
	if p.Ready {
		for _, source := range p.Sources {
			if source == ref {
				return nil
			}
		}
	}
	return fmt.Errorf("RETEX après recette : acceptation fraîche et source inchangée requises")
}
func (s *Store) retexEvolution(work, id string) (map[string]string, error) {
	w, err := s.get(work)
	if err != nil {
		return nil, err
	}
	r, err := findRetex(&w, id)
	if err != nil {
		return nil, err
	}
	if r.SourceKind != "acceptance" || r.Status == "ecarte" || r.Status == "remplace" {
		return nil, fmt.Errorf("évolution issue d’une recette indisponible")
	}
	if err = s.verifyAcceptanceRetexSource(&w, r.Source); err != nil {
		return nil, err
	}
	need := fmt.Sprintf("Projet existant : %s\nSource : %s / %s ; SHA-256 %s\nConstat (%s) : %s\nAmélioration souhaitée : %s\nValeur utilisateur : %s\nType d’évolution : %s\nEffort : %s\nRisque : %s\nCritères de réussite : %s\nPreuves : %s\nComportements à préserver : ceux de la recette acceptée.\nPréparer un nouvel incrément à adopter avant toute exécution.", w.Title, w.ID, r.ID, r.Source.SHA256, r.Nature, r.Problem, r.Proposal, r.Impact, r.OpportunityKind, r.Effort, r.Risk, r.Criteria, r.Proof)
	if len(need) > preparationDocumentLimit {
		return nil, fmt.Errorf("RETEX trop long pour le cadrage : réduire les champs avant préparation")
	}
	return map[string]string{"title": r.Problem, "need": need}, nil
}
