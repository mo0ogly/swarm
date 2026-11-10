//! Atomic local start/resume configuration; the conductor loop is a separate consumer.
use super::{
    mission_controls::{consent, controls},
    mission_supervision_store::{execute, optional},
};
use crate::{
    mission_control_request::cause, EngineSettingsDocument, MissionControlAction as Action,
    MissionControlRequest, MissionSupervisionError as E, Work,
};
use rusqlite::{params, Connection};
use std::path::Path;

/// Validate current profile sources without launching a probe or reserving funds.
/// Reuse canonical project containment, provider/rules/catalogue and saved limit
/// validation. Do not claim executable health, model identity or future admission.
/// Close the checked Root directory with joined errors; private values stay out
/// of diagnostics. The caller owns its settings/Work transaction and revision.
fn profile(
    root: &Path,
    work: &Work,
    supplied: &crate::LaunchProfile,
    settings: &EngineSettingsDocument,
) -> Result<crate::LaunchProfile, E> {
    let fail = |e: crate::AgentLaunchError| cause("mission-profile-invalid", e);
    if !swarm_identity_contract::name(supplied.provider.as_bytes())
        || !matches!(supplied.role.as_str(), "worker" | "planner" | "subplanner")
        || supplied.timeout_seconds < 0
    {
        return Err(E::Refused("mission-profile-invalid"));
    }
    let workspace =
        crate::agent_launch_preflight::workspace(root, &supplied.workspace).map_err(fail)?;
    if let Some(limits) = supplied.limits.as_deref() {
        if limits.observation_mode != 0 {
            return Err(E::Refused("runtime-observation-requires-admin"));
        }
        settings
            .values
            .runtime_limit_policy()
            .map_err(E::Settings)?
            .validate_override(limits)
            .map_err(|e| cause(e.code(), e))?;
    }
    let directory = crate::local_storage::LocalDirectory::open(&root.join(".swarm"))
        .map_err(|e| cause("mission-profile-root-open", e))?;
    let result = (|| {
        let policy = settings.values.agent_launch_policy().map_err(E::Settings)?;
        let launch = crate::Launch {
            provider: supplied.provider.clone(),
            level: supplied.level.clone(),
            ..Default::default()
        };
        let (provider, _) =
            crate::agent_launch::provider(&directory, &launch, &policy).map_err(fail)?;
        crate::provider_model_route::resolve_configuration_metadata(
            &provider,
            &launch,
            &settings.values,
            &policy,
        )
        .map_err(fail)?;
        crate::agent_reviewer_preflight::check(root, &directory, work.planning.as_deref(), &policy)
            .map_err(fail)?;
        let mut candidate = supplied.clone();
        candidate.workspace = workspace
            .to_str()
            .ok_or(E::Refused("mission-profile-workspace-utf8"))?
            .into();
        Ok(candidate)
    })();
    crate::mission_supervision::finish(
        result,
        directory
            .close()
            .map_err(|e| cause("mission-profile-root-close", e)),
    )
}

/// Stage explicit start consent or resume under an existing true consent record.
/// Caller supplies observed actor/time and owns CAS, replay, transaction and Root
/// settings reservation. Start may reuse a saved profile and adopts only saved
/// Admin initial slots when controls/explicit slots are absent. Resume preserves
/// original consent/profile and requires existing controls. Current slot policy,
/// archive, organization/reviewer and profile sources must validate for both.
/// No inference, process, lease, dispatch, acceptance, refund or counter reset.
/// SQL/codec/source failures retain safe causes and require caller rollback.
pub(super) fn stage(
    db: &Connection,
    root: &Path,
    work: &mut Work,
    action: Action,
    r: &MissionControlRequest,
    settings: &EngineSettingsDocument,
    observed: (&str, &str),
) -> Result<(), E> {
    let (at, actor) = observed;
    settings.validate().map_err(E::Settings)?;
    crate::mission_supervision::parse(at)?;
    if !crate::text::nonempty(actor) || crate::agent_terminal_text::terminal_text(actor) != actor {
        return Err(E::Refused("mission-control-actor-required"));
    }
    let archived = optional(
        db,
        "SELECT count(*) FROM mission_lifecycle WHERE work_id=?",
        [&work.id],
        |row| row.get::<_, i64>(0),
    )?
    .ok_or(E::Refused("mission-archive-observation-missing"))?;
    if archived != 0 {
        return Err(E::Refused("mission_archived"));
    }
    let previous = controls(db, &work.id)?;
    if action == Action::Resume {
        if !consent(db, &work.id)?.is_some_and(|c| c.enabled) {
            return Err(E::Refused("mission-consent-required"));
        }
        if previous.is_none() {
            return Err(E::Refused("mission-controls-required"));
        }
    }
    crate::validate_mission_organization(work).map_err(|e| cause(e.code(), e))?;
    let selected = if action == Action::Start {
        r.profile.as_deref().or(work.launch_profile.as_deref())
    } else {
        work.launch_profile.as_deref()
    }
    .ok_or(E::Refused("mission-profile-required"))?;
    let mut selected = profile(root, work, selected, settings)?;
    let slots = settings
        .values
        .mission_dispatch_policy()
        .map_err(E::Settings)?
        .slots(r.slots.or_else(|| previous.as_ref().map(|c| c.slots)))
        .map_err(E::Settings)?
        .0;
    if action == Action::Start {
        selected.actor = actor.into();
        selected.updated = at.into();
        work.launch_profile = Some(Box::new(selected));
    }
    if execute(db,"INSERT INTO cockpit_controls(work_id,paused,autonomy,slots) VALUES(?,0,'autonome',?) ON CONFLICT(work_id) DO UPDATE SET paused=0,autonomy='autonome',slots=excluded.slots",params![work.id,slots])?!=1 {
        return Err(E::Refused("mission-control-write-missing"));
    }
    if action == Action::Start {
        let body =
            serde_json::to_string(&serde_json::json!({"enabled":true,"actor":actor,"at":at}))
                .map_err(|e| cause("mission-control-policy-encoding", e))?;
        if execute(
            db,
            "INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,'mission-policy',?)",
            params![work.id, at, body],
        )? != 1
        {
            return Err(E::Refused("mission-control-policy-write-missing"));
        }
    }
    let message = format!(
        "local operator configured {:?}; event={}",
        action, r.event_id
    );
    if execute(
        db,
        "INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,'mission-control',?)",
        params![work.id, at, message],
    )? != 1
    {
        return Err(E::Refused("mission-control-event-write-missing"));
    }
    Ok(())
}
