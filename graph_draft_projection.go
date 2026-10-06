package main

import (
	"encoding/json"
	"sort"
)

// GraphDraftProofProjection identifies proof inputs changed by a graph
// revision. The global work revision is deliberately excluded: presentation
// and administrative changes may advance it without changing tested inputs.
type GraphDraftProofProjection struct {
	Kind          string   `json:"kind"`
	Relevant      bool     `json:"proof_relevant"`
	AffectedTasks []string `json:"affected_tasks"`
	BeforeDigest  string   `json:"before_inputs_sha256"`
	AfterDigest   string   `json:"after_inputs_sha256"`
	Reason        string   `json:"reason"`
}

// MissionAttemptProjection binds a task selection to one concrete agent and
// business attempt. Process and activity are observations, never validation.
type MissionAttemptProjection struct {
	AgentID        string `json:"agent_id,omitempty"`
	AttemptID      string `json:"attempt_id,omitempty"`
	Role           string `json:"role,omitempty"`
	ProcessState   string `json:"process_state,omitempty"`
	Activity       string `json:"activity,omitempty"`
	Validation     string `json:"validation_state"`
	RequestedModel string `json:"requested_model,omitempty"`
	ObservedModel  string `json:"observed_model,omitempty"`
	CostState      string `json:"cost_state"`
}

func taskRelevantInputs(w *Work, t *Task) string {
	if t == nil {
		return ""
	}
	candidate := ""
	if w != nil && w.Planning != nil && w.Planning.Repository != nil {
		candidate = w.Planning.Repository.Candidate
	}
	policy := ""
	if t.ValidationPolicy != nil {
		policy = validationPolicyDigest(*t.ValidationPolicy)
	}
	dependencies := append([]string(nil), t.Depends...)
	sort.Strings(dependencies)
	attempt := ""
	if len(t.Attempts) > 0 {
		attempt = t.Attempts[len(t.Attempts)-1].ID
	}
	raw, _ := json.Marshal(struct {
		ID, Deliverable, Scope, Policy, Candidate, Attempt string
		Criteria, Dependencies, Requirements               []string
	}{t.ID, t.Deliverable, t.ScopeID, policy, candidate, attempt, append([]string(nil), t.Criteria...), dependencies, append([]string(nil), t.Requirements...)})
	return hash(raw)
}

func graphDraftProofProjection(before, after *Work, affected []string) GraphDraftProofProjection {
	ids := append([]string(nil), affected...)
	sort.Strings(ids)
	beforeInputs, afterInputs := map[string]string{}, map[string]string{}
	for _, id := range ids {
		if before != nil {
			t, _ := before.task(id)
			beforeInputs[id] = taskRelevantInputs(before, t)
		}
		if after != nil {
			t, _ := after.task(id)
			afterInputs[id] = taskRelevantInputs(after, t)
		}
	}
	oldRaw, _ := json.Marshal(beforeInputs)
	newRaw, _ := json.Marshal(afterInputs)
	p := GraphDraftProofProjection{Kind: "administrative", AffectedTasks: ids, BeforeDigest: hash(oldRaw), AfterDigest: hash(newRaw), Reason: "Entrées pertinentes identiques ; la révision administrative ne périme pas la preuve."}
	p.Relevant = p.BeforeDigest != p.AfterDigest
	if p.Relevant {
		p.Kind = "relevant"
		p.Reason = "Le candidat, le contrat de tâche, les dépendances ou la politique de contrôle ont changé ; les reçus antérieurs restent historiques mais ne valent plus acceptation courante."
	}
	return p
}

func projectMissionAttempt(t *Task, agents []Agent) MissionAttemptProjection {
	p := MissionAttemptProjection{Validation: "unknown", CostState: "unknown"}
	if t == nil {
		return p
	}
	p.Validation = t.Status
	for _, a := range agents { // store readers return newest attempts first.
		if a.TaskID != t.ID {
			continue
		}
		p.AgentID, p.AttemptID, p.Role = a.ID, a.Attempt, a.Role
		p.ProcessState, p.Activity = a.Status, a.Activity
		if a.ModelRoute != nil {
			p.RequestedModel = a.ModelRoute.Model
		}
		if a.ReportedModel != nil {
			p.ObservedModel = a.ReportedModel.Model
		}
		if a.Usage != nil && a.Usage.ReportedCost != nil {
			p.CostState = "reported"
		}
		return p
	}
	if len(t.Attempts) > 0 {
		p.AttemptID = t.Attempts[len(t.Attempts)-1].ID
	}
	return p
}
