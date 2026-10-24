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
/// No raw kernel/owner fields are copied to the public validation artifact.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub schema_version: i64,
    pub job: Job,
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
fn claim(store: &mut RootStore, work: &str, r: &Request, raw: &[u8], lease: Option<&crate::MissionLease>) -> Result<Claim, E> {
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
                let prepared = super::prepare(tx, root, settings, work, r)?;
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
                let controls = prepared
                    .policy
                    .controls
                    .as_deref()
                    .ok_or(A::Refused("task-validation-controls-required"))?;
                let mut j = Job {
                    id: crate::generate_identifier("validation-")
                        .map_err(|e| caused("task-validation-id", e))?,
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
                let binding = lease.map(|lease| super::mission_authority::claim(tx,directory,lease,&w,&prepared.preview.producer_agent_id,settings,&j.id)).transpose()?;
                j.source_sha256=super::mission_authority::digest(&j.source_sha256,binding.as_ref())?;
                let steps = controls
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        Ok(Step {
                            id: crate::generate_identifier("validation-step-")
                                .map_err(|e| caused("task-validation-id", e))?,
                            job_id: j.id.clone(),
                            ordinal: i64::try_from(i)
                                .map_err(|e| caused("task-validation-slot-index", e))?,
                            control_id: c.id.clone(),
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
    let f = match claim(store, work, &r, &canonical, lease)? {
        Claim::Replay(j) => return publication::replay(store, &j),
        Claim::Fresh(f) => *f,
    };
    let known = (f.work.id.clone(), f.work.revision);
    let result = (|| {
        let mut records = Vec::new();
        let mut failure = None;
        let mut stop = false;
        let mut local_stop_transport_complete=false;
        let controls = f
            .prepared
            .policy
            .controls
            .as_deref()
            .ok_or(E::Refused("task-validation-controls-required"))?;
        for (slot, control) in f.steps.iter().zip(controls) {
            let (receipt, primary) = if stop {
                (journal::unattempted(&f.job, slot, failure.as_ref()), None)
            } else {
                match process::run(store, &f.job, slot, control, lease) {
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
            let record = Receipt {
                schema_version: 1,
                job: f.job,
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
/// missing/unknown observations remain exclusive, not successful empty recovery.
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
    let (record, hash): (Receipt, String) = journal::read(store, &journal::name("job", &j.id))?;
    publication::apply(store, &record, &hash, true)
}
