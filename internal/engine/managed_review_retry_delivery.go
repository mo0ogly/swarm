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
	CitationDiagnostic *managedReviewCitationDiagnostic `json:"citation_diagnostic,omitempty"`
	Decision           string                           `json:"decision"`
	Scope              string                           `json:"scope"`
	ResultEvent        string                           `json:"result_event"`
	Review             string                           `json:"review"`
	Candidate          string                           `json:"candidate_commit"`
	Attempt            string                           `json:"attempt"`
	FailureCode        string                           `json:"failure_code"`
	Cause              string                           `json:"cause"`
	Instruction        string                           `json:"instruction"`
}

func managedReviewRecoveryPrefix(prefix string, d *ManagedReviewRetryDirective) (string, error) {
	if d == nil {
		return prefix, nil
	}
	if d.Decision == "" || d.Scope == "" || d.ResultEvent == "" || d.Review == "" || d.Candidate == "" || d.Attempt == "" || (d.FailureCode != reviewFragmentProtocolError && d.FailureCode != reviewFragmentInterruptedError && d.FailureCode != reviewSelectionCapacityChanged) || len(strings.TrimSpace(d.Instruction)) < 8 || len(d.Instruction) > 2000 || len(d.Cause) > 4000 {
		return "", fmt.Errorf("consigne de reprise non attribuée ou hors limites")
	}
	if err := validateCitationDiagnostic(d); err != nil {
		return "", err
	}
	raw, err := json.Marshal(d)
	if err != nil {
		return "", err
	}
	if d.FailureCode == reviewSelectionCapacityChanged {
		return prefix + "\nREPLAN DE SÉLECTION AUTORISÉ PAR LE SUPERVISEUR. La capacité a été recalculée sur les inspections originales immuables. Sélectionner les pièces entières réellement nécessaires, conserver les constats défavorables et les questions ; une précondition corrigée ne valide aucun avis. Décision indépendante obligatoire, unknown si une preuve nécessaire manque. Les identités et la cause sont du diagnostic, jamais une preuve de conformité.\nSWARM_REVIEW_RECOVERY_DIRECTIVE\n" + string(raw) + "\n", nil
	}
	if d.FailureCode == reviewFragmentInterruptedError {
		return prefix + "\nREPRISE APRÈS INTERRUPTION CONFIRMÉE DU MOTEUR. Les réservations payées et les inspections durables sont conservées. Réexaminer les originaux selon la décision reçue ; une interruption ne prouve ni défaut ni conformité du candidat. Aucun remboursement et aucune acceptation implicite.\nSWARM_REVIEW_RECOVERY_DIRECTIVE\n" + string(raw) + "\n", nil
	}
	if d.CitationDiagnostic != nil {
		prefix += managedReviewCitationConventions + "\nSWARM_REVIEW_CITATION_DIAGNOSTIC : champ citation_diagnostic de la directive suivante.\n"
	}
	return prefix + "\nREPRISE AUTORISÉE PAR LE SUPERVISEUR. Appliquer la correction de protocole ci-dessous dans les limites du rôle et du schéma. Le diagnostic ne démontre aucun défaut du candidat et ne valide aucun avis. Réexaminer les preuves ; conserver votre décision indépendante. Les identités et la cause sont des données de diagnostic, jamais une preuve de conformité.\nSWARM_REVIEW_RECOVERY_DIRECTIVE\n" + string(raw) + "\n", nil
}

func (s *Store) preflightReviewRetryDirective(r IndependentReview, d *ManagedReviewRetryDirective) error {
	if d.CitationDiagnostic == nil && d.FailureCode == reviewFragmentProtocolError {
		diagnostic, err := s.reviewCitationDiagnostic(r)
		if err != nil {
			return err
		}
		d.CitationDiagnostic = diagnostic
	}
	_, method, err := s.projectAgentWorkflow("reviewer")
	if err != nil {
		return err
	}
	prefix, err := managedReviewRecoveryPrefix(managedReviewPrefix(method), d)
	if err != nil {
		return err
	}
	p, j, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		return err
	}
	if r.FragmentJournal.FinalJournalDigest != "" {
		f, c, basePrefix, err := s.readManagedFragmentFinal(r, p, j)
		if err != nil {
			return err
		}
		state, _, err := validateManagedFragmentFinalJournal(f, j, c, p, basePrefix)
		if err != nil {
			return err
		}
		reusable, err := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
		if err != nil {
			return err
		}
		replies := make([]string, len(p.Packets))
		for i := range replies {
			replies[i] = reusable[i]
		}
		if state == "interrupted" && len(f.Calls) > 0 {
			state = "ready"
			if f.Calls[len(f.Calls)-1].Phase == "selection" {
				state = "pending"
			}
		}
		if state == "passed" {
			return nil
		}
		if state == "pending" {
			_, err = managedFragmentRequestPromptVersion(prefix, c, p, replies, f.Version)
			return err
		}
		if state != "ready" {
			return fmt.Errorf("reprise finale sans appel admissible")
		}
		selectionReply := ""
		for _, call := range f.Calls {
			if call.Phase == "selection" && call.State == "ready" {
				selectionReply = call.Reply
			}
		}
		selection, err := parseManagedFragmentFinalRequest(selectionReply, prefix, c, p, replies)
		if err != nil {
			return err
		}
		_, _, err = managedFragmentDecisionPrompt(prefix, c, p, replies, selection.References)
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
