//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"
	"time"
)

// Additions are attributed decisions. The original instruction and rejected
// additions remain durable; changing instructions never changes acceptance.
type InstructionRevision struct {
	ID                 string   `json:"id"`
	Addition           string   `json:"addition"`
	Reason             string   `json:"reason"`
	Scope              string   `json:"scope"`
	Holder             string   `json:"holder"`
	Generation         int      `json:"generation"`
	Inputs             []string `json:"input_events"`
	Decision           string   `json:"decision"`
	At                 string   `json:"at"`
	State              string   `json:"state"`
	AttemptsAtCreation int      `json:"attempts_at_creation"`
	BeforeSHA          string   `json:"before_sha256"`
	AfterSHA           string   `json:"after_sha256"`
	UsedByAttempt      string   `json:"used_by_attempt,omitempty"`
	RejectedAt         string   `json:"rejected_at,omitempty"`
	RejectedReason     string   `json:"rejected_reason,omitempty"`
}

func composedTaskInstructions(t *Task) string {
	if t.InstructionBase == nil {
		return t.Next
	}
	parts := []string{}
	// New learning appears first in the bounded planner projection.
	for i := len(t.InstructionRevisions) - 1; i >= 0; i-- {
		r := t.InstructionRevisions[i]
		if r.State == "applied" {
			parts = append(parts, r.Addition)
		}
	}
	parts = append(parts, instructionOperatorBase(t))
	return strings.Join(parts, "\n\n")
}

func instructionOperatorBase(t *Task) string {
	if t.InstructionOperatorBase != nil {
		return *t.InstructionOperatorBase
	}
	return *t.InstructionBase
}

// Older public task.update calls prepended operator inputs to the entire
// composed view. Recover only this recognizable form; never infer a replacement
// from an arbitrary edit. The original base and attributed learning stay intact.
func reconcileInstructionOperatorPrefix(t *Task) error {
	current := composedTaskInstructions(t)
	if t.Next == current {
		return nil
	}
	if t.InstructionBase == nil || current == "" || !strings.HasSuffix(t.Next, current) {
		return fmt.Errorf("consigne modifiée depuis les ajouts ; réexaminer le contexte actuel")
	}
	base := strings.TrimSuffix(t.Next, current) + instructionOperatorBase(t)
	t.InstructionOperatorBase = &base
	t.Next = composedTaskInstructions(t)
	return nil
}

// A public operator edit changes the operator input, not the applied learning.
// Keep both separable so rejecting one learning cannot erase later operator work.
func updateTaskInstructions(t *Task, next string) error {
	if t.InstructionBase == nil || t.Status != "todo" {
		t.Next = next
		return nil
	}
	previous := t.Next
	if err := reconcileInstructionOperatorPrefix(t); err != nil {
		return err
	}
	current, base := t.Next, instructionOperatorBase(t)
	switch {
	case next == previous || next == current:
		return nil
	case strings.HasSuffix(next, previous):
		base = strings.TrimSuffix(next, previous) + base
	case strings.HasSuffix(next, current):
		base = strings.TrimSuffix(next, current) + base
	default:
		learning := strings.TrimSuffix(current, base)
		if learning != "" && strings.HasPrefix(next, learning) {
			base = strings.TrimPrefix(next, learning)
		} else {
			for _, revision := range t.InstructionRevisions {
				if revision.State == "applied" && strings.Contains(next, revision.Addition) {
					return fmt.Errorf("ajout IA déplacé dans la consigne ; modifier la base opérateur ou rejeter l’ajout explicitement")
				}
			}
			base = next
		}
	}
	t.InstructionOperatorBase = &base
	t.Next = composedTaskInstructions(t)
	return nil
}

func (s *Store) reviseTaskInstructions(w *Work, scope string, op PlanningOperation, r PlanningRequest, at time.Time) error {
	t, err := w.task(op.ID)
	if err != nil {
		return err
	}
	if t.ScopeID != scope || t.Status != "todo" || t.PlanRole == "subplanner" {
		return fmt.Errorf("consignes révisables uniquement pour une tâche future de ce périmètre")
	}
	if len(op.Next) < 8 || len(op.Next) > 2000 || !nonempty(op.Next) {
		return fmt.Errorf("ajout de consigne requis : 8 à 2000 octets")
	}
	if op.Title != "" || len(op.Requirements) != 0 || op.Deliverable != "" || len(op.Criteria) != 0 || len(op.Depends) != 0 || op.MaxTasks != 0 || op.MaxActivations != 0 {
		return fmt.Errorf("revise-instructions accepte uniquement id et next")
	}
	if err := reconcileInstructionOperatorPrefix(t); err != nil {
		return err
	}
	for _, prior := range t.InstructionRevisions {
		if prior.Addition == op.Next {
			return fmt.Errorf("ajout de consigne déjà enregistré ; proposer un apprentissage différent")
		}
	}
	before := t.Next
	if t.InstructionBase == nil {
		base := t.Next
		t.InstructionBase = &base
	}
	revision := InstructionRevision{ID: "instruction-" + hash([]byte(r.EventID + "|" + op.ID))[:24], Addition: op.Next, Reason: r.Reason, Scope: scope, Holder: r.Holder, Generation: r.Generation, Inputs: append([]string(nil), r.Inputs...), Decision: r.EventID, At: at.Format(time.RFC3339Nano), State: "applied", AttemptsAtCreation: len(t.Attempts), BeforeSHA: hash([]byte(before))}
	for _, prior := range t.InstructionRevisions {
		if prior.ID == revision.ID {
			return fmt.Errorf("une seule révision de consigne par tâche et décision")
		}
	}
	t.InstructionRevisions = append(t.InstructionRevisions, revision)
	t.Next = composedTaskInstructions(t)
	t.InstructionRevisions[len(t.InstructionRevisions)-1].AfterSHA = hash([]byte(t.Next))
	return nil
}

func instructionIdleTx(tx *sql.Tx, work, task string) error {
	var count int
	if err := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=? AND status IN ('queued','starting','running','stopping')", work, task).Scan(&count); err != nil {
		return err
	}
	if count != 0 {
		return fmt.Errorf("consignes déjà réservées par une tentative active")
	}
	return nil
}

func (s *Store) rejectTaskInstructions(work string, r PlanningRequest) (Work, error) {
	raw, _ := json.Marshal(r)
	return s.mutateWithHook(work, "planning.reject-instructions", r.EventID, r.Revision, raw, func(w *Work) error {
		if w.Planning == nil {
			return fmt.Errorf("planification non activée")
		}
		if !nonempty(r.Reason) || len(r.Reason) > 4000 || len(r.Operations) != 0 || len(r.Inputs) != 0 || r.Holder != "" {
			return fmt.Errorf("rejet opérateur : motif requis, sans décision IA implicite")
		}
		t, err := w.task(r.Task)
		if err != nil {
			return err
		}
		if r.Scope != "" && r.Scope != t.ScopeID {
			return fmt.Errorf("périmètre de la consigne remplacé")
		}
		if t.Status != "todo" {
			return fmt.Errorf("consigne déjà transmise ou modifiée ; rejet indisponible")
		}
		if err := reconcileInstructionOperatorPrefix(t); err != nil {
			return err
		}
		for i := range t.InstructionRevisions {
			v := &t.InstructionRevisions[i]
			if v.ID != r.InstructionRevisionID {
				continue
			}
			if v.State != "applied" || v.UsedByAttempt != "" || len(t.Attempts) != v.AttemptsAtCreation {
				return fmt.Errorf("consigne déjà rejetée ou transmise à un agent")
			}
			v.State = "rejected"
			v.RejectedAt = now()
			v.RejectedReason = r.Reason
			t.Next = composedTaskInstructions(t)
			scope, e := w.Planning.scope(t.ScopeID)
			if e != nil {
				return e
			}
			scope.Revision++
			scope.State = "ready"
			w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: r.EventID, Scope: scope.ID, Kind: "instruction_rejected", Task: t.ID, Message: "Ajout rejeté par l’utilisateur : " + v.Addition + "\nMotif : " + r.Reason, At: v.RejectedAt})
			return nil
		}
		return fmt.Errorf("ajout de consigne inconnu")
	}, func(tx *sql.Tx, w *Work) error { return instructionIdleTx(tx, work, r.Task) })
}

func freezeInstructionRevisions(t *Task, attempt string) {
	for i := range t.InstructionRevisions {
		v := &t.InstructionRevisions[i]
		if v.State == "applied" && v.UsedByAttempt == "" {
			v.UsedByAttempt = attempt
		}
	}
}
