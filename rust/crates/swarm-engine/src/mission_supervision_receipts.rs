//! Normalized conductor provenance, not authenticated agent or human identity.
use super::{auxiliary_schema, caused, execute, finish, optional, root, MissionLease, RootStore, E};
use crate::{EngineSettingsDocument, MissionSupervisionPolicy, SettingCount, SupervisorIdentity};
use rusqlite::{params, Connection};
const CLAIMS:&str="CREATE TABLE native_mission_claim_receipts(work_id TEXT NOT NULL,generation INTEGER NOT NULL CHECK(generation>0),conductor_id TEXT NOT NULL,source TEXT NOT NULL,started_at TEXT NOT NULL,pid INTEGER NOT NULL,process_stamp TEXT NOT NULL,host TEXT NOT NULL,settings_revision INTEGER NOT NULL CHECK(settings_revision>0),poll_ms INTEGER NOT NULL,stale_ms INTEGER NOT NULL,future_ms INTEGER NOT NULL,PRIMARY KEY(work_id,generation))";
const COMMANDS:&str="CREATE TABLE native_mission_work_receipts(work_id TEXT NOT NULL,event_id TEXT NOT NULL,work_revision INTEGER NOT NULL CHECK(work_revision>0),kind TEXT NOT NULL,generation INTEGER NOT NULL CHECK(generation>0),claim_settings_revision INTEGER NOT NULL CHECK(claim_settings_revision>0),command_settings_revision INTEGER NOT NULL CHECK(command_settings_revision>0),PRIMARY KEY(work_id,event_id),FOREIGN KEY(work_id,generation) REFERENCES native_mission_claim_receipts(work_id,generation))";
/// Validated Root claim input; no request can construct an owning handle from it.
pub(super) struct ClaimInput<'a> {
    pub work: &'a str,
    pub generation: i64,
    pub conductor: &'a str,
    pub source: &'a str,
    pub started: &'a str,
    pub identity: &'a SupervisorIdentity,
    pub policy: &'a MissionSupervisionPolicy,
    pub settings: &'a EngineSettingsDocument,
}
/// Stored normalized facts remain private; callers receive only validated links.
struct ClaimData {
    conductor: String,
    source: String,
    started: String,
    pid: i64,
    stamp: String,
    host: String,
    revision: SettingCount,
    policy: MissionSupervisionPolicy,
}
/// Read exact optional schema; readonly discovery never creates auxiliary tables.
fn claim_schema(db: &Connection) -> Result<bool, E> {
    auxiliary_schema(
        db,
        "native_mission_claim_receipts",
        CLAIMS,
        "mission-claim-schema-invalid",
    )
}
/// Read action-link schema without interpreting malformed storage as no history.
fn command_schema(db: &Connection) -> Result<bool, E> {
    auxiliary_schema(
        db,
        "native_mission_work_receipts",
        COMMANDS,
        "mission-command-schema-invalid",
    )
}
/// Require the exact known settings revision, not a current or matching-value
/// substitute. Root validates the full monotonic journal before supplying it.
fn history(
    settings: &EngineSettingsDocument,
    revision: SettingCount,
) -> Result<&crate::EngineSettingsEntry, E> {
    let index = settings
        .history
        .binary_search_by_key(&revision.0, |entry| entry.revision.0)
        .map_err(|_| E::Refused("mission-claim-provenance-invalid"))?;
    Ok(&settings.history[index])
}
/// Validate the exact historical policy fields at a claimed revision. This is
/// trusted-local journal consistency, not crypto actor/clock/rollback protection.
fn policy_history(
    settings: &EngineSettingsDocument,
    revision: SettingCount,
    policy: &MissionSupervisionPolicy,
) -> Result<(), E> {
    if &history(settings, revision)?
        .change
        .values
        .mission_supervision_policy()
        .map_err(E::Settings)?
        != policy
    {
        return Err(E::Refused("mission-claim-provenance-invalid"));
    }
    Ok(())
}
/// Stage one immutable claim snapshot in the caller-owned acquisition transaction.
/// A duplicate/invalid schema or SQL failure refuses the entire claim; no update,
/// retrospective attribution, cleanup policy or current-default fallback exists.
pub(super) fn stage_claim(db: &Connection, input: ClaimInput<'_>) -> Result<(), E> {
    policy_history(input.settings, input.settings.revision, input.policy)?;
    if !claim_schema(db)? {
        db.execute_batch(CLAIMS).map_err(E::Sql)?;
    }
    let n = execute(
        db,
        "INSERT INTO native_mission_claim_receipts VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
        params![
            input.work,
            input.generation,
            input.conductor,
            input.source,
            input.started,
            input.identity.pid(),
            input.identity.process_stamp(),
            input.identity.host(),
            input.settings.revision.0,
            input.policy.poll_interval_ms.0,
            input.policy.stale_after_ms.0,
            input.policy.future_tolerance_ms.0
        ],
    )?;
    if n != 1 {
        return Err(E::Refused("mission-claim-receipt-missing"));
    }
    Ok(())
}
/// Decode only an existing receipt. Missing is legacy unknown unless an action
/// link already requires it, in which case disappearance is an integrity refusal.
fn claim_data(db: &Connection, work: &str, generation: i64) -> Result<Option<ClaimData>, E> {
    let row = if claim_schema(db)? {
        optional(db,"SELECT conductor_id,source,started_at,pid,process_stamp,host,settings_revision,poll_ms,stale_ms,future_ms FROM native_mission_claim_receipts WHERE work_id=? AND generation=?",params![work,generation],|r|Ok(ClaimData{conductor:r.get(0)?,source:r.get(1)?,started:r.get(2)?,pid:r.get(3)?,stamp:r.get(4)?,host:r.get(5)?,revision:SettingCount(r.get(6)?),policy:MissionSupervisionPolicy{poll_interval_ms:SettingCount(r.get(7)?),stale_after_ms:SettingCount(r.get(8)?),future_tolerance_ms:SettingCount(r.get(9)?)}}))?
    } else {
        None
    };
    if let Some(ref data) = row {
        if data.revision.0 < 1 {
            return Err(E::Refused("mission-claim-provenance-invalid"));
        }
        data.policy.validate().map_err(E::Settings)?;
        super::parse(&data.started)?;
    }
    if row.is_none()
        && command_schema(db)?
        && optional(
            db,
            "SELECT 1 FROM native_mission_work_receipts WHERE work_id=? AND generation=? LIMIT 1",
            params![work, generation],
            |r| r.get::<_, i64>(0),
        )?
        .is_some()
    {
        return Err(E::Refused("mission-claim-provenance-invalid"));
    }
    Ok(row)
}
/// Preserve real legacy absence, but refuse to downgrade a known current claim
/// when its authoritative epoch anchor disappears. Historical unrelated claims
/// do not establish current ownership or force adoption by an old Go conductor.
pub(super) fn legacy_unproven(db: &Connection, work: &str, conductor: &str) -> Result<(), E> {
    if claim_schema(db)? && optional(db,"SELECT 1 FROM native_mission_claim_receipts WHERE work_id=? AND conductor_id=? LIMIT 1",params![work,conductor],|r|r.get::<_,i64>(0))?.is_some(){
        return Err(E::Refused("mission-claim-provenance-invalid"));
    }
    Ok(())
}
/// Resolve a current native lease's receipt against authoritative epoch/lease
/// columns and exact saved history. Legacy absence remains None without writes.
pub(super) fn claim_revision(
    store: &RootStore,
    work: &str,
    generation: i64,
    conductor: &str,
    policy: &MissionSupervisionPolicy,
) -> Result<Option<SettingCount>, E> {
    let Some(data) = claim_data(&store.db, work, generation)? else {
        return Ok(None);
    };
    if data.conductor != conductor || &data.policy != policy {
        return Err(E::Refused("mission-claim-provenance-invalid"));
    }
    let valid=optional(&store.db,"SELECT e.pid=? AND e.process_stamp=? AND e.host=? AND m.source=? AND m.started_at=? AND e.conductor_id=? AND m.conductor_id=? FROM native_mission_lease_epochs e JOIN mission_supervision m USING(work_id) WHERE e.work_id=? AND e.generation=?",params![data.pid,data.stamp,data.host,data.source,data.started,conductor,conductor,work,generation],|r|r.get::<_,bool>(0))?;
    if valid != Some(true) {
        return Err(E::Refused("mission-claim-provenance-invalid"));
    }
    let settings = store.settings().map_err(root)?;
    policy_history(&settings, data.revision, policy)?;
    Ok(Some(data.revision))
}
/// Stage an actual new command link after the authoritative Work event exists.
/// Replay is excluded by caller. Any missing provenance/schema/event refuses and
/// the caller rolls back all SQL effects; this function never commits or retries.
pub(super) fn stage_work(
    db: &Connection,
    lease: &MissionLease,
    receipt: crate::work_engine::MutationFenceReceipt<'_>,
) -> Result<(), E> {
    if receipt.work != lease.work {
        return Err(E::Refused("mission-lease-work-mismatch"));
    }
    let claim = claim_data(db, receipt.work, lease.generation)?
        .ok_or(E::Refused("mission-claim-provenance-invalid"))?;
    if claim.revision != lease.settings_revision
        || claim.conductor != lease.conductor
        || claim.policy != lease.policy
        || claim.pid != lease.identity.pid()
        || claim.stamp != lease.identity.process_stamp()
        || claim.host != lease.identity.host()
    {
        return Err(E::Refused("mission-claim-provenance-invalid"));
    }
    policy_history(receipt.command_settings, claim.revision, &claim.policy)?;
    history(receipt.command_settings, receipt.command_settings.revision)?;
    let valid = optional(
        db,
        "SELECT revision=? AND kind=? FROM main.events WHERE work_id=? AND id=?",
        params![
            receipt.work_revision,
            receipt.kind,
            receipt.work,
            receipt.event
        ],
        |r| r.get::<_, bool>(0),
    )?;
    if valid != Some(true) {
        return Err(E::Refused("mission-command-event-invalid"));
    }
    if !command_schema(db)? {
        db.execute_batch(COMMANDS).map_err(E::Sql)?;
    }
    let n = execute(
        db,
        "INSERT INTO native_mission_work_receipts VALUES(?,?,?,?,?,?,?)",
        params![
            receipt.work,
            receipt.event,
            receipt.work_revision,
            receipt.kind,
            lease.generation,
            claim.revision.0,
            receipt.command_settings.revision.0
        ],
    )?;
    if n != 1 {
        return Err(E::Refused("mission-command-receipt-missing"));
    }
    Ok(())
}
/// Linkage facts only, not actor identity or acceptance. No PID/host/conductor
/// identifier or private request content is exposed. This is a local SDK model,
/// not an approved HTTP DTO. Revisions distinguish claim versus command settings.
#[derive(Clone, PartialEq, Eq)]
pub struct MissionWorkReceipt {
    pub event_id: String,
    pub work_revision: i64,
    pub kind: String,
    pub generation: i64,
    pub claim_settings_revision: SettingCount,
    pub command_settings_revision: SettingCount,
}
impl std::fmt::Debug for MissionWorkReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MissionWorkReceipt(<linkage>)")
    }
}
/// Validate an action link against its historical event and claim rather than
/// an inner join which would silently omit broken proof. No latest-owner inference.
fn validate_link(
    db: &Connection,
    work: &str,
    row: &MissionWorkReceipt,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    let claim = claim_data(db, work, row.generation)?
        .ok_or(E::Refused("mission-claim-provenance-invalid"))?;
    if claim.revision != row.claim_settings_revision {
        return Err(E::Refused("mission-claim-provenance-invalid"));
    }
    policy_history(settings, claim.revision, &claim.policy)?;
    history(settings, row.command_settings_revision)?;
    let valid = optional(
        db,
        "SELECT revision=? AND kind=? FROM main.events WHERE work_id=? AND id=?",
        params![row.work_revision, row.kind, work, row.event_id],
        |r| r.get::<_, bool>(0),
    )?;
    if valid != Some(true) {
        return Err(E::Refused("mission-command-event-invalid"));
    }
    Ok(())
}
/// Hold the authoritative settings reservation across the SQL read snapshot.
/// Settings/BEGIN/query/rollback/release failures retain both typed causes. This
/// prevents Admin from publishing a different policy between file and SQL views;
/// there is no new timer, retry, mutation or default adoption.
fn read_snapshot<T>(
    store: &RootStore,
    read: impl FnOnce(&EngineSettingsDocument) -> Result<T, E>,
) -> Result<T, E> {
    let reservation = store
        .directory
        .reserve(false, false)
        .map_err(|e| root(super::RootStoreError::Storage(e)))?;
    let result = (|| {
        let settings = store.read_settings().map_err(root)?;
        super::read_transaction(&store.db, || read(&settings))
    })();
    finish(
        result,
        reservation
            .close()
            .map_err(|e| root(super::RootStoreError::Storage(e))),
    )
}
impl RootStore {
    /// Read all existing fenced Work links in a coherent read-only SQL snapshot.
    /// A valid Work without this optional journal returns an empty list, not proof
    /// of no prior/manual activity. Unknown Work, broken schema/history/event or
    /// SQL/finalize/rollback failures retain typed causes and refuse the whole
    /// read. No table creation, pruning, retrospective attribution or actor auth.
    pub fn read_mission_work_receipts(&self, work: &str) -> Result<Vec<MissionWorkReceipt>, E> {
        read_snapshot(self, |settings| {
            crate::read_recorded_work(&self.db, work)
                .map_err(|e| root(super::RootStoreError::Work(e)))?;
            if !command_schema(&self.db)? {
                return Ok(vec![]);
            }
            let mut statement=self.db.prepare("SELECT event_id,work_revision,kind,generation,claim_settings_revision,command_settings_revision FROM native_mission_work_receipts WHERE work_id=? ORDER BY work_revision,event_id").map_err(E::Sql)?;
            let rows = (|| {
                let mut rows = statement.query([work]).map_err(E::Sql)?;
                let mut out = Vec::new();
                while let Some(r) = rows.next().map_err(E::Sql)? {
                    out.push(MissionWorkReceipt {
                        event_id: r.get(0).map_err(E::Sql)?,
                        work_revision: r.get(1).map_err(E::Sql)?,
                        kind: r.get(2).map_err(E::Sql)?,
                        generation: r.get(3).map_err(E::Sql)?,
                        claim_settings_revision: SettingCount(r.get(4).map_err(E::Sql)?),
                        command_settings_revision: SettingCount(r.get(5).map_err(E::Sql)?),
                    });
                }
                Ok(out)
            })();
            let out = finish(rows, statement.finalize().map_err(E::Sql))?;
            for row in &out {
                validate_link(&self.db, work, row, settings)?;
            }
            Ok(out)
        })
    }
}

/// Validate a retained adoption's target claim/settings history without treating
/// that historical link as present permission. Called by the append-only journal
/// reader and writer; missing/altered claim or policy retains typed causes.
pub(super) fn validate_adoption_claim(
    db: &Connection, work: &str, generation: i64, conductor: &str,
    claim_revision: SettingCount, command_revision: SettingCount,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    let claim = claim_data(db, work, generation)?
        .ok_or(E::Refused("mission-adoption-claim-required"))?;
    if claim.conductor != conductor || claim.revision != claim_revision
        || claim.pid < 1 || claim.stamp.is_empty() || claim.host.is_empty() {
        return Err(E::Refused("mission-adoption-claim-invalid"));
    }
    policy_history(settings, claim_revision, &claim.policy)?;
    history(settings, command_revision)?;
    Ok(())
}

/// Match the actual new lease to its immutable normalized claim before writing
/// an adoption. Unlike the historical link reader, this also compares the real
/// held process/policy tuple; no JSON conductor or copied claim creates authority.
pub(super) fn validate_current_adoption_claim(
    db: &Connection, lease: &MissionLease, settings: &EngineSettingsDocument,
) -> Result<(), E> {
    validate_adoption_claim(db, &lease.work, lease.generation, &lease.conductor,
        lease.settings_revision, settings.revision, settings)?;
    let claim = claim_data(db, &lease.work, lease.generation)?
        .ok_or(E::Refused("mission-adoption-claim-required"))?;
    if claim.pid != lease.identity.pid() || claim.stamp != lease.identity.process_stamp()
        || claim.host != lease.identity.host() || claim.policy != lease.policy {
        return Err(E::Refused("mission-adoption-claim-invalid"));
    }
    Ok(())
}

/// Check the currently effective owner after original admission verification.
/// An adoption may select a newer claim, but never changes the Agent's immutable
/// original proof. Same host, exact epoch/policy and live conductor stamp remain
/// required immediately before grant; this never probes the provider's vitality.
fn verify_grant_owner(
    db: &Connection, agent: &crate::Agent,
    binding: &crate::root_store::mission_agent::Binding,
    settings: &EngineSettingsDocument,
) -> Result<(), E> {
    let claim = claim_data(db, &agent.work_id, binding.generation)?
        .ok_or(E::Refused("mission-agent-claim-proof-required"))?;
    validate_adoption_claim(db, &agent.work_id, binding.generation, &binding.conductor,
        binding.claim_settings_revision, binding.command_settings_revision, settings)?;
    if !super::epoch_schema(db)? { return Err(E::Refused("mission-lease-lost")); }
    let (generation, policy) = super::previous(db, &agent.work_id, &binding.conductor)?;
    if generation != binding.generation || policy != claim.policy {
        return Err(E::Refused("mission-lease-lost"));
    }
    let heartbeat=optional(db,"SELECT m.heartbeat_at FROM native_mission_lease_epochs e JOIN mission_supervision m USING(work_id) WHERE e.work_id=? AND m.conductor_id=? AND e.conductor_id=m.conductor_id AND m.stopped_at='' AND e.pid=? AND e.process_stamp=? AND e.host=?",params![agent.work_id,binding.conductor,claim.pid,claim.stamp,claim.host],|r|r.get::<_,String>(0))?.ok_or(E::Refused("mission-lease-lost"))?;
    let now = crate::system_timestamp().map_err(|e| caused("mission-agent-clock", e))?;
    if policy.observe(&heartbeat, &now)? != crate::MissionLeaseClock::Present {
        return Err(E::Refused("mission-lease-not-present"));
    }
    let current = SupervisorIdentity::current().map_err(|e| caused("mission-agent-host", e))?;
    if current.host() != claim.host { return Err(E::Refused("mission-agent-other-host")); }
    let pid = u32::try_from(claim.pid).map_err(|e| caused("mission-agent-owner-representation", e))?;
    let actual = crate::observe_linux_process(pid).map_err(|e| caused("mission-agent-owner-unavailable", e))?;
    if actual.process_stamp() != claim.stamp { return Err(E::Refused("mission-agent-owner-replaced")); }
    Ok(())
}

/// Validate actual automatic request/action/claim linkage, never a body flag alone.
/// Caller inspect checks v4 physical-instance provenance before new claim/grant.
/// BeforeGrant validates optional adoption lineage, current effective epoch/owner,
/// kernel stamp/frozen heartbeat policy and current consent/profiles/capacity;
/// Original preserves attribution after revocation for safe queued cancellation.
/// No mutation, renewal, signal or process permission occurs in this reader.
pub(super) fn verify_agent_binding(
    db: &Connection,
    agent: &crate::Agent,
    binding: &crate::root_store::mission_agent::Binding,
    settings: &EngineSettingsDocument,
    check: crate::root_store::mission_agent::Check<'_>,
) -> Result<(), E> {
    if !command_schema(db)? {
        return Err(E::Refused("mission-agent-command-proof-required"));
    }
    let row=optional(db,"SELECT event_id,work_revision,kind,generation,claim_settings_revision,command_settings_revision FROM native_mission_work_receipts WHERE work_id=? AND event_id=?",params![agent.work_id,binding.event_id],|r|Ok(MissionWorkReceipt{event_id:r.get(0)?,work_revision:r.get(1)?,kind:r.get(2)?,generation:r.get(3)?,claim_settings_revision:SettingCount(r.get(4)?),command_settings_revision:SettingCount(r.get(5)?)}))?.ok_or(E::Refused("mission-agent-command-proof-required"))?;
    if row.kind != "agent.start"
        || row.work_revision != binding.work_revision
        || row.generation != binding.generation
        || row.claim_settings_revision != binding.claim_settings_revision
        || row.command_settings_revision != binding.command_settings_revision
    {
        return Err(E::Refused("mission-agent-command-proof-invalid"));
    }
    validate_link(db, &agent.work_id, &row, settings)?;
    let claim = claim_data(db, &agent.work_id, binding.generation)?
        .ok_or(E::Refused("mission-agent-claim-proof-required"))?;
    if claim.conductor != binding.conductor
        || claim.pid < 1
        || claim.stamp.is_empty()
        || claim.host.is_empty()
    {
        return Err(E::Refused("mission-agent-claim-proof-invalid"));
    }
    let raw=optional(db,"SELECT e.request FROM events e JOIN agents a ON a.id=? WHERE e.id=? AND e.work_id=a.work_id AND e.request=a.request",params![agent.id,binding.event_id],|r|r.get::<_,Vec<u8>>(0))?.ok_or(E::Refused("mission-agent-request-proof-invalid"))?;
    let request: crate::Launch =
        crate::work::decode_mode(&raw, crate::agent_models::shape_launch(), true)
            .map_err(|e| caused("mission-agent-request-invalid", e))?;
    if request.event_id != agent.id
        || request.task_id != agent.task_id
        || request.origin != "conducteur"
        || request.conductor_id != binding.conductor
    {
        return Err(E::Refused("mission-agent-request-proof-invalid"));
    }
    // Original attribution remains sufficient for terminal/cancellation. An
    // adoption is consulted only when preparing a new claim/grant; it never
    // rewrites or stands in for the original checks above.
    let effective = if matches!(check, crate::root_store::mission_agent::Check::Original) {
        binding.clone()
    } else {
        super::worker_adoption::authority(db, agent, binding, settings)?
    };
    if matches!(check, crate::root_store::mission_agent::Check::BeforeGrant(_)) {
        verify_grant_owner(db, agent, &effective, settings)?;
        let work = crate::read_recorded_work(db, &agent.work_id)
            .map_err(|e| root(super::RootStoreError::Work(e)))?;
        let sequence = crate::root_store::mission_agent::admission(db, &work,
            &binding.profile_sha256, binding.parent_profile_sha256.as_deref(),
            &agent.id, &agent.task_id, settings)?;
        if sequence != effective.consent_sequence {
            return Err(E::Refused("mission-agent-consent-changed"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Real Root settings application must refuse while a journal SQL snapshot
    /// holds its settings reservation, then the same preview/event recovers after
    /// release. Channel barriers expose the phase, with no elapsed-time guard.
    #[test]
    fn receipt_read_snapshot_holds_settings_reservation_until_rollback() {
        use std::{fs, os::unix::fs::PermissionsExt, sync::mpsc};
        let path =
            std::env::temp_dir().join(crate::generate_identifier("swarm-receipt-read-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut env = crate::SystemEnvironment;
        let parent = RootStore::initialize(&path, &mut env).unwrap();
        let doc = parent.settings().unwrap();
        let mut values = doc.values.clone();
        values.task_max_attempts = Some(SettingCount(4));
        let mut change = crate::EngineSettingsChange {
            schema_version: doc.schema_version,
            event_id: "read-reservation".into(),
            expected_revision: doc.revision,
            values,
            reason: "Fixture shared read reservation".into(),
            preview_token: String::new(),
        };
        change.preview_token = parent.preview_settings(&change).unwrap().preview_token;
        let before = fs::read(path.join(".swarm/engine-settings.json")).unwrap();
        let (arrive_tx, arrive_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let child_path = path.clone();
        std::thread::scope(|scope| {
            let child = scope.spawn(move || {
                let root = RootStore::open(&child_path, false).unwrap();
                let result = read_snapshot(&root, |_| {
                    optional(&root.db, "SELECT count(*) FROM main.works", [], |r| {
                        r.get::<_, i64>(0)
                    })?;
                    arrive_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                });
                root.close().unwrap();
                result.unwrap();
            });
            arrive_rx.recv().unwrap();
            let outcome = parent.apply_settings(&change, &mut env);
            let refused = matches!(&outcome,Err(super::super::RootStoreError::Storage(error)) if error.operation==crate::LocalStorageOperation::Lock);
            release_tx.send(()).unwrap();
            child.join().unwrap();
            assert!(
                refused,
                "Admin write was allowed during journal/settings snapshot"
            );
            assert_eq!(
                fs::read(path.join(".swarm/engine-settings.json")).unwrap(),
                before
            );
        });
        parent.apply_settings(&change, &mut env).unwrap();
        assert_eq!(
            parent.settings().unwrap().values.task_max_attempts,
            Some(SettingCount(4))
        );
        parent.close().unwrap();
        fs::remove_dir_all(path).unwrap();
    }
}
