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
/// Refused outcomes remain nonzero; policy/slot/result mapping is rechecked
/// before exposing outcome data. No process, grant or Work mutation occurs.
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
    bound_slots(&receipt).map_err(|e| E::Launch(Box::new(e)))?;
    verify_origin(&store.db, job, &receipt).map_err(|e| E::Launch(Box::new(e)))?;
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
/// Check the complete stored-policy → original-slot → observed-result mapping.
///
/// Receives private receipt data, never an execution grant. Requires supported
/// receipt versions, exact counts/order/job identity, unique step/control names
/// and result identity/argv matching the effective controls. Executable argv
/// must also satisfy the existing normalizer's nonempty/non-NUL form; two equal
/// malformed commands cannot become success. Managed slots use unique transport
/// names while retaining per-task original control names. Returns optional v3
/// candidate/cohort binding, None for ordinary originals. Failed or unattempted
/// slots may lack a result, but may not borrow another command's result. Missing
/// or unsuccessful results must carry the failed flag; otherwise refuse rather
/// than emitting a blocked artifact with no per-slot failure entry. Returns
/// a safe static refusal on any mismatch, without IO, mutation or partial public
/// conversion. Actual SQL rows, native owner, hashes, permission and quiescence
/// remain obligations of original_for and its callers; data equality grants none.
fn bound_slots(r: &Receipt) -> Result<Option<super::managed_receipt::Binding<'_>>, A> {
    let invalid = || A::Refused("task-validation-receipt-slot-source-changed");
    let managed = if r.schema_version == 3 || r.managed_candidate.is_some() {
        super::managed_receipt::bound(r)?
    } else { None };
    let controls: Vec<(&str, &crate::ValidationControl)> = if let Some(binding) = &managed {
        binding.plan.slots.iter().map(|slot| (slot.transport_id.as_str(), &slot.control)).collect()
    } else {
        let version_matches_origin = matches!((r.schema_version, r.source_origin.is_some(), r.effective_policy.is_some()), (1, false, false) | (2, true, true));
        if !version_matches_origin { return Err(invalid()); }
        if let (Some(origin), Some(policy)) = (r.source_origin.as_ref(), r.effective_policy.as_ref()) {
            origin.verify_controls(&r.job.id, policy)?;
            origin.verify_candidate_binding(None)?;
        }
        r.effective_policy.as_ref().unwrap_or(&r.policy).controls.as_deref()
            .filter(|c| !c.is_empty()).ok_or_else(invalid)?
            .iter().map(|c| (c.id.as_str(), c)).collect()
    };
    if controls.is_empty() || controls.len() != r.steps.len()
        || usize::try_from(r.job.control_count).ok() != Some(r.steps.len()) {
        return Err(invalid());
    }
    let mut names = std::collections::BTreeSet::new();
    let mut steps = std::collections::BTreeSet::new();
    for (index, ((s, _), (transport, control))) in r.steps.iter().zip(controls).enumerate() {
        let argv = control.command.as_deref().filter(|args| !args.is_empty()).ok_or_else(invalid)?;
        if argv.iter().any(|arg| crate::text::trim_space(arg).is_empty() || arg.contains('\0')) {
            return Err(invalid());
        }
        if s.schema_version != 2 || s.job != r.job || s.step.job_id != r.job.id
            || i64::try_from(index).ok() != Some(s.step.ordinal)
            || s.step.control_id != transport || !names.insert(transport)
            || !steps.insert(s.step.id.as_str())
            || s.result.as_ref().is_some_and(|result| result.id != control.id || result.command != control.command)
            || !s.failed && s.result.as_ref().is_none_or(|result| !result.executed || !result.passed || result.exit_code != 0) {
            return Err(invalid());
        }
    }
    Ok(managed)
}
/// Check full SQL origin backing for the receipt's explicit private variant.
/// Legacy/ordinary records cannot claim a candidate digest without a preimage;
/// schema3 binds the complete control plan and candidate digest to the Job.
/// Reads SQL only; no command, current source freshness or grant is established.
fn verify_origin(db: &rusqlite::Connection, job: &Job, receipt: &Receipt) -> Result<(), A> {
    match bound_slots(receipt)? {
        Some(binding) => super::source_origin::verify_bound_job_receipt(
            db, job, receipt.source_origin.as_ref(), Some(binding.plan), Some(binding.candidate_sha256.as_str())),
        None => super::source_origin::verify_job_receipt(
            db, job, receipt.source_origin.as_ref(), receipt.effective_policy.as_ref()),
    }
}
/// Require policy-bound original slots, known waits and successful observations.
/// Empty, foreign/incomplete results, helper refusals and unknown streams never
/// pass. This pure predicate neither checks SQL authority nor grants execution;
/// callers performing public conversion use bound_slots to preserve diagnostics.
pub(super) fn all_passed(r: &Receipt) -> bool {
    bound_slots(r).is_ok() && observed_passed(r)
}
/// Require known successful process observations for every already bound slot.
/// This helper alone checks no mapping/authority; public conversion first binds
/// the whole receipt, including every cumulative slot, then uses this predicate.
fn observed_passed(r: &Receipt) -> bool {
    !r.steps.is_empty()
        && r.steps.iter().all(|(s, _)| {
            s.process_attempted
                && s.step.phase == StepPhase::Permitted
                && s.step.permission == 1
                && s.waited
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
/// Managed schema3 projects only target-task results with its actual candidate
/// SHA, but every cohort failure blocks the overall outcome and stays attributed.
/// The receipt's own hash is added to Work after encoding, avoiding a circular
/// body. Ordinary roots have no managed candidate SHA; no commit is invented.
/// Incoherent policy/slot/result mapping and codec errors refuse before returning
/// any artifact bytes. Known failed/unattempted results remain blocked data;
/// this conversion performs no publication, SQL mutation or authority check.
pub(super) fn public(r: &Receipt) -> Result<(crate::AutomaticValidation, String, Vec<u8>), A> {
    let managed = bound_slots(r)?;
    let name = format!(
        "native-validation-public-{}.json",
        crate::provider_configuration::configuration_digest(r.job.id.as_bytes())
    );
    let receipt = format!(".swarm/{name}");
    let passed = observed_passed(r);
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
        .enumerate()
        .filter(|(index, _)| managed.as_ref().is_none_or(|binding|
            binding.plan.slots[*index].task_id == r.job.task_id))
        .filter_map(|(_, (s, _))| s.result.clone())
        .collect();
    let a = crate::AutomaticValidation {
        delivery_version: r.producer_delivery_version,
        attempt_id: r.attempt_id.clone(),
        revision: r.job.work_revision,
        candidate_sha: managed.as_ref().map(|b| b.candidate.commit.clone()).unwrap_or_default(),
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
    let failures=r.steps.iter().enumerate().filter(|(_, (s,_))|s.failed||s.gate.is_some()).map(|(index,(s,_))| {
        let mut value=serde_json::json!({"control_id":s.step.control_id,"causes":s.causes,"gate":s.gate});
        if let Some(binding)=&managed {
            let slot=&binding.plan.slots[index];
            value["task_id"]=serde_json::json!(slot.task_id);
            value["control_id"]=serde_json::json!(slot.control.id);
            value["transport_id"]=serde_json::json!(slot.transport_id);
        }
        value
    }).collect::<Vec<_>>();
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
    let current = original(store, r, hash, recovery)?;
    if current.phase == JobPhase::Recorded {
        return replay(store, &current);
    }
    if !matches!(current.phase, JobPhase::Active | JobPhase::Unresolved) {
        return Err(E::Refused("task-validation-original-refused"));
    }
    let managed = r.managed_candidate.as_ref().map(|candidate|
        super::managed_execution::acquire_job(store, &current, candidate)).transpose()?;
    let result = apply_prepared(store, r, hash, recovery, current, managed.as_ref());
    let known = match &result {
        Ok(outcome) => Some((outcome.work.id.clone(), outcome.work.revision)),
        Err(error) => error.recorded_work().map(|(work, rev)| (work.to_owned(), rev)),
    };
    crate::root_store::finished(result, managed.map_or(Ok(()), |session| session.close())).map_err(|error| {
        match known {
            Some((work, revision)) if error.recorded_work().is_none() => E::WorkRecorded { primary: Box::new(error), work, revision },
            _ => error,
        }
    })
}
/// Publish original observations while an external owner retains any managed
/// candidate custody. Root source revalidation remains inside the writer, but
/// candidate Git/raw-source inspection was completed outside its locks.
fn apply_prepared(store: &mut RootStore, r: &Receipt, hash: &str, recovery: bool, current: Job,
    managed: Option<&super::managed_execution::Session>) -> Result<TaskValidationOutcome, E> {
    let launch = |e| E::Launch(Box::new(e));
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
                let prepared = match managed {
                    Some(session) => super::managed_execution::prepare(tx, directory, root, settings, &old.work_id, &request, &session.data)?,
                    None => super::prepare(tx, root, settings, &old.work_id, &request)?,
                };
                if super::managed_execution::bound_source(tx,&old,&prepared.preview.source_sha256,prepared.managed.as_ref().map(|m| &m.candidate))? != old.source_sha256 {
                    return Err(A::Refused("task-validation-source-changed"));
                }
                let task = w
                    .tasks
                    .as_ref()
                    .and_then(|v| v.iter().find(|t| t.id == old.task_id))
                    .ok_or(A::Refused("task-validation-task-missing"))?;
                let mut task = if validation.controls.as_ref().map(Vec::len) == r.policy.controls.as_ref().map(Vec::len)
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
/// Read the exact private original and scalar job, validate caller-purpose owner
/// or terminal state, Root identity, policy/slot/result mapping, every immutable
/// slot hash and known quiescence. Returns the current original Job or preserves
/// safe typed IO/SQL/codec/source/ownership failures. Reads only; receipt tuples
/// and equality do not grant execution, prove OS confinement or accept a task.
fn original_for(store: &RootStore, r: &Receipt, hash: &str, mode: OriginalUse) -> Result<Job, E> {
    let launch = |e| E::Launch(Box::new(e));
    let (saved, body_hash): (Receipt, String) =
        journal::read(store, &journal::name("job", &r.job.id))?;
    if body_hash != hash
        || crate::provider_configuration::configuration_digest(
            &serde_json::to_vec(r)
                .map_err(|e| launch(caused("task-validation-journal-encode", e)))?,
        ) != hash
        || !matches!(saved.schema_version, 1 | 2 | 3)
    {
        return Err(E::Refused("task-validation-original-receipt-changed"));
    }
    bound_slots(r).map_err(launch)?;
    let current = store::job(&store.db, &r.job.id)
        .map_err(launch)?
        .ok_or(E::Refused("task-validation-job-missing"))?;
    if !same_origin(&current, &r.job) {
        return Err(E::Refused("task-validation-job-source-changed"));
    }
    verify_origin(&store.db, &current, r).map_err(launch)?;
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

#[cfg(test)]
mod binding_tests {
    use super::*;
    use super::super::{journal::StepReceipt, store::Step};
    /// Explicit receipt-data fixture only: these supplied tuples grant nothing.
    /// Public native journeys separately exercise actual SQL/child permissions.
    fn receipt() -> Receipt {
        let job = Job {
            id: "job".into(), work_id: "work".into(), task_id: "task".into(), event_id: "event".into(),
            request_sha256: "a".repeat(64), source_sha256: "b".repeat(64), work_revision: 1,
            settings_revision: 1, owner_pid: 1, owner_stamp: "fixture".into(), owner_host: "fixture".into(),
            directory_device: "1".into(), directory_inode: "1".into(), control_count: 2,
            phase: JobPhase::Active, receipt_sha256: None, published_revision: None,
        };
        let controls = ["first", "second"].map(|id| crate::ValidationControl {
            id: id.into(), command: Some(vec!["python3".into(), "-c".into(), "print('fixture')".into()]),
            criteria: Some(vec![1]), justification: "Explicit source-binding fixture".into(),
            timeout_seconds: 1, ..Default::default()
        });
        let steps = controls.iter().enumerate().map(|(i,c)| (StepReceipt {
            schema_version: 2, job: job.clone(),
            step: Step { id: format!("step-{i}"), job_id: job.id.clone(), ordinal: i as i64,
                control_id: c.id.clone(), phase: StepPhase::Permitted, permission: 1,
                child_pid: Some(10+i as i64), parent_pid: Some(1), group_id: Some(10+i as i64),
                child_stamp: Some("fixture".into()), receipt_sha256: None },
            process_attempted: true, grant_attempted: true, channel_closed: true, waited: true,
            failed: false, exit_code: Some(0), signal: None, settings_revision: Some(crate::SettingCount(1)),
            result: Some(crate::ValidationControlResult { id: c.id.clone(), command: c.command.clone(),
                executed: true, passed: true, exit_code: 0, started_at: "fixture-start".into(),
                finished_at: "fixture-finish".into(), ..Default::default() }),
            retained: Some(Vec::new()), total: Some(0), truncated: Some(false), causes: Vec::new(), gate: None,
        }, "c".repeat(64))).collect();
        Receipt { schema_version: 1, job, source_origin: None, effective_policy: None, managed_candidate: None, steps, policy: crate::ValidationPolicy { mode: "automatic".into(),
            controls: Some(controls.into()), actor: "fixture".into(), authorized_at: "fixture".into() },
            artifacts: Default::default(), producer_agent_id: "worker".into(), attempt_id: "attempt".into(),
            producer_delivery_version: 1, finished_at: "fixture-finish".into(), result_event: "result".into() }
    }
    /// A complete valid witness precedes each corruption. Mapping errors must
    /// refuse public conversion, not become a plausible blocked/success artifact.
    #[test]
    fn native_validation_receipt_binds_policy_slot_and_result_before_success() {
        for fault in 0..10 {
            let mut r = receipt();
            assert!(all_passed(&r));
            assert!(public(&r).is_ok());
            match fault {
                0 => r.steps[0].0.result.as_mut().unwrap().id = "foreign".into(),
                1 => r.steps[0].0.result.as_mut().unwrap().command = Some(vec!["git".into()]),
                2 => r.steps[0].0.step.control_id = "foreign".into(),
                3 => r.steps[0].0.step.ordinal = 1,
                4 => r.steps[0].0.step.job_id = "other-job".into(),
                5 => r.steps[1].0.step.id = r.steps[0].0.step.id.clone(),
                6 => { r.policy.controls.as_mut().unwrap().pop(); },
                7 => r.steps[0].0.job.source_sha256 = "d".repeat(64),
                8 => r.steps[0].0.schema_version = 99,
                _ => r.schema_version = 99,
            }
            assert!(!all_passed(&r), "foreign receipt mapping accepted: {fault}");
            assert_eq!(public(&r).err().unwrap().code(), "task-validation-receipt-slot-source-changed");
            assert!(all_passed(&receipt()));
        }
    }
    /// Known failed/unattempted observations remain publishable blocked data;
    /// absence of permission or attempted-process proof can never yield success.
    #[test]
    fn native_validation_receipt_keeps_failed_and_unattempted_results_blocked() {
        let mut r = receipt();
        assert!(all_passed(&r));
        let s = &mut r.steps[0].0;
        s.failed = true;
        s.exit_code = Some(1);
        s.result.as_mut().unwrap().passed = false;
        s.result.as_mut().unwrap().exit_code = 1;
        assert!(!all_passed(&r));
        assert_eq!(public(&r).unwrap().0.state, "blocked");
        let mut r = receipt();
        assert!(all_passed(&r));
        let mut step = r.steps[1].0.step.clone();
        step.phase = StepPhase::Prepared;
        step.permission = 0;
        step.child_pid = None;
        step.parent_pid = None;
        step.group_id = None;
        step.child_stamp = None;
        r.steps[1].0 = super::super::journal::unattempted(&r.job, &step, None);
        assert!(!all_passed(&r));
        assert_eq!(public(&r).unwrap().0.state, "blocked");
        let mut missing = receipt();
        assert!(all_passed(&missing));
        missing.steps[0].0.result = None;
        assert!(!all_passed(&missing));
        assert_eq!(public(&missing).err().unwrap().code(), "task-validation-receipt-slot-source-changed");
        missing.steps[0].0.failed = true;
        assert_eq!(public(&missing).unwrap().0.state, "blocked");
        for flag in 0..3 {
            let mut r = receipt();
            assert!(all_passed(&r));
            match flag {
                0 => r.steps[0].0.process_attempted = false,
                1 => r.steps[0].0.step.permission = 0,
                _ => r.steps[0].0.step.phase = StepPhase::Registered,
            }
            assert!(!all_passed(&r));
        }
    }

    /// Version1 must preserve its omitted field; version2 cannot silently drop
    /// the mandatory new backing. DTO encoding is not a job or authority proof.
    #[test]
    fn native_validation_receipt_version_requires_explicit_origin_and_preserves_legacy() {
        let mut r = receipt();
        let bytes = serde_json::to_vec(&r).unwrap();
        assert!(!serde_json::from_slice::<serde_json::Value>(&bytes).unwrap().as_object().unwrap().contains_key("source_origin"));
        let decoded: Receipt = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
        assert!(all_passed(&decoded));
        r.schema_version = 2;
        assert!(public(&r).is_err());
        r.source_origin = Some(super::super::source_origin::Origin::new(&r.job.id, &"a".repeat(64), &r.policy, None).unwrap());
        assert!(public(&r).is_err());
        r.effective_policy = Some(r.policy.clone());
        assert!(public(&r).is_ok());
        r.effective_policy.as_mut().unwrap().controls.as_mut().unwrap()[0].timeout_seconds = 2;
        assert!(public(&r).is_err(), "public conversion accepted a changed effective-policy preimage");
        r.effective_policy = Some(r.policy.clone());
        assert!(public(&r).is_ok());
        r.schema_version = 1;
        assert!(public(&r).is_err());
    }
    /// Pure receipt-data counterexample: a valid ordinary v2 witness must not
    /// claim an opaque candidate binding without retaining its preimage. This
    /// creates no SQL job, kernel permission, execution or task acceptance.
    #[test]
    fn native_validation_receipt_refuses_ordinary_opaque_candidate_binding() {
        let mut r = receipt();
        r.schema_version = 2;
        r.effective_policy = Some(r.policy.clone());
        r.source_origin = Some(super::super::source_origin::Origin::new(
            &r.job.id, &"a".repeat(64), &r.policy, None).unwrap());
        assert!(public(&r).is_ok());
        r.source_origin = Some(super::super::source_origin::Origin::new(
            &r.job.id, &"a".repeat(64), &r.policy, Some(&"c".repeat(64))).unwrap());
        assert!(public(&r).is_err(), "ordinary receipt accepted an opaque unbacked candidate binding");
    }
    /// Explicit receipt-data fixture produced from the real per-task planner.
    /// No Root job, owned FS, native process, permission or acceptance is made.
    fn cumulative_receipt() -> Receipt {
        let mut r = receipt();
        let mut control = r.policy.controls.as_ref().unwrap()[0].clone();
        control.id = "shared".into();
        let original = crate::ValidationPolicy {
            mode: "automatic".into(), actor: "fixture".into(), authorized_at: "fixture".into(),
            controls: Some(vec![control]),
        };
        let task = |id: &str, status: &str| crate::Task {
            id: id.into(), status: status.into(), criteria: Some(vec!["criterion".into()]),
            validation_policy: Some(Box::new(original.clone())), ..Default::default()
        };
        let work = crate::Work { id: r.job.work_id.clone(),
            tasks: Some(vec![task("prior", "accepted"), task("task", "submitted")]), ..Default::default() };
        let mut limits = crate::ValidationPolicyLimits::legacy_go();
        limits.max_controls = 1; limits.max_total_timeout_seconds = 1;
        let plan = crate::managed_repository::managed_control_plan(&work, "task", &limits).unwrap();
        assert_eq!(plan.slots.len(), 2); // No aggregate re-normalization ceiling.
        let value = serde_json::json!({"schema_version":2,"work":r.job.work_id,"task":r.job.task_id,
            "agent":r.producer_agent_id,"attempt":r.attempt_id,"previous":"1".repeat(40),
            "result":"2".repeat(40),"contract":plan.contract,"control_plan":&plan,
            "commit":"3".repeat(40),"stage":"candidate-fixture","workspace":"/fixture/copy",
            "copy_identity":[1,2],"git_identity":[1,3],"workspace_identity":[1,4],"tracked":{}});
        r.schema_version = 3; r.effective_policy = None; r.policy = original;
        r.managed_candidate = Some(serde_json::from_value(value).unwrap());
        for ((step, _), slot) in r.steps.iter_mut().zip(&plan.slots) {
            step.step.control_id = slot.transport_id.clone();
            let result = step.result.as_mut().unwrap();
            result.id = slot.control.id.clone(); result.command = slot.control.command.clone();
        }
        rebind_cumulative(&mut r);
        r
    }
    /// Reseal supplied private data to test structural checks beyond stale hash
    /// rejection. No trusted row, grant or current Work is changed by this helper.
    fn rebind_cumulative(r: &mut Receipt) {
        let candidate = r.managed_candidate.as_ref().unwrap();
        let hash = candidate.binding_digest().unwrap();
        let mut value = serde_json::to_value(candidate).unwrap();
        value["binding_sha256"] = serde_json::json!(hash);
        r.managed_candidate = Some(serde_json::from_value(value).unwrap());
        let candidate = r.managed_candidate.as_ref().unwrap();
        let plan = candidate.control_plan.as_ref().unwrap();
        r.source_origin = Some(super::super::source_origin::Origin::new(
            &r.job.id, &"a".repeat(64), plan, Some(&hash)).unwrap());
        r.job.source_sha256 = super::super::source_origin::digest(r.source_origin.as_ref().unwrap()).unwrap();
        for (step, _) in &mut r.steps { step.job = r.job.clone(); }
    }
    /// Homonymous controls keep separate transport IDs. Public target results
    /// stay target-only, while an accepted predecessor failure blocks the whole
    /// candidate even when the target's own observed result passed.
    #[test]
    fn native_validation_receipt_cumulative_target_projection_and_prior_failure() {
        let mut r = cumulative_receipt();
        assert!(all_passed(&r));
        assert_ne!(r.steps[0].0.step.control_id, r.steps[1].0.step.control_id);
        let (validation, _, bytes) = public(&r).unwrap();
        assert_eq!(validation.candidate_sha, "3".repeat(40));
        assert_eq!(validation.controls.as_ref().unwrap().len(), 1);
        assert_eq!(validation.controls.as_ref().unwrap()[0].id, "shared");
        assert_eq!(validation.state, "pending_review");
        assert!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["failures"].as_array().unwrap().is_empty());
        let prior = &mut r.steps[0].0;
        prior.failed = true; prior.exit_code = Some(1);
        prior.result.as_mut().unwrap().passed = false;
        prior.result.as_mut().unwrap().exit_code = 1;
        assert!(!all_passed(&r));
        let (blocked, _, bytes) = public(&r).unwrap();
        assert_eq!(blocked.state, "blocked");
        assert!(blocked.controls.as_ref().unwrap()[0].passed);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["failures"][0]["task_id"], "prior");
        assert_eq!(value["failures"][0]["control_id"], "shared");
        assert_eq!(value["failures"][0]["transport_id"], r.steps[0].0.step.control_id);
    }
    /// A valid witness precedes every corruption. Some preimages are resealed
    /// deliberately: internal shape/version checks cannot rely on stale hashes.
    #[test]
    fn native_validation_receipt_cumulative_refuses_mixed_and_resealed_corruption() {
        for fault in 0..20 {
            let mut r = cumulative_receipt();
            assert!(public(&r).is_ok());
            match fault {
                0 => r.steps[0].0.step.control_id = "foreign".into(),
                1 => r.steps[0].0.result.as_mut().unwrap().id = "foreign".into(),
                2 => r.steps[0].0.step.ordinal = 1,
                3 => { r.managed_candidate.as_mut().unwrap().work = "other".into(); rebind_cumulative(&mut r); },
                4 => r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap().slots[0].control.command = Some(vec!["different".into()]),
                5 => r.source_origin = Some(super::super::source_origin::Origin::new(
                    &r.job.id, &"a".repeat(64), &r.policy,
                    Some(&r.managed_candidate.as_ref().unwrap().binding_digest().unwrap())).unwrap()),
                6 => r.source_origin = Some(super::super::source_origin::Origin::new(
                    &r.job.id, &"a".repeat(64), r.managed_candidate.as_ref().unwrap().control_plan.as_ref().unwrap(), Some(&"c".repeat(64))).unwrap()),
                7 => r.effective_policy = Some(r.policy.clone()),
                8 => r.schema_version = 2,
                9 => r.managed_candidate = None,
                10 => r.source_origin = None,
                11 => {r.schema_version = 2; r.effective_policy = Some(r.policy.clone()); r.managed_candidate = None;},
                12 => {r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap().policies.remove(0); rebind_cumulative(&mut r);},
                13 => {r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap().slots.swap(0,1); rebind_cumulative(&mut r);},
                14 => {r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap().policies[0].task_id = "task".into(); rebind_cumulative(&mut r);},
                15 => {
                    let plan = r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap();
                    plan.policies[0].normalized.controls.as_mut().unwrap()[0].command = Some(vec!["different".into()]);
                    plan.slots[0].control.command = Some(vec!["different".into()]); rebind_cumulative(&mut r);
                },
                16 => {
                    let plan = r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap();
                    plan.policies[0].normalized.controls.as_mut().unwrap()[0].timeout_seconds = 0;
                    plan.slots[0].control.timeout_seconds = 0; rebind_cumulative(&mut r);
                },
                17 => {r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap().policies[1].original.actor = "different".into(); rebind_cumulative(&mut r);},
                18 => {r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap().policies[0].normalized.actor = "different".into(); rebind_cumulative(&mut r);},
                _ => {r.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap().policies[0].original.actor.clear(); rebind_cumulative(&mut r);},
            }
            assert!(public(&r).is_err(), "cumulative corruption accepted: {fault}");
            assert!(!all_passed(&r));
            assert!(public(&cumulative_receipt()).is_ok());
        }
    }
    /// The actual source-origin SQL reader rejects a coherently resealed partial
    /// cohort against the recorded full origin. This minimal SQL shape fixture
    /// is not a real native Job, permission, Root acceptance or current Work.
    #[test]
    fn native_validation_receipt_cumulative_sql_origin_refuses_partial_rebinding() {
        let r = cumulative_receipt();
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE native_validation_jobs(id TEXT PRIMARY KEY); INSERT INTO native_validation_jobs VALUES('job');").unwrap();
        let tx = db.transaction().unwrap();
        super::super::source_origin::stage(&tx, r.source_origin.as_ref().unwrap()).unwrap();
        tx.commit().unwrap();
        verify_origin(&db, &r.job, &r).unwrap();
        assert_eq!(super::super::source_origin::source(&db, &r.job.id, &"a".repeat(64))
            .err().unwrap().code(), "native-validation-origin-candidate-mismatch");
        let before = super::super::source_origin::read(&db, &r.job.id).unwrap();
        let mut partial = cumulative_receipt();
        let plan = partial.managed_candidate.as_mut().unwrap().control_plan.as_mut().unwrap();
        plan.policies.remove(0); plan.slots.remove(0); plan.slots[0].ordinal = 0;
        partial.steps.remove(0);partial.steps[0].0.step.ordinal = 0;partial.job.control_count = 1;
        rebind_cumulative(&mut partial);
        // Coherent data alone cannot establish full current Work membership.
        assert!(all_passed(&partial));
        assert_eq!(verify_origin(&db, &r.job, &partial).err().unwrap().code(), "native-validation-origin-receipt-mismatch");
        assert_eq!(super::super::source_origin::read(&db, &r.job.id).unwrap(), before);
        verify_origin(&db, &r.job, &r).unwrap();
        let plan = r.managed_candidate.as_ref().unwrap().control_plan.as_ref().unwrap();
        assert_eq!(super::super::source_origin::verify_job_receipt(&db, &r.job, r.source_origin.as_ref(), Some(plan))
            .err().unwrap().code(), "native-validation-origin-candidate-mismatch");
        let bytes = serde_json::to_vec(&r).unwrap();
        let decoded: Receipt = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
        verify_origin(&db, &r.job, &decoded).unwrap();
    }
    /// Actual producers require executable argv. A matching empty policy/result
    /// pair must never become success merely because both malformed values agree.
    /// Pure data fixture only, with a valid witness before every invalid variant.
    #[test]
    fn native_validation_receipt_refuses_matching_empty_or_invalid_argv() {
        for bad in [None, Some(vec![]), Some(vec![String::new()]), Some(vec!["python3".into(), "nul\0argument".into()])] {
            let mut r = receipt();
            assert!(public(&r).is_ok());
            r.policy.controls.as_mut().unwrap()[0].command = bad.clone();
            r.steps[0].0.result.as_mut().unwrap().command = bad;
            assert!(public(&r).is_err(), "matching malformed argv became success");
            assert!(!all_passed(&r));
        }
    }

}
