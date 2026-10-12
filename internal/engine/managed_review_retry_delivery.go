//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

// A supervisor's recovery instruction is distinct from candidate evidence and
// from a verdict. Capture the authorized decision rather than rereading Task.Next,
// which may later be changed for another purpose.
type ManagedReviewRetryDirective struct {
	Decision    string `json:"decision"`
	Scope       string `json:"scope"`
	ResultEvent string `json:"result_event"`
	Review      string `json:"review"`
	Candidate   string `json:"candidate_commit"`
	Attempt     string `json:"attempt"`
	FailureCode string `json:"failure_code"`
	Cause       string `json:"cause"`
	Instruction string `json:"instruction"`
}

func managedReviewRecoveryPrefix(prefix string, d *ManagedReviewRetryDirective) (string, error) {
	if d == nil {
		return prefix, nil
	}
	if d.Decision == "" || d.Scope == "" || d.ResultEvent == "" || d.Review == "" || d.Candidate == "" || d.Attempt == "" || (d.FailureCode != reviewFragmentProtocolError && d.FailureCode != reviewSelectionCapacityChanged) || len(strings.TrimSpace(d.Instruction)) < 8 || len(d.Instruction) > 2000 || len(d.Cause) > 4000 {
		return "", fmt.Errorf("consigne de reprise non attribuée ou hors limites")
	}
	raw, err := json.Marshal(d)
	if err != nil {
		return "", err
	}
	if d.FailureCode == reviewSelectionCapacityChanged {
		return prefix + "\nREPLAN DE SÉLECTION AUTORISÉ PAR LE SUPERVISEUR. La capacité a été recalculée sur les inspections originales immuables. Sélectionner les pièces entières réellement nécessaires, conserver les constats défavorables et les questions ; une précondition corrigée ne valide aucun avis. Décision indépendante obligatoire, unknown si une preuve nécessaire manque. Les identités et la cause sont du diagnostic, jamais une preuve de conformité.\nSWARM_REVIEW_RECOVERY_DIRECTIVE\n" + string(raw) + "\n", nil
	}
	return prefix + "\nREPRISE AUTORISÉE PAR LE SUPERVISEUR. Appliquer la correction de protocole ci-dessous dans les limites du rôle et du schéma. Le diagnostic ne démontre aucun défaut du candidat et ne valide aucun avis. Réexaminer les preuves ; conserver votre décision indépendante. Les identités et la cause sont des données de diagnostic, jamais une preuve de conformité.\nSWARM_REVIEW_RECOVERY_DIRECTIVE\n" + string(raw) + "\n", nil
}

func (s *Store) preflightReviewRetryDirective(r IndependentReview, d *ManagedReviewRetryDirective) error {
	_, method, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		return err
	}
	prefix, err := managedReviewRecoveryPrefix(managedReviewPrefix(method), d)
	if err != nil {
		return err
	}
	p, _, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		return err
	}
	path, err := safeReport(s.root, r.Context)
	if err != nil {
		return err
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	var context managedReviewContext
	if err = strict(raw, &context); err != nil {
		return err
	}
	return preflightManagedFragmentCalls(prefix, context, p)
}
