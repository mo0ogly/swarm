//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"
)

// A validated proposal is not a review result or permission to spend calls.
type ReviewCoordinationLot struct {
	ID        string   `json:"id"`
	Kind      string   `json:"kind"`
	Objective string   `json:"objective"`
	Files     []string `json:"files"`
	Criteria  []string `json:"criteria"`
	Depends   []string `json:"depends"`
}
type ReviewCoordinationProposal struct {
	Candidate   string                  `json:"candidate_commit"`
	Evidence    string                  `json:"evidence_sha256"`
	Lots        []ReviewCoordinationLot `json:"lots"`
	FinalReview string                  `json:"final_review"`
}
type ReviewCoordinationRecord struct {
	Proposal ReviewCoordinationProposal `json:"proposal"`
	Digest   string                     `json:"sha256"`
	Scope    string                     `json:"scope"`
	Decision string                     `json:"decision"`
	State    string                     `json:"state"`
	Order    []string                   `json:"order"`
}

func coordinationCriteria(c managedReviewContext) []string {
	ids := []string{}
	for _, t := range c.Tasks {
		for i := range t.Criteria {
			ids = append(ids, fmt.Sprintf("%s#%d", t.Task, i+1))
		}
	}
	sort.Strings(ids)
	return ids
}
func coordinationEvidence(c managedReviewContext) string { b, _ := json.Marshal(c); return hash(b) }
func validateReviewCoordination(p ReviewCoordinationProposal, candidate, evidence string, files, criteria []string) ([]string, error) {
	if p.Candidate != candidate || p.Evidence != evidence {
		return nil, fmt.Errorf("candidat ou preuves du plan périmés")
	}
	if len(p.Lots) < 1 || len(p.Lots) > 100 || len(strings.TrimSpace(p.FinalReview)) < 16 || len(p.FinalReview) > 4000 {
		return nil, fmt.Errorf("lots bornés et revue finale des interactions requis")
	}
	knownFiles := map[string]bool{}
	for _, f := range files {
		knownFiles[f] = true
	}
	knownCriteria := map[string]bool{}
	for _, c := range criteria {
		knownCriteria[c] = true
	}
	coveredFiles := map[string]bool{}
	coveredCriteria := map[string]bool{}
	lots := map[string]ReviewCoordinationLot{}
	for _, lot := range p.Lots {
		if !safeName(lot.ID) || lots[lot.ID].ID != "" || len(strings.TrimSpace(lot.Objective)) < 8 || len(lot.Objective) > 2000 {
			return nil, fmt.Errorf("identité ou objectif de lot invalide")
		}
		switch lot.Kind {
		case "requirement", "component", "dependency", "specialty", "volume":
		default:
			return nil, fmt.Errorf("type de découpage inconnu")
		}
		if len(lot.Files) == 0 || len(lot.Files) > managedScopeFiles || len(lot.Criteria) == 0 {
			return nil, fmt.Errorf("lot sans fichiers, trop large ou sans critères")
		}
		local := map[string]bool{}
		for _, f := range lot.Files {
			if !knownFiles[f] || local[f] {
				return nil, fmt.Errorf("fichier inconnu ou répété : %q", f)
			}
			local[f] = true
			coveredFiles[f] = true
		}
		local = map[string]bool{}
		for _, c := range lot.Criteria {
			if !knownCriteria[c] || local[c] {
				return nil, fmt.Errorf("critère inconnu ou répété : %s", c)
			}
			local[c] = true
			coveredCriteria[c] = true
		}
		lots[lot.ID] = lot
	}
	if len(coveredFiles) != len(knownFiles) || len(coveredCriteria) != len(knownCriteria) {
		return nil, fmt.Errorf("couverture incomplète des fichiers ou critères")
	}
	for _, lot := range p.Lots {
		seen := map[string]bool{}
		for _, dep := range lot.Depends {
			if dep == lot.ID || lots[dep].ID == "" || seen[dep] {
				return nil, fmt.Errorf("dépendance absente, répétée ou réflexive")
			}
			seen[dep] = true
		}
	}
	state := map[string]int{}
	order := []string{}
	var visit func(string) error
	visit = func(id string) error {
		if state[id] == 1 {
			return fmt.Errorf("cycle entre lots de revue")
		}
		if state[id] == 2 {
			return nil
		}
		state[id] = 1
		for _, dep := range lots[id].Depends {
			if e := visit(dep); e != nil {
				return e
			}
		}
		state[id] = 2
		order = append(order, id)
		return nil
	}
	for _, lot := range p.Lots {
		if e := visit(lot.ID); e != nil {
			return nil, e
		}
	}
	return order, nil
}

type preparedCoordination struct {
	Evidence *managedPreflightRetry
	Files    []string
}

func (s *Store) adoptReviewCoordination(w *Work, scope string, op PlanningOperation, r PlanningRequest) error {
	task, err := w.task(op.ID)
	if err != nil {
		return err
	}
	if task.ScopeID != scope || task.Status != "blocked" || task.IndependentReview != nil {
		return fmt.Errorf("plan de revue réservé au responsable de la tâche bloquée sans revue engagée")
	}
	if task.ReviewCoordination != nil {
		return fmt.Errorf("plan déjà enregistré ; aucune substitution implicite")
	}
	if len(op.Deliverable) > 65536 {
		return fmt.Errorf("proposition de revue trop longue")
	}
	var proposal ReviewCoordinationProposal
	if err = strict([]byte(op.Deliverable), &proposal); err != nil {
		return err
	}
	prepared := r.coordinationInputs[op.ID]
	if prepared == nil {
		return fmt.Errorf("précontrôle du plan absent")
	}
	e := prepared.Evidence
	if e.Revision != w.Revision {
		return fmt.Errorf("preuves modifiées pendant la décision")
	}
	order, err := validateReviewCoordination(proposal, e.Context.Candidate, coordinationEvidence(e.Context), prepared.Files, coordinationCriteria(e.Context))
	if err != nil {
		return err
	}
	raw, _ := json.Marshal(proposal)
	task.ReviewCoordination = &ReviewCoordinationRecord{Proposal: proposal, Digest: hash(raw), Scope: scope, Decision: r.EventID, State: "validated_not_executed", Order: order}
	task.Next = "Plan de revue enregistré et couverture vérifiée ; contrôles de capacité, budget et exécution des lots encore requis. Aucune acceptation accordée."
	return nil
}

func (s *Store) planningReviewInputs(w Work, scope string) map[string]any {
	inputs := map[string]any{}
	for _, event := range w.Planning.Inbox {
		if event.Scope != scope || event.Decision != "" {
			continue
		}
		task, e := w.task(event.Task)
		if e != nil || task.ScopeID != scope || task.Status != "blocked" || task.ReviewCoordination != nil {
			continue
		}
		if !strings.Contains(task.Blocker, managedReviewContextTooLarge) && !strings.HasPrefix(task.Blocker, managedScopeRefusal+" : ") {
			continue
		}
		preview, e := s.managedScopePreview(w.ID, PlanningRequest{Task: task.ID}, false)
		if e != nil {
			inputs[task.ID] = map[string]string{"unavailable": e.Error()}
			continue
		}
		inputs[task.ID] = preview
	}
	return inputs
}
