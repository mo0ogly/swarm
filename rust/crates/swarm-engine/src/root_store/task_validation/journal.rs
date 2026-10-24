//! Immutable control observations before SQL publication. Private kernel/owner
//! facts never enter the public validation artifact. Files are local trust data,
//! not cryptographic identity or proof of malicious same-UID isolation.
use super::{
    process::Observation,
    store::{Job, Step},
};
use crate::{
    root_store::{RootStore, RootStoreError as E},
    ValidationControlResult,
};
use serde::{Deserialize, Serialize};
use std::os::unix::process::ExitStatusExt;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StepReceipt {
    pub schema_version: i64,
    pub job: Job,
    pub step: Step,
    /// True once the owned-process primitive was invoked, even if spawn failed.
    /// False is emitted only by the controller's pre-invocation path.
    pub process_attempted: bool,
    pub grant_attempted: bool,
    pub channel_closed: bool,
    pub waited: bool,
    pub failed: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub settings_revision: Option<crate::SettingCount>,
    pub result: Option<ValidationControlResult>,
    pub retained: Option<Vec<u8>>,
    pub total: Option<u64>,
    pub truncated: Option<bool>,
    pub causes: Vec<crate::native_diagnostics::NativeGateCause>,
    pub gate: Option<crate::agent_execution_gate::GateFailure>,
}
/// Freeze actual observations, retaining the original owned error separately.
/// An absent wait/stream is unknown, never synthesized exit0/empty output. The
/// private gate can disprove exec, while empty EOF alone cannot prove it. A
/// returned primary error can still own an unreaped Child and must not be lost.
/// Nonserialized live observations. Transport eligibility proves no persistence,
/// permission or completion; runtime must still publish every original/result.
pub(super) struct JournalObservation{
    pub receipt:StepReceipt,
    pub primary:Option<E>,
    pub local_stop_transport_complete:bool,
}
/// Freeze actual observations and preserve gate/transport/stop causes. The live
/// local-stop flag is never serialized or inferred from retained/replayed JSON.
pub(super) fn observe(job:&Job,mut observation:Observation)->JournalObservation{
    let local_stop_transport_complete=matches!(observation.interruption.as_ref(),Some(E::Refused("task-validation-execution-interrupted")))
        && observation.result.as_ref().is_ok_and(|run|!run.timed_out&&!run.output_drain_timed_out)
        && observation.grant_attempted && observation.channel_closed && observation.reaped_status.as_ref().is_some_and(crate::execution_interrupt::stopped_status)
        && observation.gate.is_none() && observation.transport_errors.is_empty();
    let mut causes = Vec::new();
    let mut primary = None;
    let mut failed = observation.result.is_err()
        || observation.gate.is_some()
        || observation.interruption.is_some()
        || !observation.transport_errors.is_empty();
    let mut result = None;
    let mut retained = None;
    let mut total = None;
    let mut truncated = None;
    let mut settings_revision = None;
    match observation.result {
        Ok(mut run) => {
            if observation.gate.is_some() {
                run.result.executed = false;
                run.result.passed = false;
            }
            failed |= !run.result.passed;
            settings_revision = Some(run.settings_revision);
            retained = Some(run.output.retained().to_vec());
            total = Some(run.output.total());
            truncated = Some(run.output.truncated());
            result = Some(run.result);
        }
        Err(error) => {
            if let Some(output) = error.captured() {
                retained = Some(output.retained().to_vec());
                total = Some(output.total());
                truncated = Some(output.truncated());
            }
            let error = E::Launch(Box::new(crate::AgentLaunchError::caused(
                "task-validation-owned-process",
                error,
            )));
            causes.extend(crate::native_diagnostics::project(&error));
            primary = Some(error);
        }
    }
    if observation.gate.is_some(){
        let error=E::Refused("task-validation-private-gate-refused");
        causes.extend(crate::native_diagnostics::project(&error));merge(&mut primary,error);
    }
    for error in observation
        .transport_errors
        .drain(..)
        .chain(observation.interruption.take())
    {
        causes.extend(crate::native_diagnostics::project(&error));
        merge(&mut primary, error);
    }
    let receipt = StepReceipt {
        schema_version: 2,
        job: job.clone(),
        step: observation.step,
        process_attempted: true,
        grant_attempted: observation.grant_attempted,
        channel_closed: observation.channel_closed,
        failed,
        waited: observation.reaped_status.is_some(),
        exit_code: observation.reaped_status.and_then(|s| s.code()),
        signal: observation.reaped_status.and_then(|s| s.signal()),
        settings_revision,
        result,
        retained,
        total,
        truncated,
        causes,
        gate: observation.gate,
    };
    JournalObservation{receipt,primary,local_stop_transport_complete}
}
/// Record the actual original controller's decision not to invoke a remaining
/// slot, or setup failure before the primitive was attempted. No fork/reap is
/// asserted. Publication still checks the stored source and exact slot shape.
pub(super) fn unattempted(job: &Job, step: &Step, error: Option<&E>) -> StepReceipt {
    StepReceipt {
        schema_version: 2,
        job: job.clone(),
        step: step.clone(),
        process_attempted: false,
        grant_attempted: false,
        channel_closed: true,
        waited: false,
        failed: true,
        exit_code: None,
        signal: None,
        settings_revision: None,
        result: None,
        retained: None,
        total: None,
        truncated: None,
        causes: error
            .map(|cause| crate::native_diagnostics::project(cause))
            .unwrap_or_default(),
        gate: None,
    }
}
/// Preserve every independent error branch without IO or fallback success.
/// Keeping the original typed cause also retains owned unresolved Child data.
pub(super) fn merge(primary: &mut Option<E>, error: E) {
    *primary = Some(match primary.take() {
        None => error,
        Some(old) => E::Cleanup {
            primary: Some(Box::new(old)),
            cleanup: Box::new(error),
        },
    });
}
/// File-format addressing uses a full digest of a validated engine ID. It is
/// not a retention rule, operator path, identity proof or history scan limit.
pub(super) fn name(kind: &str, id: &str) -> String {
    format!(
        "native-validation-{kind}-{}.receipt",
        crate::provider_configuration::configuration_digest(id.as_bytes())
    )
}
/// Publish one original framed observation create-only, synchronizing complete
/// bytes and directory before any terminal SQL. Existing files are not replaced;
/// uncertainty retains the journal and original failure for explicit readback.
pub(super) fn persist(store: &RootStore, name: &str, body: &impl Serialize) -> Result<String, E> {
    let raw = serde_json::to_vec(body).map_err(|e| {
        E::Launch(Box::new(crate::AgentLaunchError::caused(
            "task-validation-journal-encode",
            e,
        )))
    })?;
    let frame = crate::native_receipt::encode(&raw).map_err(|e| {
        E::Launch(Box::new(crate::AgentLaunchError::caused(
            "task-validation-journal-frame",
            e,
        )))
    })?;
    store
        .directory
        .publish_new_with_fault(name, &frame, crate::local_storage::AtomicStorageFault::None)
        .map_err(E::Storage)?;
    Ok(crate::provider_configuration::configuration_digest(&raw))
}
/// Read a complete framed private journal and return its exact body hash.
/// Root ownership/regular file/frame extent/checksum and close errors refuse;
/// decoding strict domain fields proves no process or current authority itself.
pub(super) fn read<T: serde::de::DeserializeOwned>(
    store: &RootStore,
    name: &str,
) -> Result<(T, String), E> {
    let raw = store
        .directory
        .read_native_receipt(name)
        .map_err(E::Storage)?;
    let value = serde_json::from_slice(&raw).map_err(|e| {
        E::Launch(Box::new(crate::AgentLaunchError::caused(
            "task-validation-journal-decode",
            e,
        )))
    })?;
    Ok((
        value,
        crate::provider_configuration::configuration_digest(&raw),
    ))
}
/// Atomically publish the exact public JSON, or synchronize an already equal
/// file after uncertain publication. The expected extent is a format equality
/// bound, not an operator cap. Changed/oversized/symlink files refuse; no rewrite
/// or hidden retry converts a different artifact into the expected evidence.
pub(super) fn public_file(store: &RootStore, name: &str, raw: &[u8]) -> Result<(), E> {
    match store.directory.read_bounded(name, raw.len()) {
        Ok(existing) => {
            if existing != raw {
                return Err(E::Refused("task-validation-public-artifact-changed"));
            }
            store
                .directory
                .synchronize_existing(name)
                .map_err(E::Storage)
        }
        Err(error) if crate::root_store::absent(&error) => store
            .directory
            .publish_new_with_fault(name, raw, crate::local_storage::AtomicStorageFault::None)
            .map_err(E::Storage),
        Err(error) => Err(E::Storage(error)),
    }
}

/// Require a known wait after every attempted process primitive. A closed gate,
/// absent child tuple or missing permission cannot prove no fork or a reap.
/// Explicit unattempted slots must retain their pristine pre-invocation shape.
/// Pure receipt validation grants no execution, signal, result or task authority.
pub(super) fn quiescent(s: &StepReceipt) -> Result<(), crate::AgentLaunchError> {
    use crate::AgentLaunchError as A;
    if s.schema_version != 2
        || s.waited != (s.exit_code.is_some() || s.signal.is_some())
        || s.exit_code.is_some() && s.signal.is_some()
        || s.exit_code.is_some_and(|v| !(0..=255).contains(&v))
        || s.signal.is_some_and(|v| v < 1)
    {
        return Err(A::Refused("task-validation-wait-receipt-invalid"));
    }
    if s.process_attempted && !s.waited {
        return Err(A::Refused("task-validation-process-reap-unknown"));
    }
    if !s.process_attempted
        && (s.waited
            || s.grant_attempted
            || !s.channel_closed
            || s.step.phase != super::store::StepPhase::Prepared
            || s.step.permission != 0
            || s.step.child_pid.is_some()
            || s.step.parent_pid.is_some()
            || s.step.group_id.is_some()
            || s.step.child_stamp.is_some()
            || s.result.is_some()
            || !s.failed)
    {
        return Err(A::Refused("task-validation-unattempted-receipt-invalid"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Pure receipt fixture; numbers/IDs are test data, not kernel observations
    /// or authority. Runtime socket/pidfd checks live in the public recipe.
    fn receipt() -> StepReceipt {
        let job = Job {
            id: "j".into(),
            work_id: "w".into(),
            task_id: "t".into(),
            event_id: "e".into(),
            request_sha256: "a".repeat(64),
            source_sha256: "b".repeat(64),
            work_revision: 1,
            settings_revision: 1,
            owner_pid: 17,
            owner_stamp: "stamp".into(),
            owner_host: "fixture".into(),
            directory_device: "1".into(),
            directory_inode: "2".into(),
            control_count: 1,
            phase: super::super::store::JobPhase::Active,
            receipt_sha256: None,
            published_revision: None,
        };
        let step = Step {
            id: "s".into(),
            job_id: job.id.clone(),
            ordinal: 0,
            control_id: "check".into(),
            phase: super::super::store::StepPhase::Prepared,
            permission: 0,
            child_pid: None,
            parent_pid: None,
            group_id: None,
            child_stamp: None,
            receipt_sha256: None,
        };
        unattempted(&job, &step, None)
    }
    /// Valid pre-invocation and waited controls precede malformed/unknown cases.
    /// A primitive attempt with no tuple/grant and closed gate remains unknown;
    /// that counterexample was admitted by the previous grant-or-channel test.
    #[test]
    fn wait_required_after_every_primitive_attempt_even_without_tuple_or_grant() {
        let mut s = receipt();
        assert!(quiescent(&s).is_ok());
        s.process_attempted = true;
        assert!(!s.waited && !s.grant_attempted && s.channel_closed && s.step.child_pid.is_none());
        let old_refusal = !s.waited && (s.grant_attempted || !s.channel_closed);
        assert!(!old_refusal, "old predicate misses this unknown wait");
        assert_eq!(
            quiescent(&s).unwrap_err().code(),
            "task-validation-process-reap-unknown"
        );
        s.waited = true;
        s.exit_code = Some(0);
        assert!(quiescent(&s).is_ok());
        s.signal = Some(15);
        assert_eq!(
            quiescent(&s).unwrap_err().code(),
            "task-validation-wait-receipt-invalid"
        );
        s.signal = None;
        s.exit_code = Some(256);
        assert!(quiescent(&s).is_err());
        s.exit_code = Some(0);
        assert!(quiescent(&s).is_ok());
        s.process_attempted = false;
        assert_eq!(
            quiescent(&s).unwrap_err().code(),
            "task-validation-unattempted-receipt-invalid"
        );
        let mut s = receipt();
        s.schema_version = 1;
        assert!(quiescent(&s).is_err());
        let mut s = receipt();
        assert!(quiescent(&s).is_ok());
        s.step.permission = 1;
        assert!(quiescent(&s).is_err());
        let mut s = receipt();
        assert!(quiescent(&s).is_ok());
        s.channel_closed = false;
        assert!(quiescent(&s).is_err());
    }
}

#[cfg(test)]
mod interruption_cause_tests {
    use super::*;
    use crate::root_store::task_validation::store::{JobPhase,StepPhase};
    /// Synthetic observer DTOs only: no process, identity, permission, wait or
    /// publication is established. Real held-role journeys cover those separately.
    fn input(gate:bool,interrupted:bool)->(Job,Observation){
        use std::os::unix::process::ExitStatusExt;
        let job=Job{id:"fixture-job".into(),work_id:"fixture-work".into(),task_id:"fixture-task".into(),event_id:"fixture-event".into(),request_sha256:"0".repeat(64),source_sha256:"0".repeat(64),work_revision:1,settings_revision:1,owner_pid:123,owner_stamp:"1".into(),owner_host:"fixture".into(),directory_device:"1".into(),directory_inode:"1".into(),control_count:1,phase:JobPhase::Active,receipt_sha256:None,published_revision:None};
        let step=Step{id:"fixture-step".into(),job_id:job.id.clone(),ordinal:0,control_id:"fixture-control".into(),phase:StepPhase::Permitted,permission:1,child_pid:Some(124),parent_pid:Some(123),group_id:Some(124),child_stamp:Some("2".into()),receipt_sha256:None};
        let run=crate::ValidationRun{result:crate::ValidationControlResult{executed:true,passed:true,..Default::default()},output:crate::ValidationOutput::from_settings(&crate::EngineSettingsValues::installation_defaults().unwrap()).unwrap(),settings_revision:crate::SettingCount(1),timed_out:false,group_signal_delivered:false,output_drain_timed_out:false};
        let observed=Observation{step,result:Ok(run),grant_attempted:true,gate:gate.then(||crate::agent_execution_gate::diagnostic(&E::Refused("fixture-gate-denied"))),interruption:interrupted.then_some(E::Refused("task-validation-execution-interrupted")),channel_closed:true,transport_errors:vec![],reaped_status:Some(std::process::ExitStatus::from_raw(0))};
        (job,observed)
    }
    /// Exercise the actual live observer, not a serialized flag. Deadline,
    /// drain, gate, IO and independent exit exclude clean-stop metadata even
    /// with a local request. DTOs remain synthetic, not process/identity proof.
    #[test]
    fn control_local_stop_transport_excludes_independent_failures(){
        use std::os::unix::process::ExitStatusExt;
        for fault in 0..5{
            let (job,healthy)=input(false,true);let healthy=observe(&job,healthy);assert!(healthy.local_stop_transport_complete);
            assert!(serde_json::to_value(&healthy.receipt).unwrap().get("local_stop_transport_complete").is_none());
            let (job,mut observed)=input(false,true);
            match fault{
                0=>observed.gate=Some(crate::agent_execution_gate::diagnostic(&E::Refused("fixture-gate-denied"))),
                1=>observed.result.as_mut().unwrap().timed_out=true,
                2=>observed.result.as_mut().unwrap().output_drain_timed_out=true,
                3=>{observed.channel_closed=false;observed.transport_errors.push(E::Refused("fixture-close-failed"));},
                4=>{observed.reaped_status=Some(std::process::ExitStatus::from_raw(42<<8));observed.result.as_mut().unwrap().result.passed=false;},
                _=>unreachable!(),
            }
            assert!(!observe(&job,observed).local_stop_transport_complete);
        }
    }
    /// Healthy observation precedes each gate fault. Gate plus local stop must
    /// preserve both errors, otherwise a clean-stop classifier could hide denial.
    #[test]
    fn control_gate_and_interruption_causes_are_both_retained(){
        for interrupted in [false,true]{
            let (job,healthy)=input(false,false);let observed=observe(&job,healthy);assert!(!observed.receipt.failed&&observed.primary.is_none());
            let (job,fault)=input(true,interrupted);let observed=observe(&job,fault);assert!(observed.receipt.failed&&observed.receipt.gate.is_some());assert!(!observed.local_stop_transport_complete);
            let error=observed.primary.expect("private gate must produce a fatal cause");
            if interrupted{
                match error{E::Cleanup{primary:Some(primary),cleanup}=>{
                    let codes=[primary.code(),cleanup.code()];assert!(codes.contains(&"task-validation-private-gate-refused"));assert!(codes.contains(&"task-validation-execution-interrupted"));
                },_=>panic!("gate and local stop must both remain typed")}
            }else{assert_eq!(error.code(),"task-validation-private-gate-refused");}
        }
    }
}
