//! Normal reviewer orchestration: one private gated process, original, result.
use super::{capture, claim, gate, guard, journal, process, publication, store};
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
            if original.failed {
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
) -> Result<TaskReviewOutcome, E> {
    execute_using(root,staged,helper,None)
}
/// Same decoder/capture/original/publication with engine-only borrowed mission
/// authority. A terminal readback never enters this process path; known Work,
/// monetary reserves and all original error trees remain intact.
fn execute_using(root:&RootStore,staged:claim::Staged,helper:std::path::PathBuf,lease:Option<&crate::MissionLease>)->Result<TaskReviewOutcome,E>{
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
    let observed = {
        let (mut stdout, mut stderr) = transport.sinks();
        match lease {
            None=>process::run(root,&staged.job,command,&mut stdout,&mut stderr)?,
            Some(lease)=>process::run_using(root,&staged.job,command,&mut stdout,&mut stderr,Some(lease))?,
        }
    };
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
    receipt.managed_review=managed_snapshot;
    if reply.is_err() {
        if let Some(e) = &facts_error {
            receipt.causes.extend(crate::native_diagnostics::project(e));
        }
    }
    let local_stop_complete=local_stop_transport_complete && receipt.original_reply.is_none() && receipt.raw_capture.as_ref().is_some_and(|raw|!raw.failed) && facts_error.is_none();
    let mut error = invocation_errors(observed, reply, facts_error);
    if let Err(persist) = journal::persist(root, &receipt) {
        if let Some(primary) = error {
            return Err(E::Cleanup {
                primary: Some(Box::new(primary)),
                cleanup: Box::new(persist),
            });
        }
        return Err(persist);
    }
    if receipt.failed {
        let primary = error
            .take()
            .unwrap_or(E::Refused("review-call-failed-original-retained"));
        return match publication::reconcile_failed(root, &receipt.job.id) {
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
    publication::record(root, &receipt.job.id).map(Into::into)
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
    pub(in crate::root_store) fn run_task_review_using(&self,work:&str,raw:&[u8],lease:Option<&crate::MissionLease>)->Result<TaskReviewOutcome,E>{
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let request = task_validation::decode(raw).map_err(|e| E::Launch(Box::new(e)))?;
        let helper = std::env::current_exe().map_err(|e| caused("review-runtime-self-image", e))?;
        let mut known=None;
        let result=super::sources::request(self,work,&request,|source| {
        let staged=claim::record_using_candidate(self,work,&request,lease,source)?;
        let (job_work, revision) = match &staged {
            claim::Recorded::New(v) => (v.work.id.clone(), v.work.revision),
            claim::Recorded::Replay(j) => (j.work_id.clone(), j.work_revision),
        };
        known=Some((job_work,revision));
        match staged {
            claim::Recorded::New(v) => match lease {None=>execute(self,*v,helper),Some(lease)=>execute_using(self,*v,helper,Some(lease))},
            claim::Recorded::Replay(j) => recover(self, &j.id, Recovery::ApplyOriginal),
        }
        });
        result.map_err(|primary| {
            if primary.recorded_work().is_some() || known.is_none() {
                primary
            } else {
                E::WorkRecorded {
                    primary: Box::new(primary),
                    work: known.as_ref().unwrap().0.clone(),
                    revision: known.as_ref().unwrap().1,
                }
            }
        })
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
