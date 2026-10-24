//! Candidate immutable private reviewer observations, never public acceptance.
use super::{capture, guard, process::Observation, store::Job};
use crate::{
    root_store::{RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use serde::{Deserialize, Serialize};
use std::os::unix::process::ExitStatusExt;

/// Original private data. Reading these fields does not authenticate provider
/// declarations, settle spending or permit applying a review. Fresh publication
/// must independently bind the current ledger, inputs, policy and exact reply.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub schema_version: i64,
    pub job: Job,
    pub finished_at: String,
    pub process_attempted: bool,
    pub grant_attempted: bool,
    pub channel_closed: bool,
    pub waited: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub unresolved_pid: Option<u32>,
    pub ownership_lost_pid: Option<u32>,
    /// Call/transport/domain failure, not the criterion verdict: a valid
    /// changes-requested reply is not a failed call and still accepts no task.
    pub failed: bool,
    pub output_complete: bool,
    pub duration_expired: bool,
    pub drain_expired: bool,
    pub source_interrupted: bool,
    pub original_reply: Option<Vec<u8>>,
    pub provider_facts: crate::GoRaw,
    /// Version1 originals have unknown raw capture. Version2 requires actual
    /// saved-policy capture evidence, never fabricated empty-success defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_capture: Option<capture::Snapshot>,
    pub causes: Vec<crate::native_diagnostics::NativeGateCause>,
    pub gate: Option<crate::agent_execution_gate::GateFailure>,
}
/// Preserve original codec/clock causes behind a static private journal family.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
/// File-format addressing by complete engine ID digest, not a user path,
/// retention policy, identity assertion or authorization to publish a verdict.
pub(super) fn name(id: &str) -> String {
    format!(
        "native-review-{}.receipt",
        crate::provider_configuration::configuration_digest(id.as_bytes())
    )
}
/// Check structural consistency only. Timestamp/exit ranges are wire/OS domains,
/// never operator cutoffs. This validates no current source, model or authority;
/// received journal bytes remain untrusted data for a later publication fence.
fn validate(value: &Receipt) -> Result<(), E> {
    if !matches!(value.schema_version, 1 | 2)
        || !swarm_identity_contract::name(value.job.id.as_bytes())
        || (value.schema_version == 1 && value.raw_capture.is_some())
        || (value.schema_version == 2 && value.raw_capture.is_none())
    {
        return Err(E::Refused("review-receipt-schema-or-id"));
    }
    if let Some(raw) = &value.raw_capture {
        raw.validate()
            .map_err(|e| caused("review-receipt-capture", e))?;
        if raw.failed && (!value.failed || value.output_complete) {
            return Err(E::Refused("review-receipt-capture-failure"));
        }
    }
    swarm_wire_json::go_rfc3339::parse(&value.finished_at)
        .map_err(|e| caused("review-receipt-clock", e))?;
    let status_fields =
        usize::from(value.exit_code.is_some()) + usize::from(value.signal.is_some());
    if status_fields != usize::from(value.waited)
        || (!value.process_attempted
            && (value.waited
                || value.grant_attempted
                || value.unresolved_pid.is_some()
                || value.ownership_lost_pid.is_some()
                || value.job.child_pid.is_some()))
        || value.exit_code.is_some_and(|c| !(0..=255).contains(&c))
        || value.signal.is_some_and(|s| s <= 0)
        || value.unresolved_pid == Some(0)
        || value.ownership_lost_pid == Some(0)
        || (value.waited && (value.unresolved_pid.is_some() || value.ownership_lost_pid.is_some()))
        || (value.grant_attempted
            && (!value.process_attempted
                || value.job.permission != 1
                || value.job.child_pid.is_none()))
        || (!value.failed
            && (!value.process_attempted
                || !value.grant_attempted
                || !value.channel_closed
                || !value.waited
                || value.exit_code != Some(0)
                || !value.output_complete
                || value.original_reply.is_none()
                || value.duration_expired
                || value.drain_expired
                || value.source_interrupted
                || value.gate.is_some()))
    {
        return Err(E::Refused("review-receipt-inconsistent"));
    }
    Ok(())
}
/// Freeze actual observations without consuming their typed errors/owned Child.
/// Exact reply bytes are already admitted by the role cap; malformed/domain/IO
/// originals may remain evidence but can never make output_complete true.
/// Facts are original unverified declarations, not a cost0 or invoice. Caller
/// retains the original Observation and parse error for cleanup/recovery. Raw
/// stdout/stderr capture comes from the actual streaming adapters; intentional
/// truncation is separate from capture/parse failure. Legacy version1 remains
/// readable with unknown capture, while this producer always emits version2.
/// No SQL, process, grant, receipt write, acceptance or refund happens here.
pub(super) fn observe(
    observation: &Observation,
    original_reply: Option<&[u8]>,
    provider_facts: crate::GoRaw,
    parse_error: Option<&A>,
    finished_at: String,
    raw_capture: capture::Snapshot,
) -> Result<Receipt, E> {
    let mut causes = Vec::new();
    let (status, unresolved_pid, ownership_lost_pid) = match &observation.process {
        Ok(run) => (Some(run.usage.status), None, None),
        Err(error) => {
            causes.extend(crate::native_diagnostics::project(error));
            (
                error.reaped_exit_status(),
                error.unresolved_process_id(),
                error.ownership_lost_process_id(),
            )
        }
    };
    for error in observation
        .transport_errors
        .iter()
        .chain(observation.interruption.iter())
    {
        causes.extend(crate::native_diagnostics::project(error));
    }
    if let Some(error) = parse_error {
        causes.extend(crate::native_diagnostics::project(error));
    }
    let failed = raw_capture.failed
        || observation.process.is_err()
        || status.is_none_or(|s| !s.success())
        || !observation.grant_attempted
        || !observation.channel_closed
        || observation.interruption.is_some()
        || observation.duration_expired
        || observation.drain_expired
        || observation.gate.is_some()
        || !observation.transport_errors.is_empty()
        || parse_error.is_some()
        || original_reply.is_none();
    let gate = observation
        .gate
        .as_ref()
        .map(|g| crate::agent_execution_gate::GateFailure {
            schema_version: g.schema_version,
            kind: g.kind.clone(),
            code: g.code.clone(),
            io_errno: g.io_errno,
            sqlite_code: g.sqlite_code,
            causes_scope: g.causes_scope.clone(),
            causes: g.causes.clone(),
        });
    let value = Receipt {
        schema_version: 2,
        job: observation.job.clone(),
        finished_at,
        process_attempted: true,
        grant_attempted: observation.grant_attempted,
        channel_closed: observation.channel_closed,
        waited: status.is_some(),
        exit_code: status.and_then(|s| s.code()),
        signal: status.and_then(|s| s.signal()),
        unresolved_pid,
        ownership_lost_pid,
        failed,
        output_complete: !failed,
        duration_expired: observation.duration_expired,
        drain_expired: observation.drain_expired,
        source_interrupted: observation.interruption.is_some(),
        original_reply: original_reply.map(<[u8]>::to_vec),
        provider_facts,
        raw_capture: Some(raw_capture),
        causes,
        gate,
    };
    validate(&value)?;
    Ok(value)
}
/// Bind the original private directory only. Late revoked-input observations
/// must still be preservable: this does not renew admission or require stale
/// Work to look current. A later publication fence owns current ledger checks.
fn root_binding(root: &RootStore, value: &Receipt) -> Result<(), E> {
    let (device, inode) = root.directory.identity().map_err(E::Storage)?;
    if device.to_string() != value.job.directory_device
        || inode.to_string() != value.job.directory_inode
    {
        return Err(E::Refused("review-receipt-root-changed"));
    }
    Ok(())
}
/// Create-only framed private publication by the actual original foreground
/// owner, syncing complete bytes and directory before any terminal SQL update.
/// Existing data is never replaced. IO/fsync uncertainty stays an error for
/// explicit readback; it is not proof of no write, refund or restart permission.
/// A stale source may be recorded as failure evidence; no business result is
/// applied and current ledger/source freshness is still required for publication.
pub(super) fn persist(root: &RootStore, value: &Receipt) -> Result<String, E> {
    if !root.writable {
        return Err(E::Refused("root-read-only"));
    }
    validate(value)?;
    root_binding(root, value)?;
    guard::owner(&value.job)?;
    let raw = serde_json::to_vec(value).map_err(|e| caused("review-receipt-encode", e))?;
    let frame =
        crate::native_receipt::encode(&raw).map_err(|e| caused("review-receipt-frame", e))?;
    root.directory
        .publish_new_with_fault(
            &name(&value.job.id),
            &frame,
            crate::local_storage::AtomicStorageFault::None,
        )
        .map_err(E::Storage)?;
    Ok(crate::provider_configuration::configuration_digest(&raw))
}
/// Read strict framed data anchored to this Root and expected engine job ID.
/// Extent/checksum/regular-file/owner/close, JSON/schema/consistency/root mismatches
/// refuse with original causes. No creator-liveness check is used for reading:
/// recovery still owes independent owner/child absence and current authority.
/// Returned booleans/reply/facts grant no permission and may not be trusted as a
/// successful current review without the future fresh publication fence.
pub(super) fn read(root: &RootStore, id: &str) -> Result<(Receipt, String), E> {
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(E::Refused("review-receipt-id"));
    }
    let raw = root
        .directory
        .read_native_receipt(&name(id))
        .map_err(E::Storage)?;
    let value: Receipt =
        serde_json::from_slice(&raw).map_err(|e| caused("review-receipt-decode", e))?;
    validate(&value)?;
    root_binding(root, &value)?;
    if value.job.id != id {
        return Err(E::Refused("review-receipt-job-changed"));
    }
    Ok((
        value,
        crate::provider_configuration::configuration_digest(&raw),
    ))
}
