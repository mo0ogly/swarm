//! Explicit abandonment of original observations, never execution or acceptance.
use super::{
    journal, publication,
    runtime::TaskValidationOutcome,
    store::{self, JobPhase, StepPhase},
};
use crate::{
    root_store::{RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use serde::{Deserialize, Serialize};

/// Durable intent binds the original complete receipt and a generated event.
/// Time is a local observation, not human identity or current result validity.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Retirement {
    schema_version: i64,
    job_id: String,
    original_sha256: String,
    event_id: String,
    at: String,
}
/// Retire only the exact original event. Readback/replay never invokes a process;
/// an existing complete original needs no current source because no result is
/// adopted. Rebuilding an absent combined body requires matching current source
/// and every known slot original. A live
/// owner or unknown original process outcome remains exclusive. Original receipts
/// precede atomic terminal rows and Work audit. Every SQL/file/close error keeps
/// its typed cause; known commit metadata survives saved-settings release failure.
pub(super) fn run(
    store: &mut RootStore,
    work: &str,
    raw: &[u8],
) -> Result<TaskValidationOutcome, E> {
    let launch = |e| E::Launch(Box::new(e));
    let request = super::decode(raw).map_err(launch)?;
    let intent = serde_json::to_vec(&request)
        .map_err(|e| launch(A::caused("task-validation-request-encode", e)))?;
    let job = store::event(&store.db, &request.event_id)
        .map_err(launch)?
        .ok_or(E::Refused("task-validation-job-missing"))?;
    if job.work_id != work
        || job.task_id != request.task_id
        || job.request_sha256 != crate::provider_configuration::configuration_digest(&intent)
    {
        return Err(E::Refused("task-validation-replay-conflict"));
    }
    let (receipt, hash) = super::reconstruction::read(store, &job)?;
    let current = publication::original(store, &receipt, &hash, true)?;
    let name = journal::name("retirement", &job.id);
    if current.phase == JobPhase::Refused {
        let (retired, _): (Retirement, String) = journal::read(store, &name)?;
        if retired.schema_version != 1
            || retired.job_id != job.id
            || retired.original_sha256 != hash
            || current.receipt_sha256.as_deref() != Some(&hash)
        {
            return Err(E::Refused("task-validation-retirement-changed"));
        }
        for (slot, expected_hash) in &receipt.steps {
            let (_, actual) = store::step(&store.db, &slot.step.id).map_err(launch)?;
            let mut expected = slot.step.clone();
            expected.phase = StepPhase::Refused;
            expected.receipt_sha256 = Some(expected_hash.clone());
            if actual != expected {
                return Err(E::Refused("task-validation-retirement-changed"));
            }
        }
        return Ok(TaskValidationOutcome {
            schema_version: 1,
            job_id: job.id,
            controls_passed: false,
            task_accepted: false,
            retired: true,
            replayed: true,
            work: store.read_work(work)?,
        });
    }
    if !matches!(current.phase, JobPhase::Active | JobPhase::Unresolved) {
        return Err(E::Refused("task-validation-already-published"));
    }
    // A retained intent permits retry after a known/uncertain publication failure;
    // absence alone permits create, never replacement of a different receipt.
    let retired: Retirement = match store.directory.read_native_receipt(&name) {
        Ok(raw) => serde_json::from_slice(&raw)
            .map_err(|e| launch(A::caused("task-validation-retirement-decode", e)))?,
        Err(e) if super::super::absent(&e) => {
            let retired = Retirement {
                schema_version: 1,
                job_id: job.id.clone(),
                original_sha256: hash.clone(),
                event_id: crate::generate_identifier("validation-retired-").map_err(E::Runtime)?,
                at: crate::system_timestamp().map_err(E::Runtime)?,
            };
            journal::persist(store, &name, &retired)?;
            retired
        }
        Err(e) => return Err(E::Storage(e)),
    };
    if retired.schema_version != 1
        || retired.job_id != job.id
        || retired.original_sha256 != hash
        || !swarm_identity_contract::name(retired.event_id.as_bytes())
        || swarm_wire_json::go_rfc3339::parse(&retired.at).is_err()
    {
        return Err(E::Refused("task-validation-retirement-changed"));
    }
    let mut known = None;
    let result =
        crate::agent_storage_retry::execute(&mut store.db, &store.directory, false, |db, _| {
            let committed = crate::agent_execution_store::transaction(db, |tx| {
                let actual =
                    store::job(tx, &job.id)?.ok_or(A::Refused("task-validation-job-missing"))?;
                if actual != current {
                    return Err(A::Refused("task-validation-publication-state-changed"));
                }
                publication::departed(&actual)?;
                let mut w = crate::read_recorded_work(tx, work)
                    .map_err(|e| A::caused("task-validation-work", e))?;
                let task = w
                    .tasks
                    .as_mut()
                    .and_then(|v| v.iter_mut().find(|t| t.id == actual.task_id))
                    .ok_or(A::Refused("task-validation-task-missing"))?;
                task.evidence_stale_reason = "validation-observations-retired".into();
                let revision = w.revision;
                w.revision = revision
                    .checked_add(1)
                    .ok_or(A::Refused("task-validation-revision-overflow"))?;
                w.updated = retired.at.clone();
                let event = crate::encode_model(&retired)
                    .map_err(|e| A::caused("task-validation-retirement-encode", e))?;
                let staged = crate::stage_recorded_change_with_hook(
                    tx,
                    crate::RecordedChange {
                        work: &w,
                        expected_revision: revision,
                        event: &retired.event_id,
                        kind: "task.validation-retired",
                        request: &event,
                    },
                    |db, _| {
                        for (s, h) in &receipt.steps {
                            let (_, before) = store::step(db, &s.step.id)?;
                            if before != s.step {
                                return Err(Box::new(A::Refused(
                                    "task-validation-slot-state-changed",
                                ))
                                    as Box<dyn std::error::Error + Send + Sync>);
                            }
                            let mut terminal = before.clone();
                            terminal.phase = StepPhase::Refused;
                            terminal.receipt_sha256 = Some(h.clone());
                            store::change_step(db, &before, &terminal)?;
                        }
                        let mut terminal = actual.clone();
                        terminal.phase = JobPhase::Refused;
                        terminal.receipt_sha256 = Some(hash.clone());
                        terminal.published_revision = Some(w.revision);
                        store::change_job(db, &actual, &terminal)?;
                        Ok(())
                    },
                )
                .map_err(|e| A::caused("task-validation-retirement-publication", e))?;
                if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                    return Err(A::Refused("task-validation-result-event-conflict"));
                }
                Ok(TaskValidationOutcome {
                    schema_version: 1,
                    job_id: actual.id,
                    controls_passed: false,
                    task_accepted: false,
                    retired: true,
                    replayed: false,
                    work: w,
                })
            });
            if let Ok(value) = &committed {
                known = Some((value.work.id.clone(), value.work.revision));
            }
            committed.map_err(launch)
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
