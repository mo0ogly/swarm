//! Native mission leases and fenced Work controls; this does not start a supervisor.
use super::{RootBinding, RootStore, RootStoreError};
use crate::{
    mission_supervision::{finish, parse},
    EngineSettingsDocument, MissionLeaseClock, MissionSupervisionError as E,
    MissionSupervisionPolicy, SettingCount, SupervisorIdentity,
};
use rusqlite::{params, Connection, Params, Row};

#[path = "mission_worker_adoption.rs"]
mod worker_adoption;
#[path = "mission_worker_recovery.rs"]
mod worker_recovery;
pub use worker_recovery::MissionWorkerRecovery;
pub use worker_adoption::{MissionWorkerAdoptionRequest, MissionWorkerAdoptionOutcome};

// Representation schema, not an operator policy. No FK: generations survive work
// deletion/recreation. Go schema37 remains unchanged; restoration needs quiescence.
const TABLE: &str = "CREATE TABLE native_mission_lease_epochs(work_id TEXT PRIMARY KEY,generation INTEGER NOT NULL CHECK(generation>0),conductor_id TEXT NOT NULL,pid INTEGER NOT NULL,process_stamp TEXT NOT NULL,host TEXT NOT NULL,poll_ms INTEGER NOT NULL,stale_ms INTEGER NOT NULL,future_ms INTEGER NOT NULL)";
/// Non-deserializable possession handle, tied to an opened datastore descriptor,
/// current Linux process and persistent epoch. A label alone cannot construct it.
/// Same-UID filesystem access remains trusted; this is not authenticated agent
/// identity, human consent or fencing of unrelated Work mutations. Explicitly close.
pub struct MissionLease {
    work: String,
    generation: i64,
    conductor: String,
    identity: SupervisorIdentity,
    binding: RootBinding,
    policy: MissionSupervisionPolicy,
    settings_revision: SettingCount,
}
impl std::fmt::Debug for MissionLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MissionLease(<private>)")
    }
}
impl MissionLease {
    /// Persistent generation is observation, not a caller-supplied capability.
    pub fn generation(&self) -> i64 {
        self.generation
    }
    /// Observe the original directory and database through the retained handles.
    /// No pathname or caller tuple is adopted. IO failures preserve Root causes;
    /// the observation grants no authority without this actual owning lease.
    pub(crate) fn instance(&self) -> Result<super::RootInstanceIdentity, E> {
        self.binding.instance().map_err(root)
    }
    /// Frozen policy from the validated settings revision at successful claim.
    pub fn policy(&self) -> &MissionSupervisionPolicy {
        &self.policy
    }
    /// Revision identifying the claim policy, not current Admin configuration.
    pub fn settings_revision(&self) -> SettingCount {
        self.settings_revision
    }
    /// Close the owned binding, preserving any close error. Does not stop or renew
    /// the persisted lease; an abandoned lease expires under its frozen policy.
    pub fn close(self) -> Result<(), E> {
        self.binding.close().map_err(root)
    }
}
/// Preserve typed Root errors without rendering paths or private state.
fn root(e: RootStoreError) -> E {
    E::Root(Box::new(e))
}
/// Preserve typed identity/entropy errors with a safe public operation code.
pub(super) fn caused(code: &'static str, e: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Cause {
        code,
        cause: Box::new(e),
    }
}
/// Execute one parameterized write and finalize its statement explicitly.
/// A cleanup failure is not empty success; both typed causes are retained.
pub(super) fn execute(db: &Connection, sql: &str, params: impl Params) -> Result<usize, E> {
    let mut statement = db.prepare(sql).map_err(E::Sql)?;
    let result = statement.execute(params).map_err(E::Sql);
    finish(result, statement.finalize().map_err(E::Sql))
}
/// Read an optional unique row and explicitly finalize after its cursor drops.
/// Zero rows is represented only by None; SQL/decoder/cleanup errors stay errors.
pub(super) fn optional<T>(
    db: &Connection,
    sql: &str,
    params: impl Params,
    read: impl FnOnce(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Option<T>, E> {
    let mut statement = db.prepare(sql).map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement.query(params).map_err(E::Sql)?;
        rows.next()
            .map_err(E::Sql)?
            .map(read)
            .transpose()
            .map_err(E::Sql)
    })();
    finish(result, statement.finalize().map_err(E::Sql))
}
/// Inspect the optional epoch schema without creating it. Exact representation
/// and user indexes/triggers are checked before consuming any native row. Missing
/// means legacy/absent metadata, not permission to adopt current policy.
pub(super) fn epoch_schema(db: &Connection) -> Result<bool, E> {
    auxiliary_schema(
        db,
        "native_mission_lease_epochs",
        TABLE,
        "mission-epoch-schema-invalid",
    )
}
/// Validate one internal optional table without creation or policy adoption.
/// Reserved-name collisions, altered DDL and added indexes/triggers refuse.
pub(super) fn auxiliary_schema(db: &Connection, name: &str, ddl: &str, code: &'static str) -> Result<bool, E> {
    let collisions: i64 = optional(
        db,
        "SELECT count(*) FROM main.sqlite_schema WHERE name=? AND type!='table'",
        [name],
        |r| r.get(0),
    )?
    .ok_or(E::Refused("mission-storage-row-missing"))?;
    if collisions != 0 {
        return Err(E::Refused(code));
    }
    let sql: Option<String> = optional(
        db,
        "SELECT sql FROM main.sqlite_schema WHERE type='table' AND name=?",
        [name],
        |r| r.get(0),
    )?;
    match sql {
        None => return Ok(false),
        Some(sql) if sql == ddl => {}
        Some(_) => return Err(E::Refused(code)),
    }
    let extras:i64=optional(db,"SELECT count(*) FROM main.sqlite_schema WHERE tbl_name=? AND type!='table' AND substr(name,1,7)!='sqlite_'",[name],|r|r.get(0))?.ok_or(E::Refused("mission-storage-row-missing"))?;
    if extras != 0 {
        return Err(E::Refused(code));
    }
    Ok(true)
}
#[path = "mission_supervision_cycles.rs"]
mod cycles;
#[path = "mission_cycle_runner.rs"]
mod runner;
pub use runner::{MissionCycleFailure, MissionCycleMonitor, MissionCycleCause, MissionCycleDiagnostics};
#[path = "mission_supervision_receipts.rs"]
mod receipts;
pub use receipts::MissionWorkReceipt;
/// Resolve exact persisted claim provenance for the native reader. Absence is
/// explicit legacy unknown; inconsistent history or metadata refuses with cause.
pub(super) fn claim_revision(
    store: &RootStore,
    work: &str,
    generation: i64,
    conductor: &str,
    policy: &MissionSupervisionPolicy,
) -> Result<Option<SettingCount>, E> {
    receipts::claim_revision(store, work, generation, conductor, policy)
}
/// Verify that absence really is unproven legacy, not a detached known claim.
/// This pure SQL read never creates a table or infers identity from its label.
pub(super) fn legacy_unproven(db: &Connection, work: &str, conductor: &str) -> Result<(), E> {
    receipts::legacy_unproven(db, work, conductor)
}
/// Create only the absent native representation within the writer transaction.
/// Existing definitions are validated by the same read-only contract as status.
fn schema(db: &Connection) -> Result<(), E> {
    if !epoch_schema(db)? {
        db.execute_batch(TABLE).map_err(E::Sql)?;
    }
    Ok(())
}
/// Execute one coherent SQL read snapshot, preserving query and mandatory
/// rollback failures together. The caller owns this autocommit connection and
/// any required settings reservation; this helper never adopts policy, creates
/// tables, retries, commits a mutation or grants ownership. BEGIN failures leave
/// their original typed cause; an existing transaction is not silently reused.
pub(super) fn read_transaction<T>(
    db: &Connection,
    read: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    db.execute_batch("BEGIN DEFERRED").map_err(E::Sql)?;
    let result = read();
    let cleanup = if db.is_autocommit() {
        Ok(())
    } else {
        db.execute_batch("ROLLBACK").map_err(E::Sql)
    };
    finish(result, cleanup)
}
/// Execute one explicit transaction, checking rollback/commit and preserving
/// both failures. Uses saved SQLite busy timeout; no hidden retry or test watchdog.
/// A commit error with autocommit true remains uncertain, never assumed rollback.
fn transaction<T>(db: &Connection, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    db.execute_batch("BEGIN IMMEDIATE").map_err(E::Sql)?;
    match operation() {
        Ok(value) => match db.execute_batch("COMMIT") {
            Ok(()) => Ok(value),
            Err(cause) => {
                let active = !db.is_autocommit();
                let rollback = if active {
                    db.execute_batch("ROLLBACK")
                } else {
                    Ok(())
                };
                let rollback_confirmed = active && rollback.is_ok() && db.is_autocommit();
                Err(E::Commit {
                    cause: Box::new(cause),
                    rollback_confirmed,
                    cleanup: rollback.err().map(Box::new),
                })
            }
        },
        Err(primary) => {
            let cleanup = if db.is_autocommit() {
                Ok(())
            } else {
                db.execute_batch("ROLLBACK").map_err(E::Sql)
            };
            finish(Err(primary), cleanup)
        }
    }
}
/// Require the Root/process binding before SQL access, including after fork.
/// Stamps are PID reuse protection, not model identity or OS confinement.
fn bound(store: &RootStore, lease: &MissionLease) -> Result<(), E> {
    if !lease.binding.matches(store).map_err(root)? {
        return Err(E::Refused("mission-lease-root-mismatch"));
    }
    let current =
        SupervisorIdentity::current().map_err(|e| caused("mission-process-unavailable", e))?;
    if current.pid() != lease.identity.pid()
        || current.process_stamp() != lease.identity.process_stamp()
        || current.host() != lease.identity.host()
    {
        return Err(E::Refused("mission-lease-process-mismatch"));
    }
    Ok(())
}
/// Borrow only the Work identity after the actual Root descriptor/process check.
/// This neither exposes the binding nor checks time/SQL ownership or grants a
/// launch: the replay/sync writers must repeat their actual fenced guards.
pub(super) fn bound_mission_work<'a>(store: &RootStore, lease: &'a MissionLease) -> Result<&'a str, E> {
    bound(store, lease)?;
    Ok(&lease.work)
}
/// Hold the existing shared settings reservation, read/validate persisted policy,
/// and release explicitly. A late release failure retains the known lease write.
/// This is one SQL operation; contention is returned, not silently retried.
fn write<T>(
    store: &RootStore,
    operation: impl FnOnce(&EngineSettingsDocument) -> Result<T, E>,
) -> Result<T, E> {
    if !store.writable {
        return Err(E::Refused("mission-root-read-only"));
    }
    let lock = store
        .directory
        .reserve(false, false)
        .map_err(|e| root(RootStoreError::Storage(e)))?;
    let result = (|| {
        let settings = store.read_settings().map_err(root)?;
        store
            .db
            .busy_timeout(std::time::Duration::from_millis(
                u64::try_from(settings.values.storage_busy_timeout_ms.0)
                    .map_err(|e| caused("mission-storage-timeout-invalid", e))?,
            ))
            .map_err(E::Sql)?;
        transaction(&store.db, || operation(&settings))
    })();
    let recorded = result.is_ok();
    let joined = finish(
        result,
        lock.close().map_err(|e| root(RootStoreError::Storage(e))),
    );
    match joined {
        Err(primary) if recorded => Err(E::Recorded {
            primary: Box::new(primary),
        }),
        other => other,
    }
}
/// Return persisted frozen policy for a native row; absent legacy metadata refuses
/// takeover rather than inventing its policy. Row ownership and positive epoch
/// are checked; clock classification alone grants no replacement permission.
pub(super) fn previous(
    db: &Connection,
    work: &str,
    conductor: &str,
) -> Result<(i64, MissionSupervisionPolicy), E> {
    let row = optional(db, "SELECT generation,conductor_id,poll_ms,stale_ms,future_ms FROM native_mission_lease_epochs WHERE work_id=?", [work], |r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?)))?.ok_or(E::Refused("mission-legacy-lease-requires-migration"))?;
    if row.0 < 1 || row.1 != conductor {
        return Err(E::Refused("mission-epoch-owner-invalid"));
    }
    let policy = MissionSupervisionPolicy {
        poll_interval_ms: SettingCount(row.2),
        stale_after_ms: SettingCount(row.3),
        future_tolerance_ms: SettingCount(row.4),
    };
    policy.validate().map_err(E::Settings)?;
    Ok((row.0, policy))
}
/// Read the live owner in a caller-owned writer transaction. The caller must
/// already check Root/current-process binding and hold the SQLite writer; this
/// helper never acquires authority or renews time. Missing, stopped or replaced
/// owners refuse. Exact epoch/policy/PID tuple validation is shared by heartbeat,
/// cycle recording and Work fencing; SQL/schema failures preserve typed causes.
fn owned_heartbeat(db: &Connection, lease: &MissionLease) -> Result<String, E> {
    if !epoch_schema(db)? {
        return Err(E::Refused("mission-lease-lost"));
    }
    let owner=optional(db,"SELECT e.pid,e.process_stamp,e.host,m.heartbeat_at FROM native_mission_lease_epochs e JOIN mission_supervision m USING(work_id) WHERE e.work_id=? AND m.conductor_id=? AND m.stopped_at=''",params![lease.work,lease.conductor],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)))?.ok_or(E::Refused("mission-lease-lost"))?;
    let (generation, policy) = previous(db, &lease.work, &lease.conductor)?;
    if generation != lease.generation
        || policy != lease.policy
        || owner.0 != lease.identity.pid()
        || owner.1 != lease.identity.process_stamp()
        || owner.2 != lease.identity.host()
    {
        return Err(E::Refused("mission-lease-lost"));
    }
    Ok(owner.3)
}
/// Compute the next check with checked datetime representation. i64 milliseconds
/// are valid policy; a timestamp outside RFC3339 range refuses without mutation.
fn next(now: &str, policy: &MissionSupervisionPolicy) -> Result<String, E> {
    let at = parse(now)?
        .checked_add(time::Duration::milliseconds(policy.poll_interval_ms.0))
        .ok_or(E::Refused("mission-next-time-unrepresentable"))?;
    crate::format_utc_timestamp(at.unix_timestamp(), i64::from(at.nanosecond()))
        .map_err(|e| caused("mission-next-time-unrepresentable", e))
}
impl RootStore {
    /// Acquire a native mission lease under saved v21 settings. The source is a
    /// nonempty printable attribution label, never identity. Requires an existing
    /// valid Work and writable trusted local Root. Supplied RFC3339 time is an
    /// observation, not authenticated. Active leases always conflict, including
    /// same-label/process claims; stopped/stale native leases increment the epoch.
    /// Legacy/future/malformed leases refuse pending explicit migration/diagnosis.
    /// A normalized receipt retains the exact claim settings revision and local
    /// process observations in the same transaction. On success caller owes
    /// handle.close and Root.close. Supported Work commands must explicitly use
    /// execute_mission_external for transactional fencing; acquisition alone
    /// starts no provider and grants no business decision or acceptance.
    pub fn claim_mission_supervision(
        &self,
        work: &str,
        source: &str,
        now: &str,
    ) -> Result<MissionLease, E> {
        parse(now)?;
        if source.trim().is_empty() || source.chars().any(char::is_control) {
            return Err(E::Refused("mission-source-invalid"));
        }
        let identity =
            SupervisorIdentity::current().map_err(|e| caused("mission-process-unavailable", e))?;
        let conductor = crate::generate_identifier("mission-")
            .map_err(|e| caused("mission-entropy-unavailable", e))?;
        let binding = self.bind_root().map_err(root)?;
        let result = write(self, |settings| {
            let policy = settings
                .values
                .mission_supervision_policy()
                .map_err(E::Settings)?;
            let next = next(now, &policy)?;
            crate::read_recorded_work(&self.db, work).map_err(|e| root(RootStoreError::Work(e)))?;
            schema(&self.db)?;
            let row = optional(&self.db, "SELECT conductor_id,heartbeat_at,stopped_at FROM mission_supervision WHERE work_id=?", [work], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?;
            if let Some((old, heartbeat, stopped)) = &row {
                let (_, old_policy) = previous(&self.db, work, old)?;
                if stopped.is_empty() {
                    match old_policy.observe(heartbeat, now)? {
                        MissionLeaseClock::Present => {
                            return Err(E::Refused("mission-lease-already-held"))
                        }
                        MissionLeaseClock::Future => {
                            return Err(E::Refused("mission-lease-clock-future"))
                        }
                        MissionLeaseClock::Stale => {}
                    }
                }
            }
            let epoch: Option<i64> = optional(
                &self.db,
                "SELECT generation FROM native_mission_lease_epochs WHERE work_id=?",
                [work],
                |r| r.get(0),
            )?;
            let generation = epoch
                .unwrap_or(0)
                .checked_add(1)
                .filter(|n| *n > 0)
                .ok_or(E::Refused("mission-epoch-exhausted"))?;
            execute(&self.db, "INSERT INTO native_mission_lease_epochs VALUES(?,?,?,?,?,?,?,?,?) ON CONFLICT(work_id) DO UPDATE SET generation=excluded.generation,conductor_id=excluded.conductor_id,pid=excluded.pid,process_stamp=excluded.process_stamp,host=excluded.host,poll_ms=excluded.poll_ms,stale_ms=excluded.stale_ms,future_ms=excluded.future_ms",params![work,generation,conductor,identity.pid(),identity.process_stamp(),identity.host(),policy.poll_interval_ms.0,policy.stale_after_ms.0,policy.future_tolerance_ms.0])?;
            let count = execute(&self.db, "INSERT INTO mission_supervision VALUES(?,?,?,?,?,'','','',?,'') ON CONFLICT(work_id) DO UPDATE SET conductor_id=excluded.conductor_id,source=excluded.source,started_at=excluded.started_at,heartbeat_at=excluded.heartbeat_at,reconciled_at='',last_check_at='',last_error='',next_check_at=excluded.next_check_at,stopped_at=''",params![work,conductor,source,now,now,next])?;
            if count != 1 {
                return Err(E::Refused("mission-lease-write-missing"));
            }
            receipts::stage_claim(
                &self.db,
                receipts::ClaimInput {
                    work,
                    generation,
                    conductor: &conductor,
                    source,
                    started: now,
                    identity: &identity,
                    policy: &policy,
                    settings,
                },
            )?;
            Ok((generation, policy, settings.revision))
        });
        match result {
            Ok((generation, policy, settings_revision)) => Ok(MissionLease {
                work: work.into(),
                generation,
                conductor,
                identity,
                binding,
                policy,
                settings_revision,
            }),
            Err(primary) => finish(Err(primary), binding.close().map_err(root)),
        }
    }
    /// Renew a current native lease with its frozen policy, or mark it stopped.
    /// Root/process/epoch and persisted owner tuple must still agree in the writer
    /// transaction. Expired renewals refuse; stop may retire an expired own lease.
    /// A replaced/stopped handle never revives it. Errors preserve SQL/clock/close
    /// causes; no silent no-op, auto retry, provider signal or acceptance occurs.
    pub fn update_mission_supervision(
        &self,
        lease: &MissionLease,
        now: &str,
        stop: bool,
    ) -> Result<(), E> {
        bound(self, lease)?;
        parse(now)?;
        write(self, |_| {
            let heartbeat = owned_heartbeat(&self.db, lease)?;
            let policy = &lease.policy;
            if !stop && policy.observe(&heartbeat, now)? != MissionLeaseClock::Present {
                return Err(E::Refused("mission-lease-not-present"));
            }
            let next = if stop {
                String::new()
            } else {
                next(now, policy)?
            };
            let count = execute(&self.db, "UPDATE mission_supervision SET heartbeat_at=?,next_check_at=?,stopped_at=? WHERE work_id=? AND conductor_id=? AND stopped_at=''",params![now,next,if stop {now} else {""},lease.work,lease.conductor])?;
            if count != 1 {
                return Err(E::Refused("mission-lease-lost"));
            }
            Ok(())
        })
    }
}

impl crate::work_engine::MutationFence for MissionLease {
    /// Couple a canonical new Work event to this generation and both policy
    /// revisions in the same SQL transaction; no replay attribution or auth claim.
    fn stage_receipt(
        &self,
        db: &Connection,
        receipt: crate::work_engine::MutationFenceReceipt<'_>,
    ) -> Result<(), crate::WorkEngineError> {
        receipts::stage_work(db, self, receipt)
            .map_err(|e| crate::WorkEngineError::Mission(Box::new(e)))
    }
    /// Reserve SQLite's writer before observing ownership, then check process,
    /// persistent generation/owner/policy and frozen time bounds in that same tx.
    /// Actual connected DB/directory must match the retained lease before the
    /// first SQL write; a copied datastore cannot borrow this opaque fence.
    /// A no-op pulse does not renew the lease. Refusal rolls back the caller's
    /// entire command; no actor identity, budget or business decision is inferred.
    fn check(&self, db: &Connection, work: &str, now: &str) -> Result<(), crate::WorkEngineError> {
        let result = (|| {
            if work != self.work {
                return Err(E::Refused("mission-lease-work-mismatch"));
            }
            parse(now)?;
            let actual = super::observe_instance(db, &self.binding.directory).map_err(root)?;
            if self.instance()? != actual {
                return Err(E::Refused("mission-lease-root-mismatch"));
            }
            let current = SupervisorIdentity::current()
                .map_err(|e| caused("mission-process-unavailable", e))?;
            if current.pid() != self.identity.pid()
                || current.process_stamp() != self.identity.process_stamp()
                || current.host() != self.identity.host()
            {
                return Err(E::Refused("mission-lease-process-mismatch"));
            }
            let held=execute(db,"UPDATE mission_supervision SET heartbeat_at=heartbeat_at WHERE work_id=? AND conductor_id=? AND stopped_at=''",params![work,self.conductor])?;
            if held != 1 {
                return Err(E::Refused("mission-lease-lost"));
            }
            let heartbeat = owned_heartbeat(db, self)?;
            if self.policy.observe(&heartbeat, now)? != MissionLeaseClock::Present {
                return Err(E::Refused("mission-lease-not-present"));
            }
            Ok(())
        })();
        result.map_err(|e| crate::WorkEngineError::Mission(Box::new(e)))
    }
}
impl RootStore {
    /// Prepare a manual worker intention with additional current-lease fencing.
    /// Root/current process are checked before the common factory. Ownership/time
    /// are checked inside the real admission transaction and before completion;
    /// the agent.start receipt commits with Agent/attempt/Work/event/budget. Preview
    /// rolls all SQL back. Replay requires a live lease, skips provider preflight
    /// and retains original generation attribution. Errors keep typed original
    /// causes and commit/rollback/release state. No provider starts and the origin
    /// remains operator; this neither validates automatic mission consent nor
    /// authorizes execution, authenticated identity or a complete conductor loop.
    pub fn prepare_agent_launch_under_lease<Ev: crate::WorkEnvironment>(
        &mut self,
        lease: &MissionLease,
        request: &crate::Launch,
        preview: bool,
        environment: &mut Ev,
    ) -> Result<crate::AgentLaunchOutcome, RootStoreError> {
        bound(self, lease).map_err(|e| {
            RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused(
                "agent-launch-mission-bound",
                e,
            )))
        })?;
        self.prepare_agent_launch_with_fence(
            &lease.work,
            request,
            preview,
            environment,
            Some(lease),
        )
    }
    /// Apply one supported Work control under an owned native mission lease.
    /// Root/process binding is checked first; owner/epoch/frozen policy and time
    /// are checked within the command's writer transaction and again before commit.
    /// Expiry during staging rolls back Work, budget and events. Replays also need
    /// current ownership and return their existing receipt without lease renewal.
    /// Saved settings/retry/cleanup and original typed causes use the common Root
    /// path. Supplied local environment/time remain unverified observations; the
    /// handle grants no decision, authenticated actor, provider launch or acceptance.
    /// This does not fence separate agent/process mutations or a complete loop.
    pub fn execute_mission_external<Ev: crate::WorkEnvironment>(
        &mut self,
        lease: &MissionLease,
        kind: &str,
        raw: &[u8],
        environment: &mut Ev,
    ) -> Result<super::RootMutationOutcome, RootStoreError> {
        bound(self, lease)
            .map_err(|e| RootStoreError::Mutation(crate::WorkEngineError::Mission(Box::new(e))))?;
        self.execute_with_fence(&lease.work, kind, raw, environment, Some(lease))
    }
}

/// Validate a persisted automatic Agent through its original claim/action proof.
/// BeforeGrant also checks the actual lease owner/clock; Original grants no call.
pub(super) fn verify_agent_binding(db:&Connection,agent:&crate::Agent,binding:&super::mission_agent::Binding,settings:&EngineSettingsDocument,check:super::mission_agent::Check<'_>)->Result<(),E>{
    receipts::verify_agent_binding(db,agent,binding,settings,check)
}
impl RootStore {
    /// Replay a real automatic original before queue synchronization/selection.
    /// Absence is observed under the actual lease writer, before capacity guards;
    /// orphan canonical events/reservations, manual/foreign/revision conflicts or
    /// corruption refuse. Current profiles, consent, capacity, exact command/claim
    /// and FIFO provenance are checked in that same writer. Changed historical
    /// profiles refuse; no metadata is rewritten to manufacture compatibility.
    /// The observed request remains canonical, and a missing reread is an error,
    /// never a prepare fallback. Every result rolls back lease pulses explicitly.
    /// Settings release and all typed failures remain visible; no provider, new
    /// intention, queue sync or grant occurs. Agent.start operation identity alone
    /// does not authenticate its actor or prove it originated in this selector.
    pub(super) fn replay_mission_dispatch_original<Ev: crate::WorkEnvironment>(
        &mut self,
        lease: &MissionLease,
        request: &super::mission_worker_dispatch::MissionWorkerDispatchRequest,
        environment: &mut Ev,
    ) -> Result<Option<crate::AgentLaunchOutcome>, RootStoreError> {
        let caused = |error: E| RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("mission-dispatch-replay", error)));
        request.validate().map_err(caused)?;
        bound(self, lease).map_err(caused)?;
        if !self.writable { return Err(RootStoreError::Refused("root-read-only")); }
        let work = lease.work.clone();
        let mut observation = crate::agent_launch::DispatchReplayObservation::default();
        crate::agent_storage_retry::execute(&mut self.db, &self.directory, false, |db, settings| {
            crate::agent_launch::with_fenced_replay(db, &work, lease, settings, environment, |tx, environment| {
                let Some(original) = crate::agent_launch::recorded_dispatch_launch(tx, &work, &request.event_id, &mut observation)? else { return Ok(None); };
                if original.expected_revision != request.expected_revision || original.conductor_id != lease.conductor {
                    return Err(crate::AgentLaunchError::Refused("agent-launch-event-conflict"));
                }
                let actual = crate::read_recorded_work(tx, &work)
                    .map_err(|e| crate::AgentLaunchError::caused("agent-launch-work-invalid", e))?;
                let parent = actual.launch_profile.as_deref()
                    .ok_or(crate::AgentLaunchError::Refused("mission-profile-required"))?;
                let profile = super::mission_agent::effective_profile(&actual, &original.task_id)
                    .map_err(|e| crate::AgentLaunchError::caused("mission-dispatch-replay-profile", e))?;
                if parent.role != "worker" || profile.role != "worker" {
                    return Err(crate::AgentLaunchError::Refused("mission-worker-profile-role-unported"));
                }
                let fence = super::mission_agent::AutomaticFence {
                    lease, profile: &profile, parent_profile: parent, conductor: &lease.conductor,
                    agent: &request.event_id, task: &original.task_id,
                };
                crate::agent_launch::replay_observed_dispatch(tx, &self.directory, &work, &original, &fence, settings, environment).map(Some)
            }).map_err(|e| RootStoreError::Launch(Box::new(e)))
        })
    }
    /// Observe dispatch resources without manufacturing a managed preview Agent.
    /// Ordinary tasks retain their full common preview. Managed tasks require the
    /// actual owning lease, current saved profiles/settings and rollback-only
    /// resource reader; no copy/host probe/provider/reservation is created.
    /// The later admission repeats all guards and must inspect the physical copy.
    /// This internal observation grants nothing; typed lock/SQL/rollback failures
    /// propagate, including settings-reservation cleanup and safe causes.
    pub(super) fn preview_mission_worker_resources<Ev:crate::WorkEnvironment>(
        &mut self, lease:&MissionLease, request:&super::MissionWorkerRequest, environment:&mut Ev,
    )->Result<(),RootStoreError>{
        if !self.writable { return Err(RootStoreError::Refused("root-read-only")); }
        let work=self.read_work(&lease.work)?;
        if work.planning.as_deref().is_none_or(|p|p.repository.is_none()) {
            return self.prepare_mission_worker(lease,request,true,environment).map(|_|());
        }
        let error=|e:E|RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("mission-worker-admission",e)));
        bound(self,lease).map_err(error)?;
        if request.schema_version!=1 || request.expected_revision<1
            || [&request.event_id,&request.task_id].iter().any(|v|!swarm_identity_contract::name(v.as_bytes())) {
            return Err(error(E::Refused("mission-worker-request-invalid")));
        }
        let parent_profile=work.launch_profile.as_deref().ok_or_else(||error(E::Refused("mission-profile-required")))?;
        let profile=super::mission_agent::effective_profile(&work,&request.task_id).map_err(error)?;
        if parent_profile.role!="worker" || profile.role!="worker" {
            return Err(error(E::Refused("mission-worker-profile-role-unported")));
        }
        let launch=crate::Launch{schema_version:1,event_id:request.event_id.clone(),expected_revision:request.expected_revision,task_id:request.task_id.clone(),provider:profile.provider.clone(),role:"worker".into(),workspace:profile.workspace.clone(),instruction:profile.instruction.clone(),level:profile.level.clone(),timeout_seconds:profile.timeout_seconds,capture_output:profile.capture_output,limits:profile.limits.clone(),skills:profile.skills.clone(),origin:"conducteur".into(),conductor_id:lease.conductor.clone(),..Default::default()};
        let fence=super::mission_agent::AutomaticFence{lease,profile:&profile,parent_profile,conductor:&lease.conductor,agent:&request.event_id,task:&request.task_id};
        let root=self.root.clone();
        crate::agent_storage_retry::execute(&mut self.db,&self.directory,false,|db,settings| {
            crate::agent_launch::preview_managed_resources(db,crate::agent_launch::PreparationRoot {
                root:&root,dir:&self.directory,settings,fence:Some(&fence),
            },&lease.work,&launch,environment).map_err(|e|RootStoreError::Launch(Box::new(e)))
        })
    }
    /// Prepare or preview an automatic worker under this actual native lease.
    /// Provider/model/workspace/limits come from the authoritative saved profile,
    /// not caller-supplied identity labels. The current transaction rechecks lease,
    /// consent, mode, pause/archive, parent and effective task profiles (including
    /// saved model choice), and saved slot policy before/after
    /// staging; the private grant repeats those checks from canonical provenance.
    /// New intentions require explicit sync_mission_workspace_queue first. Their
    /// Binding3 acquires the exact standing FIFO generation atomically with
    /// agent.start; capacity, occupancy or journal failure rolls back acquisition.
    /// Replay creates no second attempt/reservation. Preview rolls back all writes,
    /// including transient turn acquisition; the prior explicit sync is retained.
    /// No provider starts; caller owes the separate native execution path and
    /// handle/Root close. Typed sources/SQL/cleanup failures retain their causes.
    pub fn prepare_mission_worker<Ev:crate::WorkEnvironment>(&mut self,lease:&MissionLease,request:&super::MissionWorkerRequest,preview:bool,environment:&mut Ev)->Result<crate::AgentLaunchOutcome,RootStoreError>{
        let error=|e:E|RootStoreError::Launch(Box::new(crate::AgentLaunchError::caused("mission-worker-admission",e)));
        bound(self,lease).map_err(error)?;
        if request.schema_version!=1 || request.expected_revision<1
            || [&request.event_id,&request.task_id].iter().any(|v|!swarm_identity_contract::name(v.as_bytes())) {
            return Err(error(E::Refused("mission-worker-request-invalid")));
        }
        let work=self.read_work(&lease.work)?;
        let parent_profile=work.launch_profile.as_deref().ok_or_else(||error(E::Refused("mission-profile-required")))?;
        if parent_profile.role!="worker" {return Err(error(E::Refused("mission-worker-profile-role-unported")));}
        let profile=super::mission_agent::effective_profile(&work,&request.task_id).map_err(error)?;
        if profile.role!="worker" {return Err(error(E::Refused("mission-worker-profile-role-unported")));}
        let launch=crate::Launch{schema_version:1,event_id:request.event_id.clone(),expected_revision:request.expected_revision,task_id:request.task_id.clone(),provider:profile.provider.clone(),role:"worker".into(),workspace:profile.workspace.clone(),instruction:profile.instruction.clone(),level:profile.level.clone(),timeout_seconds:profile.timeout_seconds,capture_output:profile.capture_output,limits:profile.limits.clone(),skills:profile.skills.clone(),origin:"conducteur".into(),conductor_id:lease.conductor.clone(),..Default::default()};
        let fence=super::mission_agent::AutomaticFence{lease,profile:&profile,parent_profile,conductor:&lease.conductor,agent:&request.event_id,task:&request.task_id};
        self.prepare_agent_launch_with_fence(&lease.work,&launch,preview,environment,Some(&fence))
    }
}

#[path = "mission_workspace_fifo.rs"]
mod workspace_fifo;
pub use workspace_fifo::{MissionQueueRegistration, MissionCandidateRefusal};
pub(super) use workspace_fifo::schema as workspace_fifo_schema;

#[path = "mission_worker_continuation.rs"]
mod worker_continuation;
pub use worker_continuation::{
    MissionWorkerContinuationObservation, MissionWorkerContinuationEntry,
    MissionWorkerContinuationState, MissionContinuationFailure, MissionContinuationProgress,
};

/// Observe current canonical epoch/parent tuple and frozen clock without creating
/// an owning lease. Private validation/reviewer gates also authenticate peers;
/// this read cannot grant execution, renew ownership or adopt a legacy policy.
/// Typed SQL/schema/clock failures refuse; caller retains its coherent snapshot.
pub(super) fn observe_validation_owner(db:&Connection,work:&str,generation:i64,pid:i64,stamp:&str,host:&str,now:&str)->Result<(),E>{
    if !epoch_schema(db)? { return Err(E::Refused("mission-validation-epoch-missing")); }
    let row=optional(db,"SELECT m.conductor_id,e.pid,e.process_stamp,e.host,m.heartbeat_at FROM mission_supervision m JOIN native_mission_lease_epochs e USING(work_id) WHERE m.work_id=? AND m.stopped_at=''",[work],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?)))?.ok_or(E::Refused("mission-validation-owner-missing"))?;
    let (actual,policy)=previous(db,work,&row.0)?;
    if actual!=generation || row.1!=pid || row.2!=stamp || row.3!=host || policy.observe(&row.4,now)?!=MissionLeaseClock::Present { return Err(E::Refused("mission-validation-owner-changed")); }
    Ok(())
}


impl RootStore {
    /// Open a writable peer for this actual lease's Root and current process.
    /// Check descriptor binding and the live epoch/policy/heartbeat in a coherent
    /// read, finish that snapshot BEFORE the callback's own transactions, and
    /// explicitly close afterwards. No heartbeat, lease acquisition, retry or
    /// launch is performed. The callback still owes its command/private-gate
    /// checks; a pathname or this observation is never an execution grant.
    /// Primary and close errors are both preserved. An error does not prove no
    /// callback effect. A success value is unavailable if close fails; callers
    /// must separately retain known effects (as the planner job report does).
    /// SQLite open/schema reads may recover their own journal;
    /// this has the same trusted-local scope as RootStore::open, not confinement.
    pub fn with_mission_peer<T>(
        root_path: &std::path::Path,
        lease: &MissionLease,
        operation: impl FnOnce(&mut Self) -> Result<T, RootStoreError>,
    ) -> Result<T, RootStoreError> {
        let mut peer = Self::open(root_path, true)?;
        let result = (|| {
            let checked = read_transaction(&peer.db, || {
                bound(&peer, lease)?;
                let heartbeat = owned_heartbeat(&peer.db, lease)?;
                let now = crate::system_timestamp().map_err(|e| caused("mission-peer-clock", e))?;
                if lease.policy.observe(&heartbeat, &now)? != MissionLeaseClock::Present {
                    return Err(E::Refused("mission-lease-not-present"));
                }
                Ok(())
            });
            checked.map_err(|error| RootStoreError::Runtime(crate::RuntimeError::new(
                crate::RuntimeOperation::Platform, error,
            )))?;
            operation(&mut peer)
        })();
        super::finished(result, peer.close())
    }
}

#[cfg(test)]
mod peer_tests {
    use super::*;
    /// Public Root/work/lease setup, never a manufactured SQL owner or claim.
    fn fixture() -> (std::path::PathBuf, RootStore, MissionLease, String) {
        use std::os::unix::fs::PermissionsExt;
        let path=std::env::temp_dir().join(crate::generate_identifier("mission-peer-").unwrap());
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut store=RootStore::initialize(&path,&mut crate::SystemEnvironment).unwrap();
        let request=serde_json::json!({"schema_version":1,"event_id":"peer-create","expected_revision":0,
            "title":"Peer fixture","objective":"Scoped role","scope":"Owned test","criteria":["No implicit grant"]});
        let work=store.execute_external("","work.create",&serde_json::to_vec(&request).unwrap(),&mut crate::SystemEnvironment).unwrap().result.work;
        let lease=store.claim_mission_supervision(&work.id,"peer-fixture",&crate::system_timestamp().unwrap()).unwrap();
        (path,store,lease,work.id)
    }
    /// Close owned test descriptors before deleting only this fresh fixture.
    fn close(path:std::path::PathBuf,store:RootStore,lease:MissionLease){
        lease.close().unwrap();store.close().unwrap();std::fs::remove_dir_all(path).unwrap();
    }
    /// Actual peer opens in a scoped thread; reads/callback errors leave the
    /// heartbeat unchanged. This proves no pulse, not concurrent-writer safety.
    #[test]
    fn mission_peer_scoped_read_and_failure_keep_heartbeat(){
        let (path,store,lease,work)=fixture();
        let before=owned_heartbeat(&store.db,&lease).unwrap();
        let revision=std::thread::scope(|scope|scope.spawn(||RootStore::with_mission_peer(&path,&lease,|peer|{
            assert!(peer.db.is_autocommit());peer.read_work(&work).map(|w|w.revision)
        })).join().unwrap().unwrap());
        assert!(revision>0);
        let error=RootStore::with_mission_peer(&path,&lease,|_|Err::<(),_>(RootStoreError::Refused("fixture-peer-operation"))).unwrap_err();
        assert!(matches!(error,RootStoreError::Refused("fixture-peer-operation")));
        assert_eq!(owned_heartbeat(&store.db,&lease).unwrap(),before);
        close(path,store,lease);
    }
    /// A valid peer precedes wrong-instance/stopped-owner refusal. Neither
    /// refusal enters the callback; no process is adopted or owner reset.
    #[test]
    fn mission_peer_refuses_other_root_and_stopped_owner(){
        let (path,store,lease,work)=fixture();let (other,other_store,other_lease,_)=fixture();
        assert_eq!(RootStore::with_mission_peer(&path,&lease,|peer|peer.read_work(&work).map(|w|w.id)).unwrap(),work);
        let entered=std::cell::Cell::new(false);
        assert!(RootStore::with_mission_peer(&other,&lease,|_|{entered.set(true);Ok(())}).is_err());assert!(!entered.get());
        store.update_mission_supervision(&lease,&crate::system_timestamp().unwrap(),true).unwrap();
        assert!(RootStore::with_mission_peer(&path,&lease,|_|{entered.set(true);Ok(())}).is_err());assert!(!entered.get());
        close(other,other_store,other_lease);close(path,store,lease);
    }
}
