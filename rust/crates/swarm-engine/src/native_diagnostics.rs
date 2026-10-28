//! Safe native error branches; no private Display text or execution authority.
use serde::{Deserialize, Serialize};
use std::{error::Error, io};

/// Preserve one concrete boxed adapter at each visited graph node. The inner
/// typed error is visited next instead of Box::source skipping its own fields.
/// Every registered handler uses this lookup, so new native handlers inherit the
/// same supported box behavior. Opaque external/nested boxes still follow their
/// declared source; absent metadata remains unknown, never no-effect evidence.
fn native_ref<'a,T:Error+'static>(error:&'a(dyn Error+'static),result:&mut NativeErrorLinks<'a>,explicit:&mut bool)->Option<&'a T>{
    if let Some(value)=error.downcast_ref::<T>(){return Some(value)}
    if let Some(value)=error.downcast_ref::<Box<T>>(){result.children.push(("source",value.as_ref()));*explicit=true;}
    None
}

/// Safe labels/numeric observations and borrowed known error branches. These
/// references expire with the error; no diagnostic data can authorize a retry,
/// refund, mutation, process or business acceptance. Unknown types use source().
pub struct NativeErrorLinks<'a> {
    pub labels: Vec<String>,
    pub io_errno: Option<i32>,
    pub sqlite_code: Option<i32>,
    pub children: Vec<(&'static str, &'a (dyn Error + 'static))>,
}
/// Classify a retained PAEA observation without rendering receipt/authority data.
/// These labels grant no retry, independent receipt or current authority.
#[cfg(target_os="linux")]
fn settlement_outcome_label(outcome:&crate::PaeaSettlementOutcome)->&'static str{
    match outcome {crate::PaeaSettlementOutcome::Recorded{replayed:false,..}=>"paea-settlement-recorded",crate::PaeaSettlementOutcome::Recorded{replayed:true,..}=>"paea-settlement-replayed",crate::PaeaSettlementOutcome::ReconciledNotCommitted{..}=>"paea-settlement-reconciled-not-committed"}
}
/// Inspect known compound native families without rendering private messages.
/// Root/launch/work/storage/process/mission/identity/budget cleanup branches are explicit;
/// std IO inner errors are visited even when source() skips their wrapper. This
/// is a projection of listed families, not a proof about custom Error types.
/// Organization, review-evidence and task-lifecycle refusals retain code and
/// static reason; no private Display is projected. The private recorded
/// reviewer failure is never rendered. Recorded transaction/event/work families
/// expose static codes and preserve independent primary/cleanup branches, including
/// recovery without a cleanup error. Context/workflow/control reasons use only
/// their static typed vocabulary; context cleanup branches remain traversable.
/// Dispatch failures expose their primary, committed queue permission/readiness
/// causes and retained resource causes separately; phase/effect labels use a
/// closed vocabulary and never include operation, task or workspace identifiers.
/// Telemetry exposes its known token-insert marker and independent primary/cleanup
/// branches, including Root/storage/custom-IO causes, never token or subject data.
/// Connected SQLite identity errors expose only their version-pinned enum phase
/// and known primary/cleanup sources. SQL input errors preserve numeric extended
/// codes like SqliteFailure, never their SQL, parser offset or message strings.
/// Continuation failures expose their snapshot cause and every retained per-entry
/// refusal, including cleanup through their existing wrappers. Progress has a
/// closed vocabulary; unknown phases and agent/task identifiers are not rendered.
/// A completed but unusable observation never implies a failed cleanup; final
/// physical revalidation can also invalidate it after successful cleanup.
/// The public boxed failure is explicitly unwrapped before its source() can skip
/// the per-entry branches; direct/custom-IO/boxed paths share the same projection.
/// Boxed Root errors are unwrapped before delegated source() can skip an
/// auxiliary/known-state receipt; metadata itself is never added to safe labels.
/// These labels establish no availability,
/// repaired configuration, actor identity or permission to retry/reset counters.
/// Linux peer credential errors retain real syscall errno separately from invalid
/// raw PID/UID/GID observations; no errno0 or credential values are manufactured.
pub fn native_error_links<'a>(e: &'a (dyn Error + 'static)) -> NativeErrorLinks<'a> {
    let mut result = NativeErrorLinks {
        labels: vec![],
        io_errno: None,
        sqlite_code: None,
        children: vec![],
    };
    let mut explicit = false;
    #[cfg(target_os="linux")]
    if let Some(error)=native_ref::<crate::GitReferenceError>(e,&mut result,&mut explicit){
        result.labels.push(error.code().into());result.labels.push(error.observation.code().into());
        result.children.extend(error.branches().into_iter().map(|cause|("source",cause)));explicit=true;
    }
    #[cfg(target_os="linux")]
    if let Some(error)=native_ref::<crate::PaeaGitIntentError>(e,&mut result,&mut explicit){
        result.labels.push(error.code.into());
        result.labels.push(match &error.observation{
            crate::PaeaGitIntentObservation::NotAttempted=>"paea-git-intent-not-attempted",
            crate::PaeaGitIntentObservation::Uncertain=>"paea-git-intent-uncertain",
            crate::PaeaGitIntentObservation::Known(crate::PaeaGitIntentReservation::New(_))=>"paea-git-intent-known-new",
            crate::PaeaGitIntentObservation::Known(crate::PaeaGitIntentReservation::Existing(_))=>"paea-git-intent-known-existing",
        }.into());
        result.children.push(("primary",error.cause.as_ref()));
        result.children.extend(error.cleanup.iter().map(|cause|("cleanup",cause as &(dyn Error+'static))));explicit=true;
    }
    #[cfg(target_os="linux")]
    if let Some(error)=native_ref::<crate::PaeaSettlementCallError>(e,&mut result,&mut explicit){
        result.labels.push(error.code().into());
        if let Some(known)=error.known_outcome(){result.labels.push(settlement_outcome_label(known).into());}
        else if error.commit_uncertain(){result.labels.push("paea-settlement-commit-uncertain".into());}
        result.children.extend(error.branches().into_iter().map(|cause|("source",cause)));explicit=true;
    }
    #[cfg(target_os="linux")]
    if let Some(error)=native_ref::<crate::PaeaSettlementError>(e,&mut result,&mut explicit){
        result.labels.push(error.code.into());
        result.labels.push(match &error.observation{
            crate::PaeaSettlementObservation::NotAttempted=>"paea-settlement-commit-not-attempted",
            crate::PaeaSettlementObservation::Uncertain=>"paea-settlement-commit-uncertain",
            crate::PaeaSettlementObservation::Known(known)=>settlement_outcome_label(known),
        }.into());
        result.children.push(("primary",error.cause.as_ref()));
        result.children.extend(error.cleanup.iter().map(|cause|("cleanup",cause as &(dyn Error+'static))));explicit=true;
    }

    #[cfg(target_os = "linux")]
    if let Some(error) = native_ref::<crate::TrustEnrollmentCallError>(e, &mut result, &mut explicit) {
        result.labels.push(error.code().into());
        result.children.extend(error.branches().into_iter().map(|cause| ("source", cause)));
        explicit = true;
    }
    #[cfg(target_os="linux")]
    if let Some(error)=native_ref::<crate::root_store::ObservedSqlFailure>(e, &mut result, &mut explicit) {
        result.labels.push(error.phase().into());
        result.labels.push(match error.effect() {crate::root_store::SqlEffect::NoMutation=>"sql-no-mutation-confirmed",crate::root_store::SqlEffect::Uncertain=>"sql-effect-uncertain"}.into());
        result.children.extend(error.children());
        explicit=true;
    }
    #[cfg(target_os = "linux")]
    if let Some(error) = native_ref::<crate::linux_peer_credentials::Error>(e, &mut result, &mut explicit) {
        result.labels.push(error.code().into());
        result.io_errno = error.os_errno();
        if let Some(cause) = error.source() { result.children.push(("linux-peer-credentials", cause)); }
        explicit = true;
    }


    #[cfg(target_os = "linux")]
    if let Some(e) = native_ref::<crate::MissionContinuationFailure>(e, &mut result, &mut explicit) {
        use crate::{MissionContinuationProgress as P, MissionWorkerContinuationState as S};
        result.labels.push("mission-continuation-failed".into());
        result.labels.push(match e.phase {
            "clock" => "continuation-phase:clock",
            "binding" => "continuation-phase:binding",
            "settings-reservation" => "continuation-phase:settings-reservation",
            "settings" => "continuation-phase:settings",
            "snapshot" => "continuation-phase:snapshot",
            "lease" => "continuation-phase:lease",
            "storage-version" => "continuation-phase:storage-version",
            "work" => "continuation-phase:work",
            "intentions" => "continuation-phase:intentions",
            "snapshot-final-binding" => "continuation-phase:snapshot-final-binding",
            "final-binding" => "continuation-phase:final-binding",
            _ => "continuation-phase:unknown",
        }.into());
        result.labels.push(match e.progress {
            P::NotStarted => "continuation-progress:not-started",
            P::PartialNotUsable => "continuation-progress:partial-not-usable",
            P::CompleteButUnusable => "continuation-progress:complete-unusable",
        }.into());
        result.children.push(("primary", &e.cause));
        for entry in &e.entries {
            if let S::Refused { cause, .. } = &entry.state {
                result.children.push(("source", cause));
            }
        }
        explicit = true;
    }
    #[cfg(target_os = "linux")]
    if let Some(e) = native_ref::<crate::MissionWorkerDispatchFailure>(e, &mut result, &mut explicit) {
        use crate::{
            MissionDispatchPreparationEffect as P, MissionDispatchQueueEffect as Q,
            MissionDispatchResourceRefusal as R,
        };
        result.labels.push("mission-dispatch-failed".into());
        let phase = match e.phase {
            "request" => "dispatch-phase:request",
            "binding" => "dispatch-phase:binding",
            "replay" => "dispatch-phase:replay",
            "work" => "dispatch-phase:work",
            "revision" => "dispatch-phase:revision",
            "synchronization" => "dispatch-phase:synchronization",
            "synchronized-revision" => "dispatch-phase:synchronized-revision",
            "queue-observation" => "dispatch-phase:queue-observation",
            "queue-observation-revision" => "dispatch-phase:queue-observation-revision",
            "resource-preview" => "dispatch-phase:resource-preview",
            "admission" => "dispatch-phase:admission",
            _ => "dispatch-phase:unknown",
        };
        result.labels.push(phase.into());
        result.labels.push(
            match &e.preparation {
                P::NotAttempted => "dispatch-preparation:not-attempted",
                P::ReplayAttempted { .. } => "dispatch-preparation:replay-attempted",
                P::PreviewAttempted { .. } => "dispatch-preparation:preview-attempted",
                P::AdmissionAttempted { .. } => "dispatch-preparation:admission-attempted",
            }
            .into(),
        );
        result.children.push(("primary", &e.cause));
        let queue_effect = match &e.queue {
            Q::NotAttempted => "dispatch-queue:not-attempted",
            Q::Committed(queue) => {
                if let Some(cause) = &queue.permission_refusal {
                    result
                        .labels
                        .push("dispatch-queue:permission-refused".into());
                    result.children.push(("source", cause));
                }
                for refused in &queue.refused {
                    result
                        .labels
                        .push("dispatch-queue:candidate-refused".into());
                    result.children.push(("source", &refused.error));
                }
                "dispatch-queue:committed"
            }
            Q::RecordedProjectionUnavailable => "dispatch-queue:recorded-projection-unavailable",
            Q::CommitRollbackConfirmed => "dispatch-queue:commit-rollback-confirmed",
            Q::OutcomeUnknown => "dispatch-queue:outcome-unknown",
        };
        result.labels.push(queue_effect.into());
        for resource in &e.resources {
            match resource {
                R::Fifo { .. } => result.labels.push("dispatch-resource:fifo".into()),
                R::Occupied { cause, .. } => {
                    result.labels.push("dispatch-resource:occupied".into());
                    result.children.push(("source", cause));
                }
                R::Capacity { cause, .. } => {
                    result.labels.push("dispatch-resource:capacity".into());
                    result.children.push(("source", cause));
                }
            }
        }
        explicit = true;
    }
    if let Some(e) = native_ref::<crate::MissionOrganizationError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result.labels.push(e.reason().into());
    }
    // Decoder families/reasons are a closed safe vocabulary. Never project the
    // supplied reply or arbitrary Display; generic source traversal stays active.
    #[cfg(target_os="linux")]
    if e.is::<crate::execution_interrupt::CompletedRoleInterrupt>(){result.labels.push("execution-interrupted".into());}
    if let Some(e) = native_ref::<crate::BuildIdentityError>(e, &mut result, &mut explicit) { result.labels.push(e.code().into()); }
    if let Some(e) = native_ref::<crate::ReviewReplyError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result.labels.push(e.reason().into());
    }
    if let Some(e) = native_ref::<crate::ReviewEvidenceError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result.labels.push(e.reason().into());
    }
    if let Some(e) = native_ref::<crate::TaskLifecycleError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result.labels.push(e.reason().into());
    }
    if let Some(e) = native_ref::<crate::BudgetError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result.labels.push(e.reason().into());
        if let crate::BudgetError::Cleanup { primary, cleanup } = e {
            result.children.push(("primary", primary.as_ref()));
            result.children.push(("cleanup", cleanup));
            explicit = true;
        }
    }
    if let Some(e) = native_ref::<crate::planning_boot_clock::PlanningClockFault>(e, &mut result, &mut explicit) {
        result.labels.push(e.0.into());
    }
    if let Some(e) = native_ref::<crate::connected_database_identity::ConnectedDatabaseIdentityError>(e, &mut result, &mut explicit) {
        result.labels.push(format!("database-identity-operation:{:?}", e.operation));
        if let Some(primary) = e.source() {
            result.children.push(("source", primary));
        }
        if let Some(cleanup) = &e.cleanup {
            result.children.push(("cleanup", cleanup.as_ref()));
        }
        explicit = true;
    }
    if let Some(e) = native_ref::<crate::TelemetryStoreError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        match e {
            crate::TelemetryStoreError::Cleanup { primary, cleanup } => {
                if let Some(primary) = primary {
                    result.children.push(("primary", primary.as_ref()));
                }
                result.children.push(("cleanup", cleanup.as_ref()));
                explicit = true;
            },
            crate::TelemetryStoreError::TokenIssued { primary } => {
                result.children.push(("primary", primary.as_ref()));
                explicit = true;
            },
            crate::TelemetryStoreError::Root(error) => {
                result.children.push(("source", error.as_ref()));
                explicit = true;
            },
            crate::TelemetryStoreError::Storage(error) => {
                result.children.push(("source", error));
                explicit = true;
            },
            _ => {},
        }
    }
    // Box::source may skip the Root wrapper that owns an auxiliary receipt.

    if let Some(e) = native_ref::<crate::RootStoreError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        if let crate::RootStoreError::Cleanup { primary, cleanup } = e {
            if let Some(primary) = primary {
                result.children.push(("primary", primary.as_ref()));
            }
            result.children.push(("cleanup", cleanup.as_ref()));
            explicit = true;
        }
    }
    if let Some(e) = native_ref::<crate::AgentLaunchError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        match e {
            crate::AgentLaunchError::Cleanup { primary, cleanup } => {
                result.children.push(("primary", primary.as_ref()));
                result.children.push(("cleanup", cleanup.as_ref()));
                explicit = true;
            }
            crate::AgentLaunchError::Commit { cause, cleanup, .. } => {
                result.children.push(("primary", cause.as_ref()));
                if let Some(cleanup) = cleanup {
                    result.children.push(("cleanup", cleanup.as_ref()));
                }
                explicit = true;
            }
            _ => {}
        }
    }
    if let Some(e) = native_ref::<crate::WorkEngineError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        match e {
            crate::WorkEngineError::Rollback { primary, cleanup } => {
                result.children.push(("primary", primary.as_ref()));
                result.children.push(("cleanup", cleanup));
                explicit = true;
            }
            crate::WorkEngineError::Commit { cause, cleanup, .. } => {
                result.children.push(("primary", cause));
                if let Some(cleanup) = cleanup {
                    result.children.push(("cleanup", cleanup.as_ref()));
                }
                explicit = true;
            }
            _ => {}
        }
    }
    if let Some(e) = native_ref::<crate::RecordedChangeError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        match e {
            crate::RecordedChangeError::Finalize { primary, cleanup } => {
                result.children.push(("primary", primary.as_ref()));
                result.children.push(("cleanup", cleanup));
                explicit = true;
            }
            crate::RecordedChangeError::Recovery { primary, cleanup, .. } => {
                result.children.push(("primary", primary.as_ref()));
                if let Some(cleanup) = cleanup {
                    result.children.push(("cleanup", cleanup));
                }
                explicit = true;
            }
            _ => {}
        }
    }
    if let Some(e) = native_ref::<crate::RecordedCommandError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        if let crate::RecordedCommandError::Finalize { primary, cleanup } = e {
            result.children.push(("primary", primary.as_ref()));
            result.children.push(("cleanup", cleanup));
            explicit = true;
        }
    }
    if let Some(e) = native_ref::<crate::RecordedWorkError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        if let crate::RecordedWorkError::Finalize { primary, cleanup } = e {
            result.children.push(("primary", primary.as_ref()));
            result.children.push(("cleanup", cleanup));
            explicit = true;
        }
    }
    if let Some(e) = native_ref::<crate::RecordedEventError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        if let crate::RecordedEventError::Finalize { primary, cleanup } = e {
            result.children.push(("primary", primary.as_ref()));
            result.children.push(("cleanup", cleanup));
            explicit = true;
        }
    }
    if let Some(e) = native_ref::<crate::WorkError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
    }
    if let Some(e) = native_ref::<crate::LocalStorageError>(e, &mut result, &mut explicit) {
        result
            .labels
            .push(format!("storage-operation:{:?}", e.operation));
        if let Some(primary) = e.source() {
            result.children.push(("source", primary));
        }
        if let Some(cleanup) = &e.cleanup {
            result.children.push(("cleanup", cleanup.as_ref()));
        }
        explicit = true;
    }
    if let Some(e) = native_ref::<crate::ValidationRunError>(e, &mut result, &mut explicit) {
        result
            .labels
            .push(format!("process-operation:{}", e.operation()));
        if let Some(primary) = e.source() {
            result.children.push(("source", primary));
        }
        for cleanup in e.cleanup() {
            result
                .labels
                .push(format!("process-cleanup:{}", cleanup.operation));
            result.children.push(("cleanup", &cleanup.cause));
        }
        explicit = true;
    }
    if let Some(e) = native_ref::<crate::MissionSupervisionError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        if let Some(primary) = e.source() {
            result.children.push(("source", primary));
        }
        if let Some(cleanup) = e.cleanup() {
            result.children.push(("cleanup", cleanup));
        }
        explicit = true;
    }
    if let Some(e) = native_ref::<crate::SupervisorIdentityError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result
            .labels
            .push(format!("identity-operation:{}", e.operation()));
        if let Some(primary) = e.source() {
            result.children.push(("source", primary));
        }
        if let Some(cleanup) = e.cleanup() {
            result.children.push(("cleanup", cleanup));
        }
        explicit = true;
    }
    if let Some(e) = native_ref::<crate::ProjectContextError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result.labels.push(e.reason().into());
        if let Some(primary) = e.source() {
            result.children.push(("source", primary));
        }
        if let Some(cleanup) = e.cleanup_cause() {
            result.children.push(("cleanup", cleanup));
        }
        explicit = true;
    }
    if let Some(error)=native_ref::<crate::GateEvaluationError>(e,&mut result,&mut explicit) {
        result.labels.push(error.code().into());
        result.labels.push(error.reason().into());
        if let Some(source)=error.source(){result.children.push(("source",source));}
        if let Some(cleanup)=error.direct_cleanup_cause(){result.children.push(("cleanup",cleanup));}
        explicit=true;
    }
    #[cfg(target_os="linux")]
    if let Some(error)=native_ref::<crate::anchored_artifact::AnchoredArtifactError>(e,&mut result,&mut explicit) {
        result.labels.push(error.code().into());
        if let Some(source)=error.primary(){result.children.push(("source",source));}
        if let Some(cleanup)=error.direct_cleanup(){result.children.push(("cleanup",cleanup));}
        explicit=true;
    }
    if let Some(e) = native_ref::<crate::AgentWorkflowError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result.labels.push(e.reason().into());
    }
    if let Some(e) = native_ref::<crate::ValidationEvidenceError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
        result.labels.push(e.reason().into());
    }
    if let Some(e) = native_ref::<crate::RuntimeError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
    }
    if let Some(e) = native_ref::<crate::AgentGuardError>(e, &mut result, &mut explicit) {
        result.labels.push(e.code().into());
    }
    if let Some(e) = native_ref::<crate::EngineSettingsError>(e, &mut result, &mut explicit) {
        result.labels.push(e.reason().into());
    }
    if let Some(e) = native_ref::<serde_json::Error>(e, &mut result, &mut explicit) {
        result
            .labels
            .push(format!("json-category:{:?}", e.classify()));
    }
    if let Some(e) = native_ref::<rusqlite::Error>(e, &mut result, &mut explicit) {
        result.sqlite_code = match e {
            rusqlite::Error::SqlInputError { error, .. } => Some(error.extended_code),
            _ => e.sqlite_error().map(|error| error.extended_code),
        };
        if let Some(code) = result.sqlite_code {
            result.labels.push(format!("sqlite-code:{code}"));
        }
    }
    if let Some(e) = native_ref::<rustix::io::Errno>(e, &mut result, &mut explicit) {
        result.io_errno = Some(e.raw_os_error());
        result.labels.push(format!("os-errno:{}", e.raw_os_error()));
    }
    if let Some(e) = native_ref::<nix::errno::Errno>(e, &mut result, &mut explicit) {
        result.io_errno = Some(*e as i32);
        result.labels.push(format!("os-errno:{}", *e as i32));
    }
    if let Some(e) = native_ref::<io::Error>(e, &mut result, &mut explicit) {
        result.labels.push(format!("io-kind:{:?}", e.kind()));
        result.io_errno = e.raw_os_error();
        if let Some(code) = result.io_errno {
            result.labels.push(format!("os-errno:{code}"));
        }
        if let Some(inner) = e.get_ref() {
            result.children.push(("source", inner));
            explicit = true;
        }
    }
    if !explicit {
        if let Some(source) = e.source() {
            result.children.push(("source", source));
        }
    }
    result
}

/// Ordered unique typed-error observations and their first discovery paths.
/// Paths describe traversal, not an authenticated causal graph. Only safe known
/// labels and numeric kernel/SQLite causes are serialized; private text is absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeGateCause {
    pub path: Vec<String>,
    pub labels: Vec<String>,
    pub io_errno: Option<i32>,
    pub sqlite_code: Option<i32>,
}
impl NativeGateCause {
    /// Validate a received diagnostic's vocabulary only; no authority is inferred.
    /// Listed labels contain static classification data, never free-form messages.
    pub(crate) fn valid(&self) -> bool {
        !self.path.is_empty()
            && self.path[0] == "root"
            && self
                .path
                .iter()
                .skip(1)
                .all(|s| matches!(s.as_str(), "source" | "primary" | "cleanup"))
            && self.labels.iter().all(|s| {
                !s.is_empty()
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'-' | b'_'))
            })
    }
}
/// Walk every listed compound branch without a hidden depth/count cap. Typed
/// pointer metadata distinguishes transparent wrappers sharing a data address;
/// repeated trait objects/cycles terminate without rendering private errors.
pub(crate) fn project(e: &(dyn Error + 'static)) -> Vec<NativeGateCause> {
    let mut pending = vec![(vec!["root".into()], e)];
    let mut seen: Vec<*const dyn Error> = vec![];
    let mut result = vec![];
    while let Some((path, error)) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|prior| std::ptr::eq(*prior, pointer)) {
            continue;
        }
        seen.push(pointer);
        let links = native_error_links(error);
        result.push(NativeGateCause {
            path: path.clone(),
            labels: links.labels,
            io_errno: links.io_errno,
            sqlite_code: links.sqlite_code,
        });
        for (kind, child) in links.children.into_iter().rev() {
            let mut path = path.clone();
            path.push(kind.into());
            pending.push((path, child));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Retain snapshot and per-entry failures through custom IO and secondary
    /// cleanup wrappers. All progress states remain observations, not grants.
    /// Private phase/identity/message data must never enter the gate projection.
    #[cfg(target_os = "linux")]
    #[test]
    fn native_diagnostics_keep_continuation_snapshot_and_entry_causes() {
        use crate::{MissionContinuationFailure as F, MissionContinuationProgress as P,
            MissionWorkerContinuationEntry as Entry, MissionWorkerContinuationState as S,
            MissionSupervisionError as E};
        for progress in [P::NotStarted, P::PartialNotUsable, P::CompleteButUnusable] {
          for wrapper in 0..4 {
            let failure = F {
                phase: "PRIVATE_PHASE/work",
                progress,
                entries: vec![Entry {
                    agent_id: "PRIVATE_AGENT".into(), task_id: "PRIVATE_TASK".into(),
                    state: S::Refused { phase: "PRIVATE_ENTRY", cause: crate::RootStoreError::Cleanup {
                        primary: Some(Box::new(crate::RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("continuation-test-entry", io::Error::from_raw_os_error(13)))))),
                        cleanup: Box::new(crate::RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("continuation-test-entry", io::Error::from_raw_os_error(28))))),
                    } },
                }],
                cause: E::Cleanup {
                    primary: Box::new(E::Cause { code: "continuation-test-primary",
                        cause: Box::new(io::Error::other(io::Error::from_raw_os_error(5))) }),
                    cleanup: Box::new(E::Refused("continuation-test-cleanup")),
                },
            };
            assert!(!format!("{failure:?}").contains("PRIVATE"));
            let wrapped: Box<dyn Error + Send + Sync> = match wrapper {
                0 => Box::new(failure),
                1 => Box::new(Box::new(failure)),
                2 => Box::new(io::Error::other(failure)),
                _ => Box::new(io::Error::other(Box::new(failure))),
            };
            let projected = project(wrapped.as_ref());
            assert!(projected.iter().all(NativeGateCause::valid));
            for errno in [5, 13, 28] {
                assert!(projected.iter().any(|c| c.io_errno == Some(errno)), "wrapper {wrapper} must retain errno {errno}");
            }
            assert!(projected.iter().any(|c| c.labels.iter().any(|v| v == "continuation-test-cleanup")));
            assert!(!serde_json::to_string(&projected).unwrap().contains("PRIVATE"));
          }
        }
    }

    /// A post-cleanup binding refusal invalidates the report without proving
    /// cleanup failed. The phase and cause must survive without that false claim.
    #[cfg(target_os = "linux")]
    #[test]
    fn continuation_final_binding_refusal_does_not_invent_cleanup_failure() {
        let failure = crate::MissionContinuationFailure {
            phase: "final-binding",
            progress: crate::MissionContinuationProgress::CompleteButUnusable,
            entries: vec![],
            cause: crate::MissionSupervisionError::Refused("mission-lease-root-mismatch"),
        };
        let projected = project(&failure);
        assert!(projected.iter().any(|c| c.labels.iter().any(|v| v == "mission-lease-root-mismatch")));
        assert!(projected.iter().any(|c| c.labels.iter().any(|v| v == "continuation-phase:final-binding")));
        assert!(projected.iter().all(|c| !c.path.iter().any(|v| v == "cleanup")));
        assert!(!projected.iter().any(|c| c.labels.iter().any(|v| v == "continuation-progress:complete-cleanup-failed")),
            "binding invalidation does not establish a failed cleanup");
    }

    /// A real parser/prepare error must keep its actual extended code through
    /// direct and compound wrappers. No SQL/message/private Display is projected.
    /// This data-only fixture does not replace a builtin or establish authority.
    /// Unexpected SQL success is reduced to unit before refusal assertion, keeping
    /// Statement Debug (which includes SQL) out of the assertion diagnostic.
    #[test]
    fn native_diagnostics_keep_sql_input_error_code_through_wrapper_matrix() {
        for wrapper in 0..4 {
            let db = rusqlite::Connection::open_in_memory().unwrap();
            let error = db.prepare("SELECT PRIVATE_unknown_function(1)")
                .map(|_| ())
                .expect_err("invalid SQL must refuse");
            let expected = match &error {
                rusqlite::Error::SqlInputError { error, .. } => error.extended_code,
                _ => panic!("real SqlInputError required for this contract"),
            };
            db.close().unwrap();
            let wrapped: Box<dyn Error + Send + Sync> = match wrapper {
                0 => Box::new(error),
                1 => Box::new(io::Error::other(Transparent(io::Error::other(error)))),
                2 => Box::new(crate::AgentLaunchError::caused("sql-input-fixture", error)),
                _ => Box::new(crate::RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("sql-input-fixture", error)))),
            };
            let nodes = project(wrapped.as_ref());
            assert!(nodes.iter().any(|node| node.sqlite_code == Some(expected)));
            assert!(nodes.iter().any(|node| node.labels.iter().any(|label| label == &format!("sqlite-code:{expected}"))));
            assert!(nodes.iter().all(NativeGateCause::valid));
            assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
        }
    }

    /// Capture the real builtin on a memory database, whose main descriptor is
    /// deliberately absent, then retain a second real diagnostic as cleanup.
    /// This tests typed phase/secondary traversal; it is not a disk Root, missing
    /// production builtin, process or grant proof and injects no success token.
    #[test]
    fn native_diagnostics_keep_database_identity_phase_and_secondary_causes() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let mut error = crate::connected_database_identity::ConnectedDatabaseIdentity::capture(&db).unwrap_err();
        let cleanup = crate::connected_database_identity::ConnectedDatabaseIdentity::capture(&db).unwrap_err();
        assert_eq!(error.operation, crate::connected_database_identity::ConnectedDatabaseIdentityOperation::Diagnostic);
        error.cleanup = Some(Box::new(cleanup));
        db.close().unwrap();
        let links = native_error_links(&error);
        assert_eq!(links.children.len(), 2);
        assert_eq!(links.children[0].0, "source");
        assert_eq!(links.children[1].0, "cleanup");
        let wrapped = io::Error::other(crate::RootStoreError::DatabaseIdentity(Box::new(error)));
        let nodes = project(&wrapped);
        assert_eq!(nodes.iter().filter(|node| node.labels.iter().any(|label| label == "database-identity-operation:Diagnostic")).count(), 2);
        assert!(nodes.iter().any(|node| node.path.iter().any(|part| part == "cleanup") && node.labels.iter().any(|label| label == "database-identity-operation:Diagnostic")));
        assert!(nodes.iter().all(NativeGateCause::valid));
        let encoded = serde_json::to_string(&nodes).unwrap();
        assert!(!encoded.contains("connected main-file diagnostic required"));
        assert!(!encoded.contains("PRIVATE"));
    }

    /// Telemetry's known token effect and every independent cleanup branch must
    /// survive direct, Root/launch and IO/custom wrapper routes. SQL and storage
    /// errors are real fixture failures; the typed token marker is projection
    /// input only, not evidence of a runtime token issuance or permission.
    #[test]
    fn native_diagnostics_telemetry_token_and_cleanup_survive_wrapper_matrix() {
        use crate::TelemetryStoreError as T;
        use std::os::unix::fs::PermissionsExt;
        let primary_errno = rustix::io::Errno::ACCESS.raw_os_error();
        let cleanup_errno = rustix::io::Errno::PIPE.raw_os_error();
        let missing_errno = rustix::io::Errno::NOENT.raw_os_error();
        for wrapper in 0..4 {
            let path = std::env::temp_dir().join(crate::generate_identifier("telemetry-diagnostic-").unwrap());
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            crate::local_storage::LocalDirectory::open(&path).unwrap().close().unwrap();
            let storage = crate::local_storage::LocalDirectory::open(&path.join("PRIVATE_missing_root")).err().unwrap();
            std::fs::remove_dir(&path).unwrap();
            let db = rusqlite::Connection::open_in_memory().unwrap();
            let sql = db.execute_batch("SELECT * FROM PRIVATE_missing_cleanup").unwrap_err();
            let sql_code = sql.sqlite_error().unwrap().extended_code;
            db.close().unwrap();
            let root = crate::RootStoreError::Cleanup {
                primary: Some(Box::new(crate::RootStoreError::Launch(Box::new(
                    crate::AgentLaunchError::caused("telemetry-fixture-primary", Transparent(io::Error::from_raw_os_error(primary_errno))),
                )))),
                cleanup: Box::new(crate::RootStoreError::Sql(sql)),
            };
            let error = T::Cleanup {
                primary: Some(Box::new(T::TokenIssued { primary: Box::new(T::Root(Box::new(root))) })),
                cleanup: Box::new(T::Cleanup {
                    primary: Some(Box::new(T::Storage(storage))),
                    cleanup: Box::new(T::Io(io::Error::other(Transparent(io::Error::from_raw_os_error(cleanup_errno))))),
                }),
            };
            assert_eq!(native_error_links(&error).children.len(), 2);
            let wrapped: Box<dyn Error + Send + Sync> = match wrapper {
                0 => Box::new(error),
                1 => Box::new(io::Error::other(error)),
                2 => Box::new(crate::AgentLaunchError::caused("telemetry-fixture-wrapper", error)),
                _ => Box::new(crate::RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused(
                    "telemetry-fixture-wrapper", io::Error::other(Transparent(io::Error::other(error))),
                )))),
            };
            let nodes = project(wrapped.as_ref());
            for label in ["telemetry-token-issued", "telemetry-store-unavailable", "telemetry-fixture-primary", "storage-operation:OpenDirectory"] {
                assert!(nodes.iter().any(|node| node.labels.iter().any(|item| item == label)), "missing telemetry label {label}");
            }
            for errno in [primary_errno, cleanup_errno, missing_errno] {
                assert!(nodes.iter().any(|node| node.io_errno == Some(errno)), "lost telemetry errno {errno}");
            }
            assert!(nodes.iter().any(|node| node.sqlite_code == Some(sql_code) && node.path.iter().any(|part| part == "cleanup")));
            assert!(nodes.iter().all(NativeGateCause::valid));
            assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
        }
    }

    /// Cleanup-only results have no invented primary; a token marker confined
    /// to a secondary branch is still observed. These constructors prove only
    /// error-tree traversal, not a committed token or completed resource close.
    #[test]
    fn native_diagnostics_telemetry_cleanup_only_and_secondary_token_are_explicit() {
        use crate::TelemetryStoreError as T;
        let errno = rustix::io::Errno::IO.raw_os_error();
        let cleanup = T::Cleanup { primary: None, cleanup: Box::new(T::Io(io::Error::from_raw_os_error(errno))) };
        let links = native_error_links(&cleanup);
        assert_eq!(links.children.len(), 1);
        assert_eq!(links.children[0].0, "cleanup");
        assert!(project(&cleanup).iter().any(|node| node.io_errno == Some(errno)));
        let error = T::Cleanup {
            primary: Some(Box::new(T::Refused("telemetry-fixture-refused"))),
            cleanup: Box::new(T::TokenIssued { primary: Box::new(T::Io(io::Error::from_raw_os_error(errno))) }),
        };
        let nodes = project(&error);
        assert!(nodes.iter().any(|node| node.labels.iter().any(|label| label == "telemetry-fixture-refused")));
        assert!(nodes.iter().any(|node| node.labels.iter().any(|label| label == "telemetry-token-issued") && node.path.iter().any(|part| part == "cleanup")));
        assert!(nodes.iter().all(NativeGateCause::valid));
    }

    /// Dispatch's committed queue and earlier resource failures are independent
    /// branches, beyond Error::source(). Actual SQL errors and typed IO/custom
    /// fixture wrappers exercise projection only, not runtime commit/rollback.
    #[cfg(target_os = "linux")]
    #[test]
    fn native_diagnostics_dispatch_retains_primary_queue_resources_and_cleanup() {
        use crate::{
            MissionDispatchCause as C, MissionDispatchPreparationEffect as P,
            MissionDispatchQueueEffect as Q, MissionDispatchResourceRefusal as R,
            MissionSupervisionError as E,
        };
        let db = rusqlite::Connection::open_in_memory().unwrap();
        assert_eq!(
            db.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let sql = db
            .execute_batch("SELECT * FROM PRIVATE_DISPATCH_SQL")
            .unwrap_err();
        let primary_errno = rustix::io::Errno::ACCESS.raw_os_error();
        let ready_errno = rustix::io::Errno::NOSPC.raw_os_error();
        let resource_errno = rustix::io::Errno::NOENT.raw_os_error();
        let failure = crate::MissionWorkerDispatchFailure {
            phase: "PRIVATE phase/work/id",
            queue: Q::Committed(crate::MissionQueueRegistration {
                work: "PRIVATE_WORK".into(),
                revision: 1,
                ready: vec!["PRIVATE_READY_TASK".into()],
                permission_refusal: Some(E::Refused("mission-consent-required")),
                refused: vec![crate::MissionCandidateRefusal {
                    task: "PRIVATE_REFUSED_TASK".into(),
                    error: E::Cleanup {
                        primary: Box::new(E::Cause {
                            code: "fixture-dispatch-ready",
                            cause: Box::new(io::Error::other(Transparent(
                                io::Error::from_raw_os_error(ready_errno),
                            ))),
                        }),
                        cleanup: Box::new(E::Refused("fixture-ready-cleanup")),
                    },
                }],
            }),
            preparation: P::AdmissionAttempted {
                event_id: "PRIVATE_EVENT".into(),
                task_id: "PRIVATE_TASK".into(),
            },
            resources: vec![
                R::Fifo {
                    task: "PRIVATE_FIFO_TASK".into(),
                    blocker: crate::MissionWorkspaceBlocker {
                        task: "PRIVATE_BLOCKED_TASK".into(),
                        candidate_seq: "PRIVATE_SEQUENCE".into(),
                        older_seq: "PRIVATE_SEQUENCE".into(),
                        older_work: "PRIVATE_OLDER_WORK".into(),
                        older_task: "PRIVATE_OLDER_TASK".into(),
                    },
                },
                R::Occupied {
                    task: "PRIVATE_OCCUPIED_TASK".into(),
                    cause: crate::RootStoreError::Launch(Box::new(
                        crate::AgentLaunchError::caused(
                            "fixture-occupied",
                            io::Error::other(Transparent(io::Error::from_raw_os_error(
                                resource_errno,
                            ))),
                        ),
                    )),
                },
                R::Capacity {
                    task: "PRIVATE_CAPACITY_TASK".into(),
                    cause: crate::RootStoreError::Launch(Box::new(
                        crate::AgentLaunchError::Refused("mission-agent-slots-occupied"),
                    )),
                },
            ],
            cause: C::Root(Box::new(crate::RootStoreError::Launch(Box::new(
                crate::AgentLaunchError::Cleanup {
                    primary: Box::new(crate::AgentLaunchError::caused(
                        "fixture-dispatch-primary",
                        Transparent(io::Error::from_raw_os_error(primary_errno)),
                    )),
                    cleanup: Box::new(crate::AgentLaunchError::caused(
                        "fixture-dispatch-cleanup",
                        sql,
                    )),
                },
            )))),
        };
        assert_eq!(native_error_links(&failure).children.len(), 5);
        let nodes = project(&io::Error::other(failure));
        for label in [
            "mission-dispatch-failed",
            "dispatch-phase:unknown",
            "dispatch-preparation:admission-attempted",
            "dispatch-queue:committed",
            "dispatch-queue:permission-refused",
            "dispatch-queue:candidate-refused",
            "dispatch-resource:fifo",
            "dispatch-resource:occupied",
            "dispatch-resource:capacity",
            "mission-consent-required",
            "fixture-ready-cleanup",
            "mission-agent-slots-occupied",
        ] {
            assert!(
                nodes
                    .iter()
                    .any(|node| node.labels.iter().any(|item| item == label)),
                "missing static dispatch label: {label}"
            );
        }
        for errno in [primary_errno, ready_errno, resource_errno] {
            assert!(nodes.iter().any(|node| node.io_errno == Some(errno)));
        }
        assert!(nodes.iter().any(
            |node| node.sqlite_code.is_some() && node.path.iter().any(|part| part == "cleanup")
        ));
        assert!(nodes.iter().all(NativeGateCause::valid));
        assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
    }

    /// All queue/preparation states use static effect vocabulary; no identity or
    /// permission is inferred. This is a synthetic variant matrix, not proof of
    /// a recorded operation, rollback or uncertain commit on a real datastore.
    #[cfg(target_os = "linux")]
    #[test]
    fn native_diagnostics_dispatch_effect_matrix_uses_static_safe_labels() {
        use crate::{
            MissionDispatchCause as C, MissionDispatchPreparationEffect as P,
            MissionDispatchQueueEffect as Q, MissionSupervisionError as E,
        };
        let cases = [
            (
                Q::NotAttempted,
                P::NotAttempted,
                "dispatch-queue:not-attempted",
                "dispatch-preparation:not-attempted",
            ),
            (
                Q::RecordedProjectionUnavailable,
                P::ReplayAttempted {
                    event_id: "PRIVATE_EVENT".into(),
                },
                "dispatch-queue:recorded-projection-unavailable",
                "dispatch-preparation:replay-attempted",
            ),
            (
                Q::CommitRollbackConfirmed,
                P::PreviewAttempted {
                    event_id: "PRIVATE_EVENT".into(),
                    task_id: "PRIVATE_TASK".into(),
                },
                "dispatch-queue:commit-rollback-confirmed",
                "dispatch-preparation:preview-attempted",
            ),
            (
                Q::OutcomeUnknown,
                P::AdmissionAttempted {
                    event_id: "PRIVATE_EVENT".into(),
                    task_id: "PRIVATE_TASK".into(),
                },
                "dispatch-queue:outcome-unknown",
                "dispatch-preparation:admission-attempted",
            ),
        ];
        for (queue, preparation, queue_label, preparation_label) in cases {
            let failure = crate::MissionWorkerDispatchFailure {
                phase: "admission",
                queue,
                preparation,
                resources: vec![],
                cause: C::Mission(Box::new(E::Refused("fixture-dispatch-refusal"))),
            };
            let nodes = project(&failure);
            for label in [
                queue_label,
                preparation_label,
                "dispatch-phase:admission",
                "fixture-dispatch-refusal",
            ] {
                assert!(nodes
                    .iter()
                    .any(|node| node.labels.iter().any(|item| item == label)));
            }
            assert_eq!(native_error_links(&failure).children.len(), 1);
            assert!(nodes.iter().all(NativeGateCause::valid));
            assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
        }
    }
    /// A missing native review and a lifecycle refusal must remain actionable
    /// through generic wrappers; no Display/private payload becomes a label.
    #[test]
    fn native_diagnostics_keep_review_and_lifecycle_refusals() {
        let causes: Vec<(Box<dyn std::error::Error + Send + Sync>, &str)> = vec![
            (Box::new(crate::ReviewEvidenceError::Refused("native-change-request-required")), "native-change-request-required"),
            (Box::new(crate::TaskLifecycleError::Refused("scoped-worker-review-recovery-required")), "scoped-worker-review-recovery-required"),
        ];
        for (cause, expected) in causes {
            let wrapped = crate::AgentLaunchError::Cause { code: "planning-retry-current-evidence", cause };
            let nodes = project(&wrapped);
            assert!(nodes.iter().any(|node| node.labels.iter().any(|label| label == expected)), "typed refusal must not disappear behind a generic family");
            assert!(nodes.iter().all(NativeGateCause::valid));
        }
    }
    /// Actual event collision classification survives nested safe wrappers; no
    /// Display text, Root mutation, execution or permission is inferred here.
    #[test]
    fn native_diagnostics_recorded_event_conflict_survives_launch_wrapper() {
        let cause = crate::RecordedChangeError::Command(crate::RecordedCommandError::Conflict);
        let error = crate::AgentLaunchError::caused("review-claim-publication", cause);
        let nodes = project(&error);
        assert!(nodes.iter().any(|n| n.labels.iter().any(|s| s == "event_conflict")));
        assert!(nodes.iter().all(NativeGateCause::valid));
    }

    /// Every known recorded compound retains both independent SQL errors and
    /// traversal paths. SQL includes private fixture names; only numeric codes
    /// may appear, even when primary/cleanup have identical measured values.
    #[test]
    fn native_diagnostics_recorded_cleanup_branches_keep_independent_sql_causes() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        assert_eq!(db.query_row("SELECT 1", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
        let sql = || db.execute_batch("SELECT * FROM PRIVATE_RECORDED_SQL_MUST_NOT_LEAK").unwrap_err();
        let cases: Vec<Box<dyn Error>> = vec![
            Box::new(crate::RecordedChangeError::Finalize {
                primary: Box::new(crate::RecordedChangeError::Sql(sql())), cleanup: sql(),
            }),
            Box::new(crate::RecordedCommandError::Finalize {
                primary: Box::new(crate::RecordedCommandError::Sql(sql())), cleanup: sql(),
            }),
            Box::new(crate::RecordedWorkError::Finalize {
                primary: Box::new(crate::RecordedWorkError::Sql(sql())), cleanup: sql(),
            }),
            Box::new(crate::RecordedEventError::Finalize {
                primary: Box::new(crate::RecordedEventError::Sql(sql())), cleanup: sql(),
            }),
            Box::new(crate::RecordedChangeError::Recovery {
                primary: Box::new(crate::RecordedChangeError::Sql(sql())),
                cleanup: Some(sql()), transaction_lost: true,
            }),
        ];
        for error in cases {
            let nodes = project(error.as_ref());
            let sql_nodes: Vec<_> = nodes.iter().filter(|n| n.sqlite_code.is_some()).collect();
            assert_eq!(sql_nodes.len(), 2);
            assert!(sql_nodes.iter().any(|n| n.path.iter().any(|p| p == "primary")));
            assert!(sql_nodes.iter().any(|n| n.path.iter().any(|p| p == "cleanup")));
            assert!(nodes.iter().all(NativeGateCause::valid));
            assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE_RECORDED_SQL"));
        }
    }

    /// Missing cleanup is not a fabricated second error. Typed row/event errors
    /// keep their own stable codes without serializing private JSON or hook text.
    #[test]
    fn native_diagnostics_recorded_missing_cleanup_and_private_payloads() {
        let error = crate::RecordedChangeError::Recovery {
            primary: Box::new(crate::RecordedChangeError::Command(crate::RecordedCommandError::Conflict)),
            cleanup: None, transaction_lost: false,
        };
        let nodes = project(&error);
        assert!(nodes.iter().any(|n| n.labels.iter().any(|s| s == "event_conflict")));
        assert!(!nodes.iter().any(|n| n.path.iter().any(|p| p == "cleanup")));
        let invalid = serde_json::from_str::<serde_json::Value>("PRIVATE_PAYLOAD_MUST_NOT_LEAK").unwrap_err();
        let errors: Vec<Box<dyn Error>> = vec![
            Box::new(crate::RecordedEventError::Payload(invalid)),
            Box::new(crate::RecordedWorkError::Missing),
            Box::new(crate::RecordedChangeError::Work(crate::RecordedWorkError::Work(crate::WorkError::Incoherent))),
            Box::new(crate::RecordedChangeError::Hook(Box::new(io::Error::other("PRIVATE_HOOK_MUST_NOT_LEAK")))),
        ];
        for (error, expected) in errors.iter().zip(["event-payload", "work-missing", "row-incoherent", "recorded-change-hook"]) {
            let nodes = project(error.as_ref());
            assert!(nodes.iter().any(|n| n.labels.iter().any(|s| s == expected)));
            assert!(nodes.iter().all(NativeGateCause::valid));
            assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
        }
    }

    /// A real capped read refusal retains its safe typed reason across launch
    /// wrappers, without publishing content/path. A valid same-file witness
    /// precedes the cap fault; owned fixture data is removed only after success.
    #[test]
    fn native_diagnostics_keep_context_read_reason_through_launch_wrapper() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(crate::generate_identifier("native-context-cause-").unwrap());
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(root.join("private.md"), "ok").unwrap();
        let mut limits = crate::ContextPolicyLimits {
            profile_name_bytes: None, profile_sources: None, profile_path_bytes: None,
            profile_document_bytes: None, context_bytes: None, worker_context_bytes: None,
            review_delivery_bytes: Some(2),
        };
        assert_eq!(crate::read_review_delivery(&root, "private.md", &limits).unwrap(), "ok");
        limits.review_delivery_bytes = Some(1);
        let original = crate::read_review_delivery(&root, "private.md", &limits).err().unwrap();
        let wrapped = crate::AgentLaunchError::caused("task-validation-report", original);
        let facts = project(&wrapped);
        assert!(facts.iter().any(|v| v.labels.iter().any(|s| s == "project-context-invalid")));
        assert!(facts.iter().any(|v| v.labels.iter().any(|s| s == "source-exceeds-configured-cap")));
        let encoded = serde_json::to_string(&facts).unwrap();
        assert!(!encoded.contains("private.md") && !encoded.contains(root.to_str().unwrap()));
        assert!(facts.iter().all(NativeGateCause::valid));
        for (error, code, reason) in [
            (Box::new(crate::AgentWorkflowError::Refused("fixture-workflow-reason")) as Box<dyn Error + Send + Sync>, "agent-workflow-invalid", "fixture-workflow-reason"),
            (Box::new(crate::ValidationEvidenceError::Refused("fixture-controls-reason")) as Box<dyn Error + Send + Sync>, "validation-evidence-invalid", "fixture-controls-reason"),
        ] {
            let wrapped = crate::AgentLaunchError::Cause { code: "fixture-wrapper", cause: error };
            let facts = project(&wrapped);
            assert!(facts.iter().any(|v| v.labels.iter().any(|s| s == code)));
            assert!(facts.iter().any(|v| v.labels.iter().any(|s| s == reason)));
            assert!(facts.iter().all(NativeGateCause::valid));
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    /// Organization code/reason must survive wrappers; private recorded failure
    /// stays out of the safe graph. These observations grant no reviewer rights.
    #[test]
    fn native_diagnostics_expose_organization_reason_without_private_failure() {
        let organization =
            crate::MissionOrganizationError::Structure("organization-requirement-controls-missing");
        let wrapped = crate::AgentLaunchError::caused(organization.code(), organization);
        let facts = project(&wrapped);
        assert!(facts
            .iter()
            .any(|v| v.labels.iter().any(|s| s == "organization_incomplete")));
        assert!(
            facts.iter().any(|v| v
                .labels
                .iter()
                .any(|s| s == "organization-requirement-controls-missing")),
            "organization-requirement-controls-missing not preserved"
        );
        for (calls, limit, failure, reason) in [
            (
                1,
                2,
                "PRIVATE_REVIEWER_FAILURE_MUST_NOT_LEAK",
                "reviewer-recorded-failure",
            ),
            (-1, 2, "", "reviewer-counter-invalid"),
            (2, 2, "", "reviewer-budget-exhausted"),
        ] {
            let error = crate::MissionOrganizationError::ReviewerUnavailable {
                calls,
                limit,
                failure: failure.into(),
            };
            let facts = project(&error);
            assert!(facts.iter().all(|cause| cause.valid()));
            assert!(facts.iter().any(|v| v.labels.iter().any(|s| s == reason)));
            assert!(!serde_json::to_string(&facts)
                .unwrap()
                .contains("PRIVATE_REVIEWER_FAILURE_MUST_NOT_LEAK"));
        }
    }

    /// Deliberately transparent fixture: its typed wrapper shares the IO address.
    /// Display text must never be consulted by either classifier or projector.
    #[repr(transparent)]
    #[derive(Debug)]
    struct Transparent(io::Error);
    impl std::fmt::Display for Transparent {
        /// Private fixture text exists only to detect accidental Display serialization.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("PRIVATE identity and path")
        }
    }
    impl Error for Transparent {
        /// Return the original typed IO field at the same address as this wrapper.
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }
    /// Assert numeric IO remains visible through distinct typed aliases, and that
    /// safe JSON contains no private Display message. No process/authority involved.
    #[test]
    fn native_diagnostics_preserve_transparent_typed_source_and_private_text() {
        let errno = rustix::io::Errno::ACCESS.raw_os_error();
        let wrapped = Transparent(io::Error::from_raw_os_error(errno));
        let nodes = project(&wrapped);
        assert!(nodes.iter().any(|n| n.io_errno == Some(errno)));
        assert!(nodes.len() >= 2);
        assert!(nodes.iter().all(NativeGateCause::valid));
        assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
    }
    /// Distinct SQL failures with identical codes are independent error objects;
    /// a Work rollback must retain both rather than deduplicate measured values.
    #[test]
    fn native_diagnostics_keep_both_work_rollback_sql_errors() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let first = db
            .execute_batch("SELECT * FROM missing_primary")
            .unwrap_err();
        let second = db
            .execute_batch("SELECT * FROM missing_cleanup")
            .unwrap_err();
        let expected = first.sqlite_error().unwrap().extended_code;
        let error = crate::WorkEngineError::Rollback {
            primary: Box::new(crate::WorkEngineError::Sql(first)),
            cleanup: second,
        };
        let nodes = project(&error);
        assert_eq!(
            nodes
                .iter()
                .filter(|n| n.sqlite_code == Some(expected))
                .count(),
            2
        );
        assert!(nodes.iter().any(|n| n.path.iter().any(|p| p == "cleanup")));
    }
    /// Budget statement finalization is an independent typed SQL branch, not
    /// private Display text. Projection retains both codes and the cleanup path.
    #[test]
    fn native_diagnostics_keep_budget_statement_primary_and_cleanup() {
        let first = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(5),
            Some("PRIVATE primary".into()),
        );
        let second = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(14),
            Some("PRIVATE cleanup".into()),
        );
        let error = crate::BudgetError::Cleanup {
            primary: Box::new(crate::BudgetError::Sql(first)),
            cleanup: second,
        };
        let nodes = project(&error);
        assert!(nodes.iter().any(|n| n.sqlite_code == Some(5)));
        assert!(nodes
            .iter()
            .any(|n| n.sqlite_code == Some(14) && n.path.iter().any(|p| p == "cleanup")));
        assert!(nodes.iter().all(NativeGateCause::valid));
        assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
    }
    #[derive(Debug)]
    struct Cycle;
    impl std::fmt::Display for Cycle {
        /// A private cyclic message must not be rendered during safe traversal.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("PRIVATE cyclic message")
        }
    }
    impl Error for Cycle {
        /// Deliberate fixture cycle, not a production source or claimed authority.
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(self)
        }
    }
    /// Repeated typed objects terminate traversal without a depth/attempt cutoff.
    /// Malformed paths/free-form labels refuse independently of observation data.
    #[test]
    fn native_diagnostics_cycles_terminate_and_private_labels_refuse() {
        let nodes = project(&Cycle);
        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].valid());
        let mut bad = nodes[0].clone();
        bad.labels.push("PRIVATE path".into());
        assert!(!bad.valid());
        let mut bad = nodes[0].clone();
        bad.path.push("actor".into());
        assert!(!bad.valid());
    }
}

#[cfg(test)]
mod migration_review_error_tests {
    use super::*;
    /// Real reply decoder: valid literal original first, then an absent private
    /// quote. Direct/launch/custom-IO/cleanup wrappers retain static reason only.
    #[test]
    fn migration_review_error_static_reason_survives_wrapper_matrix(){
        let values=crate::EngineSettingsValues::installation_defaults().unwrap();let limits=values.review_reply_limits().unwrap();
        let originals=[crate::ReviewQuoteOriginal{name:"docs/task.md",kind:"source",content:"Owned original worker report"}];
        let good=serde_json::json!({"reason":"Owned fixture review","criteria":[{"index":1,"verdict":"pass","evidence":"Owned original worker report"}]});
        assert!(crate::parse_review_reply(&serde_json::to_vec(&good).unwrap(),1,&originals,limits).is_ok());
        for wrapper in 0..4 {
            let bad=serde_json::json!({"reason":"Owned fixture review","criteria":[{"index":1,"verdict":"pass","evidence":"PRIVATE absent evidence, never echo"}]});
            let cause=crate::parse_review_reply(&serde_json::to_vec(&bad).unwrap(),1,&originals,limits).expect_err("absent quote must refuse");
            assert_eq!(cause.reason(),"review-reply-quote-absent");
            let error:Box<dyn Error+Send+Sync>=match wrapper {
                0=>Box::new(cause),
                1=>Box::new(crate::AgentLaunchError::caused("review_fragment_protocol",cause)),
                2=>Box::new(io::Error::other(crate::AgentLaunchError::caused("review_fragment_protocol",cause))),
                _=>Box::new(crate::RootStoreError::Cleanup{primary:Some(Box::new(crate::RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("review_fragment_protocol",cause))))),cleanup:Box::new(crate::RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("fixture-cleanup",io::Error::from_raw_os_error(28)))))}),
            };
            let nodes=project(error.as_ref());assert!(nodes.iter().any(|n|n.labels.iter().any(|s|s=="review-reply-quote-absent")),"static reply reason disappeared");
            assert!(nodes.iter().all(NativeGateCause::valid));assert!(!serde_json::to_string(&nodes).unwrap().contains("PRIVATE"));
            if wrapper==3{assert!(nodes.iter().any(|n|n.io_errno==Some(28) && n.path.iter().any(|p|p=="cleanup")));}
        }
    }
}
