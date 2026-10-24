//! Engine-owned automatic worker admission, not a caller-supplied identity token.
use super::{
    mission_controls::{consent, controls, MissionAutonomy},
    mission_supervision_store::{caused, optional},
    RootStoreError,
};
use crate::work_engine::{MutationFence, MutationFenceReceipt};
use crate::{
    Agent, AgentLaunchError, EngineSettingsDocument, LaunchProfile, MissionLease,
    MissionSupervisionError as E, Work,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Explicit worker intention identifiers only. The owning Root derives provider,
/// workspace, role and limits from the effective saved task/mission profile. These fields
/// provide no lease, automatic consent, model choice or agent authenticity.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionWorkerRequest {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub task_id: String,
}

/// Strict opaque linkage written by the engine with the actual agent.start
/// event. Alone this data grants nothing: readers must validate its SQL link,
/// original claim and, before grant, current ownership/controls/profile/capacity.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binding {
    pub schema_version: i64,
    pub event_id: String,
    pub generation: i64,
    pub claim_settings_revision: crate::SettingCount,
    pub command_settings_revision: crate::SettingCount,
    pub work_revision: i64,
    pub consent_sequence: i64,
    pub profile_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_hash")]
    pub parent_profile_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_hash")]
    pub workspace_turn_seq: Option<String>,
    pub conductor: String,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_instance")]
    pub root_instance: Option<super::RootInstanceIdentity>,
}
impl Binding {
    /// Validate representation only; no SQL, owner, consent or authorization.
    pub(crate) fn validate(&self) -> Result<(), E> {
        if !matches!((self.schema_version, &self.parent_profile_sha256, &self.workspace_turn_seq, &self.root_instance), (1, None, None, None) | (2, Some(_), None, None) | (3, Some(_), Some(_), None) | (4, Some(_), Some(_), Some(_)))
            || self.workspace_turn_seq.as_ref().is_some_and(|seq| seq.parse::<i64>().map_or(true, |value| value < 1 || value.to_string() != *seq))
            || self.parent_profile_sha256.as_ref().is_some_and(|v| !valid_hash(v))
            || self.generation < 1
            || self.work_revision < 1
            || self.claim_settings_revision.0 < 1
            || self.command_settings_revision.0 < 1
            || self.consent_sequence < 1
            || !swarm_identity_contract::name(self.event_id.as_bytes())
            || !swarm_identity_contract::name(self.conductor.as_bytes())
            || self.profile_sha256.len() != 64
            || !self
                .profile_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(E::Refused("mission-agent-binding-invalid"));
        }
        Ok(())
    }
}
/// Present optional parent-hash/turn-sequence fields must be strings; explicit
/// null is not omission. Binding::validate checks their distinct representations.
/// This wire rule grants no authority and preserves legacy v1/v2 shapes.
fn present_hash<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
/// Decode a present physical-instance observation strictly; null is not legacy
/// omission. Its numbers confer no rights without canonical SQL provenance and
/// comparison with the consumer's actual opened datastore.
fn present_instance<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<super::RootInstanceIdentity>, D::Error> {
    super::RootInstanceIdentity::deserialize(d).map(Some)
}
/// Protocol SHA-256 representation, not an operator policy or identity proof.
fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// Resolve the saved task override, falling back to the required parent profile.
/// Saved model selection overrides provider/level; common launch preflight still
/// checks its policy hash, route and permissions. Missing/duplicate tasks refuse.
/// This pure read authenticates no actor and changes no persisted profile.
pub(super) fn effective_profile(work: &Work, task: &str) -> Result<LaunchProfile, E> {
    let parent = work.launch_profile.as_deref().ok_or(E::Refused("mission-profile-required"))?;
    let mut tasks = work.tasks.iter().flatten().filter(|t| t.id == task);
    let selected = tasks.next().ok_or(E::Refused("mission-worker-task-missing"))?;
    if tasks.next().is_some() { return Err(E::Refused("mission-worker-task-duplicate")); }
    let mut profile = selected.launch_profile.as_deref().unwrap_or(parent).clone();
    if let Some(model) = &selected.model_selection {
        profile.provider.clone_from(&model.provider);
        profile.level.clone_from(&model.route.level);
    }
    Ok(profile)
}
/// Historical provenance suffices to cancel an unstarted owned intention.
/// BeforeClaim and BeforeGrant require Binding4 and the real opened instance.
/// BeforeGrant additionally checks live mission permission. Already released
/// workers keep independent supervision, matching the existing Go contract.
#[derive(Clone, Copy)]
pub(crate) enum Check<'a> {
    Original,
    BeforeClaim(&'a crate::local_storage::LocalDirectory),
    BeforeGrant(&'a crate::local_storage::LocalDirectory),
}

/// Hash exact saved profile bytes, including its engine-observed audit fields.
/// Missing/unencodable profile refuses; a hash proves bytes, not actor identity.
pub(super) fn profile_hash(profile: &LaunchProfile) -> Result<String, E> {
    crate::encode_model(profile)
        .map(|raw| crate::agent_launch_preflight::hash(&raw))
        .map_err(|e| caused("mission-agent-profile-encoding", e))
}
/// Validate current mission permission without capacity or occupancy filtering.
/// Returned controls/sequence are transaction-local observations, not transferable
/// permits. Absence, pause/archive, malformed SQL and consent errors retain cause.
pub(super) fn permission(db: &Connection, work: &Work) -> Result<(super::MissionControlSettings, i64), E> {
    super::require_planning_mission_active(db, &work.id)?;
    let c = controls(db, &work.id)?.ok_or(E::Refused("mission-controls-required"))?;
    if c.autonomy != MissionAutonomy::Automatic {
        return Err(E::Refused("mission-autonomy-required"));
    }
    let approval = consent(db, &work.id)?
        .filter(|c| c.enabled)
        .ok_or(E::Refused("mission-consent-required"))?;
    Ok((c, approval.event_sequence))
}

/// Check current SQL controls and saved Admin policy in the caller transaction.
/// v1 checks only its historical parent hash; v2/v3/v4 also re-resolve the task and
/// its saved model override. Neither metadata version creates permission.
/// The single excluded Agent is the factory/execution's own validated row, so a
/// last-slot intent does not count twice. Current per-task reported-cost policy
/// is checked in the same transaction; task identifies the intended agent task.
/// Missing policy/invalid history/budget or exceeded threshold refuses without
/// clamping, permission default, write, refund or cancellation of released work.
pub(super) fn admission(
    db: &Connection,
    work: &Work,
    profile: &str,
    parent_profile: Option<&str>,
    agent: &str,
    task: &str,
    settings: &EngineSettingsDocument,
) -> Result<i64, E> {
    let (c, sequence) = permission(db, work)?;
    let actual = work
        .launch_profile
        .as_deref()
        .ok_or(E::Refused("mission-profile-required"))?;
    if profile_hash(actual)? != parent_profile.unwrap_or(profile) {
        return Err(E::Refused("mission-agent-profile-changed"));
    }
    if parent_profile.is_some() && profile_hash(&effective_profile(work, task)?)? != profile {
        return Err(E::Refused("mission-agent-task-profile-changed"));
    }
    let slots = settings
        .values
        .mission_dispatch_policy()
        .map_err(E::Settings)?
        .slots(Some(c.slots))
        .map_err(E::Settings)?
        .0;
    let active=optional(db,"SELECT count(*) FROM agents WHERE work_id=? AND id!=? AND status IN ('queued','starting','running','stopping')",params![work.id,agent],|row|row.get::<_,i64>(0))?
        .ok_or(E::Refused("mission-agent-capacity-observation-missing"))?;
    if active >= slots {
        return Err(E::Refused("mission-agent-slots-occupied"));
    }
    super::mission_cost_guard::admit(db, &work.id, task, settings)?;
    Ok(sequence)
}

/// Borrow an actual owning lease and saved profile; callers cannot deserialize
/// this factory capability. Normal lease factories remain explicitly manual.
pub(super) struct AutomaticFence<'a> {
    pub lease: &'a MissionLease,
    pub profile: &'a LaunchProfile,
    pub parent_profile: &'a LaunchProfile,
    pub conductor: &'a str,
    pub agent: &'a str,
    pub task: &'a str,
}
impl MutationFence for AutomaticFence<'_> {
    /// Preserve actual kernel/process/epoch/time fencing before all SQL work.
    fn check(&self, db: &Connection, work: &str, now: &str) -> Result<(), crate::WorkEngineError> {
        self.lease.check(db, work, now)
    }
    /// Recheck permission/capacity using the Root's CURRENT reserved settings.
    fn check_agent(
        &self,
        db: &Connection,
        work: &str,
        now: &str,
        settings: &EngineSettingsDocument,
    ) -> Result<(), crate::WorkEngineError> {
        self.check(db, work, now)?;
        let result = (|| {
            let actual = crate::read_recorded_work(db, work)
                .map_err(|e| E::Root(Box::new(RootStoreError::Work(e))))?;
            admission(
                db,
                &actual,
                &profile_hash(self.profile)?,
                Some(&profile_hash(self.parent_profile)?),
                self.agent,
                self.task,
                settings,
            )?;
            Ok(())
        })();
        result.map_err(|e| crate::WorkEngineError::Mission(Box::new(e)))
    }
    /// Protocol provenance only, never an operator default or JSON permission.
    fn mission_conductor(&self) -> Option<&str> {
        Some(self.conductor)
    }
    /// Automatic attempts preserve the configured mission-wide profile.
    fn mission_profile(&self) -> Option<&LaunchProfile> {
        Some(self.profile)
    }
    /// Bind the actual generated event and both saved policy revisions before
    /// encoding the Agent; v4 observes the retained lease directory and DB handles.
    /// A later SQL failure rolls back body/link together; no caller tuple is used.
    fn mission_binding(
        &self,
        db: &Connection,
        agent: &Agent,
        receipt: MutationFenceReceipt<'_>,
    ) -> Result<Option<crate::GoRaw>, crate::WorkEngineError> {
        let result = (|| {
            let approval = consent(db, receipt.work)?
                .filter(|c| c.enabled)
                .ok_or(E::Refused("mission-consent-required"))?;
            if !super::mission_supervision_store::workspace_fifo_schema(db, false)? {
                return Err(E::Refused("mission-workspace-queue-sync-required"));
            }
            let turns = crate::workspace_turn::read(db).map_err(super::mission_workspace_queue::error)?;
            let turn = turns.iter().find(|turn| turn.work_id == agent.work_id && turn.task_id == agent.task_id && turn.workspace == agent.workspace)
                .ok_or(E::Refused("mission-workspace-queue-sync-required"))?;
            let acquired = crate::workspace_turn_write::acquire(db, turn.seq, &agent.work_id, &agent.task_id, &agent.workspace, &agent.id, &agent.started)
                .map_err(super::mission_workspace_queue::error)?;
            let binding = Binding {
                schema_version: 4,
                event_id: receipt.event.into(),
                generation: self.lease.generation(),
                claim_settings_revision: self.lease.settings_revision(),
                command_settings_revision: receipt.command_settings.revision,
                work_revision: receipt.work_revision,
                consent_sequence: approval.event_sequence,
                profile_sha256: profile_hash(self.profile)?,
                parent_profile_sha256: Some(profile_hash(self.parent_profile)?),
                workspace_turn_seq: Some(acquired.seq.to_string()),
                conductor: self.conductor.into(),
                root_instance: Some(self.lease.instance()?),
            };
            binding.validate()?;
            Ok(Some(crate::GoRaw(
                serde_json::value::to_raw_value(&binding)
                    .map_err(|e| caused("mission-agent-binding-encoding", e))?,
            )))
        })();
        result.map_err(|e| crate::WorkEngineError::Mission(Box::new(e)))
    }
    /// Same canonical action receipt as the real lease; replay is never restaged.
    fn stage_receipt(
        &self,
        db: &Connection,
        receipt: MutationFenceReceipt<'_>,
    ) -> Result<(), crate::WorkEngineError> {
        self.lease.stage_receipt(db, receipt)
    }
}

/// Decode only recorded provenance with the same strict representation for live
/// inspection and completed receipts. Manual absence remains absence; automatic
/// origin requires its binding. No claim, current consent, turn or actor identity
/// is established by this pure decoding; all typed codec causes survive.
fn recorded_binding(agent: &Agent) -> Result<Option<Binding>, E> {
    if agent.native_mission.is_none() && agent.origin != "conducteur" {
        return Ok(None);
    }
    if agent.origin != "conducteur" {
        return Err(E::Refused("mission-agent-origin-invalid"));
    }
    let raw = agent.native_mission.as_ref()
        .ok_or(E::Refused("mission-agent-binding-required"))?;
    let binding: Binding = swarm_wire_json::decode(raw.0.get().as_bytes())
        .map_err(|cause| caused("mission-agent-binding-invalid", cause))?;
    binding.validate()?;
    Ok(Some(binding))
}

/// Verify FIFO provenance of an exact terminal Agent already matched to its
/// original durable process receipt and ownership/reservation by the caller.
/// Binding3/4 require their same-owner released historical generation; a newer
/// task generation is never modified. Legacy v1/v2 retain historical semantics.
/// Missing/corrupt metadata, SQL/journal/finalize failures preserve typed causes.
/// No current consent/lease is required after completion; this read creates no
/// permission, schema, release, repair, provider start or task acceptance.
pub(crate) fn verify_terminal_workspace(db: &Connection, agent: &Agent) -> Result<(), AgentLaunchError> {
    let result = (|| {
        if !matches!(agent.status.as_str(), "completed" | "failed" | "interrupted") {
            return Err(E::Refused("mission-agent-turn-terminal-required"));
        }
        match recorded_binding(agent)? {
            Some(binding) => verify_turn(db, agent, &binding),
            None => Ok(()),
        }
    })();
    result.map_err(|cause: E| AgentLaunchError::caused("agent-mission-terminal-turn", cause))
}

/// Verify historical provenance, with current mission checks only before grant.
/// Manual legacy rows without native metadata remain manual. Unknown/mismatched
/// origin, malformed metadata or SQL/claim/event failure never becomes permission.
/// Original permits queued cancellation after revocation, not a new execution.
/// v3/v4 require exact acquired turn/history, or same-owner released history for
/// terminal replay. New claim/grant requires v4 physical-instance provenance;
/// Original preserves legacy historical/cancellation behavior without new rights.
/// Optional append-only adoption changes effective grant authority only; original
/// Agent/event/FIFO attribution and the returned Binding remain immutable.
pub(crate) fn inspect(
    db: &Connection,
    agent: &Agent,
    settings: &EngineSettingsDocument,
    check: Check<'_>,
) -> Result<Option<Binding>, AgentLaunchError> {
    if agent.native_mission.is_none() && agent.origin != "conducteur" {
        return Ok(None);
    }
    let result = (|| {
        let binding = recorded_binding(agent)?
            .ok_or(E::Refused("mission-agent-binding-required"))?;
        match check {
            Check::Original => {},
            Check::BeforeClaim(directory) | Check::BeforeGrant(directory) => {
                if binding.schema_version != 4 {
                    return Err(E::Refused("mission-agent-instance-required"));
                }
                let actual = super::observe_instance(db, directory).map_err(|error| E::Root(Box::new(error)))?;
                verify_instance(&binding, &actual)?;
            },
        }
        super::mission_supervision_store::verify_agent_binding(
            db, agent, &binding, settings, check,
        )?;
        verify_turn(db, agent, &binding)?;
        // The original binding is deliberately returned unchanged. The receipt
        // verifier checks current consent/admission against any validated adoption
        // authority before grant; cancellation/terminal attribution uses Original.

        Ok(Some(binding))
    })();
    result.map_err(|e: E| AgentLaunchError::caused("agent-mission-provenance", e))
}

/// Require new execution authority to carry the engine-observed original
/// instance. This pure comparison grants nothing; callers must first decode a
/// canonical Agent and then validate its claim/action/FIFO links. Legacy binding
/// reads and cancellation remain available through Check::Original.
fn verify_instance(binding: &Binding, actual: &super::RootInstanceIdentity) -> Result<(), E> {
    let original = binding.root_instance.as_ref()
        .filter(|_| binding.schema_version == 4)
        .ok_or(E::Refused("mission-agent-instance-required"))?;
    if original != actual { return Err(E::Refused("mission-agent-instance-mismatch")); }
    Ok(())
}

/// Read the exact v3/v4 generation's latest validated journal snapshot. Missing or
/// changed proof refuses; no schema or history is created during inspection.
fn turn_history(db: &Connection, binding: &Binding) -> Result<crate::workspace_turn::WorkspaceTurn, E> {
    if !super::mission_supervision_store::workspace_fifo_schema(db, false)? {
        return Err(E::Refused("mission-agent-turn-journal-required"));
    }
    let seq = binding.workspace_turn_seq.as_deref().ok_or(E::Refused("mission-agent-turn-required"))?
        .parse::<i64>().map_err(|cause| caused("mission-agent-turn-invalid", cause))?;
    crate::workspace_turn_write::history(db, seq).map_err(super::mission_workspace_queue::error)?
        .pop().ok_or(E::Refused("mission-agent-turn-history-missing"))
}
/// Verify actual current ownership or a terminal historical release. String
/// identifiers alone grant nothing: caller first verifies canonical agent/claim/
/// event provenance. New v3/v4 cannot downgrade missing proof into legacy absence.
fn verify_turn(db: &Connection, agent: &Agent, binding: &Binding) -> Result<(), E> {
    if !matches!(binding.schema_version, 3 | 4) { return Ok(()); }
    let last = turn_history(db, binding)?;
    if last.work_id != agent.work_id || last.task_id != agent.task_id
        || last.workspace != agent.workspace || last.agent_id != agent.id {
        return Err(E::Refused("mission-agent-turn-binding-mismatch"));
    }
    if matches!(agent.status.as_str(), "completed" | "failed" | "interrupted") {
        if last.state != crate::workspace_turn::TurnState::Released {
            return Err(E::Refused("mission-agent-turn-release-missing"));
        }
    } else {
        let turns = crate::workspace_turn::read(db).map_err(super::mission_workspace_queue::error)?;
        if last.state != crate::workspace_turn::TurnState::Acquired || !turns.iter().any(|turn| turn == &last) {
            return Err(E::Refused("mission-agent-turn-acquisition-missing"));
        }
    }
    Ok(())
}
/// Release only after an already verified automatic Agent has reached a safe
/// terminal state inside the same cancellation/settlement transaction. Historical
/// released generations replay without touching a newer active row. Legacy v1/v2
/// own no FIFO proof. Typed write/finalize failure rolls back with the caller;
/// this never releases quarantined/uncertain execution or infers task acceptance.
pub(crate) fn release_workspace(db: &Connection, agent: &Agent, binding: Option<&Binding>, at: &str) -> Result<(), AgentLaunchError> {
    let Some(binding) = binding.filter(|value| matches!(value.schema_version, 3 | 4)) else { return Ok(()); };
    let result = (|| {
        if !matches!(agent.status.as_str(), "completed" | "failed" | "interrupted") {
            return Err(E::Refused("mission-agent-turn-terminal-required"));
        }
        let last = turn_history(db, binding)?;
        if last.work_id != agent.work_id || last.task_id != agent.task_id
            || last.workspace != agent.workspace || last.agent_id != agent.id {
            return Err(E::Refused("mission-agent-turn-binding-mismatch"));
        }
        if last.state == crate::workspace_turn::TurnState::Released { return Ok(()); }
        crate::workspace_turn_write::release(db, last.seq, &agent.work_id, &agent.task_id, &agent.workspace, &agent.id, at)
            .map_err(super::mission_workspace_queue::error)?;
        Ok(())
    })();
    result.map_err(|cause: E| AgentLaunchError::caused("agent-mission-turn-release", cause))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Pure saved-input precedence and invalid task variants; no launch authority.
    #[test]
    fn effective_task_profile_and_model_selection_are_deterministic() {
        let parent = LaunchProfile { provider: "parent".into(), role: "worker".into(), workspace: "parent-space".into(), ..Default::default() };
        let mut work = Work { launch_profile: Some(Box::new(parent.clone())), tasks: Some(vec![crate::Task { id: "task".into(), ..Default::default() }]), ..Default::default() };
        assert_eq!(effective_profile(&work, "task").unwrap(), parent);
        let configured = LaunchProfile { provider: "task-provider".into(), workspace: "task-space".into(), ..parent.clone() };
        work.tasks.as_mut().unwrap()[0].launch_profile = Some(Box::new(configured.clone()));
        assert_eq!(effective_profile(&work, "task").unwrap(), configured);
        work.tasks.as_mut().unwrap()[0].model_selection = Some(Box::new(crate::TaskModel { provider: "chosen-provider".into(), route: crate::ModelRoute { level: "simple".into(), ..Default::default() }, ..Default::default() }));
        let resolved = effective_profile(&work, "task").unwrap();
        assert_eq!(resolved.provider, "chosen-provider");
        assert_eq!(resolved.level, "simple");
        assert_eq!(resolved.workspace, "task-space");
        assert!(effective_profile(&work, "absent").is_err());
        let duplicate = work.tasks.as_ref().unwrap()[0].clone();
        work.tasks.as_mut().unwrap().push(duplicate);
        assert!(effective_profile(&work, "task").is_err());
        work.launch_profile = None;
        assert!(effective_profile(&work, "task").is_err());
    }
    /// Data-only positive shape precedes malformed/foreign fields. This fixture
    /// creates no SQL claim, grant, identity or automatic worker permission.
    #[test]
    fn opaque_mission_binding_requires_complete_canonical_representation() {
        let valid = Binding {
            schema_version: 1,
            event_id: "launch-fixture".into(),
            generation: 1,
            claim_settings_revision: crate::SettingCount(1),
            command_settings_revision: crate::SettingCount(1),
            work_revision: 2,
            consent_sequence: 1,
            profile_sha256: "a".repeat(64),
            parent_profile_sha256: None,
            workspace_turn_seq: None,
            conductor: "mission-fixture".into(),
            root_instance: None,
        };
        valid.validate().unwrap();
        let mut current = valid.clone();
        current.schema_version = 2;
        assert!(current.validate().is_err());
        current.parent_profile_sha256 = Some("b".repeat(64));
        current.validate().unwrap();
        let encoded = serde_json::to_vec(&current).unwrap();
        swarm_wire_json::decode::<Binding>(&encoded).unwrap().validate().unwrap();
        current.schema_version = 1;
        assert!(current.validate().is_err());
        current.schema_version = 3;
        assert!(current.validate().is_err());
        current.workspace_turn_seq = Some("9223372036854775807".into());
        current.validate().unwrap();
        current.schema_version = 4;
        assert!(current.validate().is_err());
        current.root_instance = Some(super::super::RootInstanceIdentity {
            directory: (0, u64::MAX), database: (u64::MAX, 1),
        });
        current.validate().unwrap();
        let encoded = serde_json::to_vec(&current).unwrap();
        let decoded: Binding = swarm_wire_json::decode(&encoded).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.root_instance, current.root_instance);
        verify_instance(&decoded, current.root_instance.as_ref().unwrap()).unwrap();
        let mut other_directory = current.root_instance.clone().unwrap();
        other_directory.directory.1 -= 1;
        assert!(matches!(verify_instance(&decoded, &other_directory), Err(E::Refused("mission-agent-instance-mismatch"))));
        let mut other_database = current.root_instance.clone().unwrap();
        other_database.database.1 += 1;
        assert!(matches!(verify_instance(&decoded, &other_database), Err(E::Refused("mission-agent-instance-mismatch"))));
        assert!(matches!(verify_instance(&valid, &other_database), Err(E::Refused("mission-agent-instance-required"))));
        let mut invalid_instance = serde_json::to_value(&current).unwrap();
        invalid_instance["root_instance"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<Binding>(invalid_instance).is_err());
        let mut unknown_instance = serde_json::to_value(&current).unwrap();
        unknown_instance["root_instance"]["permission"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Binding>(unknown_instance).is_err());
        for malformed in [
            serde_json::json!({"directory":[1,2],"database":[1]}),
            serde_json::json!({"directory":[1,2],"database":[1,2.0]}),
            serde_json::json!({"directory":[1,2],"database":[1,true]}),
            serde_json::json!({"directory":[1,2],"database":[1,"2"]}),
            serde_json::json!({"directory":[1,2]}),
        ] {
            let mut invalid = serde_json::to_value(&current).unwrap();
            invalid["root_instance"] = malformed;
            assert!(serde_json::from_value::<Binding>(invalid).is_err());
        }
        current.schema_version = 3;
        assert!(current.validate().is_err());
        current.root_instance = None;
        current.validate().unwrap();
        for invalid_seq in ["0", "-1", "01", "+1", "9223372036854775808"] {
            current.workspace_turn_seq = Some(invalid_seq.into());
            assert!(current.validate().is_err());
        }
        current.workspace_turn_seq = None;
        current.schema_version = 2;
        current.parent_profile_sha256 = Some("bad".into());
        assert!(current.validate().is_err());
        let mut null_parent = serde_json::to_value(&valid).unwrap();
        null_parent["parent_profile_sha256"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<Binding>(null_parent).is_err());
        let mut invalid = valid.clone();
        invalid.generation = 0;
        assert!(invalid.validate().is_err());
        invalid = valid.clone();
        invalid.profile_sha256 = "not-a-hash".into();
        assert!(invalid.validate().is_err());
        let mut value = serde_json::to_value(&valid).unwrap();
        value["enabled"] = serde_json::json!(true);
        assert!(swarm_wire_json::decode::<Binding>(&serde_json::to_vec(&value).unwrap()).is_err());
        let r = serde_json::json!({"schema_version":1,"event_id":"worker","expected_revision":2,"task_id":"task"});
        serde_json::from_value::<MissionWorkerRequest>(r.clone()).unwrap();
        let mut forged = r;
        forged["conductor_id"] = serde_json::json!("mission-fixture");
        assert!(serde_json::from_value::<MissionWorkerRequest>(forged).is_err());
    }
    /// SQL-memory projection only: fabricated Agent/Binding/history values test
    /// terminal FIFO consistency, never Root authority, actual receipt matching,
    /// identity, execution or acceptance. A valid released witness precedes each
    /// missing/corrupt-history case; SAVEPOINT rollback restores exact SQL rows.
    #[test]
    fn terminal_workspace_proof_preserves_legacy_and_requires_exact_released_generation() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("BEGIN DEFERRED").unwrap();
        let refusal = |error: AgentLaunchError, expected: &str| {
            match error {
                AgentLaunchError::Cause { code: "agent-mission-terminal-turn", cause } => {
                    assert!(matches!(cause.downcast_ref::<E>(), Some(E::Refused(code)) if *code == expected),
                        "expected exact typed terminal FIFO refusal");
                }
                _ => panic!("expected typed terminal FIFO error wrapper"),
            }
        };
        let mut agent = Agent {
            id: "agent".into(), work_id: "work".into(), task_id: "task".into(),
            workspace: "/project".into(), origin: "operateur".into(),
            status: "running".into(), ..Default::default()
        };
        refusal(verify_terminal_workspace(&db, &agent).unwrap_err(), "mission-agent-turn-terminal-required");
        agent.status = "completed".into();
        verify_terminal_workspace(&db, &agent).unwrap();
        let mut binding = Binding {
            schema_version: 1, event_id: "launch-fixture".into(), generation: 1,
            claim_settings_revision: crate::SettingCount(1),
            command_settings_revision: crate::SettingCount(1), work_revision: 2,
            consent_sequence: 1, profile_sha256: "a".repeat(64),
            parent_profile_sha256: None, workspace_turn_seq: None,
            conductor: "mission-fixture".into(),
            root_instance: None,
        };
        agent.origin = "conducteur".into();
        for version in [1, 2] {
            binding.schema_version = version;
            binding.parent_profile_sha256 = if version == 2 { Some("b".repeat(64)) } else { None };
            agent.native_mission = Some(crate::GoRaw(serde_json::value::to_raw_value(&binding).unwrap()));
            verify_terminal_workspace(&db, &agent).unwrap();
        }
        binding.schema_version = 3;
        binding.workspace_turn_seq = Some("7".into());
        agent.native_mission = Some(crate::GoRaw(serde_json::value::to_raw_value(&binding).unwrap()));
        db.execute_batch(crate::workspace_turn_write::JOURNAL_DDL).unwrap();
        let at = "2026-10-22T10:00:00Z";
        let insert = "INSERT INTO workspace_turn_events(turn_seq,work_id,task_id,workspace,state,agent_id,requested_at,acquired_at,released_at,transition) VALUES(7,'work','task','/project',?,?,?,?,?,?)";
        for (state, owner, acquired, released, transition) in [
            ("waiting", "", "", "", "enqueued"),
            ("acquired", "agent", at, "", "acquired"),
            ("released", "agent", at, at, "released"),
        ] {
            db.execute(insert, params![state, owner, at, acquired, released, transition]).unwrap();
        }
        verify_terminal_workspace(&db, &agent).unwrap();
        let changes = db.total_changes();
        verify_terminal_workspace(&db, &agent).unwrap();
        assert_eq!(db.total_changes(), changes, "terminal proof inspection is read-only");
        db.execute_batch("SAVEPOINT missing_release; DELETE FROM workspace_turn_events WHERE transition='released'").unwrap();
        refusal(verify_terminal_workspace(&db, &agent).unwrap_err(), "mission-agent-turn-release-missing");
        db.execute_batch("ROLLBACK TO missing_release; RELEASE missing_release").unwrap();
        verify_terminal_workspace(&db, &agent).unwrap();
        db.execute_batch("SAVEPOINT foreign_owner; UPDATE workspace_turn_events SET agent_id='foreign' WHERE state IN ('acquired','released')").unwrap();
        refusal(verify_terminal_workspace(&db, &agent).unwrap_err(), "mission-agent-turn-binding-mismatch");
        db.execute_batch("ROLLBACK TO foreign_owner; RELEASE foreign_owner").unwrap();
        verify_terminal_workspace(&db, &agent).unwrap();
        binding.workspace_turn_seq = Some("8".into());
        agent.native_mission = Some(crate::GoRaw(serde_json::value::to_raw_value(&binding).unwrap()));
        refusal(verify_terminal_workspace(&db, &agent).unwrap_err(), "mission-agent-turn-history-missing");
        binding.workspace_turn_seq = Some("7".into());
        agent.native_mission = Some(crate::GoRaw(serde_json::value::to_raw_value(&binding).unwrap()));
        verify_terminal_workspace(&db, &agent).unwrap();
        binding.schema_version = 4;
        binding.root_instance = Some(super::super::RootInstanceIdentity { directory: (1, 2), database: (1, 3) });
        agent.native_mission = Some(crate::GoRaw(serde_json::value::to_raw_value(&binding).unwrap()));
        verify_terminal_workspace(&db, &agent).unwrap();
        let before = db.total_changes();
        release_workspace(&db, &agent, Some(&binding), at).unwrap();
        assert_eq!(db.total_changes(), before, "v4 terminal release replay must not rewrite history");
        db.execute_batch("SAVEPOINT v4_missing; DELETE FROM workspace_turn_events WHERE transition='released'").unwrap();
        refusal(verify_terminal_workspace(&db, &agent).unwrap_err(), "mission-agent-turn-release-missing");
        db.execute_batch("ROLLBACK TO v4_missing; RELEASE v4_missing").unwrap();
        verify_terminal_workspace(&db, &agent).unwrap();
        db.execute_batch("ROLLBACK").unwrap();
    }
}
