//! Pure task instruction learning helpers (revise/reject).
//!
//! These functions mirror `internal/engine/instruction_revision.go` semantics
//! for the attributed-learning slice: additions are attributed decisions with
//! cause (reason), atomicity (clone-then-commit, never mutate on `Err`), base
//! preservation (operator base / original base untouched), and hashes
//! (full before/after SHA-256 of `task.next`).
//!
//! No SQL write, planning decision or authority is produced here. Rejection
//! prepares an inbox event as data in a cloned Work; only the writer persists it.
//! The caller (principal) checks the real current scope/delivery/lease, SQL
//! active agents (`instructionIdleTx` equivalent), commits atomically and
//! emits the rejection inbox event. Protocol limits enforced here are the
//! existing Go protocol limits (8..2000 UTF-8 bytes for the addition, 4000
//! for the rejection reason), not new operator policy.
//!
use crate::{update_task_instructions, AgentLaunchError, PlanningOperation, PlanningRequest, Task};
use sha2::{Digest, Sha256};

/// Protocol limit from Go: `len(op.Next) < 8 || len(op.Next) > 2000`.
const ADDITION_MIN_BYTES: usize = 8;
const ADDITION_MAX_BYTES: usize = 2000;
/// Protocol limit from Go: `len(r.Reason) > 4000`.
const REASON_MAX_BYTES: usize = 4000;

fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

fn composed(task: &Task) -> String {
    crate::composed_task_instructions(task)
}

/// Apply an attributed planner instruction-learning addition (`revise-instructions`).
///
/// Mirrors Go `reviseTaskInstructions`: revisable only for a future (`todo`)
/// task of this scope that is not a subplanner. Task-definition and child-budget
/// fields must be empty (Go applicability;
/// unrelated judge metadata remains passive). Duplicates, even previously
/// rejected, refuse; a second
/// revision for the same (decision, task) pair refuses. On success the
/// revision is appended and `task.next` recomposed; on any `Err` the task is
/// left untouched.
pub(crate) fn revise(
    task: &mut Task,
    scope: &str,
    op: &PlanningOperation,
    request: &PlanningRequest,
    at: &str,
) -> Result<(), AgentLaunchError> {
    if task.scope_id != scope || task.status != "todo" || task.plan_role == "subplanner" {
        return Err(AgentLaunchError::Refused(
            "instruction-task-not-future-in-scope",
        ));
    }
    let addition = op.next.as_str();
    let bytes = addition.as_bytes();
    if bytes.len() < ADDITION_MIN_BYTES
        || bytes.len() > ADDITION_MAX_BYTES
        || !crate::text::nonempty(addition)
    {
        return Err(AgentLaunchError::Refused(
            "instruction-addition-size-invalid",
        ));
    }
    if !op.title.is_empty()
        || op.requirements.as_ref().map_or(0, Vec::len) != 0
        || !op.deliverable.is_empty()
        || op.criteria.as_ref().map_or(0, Vec::len) != 0
        || op.depends.as_ref().map_or(0, Vec::len) != 0
        || op.max_tasks != 0
        || op.max_activations != 0
    {
        return Err(AgentLaunchError::Refused(
            "instruction-operation-fields-invalid",
        ));
    }

    // Work on a clone so `task` is never mutated on failure (atomité).
    let mut candidate = task.clone();
    let previous = candidate.next.clone();

    // Reconcile a recognizable operator prefix through the existing crate
    // helper, operating on the clone with its old `next` intact.
    update_task_instructions(&mut candidate, &previous)
        .map_err(|e| AgentLaunchError::caused("instruction-current-context-changed", e))?;

    let revisions = candidate.instruction_revisions.as_deref().unwrap_or(&[]);
    for prior in revisions {
        if prior.addition == addition {
            return Err(AgentLaunchError::Refused(
                "instruction-addition-already-recorded",
            ));
        }
    }

    let before = candidate.next.clone();
    if candidate.instruction_base.is_none() {
        candidate.instruction_base = Some(Box::new(candidate.next.clone()));
    }

    // ID = "instruction-" + first24(SHA256(event_id + '|' + task.id)).
    let mut hasher = Sha256::new();
    hasher.update(request.event_id.as_bytes());
    hasher.update(b"|");
    hasher.update(task.id.as_bytes());
    let digest = hasher.finalize();
    let mut short = String::with_capacity(24);
    for byte in digest.iter().take(12) {
        short.push_str(&format!("{:02x}", byte));
    }
    let revision_id = format!("instruction-{}", short);

    for prior in candidate.instruction_revisions.iter().flatten() {
        if prior.id == revision_id {
            return Err(AgentLaunchError::Refused(
                "instruction-decision-already-recorded",
            ));
        }
    }

    let revision = crate::InstructionRevision {
        id: revision_id,
        addition: addition.to_owned(),
        reason: request.reason.clone(),
        scope: scope.to_owned(),
        holder: request.holder.clone(),
        generation: request.generation,
        // Clone current typed Inputs exactly (None stays None).
        input_events: request.input_events.clone(),
        decision: request.event_id.clone(),
        at: at.to_owned(),
        state: "applied".to_owned(),
        attempts_at_creation: i64::try_from(candidate.attempts.as_ref().map_or(0, Vec::len))
            .map_err(|_| AgentLaunchError::Refused("instruction-attempt-count-overflow"))?,
        before_sha256: sha256_hex(before.as_bytes()),
        after_sha256: String::new(),
        used_by_attempt: String::new(),
        rejected_at: String::new(),
        rejected_reason: String::new(),
    };

    candidate
        .instruction_revisions
        .get_or_insert_with(Vec::new)
        .push(revision);
    candidate.next = composed(&candidate);
    let after = candidate.next.clone();
    if let Some(last) = candidate
        .instruction_revisions
        .as_mut()
        .and_then(|v| v.last_mut())
    {
        last.after_sha256 = sha256_hex(after.as_bytes());
    }

    *task = candidate;
    Ok(())
}

/// Reject a previously applied instruction addition (`reject-instructions`).
///
/// Mirrors Go `rejectTaskInstructions` task-side validation: a required
/// non-empty reason (≤ 4000 bytes), no implicit AI decision (no operations,
/// no inputs, no holder), the task still `todo`, and the revision applied,
/// never delivered to an attempt, with unchanged attempt count. Marks the
/// revision `rejected` with cause and timestamp, and recomposes `task.next`.
/// Inbox event and SQL idle check remain the caller's responsibility. On any
/// `Err` the task is left untouched.
fn reject(task: &mut Task, request: &PlanningRequest, at: &str) -> Result<(), AgentLaunchError> {
    if !crate::text::nonempty(&request.reason)
        || request.reason.as_bytes().len() > REASON_MAX_BYTES
        || request.operations.as_ref().map_or(false, |o| !o.is_empty())
        || request
            .input_events
            .as_ref()
            .map_or(false, |i| !i.is_empty())
        || !request.holder.is_empty()
    {
        return Err(AgentLaunchError::Refused(
            "instruction-operator-rejection-invalid",
        ));
    }
    if !request.scope.is_empty() && request.scope != task.scope_id {
        return Err(AgentLaunchError::Refused("instruction-scope-changed"));
    }
    if task.status != "todo" {
        return Err(AgentLaunchError::Refused("instruction-task-not-future"));
    }

    let mut candidate = task.clone();
    let previous = candidate.next.clone();
    update_task_instructions(&mut candidate, &previous)
        .map_err(|e| AgentLaunchError::caused("instruction-current-context-changed", e))?;

    let attempts_len = i64::try_from(candidate.attempts.as_ref().map_or(0, Vec::len))
        .map_err(|_| AgentLaunchError::Refused("instruction-attempt-count-overflow"))?;
    let revisions = match candidate.instruction_revisions.as_deref_mut() {
        Some(v) => v,
        None => {
            return Err(AgentLaunchError::Refused("instruction-revision-not-found"));
        }
    };
    let mut found = false;
    for revision in revisions.iter_mut() {
        if revision.id != request.instruction_revision_id {
            continue;
        }
        found = true;
        if revision.state != "applied"
            || !revision.used_by_attempt.is_empty()
            || attempts_len != revision.attempts_at_creation
        {
            return Err(AgentLaunchError::Refused(
                "instruction-revision-already-used-or-rejected",
            ));
        }
        revision.state = "rejected".to_owned();
        revision.rejected_at = at.to_owned();
        revision.rejected_reason = request.reason.clone();
        break;
    }
    if !found {
        return Err(AgentLaunchError::Refused("instruction-revision-not-found"));
    }
    candidate.next = composed(&candidate);

    *task = candidate;
    Ok(())
}

/// Prepare an operator rejection and its inbox notification on a cloned Work.
/// The transaction owner must still check active task ownership and commit with
/// event/revision idempotence. No caller identity, IO or acceptance is established.
/// Missing task/scope, wrong optional scope and overflow refuse without mutation.
pub(crate) fn reject_candidate(
    work: &crate::Work,
    request: &PlanningRequest,
    at: &str,
) -> Result<crate::Work, AgentLaunchError> {
    crate::validate_planning_state(work)
        .map_err(|e| AgentLaunchError::caused("instruction-planning-state-invalid", e))?;
    if work.planning.is_none() {
        return Err(AgentLaunchError::Refused("planning-not-enabled"));
    }
    let mut next = work.clone();
    let task = next
        .tasks
        .iter_mut()
        .flatten()
        .find(|t| t.id == request.task_id)
        .ok_or(AgentLaunchError::Refused("instruction-task-not-found"))?;
    if !request.scope.is_empty() && request.scope != task.scope_id {
        return Err(AgentLaunchError::Refused("instruction-scope-changed"));
    }
    reject(task, request, at)?;
    let revision = task
        .instruction_revisions
        .iter()
        .flatten()
        .find(|v| v.id == request.instruction_revision_id)
        .ok_or(AgentLaunchError::Refused("instruction-revision-not-found"))?;
    let event = crate::PlanningEvent {
        id: request.event_id.clone(),
        scope: task.scope_id.clone(),
        kind: "instruction_rejected".into(),
        task: task.id.clone(),
        message: format!(
            "Ajout rejeté par l’utilisateur : {}\nMotif : {}",
            revision.addition, request.reason
        ),
        at: at.into(),
        ..Default::default()
    };
    let planning = next
        .planning
        .as_deref_mut()
        .ok_or(AgentLaunchError::Refused("planning-not-enabled"))?;
    let scope = planning
        .scopes
        .iter_mut()
        .flatten()
        .find(|s| s.id == event.scope)
        .ok_or(AgentLaunchError::Refused("planning-scope-not-found"))?;
    scope.revision = scope
        .revision
        .checked_add(1)
        .ok_or(AgentLaunchError::Refused(
            "instruction-scope-revision-overflow",
        ))?;
    scope.state = "ready".into();
    planning.inbox.get_or_insert_default().push(event);
    crate::validate_planning_state(&next)
        .map_err(|e| AgentLaunchError::caused("instruction-planning-state-invalid", e))?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn future() -> Task {
        Task {
            id: "future".into(),
            scope_id: "root".into(),
            status: "todo".into(),
            next: "operator base".into(),
            ..Default::default()
        }
    }
    fn addition(text: &str) -> PlanningOperation {
        PlanningOperation {
            id: "future".into(),
            kind: "revise-instructions".into(),
            next: text.into(),
            ..Default::default()
        }
    }
    fn request() -> PlanningRequest {
        PlanningRequest {
            event_id: "decision".into(),
            reason: "verified observation".into(),
            holder: "planner".into(),
            generation: 2,
            input_events: Some(vec!["bootstrap".into()]),
            ..Default::default()
        }
    }
    fn snapshot(t: &Task) -> Vec<u8> {
        crate::encode_model(t).unwrap()
    }
    #[test]
    fn learning_preserves_original_operator_and_attribution() {
        let mut t = future();
        let r = request();
        revise(
            &mut t,
            "root",
            &addition("first learning"),
            &r,
            "2026-10-25T00:00:00Z",
        )
        .unwrap();
        update_task_instructions(&mut t, "edited operator base").unwrap();
        revise(
            &mut t,
            "root",
            &addition("second learning"),
            &PlanningRequest {
                event_id: "decision-2".into(),
                ..r.clone()
            },
            "2026-10-25T00:01:00Z",
        )
        .unwrap();
        assert_eq!(
            t.instruction_base.as_deref().map(String::as_str),
            Some("operator base")
        );
        assert_eq!(
            t.next,
            "second learning\n\nfirst learning\n\nedited operator base"
        );
        let revisions = t.instruction_revisions.as_ref().unwrap();
        assert_eq!(revisions[0].input_events, r.input_events);
        assert_eq!(revisions[0].holder, "planner");
        assert_eq!(revisions[1].after_sha256, sha256_hex(t.next.as_bytes()));
        let before = snapshot(&t);
        assert!(revise(&mut t, "root", &addition("third learning"), &r, "at").is_err());
        assert_eq!(snapshot(&t), before);
    }
    #[test]
    fn learning_limits_are_utf8_bytes_and_refusals_are_atomic() {
        let mut t = future();
        revise(&mut t, "root", &addition(&"é".repeat(4)), &request(), "at").unwrap();
        for op in [
            addition("short"),
            addition(&"é".repeat(1001)),
            PlanningOperation {
                title: "unwanted".into(),
                ..addition("valid addition")
            },
        ] {
            let before = snapshot(&t);
            assert!(revise(&mut t, "root", &op, &request(), "at").is_err());
            assert_eq!(snapshot(&t), before);
        }
        let before = snapshot(&t);
        assert!(revise(
            &mut t,
            "other",
            &addition("valid addition"),
            &request(),
            "at"
        )
        .is_err());
        assert_eq!(snapshot(&t), before);
    }
    #[test]
    fn rejection_preserves_history_and_cannot_reintroduce_duplicate() {
        let mut t = future();
        revise(
            &mut t,
            "root",
            &addition("first learning"),
            &request(),
            "at",
        )
        .unwrap();
        let mut r = PlanningRequest {
            reason: "operator declines".into(),
            instruction_revision_id: t.instruction_revisions.as_ref().unwrap()[0].id.clone(),
            ..Default::default()
        };
        reject(&mut t, &r, "later").unwrap();
        assert_eq!(t.next, "operator base");
        assert_eq!(
            t.instruction_revisions.as_ref().unwrap()[0].rejected_at,
            "later"
        );
        let before = snapshot(&t);
        assert!(revise(
            &mut t,
            "root",
            &addition("first learning"),
            &request(),
            "at"
        )
        .is_err());
        assert_eq!(snapshot(&t), before);
        r.holder = "ai-planner".into();
        assert!(reject(&mut t, &r, "later").is_err());
        assert_eq!(snapshot(&t), before);
    }
    #[test]
    fn rejection_refuses_used_and_changed_attempt_count() {
        let mut base = future();
        revise(
            &mut base,
            "root",
            &addition("first learning"),
            &request(),
            "at",
        )
        .unwrap();
        let r = PlanningRequest {
            reason: "decline".into(),
            instruction_revision_id: base.instruction_revisions.as_ref().unwrap()[0].id.clone(),
            ..Default::default()
        };
        let mut used = base.clone();
        used.instruction_revisions.as_mut().unwrap()[0].used_by_attempt = "reserved".into();
        let before = snapshot(&used);
        assert!(reject(&mut used, &r, "at").is_err());
        assert_eq!(snapshot(&used), before);
        base.instruction_revisions.as_mut().unwrap()[0].attempts_at_creation = 1;
        let before = snapshot(&base);
        assert!(reject(&mut base, &r, "at").is_err());
        assert_eq!(snapshot(&base), before);
    }
}
