//! Normal reviewer orchestration: one private gated process, original, result.
use super::{archive, capture, claim, diagnostics, gate, guard, journal, process, publication, store};
use std::time::Instant;
use crate::{
    root_store::{finished, task_validation, RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use std::{convert::Infallible, io, os::unix::net::UnixStream, path::Path, process::Command};

/// Public recorded result acknowledgement, never task acceptance or fresh exec.
/// Work is current at the result transaction; replay describes a historical
/// outcome, which may be an error/stale non-verdict rather than an applicable view.
#[derive(serde::Serialize)]
pub struct TaskReviewOutcome {
    pub schema_version: i64,
    pub review_id: String,
    pub review_recorded: bool,
    pub verdict_applicable: bool,
    pub task_accepted: bool,
    pub replayed: bool,
    /// Safe independent log sink failures. The review outcome is unchanged.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub review_diagnostic_failures: Vec<&'static str>,
    pub work: crate::Work,
}
impl From<publication::Outcome> for TaskReviewOutcome {
    fn from(v: publication::Outcome) -> Self {
        Self {
            schema_version: v.schema_version,
            review_id: v.review_id,
            review_recorded: v.review_recorded,
            verdict_applicable: v.verdict_applicable,
            task_accepted: v.task_accepted,
            replayed: v.replayed,
            review_diagnostic_failures: Vec::new(),
            work: v.work,
        }
    }
}
/// Preserve exact typed failures behind safe static boundaries, without provider text.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
/// Preserve every independent failure; this neither retries nor infers rollback.
fn accumulate(target: &mut Option<E>, error: E) {
    *target = Some(match target.take() {
        None => error,
        Some(primary) => E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(error),
        },
    });
}
/// Recover original/caller boundary from actual OS identity, never a request PID.
/// A different live owner/host/uncertain observation refuses; no adoption/signal.
fn owner(job: &store::Job) -> Result<publication::OwnerBoundary, E> {
    let current =
        crate::SupervisorIdentity::current().map_err(|e| caused("review-recovery-self", e))?;
    if job.owner_pid == current.pid()
        && job.owner_stamp == current.process_stamp()
        && job.owner_host == current.host()
    {
        Ok(publication::OwnerBoundary::CurrentOriginal)
    } else {
        guard::departed(job)?;
        Ok(publication::OwnerBoundary::ConfirmedDeparted)
    }
}
/// Explicit result action, not a runtime policy/default or a new provider permission.
enum Recovery {
    ApplyOriginal,
    RetireStale,
}
/// Reconcile one retained original under exact current/departed ownership. Missing,
/// legacy, unknown-wait and live-child evidence stays an error; never rerun. Terminal
/// replay uses its immutable result directly, without claiming current owner identity.
fn recover(root: &RootStore, id: &str, action: Recovery) -> Result<TaskReviewOutcome, E> {
    if !root.writable {
        return Err(E::Refused("root-read-only"));
    }
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(E::Refused("review-recovery-id-invalid"));
    }
    let job = store::job(&root.db, id)
        .map_err(|e| E::Launch(Box::new(e)))?
        .ok_or(E::Refused("review-recovery-job-required"))?;
    if matches!(job.phase, store::Phase::Recorded | store::Phase::Refused) {
        return publication::record(root, id).map(Into::into);
    }
    let boundary = owner(&job)?;
    let (original, _) = journal::read(root, id)?;
    let mode = match action {
        Recovery::RetireStale => publication::Mode::StaleOriginal,
        Recovery::ApplyOriginal => {
            if journal::no_grant_original(&original) {
                publication::Mode::Unstarted
            } else if original.failed {
                publication::Mode::FailedOriginal
            } else {
                publication::Mode::Verdict
            }
        }
    };
    publication::record_mode(
        root,
        id,
        mode,
        boundary,
        crate::root_store::native_review_context::CertificateFormat::ContentV2,
    )
    .map(Into::into)
}
/// Collect runtime/decode failures only after their original observations were
/// projected into a receipt. No status0/EOF/empty response is made successful.
fn invocation_errors(
    observed: process::Observation,
    reply: Result<super::reply::Output, A>,
    facts_error: Option<A>,
) -> Option<E> {
    let mut errors = None;
    match observed.process {
        Err(e) => accumulate(&mut errors, caused("review-runtime-process", e)),
        Ok(p) if !p.usage.status.success() => {
            accumulate(&mut errors, E::Refused("review-provider-exit-unsuccessful"))
        }
        Ok(_) => {}
    }
    for e in observed.transport_errors {
        accumulate(&mut errors, e);
    }
    if let Some(e) = observed.interruption {
        accumulate(&mut errors, e);
    }
    if observed.gate.is_some() {
        accumulate(&mut errors, E::Refused("review-private-gate-refused"));
    }
    if let Err(e) = reply {
        accumulate(&mut errors, E::Launch(Box::new(e)));
    }
    if let Some(e) = facts_error {
        accumulate(&mut errors, E::Launch(Box::new(e)));
    }
    errors
}
/// Run only a newly committed claim using the actual current-image private entry.
/// The decoder consumes exact provider stdout while raw retention follows saved
/// combined caps; no SDK header filter, tee or extra policy exists here. Known
/// runner failures retain wait/transport/parser/facts causes before publication.
/// Setup failures before spawn remain explicit; absent grant/wait evidence is
/// never synthesized, retried or called completed. Caller preserves known Work.
fn execute(
    root: &RootStore,
    staged: claim::Staged,
    helper: std::path::PathBuf,
    diag: Option<&diagnostics::Attempt<'_>>,
) -> Result<TaskReviewOutcome, E> {
    execute_using(root,staged,helper,None,diag)
}
/// Same decoder/capture/original/publication with engine-only borrowed mission
/// authority. A terminal readback never enters this process path; known Work,
/// monetary reserves and all original error trees remain intact.
fn execute_using(root:&RootStore,staged:claim::Staged,helper:std::path::PathBuf,lease:Option<&crate::MissionLease>,diag:Option<&diagnostics::Attempt<'_>>)->Result<TaskReviewOutcome,E>{
    if let Some(diag) = diag {
        diag.emit(root, "execution", Instant::now(), None, serde_json::json!({
            "review_id": staged.job.id,
            "settings_revision": staged.material.settings.revision.0,
        }));
    }
    let originals: Vec<_> = staged
        .material
        .inputs
        .materials
        .iter()
        .map(|s| crate::ReviewQuoteOriginal {
            name: &s.name,
            kind: &s.quote_kind,
            content: &s.content,
        })
        .collect();
    let managed_snapshot=crate::managed_review_record::Snapshot::capture(&staged.material.inputs,&staged.material.settings.values)
        .map_err(|e|E::Launch(Box::new(e)))?;
    let protocol=super::reply::protocol(&staged.material.inputs)?;
    let decoder = super::reply::Stream::new(
        &staged.material.settings.values,
        staged.material.inputs.preview.criterion_count,
        &originals,protocol.as_ref(),
    )
    .map_err(|e| E::Launch(Box::new(e)))?;
    let policy = staged
        .material
        .settings
        .values
        .agent_execution_policy()
        .map_err(E::Settings)?;
    let transport = capture::Transport::new_domain(policy, decoder).map_err(|e| E::Launch(Box::new(e)))?;
    let mut command = Command::new(helper);
    command
        .arg("_reviewer_gate")
        .arg(&staged.job.id)
        .arg("--root")
        .arg(&root.root)
        .arg("--json");
    let process_start = Instant::now();
    let observed = {
        let (mut stdout, mut stderr) = transport.sinks();
        process::run_using(root, &staged.job, &staged.material, command, &mut stdout, &mut stderr, lease, diag)
    };
    let observed = observed.inspect_err(|error| {
        if let Some(diag) = diag {
            diag.emit(root, "process", process_start, Some(error.code()), ());
        }
    })?;
    let local_stop_transport_complete=matches!(observed.interruption.as_ref(),Some(E::Refused("review-execution-interrupted")))
        && observed.process.as_ref().is_ok_and(|p|!p.timed_out&&!p.output_drain_timed_out&&crate::execution_interrupt::stopped_status(&p.usage.status))
        && observed.grant_attempted && observed.channel_closed && !observed.duration_expired && !observed.drain_expired
        && observed.gate.is_none() && observed.gate_bytes.is_empty() && observed.transport_errors.is_empty();
    let valid_transport = observed
        .process
        .as_ref()
        .is_ok_and(|p| p.usage.status.success() && !p.timed_out && !p.output_drain_timed_out)
        && observed.interruption.is_none()
        && !observed.duration_expired
        && !observed.drain_expired
        && observed.transport_errors.is_empty()
        && observed.gate.is_none()
        && observed.gate_bytes.is_empty()
        && observed.grant_attempted
        && observed.channel_closed;
    if let Some(diag) = diag {
        let cause = if valid_transport { None } else {
            observed.gate.as_ref().map(|gate| gate.code.as_str()).and_then(diagnostics::gate_cause)
                .or(Some("review-provider-transport-incomplete"))
        };
        diag.emit(root, "process", process_start, cause, serde_json::json!({
            "helper_registered": observed.job.child_pid.is_some(),
            "gate_permission_recorded": observed.job.permission == 1,
            "grant_attempted": observed.grant_attempted,
            "channel_closed": observed.channel_closed,
            "duration_expired": observed.duration_expired,
            "drain_expired": observed.drain_expired,
            "interrupted": observed.interruption.is_some(),
            "transport_error_count": observed.transport_errors.len(),
        }));
    }
    let reply_start = Instant::now();
    let (raw, mut stream) = transport.into_parts();
    if !valid_transport {
        stream.incomplete_transport();
    }
    let reply = stream.finish();
    let original_reply = reply
        .as_ref()
        .ok()
        .map(|r| r.transport.response.as_slice())
        .or_else(|| stream.retained_reply());
    let (facts, facts_error) = match stream.provider_facts() {
        Ok(f) => (f, None),
        Err(e) => (crate::GoRaw::default(), Some(e)),
    };
    // Null facts here means encoding unavailable, never empty successful facts or
    // cost0: the actual encoding error makes the original failed and is retained.
    let mut receipt = journal::observe(
        &observed,
        original_reply,
        facts,
        reply.as_ref().err().or(facts_error.as_ref()),
        crate::system_timestamp().map_err(E::Runtime)?,
        raw,
    )?;
    if let Some(diag) = diag {
        let cause = reply.as_ref().err().or(facts_error.as_ref()).map(A::code);
        let raw = receipt.raw_capture.as_ref();
        diag.emit(root, "reply", reply_start, cause, serde_json::json!({
            "exit_code": receipt.exit_code,
            "output_complete": receipt.output_complete,
            "opinion": reply.as_ref().ok().map(|decoded| decoded.state),
            "stdout_bytes": raw.map(|capture| capture.stdout.observed_bytes),
            "stderr_bytes": raw.map(|capture| capture.stderr.observed_bytes),
            "truncated": raw.map(|capture| capture.stdout.truncated || capture.stderr.truncated),
        }));
    }
    receipt.managed_review=managed_snapshot;
    if reply.is_err() {
        if let Some(e) = &facts_error {
            receipt.causes.extend(crate::native_diagnostics::project(e));
        }
    }
    let local_stop_complete=local_stop_transport_complete && receipt.original_reply.is_none() && receipt.raw_capture.as_ref().is_some_and(|raw|!raw.failed) && facts_error.is_none();
    let mut error = invocation_errors(observed, reply, facts_error);
    let receipt_start = Instant::now();
    if let Err(persist) = journal::persist(root, &receipt) {
        if let Some(diag) = diag {
            diag.emit(root, "receipt", receipt_start, Some(persist.code()), ());
        }
        if let Some(primary) = error {
            return Err(E::Cleanup {
                primary: Some(Box::new(primary)),
                cleanup: Box::new(persist),
            });
        }
        return Err(persist);
    }
    if let Some(diag) = diag {
        diag.emit(root, "receipt", receipt_start, None, serde_json::json!({
            "review_id": receipt.job.id,
            "failed": receipt.failed,
            "waited": receipt.waited,
        }));
    }
    if receipt.failed {
        let primary = error
            .take()
            .unwrap_or(E::Refused("review-call-failed-original-retained"));
        let publication_start = Instant::now();
        let settled = if journal::waited_before_grant(&receipt) {
            publication::record_mode(
                root, &receipt.job.id, publication::Mode::Unstarted,
                publication::OwnerBoundary::CurrentOriginal,
                crate::root_store::native_review_context::CertificateFormat::ContentV2,
            )
        } else {
            publication::reconcile_failed(root, &receipt.job.id)
        };
        if let Some(diag) = diag {
            let cause = settled.as_ref().err().map(E::code);
            diag.emit(root, "publication", publication_start, cause, serde_json::json!({
                "review_recorded": settled.as_ref().ok().map(|outcome| outcome.review_recorded),
                "verdict_applicable": settled.as_ref().ok().map(|outcome| outcome.verdict_applicable),
            }));
        }
        return match settled {
            Ok(v) => Err(E::WorkRecorded {
                primary: Box::new(if local_stop_complete{crate::execution_interrupt::completed(crate::execution_interrupt::InterruptedRole::Reviewer,primary)}else{primary}),
                work: v.work.id,
                revision: v.work.revision,
            }),
            Err(publish) => Err(E::Cleanup {
                primary: Some(Box::new(primary)),
                cleanup: Box::new(publish),
            }),
        };
    }
    if let Some(error) = error {
        return Err(error);
    }
    let publication_start = Instant::now();
    let result = publication::record(root, &receipt.job.id);
    if let Some(diag) = diag {
        let outcome = result.as_ref().ok();
        diag.emit(root, "publication", publication_start, result.as_ref().err().map(E::code), serde_json::json!({
            "review_recorded": outcome.map(|value| value.review_recorded),
            "verdict_applicable": outcome.map(|value| value.verdict_applicable),
            "task_accepted": outcome.map(|value| value.task_accepted),
            "recorded_revision": outcome.map(|value| value.work.revision),
        }));
    }
    result.map(Into::into)
}
impl RootStore {
    /// Execute the configured native reviewer after genuine current worker/control
    /// evidence. The bounded request has the task-validation envelope, never argv,
    /// PID, actor or grant. Claim/call/estimate commit before spawn; exact event replay
    /// only reads/reconciles an original. Managed execution retains actual
    /// candidate custody outside snapshots, validates the full cohort and freezes
    /// its protocol/policy in the original journal. No accepted baseline or task
    /// acceptance is inferred. The host binary must provide the native
    /// private helper entry. Failed calls record explicit error only with known
    /// wait/grant evidence and still return an error; no acceptance/refund/extra exec.
    /// Every later failure retains the known claim Work and original typed causes.
    pub fn run_task_review(&self, work: &str, raw: &[u8]) -> Result<TaskReviewOutcome, E> {
        self.run_task_review_using(work,raw,None)
    }
    /// Engine-only optional mission entry. Caller must maintain its owning cycle
    /// heartbeat. Fresh claim/permission carry the actual lease; exact terminal
    /// replay only reads the original. No request actor, refund or auto-accept.
    fn run_task_review_using(&self,work:&str,raw:&[u8],lease:Option<&crate::MissionLease>)->Result<TaskReviewOutcome,E>{
        let failures = diagnostics::Failures::default();
        let result = self.run_task_review_observed(work, raw, lease, &failures);
        let mut failures = failures.into_inner();
        failures.sort_unstable();
        failures.dedup();
        match result {
            Ok(mut outcome) => {
                outcome.review_diagnostic_failures = failures;
                Ok(outcome)
            }
            Err(primary) if failures.is_empty() => Err(primary),
            Err(primary) => Err(E::ReviewDiagnostics { primary: Box::new(primary), failures }),
        }
    }

    /// Execute the normal reviewer contract and collect optional journal failures
    /// separately, without writing to process response streams or retrying.
    fn run_task_review_observed(
        &self, work: &str, raw: &[u8], lease: Option<&crate::MissionLease>,
        failures: &diagnostics::Failures,
    ) -> Result<TaskReviewOutcome, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let request = task_validation::decode(raw).map_err(|e| E::Launch(Box::new(e)))?;
        let helper = std::env::current_exe().map_err(|e| caused("review-runtime-self-image", e))?;
        let mut known=None;
        let intent = crate::encode_model(&request).map_err(E::Encode)?;
        let correlation = crate::provider_configuration::configuration_digest(&intent);
        let started = Instant::now();
        let mut preflight = diagnostics::Preflight::default();
        let result=super::sources::request(self,work,&request,|source| {
        let reservation_start = Instant::now();
        let staged = claim::record_using_candidate(self, work, &request, lease, source, &mut preflight);
        let reservation_start = preflight.ended_at().unwrap_or(reservation_start);
        let preflight_passed = preflight.emit(self, &correlation, failures);
        let diag = preflight.attempt(&correlation, failures);
        if let Some(diag) = &diag {
            match &staged {
                Ok(claim::Recorded::New(staged)) => diag.emit(
                    self, "reservation", reservation_start, None,
                    diagnostics::reservation(&staged.job, &staged.work),
                ),
                Err(error) if preflight_passed => diag.emit(
                    self, "reservation", reservation_start, Some(error.code()), (),
                ),
                _ => {}
            }
        }
        let staged = staged?;
        let (job_work, revision) = match &staged {
            claim::Recorded::New(v) => (v.work.id.clone(), v.work.revision),
            claim::Recorded::Replay(j) => (j.work_id.clone(), j.work_revision),
        };
        known=Some((job_work,revision));
        match staged {
            claim::Recorded::New(v) => match lease {None=>execute(self,*v,helper,diag.as_ref()),Some(lease)=>execute_using(self,*v,helper,Some(lease),diag.as_ref())},
            claim::Recorded::Replay(j) => recover(self, &j.id, Recovery::ApplyOriginal),
        }
        });
        let result = result.map_err(|primary| {
            if primary.recorded_work().is_some() || known.is_none() {
                primary
            } else {
                E::WorkRecorded {
                    primary: Box::new(primary),
                    work: known.as_ref().unwrap().0.clone(),
                    revision: known.as_ref().unwrap().1,
                }
            }
        });
        preflight.summarize(self, &correlation, failures, started, lease.is_some(), &result);
        result
    }
    /// Reconcile a known reviewer original without any new invocation. Exact current
    /// owner or confirmed-departed owner is required before nonterminal application;
    /// terminal replay is historical. Missing/unknown evidence and live foreign
    /// owners refuse, retaining original cause and reservations. No task acceptance.
    pub fn reconcile_task_review(&self, id: &str) -> Result<TaskReviewOutcome, E> {
        recover(self, id, Recovery::ApplyOriginal)
    }
    /// Explicitly retire a successful original only after an observed dependency
    /// change. Same ownership/wait/financial/atomic guards; no stale result is guessed
    /// from arbitrary IO errors and no current favorable verdict is produced.
    pub fn retire_stale_task_review(&self, id: &str) -> Result<TaskReviewOutcome, E> {
        recover(self, id, Recovery::RetireStale)
    }

    /// Close and archive a confirmed departed reservation whose helper was
    /// never authorized. Exact terminal replay returns the recorded result.
    /// No provider call, quota adjustment or task acceptance occurs.
    pub fn retire_unstarted_task_review(&self, id: &str) -> Result<TaskReviewOutcome, E> {
        if !swarm_identity_contract::name(id.as_bytes()) {
            return Err(E::Refused("review-recovery-id-invalid"));
        }
        publication::record_mode(
            self, id, publication::Mode::Unstarted, publication::OwnerBoundary::ConfirmedDeparted,
            crate::root_store::native_review_context::CertificateFormat::ContentV2,
        ).map(Into::into)
    }

    /// Archive a recorded terminal managed process failure without clearing its
    /// financial reservation, replacing its candidate or creating a verdict.
    pub fn archive_failed_task_review(&self, id: &str) -> Result<TaskReviewOutcome, E> {
        archive::record(self, id).map(Into::into)
    }

    /// Verify an explicit managed failure archive before a new claim uses
    /// existing producer evidence. Ordinary non-archived errors retain their
    /// recovery rules. Verification does not authorize any execution.
    pub(in crate::root_store) fn archived_failed_task_review(&self, tx: &rusqlite::Transaction<'_>,
        work: &crate::Work, task: &str, review: &crate::IndependentReview) -> Result<bool, E> {
        archive::verified(self, tx, work, task, review)
    }

    /// Recognize the verified archived no-grant review in a current snapshot.
    /// It carries no provider failure and grants no retry by itself.
    pub(in crate::root_store) fn archived_unstarted_review(&self, tx: &rusqlite::Transaction<'_>, work: &str, task: &str, id: &str) -> Result<bool, E> {
        publication::verified_unstarted(self, tx, work, task, id)
    }
}
/// Private current-image helper: require an actual Unix socket before reporting
/// anything into it, duplicate CLOEXEC stdin, then use the shared kernel-peer/
/// parent-death/grant/source/host/sealed-exec gate. A regular file is never written
/// as a diagnostic socket. Success replaces this process exactly once. Failure
/// retains primary and close/report errors; no new model identity is authenticated.
pub fn execute_review_gate(root: &Path, id: &str) -> Result<Infallible, E> {
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(E::Refused("review-gate-id-invalid"));
    }
    let fd = rustix::io::fcntl_dupfd_cloexec(io::stdin(), 3)
        .map_err(|e| caused("review-gate-channel-duplicate", e))?;
    let channel = UnixStream::from(fd);
    if let Err(e) = channel.peer_addr() {
        return finished(
            Err(caused("review-gate-channel-kind", e)),
            nix::unistd::close(channel).map_err(|e| caused("review-gate-channel-close", e)),
        );
    }
    gate::execute_reported(root, id, channel)
}

impl RootStore {
    /// Select one fresh automatic review after real current controls. Requires
    /// the actual owning mission lease and an active caller-maintained cycle
    /// heartbeat. Saved reviewer/budget/provider/source guards remain authoritative;
    /// no public request can supply automatic rights or reset consumption.
    /// Returns None for explicitly paused/unconfigured planning or no new task;
    /// an existing same-attempt result is not automatically retried or declared
    /// fresh. A retained running review requires explicit reconciliation.
    /// Runs foreground; no task acceptance or whole-mission completion follows.
    /// Errors preserve actual claim/result Work and original cleanup/IO causes.
    pub fn run_next_mission_review(&self,lease:&crate::MissionLease)->Result<Option<TaskReviewOutcome>,E>{
        let id=crate::root_store::mission_supervision_store::bound_mission_work(self,lease)
            .map_err(|e|caused("mission-review-lease",e))?.to_owned();
        let work=self.read_work(&id)?;
        let Some(planning)=work.planning.as_ref() else{return Ok(None)};
        if planning.paused || !planning.failure.is_empty() || planning.reviewer.is_none(){return Ok(None)}
        for task in work.tasks.iter().flatten().filter(|t|t.status=="submitted" && t.automatic_validation.is_some()){
            let Some(attempt)=task.attempts.as_ref().and_then(|a|a.last()) else{continue};
            if attempt.status!="completed"{continue;}
            if let Some(review)=task.independent_review.as_ref().filter(|r|r.attempt==attempt.id){
                if review.state=="running"{return Err(E::Refused("review-running-state-requires-reconciliation"));}
                continue;
            }
            let request=task_validation::Request{schema_version:1,event_id:crate::generate_identifier("mission-review-").map_err(E::Runtime)?,expected_revision:work.revision,task_id:task.id.clone()};
            let raw=serde_json::to_vec(&request).map_err(|e|caused("mission-review-request",e))?;
            return self.run_task_review_using(&id,&raw,Some(lease)).map(Some);
        }
        Ok(None)
    }
}
