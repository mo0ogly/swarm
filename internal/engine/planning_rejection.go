//go:build linux

package engine

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
)

// A parsed but refused proposal is new diagnostic input to its owner, not a
// mission-wide failure or an applied decision. Original response and admission
// receipt are durable; the projected diagnostic names the exact failed operation
// without pretending to supply the full raw response as review evidence.
func (s *Store) planningRejected(work, scope string, generation int, claim, reply string, cause error) error {
	rawPath := filepath.ToSlash(filepath.Join(".swarm", "planning", work, claim+"-rejected-"+hash([]byte(reply))+".json"))
	if err := os.MkdirAll(filepath.Dir(filepath.Join(s.root, rawPath)), 0700); err != nil {
		return s.planningFailure(work, scope, generation, "Stockage du refus indisponible : "+err.Error())
	}
	if err := atomicWrite(filepath.Join(s.root, rawPath), []byte(reply)); err != nil {
		return s.planningFailure(work, scope, generation, "Conservation de la proposition refusée impossible : "+err.Error())
	}
	for attempt := 0; attempt < 3; attempt++ {
		w, err := s.get(work)
		if err != nil {
			return err
		}
		if w.Planning == nil {
			return nil
		}
		owner, err := w.Planning.scope(scope)
		if err != nil {
			return err
		}
		if owner.Generation != generation || owner.Holder == "" {
			return nil
		}
		operation := map[string]any{}
		key := cause.Error()
		var failed *planningOperationError
		if errors.As(cause, &failed) {
			operation = map[string]any{"index": failed.Index + 1, "kind": failed.Kind, "id": failed.ID}
			key = failed.Kind + "\n" + failed.ID + "\n" + failed.Err.Error()
		}
		state := planningRejectionState(w, owner)
		problemKey := hash([]byte(scope + "\n" + key + "\n" + state))
		eventID := "planning-rejected-" + hash([]byte(claim+"\n"+problemKey))
		var proposed PlanningProposal
		identities := []map[string]any{}
		if json.Unmarshal([]byte(reply), &proposed) == nil {
			for index, op := range proposed.Operations {
				identities = append(identities, map[string]any{"index": index + 1, "kind": op.Kind, "id": op.ID})
			}
		}
		failedTask := ""
		if failed != nil && failed.Index >= 0 && failed.Index < len(proposed.Operations) {
			op := proposed.Operations[failed.Index]
			operation["proposal"] = op
			if task, err := w.task(op.ID); err == nil && task.ScopeID == scope {
				failedTask = task.ID
			}
		}
		diagnostic := map[string]any{"scope": scope, "generation": generation, "claim": claim, "cause": cause.Error(), "failed_operation": operation, "operation_inventory": identities, "proposed_input_events": proposed.Inputs, "raw_response": ExchangeArtifact{Path: rawPath, SHA256: hash([]byte(reply))}, "received_delivery": owner.Delivery, "workflow": owner.Workflow, "state_sha256": state, "effect": "Entire proposal refused; no operation applied and no input acknowledged. JSON response returned by the provider adapter retained intact; this diagnostic supplies its reference, exact operation identities and the complete identified failed operation as an untrusted proposal; it does not substitute for the full original response. Reassess current state and correct the proposed operation within authority. A task operation requires a new global task ID, unique within the proposal too; existing tasks are not recreated."}
		data, _ := json.Marshal(diagnostic)
		path := filepath.ToSlash(filepath.Join(filepath.Dir(rawPath), "diagnostic-"+hash(data)+".json"))
		if err = atomicWrite(filepath.Join(s.root, path), data); err != nil {
			return s.planningFailure(work, scope, generation, "Conservation du diagnostic impossible : "+err.Error())
		}
		_, err = s.mutate(work, "planning.rejected", claim+"-rejected", w.Revision, data, func(current *Work) error {
			if current.Planning == nil {
				return nil
			}
			target, err := current.Planning.scope(scope)
			if err != nil {
				return err
			}
			if target.Generation != generation || target.Holder == "" {
				return nil
			}
			if planningRejectionState(*current, target) != state {
				return planningError("revision_conflict", "préconditions du refus modifiées ; relire le contexte")
			}
			repeated := false
			for _, event := range current.Planning.Inbox {
				if event.Scope == scope && event.Kind == "planning_rejected" && event.RejectionKey == problemKey {
					repeated = true
				}
			}
			if repeated {
				target.RejectionWait = &PlanningRejectionWait{Event: eventID, StateSHA256: state}
			} else {
				target.RejectionWait = nil
			}
			current.Planning.Inbox = append(current.Planning.Inbox, PlanningEvent{ID: eventID, RejectionKey: problemKey, Scope: scope, Task: failedTask, Kind: "planning_rejected", Message: "Proposition refusée : " + cause.Error() + ". Diagnostic attribué fourni ; aucune opération ni acceptation appliquée.", Handoff: &ExchangeArtifact{Path: path, SHA256: hash(data)}, Artifacts: []ExchangeArtifact{{Path: path, SHA256: hash(data)}, {Path: rawPath, SHA256: hash([]byte(reply))}}, At: now()})
			target.Holder = ""
			target.Until = ""
			target.Generation++
			target.Revision++
			target.State = "ready"
			return nil
		})
		if err == nil {
			return fmt.Errorf("proposition refusée, diagnostic transmis à %s : %w", scope, cause)
		}
		if commandFailure(err).Code != "revision_conflict" {
			return err
		}
	}
	return fmt.Errorf("diagnostic de refus à retransmettre après conflit : %w", cause)
}
