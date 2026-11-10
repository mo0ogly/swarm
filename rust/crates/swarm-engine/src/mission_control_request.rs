//! Audited local mission controls and explicit start configuration; no process starts.
use crate::{MissionSupervisionError as E, Work};
use serde::{Deserialize, Serialize};

/// Local operator audit envelope. Priority uses exact task/ordinal metadata;
/// its dedicated decoder alone accepts those fields. Other encodings omit them,
/// preserving existing canonical request bytes. Start may supply profile/slots;
/// other actions reject them. No actor/time/autonomy/budget or native ownership
/// is accepted from these fields. Start records local consent in its transaction. The common Root boundary owns size,
/// canonical event/revision/replay and the transaction; this data starts no process.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MissionControlRequest {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub reason: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<Box<crate::LaunchProfile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots: Option<i64>,
}
/// Fixed protocol choices. Unpause preserves mode/consent and does not mean
/// automatic mission resume; stop records consent=false as well as departure pause.
/// Start records local consent/profile; Resume requires existing consent.
/// Priority changes ordinal metadata only, without pause/consent/slot effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionControlAction {
    Pause,
    Unpause,
    Stop,
    Start,
    Resume,
    Priority,
}
impl MissionControlAction {
    /// Map only supported canonical command names; unknown commands stay refused.
    pub fn from_kind(kind: &str) -> Option<Self> {
        match kind {
            "mission.pause" => Some(Self::Pause),
            "mission.unpause" => Some(Self::Unpause),
            "mission.stop" => Some(Self::Stop),
            "mission.start" => Some(Self::Start),
            "mission.resume" => Some(Self::Resume),
            "mission.priority" => Some(Self::Priority),
            _ => None,
        }
    }
}
/// Keep a static safe diagnostic and the original codec/allocation/SQL cause.
/// This neither renders request contents nor supplies rollback or retry evidence.
pub(crate) fn cause(
    code: &'static str,
    error: impl std::error::Error + Send + Sync + 'static,
) -> E {
    E::Cause {
        code,
        cause: Box::new(error),
    }
}

/// Decode the strict inherited JSON envelope before Root access. Require schema1,
/// a safe event ID, positive revision and nonblank audit reason. The latter is an
/// audit invariant, not a hidden text-size policy; existing transport limits apply.
/// Unknown fields, malformed types/UTF8 and codec failures preserve their cause.
/// No IO, ownership/consent grant or mutation is performed.
pub fn decode_mission_control_request(raw: &[u8]) -> Result<MissionControlRequest, E> {
    use crate::model_wire::Kind;
    let fields = &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("reason", Kind::String),
    ];
    let r: MissionControlRequest = crate::work::decode_mode(raw, fields, true)
        .map_err(|e| cause("mission-control-request-invalid", e))?;
    if r.schema_version != 1
        || r.expected_revision < 1
        || !swarm_identity_contract::name(r.event_id.as_bytes())
        || !crate::text::nonempty(&r.reason)
    {
        return Err(E::Refused("mission-control-request-invalid"));
    }
    Ok(r)
}
/// Parse only canonical exact signed decimal SQLite priority representation.
/// The i64 bound is storage representation, not an operator range or capacity.
/// Missing/null/noncanonical/overflow values refuse with retained parse cause;
/// no whitespace, numeric JSON conversion, clamping or state mutation occurs.
pub(crate) fn priority_value(request: &MissionControlRequest) -> Result<i64, E> {
    let raw = request.priority.as_deref().ok_or(E::Refused("mission-priority-required"))?;
    let value = raw.parse::<i64>().map_err(|e| cause("mission-priority-invalid", e))?;
    if value.to_string() != raw { return Err(E::Refused("mission-priority-noncanonical")); }
    Ok(value)
}
/// Decode an audited local ordinal change before Root access. Require schema1,
/// event/task identifiers, positive revision, audit reason and exact string i64.
/// Unknown fields, actor/consent/profile/slot input and numeric/null priorities
/// refuse. Priority is metadata only; this grants no identity or launch right.
pub fn decode_mission_priority_request(raw: &[u8]) -> Result<MissionControlRequest, E> {
    use crate::model_wire::Kind;
    let fields = &[("schema_version",Kind::Int64),("event_id",Kind::String),
        ("expected_revision",Kind::Int64),("reason",Kind::String),
        ("task_id",Kind::String),("priority",Kind::String)];
    let request: MissionControlRequest = crate::work::decode_mode(raw,fields,true)
        .map_err(|e|cause("mission-priority-request-invalid",e))?;
    if request.schema_version != 1 || request.expected_revision < 1
        || !swarm_identity_contract::name(request.event_id.as_bytes())
        || !swarm_identity_contract::name(request.task_id.as_bytes())
        || !crate::text::nonempty(&request.reason) {
        return Err(E::Refused("mission-priority-request-invalid"));
    }
    priority_value(&request)?;
    Ok(request)
}
/// Launch profile input excludes engine-owned actor/time while preserving the
/// inherited profile vocabulary. Nested limits and skills retain strict shapes.
fn start_profile_shape() -> crate::model_wire::Fields {
    use crate::model_wire::Kind;
    &[
        ("provider", Kind::String),
        ("role", Kind::String),
        ("workspace", Kind::String),
        ("instruction", Kind::String),
        ("level", Kind::String),
        ("timeout_seconds", Kind::Int64),
        ("capture_output", Kind::Bool),
        (
            "limits",
            Kind::Pointer(&Kind::Object(crate::models::shape_run_limits)),
        ),
        (
            "skills",
            Kind::Array(&Kind::Object(crate::models::shape_action_skill_selection)),
        ),
    ]
}
/// Decode an audited explicit local start, never a serialized ownership proof.
/// Optional profile/slots mean reuse persisted values or the saved Admin initial
/// slots when absent; semantic/current-source checks belong to the transaction.
/// User actor/updated, foreign fields, malformed envelopes or negative slots
/// refuse without IO. Missing profile is valid only if Work already stores one.
pub fn decode_mission_start_request(raw: &[u8]) -> Result<MissionControlRequest, E> {
    use crate::model_wire::Kind;
    let fields = &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("reason", Kind::String),
        ("profile", Kind::Pointer(&Kind::Object(start_profile_shape))),
        ("slots", Kind::Pointer(&Kind::Int64)),
    ];
    let r: MissionControlRequest = crate::work::decode_mode(raw, fields, true)
        .map_err(|e| cause("mission-control-request-invalid", e))?;
    if r.schema_version != 1
        || r.expected_revision < 1
        || !swarm_identity_contract::name(r.event_id.as_bytes())
        || !crate::text::nonempty(&r.reason)
        || r.slots.is_some_and(|v| v < 1)
    {
        return Err(E::Refused("mission-control-request-invalid"));
    }
    Ok(r)
}
/// Validated safe local view. None preserves absence; no legacy default or true
/// consent is inferred. Declared actor/private host tuples are deliberately absent.
#[derive(Debug, Serialize)]
pub struct MissionControlView {
    pub revision: i64,
    pub paused: Option<bool>,
    pub autonomy: Option<&'static str>,
    pub slots: Option<i64>,
    pub consent_enabled: Option<bool>,
    pub archived: bool,
}
/// Revoke only logical planner claims on a private candidate. Preserve paused
/// planning flag, delivery originals, history, counters and physical run registry.
/// Checked generations refuse overflow before mutation; caller must discard on
/// error and commit with global pause. No execution completion or refund follows.
pub(crate) fn revoke(work: &mut Work) -> Result<(), E> {
    crate::validate_planning_state(work)
        .map_err(|e| cause("mission-control-planning-invalid", e))?;
    let Some(p) = work.planning.as_mut() else {
        return Ok(());
    };
    let mut generations = Vec::new();
    generations
        .try_reserve_exact(p.scopes.as_deref().unwrap_or_default().len())
        .map_err(|e| cause("mission-control-generation-capacity", e))?;
    for s in p.scopes.iter().flatten() {
        generations.push(
            s.generation
                .checked_add(1)
                .ok_or(E::Refused("mission-control-generation-overflow"))?,
        );
    }
    for (scope, generation) in p.scopes.iter_mut().flatten().zip(generations) {
        scope.generation = generation;
        scope.holder.clear();
        scope.lease_until.clear();
    }
    crate::validate_planning_state(work)
        .map_err(|e| cause("mission-control-planning-result-invalid", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A valid request precedes unknown privilege fields, malformed and empty
    /// audit envelopes. Nonblank long reason is data under the caller transport.
    #[test]
    fn envelope_refuses_foreign_authority_without_hidden_reason_cap() {
        let r=decode_mission_control_request(br#"{"schema_version":1,"event_id":"pause","expected_revision":2,"reason":"Keep pending work"}"#).unwrap();
        assert_eq!(r.event_id, "pause");
        for raw in [br#"{"schema_version":1,"event_id":"pause","expected_revision":2,"reason":"x","enabled":true}"#.as_slice(),br#"{"schema_version":0,"event_id":"pause","expected_revision":2,"reason":"x"}"#,br#"{"schema_version":1,"event_id":"pause","expected_revision":0,"reason":"x"}"#,br#"{"schema_version":1,"event_id":"pause","expected_revision":2,"reason":" "}"#] {assert!(decode_mission_control_request(raw).is_err());}
        let raw = serde_json::json!({"schema_version":1,"event_id":"long","expected_revision":2,"reason":"é".repeat(2500)});
        assert_eq!(
            decode_mission_control_request(&serde_json::to_vec(&raw).unwrap())
                .unwrap()
                .reason
                .len(),
            5000
        );
    }
    /// Protocol routing distinguishes stopping consent from unpausing departures.
    #[test]
    fn action_vocabulary_separates_start_resume_from_watch() {
        assert_eq!(
            MissionControlAction::from_kind("mission.pause"),
            Some(MissionControlAction::Pause)
        );
        assert_eq!(
            MissionControlAction::from_kind("mission.stop"),
            Some(MissionControlAction::Stop)
        );
        assert_eq!(
            MissionControlAction::from_kind("mission.unpause"),
            Some(MissionControlAction::Unpause)
        );
        assert_eq!(
            MissionControlAction::from_kind("mission.start"),
            Some(MissionControlAction::Start)
        );
        assert_eq!(
            MissionControlAction::from_kind("mission.resume"),
            Some(MissionControlAction::Resume)
        );
        for kind in ["mission.watch", "planning.pause"] {
            assert!(MissionControlAction::from_kind(kind).is_none());
        }
    }
    /// Start accepts explicit profile/slots only; actor/updated stay engine-owned.
    /// The same fields are rejected by pause/resume's original audit envelope.
    #[test]
    fn start_profile_never_accepts_forged_actor_or_updates_pause_fields() {
        let raw = serde_json::json!({"schema_version":1,"event_id":"start","expected_revision":2,"reason":"Explicit local start","slots":17,"profile":{"provider":"fixture","role":"worker","workspace":".","level":"standard"}});
        let bytes = serde_json::to_vec(&raw).unwrap();
        assert_eq!(
            decode_mission_start_request(&bytes).unwrap().slots,
            Some(17)
        );
        assert!(decode_mission_control_request(&bytes).is_err());
        for key in ["actor", "updated"] {
            let mut changed = raw.clone();
            changed["profile"][key] = serde_json::json!("forged");
            assert!(decode_mission_start_request(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
        let mut invalid = raw;
        invalid["slots"] = serde_json::json!(0);
        assert!(decode_mission_start_request(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
    /// A valid logical revocation precedes the representation-overflow fault.
    /// This pure fixture grants no process ownership and must never free one.
    #[test]
    fn logical_revocation_preserves_history_and_refuses_generation_overflow() {
        let at = "2026-10-22T12:00:00Z";
        let settings = crate::EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture".into(),
            at.into(),
        )
        .unwrap();
        let mut work = crate::apply_planning_control(
            &Work {
                schema_version: 1,
                id: "work".into(),
                title: "Fixture".into(),
                objective: "Keep original claims".into(),
                criteria: Some(vec!["Owned".into()]),
                ..Default::default()
            },
            crate::PlanningControlAction::Enable,
            &crate::PlanningRequest {
                schema_version: 1,
                event_id: "enable".into(),
                max_tasks: 3,
                max_activations: 3,
                max_decisions: 3,
                ..Default::default()
            },
            at,
            &settings,
        )
        .unwrap();
        let original = work.clone();
        let generation = work.planning.as_ref().unwrap().scopes.as_ref().unwrap()[0].generation;
        revoke(&mut work).unwrap();
        let scope = &work.planning.as_ref().unwrap().scopes.as_ref().unwrap()[0];
        assert_eq!(scope.generation, generation + 1);
        assert!(scope.holder.is_empty() && scope.lease_until.is_empty());
        assert_eq!(
            work.planning.as_ref().unwrap().inbox,
            original.planning.as_ref().unwrap().inbox
        );
        assert_eq!(
            work.planning.as_ref().unwrap().judge,
            original.planning.as_ref().unwrap().judge
        );
        assert_eq!(
            work.planning.as_ref().unwrap().activations,
            original.planning.as_ref().unwrap().activations
        );
        assert_eq!(
            work.planning.as_ref().unwrap().decisions,
            original.planning.as_ref().unwrap().decisions
        );
        assert!(!work.planning.as_ref().unwrap().paused);
        work.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].generation = i64::MAX;
        let before = work.clone();
        assert_eq!(
            revoke(&mut work).unwrap_err().code(),
            "mission-control-generation-overflow"
        );
        assert_eq!(work, before);
    }
}

#[cfg(test)]
mod priority_tests {
    use super::*;
    /// Exact data representation and old canonical envelopes, not permission.
    #[test]
    fn exact_priority_decoder_and_legacy_envelopes_remain_distinct() {
        let base=serde_json::json!({"schema_version":1,"event_id":"priority","expected_revision":1,"task_id":"task","priority":"0","reason":"Fixture ordinal metadata"});
        for value in [i64::MIN,0,i64::MAX] {
            let mut input=base.clone();input["priority"]=serde_json::json!(value.to_string());
            let raw=serde_json::to_vec(&input).unwrap();let request=decode_mission_priority_request(&raw).unwrap();assert_eq!(priority_value(&request).unwrap(),value);
            assert!(decode_mission_control_request(&raw).is_err());assert!(decode_mission_start_request(&raw).is_err());
        }
        for value in [serde_json::json!(0),serde_json::Value::Null,serde_json::json!("-0"),serde_json::json!("01"),serde_json::json!("+1"),serde_json::json!("9223372036854775808")] {
            let mut input=base.clone();input["priority"]=value;assert!(decode_mission_priority_request(&serde_json::to_vec(&input).unwrap()).is_err());
        }
        for field in ["actor","enabled","slots"] {
            let mut input=base.clone();input[field]=serde_json::json!(true);assert!(decode_mission_priority_request(&serde_json::to_vec(&input).unwrap()).is_err());
        }
        let old=serde_json::json!({"schema_version":1,"event_id":"pause","expected_revision":1,"reason":"Old envelope"});
        let request=decode_mission_control_request(&serde_json::to_vec(&old).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(),old);
        let mut missing=base;missing.as_object_mut().unwrap().remove("priority");assert!(decode_mission_priority_request(&serde_json::to_vec(&missing).unwrap()).is_err());
    }
}
