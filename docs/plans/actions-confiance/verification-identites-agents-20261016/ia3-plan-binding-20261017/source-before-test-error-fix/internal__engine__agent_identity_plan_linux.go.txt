//go:build linux

package engine

import (
	"bytes"
	"context"
	"database/sql"
	"encoding/json"
)

// This is a version of a canonical projection, not an execution policy or a
// source of creation rights. Unrecognized fields stay in the projection.
const agentIdentityPlanProjectionVersion = 1

func identityPlanObject(raw json.RawMessage) (map[string]json.RawMessage, error) {
	var object map[string]json.RawMessage
	err := json.Unmarshal(raw, &object)
	if err != nil || object == nil {
		return nil, agentIdentityError("agent_identity_plan_invalid")
	}
	return object, nil
}

func identityPlanOmit(object map[string]json.RawMessage, keys ...string) {
	for _, key := range keys {
		delete(object, key)
	}
}

func identityPlanNested(object map[string]json.RawMessage, key string, keys ...string) error {
	raw, ok := object[key]
	if !ok || string(raw) == "null" {
		return nil
	}
	nested, err := identityPlanObject(raw)
	if err != nil {
		return err
	}
	identityPlanOmit(nested, keys...)
	object[key], err = json.Marshal(nested)
	return err
}

// The stored contract is data, including an imported plan. Its digest does not
// attest a human adoption and never supplies a permit, root or checkpoint.
// Results, leases and counters do not change the plan definition. Instructions,
// owners, hierarchy, budgets, profiles and all unrecognized fields are retained.
func agentIdentityRecordedPlanDigest(raw []byte, work string, revision int) (string, error) {
	var w Work
	if err := json.Unmarshal(raw, &w); err != nil || w.Schema != 1 || w.ID != work || w.Revision != revision || revision <= 0 {
		return "", agentIdentityError("agent_identity_plan_invalid")
	}
	if len(w.Plans) == 0 && w.Planning == nil {
		return "", agentIdentityError("agent_identity_plan_unavailable")
	}
	for _, plan := range w.Plans {
		if _, err := validateActionPlan(plan.Spec, true); err != nil || len(plan.TaskIDs) != len(plan.Spec.Tasks) {
			return "", agentIdentityError("agent_identity_plan_invalid")
		}
		seen := map[string]bool{}
		for _, id := range plan.TaskIDs {
			if _, err := w.task(id); err != nil || seen[id] {
				return "", agentIdentityError("agent_identity_plan_invalid")
			}
			seen[id] = true
		}
	}
	if w.Planning != nil {
		if err := validatePlanningState(&w); err != nil {
			return "", agentIdentityError("agent_identity_plan_invalid")
		}
	}
	return agentIdentityPlanProjectionDigest(raw)
}

// The SQLite trigger also projects records without an adopted plan. It tracks
// changes but cannot validate adoption or create authority. Validation above is
// mandatory at the identity consumer, not while migrating historical records.
func agentIdentityPlanProjectionDigest(raw []byte) (string, error) {
	object, err := identityPlanObject(raw)
	if err != nil {
		return "", err
	}
	identityPlanOmit(object, "revision", "updated", "git", "retex", "summary", "project_memory")
	if err = identityPlanNested(object, "launch_profile", "updated", "actor"); err != nil {
		return "", err
	}
	var tasks []map[string]json.RawMessage
	if raw, ok := object["tasks"]; ok {
		if err = json.Unmarshal(raw, &tasks); err != nil {
			return "", agentIdentityError("agent_identity_plan_invalid")
		}
	}
	for _, task := range tasks {
		identityPlanOmit(task, "status", "blocker", "attempts", "gate", "auto_validation", "override", "revalidation", "evidence_stale_reason", "launch_held",
			"response", "response_error", "answers", "contexts", "planning_retry", "corrective_recovery", "recovered_result", "independent_review", "previous_reviews",
			"batch_review_resume", "review_coordination", "legacy_report_submissions", "restarts", "integration_retries", "requalifications")
		if err = identityPlanNested(task, "launch_profile", "updated", "actor"); err != nil {
			return "", err
		}
	}
	if _, ok := object["tasks"]; ok {
		object["tasks"], err = json.Marshal(tasks)
		if err != nil {
			return "", err
		}
	}
	if raw, ok := object["planning"]; ok && string(raw) != "null" {
		planning, err := identityPlanObject(object["planning"])
		if err != nil {
			return "", err
		}
		identityPlanOmit(planning, "activations", "decisions", "failure", "inbox")
		if err = identityPlanNested(planning, "repository", "candidate_commit"); err != nil {
			return "", err
		}
		if err = identityPlanNested(planning, "judge", "verdict", "reason", "evidence_ref", "at", "cycle", "closed", "history"); err != nil {
			return "", err
		}
		var scopes []map[string]json.RawMessage
		if err = json.Unmarshal(planning["scopes"], &scopes); err != nil {
			return "", agentIdentityError("agent_identity_plan_invalid")
		}
		for _, scope := range scopes {
			identityPlanOmit(scope, "revision", "generation", "holder", "lease_until", "state", "activations", "last_model", "delivery", "workflow", "rejection_wait")
		}
		planning["scopes"], err = json.Marshal(scopes)
		if err != nil {
			return "", err
		}
		object["planning"], err = json.Marshal(planning)
		if err != nil {
			return "", err
		}
	}
	canonical, err := json.Marshal(struct {
		Version int                        `json:"projection_version"`
		Work    map[string]json.RawMessage `json:"work"`
	}{agentIdentityPlanProjectionVersion, object})
	if err != nil {
		return "", err
	}
	// RawMessage preserves nested source key order. Re-decode using Number,
	// never float64, to sort every object without losing integer precision.
	decoder := json.NewDecoder(bytes.NewReader(canonical))
	decoder.UseNumber()
	var value any
	if err = decoder.Decode(&value); err != nil {
		return "", err
	}
	canonical, err = json.Marshal(value)
	if err != nil {
		return "", err
	}
	return hash(canonical), nil
}

func readAgentIdentityRecordedPlan(ctx context.Context, query agentIdentityRegistryQuery, work string) (agentIdentityScope, error) {
	var raw []byte
	var revision int
	var generation, projection sql.NullInt64
	var stored sql.NullString
	if err := query.QueryRowContext(ctx, `SELECT w.revision,w.body,p.generation,p.digest,p.projection_version FROM works w LEFT JOIN agent_identity_plan_heads p ON p.work_id=w.id WHERE w.id=?`, work).Scan(&revision, &raw, &generation, &stored, &projection); err != nil {
		if err == sql.ErrNoRows {
			return agentIdentityScope{}, agentIdentityError("agent_identity_work_missing")
		}
		return agentIdentityScope{}, err
	}
	digest, err := agentIdentityRecordedPlanDigest(raw, work, revision)
	if err != nil {
		return agentIdentityScope{}, err
	}
	if !generation.Valid || generation.Int64 < 1 || int64(int(generation.Int64)) != generation.Int64 || !stored.Valid || stored.String != digest || !projection.Valid || projection.Int64 != agentIdentityPlanProjectionVersion {
		return agentIdentityScope{}, agentIdentityError("agent_identity_plan_stamp_changed")
	}
	return agentIdentityScope{WorkID: work, PlanVersion: int(generation.Int64), PlanDigest: digest}, nil
}

// A read-only producer for a future controller approval. It authenticates an
// existing head, but neither issues a publication permit nor adopts an import.
// Even an unchanged plan still needs a fresh permit for a new publication.
func (s *Store) readAgentIdentityNextPlanScope(ctx context.Context, checkpoint agentIdentityRegistryCheckpoint) (agentIdentityScope, error) {
	var result agentIdentityScope
	if ctx == nil || !checkpoint.valid() || checkpoint.projectRoot != s.root {
		return result, agentIdentityError("agent_identity_checkpoint_required")
	}
	_, err := s.withTrustReadAvailable(func(state *trustState) (any, error) {
		tx, err := s.db.BeginTx(ctx, nil)
		if err != nil {
			return nil, err
		}
		defer tx.Rollback()
		var version int
		if err = tx.QueryRowContext(ctx, "PRAGMA user_version").Scan(&version); err != nil {
			return nil, err
		}
		if version != schemaVersion {
			return nil, agentIdentityError("storage_version_conflict")
		}
		current := state.AgentIdentities[checkpoint.workID]
		if current.Pending != nil {
			return nil, agentIdentityError("agent_identity_publication_pending")
		}
		if current.Head != nil {
			if _, _, err = loadAgentIdentityRegistry(ctx, tx, state, checkpoint); err != nil {
				return nil, err
			}
		} else {
			var count int
			if checkpoint.headDigest != "" {
				return nil, agentIdentityError("agent_identity_checkpoint_mismatch")
			}
			if err = tx.QueryRowContext(ctx, `SELECT (SELECT count(*) FROM agent_identity_snapshots WHERE work_id=?)+(SELECT count(*) FROM agent_identity_publications WHERE work_id=?)`, checkpoint.workID, checkpoint.workID).Scan(&count); err != nil {
				return nil, err
			}
			if count != 0 {
				return nil, agentIdentityError("agent_identity_import_not_authority")
			}
		}
		result, err = readAgentIdentityRecordedPlan(ctx, tx, checkpoint.workID)
		if err != nil {
			return nil, err
		}
		return nil, tx.Commit()
	})
	if err != nil {
		return agentIdentityScope{}, err
	}
	return result, nil
}
