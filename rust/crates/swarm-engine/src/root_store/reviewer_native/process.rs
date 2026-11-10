//! Candidate owned reviewer lifecycle; no result publication or production CLI.
//! Callers supply saved-policy streaming sinks and must preserve their original
//! bytes/facts and incomplete status before durable receipt/decision publication.
use super::{
    gate, guard,
    material::Material,
    store::{Job, Phase},
    transaction::{self, Change},
};
use crate::{
    root_store::{finished, RootStore, RootStoreError as E},
    validation_run::{run_controlled_command, OwnedProcessControl, OwnedProcessRun},
    AgentExecutionPolicy, AgentLaunchError as A, PlanningBootInstant as At, PlanningBootTimer,
    ValidationRunError,
};
use rustix::process::Signal;
use std::{
    io::{self, Read, Write},
    os::{
        fd::{AsFd, BorrowedFd, OwnedFd},
        unix::net::UnixStream,
    },
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Keep original typed causes behind a static safe boundary, without private IO.
fn caused(code: &'static str, error: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, error)))
}
/// Convert saved nonnegative milliseconds without fallback, clamping or ceiling.
fn millis(value: i64) -> Result<Duration, E> {
    Ok(Duration::from_millis(
        value
            .try_into()
            .map_err(|e| caused("review-process-duration", e))?,
    ))
}
/// Add a saved interval on the suspend-aware clock; representation failure refuses.
fn after(now: At, duration: Duration) -> Result<At, E> {
    now.checked_add(duration)
        .map_err(|e| caused("review-process-clock-range", e))
}
/// Map the original UTC admission expiry onto the current boot clock once. A
/// past expiry stays due. This does not renew admission or bound provider life.
fn startup_due(until: &str, now: At) -> Result<At, E> {
    let current = crate::system_timestamp().map_err(E::Runtime)?;
    let parse = |s: &str| {
        swarm_wire_json::go_rfc3339::parse(s).map_err(|e| caused("review-process-utc", e))
    };
    let delta = parse(until)? - parse(&current)?;
    let duration = if delta.is_negative() {
        Duration::ZERO
    } else {
        Duration::try_from(delta).map_err(|e| caused("review-process-utc-range", e))?
    };
    after(now, duration)
}
/// Current exact claim observation through the common saved contention policy.
/// Admission rechecks original TTL/host; later observation does not. Every SQL
/// snapshot is explicitly rolled back before settings release. No second debit,
/// permit, Work change or process retry occurs; original/cleanup causes survive.
pub(super) fn observe(root: &RootStore, job: &Job, admission: bool) -> Result<Option<Material>, E> {
    guard::owner(job)?;
    super::sources::job(root,job,|source|crate::agent_storage_retry::execute_borrowed(&root.db, &root.directory, false, |_, settings| {
        transaction::apply(root, |tx| {
            let material = if admission {
                Some(guard::fresh_using(root, tx, settings.clone(), job,source)?)
            } else {
                super::mission_authority::observe(root,tx,job,settings).map_err(|e|E::Launch(Box::new(e)))?;
                guard::held_inputs(root,tx,settings.clone(),job,source,false)?;
                None
            };
            Ok(Change::Observe(material))
        })
    }))
}
/// Saved-policy state only. No signals, sleeps, source authority or process
/// completion are inferred by these coordinates. All At values share this boot.
struct Schedule {
    policy: AgentExecutionPolicy,
    duration: Option<Duration>,
    next_poll: At,
    startup: At,
    total: Option<At>,
    birth_observed: bool,
    kill: Option<At>,
    drain: Option<At>,
    gate_eof: bool,
    stopping: bool,
    killed: bool,
    leader_done: bool,
    drain_expired: bool,
}
impl Schedule {
    /// Validate supplied saved policy and create an unstarted schedule. None
    /// duration is explicitly unlimited; no planner/silence fallback is used.
    fn new(
        policy: AgentExecutionPolicy,
        duration: Option<Duration>,
        now: At,
        startup: At,
    ) -> Result<Self, E> {
        policy.validate().map_err(E::Settings)?;
        Ok(Self {
            policy,
            duration,
            next_poll: now,
            startup,
            total: None,
            birth_observed: false,
            kill: None,
            drain: None,
            gate_eof: false,
            stopping: false,
            killed: false,
            leader_done: false,
            drain_expired: false,
        })
    }
    /// Start total duration at the actual runner's post-spawn callback. A second
    /// call refuses instead of extending a deadline. This grants no socket byte.
    fn started(&mut self, now: At) -> Result<(), E> {
        if self.birth_observed {
            return Err(E::Refused("review-process-clock-already-started"));
        }
        self.total = self.duration.map(|d| after(now, d)).transpose()?;
        self.birth_observed = true;
        Ok(())
    }
    /// Request TERM once, with saved nonnegative grace. Zero grace still returns
    /// TERM before the next observation requests KILL; no hidden minimum delay.
    fn stop(&mut self, now: At) -> Result<Signal, E> {
        self.kill = Some(after(now, millis(self.policy.term_grace_ms.0)?)?);
        self.stopping = true;
        Ok(Signal::TERM)
    }
    /// Observe actual leader readiness and start only the saved drain deadline.
    /// Its expiry records incomplete output, never successful provider completion.
    fn exited(&mut self, now: At) -> Result<(), E> {
        self.leader_done = true;
        self.drain = self
            .policy
            .output_drain_ms
            .0
            .map(|ms| millis(ms.0).and_then(|d| after(now, d)))
            .transpose()?;
        Ok(())
    }
    /// Return the earliest still-relevant wake. Handled expiry/kill never leaves
    /// an expired timer spinning, and an explicit unlimited drain installs none.
    fn next(&self) -> Option<At> {
        [
            if !self.leader_done && !self.stopping {
                Some(self.next_poll)
            } else {
                None
            },
            if !self.leader_done && !self.stopping && !self.gate_eof {
                Some(self.startup)
            } else {
                None
            },
            if !self.leader_done && !self.stopping {
                self.total
            } else {
                None
            },
            if !self.leader_done && !self.killed {
                self.kill
            } else {
                None
            },
            self.drain,
        ]
        .into_iter()
        .flatten()
        .min()
    }
    /// Classify due policy events without signalling. Drain/startup failures
    /// trigger common cleanup; duration asks TERM/grace/KILL. Source revocation
    /// is handled separately at next_poll and retains its own typed cause.
    fn due(&mut self, now: At) -> Result<Option<Signal>, E> {
        if self.drain.is_some_and(|d| now >= d) {
            self.drain_expired = true;
            return Err(E::Refused("review-process-output-drain-expired"));
        }
        if self.leader_done {
            return Ok(None);
        }
        if self.stopping {
            if !self.killed && self.kill.is_some_and(|d| now >= d) {
                self.killed = true;
                return Ok(Some(Signal::KILL));
            }
        } else if self.total.is_some_and(|d| now >= d) {
            return self.stop(now).map(Some);
        } else if !self.gate_eof && now >= self.startup {
            return Err(E::Refused("review-process-original-admission-expired"));
        }
        Ok(None)
    }
    /// Advance only the source-observation interval from the completed check.
    /// Slow IO is not hidden by saturating/clamping an expired poll coordinate.
    fn polled(&mut self, now: At) -> Result<(), E> {
        self.next_poll = after(now, millis(self.policy.poll_interval_ms.0)?)?;
        Ok(())
    }
}
/// Original independent observations after any attempt, including failed runner
/// cleanup/unreaped ownership. Raw stream bytes remain in caller-owned sinks;
/// gate bytes are private. EOF/exit0/grant alone never establish acceptance.
/// The caller still owes durable receipt, usage, fresh publication and recovery.
pub(super) struct Observation {
    pub job: Job,
    pub process: Result<OwnedProcessRun, ValidationRunError>,
    pub grant_attempted: bool,
    pub interruption: Option<E>,
    pub duration_expired: bool,
    pub drain_expired: bool,
    pub gate: Option<crate::agent_execution_gate::GateFailure>,
    pub gate_bytes: Vec<u8>,
    pub transport_errors: Vec<E>,
    pub channel_closed: bool,
    /// SDK instrumentation only; never a product bound or a completion proof.
    #[cfg(test)]
    pub max_source_observation: Duration,
}
/// Foreground owner plus actual registered child and one owned BOOTTIME wake.
struct Control<'a> {
    root: &'a RootStore,
    lease: Option<&'a crate::MissionLease>,
    job: Job,
    channel: Option<UnixStream>,
    timer: Option<PlanningBootTimer>,
    schedule: Schedule,
    gate_bytes: Vec<u8>,
    grant_attempted: bool,
    interruption: Option<E>,
    duration_expired: bool,
    max_source_observation: Duration,
}
impl Control<'_> {
    /// Read only the engine's private diagnostic endpoint. EOF stays ambiguous,
    /// IO/allocation failures propagate. Chunks are transport buffers, not caps.
    fn read_gate(&mut self) -> Result<(), E> {
        if self.schedule.gate_eof {
            return Ok(());
        }
        let channel = self
            .channel
            .as_mut()
            .ok_or(E::Refused("review-process-channel-missing"))?;
        loop {
            let mut bytes = [0u8; 4096];
            match channel.read(&mut bytes) {
                Ok(0) => {
                    self.schedule.gate_eof = true;
                    return Ok(());
                }
                Ok(n) => {
                    self.gate_bytes
                        .try_reserve(n)
                        .map_err(|e| caused("review-process-gate-allocation", e))?;
                    self.gate_bytes.extend_from_slice(&bytes[..n]);
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) => return Err(caused("review-process-gate-read", e)),
            }
        }
    }
    /// Arm only still-relevant saved-policy events. No periodic fallback timer.
    fn arm(&self) -> Result<(), E> {
        self.timer
            .as_ref()
            .ok_or(E::Refused("review-process-timer-missing"))?
            .arm(self.schedule.next())
            .map_err(|e| caused("review-process-timer-arm", e))
    }
}
impl OwnedProcessControl for Control<'_> {
    /// Commit actual registration and permission separately, updating job after
    /// each known commit. Record grant exposure before any possible socket write.
    /// Failure starts common owned cleanup and never retries a provider process.
    fn started(&mut self, pid: u32) -> io::Result<()> {
        (|| {
            self.schedule
                .started(At::now().map_err(|e| caused("review-process-clock", e))?)?;
            gate::register_recorded(self.root, &mut self.job, pid)?;
            match self.lease {
                None=>gate::permit_recorded(self.root,&mut self.job)?,
                Some(lease)=>gate::permit_recorded_using(self.root,&mut self.job,Some(lease))?,
            }
            self.grant_attempted = true;
            self.channel
                .as_mut()
                .ok_or(E::Refused("review-process-channel-missing"))?
                .write_all(&[1])
                .map_err(|e| caused("review-process-grant-write", e))?;
            self.arm()
        })()
        .map_err(io::Error::other::<E>)
    }
    /// Only common runner pidfd readiness starts post-leader drain observation.
    fn leader_exited(&mut self) -> io::Result<()> {
        self.schedule
            .exited(At::now()?)
            .and_then(|_| self.arm())
            .map_err(io::Error::other::<E>)
    }
    /// BOOTTIME timerfd supplies wakes; no Instant-based policy is substituted.
    fn next_wake(&self) -> Option<Instant> {
        None
    }
    /// Borrow only the owned timer for the common IO/process poll.
    fn wake_descriptor(&self) -> Option<BorrowedFd<'_>> {
        self.timer.as_ref().map(AsFd::as_fd)
    }
    /// Borrow local stop wake independently from the unchanged policy timer.
    /// Disable it after stop/leader exit so grace/drain do not busy-loop.
    fn interruption_descriptor(&self)->Option<BorrowedFd<'_>>{
        if self.schedule.stopping||self.schedule.leader_done{None}else{self.root.interruption.as_ref().map(crate::ExecutionInterrupt::descriptor)}
    }
    /// Apply due policy and current saved-interval source checks. Source failure
    /// retains its cause and requests TERM then saved-grace KILL. Checks can block
    /// on IO/saved storage contention: measured latency is retained, no realtime
    /// reaction guarantee is claimed. Leader readiness cancels admission/total.
    fn observe(&mut self) -> io::Result<Option<Signal>> {
        (|| {
            if let Some(timer) = &self.timer {
                timer
                    .take_expirations()
                    .map_err(|e| caused("review-process-timer-read", e))?;
            }
            self.read_gate()?;
            let now = At::now().map_err(|e| caused("review-process-clock", e))?;
            if !self.schedule.leader_done && !self.schedule.stopping && self.root.interruption.as_ref().map(crate::ExecutionInterrupt::reason).transpose().map_err(|e|caused("review-interrupt-observation",e))?.flatten().is_some(){
                self.interruption=Some(E::Refused("review-execution-interrupted"));
                let signal=self.schedule.stop(now)?;self.arm()?;return Ok(Some(signal));
            }
            let was_stopping = self.schedule.stopping;
            if let Some(signal) = self.schedule.due(now)? {
                if !was_stopping {
                    self.duration_expired = true;
                }
                self.arm()?;
                return Ok(Some(signal));
            }
            if !self.schedule.leader_done
                && !self.schedule.stopping
                && now >= self.schedule.next_poll
            {
                let result = observe(self.root, &self.job, false).map(|_| ());
                let after = At::now().map_err(|e| caused("review-process-clock", e))?;
                let elapsed = after
                    .duration_since(now)
                    .map_err(|e| caused("review-process-clock", e))?;
                self.max_source_observation = self.max_source_observation.max(elapsed);
                self.schedule.polled(after)?;
                if let Err(error) = result {
                    self.interruption = Some(error);
                    let signal = self.schedule.stop(after)?;
                    self.arm()?;
                    return Ok(Some(signal));
                }
            }
            self.arm()?;
            Ok(None)
        })()
        .map_err(io::Error::other::<E>)
    }
}
/// Run one already-claimed reviewer via a caller-constructed PRIVATE gated helper.
/// Caller must select the current self-image helper, never provider argv directly,
/// and supply saved-policy capped/parser sinks. The normal Root runtime owns this
/// construction; actual registration/grant is controlled here. Setup
/// failures return Err before spawn; after attempting the common runner, original
/// wait/cleanup, independent gate errors and last known job all remain in the
/// Observation. No global watchdog, silent timeout, retry, receipt, refund or
/// publication is introduced. Output failure must invalidate the caller's parser.
pub(super) fn run(
    root: &RootStore,
    job: &Job,
    command: Command,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<Observation, E> {
    run_using(root,job,command,stdout,stderr,None)
}
/// Retain the exact owned process lifecycle while carrying a borrowed owning
/// mission lease solely to the parent permission writer. No timer, process
/// policy, unknown wait/grant/stream semantics or manual signature changes.
pub(super) fn run_using(root:&RootStore,job:&Job,command:Command,stdout:&mut dyn Write,stderr:&mut dyn Write,lease:Option<&crate::MissionLease>)->Result<Observation,E>{
    if job.phase != Phase::Claimed || job.permission != 0 {
        return Err(E::Refused("review-process-new-claim-required"));
    }
    let material =
        observe(root, job, true)?.ok_or(E::Refused("review-process-material-required"))?;
    let policy = material
        .settings
        .values
        .agent_execution_policy()
        .map_err(E::Settings)?;
    let now = At::now().map_err(|e| caused("review-process-clock", e))?;
    let schedule = Schedule::new(
        policy,
        material.duration,
        now,
        startup_due(&job.preflight_until, now)?,
    )?;
    let (channel, input) =
        UnixStream::pair().map_err(|e| caused("review-process-channel-create", e))?;
    let timer = match channel
        .set_nonblocking(true)
        .map_err(|e| caused("review-process-channel-nonblocking", e))
        .and_then(|_| {
            PlanningBootTimer::new().map_err(|e| caused("review-process-timer-create", e))
        }) {
        Ok(timer) => timer,
        Err(error) => {
            return finished(
                finished(
                    Err(error),
                    nix::unistd::close(channel)
                        .map_err(|e| caused("review-process-channel-close", e)),
                ),
                nix::unistd::close(input).map_err(|e| caused("review-process-input-close", e)),
            )
        }
    };
    let mut control = Control {
        root,
        lease,
        job: job.clone(),
        channel: Some(channel),
        timer: Some(timer),
        schedule,
        gate_bytes: Vec::new(),
        grant_attempted: false,
        interruption: None,
        duration_expired: false,
        max_source_observation: Duration::ZERO,
    };
    let process = run_controlled_command(
        command,
        Stdio::from(OwnedFd::from(input)),
        None,
        stdout,
        Some(stderr),
        Some(&mut control),
    );
    let mut transport_errors = Vec::new();
    let gate = match control.read_gate().and_then(|_| {
        crate::agent_supervision::decode_gate(&control.gate_bytes, control.schedule.gate_eof)
    }) {
        Ok(frame) => frame,
        Err(error) => {
            transport_errors.push(error);
            None
        }
    };
    let close = control
        .channel
        .take()
        .ok_or(E::Refused("review-process-channel-missing"))
        .and_then(|s| nix::unistd::close(s).map_err(|e| caused("review-process-channel-close", e)));
    let channel_closed = close.is_ok();
    if let Err(error) = close {
        transport_errors.push(error);
    }
    if let Some(timer) = control.timer.take() {
        if let Err(error) = timer.close() {
            transport_errors.push(caused("review-process-timer-close", error));
        }
    }
    Ok(Observation {
        job: control.job,
        process,
        grant_attempted: control.grant_attempted,
        interruption: control.interruption,
        duration_expired: control.duration_expired,
        drain_expired: control.schedule.drain_expired,
        gate,
        gate_bytes: control.gate_bytes,
        transport_errors,
        channel_closed,
        #[cfg(test)]
        max_source_observation: control.max_source_observation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Synthetic boot coordinates exercise saved policies without wall sleeps or
    /// a test watchdog. The numeric values are fixture data, not product defaults.
    fn schedule(duration: Option<Duration>, grace: i64, drain: Option<i64>) -> (Schedule, At) {
        let mut policy = crate::EngineSettingsValues::installation_defaults()
            .unwrap()
            .agent_execution_policy()
            .unwrap();
        policy.term_grace_ms = crate::SettingCount(grace);
        policy.output_drain_ms = crate::SettingCap(drain.map(crate::SettingCount));
        let now = At::now().unwrap();
        let mut schedule = Schedule::new(
            policy,
            duration,
            now,
            after(now, Duration::from_secs(1)).unwrap(),
        )
        .unwrap();
        schedule.started(now).unwrap();
        (schedule, now)
    }
    /// Admission expiry is not total duration once private gate EOF is observed;
    /// None duration never inherits a planner/silence/default timeout.
    #[test]
    fn reviewer_process_unlimited_duration_outlives_original_admission() {
        let (mut s, now) = schedule(None, 0, None);
        s.gate_eof = true;
        assert!(s
            .due(after(now, Duration::from_secs(100)).unwrap())
            .unwrap()
            .is_none());
        s.gate_eof = false;
        assert!(matches!(
            s.due(after(now, Duration::from_secs(100)).unwrap()),
            Err(E::Refused("review-process-original-admission-expired"))
        ));
    }
    /// Positive total duration requests TERM then exact saved grace KILL once;
    /// handled deadlines disappear from wakes, including immediate zero grace.
    #[test]
    fn reviewer_process_duration_term_grace_kill_has_no_expired_wake_loop() {
        for grace in [0, 50] {
            let (mut s, now) = schedule(Some(Duration::from_secs(2)), grace, None);
            s.gate_eof = true;
            let due = after(now, Duration::from_secs(2)).unwrap();
            assert_eq!(s.due(due).unwrap(), Some(Signal::TERM));
            let kill = after(due, Duration::from_millis(grace as u64)).unwrap();
            assert_eq!(s.next(), Some(kill));
            assert_eq!(s.due(kill).unwrap(), Some(Signal::KILL));
            assert_eq!(s.next(), None);
            assert_eq!(s.due(kill).unwrap(), None);
        }
    }
    /// The original unlimited schedule must reject a second start too; absence
    /// of a total deadline is not absence of an already observed birth.
    #[test]
    fn reviewer_process_unlimited_clock_cannot_restart() {
        let (mut s, now) = schedule(None, 0, None);
        assert!(matches!(
            s.started(now),
            Err(E::Refused("review-process-clock-already-started"))
        ));
    }
    /// Actual leader completion removes total/startup timers; drain0 is an
    /// explicit incomplete-stream refusal and null installs no drain watchdog.
    #[test]
    fn reviewer_process_leader_completion_preserves_explicit_drain_policy() {
        let (mut s, now) = schedule(Some(Duration::from_secs(1)), 0, None);
        s.exited(now).unwrap();
        assert_eq!(s.next(), None);
        assert!(s
            .due(after(now, Duration::from_secs(100)).unwrap())
            .unwrap()
            .is_none());
        let (mut s, now) = schedule(None, 0, Some(0));
        s.exited(now).unwrap();
        assert!(matches!(
            s.due(now),
            Err(E::Refused("review-process-output-drain-expired"))
        ));
        assert!(s.drain_expired);
    }
}
