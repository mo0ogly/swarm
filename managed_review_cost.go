//go:build linux

package main

import (
	"path/filepath"
	"strings"
)

// Read-only explanation, never a budget authorization or a review verdict.
type ManagedReviewCost struct {
	Candidate           string   `json:"candidate_commit"`
	Protocol            int      `json:"protocol_version"`
	ArtifactCount       int      `json:"artifacts"`
	InspectionBytes     int      `json:"inspection_content_bytes"`
	ContentBytes        int      `json:"content_bytes"`
	DiffFiles           int      `json:"diff_files"`
	ChangedSinceRefusal []string `json:"changed_since_refusal,omitempty"`
	Inspections         int      `json:"inspections"`
	FinalCalls          int      `json:"final_calls"`
	CallsRequired       int      `json:"calls_required"`
	CallsAvailable      int      `json:"calls_available"`
	TransportReady      bool     `json:"transport_ready"`
	TransportBlocker    string   `json:"transport_blocker,omitempty"`
	FitsBudget          bool     `json:"fits_budget"`
	Reusable            int      `json:"reusable_observations"`
	ReuseReason         string   `json:"reuse_reason"`
}

func (s *Store) managedReviewCostPreview(work, task string) (ManagedReviewCost, error) {
	var out ManagedReviewCost
	evidence, err := s.readManagedPreflightEvidence(work, task)
	if err != nil {
		return out, err
	}
	c, w := evidence.Context, evidence.Work
	// A computational ceiling permits an estimate even when runtime budget is
	// insufficient. It is never persisted, nor passed to a provider/reservation.
	p, err := s.planCandidateFragments(w, evidence.Agent, c, 100000)
	if err != nil {
		return out, err
	}
	out = ManagedReviewCost{Candidate: c.Candidate, Protocol: p.Version, Inspections: fragmentPaidInspections(p), FinalCalls: p.ReservedFinalCalls, CallsRequired: fragmentPaidInspections(p) + p.ReservedFinalCalls, CallsAvailable: max(0, w.Planning.Reviewer.MaxCalls-w.Planning.Reviewer.Calls), ReuseReason: "Aucune observation historique réutilisée automatiquement : dépendances et protocole doivent être démontrés avant réutilisation ; une pièce inchangée ne suffit pas."}
	for i, packet := range p.Packets {
		for _, a := range packet.Artifacts {
			out.ArtifactCount++
			out.ContentBytes += len(a.Content)
			if fragmentReuseAt(p, i) == nil {
				out.InspectionBytes += len(a.Content)
			}
			if a.Kind == "diff" {
				out.DiffFiles++
			}
		}
	}
	out.Reusable = len(p.Reused)
	if out.Reusable > 0 {
		out.ReuseReason = "Groupes d’observations locales historiques : toutes les pièces vues sont identiques. Nouveau delta complet et résolution explicite des impacts exigés dans l’avis final actuel."
	}
	out.FitsBudget = out.CallsRequired <= out.CallsAvailable
	_, method, methodErr := agentWorkflow("reviewer")
	if methodErr != nil {
		return out, methodErr
	}
	if check := preflightManagedFragmentCalls(managedReviewPrefix(method), c, p); check != nil {
		out.TransportBlocker = check.Error()
	} else {
		out.TransportReady = true
	}
	t, _ := w.task(task)
	if t.RecoveredResult != nil && t.RecoveredResult.ReplacesResult != "" {
		bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
		changed, e := managedGit(bare, "diff", "--name-only", "-z", t.RecoveredResult.ReplacesResult, evidence.Item.Result, "--")
		if e != nil {
			return out, e
		}
		out.ChangedSinceRefusal = strings.Split(strings.TrimSuffix(changed, "\x00"), "\x00")
		if changed == "" {
			out.ChangedSinceRefusal = nil
		}
	}
	return out, nil
}
