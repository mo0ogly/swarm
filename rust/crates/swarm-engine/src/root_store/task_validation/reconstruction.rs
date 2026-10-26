//! Recover only a missing combined receipt from complete, actual slot originals.
//! This assembles observations; it never invokes a command or infers an owned wait.
use super::{journal, publication, runtime::Receipt, store::{self, Job, JobPhase}, Request};
use crate::{root_store::{finished, RootStore, RootStoreError as E}, AgentLaunchError as A};

/// Preserve an existing combined original exactly. Only confirmed file absence
/// permits reconstruction; malformed/unsafe/unreadable originals never downgrade.
/// Recovery requires the recorded owner to have departed and every slot original
/// to be present with known quiescence and exact source/SQL backing. No historical
/// origin is fabricated; current-source drift refuses conservatively.
pub(super) fn read(store: &RootStore, job: &Job) -> Result<(Receipt, String), E> {
    match journal::read(store, &journal::name("job", &job.id)) {
        Ok(value) => Ok(value),
        Err(E::Storage(error)) if crate::root_store::absent(&error) => assemble(store, job),
        Err(error) => Err(error),
    }
}

/// Retain candidate custody outside read/close wrappers while inspecting all
/// actual observations. Full Git inspection is outside Root locks. A recoverable
/// sidecar precedes create-only combined publication; either write may remain
/// visible after an error, without a SQL result or implicit command retry.
fn assemble(store: &RootStore, job: &Job) -> Result<(Receipt, String), E> {
    publication::departed(job).map_err(launch)?;
    if !matches!(job.phase, JobPhase::Active | JobPhase::Unresolved) {
        return Err(E::Refused("task-validation-reconstruction-phase"));
    }
    let managed = super::managed_execution::for_helper(store, job)?;
    let result = (|| {
        let reservation = store.directory.reserve(false, false).map_err(E::Storage)?;
        let observed = (|| {
            let settings = store.read_settings()?;
            let tx = store.db.unchecked_transaction().map_err(E::Sql)?;
            let observed = inspect(store, &tx, &settings, job, managed.as_ref());
            finished(observed, tx.rollback().map_err(E::Sql))
        })();
        let mut receipt = finished(observed, reservation.close().map_err(E::Storage))?;
        // A previous interrupted assembly has its own immutable original. Reuse
        // its event/time only after exact comparison with freshly checked data.
        let name = journal::name("reconstruction", &job.id);
        match journal::read::<Receipt>(store, &name) {
            Ok((prior, _)) => {
                if !swarm_identity_contract::name(prior.result_event.as_bytes())
                    || swarm_wire_json::go_rfc3339::parse(&prior.finished_at).is_err() {
                    return Err(E::Refused("task-validation-reconstruction-changed"));
                }
                receipt.result_event.clone_from(&prior.result_event);
                receipt.finished_at.clone_from(&prior.finished_at);
                if serde_json::to_vec(&receipt).map_err(codec)? != serde_json::to_vec(&prior).map_err(codec)? {
                    return Err(E::Refused("task-validation-reconstruction-changed"));
                }
            }
            Err(E::Storage(error)) if crate::root_store::absent(&error) => {
                journal::persist(store, &name, &receipt)?;
            }
            Err(error) => return Err(error),
        }
        // Current source was checked, not granted: subsequent apply/retire must
        // still validate the actual original/owner/rows under their own writer.
        let hash = journal::persist(store, &journal::name("job", &job.id), &receipt)?;
        Ok((receipt, hash))
    })();
    finished(result, managed.map_or(Ok(()), |session| session.close()))
}

/// Reconstruct plain receipt data under one actual SQL/settings snapshot. Require
/// an existing source origin and exact current preimage; every stored slot must
/// have its canonical original, current row equality and confirmed owned wait (or
/// original pristine non-invocation). Missing/unknown observations refuse before
/// any file or SQL write. The timestamp is assembly time, not child exit time.
fn inspect(store: &RootStore, db: &rusqlite::Connection, settings: &crate::EngineSettingsDocument,
    job: &Job, managed: Option<&super::managed_execution::Session>) -> Result<Receipt, E> {
    if store::job(db, &job.id).map_err(launch)?.as_ref() != Some(job) {
        return Err(E::Refused("task-validation-reconstruction-job-changed"));
    }
    publication::departed(job).map_err(launch)?;
    let identity = store.directory.identity().map_err(E::Storage)?;
    if identity.0.to_string() != job.directory_device || identity.1.to_string() != job.directory_inode {
        return Err(E::Refused("task-validation-root-changed"));
    }
    let work = crate::read_recorded_work(db, &job.work_id).map_err(E::Work)?;
    let request = Request {schema_version:1, event_id:job.event_id.clone(),
        expected_revision:work.revision, task_id:job.task_id.clone()};
    let prepared = match managed {
        Some(session) => super::managed_execution::prepare(db, &store.directory, &store.root,
            settings, &job.work_id, &request, &session.data),
        None => super::prepare(db, &store.root, settings, &job.work_id, &request),
    }.map_err(launch)?;
    let origin = super::source_origin::read(db, &job.id).map_err(launch)?
        .ok_or(E::Refused("task-validation-reconstruction-origin-required"))?;
    if prepared.origin(&job.id).map_err(launch)? != origin
        || super::managed_execution::bound_source(db,job,&prepared.preview.source_sha256,
            prepared.managed.as_ref().map(|m| &m.candidate)).map_err(launch)? != job.source_sha256 {
        return Err(E::Refused("task-validation-reconstruction-source-changed"));
    }
    let mut steps = Vec::new();
    for step in store::ordered_steps(db,job).map_err(launch)? {
        let (slot, hash): (journal::StepReceipt,String) = journal::read(store,&journal::name("step",&step.id))?;
        if slot.job != *job || slot.step != step
            || crate::provider_configuration::configuration_digest(&serde_json::to_vec(&slot).map_err(codec)?) != hash {
            return Err(E::Refused("task-validation-reconstruction-slot-changed"));
        }
        journal::quiescent(&slot).map_err(launch)?;
        steps.push((slot,hash));
    }
    let candidate = prepared.managed.as_ref().map(|m| m.candidate.clone());
    let receipt = Receipt {schema_version:if candidate.is_some(){3}else{2},job:job.clone(),
        source_origin:Some(origin),effective_policy:if candidate.is_none(){Some(prepared.policy)}else{None},
        managed_candidate:candidate,steps,policy:prepared.original_policy,artifacts:prepared.preview.artifacts,
        producer_agent_id:prepared.preview.producer_agent_id,attempt_id:prepared.preview.attempt_id,
        producer_delivery_version:prepared.producer_delivery_version,
        finished_at:crate::system_timestamp().map_err(E::Runtime)?,
        result_event:crate::generate_identifier("validation-reconstructed-").map_err(E::Runtime)?};
    publication::verify_origin(db,job,&receipt).map_err(launch)?;
    Ok(receipt)
}

/// Keep the existing typed launch boundary; no safe diagnostic replaces a cause.
fn launch(error: A) -> E { E::Launch(Box::new(error)) }
/// Preserve serialization failures from exact canonical body comparisons.
fn codec(error: serde_json::Error) -> E { launch(A::caused("task-validation-reconstruction-encode",error)) }
