//! Adopt one never-started automatic worker without rewriting its admission.
//! The append-only lineage carries current conductor authority separately from
//! the immutable Agent/agent.start/claim/FIFO/budget originals. It is not a grant.
use super::{
    auxiliary_schema, bound, caused, execute, optional, root, write, MissionLease, RootStore, E,
};
use crate::root_store::mission_agent::{self, Binding, Check};
use crate::work_engine::MutationFence;
use crate::{Agent, EngineSettingsDocument, RootStoreError, SettingCount};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// Representation of an internal append-only journal, not a policy or schema37
// migration. No UPDATE/DELETE or replacement path is provided by this module.
const TABLE: &str = "CREATE TABLE native_mission_worker_adoptions(work_id TEXT NOT NULL,agent_id TEXT NOT NULL,event_id TEXT NOT NULL,generation INTEGER NOT NULL CHECK(generation>0),previous_generation INTEGER NOT NULL CHECK(previous_generation>0),request BLOB NOT NULL,receipt BLOB NOT NULL,PRIMARY KEY(work_id,event_id),UNIQUE(work_id,agent_id,generation),FOREIGN KEY(work_id,generation) REFERENCES native_mission_claim_receipts(work_id,generation))";

/// Caller identities and expected revision only, never a supplied permission,
/// Agent body, Root witness, profile, epoch or reservation. Validating is pure.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MissionWorkerAdoptionRequest {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub agent_id: String,
}
impl MissionWorkerAdoptionRequest {
    /// Reject malformed identities/revision before accessing Root or SQL.
    /// Protocol1/positive revision are representation, not operator policy.
    pub fn validate(&self) -> Result<(), E> {
        if self.schema_version != 1
            || self.expected_revision < 1
            || !swarm_identity_contract::name(self.event_id.as_bytes())
            || !swarm_identity_contract::name(self.agent_id.as_bytes())
        {
            return Err(E::Refused("mission-adoption-request-invalid"));
        }
        Ok(())
    }
}

/// Observed adoption/replay of the same Agent; never proof of provider start or
/// authorization to skip current claim/gate checks. Private Agent data remains a
/// trusted local engine value, not an approved HTTP/CLI DTO.
pub struct MissionWorkerAdoptionOutcome {
    pub agent: Agent,
    pub created: bool,
    pub event_id: String,
    pub generation: i64,
}

/// One retained authority transition. Hashes prove local byte consistency, not
/// authenticated actors or protection against hostile same-UID storage writers.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Adoption {
    schema_version: i64,
    event_id: String,
    work: String,
    agent: String,
    original_binding_sha256: String,
    agent_request_sha256: String,
    original_generation: i64,
    previous_generation: i64,
    generation: i64,
    conductor: String,
    claim_settings_revision: SettingCount,
    command_settings_revision: SettingCount,
    consent_sequence: i64,
    work_revision: i64,
    recorded_at: String,
}

/// Read optional exact journal schema; missing is legacy absence, not permission.
/// Invalid DDL/collisions/triggers refuse rather than being repaired on a read.
fn schema(db: &Connection) -> Result<bool, E> {
    auxiliary_schema(
        db,
        "native_mission_worker_adoptions",
        TABLE,
        "mission-adoption-schema-invalid",
    )
}

/// Preserve static-code typed causes without formatting private bytes/identities.
fn launch(error: crate::AgentLaunchError) -> E {
    root(RootStoreError::Launch(Box::new(error)))
}

/// Hash the exact retained binding, not a newly encoded/normalized replacement.
fn binding_hash(agent: &Agent) -> Result<String, E> {
    let raw = agent
        .native_mission
        .as_ref()
        .ok_or(E::Refused("mission-agent-binding-required"))?;
    Ok(crate::agent_launch_preflight::hash(raw.0.get().as_bytes()))
}

/// Read original SQL request bytes from the same Agent row. The original receipt
/// verifier separately establishes their equality with the immutable agent.start.
fn agent_request(db: &Connection, agent: &str) -> Result<Vec<u8>, E> {
    optional(
        db,
        "SELECT CAST(request AS BLOB) FROM main.agents WHERE id=?",
        [agent],
        |r| r.get(0),
    )?
    .ok_or(E::Refused("mission-adoption-agent-missing"))
}

/// Require retained body lifecycle to match authoritative SQL projections.
/// read_agent overlays status/desired for legacy observation; adoption cannot
/// turn such a contradictory row into a new authority receipt. Unknown body
/// extensions remain untouched, and normal execution performs strict preflight.
fn canonical_agent(db: &Connection, agent: &Agent) -> Result<(), E> {
    let raw: Vec<u8> = optional(
        db,
        "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
        [&agent.id],
        |r| r.get(0),
    )?
    .ok_or(E::Refused("mission-adoption-agent-missing"))?;
    let original =
        crate::decode_agent(&raw).map_err(|e| caused("mission-adoption-agent-body-invalid", e))?;
    if original != *agent {
        return Err(E::Refused("mission-adoption-agent-row-conflict"));
    }
    Ok(())
}

/// Reject registration/terminal/manual/non-worker/stop intentions before any
/// journal write. This observes stored lifecycle only and never probes/signals a
/// PID or decides that an unknown registered process may safely be restarted.
fn eligible(agent: &Agent) -> Result<(), E> {
    if agent.origin != "conducteur"
        || agent.role != "worker"
        || !agent.mode.is_empty()
        || agent.status != "queued"
        || !agent.desired.is_empty()
        || agent.has_unregistered_claim()
        || agent.supervisor_pid != 0
        || agent.child_pid != 0
        || !agent.supervisor_identity.is_empty()
        || !agent.child_identity.is_empty()
        || !agent.heartbeat.is_empty()
        || !agent.ended.is_empty()
        || agent.exit_code.is_some()
        || agent.native_execution.is_some()
        || agent.native_execution_result.is_some()
    {
        return Err(E::Refused("mission-adoption-unstarted-worker-required"));
    }
    Ok(())
}

/// Validate a single chain link against exact original bytes and normalized row
/// headers. No transition may skip its predecessor or go backwards; this does
/// not establish present ownership or permission from historical claim data.
fn link(
    receipt: &Adoption,
    request: &MissionWorkerAdoptionRequest,
    agent: &Agent,
    original: &Binding,
    previous: i64,
    original_hash: &str,
    request_hash: &str,
) -> Result<(), E> {
    request.validate()?;
    if receipt.schema_version != 1
        || receipt.event_id != request.event_id
        || receipt.work != agent.work_id
        || receipt.agent != agent.id
        || request.agent_id != agent.id
        || receipt.work_revision != request.expected_revision
        || receipt.original_generation != original.generation
        || receipt.original_binding_sha256 != original_hash
        || receipt.agent_request_sha256 != request_hash
        || receipt.previous_generation != previous
        || receipt.generation <= previous
        || receipt.claim_settings_revision.0 < 1
        || receipt.command_settings_revision.0 < 1
        || receipt.consent_sequence < 1
        || !swarm_identity_contract::name(receipt.conductor.as_bytes())
    {
        return Err(E::Refused("mission-adoption-lineage-invalid"));
    }
    super::parse(&receipt.recorded_at)?;
    Ok(())
}

/// Read every authority link for this original, preserving statement/finalize
/// failures through the shared query helper. Binding returned is a transient
/// authority projection only: Agent metadata, SQL event/request, original claim
/// and FIFO stay unchanged. Caller MUST validate original provenance first.
/// No schema creation, mutation, live-owner inference or grant occurs here.
pub(super) fn authority(
    db: &Connection,
    agent: &Agent,
    original: &Binding,
    settings: &EngineSettingsDocument,
) -> Result<Binding, E> {
    let mut effective = original.clone();
    if !schema(db)? {
        return Ok(effective);
    }
    let original_hash = binding_hash(agent)?;
    let request_hash = crate::agent_launch_preflight::hash(&agent_request(db, &agent.id)?);
    let rows = crate::agent_launch::query(db,
        "SELECT event_id,generation,previous_generation,request,receipt FROM native_mission_worker_adoptions WHERE work_id=? AND agent_id=? ORDER BY generation,event_id",
        params![agent.work_id, agent.id], |rows| {
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                out.push((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?, row.get::<_, Vec<u8>>(4)?));
            }
            Ok(out)
        }).map_err(launch)?;
    for (event, generation, previous, request_raw, receipt_raw) in rows {
        let request: MissionWorkerAdoptionRequest = swarm_wire_json::decode(&request_raw)
            .map_err(|e| caused("mission-adoption-request-invalid", e))?;
        let receipt: Adoption = swarm_wire_json::decode(&receipt_raw)
            .map_err(|e| caused("mission-adoption-receipt-invalid", e))?;
        link(
            &receipt,
            &request,
            agent,
            original,
            effective.generation,
            &original_hash,
            &request_hash,
        )?;
        if event != receipt.event_id
            || generation != receipt.generation
            || previous != receipt.previous_generation
        {
            return Err(E::Refused("mission-adoption-row-conflict"));
        }
        super::receipts::validate_adoption_claim(
            db,
            &agent.work_id,
            receipt.generation,
            &receipt.conductor,
            receipt.claim_settings_revision,
            receipt.command_settings_revision,
            settings,
        )?;
        effective.generation = receipt.generation;
        effective.conductor = receipt.conductor;
        effective.claim_settings_revision = receipt.claim_settings_revision;
        effective.command_settings_revision = receipt.command_settings_revision;
        effective.consent_sequence = receipt.consent_sequence;
    }
    Ok(effective)
}

/// Classify an exact retained event replay only under its still-current target
/// authority. A changed payload/agent is conflict; a later epoch is stale, never
/// a request to re-adopt or mint a replacement identity. This helper is pure.
fn replay(
    prior: Option<(String, i64, Vec<u8>)>,
    agent: &str,
    raw: &[u8],
    effective: &Binding,
    generation: i64,
    conductor: &str,
) -> Result<bool, E> {
    let Some((id, prior_generation, request)) = prior else {
        return Ok(false);
    };
    if id != agent || request != raw {
        return Err(E::Refused("mission-adoption-replay-conflict"));
    }
    if prior_generation != generation
        || effective.generation != generation
        || effective.conductor != conductor
    {
        return Err(E::Refused("mission-adoption-replay-stale"));
    }
    Ok(true)
}

/// Exact currently held queued data before/after the journal insertion. Amount
/// representation is read by the existing reservation contract; this comparison
/// is not a new ledger writer or a claim about unobserved concurrent connections.
#[derive(PartialEq)]
struct Preserved {
    agent_body: Vec<u8>,
    request: Vec<u8>,
    work_body: Vec<u8>,
    reservation: crate::agent_execution_store::ReservationReceipt,
    turns: Vec<crate::workspace_turn::WorkspaceTurn>,
}
impl Preserved {
    /// Read the current original and reservation/FIFO on the same SQL connection;
    /// all query/decoder/finalize causes propagate, never an empty success.
    fn read(db: &Connection, agent: &Agent) -> Result<Self, E> {
        let agent_body = optional(
            db,
            "SELECT CAST(body AS BLOB) FROM main.agents WHERE id=?",
            [&agent.id],
            |r| r.get(0),
        )?
        .ok_or(E::Refused("mission-adoption-agent-missing"))?;
        let work_body = optional(
            db,
            "SELECT CAST(body AS BLOB) FROM main.works WHERE id=?",
            [&agent.work_id],
            |r| r.get(0),
        )?
        .ok_or(E::Refused("mission-adoption-work-missing"))?;
        Ok(Self {
            agent_body,
            request: agent_request(db, &agent.id)?,
            work_body,
            reservation: crate::agent_execution_store::reservation(db, &agent.id)
                .map_err(launch)?,
            turns: crate::workspace_turn::read(db)
                .map_err(super::super::mission_workspace_queue::error)?,
        })
    }
}

impl RootStore {
    /// Adopt exactly one original Binding4 queued automatic worker under a new
    /// actual lease. Append-only authority receipt links old→new claim; no Agent,
    /// attempt/event/request/Work/FIFO/reservation/consumption is rewritten or
    /// refunded. Current consent, exact saved profiles, configured capacity/cost
    /// and Root instance are checked under the same reserved-settings writer
    /// transaction; provider/context/model rights are rechecked by normal launch
    /// preflight/private gate before any actual execution. This API starts none.
    /// Exact event replay requires the same request/current lease and still an
    /// unstarted eligible worker. Registered/terminal/stop/manual/legacy1–3 refuse.
    /// All refusal rolls back this journal transaction; commit/rollback/settings
    /// close uncertainty retains the existing MissionSupervisionError wrappers.
    pub fn adopt_mission_worker(
        &self,
        lease: &MissionLease,
        request: &MissionWorkerAdoptionRequest,
    ) -> Result<MissionWorkerAdoptionOutcome, E> {
        request.validate()?;
        bound(self, lease)?;
        let raw = serde_json::to_vec(request)
            .map_err(|e| caused("mission-adoption-request-encoding", e))?;
        write(self, |settings| {
            let now = crate::system_timestamp().map_err(|e| caused("mission-adoption-clock", e))?;
            lease
                .check(&self.db, &lease.work, &now)
                .map_err(|e| root(RootStoreError::Mutation(e)))?;
            super::receipts::validate_current_adoption_claim(&self.db, lease, settings)?;
            let work = self.read_work(&lease.work).map_err(root)?;
            let agent = self.read_agent(&request.agent_id).map_err(root)?;
            crate::agent_store::verify_agent_ownership(&self.db, &agent)
                .map_err(|e| root(RootStoreError::Agent(e)))?;
            if agent.work_id != lease.work {
                return Err(E::Refused("mission-adoption-work-mismatch"));
            }
            canonical_agent(&self.db, &agent)?;
            eligible(&agent)?;
            let original = mission_agent::inspect(&self.db, &agent, settings, Check::Original)
                .map_err(launch)?
                .ok_or(E::Refused("mission-agent-binding-required"))?;
            if original.schema_version != 4 {
                return Err(E::Refused("mission-agent-instance-required"));
            }
            // BeforeClaim repeats real opened-instance comparison and all original
            // request/action/FIFO checks; it does not fabricate current grant.
            mission_agent::inspect(
                &self.db,
                &agent,
                settings,
                Check::BeforeClaim(&self.directory),
            )
            .map_err(launch)?;
            let effective = authority(&self.db, &agent, &original, settings)?;
            let prior = if schema(&self.db)? {
                optional(&self.db, "SELECT agent_id,generation,request FROM native_mission_worker_adoptions WHERE work_id=? AND event_id=?",
                    params![lease.work, request.event_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, Vec<u8>>(2)?)))?
            } else {
                None
            };
            let replay = replay(
                prior,
                &agent.id,
                &raw,
                &effective,
                lease.generation,
                &lease.conductor,
            )?;
            if !replay {
                if work.revision != request.expected_revision {
                    return Err(E::Refused("revision_conflict"));
                }
                if effective.generation >= lease.generation {
                    return Err(E::Refused("mission-adoption-new-lease-required"));
                }
                if optional(
                    &self.db,
                    "SELECT 1 FROM events WHERE id=?",
                    [&request.event_id],
                    |r| r.get::<_, i64>(0),
                )?
                .is_some()
                {
                    return Err(E::Refused("mission-adoption-event-conflict"));
                }
            }
            let consent = mission_agent::admission(
                &self.db,
                &work,
                &original.profile_sha256,
                original.parent_profile_sha256.as_deref(),
                &agent.id,
                &agent.task_id,
                settings,
            )?;
            let before = Preserved::read(&self.db, &agent)?;
            if !replay {
                let receipt = Adoption {
                    schema_version: 1,
                    event_id: request.event_id.clone(),
                    work: lease.work.clone(),
                    agent: agent.id.clone(),
                    original_binding_sha256: binding_hash(&agent)?,
                    agent_request_sha256: crate::agent_launch_preflight::hash(&before.request),
                    original_generation: original.generation,
                    previous_generation: effective.generation,
                    generation: lease.generation,
                    conductor: lease.conductor.clone(),
                    claim_settings_revision: lease.settings_revision,
                    command_settings_revision: settings.revision,
                    consent_sequence: consent,
                    work_revision: work.revision,
                    recorded_at: now.clone(),
                };
                // The current lease's original normalized claim is checked before
                // schema creation/INSERT; fabricated receipt fields cannot grant.
                super::receipts::validate_adoption_claim(
                    &self.db,
                    &lease.work,
                    lease.generation,
                    &lease.conductor,
                    lease.settings_revision,
                    settings.revision,
                    settings,
                )?;
                let bytes = serde_json::to_vec(&receipt)
                    .map_err(|e| caused("mission-adoption-receipt-encoding", e))?;
                if !schema(&self.db)? {
                    self.db.execute_batch(TABLE).map_err(E::Sql)?;
                }
                let changed = execute(
                    &self.db,
                    "INSERT INTO native_mission_worker_adoptions VALUES(?,?,?,?,?,?,?)",
                    params![
                        lease.work,
                        agent.id,
                        request.event_id,
                        lease.generation,
                        effective.generation,
                        raw,
                        bytes
                    ],
                )?;
                if changed != 1 {
                    return Err(E::Refused("mission-adoption-not-recorded"));
                }
            }
            let final_authority = authority(&self.db, &agent, &original, settings)?;
            if final_authority.generation != lease.generation
                || final_authority.conductor != lease.conductor
                || final_authority.consent_sequence != consent
                || before != Preserved::read(&self.db, &agent)?
            {
                return Err(E::Refused("mission-adoption-not-preserved"));
            }
            let now = crate::system_timestamp().map_err(|e| caused("mission-adoption-clock", e))?;
            lease
                .check(&self.db, &lease.work, &now)
                .map_err(|e| root(RootStoreError::Mutation(e)))?;
            bound(self, lease)?;
            Ok(MissionWorkerAdoptionOutcome {
                agent,
                created: !replay,
                event_id: request.event_id.clone(),
                generation: lease.generation,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shape-only witness; never an admitted worker or a process capability.
    fn worker() -> Agent {
        Agent {
            id: "worker".into(),
            work_id: "work".into(),
            task_id: "task".into(),
            role: "worker".into(),
            origin: "conducteur".into(),
            status: "queued".into(),
            ..Agent::default()
        }
    }
    /// Current real factory emits empty mode and never-started metadata. Every
    /// invalid synthetic variant is bracketed by the valid structural witness.
    #[test]
    fn adoption_requires_unstarted_automatic_worker_and_preserves_refusal_variants() {
        let control = worker();
        eligible(&control).unwrap();
        for mutate in [
            (|a: &mut Agent| a.origin = "operateur".into()) as fn(&mut Agent),
            |a| a.role = "planner".into(),
            |a| a.mode = "terminal".into(),
            |a| a.status = "starting".into(),
            |a| a.status = "completed".into(),
            |a| a.desired = "stop".into(),
            |a| a.supervisor_pid = 1,
            |a| a.child_pid = 1,
            |a| a.supervisor_identity = "stamp".into(),
            |a| a.child_identity = "stamp".into(),
            |a| a.heartbeat = "retained".into(),
            |a| a.ended = "retained".into(),
            |a| a.exit_code = Some(Box::new(0)),
        ] {
            let mut bad = control.clone();
            mutate(&mut bad);
            assert!(matches!(
                eligible(&bad),
                Err(E::Refused("mission-adoption-unstarted-worker-required"))
            ));
            eligible(&control).unwrap();
        }
    }
    /// Request data grants nothing; unknown fields/invalid identifiers refuse
    /// without Root/SQL and the exact positive request survives every variant.
    #[test]
    fn adoption_request_preserves_explicit_identity_and_rejects_forged_authority() {
        let good = MissionWorkerAdoptionRequest {
            schema_version: 1,
            event_id: "adopt-worker".into(),
            expected_revision: 2,
            agent_id: "worker".into(),
        };
        good.validate().unwrap();
        for mutate in [
            (|r: &mut MissionWorkerAdoptionRequest| r.schema_version = 0)
                as fn(&mut MissionWorkerAdoptionRequest),
            |r| r.expected_revision = 0,
            |r| r.agent_id = "../foreign".into(),
            |r| r.event_id.clear(),
        ] {
            let mut bad = good.clone();
            mutate(&mut bad);
            assert!(bad.validate().is_err());
            good.validate().unwrap();
        }
        let mut value = serde_json::to_value(&good).unwrap();
        value["generation"] = serde_json::json!(2);
        assert!(serde_json::from_value::<MissionWorkerAdoptionRequest>(value).is_err());
    }
    /// Canonical linkage fixture only: its invented directory tuple is not an
    /// owned Root witness or a permission. The public runtime supplies real ones.
    fn original() -> Binding {
        Binding {
            schema_version: 4,
            event_id: "worker".into(),
            generation: 1,
            claim_settings_revision: SettingCount(1),
            command_settings_revision: SettingCount(1),
            work_revision: 2,
            consent_sequence: 1,
            profile_sha256: "c".repeat(64),
            parent_profile_sha256: Some("d".repeat(64)),
            workspace_turn_seq: Some("1".into()),
            conductor: "old-conductor".into(),
            root_instance: Some(crate::root_store::RootInstanceIdentity {
                directory: (1, 2),
                database: (1, 3),
            }),
        }
    }
    /// Every invalid chain starts from a valid link and returns to that witness.
    /// Changed originals/missing predecessor/future source never become authority.
    #[test]
    fn adoption_lineage_preserves_original_and_rejects_missing_or_changed_links() {
        let agent = worker();
        let original = original();
        original.validate().unwrap();
        let request = MissionWorkerAdoptionRequest {
            schema_version: 1,
            event_id: "adopt-worker".into(),
            expected_revision: 3,
            agent_id: agent.id.clone(),
        };
        let original_hash = "a".repeat(64);
        let request_hash = "b".repeat(64);
        let good = Adoption {
            schema_version: 1,
            event_id: request.event_id.clone(),
            work: agent.work_id.clone(),
            agent: agent.id.clone(),
            original_binding_sha256: original_hash.clone(),
            agent_request_sha256: request_hash.clone(),
            original_generation: 1,
            previous_generation: 1,
            generation: 2,
            conductor: "new-conductor".into(),
            claim_settings_revision: SettingCount(1),
            command_settings_revision: SettingCount(1),
            consent_sequence: 2,
            work_revision: 3,
            recorded_at: "2026-10-22T00:00:00Z".into(),
        };
        let valid = |receipt: &Adoption| {
            link(
                receipt,
                &request,
                &agent,
                &original,
                1,
                &original_hash,
                &request_hash,
            )
        };
        valid(&good).unwrap();
        for mutate in [
            (|r: &mut Adoption| r.original_binding_sha256 = "e".repeat(64)) as fn(&mut Adoption),
            |r| r.agent_request_sha256 = "f".repeat(64),
            |r| r.original_generation = 2,
            |r| r.previous_generation = 2,
            |r| r.generation = 1,
            |r| r.agent = "foreign".into(),
            |r| r.work = "foreign".into(),
            |r| r.event_id = "foreign".into(),
            |r| r.work_revision = 4,
            |r| r.claim_settings_revision = SettingCount(0),
            |r| r.consent_sequence = 0,
        ] {
            let mut bad = good.clone();
            mutate(&mut bad);
            assert!(valid(&bad).is_err());
            valid(&good).unwrap();
        }
        let mut next = good.clone();
        next.previous_generation = 2;
        next.generation = 3;
        link(
            &next,
            &request,
            &agent,
            &original,
            2,
            &original_hash,
            &request_hash,
        )
        .unwrap();
        assert!(link(
            &next,
            &request,
            &agent,
            &original,
            1,
            &original_hash,
            &request_hash
        )
        .is_err());
        link(
            &next,
            &request,
            &agent,
            &original,
            2,
            &original_hash,
            &request_hash,
        )
        .unwrap();
    }
    /// Exact event replay is idempotent, not a way to mutate a request or follow
    /// a later lease silently. The caller separately rechecks actual queued state.
    #[test]
    fn adoption_replay_keeps_same_identity_and_refuses_conflict_or_new_epoch() {
        let mut effective = original();
        effective.generation = 2;
        effective.conductor = "current".into();
        let raw = b"original-request";
        let good = || Some(("worker".into(), 2, raw.to_vec()));
        assert!(replay(good(), "worker", raw, &effective, 2, "current").unwrap());
        for prior in [
            Some(("other".into(), 2, raw.to_vec())),
            Some(("worker".into(), 2, b"changed".to_vec())),
            Some(("worker".into(), 1, raw.to_vec())),
        ] {
            assert!(replay(prior, "worker", raw, &effective, 2, "current").is_err());
            assert!(replay(good(), "worker", raw, &effective, 2, "current").unwrap());
        }
        assert!(!replay(None, "worker", raw, &effective, 2, "current").unwrap());
        assert!(replay(good(), "worker", raw, &effective, 3, "next").is_err());
        assert!(replay(good(), "worker", raw, &effective, 2, "current").unwrap());
    }
}
