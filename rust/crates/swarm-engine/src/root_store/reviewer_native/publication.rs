//! Original reviewer result publication; no provider call or task acceptance.
use super::{
    guard, journal,
    store::{self, Job, Phase},
    transaction::{self, Change},
};
use crate::{
    root_store::{RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use rusqlite::Transaction;
use serde::Serialize;

/// Historical recording acknowledgement, never fresh acceptance or invocation.
/// Work is current at the returned transaction; replay describes an original
/// event, not a timeless verdict on current task/source bytes.
#[derive(Serialize)]
pub(super) struct Outcome {
    pub schema_version: i64,
    pub review_id: String,
    pub review_recorded: bool,
    /// A recorded verdict versus explicit failed/stale non-verdict, never acceptance.
    pub verdict_applicable: bool,
    pub task_accepted: bool,
    pub replayed: bool,
    pub work: crate::Work,
}
/// Private prepared observations; no Debug/private text leakage or new authority.
pub(super) struct Ready {
    pub(super) receipt: journal::Receipt,
    pub(super) hash: String,
    pub(super) work: crate::Work,
    pub(super) review: crate::IndependentReview,
}
/// Preserve original typed cause under a static publication diagnostic family.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
/// Compare original scalar bindings, ignoring only ledger publication fields.
/// This is equality, not a transfer of owner/permission or evidence freshness.
fn same_origin(job: &Job, original: &Job) -> bool {
    let mut value = original.clone();
    value.phase = job.phase;
    value.receipt_sha256.clone_from(&job.receipt_sha256);
    value.published_revision = job.published_revision;
    value == *job
}
/// Require the selected actual owner boundary and absence of its waited child on the
/// observed host. PID reuse is absence of this tuple, never a signal right over
/// the new process. IO/observation uncertainty refuses; no process is stopped.
pub(super) fn quiescent(job: &Job, owner: OwnerBoundary) -> Result<(), E> {
    match owner {
        OwnerBoundary::CurrentOriginal => guard::owner(job)?,
        OwnerBoundary::ConfirmedDeparted => guard::departed(job)?,
    }
    let pid = u32::try_from(
        job.child_pid
            .ok_or(E::Refused("review-publication-child-required"))?,
    )
    .map_err(|e| caused("review-publication-child-pid", e))?;
    match crate::observe_linux_process(pid) {
        Ok(p) if Some(p.process_stamp()) != job.child_stamp.as_deref() => Ok(()),
        Ok(_) => Err(E::Refused("review-publication-child-still-present")),
        Err(e) if e.confirms_absence() => Ok(()),
        Err(e) => Err(caused("review-publication-child-observation", e)),
    }
}
/// Decode the anchored original and compare it with actual current ledger,
/// saved policy, Work and complete source originals. Failed/legacy/incomplete
/// receipts refuse instead of fabricating terminal events or zero usage. The
/// original admission TTL is deliberately not reused as result expiry. Does no
/// mutation, host probe, provider call, signal, file publication or settlement.
#[cfg(test)]
fn prepare(
    root: &RootStore,
    tx: &Transaction<'_>,
    settings: &crate::EngineSettingsDocument,
    id: &str,
    owner: OwnerBoundary,
) -> Result<Ready, E> {
    prepare_using(root,tx,settings,id,owner,None)
}
/// Revalidate a waited original against current complete inputs under borrowed
/// candidate custody. All ledger/policy/owner/source checks remain mandatory.
fn prepare_using(root:&RootStore,tx:&Transaction<'_>,settings:&crate::EngineSettingsDocument,id:&str,owner:OwnerBoundary,source:Option<&crate::root_store::task_validation::ReviewCandidate>)->Result<Ready,E>{
    let (r, hash) = journal::read(root, id)?;
    let job = store::job(tx, id)
        .map_err(|e| E::Launch(Box::new(e)))?
        .ok_or(E::Refused("review-publication-job-required"))?;
    if job != r.job
        || job.phase != Phase::Permitted
        || job.permission != 1
        || r.schema_version != 2
        || r.failed
        || !r.output_complete
    {
        return Err(E::Refused("review-publication-original-not-applicable"));
    }
    quiescent(&job, owner)?;
    let raw = r
        .raw_capture
        .as_ref()
        .ok_or(E::Refused("review-publication-capture-required"))?;
    let policy = settings
        .values
        .agent_execution_policy()
        .map_err(E::Settings)?;
    if raw.failed || raw.policy != policy {
        return Err(E::Refused("review-publication-capture-policy-changed"));
    }
    let (work, inputs) = guard::held_inputs(root, tx, settings.clone(), &job,source,true)?;
    let originals: Vec<_> = inputs
        .materials
        .iter()
        .map(|s| crate::ReviewQuoteOriginal {
            name: &s.name,
            kind: &s.quote_kind,
            content: &s.content,
        })
        .collect();
    let reply = r
        .original_reply
        .as_deref()
        .ok_or(E::Refused("review-publication-reply-required"))?;
    let mut review = work
        .tasks
        .iter()
        .flatten()
        .find(|t| t.id == job.task_id)
        .and_then(|t| t.independent_review.as_deref())
        .cloned()
        .ok_or(E::Refused("review-publication-review-required"))?;
    let parse = |s: &str| {
        swarm_wire_json::go_rfc3339::parse(s).map_err(|e| caused("review-publication-clock", e))
    };
    if parse(&r.finished_at)? < parse(&review.started)? {
        return Err(E::Refused("review-publication-clock-order"));
    }
    match (&inputs.managed_protocol,&r.managed_review) {
        (Some(_),Some(original))=>{
            // Reconstruct the same typed model before comparing canonical bytes:
            // JSONValue sorts object keys differently from struct field order.
            let current=crate::managed_review_record::Snapshot::capture(&inputs,&settings.values)
                .map_err(|e|E::Launch(Box::new(e)))?.ok_or(E::Refused("review-managed-original-required"))?;
            if !original.same_inputs(&current).map_err(|e|E::Launch(Box::new(e)))? || original.prompt_sha256!=job.prompt_sha256 {
                return Err(E::Refused("review-managed-original-context-changed"));
            }
            current.apply(reply,&mut review,&job.task_id).map_err(|e|E::Launch(Box::new(e)))?;
            if !original.matches(reply,&review,&job.task_id).map_err(|e|E::Launch(Box::new(e)))? {return Err(E::Refused("review-managed-original-reply-changed"));}
            let (name,bytes)=original.context().map_err(|e|E::Launch(Box::new(e)))?;
            public_file(root,&name,&bytes)?;
        }
        (None,None)=>{
    let decoded = crate::review_output::decode_retained_review(
        reply,
        &settings.values,
        inputs.preview.criterion_count,
        &originals,
    )
    .map_err(|e| E::Launch(Box::new(e)))?;
    review.state = match decoded.state {
        crate::ReviewReplyState::Passed => "passed",
        crate::ReviewReplyState::ChangesRequested => "changes_requested",
    }.into();
    review.reason=decoded.reason;review.criteria=decoded.criteria;
        }
        _=>return Err(E::Refused("review-managed-original-domain-mismatch")),
    }
    review.finished = r.finished_at.clone();
    review.result_event = format!("{}-result", job.id);
    review.reply_sha256 = crate::provider_configuration::configuration_digest(reply);
    // Original provider facts remain private/unverified. Do not project a cost,
    // clear the monetary reservation or pretend the original call was free.
    Ok(Ready {
        receipt: r,
        hash,
        work,
        review,
    })
}
/// Observe the exact original through the common saved storage policy and an
/// explicit rollback-only transaction. This candidate preflight is not an API
/// permit/lease and creates no file, result, budget mutation or business event.
#[cfg(test)]
pub(super) fn inspect(root: &RootStore, id: &str) -> Result<(), E> {
    crate::agent_storage_retry::execute_borrowed(&root.db, &root.directory, false, |_, settings| {
        transaction::apply(root, |tx| {
            prepare(root, tx, settings, id, OwnerBoundary::CurrentOriginal)
                .map(|_| Change::Observe(()))
        })
    })
}
/// Publish/sync one exact public data artifact create-only; identical existing
/// bytes permit recovery after SQL refusal, while mismatch/IO/close refuses.
/// Private host/PID/raw/provider facts never enter this public certificate.
fn public_file(root: &RootStore, name: &str, bytes: &[u8]) -> Result<(), E> {
    match root.directory.read_bounded(name, bytes.len()) {
        Ok(old) if old == bytes => root
            .directory
            .synchronize_existing(name)
            .map_err(E::Storage),
        Ok(_) => Err(E::Refused("review-publication-public-artifact-changed")),
        Err(e) if crate::root_store::absent(&e) => root
            .directory
            .publish_new_with_fault(name, bytes, crate::local_storage::AtomicStorageFault::None)
            .map_err(E::Storage),
        Err(e) => Err(E::Storage(e)),
    }
}
/// Stage a verdict or explicit failed/stale non-verdict and one scoped inbox event. A favorable review keeps
/// the task submitted; changes_requested blocks it for correction, as the Go
/// source contract does. A closed scope is not reopened. Conflicting inbox IDs,
/// missing ownership or invalid state refuse before assigning the candidate.
/// No task acceptance, ancestor delivery, scheduling or paid call is created.
fn apply_candidate(ready: &mut Ready) -> Result<(), E> {
    crate::validate_planning_state(&ready.work)
        .map_err(|e| caused("review-publication-planning", e))?;
    let task = ready
        .work
        .tasks
        .iter_mut()
        .flatten()
        .find(|t| t.id == ready.receipt.job.task_id)
        .ok_or(E::Refused("review-publication-task-required"))?;
    task.independent_review = Some(Box::new(ready.review.clone()));
    if ready.review.state == "changes_requested" {
        task.status = "blocked".into();
        task.blocker = format!("Vérificateur indépendant : {}", ready.review.reason);
        task.next = format!(
            "Corriger les critères signalés puis remettre un nouveau rapport. {}",
            ready.review.reason
        );
    }
    let event = crate::PlanningEvent {
        id: ready.review.id.clone(),
        scope: task.scope_id.clone(),
        kind: "independent_review".into(),
        task: task.id.clone(),
        attempt: ready.review.attempt.clone(),
        message: format!("{} : {}", ready.review.state, ready.review.reason),
        artifacts: if matches!(ready.review.state.as_str(), "error" | "stale") {
            None
        } else {
            Some(vec![crate::ExchangeArtifact {
                path: ready.review.report.clone(),
                sha256: ready.review.sha256.clone(),
            }])
        },
        at: ready.receipt.finished_at.clone(),
        ..Default::default()
    };
    let planning = ready
        .work
        .planning
        .as_mut()
        .ok_or(E::Refused("review-publication-planning-required"))?;
    if let Some(old) = planning.inbox.iter().flatten().find(|e| e.id == event.id) {
        if old != &event {
            return Err(E::Refused("review-publication-inbox-conflict"));
        }
    } else {
        planning.inbox.get_or_insert_default().push(event);
    }
    crate::validate_planning_state(&ready.work)
        .map_err(|e| caused("review-publication-planning", e))?;
    Ok(())
}
/// Recheck a published public certificate against the actual immutable result
/// event and ledger revision, not mutable current task fields. Hash/origin/shape
/// failures refuse historical acknowledgement. No source freshness or new permit
/// is inferred; current files may legitimately differ from a past review.
/// Exact compact certificate extent is derived before reading, without a policy cap.
fn recorded_artifact(
    root: &RootStore,
    tx: &Transaction<'_>,
    job: &Job,
    r: &journal::Receipt,
) -> Result<(), E> {
    let event = format!("{}-result", job.id);
    let (kind, revision, request): (String, i64, Vec<u8>) = tx
        .query_row(
            "SELECT kind,revision,request FROM events WHERE id=? AND work_id=?",
            rusqlite::params![event, job.work_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| caused("review-publication-recorded-event", e))?;
    if kind != "review.result" || Some(revision) != job.published_revision {
        return Err(E::Refused("review-publication-recorded-event-changed"));
    }
    let record: serde_json::Value = serde_json::from_slice(&request)
        .map_err(|e| caused("review-publication-recorded-event-json", e))?;
    if let Some(managed)=&r.managed_review {
        let typed:crate::IndependentReview=serde_json::from_slice(&request).map_err(|e|caused("review-managed-recorded-event-json",e))?;
        if job.phase==Phase::Recorded {
            let raw=r.original_reply.as_deref().ok_or(E::Refused("review-managed-original-reply-required"))?;
            if !managed.matches(raw,&typed,&job.task_id).map_err(|e|E::Launch(Box::new(e)))? {return Err(E::Refused("review-managed-recorded-event-changed"));}
            let (name,bytes)=managed.context().map_err(|e|E::Launch(Box::new(e)))?;
            let actual=root.directory.read_bounded(&name,bytes.len()).map_err(E::Storage)?;
            if actual!=bytes{return Err(E::Refused("review-managed-recorded-context-changed"));}
        }
    }

    // Empty omitted digest means unknown original reply, never malformed data.
    let recorded_digest = match record.get("reply_sha256") {
        None => "",
        Some(serde_json::Value::String(value)) => value.as_str(),
        Some(_) => return Err(E::Refused("review-publication-recorded-reply-digest-shape")),
    };
    let original_digest = r
        .original_reply
        .as_deref()
        .map(crate::provider_configuration::configuration_digest)
        .unwrap_or_default();
    if (job.phase == Phase::Refused
        && ((r.failed && record["state"] != "error")
            || (!r.failed
                && (!r.output_complete || r.exit_code != Some(0) || record["state"] != "stale"))))
        || (job.phase == Phase::Recorded
            && (r.failed
                || !r.output_complete
                || !matches!(
                    record["state"].as_str(),
                    Some("passed" | "changes_requested")
                )))
        || record["id"] != job.id
        || record["attempt"] != job.attempt_id
        || record["producer"] != job.producer_id
        || record["contract"] != job.contract_sha256
        || record["result_event"] != event
        || record["finished"] != r.finished_at
        || recorded_digest != original_digest
    {
        return Err(E::Refused("review-publication-recorded-binding-changed"));
    }
    let expected = crate::root_store::native_review_context::expected_certificate(
        record,
        &job.work_id,
        &job.task_id,
        &job.source_sha256,
        &job.validation_receipt_sha256,
    )?;
    let bytes = root
        .directory
        .read_bounded(&expected.file_name, expected.byte_len)
        .map_err(E::Storage)?;
    if crate::provider_configuration::configuration_digest(&bytes) != expected.sha256 {
        return Err(E::Refused("review-publication-recorded-public-changed"));
    }
    let certificate: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| caused("review-publication-recorded-public-json", e))?;
    if certificate != expected.document {
        return Err(E::Refused(
            "review-publication-recorded-certificate-changed",
        ));
    }
    Ok(())
}
/// Commit original result/Work/inbox/event/ledger atomically under the common
/// saved storage retry policy. File publication may survive SQL rollback and is
/// reused only byte-identically. Recheck current sources after artifact IO,
/// without treating observation as a filesystem lease. Exact recorded replay
/// checks original hash/origin and changes nothing; it is no fresh acceptance.
/// This entry requires the current original owner. Explicit departed recovery
/// uses the same result path under its own OS proof boundary. Stale-success
/// handling remains unfinished. Failed originals use reconcile_failed.
/// Spending stays reserved,
/// declared provider facts stay unverified. Commit/release uncertainty retains
/// original causes and known Work revision; no provider or grant is retried.
pub(super) fn record(root: &RootStore, id: &str) -> Result<Outcome, E> {
    record_mode(
        root,
        id,
        Mode::Verdict,
        OwnerBoundary::CurrentOriginal,
        crate::root_store::native_review_context::CertificateFormat::ContentV2,
    )
}
/// Finalize a waited failed original as a non-verdict, preserving task status,
/// original causes and held estimate. Same atomic/file/replay guarantees as
/// record; never adopts an owner, repeats a provider, accepts or refunds a task.
pub(super) fn reconcile_failed(root: &RootStore, id: &str) -> Result<Outcome, E> {
    record_mode(
        root,
        id,
        Mode::FailedOriginal,
        OwnerBoundary::CurrentOriginal,
        crate::root_store::native_review_context::CertificateFormat::ContentV2,
    )
}
/// Recover only a retained known-wait original after confirmed owner departure
/// on the original host/boot/PID namespace. The original outcome selects verdict
/// versus failed-call preparation; both still require absent child and exact
/// ledger/journal bindings. No PID, owner, permission or spending is rewritten.
/// Fresh complete inputs remain mandatory for a verdict. Historical terminal
/// replay reads its original result and grants no fresh absence/action right.
/// Missing/legacy/unknown wait, live/foreign owner or uncertain observations
/// refuse. No process, provider, authenticated agent or task acceptance is created.
#[cfg(test)]
pub(super) fn reconcile_departed(root: &RootStore, id: &str) -> Result<Outcome, E> {
    if !root.writable {
        return Err(E::Refused("root-read-only"));
    }
    let (original, _) = journal::read(root, id)?;
    let mode = if original.failed {
        Mode::FailedOriginal
    } else {
        Mode::Verdict
    };
    record_mode(
        root,
        id,
        mode,
        OwnerBoundary::ConfirmedDeparted,
        crate::root_store::native_review_context::CertificateFormat::ContentV2,
    )
}
/// Test-only genuine legacy publication through the unchanged v1 encoder and
/// the actual protected result transaction. This is fixture protocol, never a
/// product setting, public request or permission to seed a positive receipt.
#[cfg(test)]
pub(super) fn reconcile_departed_legacy_fixture(root: &RootStore, id: &str) -> Result<Outcome, E> {
    if !root.writable {
        return Err(E::Refused("root-read-only"));
    }
    let (original, _) = journal::read(root, id)?;
    let mode = if original.failed {
        Mode::FailedOriginal
    } else {
        Mode::Verdict
    };
    record_mode(
        root,
        id,
        mode,
        OwnerBoundary::ConfirmedDeparted,
        crate::root_store::native_review_context::CertificateFormat::LegacyV1,
    )
}
/// Explicitly retire a known successful original after confirmed owner departure
/// and an observed dependency change. No error-catching fallback, fresh verdict,
/// new provider permission, task acceptance, refund or authenticated actor claim.
/// Original proof and non-verdict result use the same atomic/replay contract.
#[cfg(test)]
pub(super) fn retire_stale_departed(root: &RootStore, id: &str) -> Result<Outcome, E> {
    record_mode(
        root,
        id,
        Mode::StaleOriginal,
        OwnerBoundary::ConfirmedDeparted,
        crate::root_store::native_review_context::CertificateFormat::ContentV2,
    )
}
/// Observe departed-owner recovery prerequisites under saved storage policy,
/// without files, result, grant, budget or Work mutation. Rebuild source originals
/// only for verdicts; failed calls remain non-applicable. A successful observation
/// is no lease: reconcile_departed repeats all checks in its commit transaction.
/// Original typed IO/OS/schema/rollback/close errors remain failures.
#[cfg(test)]
pub(super) fn inspect_departed(root: &RootStore, id: &str) -> Result<(), E> {
    crate::agent_storage_retry::execute_borrowed(&root.db, &root.directory, false, |_, settings| {
        transaction::apply(root, |tx| {
            let (original, _) = journal::read(root, id)?;
            if original.failed {
                super::failure::prepare(root, tx, id, OwnerBoundary::ConfirmedDeparted)
                    .map(|_| Change::Observe(()))
            } else {
                prepare(root, tx, settings, id, OwnerBoundary::ConfirmedDeparted)
                    .map(|_| Change::Observe(()))
            }
        })
    })
}
/// Lifecycle proof boundary, not a configurable policy or transferred owner.
#[derive(Clone, Copy)]
pub(super) enum OwnerBoundary {
    CurrentOriginal,
    ConfirmedDeparted,
}
/// Distinct lifecycle operations, not operator policy or configurable defaults.
#[derive(Clone, Copy)]
pub(super) enum Mode {
    Verdict,
    FailedOriginal,
    StaleOriginal,
}
/// Share the exact atomic publication/replay path for explicit lifecycle modes.
/// Mode selection cannot change the original outcome: a stale non-verdict
/// requires a successful original, while failed-call mode requires a failed one;
/// original terminal phase/receipt/certificate govern historical replay.
pub(super) fn record_mode(
    root: &RootStore,
    id: &str,
    mode: Mode,
    owner: OwnerBoundary,
    format: crate::root_store::native_review_context::CertificateFormat,
) -> Result<Outcome, E> {
    if !root.writable {
        return Err(E::Refused("root-read-only"));
    }
    let mut known = None;
    let selected=store::job(&root.db,id).map_err(|e|E::Launch(Box::new(e)))?
        .ok_or(E::Refused("review-publication-job-required"))?;
    let mut operation=|source:Option<&crate::root_store::task_validation::ReviewCandidate>|crate::agent_storage_retry::execute_borrowed(
        &root.db,
        &root.directory,
        false,
        |_, settings| {
            let outcome = transaction::apply(root, |tx| {
                let job = store::job(tx, id)
                    .map_err(|e| E::Launch(Box::new(e)))?
                    .ok_or(E::Refused("review-publication-job-required"))?;
                if matches!(job.phase, Phase::Recorded | Phase::Refused) {
                    let (r, hash) = journal::read(root, id)?;
                    if job.receipt_sha256.as_deref() != Some(&hash) || !same_origin(&job, &r.job) {
                        return Err(E::Refused("review-publication-recorded-original-changed"));
                    }
                    recorded_artifact(root, tx, &job, &r)?;
                    let work = crate::read_recorded_work(tx, &job.work_id).map_err(E::Work)?;
                    return Ok(Change::Observe(Outcome {
                        schema_version: 1,
                        review_id: id.into(),
                        review_recorded: true,
                        verdict_applicable: job.phase == Phase::Recorded,
                        task_accepted: false,
                        replayed: true,
                        work,
                    }));
                }
                let mut ready = match mode {
                    Mode::Verdict => prepare_using(root, tx, settings, id, owner,source)?,
                    Mode::FailedOriginal => super::failure::prepare(root, tx, id, owner)?,
                    Mode::StaleOriginal => super::stale::prepare(root, tx, settings, id, owner)?,
                };
                let legacy_name = format!(
                    "native-review-public-{}.json",
                    crate::provider_configuration::configuration_digest(id.as_bytes())
                );
                if matches!(
                    format,
                    crate::root_store::native_review_context::CertificateFormat::LegacyV1
                ) {
                    ready.review.receipt = format!(".swarm/{legacy_name}");
                }
                let body = crate::root_store::native_review_context::certificate_bytes_format(
                    serde_json::to_value(&ready.review)
                        .map_err(|e| caused("review-publication-public-encode", e))?,
                    &job.work_id,
                    &job.task_id,
                    &job.source_sha256,
                    &job.validation_receipt_sha256,
                    format,
                )?;
                let sha = crate::provider_configuration::configuration_digest(&body);
                let name = match format {
                    crate::root_store::native_review_context::CertificateFormat::LegacyV1 => {
                        legacy_name
                    }
                    crate::root_store::native_review_context::CertificateFormat::ContentV2 => {
                        format!("native-review-public-v2-{sha}.json")
                    }
                };
                public_file(root, &name, &body)?;
                ready.review.receipt = format!(".swarm/{name}");
                ready.review.receipt_sha256 = sha;
                quiescent(&job, owner)?;
                match mode {
                    Mode::Verdict => {
                        guard::held_inputs(root, tx, settings.clone(), &job,source,true)?;
                    }
                    Mode::StaleOriginal => {
                        let fresh = super::stale::prepare(root, tx, settings, id, owner)?;
                        if fresh.hash != ready.hash
                            || fresh.work != ready.work
                            || fresh.review.reason != ready.review.reason
                        {
                            return Err(E::Refused("review-stale-publication-changed"));
                        }
                    }
                    Mode::FailedOriginal => {
                        let fresh = super::failure::prepare(root, tx, id, owner)?;
                        if fresh.hash != ready.hash || fresh.work != ready.work {
                            return Err(E::Refused("review-failure-publication-changed"));
                        }
                    }
                }
                apply_candidate(&mut ready)?;
                let revision = ready.work.revision;
                ready.work.revision = revision
                    .checked_add(1)
                    .ok_or(E::Refused("review-publication-revision-overflow"))?;
                ready.work.updated =
                    crate::system_timestamp().map_err(|e| caused("review-publication-clock", e))?;
                let event = crate::encode_model(&ready.review).map_err(E::Encode)?;
                let staged = crate::stage_recorded_change_with_hook(
                    tx,
                    crate::RecordedChange {
                        work: &ready.work,
                        expected_revision: revision,
                        event: &ready.review.result_event,
                        kind: "review.result",
                        request: &event,
                    },
                    |db, _| {
                        let mut done = job.clone();
                        done.phase = match mode {
                            Mode::Verdict => Phase::Recorded,
                            Mode::FailedOriginal | Mode::StaleOriginal => Phase::Refused,
                        };
                        done.receipt_sha256 = Some(ready.hash.clone());
                        done.published_revision = Some(ready.work.revision);
                        store::change(db, &job, &done)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                    },
                )
                .map_err(|e| caused("review-publication-stage", e))?;
                if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                    return Err(E::Refused("review-publication-unexpected-event-replay"));
                }
                Ok(Change::Mutate(Outcome {
                    schema_version: 1,
                    review_id: id.into(),
                    review_recorded: true,
                    verdict_applicable: matches!(mode, Mode::Verdict),
                    task_accepted: false,
                    replayed: false,
                    work: ready.work,
                }))
            })?;
            known = Some((outcome.work.id.clone(), outcome.work.revision));
            Ok(outcome)
        },
    );
    let result=if matches!(mode,Mode::Verdict){super::sources::job(root,&selected,operation)}else{operation(None)};
    result.map_err(|primary| match known {
        Some((work, revision)) => E::WorkRecorded {
            primary: Box::new(primary),
            work,
            revision,
        },
        None => primary,
    })
}
