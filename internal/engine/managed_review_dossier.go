//go:build linux

package engine

import "fmt"

// A read-only export for offline capacity analysis. It is never a review result,
// provider request, budget authorization or replacement for the anchored proof.
type ManagedReviewDossier struct {
	Version  int                  `json:"schema_version"`
	Work     string               `json:"work"`
	Task     string               `json:"task"`
	Revision int                  `json:"revision"`
	Context  managedReviewContext `json:"context"`
}

func (s *Store) managedReviewDossier(work, task string) (ManagedReviewDossier, error) {
	var out ManagedReviewDossier
	evidence, err := s.readManagedPreflightEvidence(work, task)
	if err != nil {
		return out, err
	}
	current, err := s.get(work)
	if err != nil {
		return out, err
	}
	if current.Revision != evidence.Revision {
		return out, fmt.Errorf("travail modifié pendant l’export ; relire le dossier courant")
	}
	return ManagedReviewDossier{1, work, task, evidence.Revision, evidence.Context}, nil
}
