//go:build linux

package main

import (
	"context"
	"fmt"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

// This bounds one delivery, not a provider context window. Splitting review
// packets must not silently turn an oversized delivery into an affordable one.
const managedScopeFiles = 100
const managedScopeRefusal = "remise trop large"

type ManagedScopePlan struct {
	Evidence          string               `json:"evidence_sha256,omitempty"`
	Criteria          []string             `json:"criteria,omitempty"`
	Decomposition     *ReviewDecomposition `json:"decomposition,omitempty"`
	Revision          int                  `json:"revision"`
	Base              string               `json:"accepted_base"`
	Candidate         string               `json:"candidate_commit"`
	Files             []string             `json:"changed_files"`
	Limit             int                  `json:"file_limit"`
	RequiresReduction bool                 `json:"requires_reduction"`
	Selected          []string             `json:"selected_files,omitempty"`
	Deferred          []string             `json:"deferred_files,omitempty"`
	Patch             string               `json:"patch,omitempty"`
	Next              string               `json:"next_step"`
}

// Exact bytes matter for binary patches and filenames ending in whitespace.
func scopeGit(bare string, args ...string) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, "git", append([]string{"--literal-pathspecs", "-C", bare}, args...)...)
	data, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("lecture du périmètre Git : %w", err)
	}
	return string(data), nil
}
func managedScope(bare, base, candidate string) (ManagedScopePlan, error) {
	p := ManagedScopePlan{Base: base, Candidate: candidate, Limit: managedScopeFiles, Next: "Examiner les fichiers nécessaires et leurs dépendances ; préparer un patch avec scope-patch. Appliquer sur une copie de la base acceptée, puis refaire les contrôles et la revue. Le candidat original reste conservé."}
	raw, err := scopeGit(bare, "diff", "--no-ext-diff", "--no-textconv", "--no-renames", "--name-only", "-z", base, candidate, "--")
	if err != nil {
		return p, err
	}
	if raw != "" {
		p.Files = strings.Split(strings.TrimSuffix(raw, "\x00"), "\x00")
	}
	p.RequiresReduction = len(p.Files) > p.Limit
	diff, err := scopeGit(bare, "diff", "--no-ext-diff", "--no-textconv", "--no-renames", "--binary", base, candidate, "--")
	if err != nil {
		return p, err
	}
	decomposition := detectReviewDecomposition(candidate, p.Files, len(diff))
	p.Decomposition = &decomposition
	if decomposition.Required {
		p.Next = "Faire examiner les types de découpage et les lots proposés par le sous-planificateur. Conserver le candidat complet. Vérifier couverture, dépendances, capacité, budget et revue finale avant exécution ; les suggestions ne sont pas un plan autorisé."
	}
	return p, nil
}
func (s *Store) managedScopePreview(work string, r PlanningRequest, patch bool) (ManagedScopePlan, error) {
	e, err := s.readManagedPreflightEvidence(work, r.Task)
	if err != nil {
		return ManagedScopePlan{}, err
	}
	bare := filepath.Join(e.Work.Planning.Repository.Storage, "repository.git")
	p, err := managedScope(bare, e.Context.Previous, e.Context.Candidate)
	if err != nil {
		return p, err
	}
	p.Revision = e.Revision
	p.Evidence = coordinationEvidence(e.Context)
	p.Criteria = coordinationCriteria(e.Context)
	if patch {
		if r.Revision != e.Revision || r.ExpectedCandidate != p.Candidate {
			return p, fmt.Errorf("candidat ou révision modifié ; relire scope-preview")
		}
		if len(r.ScopeFiles) == 0 || len(r.ScopeFiles) > p.Limit {
			return p, fmt.Errorf("sélection explicite de 1 à %d fichiers requise", p.Limit)
		}
		known := map[string]bool{}
		for _, f := range p.Files {
			known[f] = true
		}
		selected := map[string]bool{}
		for _, f := range r.ScopeFiles {
			if !known[f] || selected[f] {
				return p, fmt.Errorf("fichier absent du delta ou répété : %q", f)
			}
			selected[f] = true
		}
		for _, f := range p.Files {
			if selected[f] {
				p.Selected = append(p.Selected, f)
			} else {
				p.Deferred = append(p.Deferred, f)
			}
		}
		// Renames are represented as delete/add so selection never pulls in a hidden
		// second path. No checkout, index, proof, counter or acceptance is modified.
		args := []string{"diff", "--binary", "--full-index", "--no-ext-diff", "--no-textconv", "--no-renames", p.Base, p.Candidate, "--"}
		p.Patch, err = scopeGit(bare, append(args, p.Selected...)...)
		if err != nil {
			return p, err
		}
	}
	current, err := s.get(work)
	if err != nil {
		return p, err
	}
	if current.Revision != e.Revision {
		return p, fmt.Errorf("travail modifié pendant la préparation ; relire le périmètre")
	}
	return p, nil
}
