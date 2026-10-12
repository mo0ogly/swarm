package engine

import (
	"fmt"
	"strconv"
	"time"
)

// The supervisor proposes cycle decisions through its ordinary leased decision.
// Reuse public judge guards; no separate authority, provider call or reset.
func (s *Store) applyPlanningJudgeOperation(w *Work, scopeID string, op PlanningOperation, r PlanningRequest, at time.Time) error {
	if scopeID != "root" {
		return fmt.Errorf("jugement et renouvellement globaux : périmètre racine requis")
	}
	if len(r.Operations) != 1 {
		return fmt.Errorf("décision de cycle seule requise ; séparer les modifications du plan")
	}
	if op.ID != "" || op.Title != "" || len(op.Requirements) != 0 || op.Deliverable != "" || len(op.Criteria) != 0 || len(op.Depends) != 0 || op.Next != "" || op.MaxTasks != 0 || op.MaxActivations != 0 {
		return fmt.Errorf("champs de plan sans rapport avec la décision de cycle")
	}
	r.JudgeCycle, r.JudgeVerdict, r.ExpectedCandidate = op.JudgeCycle, op.JudgeVerdict, op.ExpectedCandidate
	kind := "cycle_judged"
	if op.Kind == "judge" {
		if err := s.applyJudgeVerdict(w, r, at); err != nil {
			return err
		}
		if op.JudgeVerdict == "end" {
			root, _ := w.Planning.scope("root")
			root.State = "closed" // Explicit supervisor end, never inferred completion.
			return nil
		}
	} else {
		if op.JudgeVerdict != "" || op.ExpectedCandidate != "" {
			return fmt.Errorf("renouvellement sans nouveau verdict ni candidat")
		}
		if err := applyJudgeRenewal(w, r, at); err != nil {
			return err
		}
		kind = "cycle_renewed"
	}
	w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: planningEventID(r.EventID, kind), Scope: "root", Kind: kind, Message: "Décision du superviseur appliquée ; cycle " + strconv.Itoa(w.Planning.Judge.Cycle) + ". Reconstruire le contexte depuis les exigences, résultats et incidents courants ; historique et consommation conservés.", At: at.Format(time.RFC3339Nano)})
	return nil
}

// JudgeState is the mission-wide continue/end verdict (JAN-4). It is kept
// distinct from the independent per-candidate review (V1-V5, see
// independent_review.go): the judge decides on the mission objective and the
// cumulated cycle results, never on a single candidate diff. It lives inside
// the same Work/PlanningState body as every other planning field (root
// scope only), so it shares the same revision, event log and optimistic
// concurrency boundary -- no second persistence path, no second acceptance
// schema.
type JudgeState struct {
	Verdict     string             `json:"verdict,omitempty"`
	Reason      string             `json:"reason,omitempty"`
	EvidenceRef string             `json:"evidence_ref,omitempty"`
	At          string             `json:"at,omitempty"`
	Cycle       int                `json:"cycle"`
	MaxCycles   int                `json:"max_cycles"`
	Closed      bool               `json:"closed"`
	History     []JudgeCycleRecord `json:"history,omitempty"`
}

// JudgeCycleRecord is one past, immutable cycle verdict (JAN-5/FEB-3): a
// renewal never rewrites it, so the mission keeps every prior decision even
// as the live JudgeState moves on to a fresh cycle.
type JudgeCycleRecord struct {
	Verdict     string `json:"verdict"`
	Reason      string `json:"reason"`
	EvidenceRef string `json:"evidence_ref,omitempty"`
	At          string `json:"at"`
	RenewedAt   string `json:"renewed_at,omitempty"`
	Cycle       int    `json:"cycle"`
}

// A positive cycle policy is explicitly persisted by the caller. Zero means
// no cycle ceiling; activation, decision and provider budgets still apply.

// applyJudgeVerdict implements the JAN-4 global judge: a verdict ("continue"
// or "end") on the current cycle, distinct from and never overriding the
// independent candidate review (it never touches w.Planning.Repository,
// Reviewer or any V1-V5 field). A verdict can be rendered only once per
// cycle (judge_already_decided); renew (applyJudgeRenewal) must run before a
// new one. An "end" verdict is refused without an attested candidate
// matching the engine's own current managed-repository candidate (req-8:
// no global closure without fresh evidence) -- the same mismatch that
// invalidated an earlier, unrelated attempt at this task is refused here
// structurally rather than caught later by review.
func (s *Store) applyJudgeVerdict(w *Work, r PlanningRequest, at time.Time) error {
	p := w.Planning
	if p.Failure != "" {
		return fmt.Errorf("planification arrêtée : %s", p.Failure)
	}
	if p.Judge == nil || p.Judge.Closed {
		return planningError("judge_closed", "juge global clos ou non initialisé : activation ou reprise requise")
	}
	if r.JudgeCycle != p.Judge.Cycle {
		return planningError("stale_verdict", "verdict périmé : le cycle de juge a changé depuis la lecture")
	}
	if p.Judge.Verdict != "" {
		return planningError("judge_already_decided", "ce cycle a déjà reçu un verdict ; renouveler avant de rejuger")
	}
	if r.JudgeVerdict != "continue" && r.JudgeVerdict != "end" {
		return fmt.Errorf("judge_verdict doit valoir continue ou end")
	}
	if !nonempty(r.Reason) || len(r.Reason) > 4000 {
		return fmt.Errorf("motif du verdict requis, 4000 caractères maximum")
	}
	if r.JudgeVerdict == "end" {
		if !nonempty(r.ExpectedCandidate) {
			return planningError("stale_evidence", "clôture refusée : candidat attesté requis (preuve fraîche)")
		}
		if p.Repository == nil || p.Repository.Candidate != r.ExpectedCandidate {
			return planningError("stale_evidence", "clôture refusée : candidat attesté distinct du candidat courant du dépôt géré")
		}
		if err := s.validatePlanningClosure(w, "root", r); err != nil {
			return planningError("stale_evidence", "clôture refusée : "+err.Error())
		}
	}
	p.Judge.Verdict = r.JudgeVerdict
	p.Judge.Reason = r.Reason
	p.Judge.EvidenceRef = r.ExpectedCandidate
	p.Judge.At = at.Format(time.RFC3339Nano)
	if r.JudgeVerdict == "end" {
		p.Judge.Closed = true
	}
	p.Judge.History = append(p.Judge.History, JudgeCycleRecord{Cycle: p.Judge.Cycle, Verdict: p.Judge.Verdict, Reason: p.Judge.Reason, EvidenceRef: p.Judge.EvidenceRef, At: p.Judge.At})
	return nil
}

// applyJudgeRenewal implements the JAN-5/FEB-3 cycle renewal: it opens a
// fresh cycle (new context for the next judge verdict) while preserving the
// scope objective/requirements, every activation/decision counter and the
// full judge History -- a renewal, never a reset. Only a "continue" verdict
// can be renewed; an "end" verdict stays Closed permanently (no implicit
// reopening). Each renewal consumes one unit of the max_judge_cycles quota
// persisted in the mission. Its quota can be revised through the common
// public quotas API/CLI; renewal never raises it or resets consumption.
func applyJudgeRenewal(w *Work, r PlanningRequest, at time.Time) error {
	p := w.Planning
	if p.Failure != "" {
		return fmt.Errorf("planification arrêtée : %s", p.Failure)
	}
	if p.Judge == nil || p.Judge.Closed {
		return planningError("judge_closed", "juge global clos ou non initialisé : aucun renouvellement possible")
	}
	if r.JudgeCycle != p.Judge.Cycle {
		return planningError("stale_verdict", "verdict périmé : le cycle a déjà été renouvelé")
	}
	if p.Judge.Verdict == "" {
		return fmt.Errorf("renouvellement refusé : aucun verdict à renouveler, juger d'abord ce cycle")
	}
	if p.Judge.Verdict != "continue" {
		return fmt.Errorf("renouvellement refusé : le dernier verdict n'est pas continue")
	}
	if p.Judge.MaxCycles > 0 && p.Judge.Cycle >= p.Judge.MaxCycles {
		return planningError("quota_exceeded", "quota de cycles de juge atteint")
	}
	renewedAt := at.Format(time.RFC3339Nano)
	if n := len(p.Judge.History); n > 0 && p.Judge.History[n-1].Cycle == p.Judge.Cycle {
		p.Judge.History[n-1].RenewedAt = renewedAt
	}
	// Fresh context for the next cycle: only the live verdict/reason/evidence
	// reset. Objective, requirements, activation/decision counters and every
	// past cycle in History are never touched.
	p.Judge.Cycle++
	p.Judge.Verdict = ""
	p.Judge.Reason = ""
	p.Judge.EvidenceRef = ""
	p.Judge.At = ""
	return nil
}
