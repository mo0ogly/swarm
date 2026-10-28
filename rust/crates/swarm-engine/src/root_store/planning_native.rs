//! Foreground planner lifecycle. Input JSON declares fences, never exec authority.
use super::{finished, RootStore, RootStoreError as E};
#[path = "planning_rejection_feedback.rs"]
mod rejection_feedback;
#[path = "planning_run_binding.rs"]
mod run_binding;
use crate::planning_run_material::{self, Material};
use crate::planning_run_store::{self as runs, Phase, Run};
use crate::planning_run_usage::{self as usage, UsageRecord};
use crate::validation_run::{run_controlled_command, OwnedProcessControl};
use crate::work_engine::{MutationFence, MutationFenceReceipt, WorkEngineError};
use crate::{AgentLaunchError as A, PlanningCallPreviewRequest, Work};
use rustix::process::Signal;
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    convert::Infallible,
    error::Error,
    io::{self, Read, Write},
    os::{
        fd::{AsFd, BorrowedFd, OwnedFd},
        unix::{net::UnixStream, process::ExitStatusExt},
    },
    path::Path,
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

/// Internal evidence disposition; never an operator retry or execution policy.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RefusalDisposition {
    Business,
    ReapedFailure,
    UnadmittedFailure,
}

/// A recorded decision is not worker execution, acceptance, invoice or a global
/// completion receipt. This projection contains no prompt or private host stamps.
#[derive(Serialize)]
pub struct PlanningRunOutcome {
    pub schema_version: i64,
    pub claim_id: String,
    pub decision_event: String,
    pub decision_applied: bool,
    pub accepted: bool,
    pub replayed: bool,
    pub work: Work,
}
/// Join a known decision result with descriptor release, retaining both errors.
/// No IO, retry or new mutation occurs; callers must distinguish returned Work
/// state from acceptance and uncertain failed transactions.
fn planning_result_finished(
    result: Result<PlanningRunOutcome, E>,
    cleanup: Result<(), E>,
) -> Result<PlanningRunOutcome, E> {
    let known = result
        .as_ref()
        .ok()
        .map(|v| (v.work.id.clone(), v.work.revision));
    finished(result, cleanup).map_err(|primary| match known {
        Some((work, revision)) => E::WorkRecorded {
            primary: Box::new(primary),
            work,
            revision,
        },
        None => primary,
    })
}
/// Combine original refusal with delivery outcome, retaining observed causes.
/// No IO, retry or acceptance occurs; the scope may already have new Work state.
fn feedback_error(primary: E, result: Result<Option<Work>, E>) -> E {
    match result {
        Ok(Some(work)) => E::WorkRecorded {
            primary: Box::new(primary),
            work: work.id,
            revision: work.revision,
        },
        Ok(None) => primary,
        Err(cleanup) => E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        },
    }
}
/// Preserve a stop request alongside independent process/gate failures. Only a
/// pure stop without another failure can become a clean recorded interruption;
/// actual gate rejection must never be masked by a coincident local stop.
fn stopped_failure(mut failure:Option<E>,stop:Option<&'static str>,gate_refused:bool)->Option<E>{
    for reason in [stop,gate_refused.then_some("planning-provider-gate-refused")].into_iter().flatten(){
        let next=E::Refused(reason);
        failure=Some(match failure{None=>next,Some(primary)=>E::Cleanup{primary:Some(Box::new(primary)),cleanup:Box::new(next)}});
    }
    failure
}
/// Create-only private evidence; checksums do not authenticate a malicious
/// same-UID writer or complete Root rollback. Recovery checks owner absence
/// and any registered child separately; no child tuple never proves no child.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: i64,
    run: Run,
    finished_at: String,
    exit_code: Option<i32>,
    signal: Option<i32>,
    output_complete: bool,
    proposal: Option<String>,
    usage: UsageRecord,
    facts: crate::GoRaw,
    failure: Option<String>,
    /// Older receipts have unknown grant observation/causes, never false/empty.
    #[serde(default)]
    grant_attempted: Option<bool>,
    #[serde(default)]
    failure_causes: Option<Vec<crate::native_diagnostics::NativeGateCause>>,
    gate: Option<crate::agent_execution_gate::GateFailure>,
    stderr: Vec<u8>,
    stderr_truncated: bool,
}
impl Receipt {
    /// Private runtime evidence of no grant attempt, combined with its original
    /// unadmitted row. Unknown legacy observation refuses; a missing child tuple
    /// alone proves nothing. Caller must revalidate the actual stored row and
    /// atomically revoke it. This proves neither no fork nor child termination,
    /// identity authenticity, financial refund, nor permission for a new call.
    fn unadmitted(&self) -> bool {
        self.grant_attempted == Some(false)
            && self.run.phase == Phase::Claimed
            && self.run.permission == 0
            && self.run.child_pid.is_none()
            && self.run.parent_pid.is_none()
            && self.run.group_id.is_none()
            && self.run.child_stamp.is_none()
    }
}
/// Retain original typed causes behind a static safe root execution family.
fn caused(code: &'static str, cause: impl Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, cause)))
}
/// Preserve an existing native preparation/storage refusal without flattening it.
fn launch(error: A) -> E {
    E::Launch(Box::new(error))
}
/// Engine-controlled receipt basename uses a full claim digest, never an unbounded
/// business ID/path. This is file-format addressing, not authorization or a limit.
fn receipt_name(claim: &str) -> String {
    format!(
        "native-planning-{}.receipt",
        crate::agent_launch_preflight::hash(claim.as_bytes())
    )
}
/// Construct current metadata only from a recorded run and actual Work revision.
/// This does not recreate the model's input batch or extend its original lease.
fn request(row: &Run, revision: i64) -> PlanningCallPreviewRequest {
    PlanningCallPreviewRequest {
        schema_version: 1,
        expected_revision: revision,
        claim_id: row.claim_id.clone(),
        scope: row.scope_id.clone(),
        scope_revision: row.scope_revision,
        holder: row.holder.clone(),
        generation: row.generation,
    }
}
/// Compare all source/owner/child/permission bytes, allowing only terminal fields
/// to differ on replay. The receipt cannot introduce a different physical owner.
fn same_record(row: &Run, receipt: &Run) -> bool {
    let origin = |mut value: Run| {
        value.phase = Phase::Claimed;
        value.receipt_sha256 = None;
        value.applied_revision = None;
        value.refusal_code = None;
        value
    };
    origin(row.clone()) == origin(receipt.clone())
}
/// Require the current actual kernel owner; PID alone and supplied holder names
/// never suffice. Host/starttime changes refuse without signalling another PID.
fn current_owner(row: &Run) -> Result<(), E> {
    let owner = crate::SupervisorIdentity::current()
        .map_err(|e| caused("planning-run-owner-observation", e))?;
    if owner.pid() != row.owner_pid
        || owner.process_stamp() != row.owner_stamp
        || owner.host() != row.owner_host
    {
        return Err(E::Refused("planning-run-owner-changed"));
    }
    Ok(())
}
/// Recheck source identity, original permission and live scope at each actual
/// persisted-policy observation. Zombie/absence is left to the owned pidfd/wait,
/// never converted into a fabricated successful child exit or stale signal right.
fn live(store: &RootStore, row: &Run) -> Result<Work, E> {
    current_owner(row)?;
    run_binding::check(&store.db, &store.directory, &row.claim_id, false).map_err(launch)?;
    let actual = runs::load(&store.db, &row.claim_id).map_err(launch)?;
    if actual != *row {
        return Err(E::Refused("planning-run-state-changed"));
    }
    if store.directory.identity().map_err(E::Storage)?
        != (
            row.directory_device
                .parse()
                .map_err(|e| caused("planning-run-root-identity", e))?,
            row.directory_inode
                .parse()
                .map_err(|e| caused("planning-run-root-identity", e))?,
        )
    {
        return Err(E::Refused("planning-run-root-changed"));
    }
    let work = store.read_work(&row.work_id)?;
    let at = crate::system_timestamp().map_err(E::Runtime)?;
    let r = crate::PlanningRequest {
        schema_version: 1,
        scope: row.scope_id.clone(),
        scope_revision: row.scope_revision,
        holder: row.holder.clone(),
        generation: row.generation,
        ..Default::default()
    };
    crate::planning_decision::fence(&work, &r, &at).map_err(launch)?;
    super::require_planning_mission_active(&store.db, &row.work_id)
        .map_err(|e| caused(e.code(), e))?;
    if let Some(pid) = row.child_pid {
        match crate::observe_linux_process(
            pid.try_into()
                .map_err(|e| caused("planning-run-child-representation", e))?,
        ) {
            Ok(child)
                if child.process_stamp()
                    == row
                        .child_stamp
                        .as_deref()
                        .ok_or(E::Refused("planning-run-child-stamp-required"))?
                    && i64::from(child.parent_pid()) == row.owner_pid
                    && i64::from(child.process_group_id()) == pid => {}
            Ok(_) => return Err(E::Refused("planning-run-child-changed")),
            Err(error) if error.confirms_absence() => (),
            Err(error) => return Err(caused("planning-run-child-observation", error)),
        }
    }
    Ok(work)
}
impl RootStore {
    /// Prepare complete claim/model/host observations under the owning read TX and
    /// current saved policy. This starts no process and holds no provider lock on
    /// return; original freshness and a committed private gate are still required.
    fn planner_material(
        &mut self,
        work: &str,
        r: &PlanningCallPreviewRequest,
    ) -> Result<Material, E> {
        let root = &self.root;
        let directory = &self.directory;
        crate::agent_storage_retry::execute(&mut self.db, directory, false, |db, settings| {
            let mut tx = db.transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = planning_run_material::prepare(&tx, root, directory, settings, work, r)
                .map_err(launch);
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }
    /// Register exactly one actual owner before any child is spawned. Existing
    /// claim/run or orphan receipt refuses before inference, never a paid retry.
    fn claim_planner(&mut self, m: &Material, r: &PlanningCallPreviewRequest, binding: &super::RootBinding) -> Result<Run, E> {
        if !binding.matches(self)? { return Err(E::Refused("planning-run-instance-changed")); }
        let instance = binding.instance()?;
        let owner = crate::SupervisorIdentity::current()
            .map_err(|e| caused("planning-run-owner-observation", e))?;
        let directory = self.directory.identity().map_err(E::Storage)?;
        let row = Run {
            claim_id: r.claim_id.clone(),
            work_id: m.inputs.preview.work_id.clone(),
            scope_id: r.scope.clone(),
            holder: r.holder.clone(),
            generation: r.generation,
            scope_revision: r.scope_revision,
            prepared_revision: m.inputs.preview.work_revision,
            settings_revision: m.settings.revision.0,
            owner_pid: owner.pid(),
            owner_stamp: owner.process_stamp().into(),
            owner_host: owner.host().into(),
            child_pid: None,
            parent_pid: None,
            group_id: None,
            child_stamp: None,
            phase: Phase::Claimed,
            permission: 0,
            host_scope: m.host.preflight.scope.clone(),
            preflight_until: m.host.preflight.valid_until.clone(),
            context_sha256: m.inputs.preview.context_sha256.clone(),
            workflow_prompt_sha256: m.inputs.preview.workflow_prompt_sha256.clone(),
            prompt_sha256: crate::agent_launch_preflight::hash(&m.prompt),
            provider_digest: m.inputs.model.provider_digest.clone(),
            directory_device: directory.0.to_string(),
            directory_inode: directory.1.to_string(),
            decision_event: crate::generate_identifier("planning-decision-").map_err(E::Runtime)?,
            prepared_at: crate::system_timestamp().map_err(E::Runtime)?,
            receipt_sha256: None,
            applied_revision: None,
            refusal_code: None,
        };
        let file = receipt_name(&row.claim_id);
        if self
            .directory
            .names()
            .map_err(E::Storage)?
            .iter()
            .any(|n| n.as_slice() == file.as_bytes())
        {
            return Err(E::Refused("planning-run-receipt-already-exists"));
        }
        crate::agent_storage_retry::execute(&mut self.db, &self.directory, false, |db, _| {
            crate::agent_execution_store::transaction(db, |tx| {
                runs::schema(tx, true)?;
                let count: i64 = crate::agent_launch::query(
                    tx,
                    "SELECT count(*) FROM native_planning_runs WHERE claim_id=?",
                    [&row.claim_id],
                    |rows| {
                        Ok(rows
                            .next()?
                            .ok_or(A::Refused("planning-run-observation"))?
                            .get(0)?)
                    },
                )?;
                if count != 0 {
                    return Err(A::Refused("planning-run-already-recorded"));
                }
                usage::initialize(tx)?;
                runs::insert(tx, &row)?;
                run_binding::stage(tx, &self.directory, &row.claim_id, &instance)?;
                Ok(())
            })
            .map_err(launch)
        })?;
        Ok(row)
    }
    /// Persist the observed child tuple before permission, then persist permission
    /// in a separate checked transaction. Neither stage writes a grant byte; the
    /// caller releases its private socket only after both commits and live guards.
    /// Track a successful commit before settings release, so a later close error
    /// retains the actual tuple for cleanup/receipt, without issuing a grant.
    fn register_planner(&mut self, row: &mut Run, pid: u32) -> Result<(), E> {
        live(self, row)?;
        let child = crate::observe_linux_process(pid)
            .map_err(|e| caused("planning-run-child-observation", e))?;
        if i64::from(child.parent_pid()) != row.owner_pid || child.process_group_id() != pid {
            return Err(E::Refused("planning-run-child-not-owned"));
        }
        let mut registered = row.clone();
        registered.phase = Phase::Registered;
        registered.child_pid = Some(i64::from(pid));
        registered.parent_pid = Some(i64::from(child.parent_pid()));
        registered.group_id = Some(i64::from(child.process_group_id()));
        registered.child_stamp = Some(child.process_stamp().into());
        crate::agent_storage_retry::execute(&mut self.db, &self.directory, false, |db, _| {
            let committed = crate::agent_execution_store::transaction(db, |tx| {
                runs::transition(tx, row, &registered)
            });
            if committed.is_ok() {
                *row = registered.clone();
            }
            committed.map_err(launch)
        })?;
        live(self, row)?;
        let mut permitted = row.clone();
        permitted.phase = Phase::Permitted;
        permitted.permission = 1;
        crate::agent_storage_retry::execute(&mut self.db, &self.directory, false, |db, _| {
            let committed = crate::agent_execution_store::transaction(db, |tx| {
                runs::transition(tx, row, &permitted)
            });
            if committed.is_ok() {
                *row = permitted.clone();
            }
            committed.map_err(launch)
        })?;
        Ok(())
    }
    /// Reconstruct current private gate observations from committed permission and
    /// this actual child, not JSON identities. Original root/host/prompt/TTL fences
    /// are rechecked; Root must close before immutable exec transport is consumed.
    fn prepare_planning_gate(&mut self, claim: &str, peer: i64) -> Result<(Material, Run), E> {
        let row = runs::load(&self.db, claim).map_err(launch)?;
        run_binding::check(&self.db, &self.directory, claim, false).map_err(launch)?;
        let actual = crate::SupervisorIdentity::current()
            .map_err(|e| caused("planning-gate-self-observation", e))?;
        if row.phase != Phase::Permitted
            || row.permission != 1
            || row.child_pid != Some(actual.pid())
            || row.child_stamp.as_deref() != Some(actual.process_stamp())
            || row.owner_host != actual.host()
            || peer != row.owner_pid
        {
            return Err(E::Refused("planning-gate-permission-or-identity-invalid"));
        }
        let child = crate::observe_linux_process(
            actual
                .pid()
                .try_into()
                .map_err(|e| caused("planning-gate-pid", e))?,
        )
        .map_err(|e| caused("planning-gate-self-observation", e))?;
        if i64::from(child.parent_pid()) != row.owner_pid
            || Some(i64::from(child.process_group_id())) != row.group_id
        {
            return Err(E::Refused("planning-gate-parent-or-group-changed"));
        }
        let parent = crate::observe_linux_process(
            row.owner_pid
                .try_into()
                .map_err(|e| caused("planning-gate-parent-pid", e))?,
        )
        .map_err(|e| caused("planning-gate-parent-observation", e))?;
        if parent.process_stamp() != row.owner_stamp {
            return Err(E::Refused("planning-gate-parent-changed"));
        }
        let work = self.read_work(&row.work_id)?;
        let material = self.planner_material(&row.work_id, &request(&row, work.revision))?;
        if material.settings.revision.0 != row.settings_revision
            || material.host.preflight.scope != row.host_scope
            || crate::agent_launch_preflight::hash(&material.prompt) != row.prompt_sha256
            || material.inputs.preview.context_sha256 != row.context_sha256
            || material.inputs.preview.workflow_prompt_sha256 != row.workflow_prompt_sha256
            || material.inputs.model.provider_digest != row.provider_digest
        {
            return Err(E::Refused("planning-gate-preparation-changed"));
        }
        if self.directory.identity().map_err(E::Storage)?
            != (
                row.directory_device
                    .parse()
                    .map_err(|e| caused("planning-gate-root-identity", e))?,
                row.directory_inode
                    .parse()
                    .map_err(|e| caused("planning-gate-root-identity", e))?,
            )
        {
            return Err(E::Refused("planning-gate-root-changed"));
        }
        Ok((material, row))
    }
}
/// Compare original preflight and current lease with a real UTC observation;
/// a newly constructed host observation cannot refresh recorded permission.
fn original_fresh(material: &Material, row: &Run) -> Result<(), E> {
    let now = crate::system_timestamp().map_err(E::Runtime)?;
    let parse = |v: &str| {
        swarm_wire_json::go_rfc3339::parse(v).map_err(|e| caused("planning-gate-clock", e))
    };
    if parse(&now)? >= parse(&row.preflight_until)?
        || parse(&now)? >= parse(&material.inputs.preview.lease_until)?
    {
        return Err(E::Refused("planning-gate-permission-expired"));
    }
    Ok(())
}
/// Private foreground CLI helper only. Kernel parent-death and peer credentials
/// precede grant consumption; registered PID/starttime/parent/group and committed
/// permission precede exec. Empty EOF is not execution proof. No OS confinement,
/// cryptographic identity, descendant termination or business acceptance implied.
/// Shared raw credential validation rejects PID0/invalid UID-GID before grant or Root use.
pub fn execute_planning_gate(root: &Path, claim: &str) -> Result<Infallible, E> {
    if !swarm_identity_contract::name(claim.as_bytes()) {
        return Err(E::Refused("planning-gate-claim-invalid"));
    }
    let fd = rustix::io::fcntl_dupfd_cloexec(io::stdin(), 3)
        .map_err(|e| caused("planning-gate-channel-duplicate", e))?;
    let mut channel = UnixStream::from(fd);
    if let Err(error) = channel.peer_addr() {
        return finished(
            Err(caused("planning-gate-channel-kind", error)),
            nix::unistd::close(channel).map_err(|e| caused("planning-gate-channel-close", e)),
        );
    }
    let operation = (|| {
        crate::supervisor_identity::arm_parent_death_signal()
            .map_err(|e| caused("planning-gate-parent-death", e))?;
        let peer = crate::linux_peer_credentials::read(&channel)
            .map_err(|e| caused("planning-gate-peer-credentials", e))?;
        if peer.uid() != rustix::process::geteuid().as_raw() {
            return Err(E::Refused("planning-gate-peer-user-changed"));
        }
        let mut grant = [0u8; 1];
        io::stdin()
            .read_exact(&mut grant)
            .map_err(|e| caused("planning-gate-input", e))?;
        if grant != [1] {
            return Err(E::Refused("planning-gate-release-invalid"));
        }
        let mut store = RootStore::open(root, false)?;
        let (material, row) = finished(
            store.prepare_planning_gate(claim, i64::from(peer.pid())),
            store.close(),
        )?;
        let transport = crate::native_provider_exec::PreparedExec::prepare(
            material.model.provider(),
            &material.host.environment_names,
            &material.host.workspace,
            &material.prompt,
        )?;
        if let Err(error) = material
            .fresh(root)
            .map_err(launch)
            .and_then(|_| original_fresh(&material, &row))
        {
            return finished(Err(error), transport.close());
        }
        // LIVE-UX telemetry: ingest subject `planning:<claim>` (see reviewer gate).
        let telemetry = match crate::launch_telemetry_environment(
            root,
            &material.settings.values,
            &format!("planning:{claim}"),
            &material.launch.provider,
            crate::telemetry_endpoint::now_ms(),
        ) {
            Ok(t) => t,
            Err(error) => return finished(Err(caused("planning-telemetry-setup-failed", error)), transport.close()),
        };
        transport.add_environment(telemetry.environment)?.add_leading_arguments(telemetry.arguments)?.execute()
    })();
    let error = match operation {
        Ok(never) => match never {},
        Err(error) => error,
    };
    let encoded = serde_json::to_vec(&crate::agent_execution_gate::diagnostic(&error))
        .map_err(|e| caused("planning-gate-diagnostic-encoding", e));
    let report = encoded.and_then(|raw| {
        channel
            .write_all(&raw)
            .map_err(|e| caused("planning-gate-diagnostic-write", e))
    });
    let report = finished(
        report,
        nix::unistd::close(channel).map_err(|e| caused("planning-gate-channel-close", e)),
    );
    finished(Err(error), report)
}

/// Private output retains complete parsed proposal state and a shared actual-byte
/// activity clock. Only the owner feeds this sink; no metadata grants authority.
struct OutputSink {
    stream: Rc<RefCell<crate::PlanningOutputStream>>,
    activity: Rc<Cell<crate::PlanningBootInstant>>,
}
impl Write for OutputSink {
    /// Consume actual stdout bytes under saved parser caps, propagating borrow/IO
    /// failures to owned cleanup. Empty writes never simulate provider progress.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if !raw.is_empty() {
            self.activity.set(crate::PlanningBootInstant::now()?);
        }
        self.stream
            .try_borrow_mut()
            .map_err(io::Error::other)?
            .write(raw)
    }
    /// A flush is not EOF, provider completion or permission to apply a decision.
    fn flush(&mut self) -> io::Result<()> {
        self.stream
            .try_borrow_mut()
            .map_err(io::Error::other)?
            .flush()
    }
}
/// Diagnostic stderr uses the same explicit saved capture caps as workers. Raw
/// truncation does not invent successful output or alter unverified usage facts.
struct ErrorSink {
    activity: Rc<Cell<crate::PlanningBootInstant>>,
    policy: crate::AgentExecutionPolicy,
    bytes: Vec<u8>,
    lines: u64,
    open_line: bool,
    truncated: bool,
}
impl Write for ErrorSink {
    /// Count logical lines across chunks, capture only explicitly admitted bytes,
    /// and refresh silence only on actual bytes. No UTF8 replacement or hidden cap.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if !raw.is_empty() {
            self.activity.set(crate::PlanningBootInstant::now()?);
        }
        let fits = |cap: crate::SettingCap, n: u128| match cap.0 {
            None => true,
            Some(limit) => n <= limit.0 as u128,
        };
        for byte in raw {
            if !self.open_line {
                self.lines = self
                    .lines
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("planning-stderr-line-representation"))?;
                self.open_line = true;
            }
            if fits(self.policy.capture_max_bytes, self.bytes.len() as u128 + 1)
                && fits(self.policy.capture_max_lines, u128::from(self.lines))
            {
                self.bytes.try_reserve(1).map_err(io::Error::other)?;
                self.bytes.push(*byte);
            } else {
                self.truncated = true;
            }
            if *byte == b'\n' {
                self.open_line = false;
            }
        }
        Ok(raw.len())
    }
    /// Bytes remain private in memory until create-only receipt publication.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
/// Foreground-only owner, actual child and pollable kernel timer. Saved wait and
/// execution policies are frozen for this call; current ownership/scope/model
/// revocation is rechecked at the saved observation interval, then before action.
struct Control<'a> {
    store: &'a mut RootStore,
    row: Run,
    wait: crate::PlanningWaitPolicy,
    policy: crate::AgentExecutionPolicy,
    lease_seconds: i64,
    birth: crate::PlanningBootInstant,
    activity: Rc<Cell<crate::PlanningBootInstant>>,
    next_poll: crate::PlanningBootInstant,
    startup_due: crate::PlanningBootInstant,
    timer: Option<crate::PlanningBootTimer>,
    channel: Option<UnixStream>,
    startup: Option<crate::local_storage::LocalReservation>,
    gate_bytes: Vec<u8>,
    gate_eof: bool,
    grant_attempted: bool,
    stop_kind: Option<&'static str>,
    kill_due: Option<crate::PlanningBootInstant>,
    kill_sent: bool,
    leader_done: bool,
    drain_due: Option<crate::PlanningBootInstant>,
}
/// Convert an explicitly saved positive/nonnegative millisecond count without
/// clamping or a second policy ceiling. Caller validates its complete policy.
fn millis(value: i64) -> Result<Duration, E> {
    Ok(Duration::from_millis(value.try_into().map_err(|e| {
        caused("planning-duration-representation", e)
    })?))
}
/// Map current real UTC difference into this boot clock for one next wake. An
/// already due checkpoint stays due; this cannot resurrect an expired lease.
fn utc_wake(until: &str, now: crate::PlanningBootInstant) -> Result<crate::PlanningBootInstant, E> {
    let current = crate::system_timestamp().map_err(E::Runtime)?;
    let parse = |v: &str| {
        swarm_wire_json::go_rfc3339::parse(v).map_err(|e| caused("planning-wake-clock", e))
    };
    let delta = parse(until)? - parse(&current)?;
    let duration = if delta.is_negative() {
        Duration::ZERO
    } else {
        Duration::try_from(delta).map_err(|e| caused("planning-wake-representation", e))?
    };
    now.checked_add(duration)
        .map_err(|e| caused("planning-wake-representation", e))
}
/// Schedule the parent's full source validation relative to the private gate.
/// The callback may acquire the actual exclusive provider-config reservation;
/// defer it while the helper performs its authoritative pre-exec checks. Live
/// Work/lease checks and startup deadlines remain outside this gate. EOF grants
/// no process permission or successful exec proof; callback errors still refuse.
fn validate_after_gate(gate_eof: bool, validate: impl FnOnce() -> Result<(), E>) -> Result<(), E> {
    if gate_eof { validate() } else { Ok(()) }
}

impl Control<'_> {
    /// Consume only the trusted helper's CLOEXEC diagnostic endpoint. No stdout or
    /// provider JSON can emit a private pre-exec frame. EOF stays ambiguous alone.
    fn read_gate(&mut self) -> Result<(), E> {
        if self.gate_eof {
            return Ok(());
        }
        let channel = self
            .channel
            .as_mut()
            .ok_or(E::Refused("planning-gate-channel-missing"))?;
        loop {
            let mut bytes = [0u8; 4096];
            match channel.read(&mut bytes) {
                Ok(0) => {
                    self.gate_eof = true;
                    return Ok(());
                }
                Ok(n) => {
                    self.gate_bytes
                        .try_reserve(n)
                        .map_err(|e| caused("planning-gate-allocation", e))?;
                    self.gate_bytes.extend_from_slice(&bytes[..n]);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) => return Err(caused("planning-gate-read", error)),
            }
        }
    }
    /// Compute renewal from the CURRENT expiry and ORIGINAL admitted duration,
    /// including an aged claim. Percent0 disables maintenance; no default120 or
    /// half-lease constant replaces the saved ratio. Expired ownership still refuses.
    fn renewal_due(
        &self,
        work: &Work,
        now: crate::PlanningBootInstant,
    ) -> Result<Option<crate::PlanningBootInstant>, E> {
        let Some(delay) = self
            .wait
            .renewal_delay(self.lease_seconds)
            .map_err(E::Settings)?
        else {
            return Ok(None);
        };
        let scope = work
            .planning
            .as_deref()
            .and_then(|p| p.scopes.as_deref())
            .and_then(|s| s.iter().find(|s| s.id == self.row.scope_id))
            .ok_or(E::Refused("planning-scope-not-found"))?;
        let ttl = Duration::from_secs(
            self.lease_seconds
                .try_into()
                .map_err(|e| caused("planning-renewal-representation", e))?,
        );
        let remaining_threshold = ttl
            .checked_sub(delay)
            .ok_or(E::Refused("planning-renewal-representation"))?;
        let until = swarm_wire_json::go_rfc3339::parse(&scope.lease_until)
            .map_err(|e| caused("planning-renewal-clock", e))?;
        let due = until
            .checked_sub(
                time::Duration::try_from(remaining_threshold)
                    .map_err(|e| caused("planning-renewal-representation", e))?,
            )
            .ok_or(E::Refused("planning-renewal-representation"))?;
        let due = due
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| caused("planning-renewal-clock", e))?;
        utc_wake(&due, now).map(Some)
    }
    /// Authoritative renewal uses one fresh command and actual run fence, keeping
    /// generation/activation/reserve unchanged. Revision races return for replan;
    /// no hidden CAS count, repeated inference or automatic budget reset exists.
    fn renew(&mut self, work: &Work) -> Result<Work, E> {
        let value = crate::PlanningRequest {
            schema_version: 1,
            event_id: crate::generate_identifier("planning-renew-").map_err(E::Runtime)?,
            expected_revision: work.revision,
            scope: self.row.scope_id.clone(),
            scope_revision: self.row.scope_revision,
            holder: self.row.holder.clone(),
            generation: self.row.generation,
            lease_seconds: self.lease_seconds,
            ..Default::default()
        };
        let raw =
            crate::encode_model(&value).map_err(|e| caused("planning-renewal-encoding", e))?;
        let directory = self.store.directory.duplicate().map_err(E::Storage)?;
        let fence = RunFence {
            row: &self.row,
            directory: &directory,
            root: self.store.root.clone(),
            recovery: false,
            record: None,
            receipt_hash: None,
        };
        let result = self
            .store
            .execute_with_fence(
                &self.row.work_id,
                "planning.lease-renew",
                &raw,
                &mut crate::SystemEnvironment,
                Some(&fence),
            )
            .map(|v| v.result.work);
        finished(result, directory.close().map_err(E::Storage))
    }
    /// Arm only the earliest actual policy/lease/observation checkpoint on BOOTTIME.
    /// This kernel wake does not itself stop a model, renew ownership or accept work.
    /// Renewal checkpoints wait for private gate EOF to avoid writing while the
    /// helper still reads Root. Startup/lease/stop guards remain active; EOF is
    /// only a scheduling boundary, never provider-execution or success proof.
    fn schedule(&mut self, work: Option<&Work>, now: crate::PlanningBootInstant) -> Result<(), E> {
        let mut next = self.next_poll;
        if !self.gate_eof {
            next = next.min(self.startup_due);
        }
        if let Some(kill) = self.kill_due {
            if !self.kill_sent {
                next = next.min(kill);
            }
        }
        if let Some(drain) = self.drain_due {
            next = next.min(drain);
        }
        if !self.leader_done && self.stop_kind.is_none() {
            if self.wait.max_duration_seconds.0 > 0 {
                next = next.min(
                    self.birth
                        .checked_add(Duration::from_secs(self.wait.max_duration_seconds.0 as u64))
                        .map_err(|e| caused("planning-maximum-representation", e))?,
                );
            }
            if self.wait.silence_seconds.0 > 0 {
                next = next.min(
                    self.activity
                        .get()
                        .checked_add(Duration::from_secs(self.wait.silence_seconds.0 as u64))
                        .map_err(|e| caused("planning-silence-representation", e))?,
                );
            }
            if let Some(work) = work.filter(|_| self.gate_eof) {
                if let Some(due) = self.renewal_due(work, now)? {
                    next = next.min(due);
                }
            }
        }
        self.timer
            .as_ref()
            .ok_or(E::Refused("planning-wake-timer-missing"))?
            .arm(Some(next))
            .map_err(|e| caused("planning-wake-arm", e))
    }
    /// Close every remaining timer/socket/startup reservation, preserving each
    /// failure. No drop-only cleanup or socket EOF is a provider completion receipt.
    fn close(&mut self) -> Result<(), E> {
        let mut result = Ok(());
        if let Some(timer) = self.timer.take() {
            result = finished(
                result,
                timer.close().map_err(|e| caused("planning-wake-close", e)),
            );
        }
        if let Some(channel) = self.channel.take() {
            result = finished(
                result,
                nix::unistd::close(channel).map_err(|e| caused("planning-gate-close", e)),
            );
        }
        if let Some(startup) = self.startup.take() {
            result = finished(result, startup.close().map_err(E::Storage));
        }
        result
    }
}
impl OwnedProcessControl for Control<'_> {
    /// Persist kernel identity and committed permission before attempting one grant
    /// byte. Record the attempt conservatively BEFORE the write: write/close
    /// failure can never be interpreted as proof no grant was issued. Registration
    /// errors before this point retain their actual committed row and causes.
    /// Any failure enters owned-child cleanup; this flag proves no reap.
    fn started(&mut self, pid: u32) -> io::Result<()> {
        self.store
            .register_planner(&mut self.row, pid)
            .map_err(io::Error::other)?;
        self.grant_attempted = true;
        let grant = self
            .channel
            .as_mut()
            .ok_or(E::Refused("planning-gate-channel-missing"))
            .and_then(|s| {
                s.write_all(&[1])
                    .map_err(|e| caused("planning-gate-release", e))
            });
        let release = self
            .startup
            .take()
            .map_or(Ok(()), |r| r.close().map_err(E::Storage));
        finished(grant, release).map_err(io::Error::other)
    }
    /// Only actual pidfd readiness starts the saved optional output-drain deadline.
    fn leader_exited(&mut self) -> io::Result<()> {
        self.leader_done = true;
        let now = crate::PlanningBootInstant::now()?;
        self.drain_due = self
            .policy
            .output_drain_ms
            .0
            .map(|v| {
                millis(v.0).and_then(|d| {
                    now.checked_add(d)
                        .map_err(|e| caused("planning-drain-representation", e))
                })
            })
            .transpose()
            .map_err(io::Error::other)?;
        self.schedule(None, now).map_err(io::Error::other)
    }
    /// Relative Instant wake remains absent; the actual BOOTTIME descriptor is used.
    fn next_wake(&self) -> Option<Instant> {
        None
    }
    /// Borrow the owned CLOEXEC timer only for the common poll's lifetime.
    fn wake_descriptor(&self) -> Option<BorrowedFd<'_>> {
        self.timer.as_ref().map(AsFd::as_fd)
    }
    /// Borrow the local stop wake separately from the unchanged BOOTTIME timer.
    /// Once stopping/leader-exited, drop this wake to preserve the grace timer.
    fn interruption_descriptor(&self)->Option<BorrowedFd<'_>>{
        if self.stop_kind.is_some()||self.leader_done{None}else{self.store.interruption.as_ref().map(crate::ExecutionInterrupt::descriptor)}
    }
    /// Observe actual bytes/timers and saved-interval current state. Requests only
    /// owned TERM/KILL, retaining causes and unresolved states through the runner.
    /// Do not renew or reread provider configuration during private gate reads:
    /// zero-busy policy must not create self-contention with helper preparation. Gate EOF
    /// permits a renewal check only, never implies provider execution or success.
    /// Original startup/lease expiry can still refuse a slow helper.
    fn observe(&mut self) -> io::Result<Option<Signal>> {
        let operation = (|| {
            if let Some(timer) = &self.timer {
                timer
                    .take_expirations()
                    .map_err(|e| caused("planning-wake-read", e))?;
            }
            self.read_gate()?;
            let now = crate::PlanningBootInstant::now().map_err(|e| caused("planning-clock", e))?;
            if self.drain_due.is_some_and(|d| now >= d) {
                return Err(E::Refused("planning-output-drain-expired"));
            }
            if !self.gate_eof && now >= self.startup_due {
                return Err(E::Refused("planning-startup-preflight-expired"));
            }
            if self.leader_done {
                self.next_poll = now
                    .checked_add(millis(self.policy.poll_interval_ms.0)?)
                    .map_err(|e| caused("planning-poll-representation", e))?;
                self.schedule(None, now)?;
                return Ok(None);
            }
            if !self.leader_done && self.stop_kind.is_none() && self.store.interruption.as_ref().map(crate::ExecutionInterrupt::reason).transpose().map_err(|e|caused("planning-interrupt-observation",e))?.flatten().is_some(){
                self.stop_kind=Some("planning-execution-interrupted");
                self.kill_due=Some(now.checked_add(millis(self.policy.term_grace_ms.0)?).map_err(|e|caused("planning-grace-representation",e))?);
                self.schedule(None,now)?;return Ok(Some(Signal::TERM));
            }
            if self.stop_kind.is_none() {
                if let Some(stop) = self
                    .wait
                    .stop_reason(
                        now.duration_since(self.birth)
                            .map_err(|e| caused("planning-clock", e))?,
                        now.duration_since(self.activity.get())
                            .map_err(|e| caused("planning-clock", e))?,
                    )
                    .map_err(E::Settings)?
                {
                    self.stop_kind = Some(match stop {
                        crate::PlanningWaitStop::MaximumDuration => {
                            "planning-provider-maximum-duration"
                        }
                        crate::PlanningWaitStop::Silence => "planning-provider-silence",
                    });
                    self.kill_due = Some(
                        now.checked_add(millis(self.policy.term_grace_ms.0)?)
                            .map_err(|e| caused("planning-grace-representation", e))?,
                    );
                    self.schedule(None, now)?;
                    return Ok(Some(Signal::TERM));
                }
            } else if self.kill_due.is_some_and(|d| now >= d) && !self.kill_sent {
                self.kill_sent = true;
                self.schedule(None, now)?;
                return Ok(Some(Signal::KILL));
            }
            let mut work = None;
            if self.stop_kind.is_none() {
                let current = live(self.store, &self.row)?;
                validate_after_gate(self.gate_eof, || {
                    let inputs = self
                        .store
                        .planner_material(&self.row.work_id, &request(&self.row, current.revision))?;
                    if inputs.inputs.preview.context_sha256 != self.row.context_sha256
                        || inputs.inputs.preview.workflow_prompt_sha256
                            != self.row.workflow_prompt_sha256
                        || inputs.inputs.model.provider_digest != self.row.provider_digest
                        || crate::agent_launch_preflight::hash(&inputs.prompt) != self.row.prompt_sha256
                    {
                        return Err(E::Refused("planning-run-source-changed"));
                    }
                    Ok(())
                })?;

                if self.gate_eof && self.renewal_due(&current, now)?.is_some_and(|d| now >= d) {
                    work = Some(self.renew(&current)?);
                } else {
                    work = Some(current);
                }
            }
            let after =
                crate::PlanningBootInstant::now().map_err(|e| caused("planning-clock", e))?;
            self.next_poll = after
                .checked_add(millis(self.policy.poll_interval_ms.0)?)
                .map_err(|e| caused("planning-poll-representation", e))?;
            self.schedule(work.as_ref(), after)?;
            Ok(None)
        })();
        operation.map_err(io::Error::other)
    }
}

/// Root-owned identity/source/receipt guard, never a serialized permission.
/// All failures retain typed causes and leave uncertain persistence for recovery.
struct RunFence<'a> {
    row: &'a Run,
    directory: &'a crate::local_storage::LocalDirectory,
    root: std::path::PathBuf,
    recovery: bool,
    record: Option<&'a Receipt>,
    receipt_hash: Option<&'a str>,
}
impl MutationFence for RunFence<'_> {
    /// Admit only this run's durable healthy original after actual child wait.
    /// Rechecks exact row/root/owner/current source, original bytes/hash and a
    /// known child tuple whose process is absent or replaced. No foreign owner
    /// adoption, deadline inference, signalling, refund or escaped-tree proof.
    /// Unconfirmed physical evidence is a caused incident, never a bare business
    /// refusal: rollback must retain ownership and the original for reconciliation.
    fn finishing_planner(&self, db: &rusqlite::Connection, row: &Run, event: &str, now: &str) -> Result<(), WorkEngineError> {
        self.check(db, &row.work_id, now)?;
        let result = (|| {
            let record = self.record.ok_or(A::Refused("planning-closure-original-required"))?;
            if row != self.row || row.decision_event != event || !same_record(row, &record.run)
                || row.phase != Phase::Permitted || row.permission != 1
                || record.schema_version != 1 || !record.output_complete || record.exit_code != Some(0)
                || record.signal.is_some() || record.failure.is_some() || record.gate.is_some()
                || record.grant_attempted != Some(true) {
                return Err(A::Refused("planning-closure-native-run-active"));
            }
            let body = self.directory.read_native_receipt(&receipt_name(&row.claim_id)).map_err(|e| A::caused("planning-closure-original-read", e))?;
            let expected = serde_json::to_vec(record).map_err(|e| A::caused("planning-closure-original-encoding", e))?;
            if body != expected || Some(crate::agent_launch_preflight::hash(&body).as_str()) != self.receipt_hash {
                return Err(A::Refused("planning-closure-original-changed"));
            }
            let pid = row.child_pid.ok_or(A::Refused("planning-closure-child-unknown"))?;
            let stamp = row.child_stamp.as_deref().ok_or(A::Refused("planning-closure-child-unknown"))?;
            match crate::observe_linux_process(pid.try_into().map_err(|e| A::caused("planning-closure-child-pid", e))?) {
                Ok(p) if p.process_stamp() != stamp => Ok(()),
                Ok(_) => Err(A::Refused("planning-closure-child-still-present")),
                Err(e) if e.confirms_absence() => Ok(()),
                Err(e) => Err(A::caused("planning-closure-child-observation", e)),
            }
        })();
        result.map_err(|e| WorkEngineError::PlanningActivation(Box::new(A::caused("planning-closure-physical-unconfirmed", e))))
    }
    /// Validate real owner/recovery, root and source row inside the Work transaction.
    /// Before decision, recheck complete current input/lease/action fences. A
    /// separately verified original record validates its historical reservation;
    /// pre-start observations retain current start funding. After staging, require
    /// the exact applied event receipt instead of comparing changed tasks. This
    /// check grants no new process, refund or authenticated agent identity; typed
    /// source/financial/SQL/owner failures prevent application without mutation.
    fn check(
        &self,
        db: &rusqlite::Connection,
        work: &str,
        now: &str,
    ) -> Result<(), WorkEngineError> {
        let runtime = (|| {
            if work != self.row.work_id {
                return Err(A::Refused("planning-run-work-mismatch"));
            }
            let actual = runs::load(db, &self.row.claim_id)?;
            run_binding::check(db, self.directory, &actual.claim_id, actual.phase == Phase::Applied)?;
            if !same_record(&actual, self.row) {
                return Err(A::Refused("planning-run-origin-changed"));
            }
            let identity = self
                .directory
                .identity()
                .map_err(|e| A::caused("planning-run-root-observation", e))?;
            if identity.0.to_string() != actual.directory_device
                || identity.1.to_string() != actual.directory_inode
            {
                return Err(A::Refused("planning-run-root-changed"));
            }
            if self.recovery {
                absent_owner(&actual).map_err(|e| A::caused(e.code(), e))?;
            } else {
                current_owner(&actual).map_err(|e| A::caused(e.code(), e))?;
            }
            if actual.phase == Phase::Applied {
                if actual.receipt_sha256.as_deref() != self.receipt_hash {
                    return Err(A::Refused("planning-run-receipt-changed"));
                }
                let event: (String, String, i64) = crate::agent_launch::query(
                    db,
                    "SELECT work_id,kind,revision FROM events WHERE id=?",
                    [&actual.decision_event],
                    |rows| {
                        let r = rows
                            .next()?
                            .ok_or(A::Refused("planning-run-applied-event-missing"))?;
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                    },
                )?;
                if event.0 != work
                    || event.1 != "planning.decide"
                    || Some(event.2) != actual.applied_revision
                {
                    return Err(A::Refused("planning-run-applied-event-mismatch"));
                }
                return Ok(true);
            }
            if actual.phase != Phase::Permitted || actual.permission != 1 {
                return Err(A::Refused("planning-run-not-permitted"));
            }
            Ok(false)
        })();
        let applied = runtime.map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
        if applied {
            return Ok(());
        }
        if self.record.is_some() {
            let current = crate::read_recorded_work(db, work).map_err(|e| {
                WorkEngineError::PlanningActivation(Box::new(A::caused(
                    "planning-run-work-read",
                    e,
                )))
            })?;
            crate::planning_call_preview::verified_result(
                db,
                &self.root,
                &RootStore::read_settings_from(self.directory).map_err(|e| {
                    WorkEngineError::PlanningActivation(Box::new(A::caused(e.code(), e)))
                })?,
                work,
                &request(self.row, current.revision),
            )
            .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
        } else {
            let current = crate::read_recorded_work(db, work).map_err(|e| {
                WorkEngineError::PlanningActivation(Box::new(A::caused(
                    "planning-run-work-read",
                    e,
                )))
            })?;
            let r = crate::PlanningRequest {
                schema_version: 1,
                scope: self.row.scope_id.clone(),
                scope_revision: self.row.scope_revision,
                holder: self.row.holder.clone(),
                generation: self.row.generation,
                ..Default::default()
            };
            crate::planning_decision::fence(&current, &r, now)
                .map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))?;
        }
        Ok(())
    }
    /// Stage usage/run application with the exact engine event in the SAME SQL
    /// transaction as business changes. No inference, refund or extra activation.
    fn stage_receipt(
        &self,
        db: &rusqlite::Connection,
        r: MutationFenceReceipt<'_>,
    ) -> Result<(), WorkEngineError> {
        if r.kind == "planning.lease-renew" {
            return Ok(());
        }
        let result = (|| {
            let record = self
                .record
                .ok_or(A::Refused("planning-run-receipt-required"))?;
            if r.kind != "planning.decide"
                || r.work != self.row.work_id
                || r.event != self.row.decision_event
            {
                return Err(A::Refused("planning-run-action-mismatch"));
            }
            usage::record(db, &record.usage)?;
            let old = runs::load(db, &self.row.claim_id)?;
            let mut new = old.clone();
            new.phase = Phase::Applied;
            new.applied_revision = Some(r.work_revision);
            new.receipt_sha256 = Some(
                self.receipt_hash
                    .ok_or(A::Refused("planning-run-receipt-required"))?
                    .into(),
            );
            runs::transition(db, &old, &new)
        })();
        result.map_err(|e| WorkEngineError::PlanningActivation(Box::new(e)))
    }
}

/// Recognize only a bare typed planning refusal returned after successful
/// WorkEngine rollback. Never match message text, IDs or broad error codes;
/// commit/rollback/cleanup/SQL wrappers stay unknown and must be reconciled.
fn business_refusal(error: &E) -> Option<&'static str> {
    match error {
        E::Mutation(WorkEngineError::PlanningActivation(cause)) => match cause.as_ref() {
            A::Refused(reason) => Some(*reason),
            _ => None,
        },
        _ => None,
    }
}
/// Require same-host original owner and child to be absent or replaced, using
/// actual process stamps. Observation failures never authorize recovery; no PID
/// is signalled. Same-UID receipt authentication remains outside this contract.
fn absent_owner(row: &Run) -> Result<(), E> {
    let here = crate::SupervisorIdentity::current()
        .map_err(|e| caused("planning-reconcile-host-observation", e))?;
    if here.host() != row.owner_host {
        return Err(E::Refused("planning-reconcile-host-changed"));
    }
    let absent = |pid: i64, stamp: &str| -> Result<(), E> {
        match crate::observe_linux_process(
            pid.try_into()
                .map_err(|e| caused("planning-reconcile-pid", e))?,
        ) {
            Ok(p) if p.process_stamp() != stamp => Ok(()),
            Ok(_) => Err(E::Refused("planning-reconcile-process-still-present")),
            Err(e) if e.confirms_absence() => Ok(()),
            Err(e) => Err(caused("planning-reconcile-process-observation", e)),
        }
    };
    absent(row.owner_pid, &row.owner_stamp)?;
    if let Some(pid) = row.child_pid {
        absent(
            pid,
            row.child_stamp
                .as_deref()
                .ok_or(E::Refused("planning-reconcile-child-stamp-required"))?,
        )?;
    }
    Ok(())
}
impl RootStore {
    /// Run exactly one foreground provider for an existing bound claim. Retain
    /// directory and real SQLite O_PATH witnesses through provider and decision;
    /// an immutable run binding is checked by the parent, private gate and Work
    /// fence. Legacy receipts stay historical without new execution authority.
    /// Requires a writable inactive native Root and this CLI private helper. Current
    /// policy, budget and freshness refuse before exec; actual PID registration and
    /// committed permission precede grant. Wait policy uses BOOTTIME and actual IO;
    /// a valid proposal is journalled before atomic decision/usage application.
    /// Failures retain causes and receipts; repeat run never re-infers. Acceptance,
    /// invoice, confinement and termination of escaped descendants are not proved.
    pub fn run_planning(&mut self, work: &str, raw: &[u8]) -> Result<PlanningRunOutcome, E> {
        if !self.writable { return Err(E::Refused("root-read-only")); }
        // Keep both directory and database O_PATH witnesses through all provider,
        // gate, receipt and decision work; close failure preserves known Work.
        let binding = self.bind_root()?;
        let result = self.run_planning_bound(work, raw, &binding);
        planning_result_finished(result, binding.close())
    }
    /// Execute under the real owning Root witnesses from the public wrapper.
    /// The preparation journal is immutable; every new grant checks its actual
    /// connected datastore. No copied tuple or historical receipt is a permit.
    fn run_planning_bound(&mut self, work: &str, raw: &[u8], binding: &super::RootBinding) -> Result<PlanningRunOutcome, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let r = crate::planning_call_preview::decode(raw).map_err(launch)?;
        if runs::schema(&self.db, false).map_err(launch)? {
            let existing: i64 = crate::agent_launch::query(
                &self.db,
                "SELECT count(*) FROM native_planning_runs WHERE claim_id=?",
                [&r.claim_id],
                |rows| {
                    Ok(rows
                        .next()?
                        .ok_or(A::Refused("planning-run-observation"))?
                        .get(0)?)
                },
            )
            .map_err(launch)?;
            if existing != 0 {
                return Err(E::Refused("planning-run-already-recorded"));
            }
        }
        let m = self.planner_material(work, &r)?;
        let policy = m
            .settings
            .values
            .agent_execution_policy()
            .map_err(E::Settings)?;
        let wait = m
            .settings
            .values
            .planning_wait_policy()
            .map_err(E::Settings)?;
        let parser = crate::PlanningOutputStream::new(&m.settings.values).map_err(launch)?;
        let birth = crate::PlanningBootInstant::now().map_err(|e| caused("planning-clock", e))?;
        let startup_due = utc_wake(&m.host.preflight.valid_until, birth)?;
        let next_poll = birth
            .checked_add(millis(policy.poll_interval_ms.0)?)
            .map_err(|e| caused("planning-poll-representation", e))?;
        let timer =
            crate::PlanningBootTimer::new().map_err(|e| caused("planning-wake-create", e))?;
        let pair = UnixStream::pair().map_err(|e| caused("planning-gate-create", e));
        let (channel, input) = match pair {
            Ok(p) => p,
            Err(e) => {
                return finished(
                    Err(e),
                    timer.close().map_err(|e| caused("planning-wake-close", e)),
                )
            }
        };
        let setup = channel
            .set_nonblocking(true)
            .map_err(|e| caused("planning-gate-nonblocking", e))
            .and_then(|_| self.directory.reserve(false, false).map_err(E::Storage));
        let startup = match setup {
            Ok(v) => v,
            Err(e) => {
                return finished(
                    finished(
                        finished(
                            Err(e),
                            nix::unistd::close(input)
                                .map_err(|e| caused("planning-input-close", e)),
                        ),
                        nix::unistd::close(channel).map_err(|e| caused("planning-gate-close", e)),
                    ),
                    timer.close().map_err(|e| caused("planning-wake-close", e)),
                )
            }
        };
        let row = match self.claim_planner(&m, &r, binding) {
            Ok(v) => v,
            Err(e) => {
                return finished(
                    finished(
                        finished(
                            finished(Err(e), startup.close().map_err(E::Storage)),
                            nix::unistd::close(input)
                                .map_err(|e| caused("planning-input-close", e)),
                        ),
                        nix::unistd::close(channel).map_err(|e| caused("planning-gate-close", e)),
                    ),
                    timer.close().map_err(|e| caused("planning-wake-close", e)),
                )
            }
        };
        let mut command = Command::new("/proc/self/exe");
        command
            .arg("_planning_gate")
            .arg(&row.claim_id)
            .arg("--root")
            .arg(&self.root)
            .arg("--json");
        let stream = Rc::new(RefCell::new(parser));
        let activity = Rc::new(Cell::new(birth));
        let mut stdout = OutputSink {
            stream: stream.clone(),
            activity: activity.clone(),
        };
        let mut stderr = ErrorSink {
            activity: activity.clone(),
            policy: policy.clone(),
            bytes: vec![],
            lines: 0,
            open_line: false,
            truncated: false,
        };
        let mut control = Control {
            store: self,
            row,
            wait,
            policy,
            lease_seconds: m.inputs.lease_seconds,
            birth,
            activity,
            next_poll,
            startup_due,
            timer: Some(timer),
            channel: Some(channel),
            startup: Some(startup),
            gate_bytes: vec![],
            gate_eof: false,
            grant_attempted: false,
            stop_kind: None,
            kill_due: None,
            kill_sent: false,
            leader_done: false,
            drain_due: None,
        };
        let armed = control.schedule(None, birth);
        let mut status = None;
        let process = match armed {
            Ok(()) => {
                let owned = run_controlled_command(
                    command,
                    Stdio::from(OwnedFd::from(input)),
                    None,
                    &mut stdout,
                    Some(&mut stderr),
                    Some(&mut control),
                );
                status = owned
                    .as_ref()
                    .ok()
                    .map(|p| p.usage.status)
                    .or_else(|| owned.as_ref().err().and_then(|e| e.reaped_exit_status()));
                owned.map_err(|e| caused("planning-provider-process", e))
            }
            Err(e) => finished(
                Err(e),
                nix::unistd::close(input).map_err(|e| caused("planning-input-close", e)),
            ),
        };
        // Final pipe inspection and close errors remain attached to the process result.
        let mut process = finished(finished(process, control.read_gate()), control.close());
        let row = control.row.clone();
        let stop = control.stop_kind;
        let gate =
            match crate::agent_supervision::decode_gate(&control.gate_bytes, control.gate_eof) {
                Ok(frame) => frame,
                Err(e) => {
                    process = finished(process, Err(e));
                    None
                }
            };
        let mut failure = stopped_failure(process.err(),stop,gate.is_some());
        if failure.is_none() && status.is_none_or(|s| !s.success()) {
            failure = Some(E::Refused("planning-provider-exit-unsuccessful"));
        }
        if stop==Some("planning-execution-interrupted") && status.as_ref().is_some_and(|s|!crate::execution_interrupt::stopped_status(s)){
            failure=stopped_failure(failure,Some("planning-provider-exit-unsuccessful"),false);
        }
        let mut parser = stream
            .try_borrow_mut()
            .map_err(|e| caused("planning-output-borrow", e))?;
        if failure.is_some() {
            parser.incomplete_transport();
        }
        let parsed = parser.finish();
        let proposal = match parsed {
            Ok(v) => Some(
                String::from_utf8(v.raw_proposal)
                    .map_err(|e| caused("planning-proposal-encoding", e))?,
            ),
            Err(e) => {
                if failure.is_none() {
                    failure = Some(launch(e));
                }
                None
            }
        };
        let facts = parser.facts_snapshot();
        let usage = UsageRecord::observe(&row.claim_id, &facts).map_err(launch)?;
        let facts = parser.provider_facts().map_err(launch)?;
        drop(parser);
        let receipt = Receipt {
            schema_version: 1,
            run: row,
            finished_at: crate::system_timestamp().map_err(E::Runtime)?,
            exit_code: status.and_then(|s| s.code()),
            signal: status.and_then(|s| s.signal()),
            output_complete: failure.is_none(),
            proposal,
            usage,
            facts,
            failure: failure.as_ref().map(|e| e.code().into()),
            grant_attempted: Some(control.grant_attempted),
            failure_causes: failure
                .as_ref()
                .map(|e| crate::native_diagnostics::project(e)),
            gate,
            stderr: stderr.bytes,
            stderr_truncated: stderr.truncated,
        };
        let body =
            serde_json::to_vec(&receipt).map_err(|e| caused("planning-receipt-encoding", e))?;
        let frame = crate::native_receipt::encode(&body)
            .map_err(|e| caused("planning-receipt-framing", e))?;
        let saved = self
            .directory
            .publish_new_with_fault(
                &receipt_name(&receipt.run.claim_id),
                &frame,
                crate::local_storage::AtomicStorageFault::None,
            )
            .map_err(E::Storage);
        if let Some(error) = failure {
            let settled = saved.and_then(|_| {
                if status.is_some() || receipt.unadmitted() {
                    self.record_planning_failure(&receipt, &body, false)
                } else {
                    Ok(None)
                }
            });
            return Err(feedback_error(error, settled));
        }
        saved?;
        self.apply_planning_receipt(&receipt, &body, false)
    }
    /// Apply an already durable receipt without starting a model. Root/claim source,
    /// actual owner or recovery absence and current authoritative settings are checked
    /// inside the same transaction as the decision, scalar usage and run state.
    fn apply_planning_receipt(
        &mut self,
        receipt: &Receipt,
        body: &[u8],
        recovery: bool,
    ) -> Result<PlanningRunOutcome, E> {
        if receipt.schema_version != 1
            || !receipt.output_complete
            || receipt.failure.is_some()
            || receipt.gate.is_some()
            || receipt.exit_code != Some(0)
            || receipt.signal.is_some()
        {
            return Err(E::Refused("planning-receipt-not-successful"));
        }
        let row = runs::load(&self.db, &receipt.run.claim_id).map_err(launch)?;
        if !same_record(&row, &receipt.run) || receipt.usage.claim_id != row.claim_id {
            return Err(E::Refused("planning-receipt-origin-mismatch"));
        }
        receipt.usage.validate().map_err(launch)?;
        if row.phase == Phase::Refused {
            if row.refusal_code.is_none()
                || row.receipt_sha256.as_deref()
                    != Some(crate::agent_launch_preflight::hash(body).as_str())
                || usage::load(&self.db, &row.claim_id).map_err(launch)? != receipt.usage
            {
                return Err(E::Refused("planning-receipt-refusal-conflict"));
            }
            let cap = self
                .settings()?
                .values
                .planning_report_limit()
                .map_err(E::Settings)?;
            rejection_feedback::verify(
                &self.db,
                &self.directory,
                receipt,
                &crate::agent_launch_preflight::hash(body),
                row.refusal_code
                    .as_deref()
                    .ok_or(E::Refused("planning-receipt-refusal-conflict"))?,
                cap,
            )
            .map_err(launch)?;
            return Err(E::Refused("planning-receipt-business-refused"));
        }
        if row.phase == Phase::Applied {
            let directory = self.directory.duplicate().map_err(E::Storage)?;
            let hash = crate::agent_launch_preflight::hash(body);
            let fence = RunFence {
                row: &row,
                directory: &directory,
                root: self.root.clone(),
                recovery,
                record: Some(receipt),
                receipt_hash: Some(&hash),
            };
            let result = crate::agent_storage_retry::execute(
                &mut self.db,
                &self.directory,
                false,
                |db, _| {
                    let mut tx = db.transaction().map_err(E::Sql)?;
                    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
                    let value = (|| {
                        let at = crate::system_timestamp().map_err(E::Runtime)?;
                        fence.check(&tx, &row.work_id, &at).map_err(E::Mutation)?;
                        if usage::load(&tx, &row.claim_id).map_err(launch)? != receipt.usage {
                            return Err(E::Refused("planning-receipt-usage-mismatch"));
                        }
                        let work = crate::read_recorded_work(&tx, &row.work_id).map_err(E::Work)?;
                        Ok(PlanningRunOutcome {
                            schema_version: 1,
                            claim_id: row.claim_id.clone(),
                            decision_event: row.decision_event.clone(),
                            decision_applied: true,
                            accepted: false,
                            replayed: true,
                            work,
                        })
                    })();
                    planning_result_finished(value, tx.rollback().map_err(E::Sql))
                },
            );
            return planning_result_finished(result, directory.close().map_err(E::Storage));
        }
        let settings = self.settings()?;
        let original = settings
            .history
            .iter()
            .find(|e| e.revision.0 == row.settings_revision)
            .ok_or(E::Refused("planning-receipt-settings-history-missing"))?;
        let policy = original
            .change
            .values
            .planning_decision_policy()
            .map_err(E::Settings)?;
        let proposal = crate::decode_planning_proposal(
            receipt
                .proposal
                .as_deref()
                .ok_or(E::Refused("planning-receipt-proposal-required"))?
                .as_bytes(),
            &policy,
        )
        .map_err(launch)?;
        let operation_inventory = rejection_feedback::inventory(&proposal).map_err(launch)?;
        let work = self.read_work(&row.work_id)?;
        let decision = crate::PlanningRequest {
            schema_version: 1,
            event_id: row.decision_event.clone(),
            expected_revision: work.revision,
            scope: row.scope_id.clone(),
            scope_revision: row.scope_revision,
            holder: row.holder.clone(),
            generation: row.generation,
            input_events: Some(proposal.input_events),
            operations: Some(proposal.operations),
            reason: proposal.reason,
            ..Default::default()
        };
        let raw =
            crate::encode_model(&decision).map_err(|e| caused("planning-decision-encoding", e))?;
        let directory = self.directory.duplicate().map_err(E::Storage)?;
        let hash = crate::agent_launch_preflight::hash(body);
        let fence = RunFence {
            row: &row,
            directory: &directory,
            root: self.root.clone(),
            recovery,
            record: Some(receipt),
            receipt_hash: Some(&hash),
        };
        let result = self
            .execute_with_fence(
                &row.work_id,
                "planning.decide",
                &raw,
                &mut crate::SystemEnvironment,
                Some(&fence),
            )
            .map(|v| PlanningRunOutcome {
                schema_version: 1,
                claim_id: row.claim_id.clone(),
                decision_event: row.decision_event.clone(),
                decision_applied: true,
                accepted: false,
                replayed: v.result.replayed,
                work: v.result.work,
            });
        let result = planning_result_finished(result, directory.close().map_err(E::Storage));
        match result {
            Err(error) => {
                if let Some(reason) = business_refusal(&error) {
                    let settled = self.record_business_refusal(
                        receipt,
                        body,
                        reason,
                        recovery,
                        operation_inventory,
                    );
                    Err(feedback_error(error, settled))
                } else {
                    Err(error)
                }
            }
            Ok(value) => Ok(value),
        }
    }
    /// Deliver a known rolled-back parsed proposal to its still-current owner.
    /// Exact durable receipt, kernel owner/recovery and mission/source/revision
    /// guards precede the atomic Work/event/usage/run publication. The private
    /// artifact is published first; SQL failure retains it and the journal for
    /// reconciliation without inference. Saved report policy bounds it. A late
    /// superseded scope retains terminal-only history; its routing remains open.
    /// Returns known feedback Work or None for superseded ownership, never an
    /// accepted proposal. Commit/rollback/release failures preserve their causes.
    fn record_business_refusal(
        &mut self,
        receipt: &Receipt,
        body: &[u8],
        reason: &'static str,
        recovery: bool,
        operation_inventory: Vec<(String, String)>,
    ) -> Result<Option<Work>, E> {
        if receipt.schema_version != 1
            || !receipt.output_complete
            || receipt.proposal.is_none()
            || receipt.failure.is_some()
            || receipt.exit_code != Some(0)
            || receipt.signal.is_some()
            || receipt.gate.is_some()
        {
            return Err(E::Refused("planning-business-refusal-evidence-required"));
        }
        self.record_planning_feedback(
            receipt,
            body,
            reason,
            recovery,
            operation_inventory,
            RefusalDisposition::Business,
        )
    }
    /// Deliver a validated disposition under shared origin/ownership/source guards.
    /// Callers prove a complete refused proposal, actual reap or private no-grant
    /// revocation. Exact receipt and usage correlate replay; immutable artifact,
    /// Work/event/usage/run commit atomically, with no financial refund or retry.
    /// Known commits retain Work metadata through release errors; unknown commits
    /// stay errors. Superseded routing and raw-stream preservation remain open.
    fn record_planning_feedback(
        &mut self,
        receipt: &Receipt,
        body: &[u8],
        reason: &str,
        recovery: bool,
        operation_inventory: Vec<(String, String)>,
        disposition: RefusalDisposition,
    ) -> Result<Option<Work>, E> {
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let result = (|| {
            // Feedback publishes a report before its SQL transaction. Reject a
            // different datastore before that first side effect as well as at commit.
            run_binding::check(&self.db, &self.directory, &receipt.run.claim_id, false).map_err(launch)?;
            let hash = crate::agent_launch_preflight::hash(body);
            let original = self.read_work(&receipt.run.work_id)?;
            let at = crate::system_timestamp().map_err(E::Runtime)?;
            let prepared = rejection_feedback::prepare(
                &original,
                receipt,
                operation_inventory,
                reason,
                &hash,
                &at,
            )
            .map_err(launch)?;
            if let Some(value) = &prepared {
                let cap = self
                    .settings()?
                    .values
                    .planning_report_limit()
                    .map_err(E::Settings)?;
                rejection_feedback::publish(&self.directory, value, cap).map_err(launch)?;
            }
            let directory = &self.directory;
            let mut known_feedback = None;
            let result = crate::agent_storage_retry::execute(
                &mut self.db,
                directory,
                false,
                |db, settings| {
                    let committed = crate::agent_execution_store::transaction(db, |tx| {
                        let old = runs::load(tx, &receipt.run.claim_id)?;
                        run_binding::check(tx, directory, &old.claim_id, false)?;
                        if !same_record(&old, &receipt.run)
                            || old.phase == Phase::Applied
                            || (disposition == RefusalDisposition::Business
                                && (old.phase != Phase::Permitted || old.permission != 1))
                            || (disposition == RefusalDisposition::UnadmittedFailure
                                && (old.permission != 0
                                    || !matches!(old.phase, Phase::Claimed | Phase::Refused)))
                            || receipt.usage.claim_id != old.claim_id
                        {
                            return Err(A::Refused("planning-business-refusal-origin-mismatch"));
                        }
                        let identity = directory
                            .identity()
                            .map_err(|e| A::caused("planning-rejection-root-observation", e))?;
                        if identity.0.to_string() != old.directory_device
                            || identity.1.to_string() != old.directory_inode
                        {
                            return Err(A::Refused("planning-rejection-root-changed"));
                        }
                        if recovery {
                            absent_owner(&old).map_err(|e| A::caused(e.code(), e))?
                        } else {
                            current_owner(&old).map_err(|e| A::caused(e.code(), e))?
                        }
                        if old.phase == Phase::Refused {
                            if old.receipt_sha256.as_deref() != Some(&hash)
                                || old.refusal_code.as_deref() != Some(reason)
                                || usage::load(tx, &old.claim_id)? != receipt.usage
                            {
                                return Err(A::Refused("planning-failure-receipt-conflict"));
                            }
                            let cap = settings
                                .values
                                .planning_report_limit()
                                .map_err(|e| A::caused("planning-rejection-policy", e))?;
                            rejection_feedback::verify(tx, directory, receipt, &hash, reason, cap)?;
                            return Ok(None);
                        }
                        let count: i64 = crate::agent_launch::query(
                            tx,
                            "SELECT count(*) FROM events WHERE id=?",
                            [&old.decision_event],
                            |r| {
                                Ok(r.next()?
                                    .ok_or(A::Refused("planning-business-refusal-observation"))?
                                    .get(0)?)
                            },
                        )?;
                        if count != 0 {
                            return Err(A::Refused("planning-business-refusal-decision-present"));
                        }
                        let mut new = old.clone();
                        new.phase = Phase::Refused;
                        new.receipt_sha256 = Some(hash.clone());
                        new.refusal_code = Some(reason.into());
                        let settle = |db: &rusqlite::Connection| -> Result<(), A> {
                            usage::record(db, &receipt.usage)?;
                            runs::transition(db, &old, &new)
                        };
                        if let Some(value) = &prepared {
                            super::require_planning_mission_active(tx, &old.work_id)
                                .map_err(|e| A::caused("planning-rejection-mission", e))?;
                            let current = crate::read_recorded_work(tx, &old.work_id)
                                .map_err(|e| A::caused("planning-rejection-work", e))?;
                            if current != original {
                                return Err(A::Refused("planning-rejection-state-changed"));
                            }
                            let cap = settings
                                .values
                                .planning_report_limit()
                                .map_err(|e| A::caused("planning-rejection-policy", e))?;
                            if cap.is_some_and(|n| value.body.len() > n) {
                                return Err(A::Refused("planning-rejection-report-limit"));
                            }
                            let outcome = crate::stage_recorded_change_with_hook(
                                tx,
                                crate::RecordedChange {
                                    work: &value.work,
                                    expected_revision: original.revision,
                                    event: &value.event,
                                    kind: "planning.rejected",
                                    request: &value.body,
                                },
                                |db, _| {
                                    settle(db)
                                        .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
                                },
                            )
                            .map_err(|e| A::caused("planning-rejection-record", e))?;
                            if !matches!(outcome, crate::RecordedChangeOutcome::Staged) {
                                return Err(A::Refused("planning-rejection-unexpected-replay"));
                            }
                        } else {
                            settle(tx)?;
                        }
                        if runs::load(tx, &old.claim_id)? != new
                            || usage::load(tx, &old.claim_id)? != receipt.usage
                        {
                            return Err(A::Refused("planning-rejection-settlement-changed"));
                        }
                        Ok(prepared.as_ref().map(|v| v.work.clone()))
                    });
                    if let Ok(Some(work)) = &committed {
                        known_feedback = Some((work.id.clone(), work.revision));
                    }
                    committed.map_err(launch)
                },
            );
            result.map_err(|primary| match known_feedback {
                Some((work, revision)) => E::WorkRecorded {
                    primary: Box::new(primary),
                    work,
                    revision,
                },
                None => primary,
            })
        })();
        let known = result
            .as_ref()
            .ok()
            .and_then(|v| v.as_ref())
            .map(|w| (w.id.clone(), w.revision));
        finished(result, reservation.close().map_err(E::Storage)).map_err(|primary| match known {
            Some((work, revision)) => E::WorkRecorded {
                primary: Box::new(primary),
                work,
                revision,
            },
            None => primary,
        })
    }
    /// Deliver a durable reaped failure or revoke a proven unadmitted call.
    /// No proposed task/input acknowledgement, reservation refund or retry occurs.
    /// Missing wait status requires explicit private no-grant observation plus
    /// exact original unadmitted row; otherwise it stays unknown. No reap or
    /// absence of all kernel children is inferred from a spawn error or SQL row.
    /// Safe structured causes retain labels/numeric codes, never private text.
    /// SQL/publication errors preserve receipts for explicit recovery; known
    /// feedback Work is distinct from process completion/business acceptance.
    fn record_planning_failure(
        &mut self,
        receipt: &Receipt,
        body: &[u8],
        recovery: bool,
    ) -> Result<Option<Work>, E> {
        if receipt.schema_version != 1
            || receipt.output_complete
            || receipt.failure.is_none()
            || (receipt.exit_code.is_none() && receipt.signal.is_none() && !receipt.unadmitted())
        {
            return Err(E::Refused("planning-failure-evidence-required"));
        }
        if receipt
            .failure_causes
            .as_ref()
            .is_some_and(|causes| causes.is_empty() || causes.iter().any(|cause| !cause.valid()))
        {
            return Err(E::Refused("planning-failure-causes-invalid"));
        }
        let disposition = if receipt.exit_code.is_some() || receipt.signal.is_some() {
            RefusalDisposition::ReapedFailure
        } else {
            RefusalDisposition::UnadmittedFailure
        };
        self.record_planning_feedback(
            receipt,
            body,
            receipt
                .failure
                .as_deref()
                .ok_or(E::Refused("planning-failure-evidence-required"))?,
            recovery,
            Vec::new(),
            disposition,
        )
    }
    /// Reconcile a create-only journal after foreground owner loss, without a new
    /// provider call. Require original host and absent/replaced owner/child; missing,
    /// malformed, stale or unsuccessful evidence refuses. Current source/action
    /// guards still apply; receipt checksum alone is not identity authentication.
    pub fn reconcile_planning(&mut self, claim: &str) -> Result<PlanningRunOutcome, E> {
        if !self.writable { return Err(E::Refused("root-read-only")); }
        if !swarm_identity_contract::name(claim.as_bytes()) { return Err(E::Refused("planning-claim-invalid")); }
        let binding = self.bind_root()?;
        let result = self.reconcile_planning_bound(claim);
        planning_result_finished(result, binding.close())
    }
    /// Reconcile while the public wrapper owns the actual datastore witnesses.
    /// Failure feedback requires prepared physical provenance before publication
    /// and again inside its writer; historical Applied replay starts no provider.
    fn reconcile_planning_bound(&mut self, claim: &str) -> Result<PlanningRunOutcome, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        if !swarm_identity_contract::name(claim.as_bytes()) {
            return Err(E::Refused("planning-claim-invalid"));
        }
        let row = runs::load(&self.db, claim).map_err(launch)?;
        run_binding::check(&self.db, &self.directory, claim, row.phase == Phase::Applied).map_err(launch)?;
        absent_owner(&row)?;
        let body = self
            .directory
            .read_native_receipt(&receipt_name(claim))
            .map_err(E::Storage)?;
        let receipt: Receipt =
            serde_json::from_slice(&body).map_err(|e| caused("planning-receipt-format", e))?;
        if !receipt.output_complete {
            let settled = self.record_planning_failure(&receipt, &body, true);
            return Err(feedback_error(
                E::Refused("planning-receipt-not-successful"),
                settled,
            ));
        }
        self.apply_planning_receipt(&receipt, &body, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Real descriptor-level provider lock witnesses the child preflight phase.
    /// Pending gate must not let the parent contend; after EOF the same busy
    /// lock still refuses, and release permits actual validation again. No
    /// positive planning SQL, native permission, timing bound or retry is seeded.
    #[test]
    fn pending_gate_defers_parent_provider_lock_but_eof_keeps_validation() {
        use std::{fs, os::unix::fs::PermissionsExt};
        let path=std::env::temp_dir().join(crate::generate_identifier("swarm-gate184-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path,fs::Permissions::from_mode(0o700)).unwrap();
        eprintln!("Owned provider-gate fixture {}",path.display());
        let store=RootStore::initialize(&path,&mut crate::SystemEnvironment).unwrap();
        let probe=|| store.directory.reserve_provider_configuration().map_err(E::Storage)?.close().map_err(E::Storage);
        validate_after_gate(true,probe).unwrap();
        let held=store.directory.reserve_provider_configuration().unwrap();
        let mut called=false;
        let result=validate_after_gate(false,|| {called=true;probe()});
        assert!(result.is_ok() && !called,"parent contended with helper preflight");
        let failure=validate_after_gate(true,probe).unwrap_err();
        let busy=crate::native_diagnostics::project(&failure).iter()
            .any(|cause|cause.io_errno==Some(rustix::io::Errno::AGAIN.raw_os_error()));
        assert!(busy,"real post-gate provider contention must remain refused");
        held.close().unwrap();
        validate_after_gate(true,probe).unwrap();
        store.close().unwrap();
        fs::remove_dir_all(path).unwrap();
    }

    /// No-grant evidence is explicit and cannot arise from legacy/partial rows.
    /// Pure projection test only; public traced failure proves actual revocation.
    #[test]
    fn unadmitted_requires_runtime_observation_and_exact_original_shape() {
        let mut receipt = Receipt {
            schema_version: 1,
            run: Run {
                claim_id: "fixture".into(),
                work_id: "fixture".into(),
                scope_id: "fixture".into(),
                holder: "fixture".into(),
                generation: 1,
                scope_revision: 1,
                prepared_revision: 1,
                settings_revision: 1,
                owner_pid: 1,
                owner_stamp: "fixture".into(),
                owner_host: "fixture".into(),
                child_pid: None,
                parent_pid: None,
                group_id: None,
                child_stamp: None,
                phase: Phase::Claimed,
                permission: 0,
                host_scope: "fixture".into(),
                preflight_until: "fixture".into(),
                context_sha256: "fixture".into(),
                workflow_prompt_sha256: "fixture".into(),
                prompt_sha256: "fixture".into(),
                provider_digest: "fixture".into(),
                directory_device: "fixture".into(),
                directory_inode: "fixture".into(),
                decision_event: "fixture".into(),
                prepared_at: "fixture".into(),
                receipt_sha256: None,
                applied_revision: None,
                refusal_code: None,
            },
            finished_at: "fixture".into(),
            exit_code: None,
            signal: None,
            output_complete: false,
            proposal: None,
            usage: UsageRecord {
                claim_id: "fixture".into(),
                quality: "incomplete".into(),
                input_tokens: None,
                output_tokens: None,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
                cached_input_tokens: None,
                provider_reported_cost_usd: None,
                source: None,
                at: None,
                reported_model: None,
                model_source: None,
                model_at: None,
            },
            facts: crate::GoRaw::default(),
            failure: Some("fixture".into()),
            grant_attempted: Some(false),
            failure_causes: None,
            gate: None,
            stderr: Vec::new(),
            stderr_truncated: false,
        };
        assert!(receipt.unadmitted());
        for observed in [None, Some(true)] {
            receipt.grant_attempted = observed;
            assert!(!receipt.unadmitted());
        }
        receipt.grant_attempted = Some(false);
        for phase in [
            Phase::Registered,
            Phase::Permitted,
            Phase::Refused,
            Phase::Applied,
            Phase::Unresolved,
        ] {
            receipt.run.phase = phase;
            assert!(!receipt.unadmitted());
        }
        receipt.run.phase = Phase::Claimed;
        receipt.run.permission = 1;
        assert!(!receipt.unadmitted());
        receipt.run.permission = 0;
        receipt.run.child_pid = Some(2);
        assert!(!receipt.unadmitted());
        receipt.run.child_pid = None;
        receipt.run.parent_pid = Some(2);
        assert!(!receipt.unadmitted());
        receipt.run.parent_pid = None;
        receipt.run.group_id = Some(2);
        assert!(!receipt.unadmitted());
        receipt.run.group_id = None;
        receipt.run.child_stamp = Some("fixture".into());
        assert!(!receipt.unadmitted());
        receipt.run.child_stamp = None;
        assert!(receipt.unadmitted());
    }

    /// A known applied/replayed Work must survive a later release failure.
    /// This tests error projection only, without claiming a SQL or kernel fault.
    #[test]
    fn known_decision_survives_late_release_failure() {
        for replayed in [false, true] {
            let outcome = PlanningRunOutcome {
                schema_version: 1,
                claim_id: "claim".into(),
                decision_event: "decision".into(),
                decision_applied: true,
                accepted: false,
                replayed,
                work: Work {
                    id: "work".into(),
                    revision: 7,
                    ..Default::default()
                },
            };
            let error = planning_result_finished(Ok(outcome), Err(E::Refused("fixture-release")))
                .err()
                .expect("late failure must remain an error");
            assert_eq!(error.recorded_work(), Some(("work", 7)));
        }
        let error = planning_result_finished(
            Err(E::Refused("fixture-before-known-result")),
            Err(E::Refused("fixture-release")),
        )
        .err()
        .expect("paired failures must remain errors");
        assert!(error.recorded_work().is_none());
        match error {
            E::Cleanup {
                primary: Some(primary),
                cleanup,
            } => {
                assert!(matches!(
                    *primary,
                    E::Refused("fixture-before-known-result")
                ));
                assert!(matches!(*cleanup, E::Refused("fixture-release")));
            }
            _ => panic!("original and cleanup causes must both survive"),
        }
    }
    /// Feedback publication may be known even when its release failed. Preserve
    /// the NEW revision and both causes, never an older claim-only projection.
    #[test]
    fn feedback_release_keeps_newer_work_and_original_cause() {
        let primary = E::Refused("fixture-original-business-refusal");
        let cleanup = E::WorkRecorded {
            primary: Box::new(E::Refused("fixture-feedback-release")),
            work: "work".into(),
            revision: 9,
        };
        let error = feedback_error(primary, Err(cleanup));
        assert_eq!(error.recorded_work(), Some(("work", 9)));
    }
    /// Pure origin comparison fixture, not persisted identity or execution right.
    /// No kernel observation, Root mutation or business acceptance is constructed.
    fn origin() -> Run {
        Run {
            claim_id: "claim".into(),
            work_id: "work".into(),
            scope_id: "root".into(),
            holder: "holder".into(),
            generation: 1,
            scope_revision: 1,
            prepared_revision: 1,
            settings_revision: 1,
            owner_pid: 1,
            owner_stamp: "1".into(),
            owner_host: "fixture-host".into(),
            child_pid: Some(2),
            parent_pid: Some(1),
            group_id: Some(2),
            child_stamp: Some("2".into()),
            phase: Phase::Permitted,
            permission: 1,
            host_scope: "0".repeat(64),
            preflight_until: "2026-10-20T00:01:00Z".into(),
            context_sha256: "0".repeat(64),
            workflow_prompt_sha256: "0".repeat(64),
            prompt_sha256: "0".repeat(64),
            provider_digest: "0".repeat(64),
            directory_device: "1".into(),
            directory_inode: "2".into(),
            decision_event: "decision".into(),
            prepared_at: "2026-10-20T00:00:00Z".into(),
            receipt_sha256: None,
            applied_revision: None,
            refusal_code: None,
        }
    }
    /// Classification follows typed known rollback, not the wording of errors.
    /// Identical text inside a storage/rollback/late-cleanup branch cannot retire
    /// an uncertain call or turn a failed decision into a successful result.
    #[test]
    fn business_refusal_requires_bare_typed_rolled_back_decision() {
        let refusal = || {
            WorkEngineError::PlanningActivation(Box::new(A::Refused(
                "planning-operation-requirements-invalid",
            )))
        };
        assert_eq!(
            business_refusal(&E::Mutation(refusal())),
            Some("planning-operation-requirements-invalid")
        );
        let sql = || {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(5),
                Some("planning-operation-requirements-invalid".into()),
            )
        };
        assert!(business_refusal(&E::Mutation(WorkEngineError::PlanningActivation(Box::new(A::caused(
            "planning-closure-physical-unconfirmed", A::Refused("planning-closure-native-run-active")
        ))))).is_none());
        assert!(business_refusal(&E::Mutation(WorkEngineError::Sql(sql()))).is_none());
        assert!(business_refusal(&E::Mutation(WorkEngineError::Rollback {
            primary: Box::new(refusal()),
            cleanup: sql()
        }))
        .is_none());
        assert!(business_refusal(&E::Cleanup {
            primary: Some(Box::new(E::Mutation(refusal()))),
            cleanup: Box::new(E::Sql(sql()))
        })
        .is_none());
    }
    /// A terminal record and its earlier journal share the same origin in BOTH
    /// argument positions. Terminal fields are checked by replay's own guard;
    /// owner/child/permission/source changes must still differ in either order.
    #[test]
    fn replay_origin_is_symmetric_without_erasing_authority() {
        let original = origin();
        let mut applied = original.clone();
        applied.phase = Phase::Applied;
        applied.receipt_sha256 = Some("a".repeat(64));
        applied.applied_revision = Some(3);
        assert!(same_record(&original, &applied));
        assert!(same_record(&applied, &original));
        assert!(same_record(&applied, &applied));
        let mut changed = applied.clone();
        changed.owner_stamp = "3".into();
        assert!(!same_record(&changed, &original));
        assert!(!same_record(&original, &changed));
        let mut changed = applied.clone();
        changed.child_stamp = Some("3".into());
        assert!(!same_record(&changed, &original));
        let mut changed = applied.clone();
        changed.permission = 0;
        assert!(!same_record(&original, &changed));
        let mut changed = applied.clone();
        changed.provider_digest = "b".repeat(64);
        assert!(!same_record(&original, &changed));
    }
}

#[cfg(test)]
mod interruption_failure_tests{
    use super::*;
    /// A valid no-failure witness precedes stop, process/stop and gate/stop
    /// combinations; secondary errors prevent the clean-stop marker.
    #[test]
    fn planner_interruption_keeps_process_and_gate_causes(){
        assert!(stopped_failure(None,None,false).is_none());
        let pure=stopped_failure(None,Some("planning-execution-interrupted"),false).unwrap();
        let wrap=|primary|E::WorkRecorded{primary:Box::new(primary),work:"fixture".into(),revision:3};
        assert!(wrap(pure).recorded_planner_interruption().is_some());
        let combined=stopped_failure(Some(E::Refused("fixture-process-failed")),Some("planning-execution-interrupted"),false).unwrap();
        assert!(wrap(combined).recorded_planner_interruption().is_none());
        let gate=stopped_failure(None,Some("planning-execution-interrupted"),true).unwrap();
        match &gate{E::Cleanup{primary:Some(primary),cleanup}=>{assert!(matches!(primary.as_ref(),E::Refused("planning-execution-interrupted")));assert!(matches!(cleanup.as_ref(),E::Refused("planning-provider-gate-refused")));},_=>panic!("both stop and gate causes required")}
        assert!(wrap(gate).recorded_planner_interruption().is_none());
    }
}
