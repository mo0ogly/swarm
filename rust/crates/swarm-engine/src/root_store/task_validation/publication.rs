//! Publish original completed control observations, never new execution.
use super::{
    gate, journal,
    runtime::{Receipt, TaskValidationOutcome},
    store::{self, Job, JobPhase, StepPhase},
    Request,
};
use crate::{
    root_store::{RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
/// Preserve typed causes with a static public control family.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> A {
    A::caused(code, error)
}
/// Require the original process to be absent on its observed host. PID reuse is
/// checked by starttime; a missing process alone grants no result or signal right.
pub(super) fn departed(job: &Job) -> Result<(), A> {
    let current = crate::SupervisorIdentity::current()
        .map_err(|e| caused("task-validation-owner-observation", e))?;
    if current.host() != job.owner_host {
        return Err(A::Refused("task-validation-owner-host-changed"));
    }
    let pid = u32::try_from(job.owner_pid).map_err(|e| caused("task-validation-owner-pid", e))?;
    match crate::observe_linux_process(pid) {
        Ok(p) if p.process_stamp() != job.owner_stamp => Ok(()),
        Ok(_) => Err(A::Refused("task-validation-owner-still-live")),
        Err(e) if e.confirms_absence() => Ok(()),
        Err(e) => Err(caused("task-validation-owner-observation", e)),
    }
}
/// Compare immutable source/identity, excluding terminal publication fields.
fn same_origin(a: &Job, b: &Job) -> bool {
    let mut a = a.clone();
    a.phase = b.phase;
    a.receipt_sha256.clone_from(&b.receipt_sha256);
    a.published_revision = b.published_revision;
    a == *b
}
/// Replay only the exact recorded job. Returned controls_passed describes this
/// original command, not a new permission or timeless claim about current files.
/// Refused outcomes remain nonzero; no process, grant or Work mutation occurs.
pub(super) fn replay(store: &RootStore, job: &Job) -> Result<TaskValidationOutcome, E> {
    if job.phase != JobPhase::Recorded {
        return Err(E::Refused(if job.phase == JobPhase::Refused {
            "task-validation-original-refused"
        } else {
            "task-validation-reconciliation-required"
        }));
    }
    let (receipt, hash): (Receipt, String) = journal::read(store, &journal::name("job", &job.id))?;
    if job.receipt_sha256.as_deref() != Some(&hash) || !same_origin(job, &receipt.job) {
        return Err(E::Refused("task-validation-recorded-receipt-changed"));
    }
    let work = store.read_work(&job.work_id)?;
    let passed = all_passed(&receipt);
    if !passed {
        return Err(E::WorkRecorded {
            primary: Box::new(E::Refused("task-validation-controls-failed")),
            work: work.id,
            revision: work.revision,
        });
    }
    Ok(TaskValidationOutcome {
        schema_version: 1,
        job_id: job.id.clone(),
        controls_passed: true,
        task_accepted: false,
        retired: false,
        replayed: true,
        work,
    })
}
/// Require every original slot, known wait and successful authorized observation.
/// Empty/incomplete results, helper refusals and unknown streams never pass.
pub(super) fn all_passed(r: &Receipt) -> bool {
    usize::try_from(r.job.control_count).ok() == Some(r.steps.len())
        && !r.steps.is_empty()
        && r.steps.iter().all(|(s, _)| {
            s.waited
                && s.exit_code == Some(0)
                && s.signal.is_none()
                && s.grant_attempted
                && s.channel_closed
                && !s.failed
                && s.gate.is_none()
                && s.result
                    .as_ref()
                    .is_some_and(|v| v.executed && v.passed && v.exit_code == 0)
        })
}
/// Derive the public data artifact from exact original observations only. Private
/// host/PID/root metadata and raw unshared capture stay in private journals.
/// The receipt's own hash is added to Work after encoding, avoiding a circular
/// body. Ordinary roots have no managed candidate SHA; no commit is invented.
pub(super) fn public(r: &Receipt) -> Result<(crate::AutomaticValidation, String, Vec<u8>), A> {
    let name = format!(
        "native-validation-public-{}.json",
        crate::provider_configuration::configuration_digest(r.job.id.as_bytes())
    );
    let receipt = format!(".swarm/{name}");
    let passed = all_passed(r);
    let state = if !passed {
        "blocked"
    } else if r.policy.mode == "human" {
        "pending_human"
    } else {
        "pending_review"
    };
    let controls = r
        .steps
        .iter()
        .filter_map(|(s, _)| s.result.clone())
        .collect();
    let a = crate::AutomaticValidation {
        delivery_version: r.producer_delivery_version,
        attempt_id: r.attempt_id.clone(),
        revision: r.job.work_revision,
        candidate_sha: String::new(),
        producer_agent_id: r.producer_agent_id.clone(),
        controller: "controller://swarm-validation".into(),
        policy_digest: crate::validation_policy_digest(&r.policy)
            .map_err(|e| caused("task-validation-policy-digest", e))?,
        policy: r.policy.clone(),
        artifacts: Some(r.artifacts.clone()),
        controls: Some(controls),
        receipt,
        state: state.into(),
        reason: if passed {
            "controls-passed-review-required"
        } else {
            "controls-failed-or-incomplete"
        }
        .into(),
        at: r.finished_at.clone(),
    };
    let failures=r.steps.iter().filter(|(s,_)|s.failed||s.gate.is_some()).map(|(s,_)|serde_json::json!({"control_id":s.step.control_id,"causes":s.causes,"gate":s.gate})).collect::<Vec<_>>();
    let mut body =
        serde_json::to_value(&a).map_err(|e| caused("task-validation-public-encode", e))?;
    let object = body
        .as_object_mut()
        .ok_or(A::Refused("task-validation-public-envelope"))?;
    object.insert("schema_version".into(), 1.into());
    object.insert("job_id".into(), r.job.id.clone().into());
    object.insert("source_sha256".into(), r.job.source_sha256.clone().into());
    object.insert("failures".into(), failures.into());
    let raw = serde_json::to_vec(&body).map_err(|e| caused("task-validation-public-encode", e))?;
    Ok((a, name, raw))
}
/// Apply only exact original journals. Recovery requires owner departure; live
/// publication requires that actual owner. Every slot's tuple/permission and
/// original body hash are checked, and uncertain wait after a possible grant
/// remains exclusive. Files are published before the atomic Work/event/all-slots/
/// job transaction. Artifact/source/profile/SQL/close errors retain causes and
/// never rerun commands or claim task acceptance. Caller owns late output/Rootclose.
pub(super) fn apply(
    store: &mut RootStore,
    r: &Receipt,
    hash: &str,
    recovery: bool,
) -> Result<TaskValidationOutcome, E> {
    let launch = |e| E::Launch(Box::new(e));
    let current = original(store, r, hash, recovery)?;
    if current.phase == JobPhase::Recorded {
        return replay(store, &current);
    }
    if !matches!(current.phase, JobPhase::Active | JobPhase::Unresolved) {
        return Err(E::Refused("task-validation-original-refused"));
    }
    let (mut validation, name, raw) = public(r).map_err(launch)?;
    journal::public_file(store, &name, &raw)?;
    let public_hash = crate::provider_configuration::configuration_digest(&raw);
    validation
        .artifacts
        .as_mut()
        .ok_or(E::Refused("task-validation-artifacts-required"))?
        .insert(validation.receipt.clone(), public_hash);
    let root = &store.root;
    let directory = &store.directory;
    let mut committed = None;
    let result =
        crate::agent_storage_retry::execute(&mut store.db, directory, false, |db, settings| {
            let result = crate::agent_execution_store::transaction(db, |tx| {
                let old = store::job(tx, &current.id)?
                    .ok_or(A::Refused("task-validation-job-missing"))?;
                if old != current {
                    return Err(A::Refused("task-validation-publication-state-changed"));
                }
                if recovery {
                    departed(&old)?
                }
                let mut w = crate::read_recorded_work(tx, &old.work_id)
                    .map_err(|e| caused("task-validation-work", e))?;
                let request = Request {
                    schema_version: 1,
                    event_id: old.event_id.clone(),
                    expected_revision: w.revision,
                    task_id: old.task_id.clone(),
                };
                let prepared = super::prepare(tx, root, settings, &old.work_id, &request)?;
                if super::mission_authority::source(tx,&old,&prepared.preview.source_sha256)? != old.source_sha256 {
                    return Err(A::Refused("task-validation-source-changed"));
                }
                let task = w
                    .tasks
                    .as_ref()
                    .and_then(|v| v.iter().find(|t| t.id == old.task_id))
                    .ok_or(A::Refused("task-validation-task-missing"))?;
                let mut task = if validation.controls.as_ref().map(Vec::len) == Some(r.steps.len())
                {
                    gate::apply(
                        task,
                        validation
                            .artifacts
                            .as_ref()
                            .ok_or(A::Refused("task-validation-artifacts-required"))?,
                        validation.controls.as_deref().unwrap_or_default(),
                        root,
                        settings,
                        &r.finished_at,
                    )?
                } else {
                    let mut t = task.clone();
                    t.evidence_stale_reason = "validation-incomplete".into();
                    t
                };
                task.automatic_validation = Some(Box::new(validation.clone()));
                if !all_passed(r) {
                    task.status = "blocked".into();
                    task.blocker = "controls-failed-or-incomplete".into();
                }
                if all_passed(r) {
                    crate::check_validation_evidence(
                        root,
                        &task,
                        Some(
                            &settings
                                .quality_policy_snapshot()
                                .map_err(|e| caused("task-validation-quality-policy", e))?,
                        ),
                        settings
                            .values
                            .validation_output_limit()
                            .map_err(|e| caused("task-validation-output-policy", e))?,
                    )
                    .map_err(|e| caused("task-validation-current-evidence", e))?;
                }
                let target = w
                    .tasks
                    .as_mut()
                    .and_then(|v| v.iter_mut().find(|t| t.id == old.task_id))
                    .ok_or(A::Refused("task-validation-task-missing"))?;
                *target = task;
                let revision = w.revision;
                w.revision = revision
                    .checked_add(1)
                    .ok_or(A::Refused("task-validation-revision-overflow"))?;
                w.updated =
                    crate::system_timestamp().map_err(|e| caused("task-validation-clock", e))?;
                let event = crate::encode_model(&validation)
                    .map_err(|e| caused("task-validation-event-encode", e))?;
                let stage = crate::stage_recorded_change_with_hook(
                    tx,
                    crate::RecordedChange {
                        work: &w,
                        expected_revision: revision,
                        event: &r.result_event,
                        kind: "task.auto-validation",
                        request: &event,
                    },
                    |db, _| {
                        for (s, body_hash) in &r.steps {
                            let (_, before) = store::step(db, &s.step.id)?;
                            if before != s.step {
                                return Err(Box::new(A::Refused(
                                    "task-validation-slot-state-changed",
                                ))
                                    as Box<dyn std::error::Error + Send + Sync>);
                            }
                            let mut done = before.clone();
                            done.phase = if s.waited && s.step.permission == 1 {
                                StepPhase::Completed
                            } else {
                                StepPhase::Refused
                            };
                            done.receipt_sha256 = Some(body_hash.clone());
                            store::change_step(db, &before, &done)?;
                        }
                        let mut done = old.clone();
                        done.phase = JobPhase::Recorded;
                        done.receipt_sha256 = Some(hash.into());
                        done.published_revision = Some(w.revision);
                        store::change_job(db, &old, &done)?;
                        Ok(())
                    },
                )
                .map_err(|e| caused("task-validation-result-publication", e))?;
                if !matches!(stage, crate::RecordedChangeOutcome::Staged) {
                    return Err(A::Refused("task-validation-result-event-conflict"));
                }
                Ok(TaskValidationOutcome {
                    schema_version: 1,
                    job_id: old.id,
                    controls_passed: all_passed(r),
                    task_accepted: false,
                    retired: false,
                    replayed: recovery,
                    work: w,
                })
            });
            if let Ok(out) = &result {
                committed = Some((out.work.id.clone(), out.work.revision));
            }
            result.map_err(launch)
        });
    result.map_err(|primary| match committed {
        Some((work, revision)) => E::WorkRecorded {
            primary: Box::new(primary),
            work,
            revision,
        },
        None => primary,
    })
}

/// Verify original framed bodies, immutable source/root/owner identity and known
/// process outcomes before publication or explicit retirement. Recovery requires
/// owner departure. A complete wait is distinct from a closed gate. No current
/// domain source is adopted and no command or state mutation occurs here.
pub(super) fn original(
    store: &RootStore,
    r: &Receipt,
    hash: &str,
    recovery: bool,
) -> Result<Job, E> {
    original_for(
        store,
        r,
        hash,
        if recovery {
            OriginalUse::Recovery
        } else {
            OriginalUse::CurrentOwner
        },
    )
}
/// Closed internal use modes. Reading published evidence never transfers a live
/// owner or recovers a job; RecordedRead is permitted only for terminal rows.
enum OriginalUse {
    CurrentOwner,
    Recovery,
    RecordedRead,
}
/// Read immutable published evidence without requiring a prior server to exit.
/// Verifies the same original/root/slot/wait material as owner/recovery paths;
/// terminal publication is mandatory. No command, ownership transfer or mutation.
pub(super) fn recorded_original(store: &RootStore, r: &Receipt, hash: &str) -> Result<Job, E> {
    original_for(store, r, hash, OriginalUse::RecordedRead)
}
/// Check original bytes, normalized immutable origin and complete quiescent slots
/// under a closed ownership contract. Caller retains current source/Work checks;
/// success alone grants no new command, reviewer authority or task acceptance.
fn original_for(store: &RootStore, r: &Receipt, hash: &str, mode: OriginalUse) -> Result<Job, E> {
    let launch = |e| E::Launch(Box::new(e));
    let (saved, body_hash): (Receipt, String) =
        journal::read(store, &journal::name("job", &r.job.id))?;
    if body_hash != hash
        || crate::provider_configuration::configuration_digest(
            &serde_json::to_vec(r)
                .map_err(|e| launch(caused("task-validation-journal-encode", e)))?,
        ) != hash
        || saved.schema_version != 1
    {
        return Err(E::Refused("task-validation-original-receipt-changed"));
    }
    let current = store::job(&store.db, &r.job.id)
        .map_err(launch)?
        .ok_or(E::Refused("task-validation-job-missing"))?;
    if !same_origin(&current, &r.job) {
        return Err(E::Refused("task-validation-job-source-changed"));
    }
    match mode {
        OriginalUse::Recovery => departed(&current).map_err(launch)?,
        OriginalUse::CurrentOwner => {
            let owner = crate::SupervisorIdentity::current()
                .map_err(|e| launch(caused("task-validation-owner-observation", e)))?;
            if owner.pid() != current.owner_pid
                || owner.process_stamp() != current.owner_stamp
                || owner.host() != current.owner_host
            {
                return Err(E::Refused("task-validation-owner-changed"));
            }
        }
        OriginalUse::RecordedRead => {
            if current.phase != JobPhase::Recorded || current.published_revision.is_none() {
                return Err(E::Refused("task-validation-recorded-publication-required"));
            }
        }
    }
    let directory_identity = store.directory.identity().map_err(E::Storage)?;
    if directory_identity.0.to_string() != current.directory_device
        || directory_identity.1.to_string() != current.directory_inode
    {
        return Err(E::Refused("task-validation-root-changed"));
    }
    if usize::try_from(current.control_count).ok() != Some(r.steps.len()) {
        return Err(E::Refused("task-validation-slot-set-incomplete"));
    }
    for (index, (s, expected)) in r.steps.iter().enumerate() {
        let (_, original_hash): (journal::StepReceipt, String) =
            journal::read(store, &journal::name("step", &s.step.id))?;
        if original_hash != *expected
            || crate::provider_configuration::configuration_digest(
                &serde_json::to_vec(s)
                    .map_err(|e| launch(caused("task-validation-step-encode", e)))?,
            ) != *expected
            || s.schema_version != 2
            || s.job != r.job
            || s.step.job_id != r.job.id
            || s.step.ordinal
                != i64::try_from(index)
                    .map_err(|e| launch(caused("task-validation-slot-index", e)))?
        {
            return Err(E::Refused("task-validation-original-slot-changed"));
        }
        journal::quiescent(s).map_err(launch)?;
    }
    Ok(current)
}
