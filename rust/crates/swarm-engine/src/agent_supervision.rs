//! Foreground native manual supervision. Only the owned runner can grant signals.
#![cfg(target_os = "linux")]
use crate::{
    agent_execution_preflight::{inspect, ExecutionStage},
    agent_execution_store as store,
    agent_native_stream::NativeStream,
    local_storage::{LocalDirectory, LocalReservation},
    root_store::finished,
    validation_run::{run_controlled_command, OwnedProcessControl},
    Agent, AgentExecutionPolicy, AgentLaunchError, AgentProgress, RootStoreError,
};
use rusqlite::Connection;
use rustix::process::Signal;
use std::{
    cell::RefCell,
    io::{self, Read, Write},
    os::{fd::OwnedFd, unix::net::UnixStream},
    path::Path,
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
/// Public operation receipt: process settlement is not business acceptance.
/// Provider-start uncertainty is retained in the Agent's native result, not
/// inferred from an EOF, successful child exit or stored process identifiers.
#[derive(serde::Serialize)]
pub struct AgentExecutionOutcome {
    pub agent: Agent,
    pub task_accepted: bool,
}
/// Preserve original causes behind safe static runtime codes.
fn root_error(
    code: &'static str,
    e: impl std::error::Error + Send + Sync + 'static,
) -> RootStoreError {
    RootStoreError::Launch(Box::new(AgentLaunchError::caused(code, e)))
}
/// Convert an already classified domain failure into an IO cause for the owned
/// runner. The complete source chain survives cleanup, wait and final settlement.
fn ioe(e: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::other(e)
}
/// Checked monotonic policy deadline. Representation failure is explicit and
/// never becomes an immediate stop or an implicit unbounded fallback.
fn after(at: Instant, ms: i64) -> io::Result<Instant> {
    let ms = u64::try_from(ms).map_err(ioe)?;
    at.checked_add(Duration::from_millis(ms))
        .ok_or_else(|| io::Error::other("agent-execution-deadline-representation"))
}
/// Build cumulative model observations without granting vitality or accepting
/// any task. Elapsed foreground-run time is compared only to the frozen
/// observation threshold; it does not imply provider progress or permission to
/// stop. Milliseconds→seconds and metrics version1 are protocol units.
fn progress(
    stream: &NativeStream,
    duration_threshold: i64,
) -> Result<AgentProgress, RootStoreError> {
    let o = stream.observation().map_err(RootStoreError::Guard)?;
    let elapsed = stream.at().map_err(RootStoreError::Guard)? / 1000;
    Ok(AgentProgress {
        elapsed_seconds: elapsed,
        time_threshold_exceeded: duration_threshold > 0 && elapsed >= duration_threshold,
        tool_wait_seconds: o.oldest_tool_wait_millis / 1000,
        tool_time_threshold_exceeded: o.tool_time_threshold_exceeded,
        tool_calls: o.calls,
        tool_results: o.results,
        pending_tools: o.pending,
        output_state: o.output_state.into(),
        tool_state: o.tool_state.into(),
        provider_vitality: o.provider_vitality.into(),
        degraded: o.visibility.map(|v| format!("{v:?}")).unwrap_or_default(),
        metrics_version: 1,
        tool_reads: o.reads,
        tool_writes: o.writes,
        tool_unclassified: o.unclassified,
        tool_errors_total: o.total_errors,
        tool_repeats_total: o.total_repeats,
        ..Default::default()
    })
}
/// Decode only a fully closed owned helper channel. Empty EOF is ambiguous,
/// not proof of exec. Invalid frames retain codec/refusal causes and cannot
/// authorize either success or a reservation release. No provider text is read.
/// Decode one complete private helper refusal, preserving native dash and
/// Go-compatible snake diagnostic families. Missing EOF/shape/version or unsafe
/// codes refuse with typed causes; empty EOF is ambiguous, never proof of exec,
/// authenticated identity, unused budget or permission to refund/retry.
pub(crate) fn decode_gate(
    raw: &[u8],
    eof: bool,
) -> Result<Option<crate::agent_execution_gate::GateFailure>, RootStoreError> {
    if !eof {
        return Err(RootStoreError::Refused("agent-gate-incomplete"));
    }
    if raw.is_empty() {
        return Ok(None);
    }
    let frame: crate::agent_execution_gate::GateFailure =
        swarm_wire_json::decode(raw).map_err(|e| root_error("agent-gate-frame-invalid", e))?;
    if frame.schema_version != 1
        || frame.kind != "pre_exec_failure"
        || frame.code.is_empty()
        || !frame
            .code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_'))
    {
        return Err(RootStoreError::Refused("agent-gate-frame-unsupported"));
    }
    match frame.causes_scope.as_deref() {
        None if frame.causes.is_empty() => (), // legacy is explicitly unprojected
        Some("known-native-compound-v1")
            if !frame.causes.is_empty()
                && frame
                    .causes
                    .iter()
                    .all(crate::native_diagnostics::NativeGateCause::valid) => {}
        _ => {
            return Err(RootStoreError::Refused(
                "agent-gate-cause-projection-unsupported",
            ))
        }
    }
    Ok(Some(frame))
}
/// Shared stream sink with checked RefCell access. No panics or swallowed sink
/// errors when controller/output borrows conflict; owned cleanup gets the cause.
struct Sink(Rc<RefCell<NativeStream>>);
impl Write for Sink {
    /// Forward bytes without hiding borrow or stream failures.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        self.0.try_borrow_mut().map_err(ioe)?.write(raw)
    }
    /// Forward explicit flush to the same checked stream sink.
    fn flush(&mut self) -> io::Result<()> {
        self.0.try_borrow_mut().map_err(ioe)?.flush()
    }
}
/// One registered actual supervisor and its private child gate. Settings are
/// held stable only until startup release; progress mutations use fresh saved
/// storage policy. No caller-deserialized object constructs this capability.
struct Control<'a> {
    db: &'a mut Connection,
    dir: &'a LocalDirectory,
    root: &'a Path,
    agent: Agent,
    reserved: store::ReservationReceipt,
    policy: AgentExecutionPolicy,
    settings_revision: crate::SettingCount,
    startup: Option<LocalReservation>,
    channel: Option<UnixStream>,
    ack: Vec<u8>,
    ack_eof: bool,
    released: bool,
    stream: Rc<RefCell<NativeStream>>,
    next: Instant,
    stopping: Option<Instant>,
    kill_sent: bool,
    leader_exit: Option<Instant>,
    drain_at: Option<Instant>,
    drain_expired: bool,
    kill_at: Option<Instant>,
    stop_kind: Option<&'static str>,
}
impl Control<'_> {
    /// Drain only the private engine channel, never stdout. EOF alone leaves
    /// startup uncertainty; a validated helper failure permits a pre-exec result.
    fn read_gate(&mut self) -> io::Result<()> {
        if self.ack_eof {
            return Ok(());
        }
        let channel = self
            .channel
            .as_mut()
            .ok_or_else(|| io::Error::other("agent-gate-channel-missing"))?;
        loop {
            let mut chunk = [0u8; 4096];
            match channel.read(&mut chunk) {
                Ok(0) => {
                    self.ack_eof = true;
                    return Ok(());
                }
                Ok(n) => self.ack.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }
    /// Release every remaining startup/socket resource explicitly, preserving
    /// cleanup failures even if the runner failed before registration callbacks.
    fn close(&mut self) -> Result<(), RootStoreError> {
        let lock = self
            .startup
            .take()
            .map_or(Ok(()), |r| r.close().map_err(RootStoreError::Storage));
        let channel = self.channel.take().map_or(Ok(()), |s| {
            nix::unistd::close(s).map_err(|e| root_error("agent-gate-channel-close", e))
        });
        finished(lock, channel)
    }
}
impl OwnedProcessControl for Control<'_> {
    /// Register the actual held child, recheck permission, then and only then
    /// send the private protocol grant. Commit uncertainty never sends a byte.
    fn started(&mut self, pid: u32) -> io::Result<()> {
        let expected = self.agent.clone();
        self.agent =
            crate::agent_storage_retry::execute(self.db, self.dir, true, |db, settings| {
                store::register_child(
                    db,
                    self.root,
                    self.dir,
                    settings,
                    &expected,
                    pid,
                    &self.reserved,
                )
                .map_err(|e| RootStoreError::Launch(Box::new(e)))
            })
            .map_err(ioe)?;
        let expected = self.agent.clone();
        self.agent =
            crate::agent_storage_retry::execute(self.db, self.dir, true, |db, settings| {
                if settings.revision != self.settings_revision
                    || settings
                        .values
                        .agent_execution_policy()
                        .map_err(RootStoreError::Settings)?
                        != self.policy
                {
                    return Err(RootStoreError::Refused(
                        "agent-execution-startup-policy-changed",
                    ));
                }
                store::record_permission(
                    db,
                    self.root,
                    self.dir,
                    settings,
                    &expected,
                    &self.reserved,
                )
                .map_err(|e| RootStoreError::Launch(Box::new(e)))
            })
            .map_err(ioe)?;
        // Writing a single byte may consume permission even if later cleanup
        // fails. Record conservative exposure before attempting the transport.
        self.released = true;
        self.channel
            .as_mut()
            .ok_or_else(|| io::Error::other("agent-gate-channel-missing"))?
            .write_all(&[1])?;
        if let Some(lock) = self.startup.take() {
            lock.close().map_err(ioe)?;
        }
        self.next = Instant::now();
        Ok(())
    }
    /// Start only the configured post-leader drain window. The owned runner has
    /// already signalled the group; no global provider deadline is installed.
    fn leader_exited(&mut self) -> io::Result<()> {
        let at = Instant::now();
        self.leader_exit = Some(at);
        self.drain_at = self
            .policy
            .output_drain_ms
            .0
            .map(|ms| after(at, ms.0))
            .transpose()?;
        Ok(())
    }
    /// Poll and already-authorized TERM/drain deadlines only. Duration, silence
    /// and tool-wait thresholds never create a process deadline.
    fn next_wake(&self) -> Option<Instant> {
        Some(
            [
                Some(self.next),
                self.drain_at,
                if self.kill_sent { None } else { self.kill_at },
            ]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(self.next),
        )
    }
    /// Observe saved stop intent and cumulative stream guard. TERM/KILL target
    /// only the runner-held group; silence/tool time/heartbeat age never stop it.
    /// Before private-channel EOF, refresh owner/stop in a read-only transaction
    /// so this parent does not compete with helper Root bootstrap. Afterwards
    /// heartbeat writes resume. EOF alone still proves neither exec nor health.
    /// Drain expiry is an error with cleanup, never a successful empty result.
    fn observe(&mut self) -> io::Result<Option<Signal>> {
        self.read_gate()?;
        let now = Instant::now();
        if self.drain_at.is_some_and(|deadline| now >= deadline) {
            self.drain_expired = true;
            return Err(io::Error::other("agent-execution-output-drain-expired"));
        }
        let guard_stop = self
            .stream
            .try_borrow()
            .map_err(ioe)?
            .observation()
            .map_err(ioe)?
            .stop_reason
            .is_some();
        if now >= self.next {
            let (p, facts) = {
                let stream = self.stream.try_borrow().map_err(ioe)?;
                (
                    progress(&stream, self.agent.timeout_seconds).map_err(ioe)?,
                    stream.provider_facts(),
                )
            };
            let expected = self.agent.clone();
            self.agent = if self.ack_eof {
                crate::agent_storage_retry::heartbeat(self.db, self.dir, &expected, &p, &facts)
            } else {
                crate::agent_storage_retry::pending_gate_owner(self.db, self.dir, &expected)
            }.map_err(ioe)?;
            self.next = after(Instant::now(), self.policy.poll_interval_ms.0)?;
        }
        if self.stopping.is_none() && (guard_stop || self.agent.desired == "stop") {
            self.stop_kind = Some(if self.agent.desired == "stop" {
                "operateur"
            } else {
                "garde"
            });
            self.kill_at = Some(after(now, self.policy.term_grace_ms.0)?);
            self.stopping = Some(now);
            return Ok(Some(Signal::TERM));
        }
        if let Some(at) = self.stopping {
            if !self.kill_sent && now >= after(at, self.policy.term_grace_ms.0)? {
                self.kill_sent = true;
                return Ok(Some(Signal::KILL));
            }
        }
        Ok(None)
    }
}
/// Run one prepared manual intention through the current CLI image's private
/// helper dispatch. Automatic intentions additionally require canonical Binding4
/// matching the actual directory/DB at preparation and transactional registration.
/// Linux procfs, pidfd, process groups and memfd are required.
/// This function owns the full wait/drain capability; no PID/JSON alone grants
/// signals. Startup refusal before claim mutates nothing. Later failures retain
/// registration/permission facts and settle only after confirmed reap; unresolved
/// ownership stays active for explicit recovery. Provider exit never accepts the
/// task. No paid fixture/model request is introduced by this API itself.
/// The embedding executable MUST implement `_agent_gate` as Swarm CLI does.
/// Engine-produced completion captures are frozen in their persisted wire
/// spelling before journal/Agent comparisons; literal output is never a reason
/// to relax ownership, exact readback or replay guards. Encoding causes survive.
pub(crate) fn run(
    db: &mut Connection,
    root: &Path,
    dir: &LocalDirectory,
    id: &str,
) -> Result<AgentExecutionOutcome, RootStoreError> {
    let startup = dir.reserve(false, false).map_err(RootStoreError::Storage)?;
    let prepared = crate::agent_storage_retry::execute(db, dir, false, |db, settings| {
        let policy = settings
            .values
            .agent_execution_policy()
            .map_err(RootStoreError::Settings)?;
        let mut tx = db.transaction().map_err(RootStoreError::Sql)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = inspect(&tx, root, dir, settings, id, ExecutionStage::Queued)
            .map_err(|e| RootStoreError::Launch(Box::new(e)));
        let prepared = finished(result, tx.rollback().map_err(RootStoreError::Sql))?;
        Ok((prepared, policy, settings.revision))
    });
    let (prepared, policy, settings_revision) = match prepared {
        Ok(v) => v,
        Err(e) => return finished(Err(e), startup.close().map_err(RootStoreError::Storage)),
    };
    let clock = Instant::now();
    let stream = match NativeStream::new(&prepared.agent, policy.clone(), clock) {
        Ok(s) => Rc::new(RefCell::new(s)),
        Err(e) => {
            return finished(
                Err(RootStoreError::Guard(e)),
                startup.close().map_err(RootStoreError::Storage),
            )
        }
    };
    let pair = UnixStream::pair().map_err(|e| root_error("agent-gate-create", e));
    let (channel, input) = match pair {
        Ok(v) => v,
        Err(e) => return finished(Err(e), startup.close().map_err(RootStoreError::Storage)),
    };
    if let Err(e) = channel.set_nonblocking(true) {
        let close_a = nix::unistd::close(channel).map_err(|e| root_error("agent-gate-close", e));
        let close_b = nix::unistd::close(input).map_err(|e| root_error("agent-gate-close", e));
        let closed = finished(close_a, close_b);
        return finished(
            finished(Err(root_error("agent-gate-nonblocking", e)), closed),
            startup.close().map_err(RootStoreError::Storage),
        );
    }
    let claimed = crate::agent_storage_retry::execute(db, dir, true, |db, settings| {
        crate::agent_store::claim_agent_supervisor_bound(db, id, dir, settings)
            .map(|v| v.agent().clone())
            .map_err(RootStoreError::Agent)
    });
    let agent = match claimed {
        Ok(v) => v,
        Err(e) => {
            let close_a =
                nix::unistd::close(channel).map_err(|e| root_error("agent-gate-close", e));
            let close_b = nix::unistd::close(input).map_err(|e| root_error("agent-gate-close", e));
            return finished(
                finished(finished(Err(e), close_a), close_b),
                startup.close().map_err(RootStoreError::Storage),
            );
        }
    };
    let mut command = Command::new("/proc/self/exe");
    command
        .arg("_agent_gate")
        .arg(id)
        .arg("--root")
        .arg(root)
        .arg("--json");
    let mut control = Control {
        db,
        dir,
        root,
        agent,
        reserved: prepared.reserved,
        policy,
        settings_revision,
        startup: Some(startup),
        channel: Some(channel),
        ack: vec![],
        ack_eof: false,
        released: false,
        stream: stream.clone(),
        next: clock,
        stopping: None,
        kill_sent: false,
        leader_exit: None,
        drain_at: None,
        drain_expired: false,
        kill_at: None,
        stop_kind: None,
    };
    let mut sink = Sink(stream.clone());
    let result = run_controlled_command(
        command,
        Stdio::from(OwnedFd::from(input)),
        None,
        &mut sink,
        None,
        Some(&mut control),
    );
    let ack = control
        .read_gate()
        .map_err(|e| root_error("agent-gate-final-read", e));
    let closed = control.close();
    let process_unresolved = result.as_ref().err().is_some_and(|e| {
        e.unresolved_process_id().is_some() || e.ownership_lost_process_id().is_some()
    });
    if process_unresolved {
        return finished(
            finished(
                Err(root_error(
                    "agent-execution-process-unresolved",
                    result
                        .err()
                        .ok_or(RootStoreError::Refused("agent-execution-error-missing"))?,
                )),
                ack,
            ),
            closed,
        );
    }
    let process_ok = result.is_ok();
    let waited = result
        .as_ref()
        .ok()
        .map(|r| r.usage.status)
        .or_else(|| result.as_ref().err().and_then(|e| e.reaped_exit_status()));
    let process_operation = result.as_ref().err().map(|e| e.operation());
    let incomplete_output = !process_ok && control.released;
    let mut lifecycle_recorded = false;
    // Any later borrow/codec/clock/storage refusal still joins the original
    // process and resource-close causes outside this closure. A question-mark
    // here cannot discard the already observed process failure.
    let settled = (|| {
        let finish_stream = {
            let mut s = stream
                .try_borrow_mut()
                .map_err(|e| root_error("agent-stream-borrow", e))?;
            if process_ok {
                s.finish()
            } else {
                s.incomplete()
            }
        }
        .map_err(|e| root_error("agent-stream-final-frame", e));
        let stream = stream
            .try_borrow()
            .map_err(|e| root_error("agent-stream-borrow", e))?;
        let p = progress(&stream, control.agent.timeout_seconds)?;
        let provider_facts = stream.provider_facts();
        let gate_result = decode_gate(&control.ack, control.ack_eof);
        let gate_failure = gate_result.as_ref().ok().and_then(|v| v.as_ref());
        let may_have_started = control.released && gate_failure.is_none();
        let owned_exit_code = waited.and_then(|s| s.code()).map(i64::from);
        // Known helper refusal is not a provider exit. Retain its actual wait
        // code separately while leaving the Agent provider code absent.
        let exit_code = if gate_failure.is_some() {
            None
        } else {
            owned_exit_code
        };
        let failed = waited.is_none_or(|s| !s.success())
            || !process_ok
            || gate_failure.is_some()
            || gate_result.is_err()
            || ack.is_err()
            || closed.is_err()
            || finish_stream.is_err();
        let (capture, capture_invalid_utf8) = stream.captured();
        let facts = serde_json::json!({
            "schema_version":1,"gate_release_attempted":control.released,"provider_may_have_started":may_have_started,
            "private_gate_eof":control.ack_eof,"pre_exec_failure":gate_failure,"private_gate_invalid":gate_result.is_err(),
            "kernel_reaped":waited.is_some(),"recovery_required":incomplete_output,"workspace_release_confirmed":!incomplete_output,
            "incident":if control.drain_expired{Some("output_drain_incomplete")}else if incomplete_output{Some("owned_process_failure")}else{None},
            "exit_code":exit_code,"owned_process_exit_code":owned_exit_code,"process_error":process_operation,
            "output_bytes":stream.output_bytes,"output_sha256":stream.sha256(),"output_complete":process_ok&&finish_stream.is_ok(),
            "capture":capture,"capture_invalid_utf8":capture_invalid_utf8,"capture_truncated":stream.capture_truncated,
            "public_messages_truncated":stream.public_truncated,"first_event_error":stream.first_event_error.as_ref().map(|e|e.code()),
            "event_source":"provider_declared_unverified","provider_facts":&provider_facts,"stop_kind":control.stop_kind,"observation":stream.observation().map_err(RootStoreError::Guard)?
        });
        // Freeze the same escaped wire bytes that exact persistence/replay will
        // read, including literal provider capture text. Never relax readback.
        let raw = crate::model_wire::persisted_raw(&facts)
            .map_err(|e| root_error("agent-execution-result-encoding", e))?;
        let reply = stream.public_messages.join("\n");
        let completion_facts =
            crate::agent_completion_journal::CompletionFacts::observe(&provider_facts)
                .map_err(|e| RootStoreError::Launch(Box::new(e)))?;
        let receipt = store::Completion {
            exit_code,
            failed,
            interrupted: control.stopping.is_some(),
            stop_kind: control.stop_kind,
            reserved: &control.reserved,
            may_have_started,
            progress: &p,
            details: &raw,
            public_reply: &reply,
            facts: &completion_facts,
        };
        let expected = control.agent.clone();
        let journal =
            crate::agent_completion_journal::persist(dir, &expected, &receipt, incomplete_output);
        let recorded =
            crate::agent_storage_retry::execute(control.db, dir, true, |db, settings| {
                if incomplete_output {
                    store::quarantine(db, &expected, &receipt)
                } else {
                    store::complete(db, root, settings, &expected, &receipt)
                }
                .map_err(|e| RootStoreError::Launch(Box::new(e)))
            });
        lifecycle_recorded = recorded.is_ok();
        finished(
            finished(finished(recorded, journal), finish_stream),
            gate_result.map(|_| ()),
        )
    })();
    let operation = finished(
        finished(
            result
                .map(|_| ())
                .map_err(|e| root_error("agent-execution-process", e)),
            ack,
        ),
        closed,
    );
    let final_result = finished(settled, operation);
    if lifecycle_recorded {
        if let Err(primary) = final_result {
            return Err(RootStoreError::LifecycleRecorded {
                primary: Box::new(primary),
            });
        }
    }
    let agent = final_result?;
    if incomplete_output {
        return Err(RootStoreError::Refused("agent-execution-recovery-required"));
    }
    Ok(AgentExecutionOutcome {
        agent,
        task_accepted: false,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Real controller/SQLite effects: completed gate permits a valid pulse,
    /// pending gate must preserve DB bytes/logs, ownership and stop observation.
    /// Synthetic lifecycle rows carry this test process's real owner stamp but
    /// grant no execution; no child/provider or real mission is started.
    #[test]
    fn pending_gate_observes_stop_without_heartbeat_writes() {
        use crate::{AgentExecutionSnapshot, AgentExecutionPhase, AgentGuardSnapshot,
            EngineSettingsValues, SettingCount, SettingCap, SystemEnvironment};
        use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt};
        let root=std::env::temp_dir().join(crate::generate_identifier("gate-pulse-").unwrap());
        fs::create_dir(&root).unwrap();fs::set_permissions(&root,fs::Permissions::from_mode(0o700)).unwrap();
        let store=crate::RootStore::initialize(&root,&mut SystemEnvironment).unwrap();
        let settings=store.settings().unwrap();store.close().unwrap();
        let dir=LocalDirectory::open_owned(&root.join(".swarm")).unwrap();
        let policy=EngineSettingsValues::installation_defaults().unwrap().agent_execution_policy().unwrap();
        let identity=crate::SupervisorIdentity::current().unwrap();
        let metadata=AgentExecutionSnapshot {schema_version:1,settings_revision:settings.revision,
            work_revision:1,context_sha256:"0".repeat(64),phase:AgentExecutionPhase::PermissionRecorded,policy:policy.clone()};
        let agent=Agent {id:"fixture-agent".into(),work_id:"fixture-work".into(),task_id:"task".into(),
            attempt_id:"attempt".into(),workspace:root.to_string_lossy().into_owned(),status:"running".into(),
            mode:"pipe".into(),provider:"fixture".into(),role:"worker".into(),
            supervisor_pid:identity.pid(),supervisor_identity:identity.process_stamp().into(),host:identity.host().into(),
            heartbeat:"before".into(),timeout_seconds:1800,
            limits:crate::RunLimits {silence_seconds:180,tool_seconds:300,max_tool_calls:100,max_repeated_calls:4,max_consecutive_errors:3,observation_mode:0},
            native_execution:Some(crate::GoRaw(serde_json::value::to_raw_value(&metadata).unwrap())),
            native_tool_guard_policy:Some(Box::new(AgentGuardSnapshot {schema_version:1,settings_revision:"1".into(),lookback_multiplier:SettingCap(Some(SettingCount(4))),extensions:BTreeMap::new()})),
            ..Agent::default()};
        let mut db=Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE agents(id TEXT PRIMARY KEY,work_id TEXT,task_id TEXT,cwd TEXT,status TEXT,desired TEXT,body BLOB); CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY AUTOINCREMENT,agent_id TEXT,at TEXT,kind TEXT,message TEXT); PRAGMA user_version=37;").unwrap();
        let raw=crate::encode_model(&agent).unwrap();
        db.execute("INSERT INTO agents VALUES(?1,?2,?3,?4,?5,?6,?7)",rusqlite::params![agent.id,agent.work_id,agent.task_id,agent.workspace,agent.status,agent.desired,raw]).unwrap();
        let stream=Rc::new(RefCell::new(NativeStream::new(&agent,policy.clone(),Instant::now()).unwrap()));
        let (channel,peer)=UnixStream::pair().unwrap();channel.set_nonblocking(true).unwrap();
        let mut control=Control {db:&mut db,dir:&dir,root:&root,agent,reserved:None,policy,settings_revision:settings.revision,
            startup:None,channel:Some(channel),ack:Vec::new(),ack_eof:true,released:true,stream,next:Instant::now(),
            stopping:None,kill_sent:false,leader_exit:None,drain_at:None,drain_expired:false,kill_at:None,stop_kind:None};
        // Valid normal pulse control before the pending-channel assertion.
        assert!(control.observe().unwrap().is_none());
        assert_ne!(control.agent.heartbeat,"before");
        assert_eq!(control.db.query_row("SELECT count(*) FROM agent_logs",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        let before=crate::read_agent(control.db,"fixture-agent").unwrap();
        let changes=control.db.total_changes();
        control.ack_eof=false;control.next=Instant::now();
        assert!(control.observe().unwrap().is_none());
        assert_eq!(control.db.total_changes(),changes,"pending helper must not race a parent write transaction");
        assert!(crate::read_agent(control.db,"fixture-agent").unwrap()==before);
        // Invalid owner remains refused in the read-only path, then recovers.
        let mut forged=before.clone();forged.supervisor_pid+=1;
        control.db.execute("UPDATE agents SET body=?1 WHERE id=?2",rusqlite::params![crate::encode_model(&forged).unwrap(),forged.id]).unwrap();
        control.next=Instant::now();assert!(control.observe().is_err());
        control.db.execute("UPDATE agents SET body=?1 WHERE id=?2",rusqlite::params![crate::encode_model(&before).unwrap(),before.id]).unwrap();
        control.next=Instant::now();assert!(control.observe().unwrap().is_none());
        // Private EOF ends the helper phase; it never proves actual exec.
        nix::unistd::close(peer).unwrap();control.next=Instant::now();
        let writes=control.db.total_changes();
        assert!(control.observe().unwrap().is_none());assert!(control.ack_eof);
        assert_eq!(control.db.total_changes(),writes+1);
        let (channel,peer)=UnixStream::pair().unwrap();channel.set_nonblocking(true).unwrap();
        nix::unistd::close(control.channel.take().unwrap()).unwrap();control.channel=Some(channel);control.ack_eof=false;
        control.db.execute("UPDATE agents SET desired='stop' WHERE id='fixture-agent'",[]).unwrap();
        let changes=control.db.total_changes();control.next=Instant::now();
        assert_eq!(control.observe().unwrap(),Some(Signal::TERM));
        assert_eq!(control.db.total_changes(),changes);
        assert_eq!(control.stop_kind,Some("operateur"));
        control.close().unwrap();nix::unistd::close(peer).unwrap();
        drop(control);db.close().unwrap();dir.close().unwrap();fs::remove_dir_all(root).unwrap();
    }
    /// Valid structured helper refusal precedes malformed/future/private-content
    /// injections. Empty EOF stays ambiguous; invalid data never releases budget.
    #[test]
    fn private_gate_diagnostics_are_strict_and_empty_eof_is_ambiguous() {
        let valid=br#"{"schema_version":1,"kind":"pre_exec_failure","code":"agent-provider-exec","io_errno":8,"sqlite_code":null}"#;
        assert_eq!(decode_gate(valid, true).unwrap().unwrap().io_errno, Some(8));
        assert!(decode_gate(b"", true).unwrap().is_none());
        for raw in [b"PRIVATE secret".as_slice(),b"[]",br#"{"schema_version":2,"kind":"pre_exec_failure","code":"x"}"#,br#"{"schema_version":1,"kind":"pre_exec_failure","code":"PRIVATE secret"}"#,br#"{"schema_version":1,"kind":"pre_exec_failure","code":"x","prompt":"PRIVATE secret"}"#]{
            let error=decode_gate(raw,true).unwrap_err();assert!(!error.to_string().contains("PRIVATE"));
        }
        assert!(decode_gate(valid, false).is_err());
    }
    /// The real gate producer emits both native-dash and Go-compatible snake
    /// code families. Each valid dash control precedes a snake family; decoding
    /// a refusal must retain its code without granting execution or a refund.
    #[test]
    fn private_gate_accepts_actual_producer_snake_case_refusals() {
        for code in [
            "stale_lease",
            "revision_conflict",
            "storage_version_conflict",
        ] {
            let native = crate::agent_execution_gate::diagnostic(&RootStoreError::Refused(
                "planning-call-binding-required",
            ));
            let raw = serde_json::to_vec(&native).unwrap();
            assert_eq!(decode_gate(&raw, true).unwrap().unwrap().code, native.code);
            let frame = crate::agent_execution_gate::diagnostic(&RootStoreError::Refused(code));
            let raw = serde_json::to_vec(&frame).unwrap();
            assert_eq!(decode_gate(&raw, true).unwrap().unwrap().code, code);
        }
    }
    /// A real single-error producer is the control before compound cleanup.
    /// The private wire must preserve both numeric causes without private text;
    /// decoding still denotes a refusal, never execution or refund authority.
    #[test]
    fn private_gate_preserves_both_compound_os_causes_without_private_text() {
        let cause = |code, errno| {
            RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused(
                code,
                io::Error::from_raw_os_error(errno),
            )))
        };
        let access = rustix::io::Errno::ACCESS.raw_os_error();
        let space = rustix::io::Errno::NOSPC.raw_os_error();
        let simple =
            crate::agent_execution_gate::diagnostic(&cause("planning-source-read", access));
        assert_eq!(simple.io_errno, Some(access));
        let combined = RootStoreError::Cleanup {
            primary: Some(Box::new(cause("planning-source-read", access))),
            cleanup: Box::new(cause("planning-resource-close", space)),
        };
        let frame = crate::agent_execution_gate::diagnostic(&combined);
        let raw = serde_json::to_vec(&frame).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let nodes = value
            .get("causes")
            .and_then(|v| v.as_array())
            .expect("Compound causes missing from private wire");
        let numbers = nodes
            .iter()
            .filter_map(|n| n.get("io_errno").and_then(|v| v.as_i64()))
            .collect::<Vec<_>>();
        assert!(
            numbers.contains(&i64::from(access)) && numbers.contains(&i64::from(space)),
            "A cleanup cause was discarded"
        );
        assert_eq!(
            decode_gate(&raw, true).unwrap().unwrap().code,
            "root-cleanup"
        );
        let private = cause("planning-source-read", access);
        let wrapped = RootStoreError::Cleanup {
            primary: Some(Box::new(private)),
            cleanup: Box::new(RootStoreError::Launch(Box::new(
                crate::AgentLaunchError::caused(
                    "planning-resource-close",
                    io::Error::other("PRIVATE path and context"),
                ),
            ))),
        };
        let raw = serde_json::to_vec(&crate::agent_execution_gate::diagnostic(&wrapped)).unwrap();
        assert!(!String::from_utf8(raw).unwrap().contains("PRIVATE"));
    }
}
