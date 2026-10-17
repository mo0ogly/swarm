//go:build linux

package engine

import (
	"fmt"
	"os"
)

func (s *Store) managedReviewProtocolPrecondition(r IndependentReview) error {
	if r.FailureCode != reviewFragmentProtocolError {
		return nil
	}
	path, err := safeReport(s.root, r.Context)
	if err != nil {
		return err
	}
	raw, err := os.ReadFile(path)
	if err != nil || hash(raw) != r.ContextDigest {
		return fmt.Errorf("contexte original de reprise modifié")
	}
	var c managedReviewContext
	if err = strict(raw, &c); err != nil {
		return err
	}
	if c.CitationContractVersion == 2 {
		return &CommandError{Code: "review_precondition_unchanged", Message: "Erreur de protocole : le contrat de références et le fournisseur sont inchangés. Diagnostiquer et corriger le contrat versionné avant une nouvelle dépense ; un éventuel remplacement du vérificateur doit être admis par son propre précontrôle. Reformuler une consigne, changer le délai ou relever le budget ne suffit pas. Candidat, incident et appels consommés conservés."}
	}
	return nil
}
