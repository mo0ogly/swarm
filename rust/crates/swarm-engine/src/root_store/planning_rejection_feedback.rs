//! Immutable observed refusal/failure feedback. A checksum is not authorship.
use super::Receipt;
use crate::{AgentLaunchError as A, Work};
use serde::{Deserialize, Serialize};

/// An observed rejection and its complete candidate, before any SQL publication.
/// Files can outlive a failed transaction as harmless, unreferenced evidence.
pub(super) struct Prepared {
    pub work: Work,
    pub event: String,
    pub name: String,
    pub body: Vec<u8>,
}
/// Declared operation inventory is untrusted; no failed-operation index is inferred.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Diagnostic {
    schema_version: i64,
    work_id: String,
    scope_id: String,
    claim_id: String,
    generation: i64,
    observed_revision: i64,
    reason_code: String,
    state_sha256: String,
    original_receipt_sha256: String,
    proposal: Option<String>,
    #[serde(default)]
    grant_attempted: Option<bool>,
    #[serde(default)]
    failure_causes: Option<Vec<crate::native_diagnostics::NativeGateCause>>,
    operation_inventory: Vec<(String, String)>,
    effect: String,
}
/// Stable claim correlation, not an execution identity or an operator policy.
pub(super) fn event_id(claim: &str) -> String {
    format!(
        "planning-rejected-{}",
        crate::agent_launch_preflight::hash(claim.as_bytes())
    )
}
/// Copy the inventory from the already decoded proposal, preserving the codec's
/// optional/zero ID semantics. This describes supplied operations, never infers
/// which one failed. Allocation failure is typed; standard String cloning still
/// has Rust's process allocation behavior, not an aggregate memory guarantee.
pub(super) fn inventory(proposal: &crate::PlanningProposal) -> Result<Vec<(String, String)>, A> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(proposal.operations.len())
        .map_err(|e| A::caused("planning-rejection-allocation", e))?;
    for op in &proposal.operations {
        result.push((op.kind.clone(), op.id.clone()))
    }
    Ok(result)
}
/// Build feedback for the still-owned scope after a validated terminal disposition.
/// A superseded generation returns None, preserving the prior terminal-only path;
/// routing late/cancelled incidents remains separate. State/schema/codec/overflow
/// errors refuse without mutating Work. No task, input acknowledgement, financial
/// refund, automatic inference or acceptance is performed. Diagnostic facts are
/// native observations plus the complete parsed proposal when available. None
/// explicitly preserves missing output; no raw stream or failed-operation index
/// is invented. Caller validates actual reap OR explicit unadmitted revocation.
/// Optional safe cause/grant observations remain unknown for older receipts;
/// structured causes are not free-form text or an authenticated identity graph.
pub(super) fn prepare(
    work: &Work,
    receipt: &Receipt,
    inventory: Vec<(String, String)>,
    reason: &str,
    receipt_hash: &str,
    at: &str,
) -> Result<Option<Prepared>, A> {
    let row = &receipt.run;
    let proposal = receipt.proposal.as_deref();
    crate::validate_planning_state(work)
        .map_err(|e| A::caused("planning-rejection-state-invalid", e))?;
    let planning = work
        .planning
        .as_deref()
        .ok_or(A::Refused("planning-not-enabled"))?;
    let Some(scope) = planning
        .scopes
        .iter()
        .flatten()
        .find(|s| s.id == row.scope_id)
    else {
        return Ok(None);
    };
    if scope.generation != row.generation || scope.holder != row.holder {
        return Ok(None);
    }
    let state = crate::planning_rejection_state(work, &row.scope_id)
        .map_err(|e| A::caused("planning-rejection-fingerprint", e))?;
    let key = serde_json::to_vec(&(&row.scope_id, reason, &inventory, &state))
        .map_err(|e| A::caused("planning-rejection-key-encoding", e))?;
    let key = crate::agent_launch_preflight::hash(&key);
    let effect = if proposal.is_some() {
        "Entire proposal refused. No task operation or input acknowledgement applied. Reassess the complete untrusted proposal; the actual failed operation index is not established by this receipt."
    } else if receipt.exit_code.is_some() || receipt.signal.is_some() {
        "Native call failed after actual reap. No complete proposal is available and no task operation or input acknowledgement applied. Raw provider stream is not captured by this diagnostic. Reassess within authority; no retry is granted."
    } else {
        "Native call failed without a grant attempt; provider execution was not admitted. No child termination status is established. No task operation or input acknowledgement applied. Raw stream is not captured and no retry is granted."
    };
    let diagnostic = Diagnostic {
        schema_version: 1,
        work_id: work.id.clone(),
        scope_id: row.scope_id.clone(),
        claim_id: row.claim_id.clone(),
        generation: row.generation,
        observed_revision: work.revision,
        reason_code: reason.into(),
        state_sha256: state.clone(),
        original_receipt_sha256: receipt_hash.into(),
        proposal: proposal.map(str::to_owned),
        grant_attempted: receipt.grant_attempted,
        failure_causes: receipt.failure_causes.clone(),
        operation_inventory: inventory,
        effect: effect.into(),
    };
    let body =
        serde_json::to_vec(&diagnostic).map_err(|e| A::caused("planning-rejection-encoding", e))?;
    let digest = crate::agent_launch_preflight::hash(&body);
    let name = format!("planning-rejection-{digest}.json");
    let event = event_id(&row.claim_id);
    let reference = crate::ExchangeArtifact {
        path: format!(".swarm/{name}"),
        sha256: digest,
    };
    let repeated = planning.inbox.iter().flatten().any(|e| {
        e.scope == row.scope_id && e.kind == "planning_rejected" && e.rejection_key == key
    });
    let mut candidate = work.clone();
    let p = candidate
        .planning
        .as_deref_mut()
        .ok_or(A::Refused("planning-not-enabled"))?;
    let scope = p
        .scopes
        .iter_mut()
        .flatten()
        .find(|s| s.id == row.scope_id)
        .ok_or(A::Refused("planning-scope-not-found"))?;
    scope.generation = scope
        .generation
        .checked_add(1)
        .ok_or(A::Refused("planning-rejection-generation-overflow"))?;
    scope.revision = scope
        .revision
        .checked_add(1)
        .ok_or(A::Refused("planning-rejection-revision-overflow"))?;
    scope.holder.clear();
    scope.lease_until.clear();
    scope.state = "ready".into();
    scope.rejection_wait = if repeated {
        Some(Box::new(crate::PlanningRejectionWait {
            event_id: event.clone(),
            state_sha256: state,
        }))
    } else {
        None
    };
    let inbox = p.inbox.get_or_insert_with(Vec::new);
    inbox
        .try_reserve(1)
        .map_err(|e| A::caused("planning-rejection-allocation", e))?;
    let message = if proposal.is_some() {
        format!("Native proposal refused ({reason}). Complete untrusted proposal and observed diagnostic attached; reassess within authority.")
    } else {
        format!("Native call failed ({reason}). No complete proposal is available in this receipt. Observed diagnostic attached; reassess within authority.")
    };
    inbox.push(crate::PlanningEvent {
        id: event.clone(),
        scope: row.scope_id.clone(),
        kind: "planning_rejected".into(),
        rejection_key: key,
        message,
        handoff: Some(Box::new(reference.clone())),
        artifacts: Some(vec![reference]),
        at: at.into(),
        ..Default::default()
    });
    candidate.revision = work
        .revision
        .checked_add(1)
        .ok_or(A::Refused("planning-rejection-work-overflow"))?;
    candidate.updated = at.into();
    crate::validate_planning_state(&candidate)
        .map_err(|e| A::caused("planning-rejection-result-invalid", e))?;
    Ok(Some(Prepared {
        work: candidate,
        event,
        name,
        body,
    }))
}
/// Publish exact private bytes without replacement, under the saved report cap.
/// Existing identical bytes are re-synchronized; differences and publication,
/// read/close/fsync failures refuse. A racing new name is not silently retried.
/// The length equality guard is representation integrity, not a second policy.
pub(super) fn publish(
    directory: &crate::local_storage::LocalDirectory,
    value: &Prepared,
    cap: Option<usize>,
) -> Result<(), A> {
    if cap.is_some_and(|n| value.body.len() > n) {
        return Err(A::Refused("planning-rejection-report-limit"));
    }
    if directory
        .names()
        .map_err(|e| A::caused("planning-rejection-storage", e))?
        .iter()
        .any(|n| n.as_slice() == value.name.as_bytes())
    {
        let old = directory
            .read_capped(&value.name, Some(value.body.len()))
            .map_err(|e| A::caused("planning-rejection-read", e))?;
        if old != value.body {
            return Err(A::Refused("planning-rejection-artifact-conflict"));
        }
        directory
            .synchronize_existing(&value.name)
            .map_err(|e| A::caused("planning-rejection-sync", e))
    } else {
        directory
            .publish_new_with_fault(
                &value.name,
                &value.body,
                crate::local_storage::AtomicStorageFault::None,
            )
            .map_err(|e| A::caused("planning-rejection-publish", e))
    }
}
/// Verify a terminal refusal/failure handoff against its original receipt.
/// This reads immutable event/artifact bytes without inference or Work mutation.
/// Legacy terminal-only receipts have no delivered feedback and refuse explicitly;
/// their adoption/routing remains a data-migration requirement, never implied here.
pub(super) fn verify(
    db: &rusqlite::Connection,
    directory: &crate::local_storage::LocalDirectory,
    receipt: &Receipt,
    hash: &str,
    reason: &str,
    cap: Option<usize>,
) -> Result<(), A> {
    let raw = crate::agent_launch::query(
        db,
        "SELECT request FROM events WHERE id=? AND work_id=? AND kind='planning.rejected'",
        rusqlite::params![event_id(&receipt.run.claim_id), receipt.run.work_id],
        |rows| Ok(rows.next()?.map(|r| r.get::<_, Vec<u8>>(0)).transpose()?),
    )?
    .ok_or(A::Refused("planning-rejection-feedback-missing"))?;
    let value: Diagnostic = serde_json::from_slice(&raw)
        .map_err(|e| A::caused("planning-rejection-record-invalid", e))?;
    if value.schema_version != 1
        || value.work_id != receipt.run.work_id
        || value.scope_id != receipt.run.scope_id
        || value.claim_id != receipt.run.claim_id
        || value.generation != receipt.run.generation
        || value.original_receipt_sha256 != hash
        || value.reason_code != reason
        || value.proposal.as_deref() != receipt.proposal.as_deref()
        || value.grant_attempted != receipt.grant_attempted
        || value.failure_causes != receipt.failure_causes
    {
        return Err(A::Refused("planning-rejection-record-conflict"));
    }
    let name = format!(
        "planning-rejection-{}.json",
        crate::agent_launch_preflight::hash(&raw)
    );
    let file = directory
        .read_capped(&name, cap)
        .map_err(|e| A::caused("planning-rejection-read", e))?;
    if file != raw {
        return Err(A::Refused("planning-rejection-artifact-conflict"));
    }
    Ok(())
}
