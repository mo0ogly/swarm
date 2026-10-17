//go:build linux

package engine

import (
	"bytes"
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"time"
)

type agentIdentityRegistryQuery interface {
	QueryRowContext(context.Context, string, ...any) *sql.Row
}

// Current authority never comes from SELECT max(revision). The private
// checkpoint authenticates the independent Trust head; its exact snapshot is
// then decoded into owned maps. Historical rows or imported data cannot advance
// this checkpoint. A real external checkpoint issuer/recovery UI is still IA3.
func loadAgentIdentityRegistry(ctx context.Context, query agentIdentityRegistryQuery, state *trustState, checkpoint agentIdentityRegistryCheckpoint) (agentIdentitySnapshot, agentIdentityRegistryHead, error) {
	var snapshot agentIdentitySnapshot
	var head agentIdentityRegistryHead
	if ctx == nil || !checkpoint.valid() || checkpoint.headDigest == "" {
		return snapshot, head, agentIdentityError("agent_identity_checkpoint_required")
	}
	current := state.AgentIdentities[checkpoint.workID]
	if current.Pending != nil {
		return snapshot, head, agentIdentityError("agent_identity_publication_pending")
	}
	if current.Head == nil {
		return snapshot, head, agentIdentityError("agent_identity_authority_unavailable")
	}
	head = *current.Head
	digest, err := agentIdentityDigest(head)
	if err != nil || !head.valid() || digest != checkpoint.headDigest || head.ProjectDigest != hash([]byte(checkpoint.projectRoot)) || head.Scope.WorkID != checkpoint.workID {
		return snapshot, head, agentIdentityError("agent_identity_checkpoint_mismatch")
	}
	var version int
	if err = query.QueryRowContext(ctx, "PRAGMA user_version").Scan(&version); err != nil {
		return snapshot, head, err
	}
	if version != schemaVersion {
		return snapshot, head, agentIdentityError("storage_version_conflict")
	}
	var body []byte
	var storedDigest string
	var length int
	err = query.QueryRowContext(ctx, `SELECT digest,length(body),CASE WHEN length(body)<=? THEN body ELSE NULL END FROM agent_identity_snapshots WHERE work_id=? AND revision=?`, trustStateMaxBytes, checkpoint.workID, head.Revision).Scan(&storedDigest, &length, &body)
	if err != nil {
		return snapshot, head, fmt.Errorf("instantané d'identité indisponible : %w", err)
	}
	if length <= 0 || length > trustStateMaxBytes || storedDigest != head.SnapshotDigest {
		return snapshot, head, agentIdentityError("agent_identity_snapshot_changed")
	}
	snapshot, err = decodeAgentIdentitySnapshot(body)
	if err != nil {
		return snapshot, head, err
	}
	canonical, err := json.Marshal(snapshot)
	if err != nil || !bytes.Equal(body, canonical) {
		return snapshot, head, agentIdentityError("agent_identity_snapshot_changed")
	}
	if err = validateAgentIdentityRegistrySnapshot(snapshot, head); err != nil {
		return snapshot, head, err
	}
	if err = verifyAgentIdentityRegistryChain(ctx, query, head); err != nil {
		return snapshot, head, err
	}
	// No caller can mutate state/head roots by retaining a returned slice.
	head.Roots = append([]string(nil), head.Roots...)
	return snapshot, head, nil
}

func verifyAgentIdentityRegistryChain(ctx context.Context, query agentIdentityRegistryQuery, head agentIdentityRegistryHead) error {
	current := head
	for revision := head.Revision; revision > 0; revision-- {
		if !current.valid() || current.Revision != revision || current.Scope.WorkID != head.Scope.WorkID || current.ProjectDigest != head.ProjectDigest {
			return agentIdentityError("agent_identity_registry_history_changed")
		}
		expected, err := json.Marshal(current)
		if err != nil {
			return err
		}
		digest := hash(expected)
		var body []byte
		var event string
		if err = query.QueryRowContext(ctx, `SELECT event_id,CASE WHEN length(body)<=? THEN body ELSE NULL END FROM agent_identity_publications WHERE work_id=? AND head_digest=?`, trustStateMaxBytes, head.Scope.WorkID, digest).Scan(&event, &body); err != nil {
			return fmt.Errorf("journal d'identité incomplet : %w", err)
		}
		if !bytes.Equal(body, expected) || event != current.EventID {
			return agentIdentityError("agent_identity_registry_history_changed")
		}
		if revision == 1 {
			if current.Previous != "" {
				return agentIdentityError("agent_identity_registry_history_changed")
			}
			return nil
		}
		if !agentIdentityHashValid(current.Previous) {
			return agentIdentityError("agent_identity_registry_history_changed")
		}
		if err = query.QueryRowContext(ctx, `SELECT CASE WHEN length(body)<=? THEN body ELSE NULL END FROM agent_identity_publications WHERE work_id=? AND head_digest=?`, trustStateMaxBytes, head.Scope.WorkID, current.Previous).Scan(&body); err != nil {
			return err
		}
		var previous agentIdentityRegistryHead
		if hash(body) != current.Previous || strict(body, &previous) != nil {
			return agentIdentityError("agent_identity_registry_history_changed")
		}
		current = previous
	}
	return agentIdentityError("agent_identity_registry_history_changed")
}

func agentIdentityRegistryAuthority(head agentIdentityRegistryHead, root string) agentIdentityAuthority {
	roots := make(map[string]bool, len(head.Roots))
	for _, id := range head.Roots {
		roots[id] = true
	}
	return agentIdentityAuthority{projectRoot: root, snapshotDigest: head.SnapshotDigest, snapshotRevision: head.Revision, trustRevision: head.TrustRevision, scope: head.Scope, roots: roots}
}

// A deferred read transaction keeps schema/snapshot/journal reads together.
// Inspection releases its connection after copying. Message decisions use the
// writer reservation below to serialize Work amendments through evaluation.
func (s *Store) readAgentIdentityRegistrySnapshot(ctx context.Context, state *trustState, checkpoint agentIdentityRegistryCheckpoint) (agentIdentitySnapshot, agentIdentityRegistryHead, error) {
	var snapshot agentIdentitySnapshot
	var head agentIdentityRegistryHead
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return snapshot, head, err
	}
	defer tx.Rollback()
	snapshot, head, err = loadCurrentAgentIdentityRegistry(ctx, tx, state, checkpoint)
	if err != nil {
		return agentIdentitySnapshot{}, agentIdentityRegistryHead{}, err
	}
	if err = tx.Commit(); err != nil {
		return agentIdentitySnapshot{}, agentIdentityRegistryHead{}, err
	}
	return snapshot, head, nil
}

func loadCurrentAgentIdentityRegistry(ctx context.Context, query agentIdentityRegistryQuery, state *trustState, checkpoint agentIdentityRegistryCheckpoint) (agentIdentitySnapshot, agentIdentityRegistryHead, error) {
	snapshot, head, err := loadAgentIdentityRegistry(ctx, query, state, checkpoint)
	if err != nil {
		return agentIdentitySnapshot{}, agentIdentityRegistryHead{}, err
	}
	scope, err := readAgentIdentityRecordedPlan(ctx, query, checkpoint.workID)
	if err != nil {
		return agentIdentitySnapshot{}, agentIdentityRegistryHead{}, err
	}
	if head.Scope != scope {
		return agentIdentitySnapshot{}, agentIdentityRegistryHead{}, agentIdentityError("agent_identity_plan_mismatch")
	}
	return snapshot, head, nil
}

// Normal reading/evaluation rejects pending publications and stale Trust
// revisions. No snapshot or authority frame is accepted from the message.
func (s *Store) evaluateAgentIdentityRegistryMessage(ctx context.Context, checkpoint agentIdentityRegistryCheckpoint, message *agentIdentityMessage, runtime *agentIdentityRuntime, previous *agentIdentityDecision, at time.Time) (agentIdentityDecision, error) {
	var decision agentIdentityDecision
	if ctx == nil || checkpoint.projectRoot != s.root || message == nil {
		return decision, agentIdentityError("agent_identity_checkpoint_required")
	}
	request, err := decodeAgentIdentityRequest(message.raw)
	if err != nil {
		return decision, err
	}
	_, err = s.withTrustReadAvailable(func(state *trustState) (any, error) {
		tx, err := s.db.BeginTx(ctx, nil)
		if err != nil {
			return nil, err
		}
		defer tx.Rollback()
		// The plan can change without taking Trust. Reserve its SQLite writer
		// until the pure runtime decision completes: Trust -> SQLite -> runtime.
		// This updates no Work body and triggers no plan generation change.
		if _, err = tx.ExecContext(ctx, `UPDATE works SET revision=revision WHERE id=?`, checkpoint.workID); err != nil {
			return nil, err
		}
		snapshot, head, err := loadCurrentAgentIdentityRegistry(ctx, tx, state, checkpoint)
		if err != nil {
			return nil, err
		}
		if head.TrustRevision != state.Revision {
			return nil, agentIdentityError("agent_identity_registry_stale")
		}
		authority := agentIdentityRegistryAuthority(head, s.root)
		// This closed callback does not call Store or another SQL operation.
		// Future consumers must preserve the same lock order, never reenter DB.
		err = runtime.withCaller(message, s.root, state.Revision, at, func(caller agentIdentityCaller) error {
			if err := ctx.Err(); err != nil {
				return err
			}
			if previous != nil {
				decision, err = reuseAgentIdentityDecision(*previous, snapshot, authority, caller, request, caller.verifiedAt)
			} else {
				decision, err = evaluateAgentIdentity(snapshot, authority, caller, request, caller.verifiedAt)
			}
			return err
		})
		if err != nil {
			return nil, err
		}
		return nil, tx.Commit()
	})
	if err != nil {
		return agentIdentityDecision{}, err
	}
	return decision, err
}

// Source read for trusted controller inspection. This grants no business action.
func (s *Store) readAgentIdentityRegistry(ctx context.Context, checkpoint agentIdentityRegistryCheckpoint) (agentIdentitySnapshot, agentIdentityAuthority, error) {
	var snapshot agentIdentitySnapshot
	var authority agentIdentityAuthority
	if ctx == nil || checkpoint.projectRoot != s.root {
		return snapshot, authority, agentIdentityError("agent_identity_checkpoint_required")
	}
	_, err := s.withTrustReadAvailable(func(state *trustState) (any, error) {
		var head agentIdentityRegistryHead
		var err error
		snapshot, head, err = s.readAgentIdentityRegistrySnapshot(ctx, state, checkpoint)
		if err != nil {
			return nil, err
		}
		if head.TrustRevision != state.Revision {
			return nil, agentIdentityError("agent_identity_registry_stale")
		}
		authority = agentIdentityRegistryAuthority(head, s.root)
		return nil, nil
	})
	if err != nil {
		return agentIdentitySnapshot{}, agentIdentityAuthority{}, err
	}
	return snapshot, authority, nil
}
