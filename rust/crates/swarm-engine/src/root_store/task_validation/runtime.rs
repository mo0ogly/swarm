//! Foreground operator control batch. No caller-provided command/origin/actor is
//! admitted; automated conductors still owe their separate mission consent/lease.
use super::{
    gate,
    journal::{self, StepReceipt},
    process, publication,
    store::{self, Job, JobPhase, Step, StepPhase},
    Prepared, Request,
};
use crate::{
    root_store::{RootStore, RootStoreError as E},
    AgentLaunchError as A, Work,
};
use serde::{Deserialize, Serialize};

/// A recorded control result, never independent review or acceptance. Replay is
/// the original command outcome plus current Work, not a new permission to run.
#[derive(Serialize)]
pub struct TaskValidationOutcome {
    pub schema_version: i64,
    pub job_id: String,
    pub controls_passed: bool,
    pub task_accepted: bool,
    /// Explicit abandonment of observations, never a successful control result.
    pub retired: bool,
    pub replayed: bool,
    pub work: Work,
}
/// Private complete original observations before public artifact/SQL. Embedded
/// per-slot bodies and hashes permit exact recovery, never a replacement process.
/// Ordinary body version2 requires source_origin; historical version1 omits it
/// and keeps its original encoding. Actual managed execution produces version3
/// with the complete candidate/cohort preimage; final managed review stays guarded.
/// These are protocol versions, not policy knobs.
/// No raw kernel/owner fields are copied to the public validation artifact.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub schema_version: i64,
    pub job: Job,
    /// Absent on historical originals only; a new origin must match its scalar
    /// SQL row. Omission preserves historical canonical receipt bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_origin: Option<super::source_origin::Origin>,
    /// Ordinary v2 only: exact normalized commands hashed in source_origin.
    /// Schema3 omits this field and binds the full candidate cohort instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_policy: Option<crate::ValidationPolicy>,
    /// Private schema3 only: complete candidate/cohort preimage, never a grant.
    /// Ordinary v1/v2 must omit this field so historical canonical bytes survive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_candidate: Option<crate::managed_repository::ManagedCandidate>,
    pub steps: Vec<(StepReceipt, String)>,
    pub policy: crate::ValidationPolicy,
    pub artifacts: std::collections::BTreeMap<String, String>,
    pub producer_agent_id: String,
    pub attempt_id: String,
    pub producer_delivery_version: i64,
    pub finished_at: String,
    pub result_event: String,
}
struct Fresh {
    job: Job,
    source_origin: super::source_origin::Origin,
    steps: Vec<Step>,
    prepared: Prepared,
    work: Work,
}
/// Private owned admission or immutable replay. Heap indirection keeps the
/// controller stack independent of the full prepared Work or job representation.
enum Claim {
    Fresh(Box<Fresh>),
    Replay(Box<Job>),
}
/// Preserve typed SQL/runtime/codec causes under a static control family.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> A {
    A::caused(code, error)
}
/// Canonical intent identifies exact command replay, not actor authority.
fn intent(r: &Request) -> Result<Vec<u8>, E> {
    serde_json::to_vec(r)
        .map_err(|e| E::Launch(Box::new(caused("task-validation-request-encode", e))))
}
/// Admit and register every immutable slot in the same transaction as clearing
/// obsolete current validation. Keep existing gate configuration; precheck may
/// refuse its replacement before any process. Known commit metadata survives
/// later saved-settings release failure; unknown commit is never relabeled PASS.
/// Exact terminal replay is readback without fresh execution authority; a live
/// or uncertain original remains refused by publication, never relaunched.
fn claim(store: &mut RootStore, work: &str, r: &Request, raw: &[u8], lease: Option<&crate::MissionLease>, managed: Option<&super::managed_execution::Session>, managed_job: Option<&str>) -> Result<Claim, E> {
    let directory = &store.directory;
    let root = &store.root;
    let mut known = None;
    let result =
        crate::agent_storage_retry::execute(&mut store.db, directory, false, |db, settings| {
            let committed = crate::agent_execution_store::transaction(db, |tx| {
                let request_sha256 = crate::provider_configuration::configuration_digest(raw);
                if let Some(j) = store::event(tx, &r.event_id)? {
                    if j.work_id != work
                        || j.task_id != r.task_id
                        || j.request_sha256 != request_sha256
                    {
                        return Err(A::Refused("task-validation-replay-conflict"));
                    }
                    // Readback never starts a slot or creates permission. The
                    // publisher validates original terminal receipt/rows; active
                    // or uncertain jobs still require explicit reconciliation.
                    return Ok(Claim::Replay(Box::new(j)));
                }
                if store::active(tx, work, &r.task_id)?.is_some() {
                    return Err(A::Refused("task-validation-controller-active"));
                }
                let prepared = match managed {
                    Some(session) => super::managed_execution::prepare(tx, directory, root, settings, work, r, &session.data)?,
                    None => super::prepare(tx, root, settings, work, r)?,
                };
                let mut w = crate::read_recorded_work(tx, work)
                    .map_err(|e| caused("task-validation-work", e))?;
                let task = w
                    .tasks
                    .iter()
                    .flatten()
                    .find(|t| t.id == r.task_id)
                    .ok_or(A::Refused("task-validation-task-missing"))?;
                gate::precheck(task, &prepared.preview.artifacts, root, settings)?;
                let owner = crate::SupervisorIdentity::current()
                    .map_err(|e| caused("task-validation-owner-observation", e))?;
                let identity = directory
                    .identity()
                    .map_err(|e| caused("task-validation-directory-observation", e))?;
                let controls = prepared.controls()?;
                let mut j = Job {
                    id: match managed_job {
                        Some(id) if managed.is_some() => id.into(),
                        None if managed.is_none() => crate::generate_identifier("validation-")
                            .map_err(|e| caused("task-validation-id", e))?,
                        _ => return Err(A::Refused("task-validation-managed-job-binding")),
                    },
                    work_id: work.into(),
                    task_id: r.task_id.clone(),
                    event_id: r.event_id.clone(),
                    request_sha256,
                    source_sha256: prepared.preview.source_sha256.clone(),
                    work_revision: w.revision,
                    settings_revision: settings.revision.0,
                    owner_pid: owner.pid(),
                    owner_stamp: owner.process_stamp().into(),
                    owner_host: owner.host().into(),
                    directory_device: identity.0.to_string(),
                    directory_inode: identity.1.to_string(),
                    control_count: i64::try_from(controls.len())
                        .map_err(|e| caused("task-validation-slot-count", e))?,
                    phase: JobPhase::Active,
                    receipt_sha256: None,
                    published_revision: None,
                };
                let source_origin = prepared.origin(&j.id)?;
                j.source_sha256 = super::source_origin::digest(&source_origin)?;
                let binding = lease.map(|lease| super::mission_authority::claim(tx,directory,lease,&w,&prepared.preview.producer_agent_id,settings,&j.id)).transpose()?;
                j.source_sha256=super::mission_authority::digest(&j.source_sha256,binding.as_ref())?;
                let steps = controls
                    .iter()
                    .enumerate()
                    .map(|(i, (transport, _control))| {
                        Ok(Step {
                            id: crate::generate_identifier("validation-step-")
                                .map_err(|e| caused("task-validation-id", e))?,
                            job_id: j.id.clone(),
                            ordinal: i64::try_from(i)
                                .map_err(|e| caused("task-validation-slot-index", e))?,
                            control_id: (*transport).into(),
                            phase: StepPhase::Prepared,
                            permission: 0,
                            child_pid: None,
                            parent_pid: None,
                            group_id: None,
                            child_stamp: None,
                            receipt_sha256: None,
                        })
                    })
                    .collect::<Result<Vec<_>, A>>()?;
                let task = w
                    .tasks
                    .as_mut()
                    .and_then(|ts| ts.iter_mut().find(|t| t.id == r.task_id))
                    .ok_or(A::Refused("task-validation-task-missing"))?;
                task.automatic_validation = None;
                task.evidence_stale_reason = "validation-controller-active".into();
                w.revision = w
                    .revision
                    .checked_add(1)
                    .ok_or(A::Refused("task-validation-revision-overflow"))?;
                w.updated =
                    crate::system_timestamp().map_err(|e| caused("task-validation-clock", e))?;
                let staged = crate::stage_recorded_change_with_hook(
                    tx,
                    crate::RecordedChange {
                        work: &w,
                        expected_revision: r.expected_revision,
                        event: &r.event_id,
                        kind: "task.validation-start",
                        request: raw,
                    },
                    |db, _| {
                        store::claim(db, &j, &steps)
                            .and_then(|()| super::source_origin::stage(db, &source_origin))
                            .and_then(|()| match binding.as_ref() {Some(b)=>super::mission_authority::stage(db,b),None=>Ok(())})
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    },
                )
                .map_err(|e| caused("task-validation-claim-publication", e))?;
                if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                    return Err(A::Refused("task-validation-recorded-job-missing"));
                }
                Ok(Claim::Fresh(Box::new(Fresh {
                    job: j,
                    source_origin,
                    steps,
                    prepared,
                    work: w,
                })))
            });
            if let Ok(Claim::Fresh(f)) = &committed {
                known = Some((f.work.id.clone(), f.work.revision));
            }
            committed.map_err(|e| E::Launch(Box::new(e)))
        });
    result.map_err(|primary| match known {
        Some((work, revision)) => E::WorkRecorded {
            primary: Box::new(primary),
            work,
            revision,
        },
        None => primary,
    })
}
/// Execute exactly one admitted foreground batch. Every process's original
/// receipt is durable before the next invocation; fatal source/transport errors
/// leave later slots explicitly unattempted. Ordinary failed checks continue the
/// configured ordered set, as in the existing validation contract. Never retry a
/// command, reset a budget, infer a reap or promote helper EOF into execution.
/// Known claimed Work survives later errors and requires readback before retry.
pub(super) fn run(
    store: &mut RootStore,
    work: &str,
    raw: &[u8],
) -> Result<TaskValidationOutcome, E> {
    run_using(store,work,raw,None)
}
/// Internal engine-only admission with an actual borrowed owning lease.
/// No JSON field confers authority; original known Work/errors remain preserved.
pub(super) fn run_using(store:&mut RootStore,work:&str,raw:&[u8],lease:Option<&crate::MissionLease>)->Result<TaskValidationOutcome,E>{
    let r = super::decode(raw).map_err(|e| E::Launch(Box::new(e)))?;
    let canonical = intent(&r)?;
    // Preserve exact-event readback before any candidate Git or frame creation.
    let exists = store::event(&store.db, &r.event_id).map_err(|e| E::Launch(Box::new(e)))?.is_some();
    let managed = if exists { None } else { super::managed_execution::acquire_fresh(store, work, &r, true)? };
    let result = (|| {
        let id = match managed.as_ref() {
            Some(session) => {
                let id = crate::generate_identifier("validation-").map_err(E::Runtime)?;
                super::managed_execution::persist(store, &id, session)?;
                Some(id)
            }
            None => None,
        };
        run_prepared(store, work, &r, &canonical, lease, managed.as_ref(), id.as_deref())
    })();
    let known = match &result {
        Ok(outcome) => Some((outcome.work.id.clone(), outcome.work.revision)),
        Err(error) => error.recorded_work().map(|(work, rev)| (work.to_owned(), rev)),
    };
    let close = managed.map_or(Ok(()), |session| session.close());
    crate::root_store::finished(result, close).map_err(|error| {
        match known {
            Some((work, revision)) if error.recorded_work().is_none() => E::WorkRecorded { primary: Box::new(error), work, revision },
            _ => error,
        }
    })
}
/// Run one prepared batch while its external owner retains candidate custody.
/// Claim revalidates Root under the writer; replay owns no new process permit.
/// Original commands, slot journals and known Work metadata remain authoritative.
fn run_prepared(store: &mut RootStore, work: &str, r: &Request, canonical: &[u8], lease: Option<&crate::MissionLease>,
    managed: Option<&super::managed_execution::Session>, managed_job: Option<&str>) -> Result<TaskValidationOutcome, E> {
    let f = match claim(store, work, r, canonical, lease, managed, managed_job)? {
        Claim::Replay(j) => return publication::replay(store, &j),
        Claim::Fresh(f) => *f,
    };
    let known = (f.work.id.clone(), f.work.revision);
    let result = (|| {
        let mut records = Vec::new();
        let mut failure = None;
        let mut stop = false;
        let mut local_stop_transport_complete=false;
        let controls = f.prepared.controls().map_err(|e| E::Launch(Box::new(e)))?;
        for (slot, (_transport, control)) in f.steps.iter().zip(controls) {
            let (receipt, primary) = if stop {
                (journal::unattempted(&f.job, slot, failure.as_ref()), None)
            } else {
                match process::run(store, &f.job, slot, control, lease, managed) {
                    Ok(observed)=>{
                        let observed=journal::observe(&f.job,observed);
                        local_stop_transport_complete|=observed.local_stop_transport_complete;
                        (observed.receipt,observed.primary)
                    },
                    Err(error) => {
                        let receipt = journal::unattempted(&f.job, slot, Some(&error));
                        (receipt, Some(error))
                    }
                }
            };
            if let Some(error) = primary {
                stop = true;
                journal::merge(&mut failure, error);
            }
            let saved = journal::persist(store, &journal::name("step", &slot.id), &receipt);
            let hash = match saved {
                Ok(hash) => hash,
                Err(error) => {
                    journal::merge(&mut failure, error);
                    return Err(failure.expect("failure was just retained"));
                }
            };
            records.push((receipt, hash));
        }
        let published = (|| {
            let candidate = f.prepared.managed.as_ref().map(|m| m.candidate.clone());
            let record = Receipt {
                schema_version: if candidate.is_some() { 3 } else { 2 },
                effective_policy: if candidate.is_some() { None } else { Some(f.prepared.policy) },
                managed_candidate: candidate,
                job: f.job,
                source_origin: Some(f.source_origin),
                steps: records,
                policy: f.prepared.original_policy,
                artifacts: f.prepared.preview.artifacts,
                producer_agent_id: f.prepared.preview.producer_agent_id,
                attempt_id: f.prepared.preview.attempt_id,
                producer_delivery_version: f.prepared.producer_delivery_version,
                finished_at: crate::system_timestamp().map_err(E::Runtime)?,
                result_event: crate::generate_identifier("validation-result-")
                    .map_err(E::Runtime)?,
            };
            let body_hash =
                journal::persist(store, &journal::name("job", &record.job.id), &record)?;
            publication::apply(store, &record, &body_hash, false)
        })();
        match (published, failure) {
            (Ok(out), None) if out.controls_passed => Ok(out),
            (Ok(out), None) => Err(E::WorkRecorded {
                primary: Box::new(E::Refused("task-validation-controls-failed")),
                work: out.work.id,
                revision: out.work.revision,
            }),
            (Ok(out),Some(primary))=>{
                let primary=if local_stop_transport_complete && matches!(&primary,E::Refused("task-validation-execution-interrupted")){
                    crate::execution_interrupt::completed(crate::execution_interrupt::InterruptedRole::Controls,primary)
                }else{primary};
                Err(E::WorkRecorded{primary:Box::new(primary),work:out.work.id,revision:out.work.revision})
            },
            (Err(primary), None) => Err(primary),
            (Err(primary), Some(original)) => Err(E::Cleanup {
                primary: Some(Box::new(original)),
                cleanup: Box::new(primary),
            }),
        }
    })();
    result.map_err(|primary| {
        if primary.recorded_work().is_some() {
            primary
        } else {
            E::WorkRecorded {
                primary: Box::new(primary),
                work: known.0,
                revision: known.1,
            }
        }
    })
}
/// Reconcile original immutable observations without a new process. The publisher
/// requires actual owner departure and exact source/rows/receipt before applying;
/// A missing combined body may be assembled from every exact known slot and
/// current source; missing/unknown slot observations remain exclusive.
pub(super) fn reconcile(
    store: &mut RootStore,
    work: &str,
    raw: &[u8],
) -> Result<TaskValidationOutcome, E> {
    let r = super::decode(raw).map_err(|e| E::Launch(Box::new(e)))?;
    let canonical = intent(&r)?;
    let j = store::event(&store.db, &r.event_id)
        .map_err(|e| E::Launch(Box::new(e)))?
        .ok_or(E::Refused("task-validation-job-missing"))?;
    if j.work_id != work
        || j.task_id != r.task_id
        || j.request_sha256 != crate::provider_configuration::configuration_digest(&canonical)
    {
        return Err(E::Refused("task-validation-replay-conflict"));
    }
    if matches!(j.phase, JobPhase::Recorded | JobPhase::Refused) {
        return publication::replay(store, &j);
    }
    let (record, hash) = super::reconstruction::read(store, &j)?;
    publication::apply(store, &record, &hash, true)
}
