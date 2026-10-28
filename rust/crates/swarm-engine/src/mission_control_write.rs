//! Transactional global departure pause/stop, separate from planner pause.
use super::{
    mission_controls::{controls, MissionAutonomy},
    mission_supervision_store::execute,
    RootStore, RootStoreError,
};
use crate::{
    mission_control_request::cause, EngineSettingsDocument, MissionControlAction as Action,
    MissionControlRequest, MissionControlView, MissionSupervisionError as E, Work,
};
use rusqlite::{params, Connection};

/// Render only the established persisted protocol vocabulary, never a fallback.
fn mode(value: MissionAutonomy) -> &'static str {
    match value {
        MissionAutonomy::Manual => "manuel",
        MissionAutonomy::Assisted => "assiste",
        MissionAutonomy::Automatic => "autonome",
    }
}

/// Start/resume delegate to the shared profile/consent transaction with the Root
/// path; pause/stop/unpause retain their established semantics below.
/// Priority changes only audited ordinal metadata; it preserves all execution controls.
/// Stage global departure control inside caller's writer transaction, after its
/// event/revision/replay guard. Validate current controls; preserve existing mode
/// and slots even if a newer capacity policy is tighter (pause must not clamp).
/// Missing controls on pause/stop become explicitly manual+paused, never SQL's
/// automatic/two-slot defaults; slots use saved Admin initial-slot policy. Manual
/// here denies automatic admission in previously absent state, not launch consent.
/// Unpause requires existing controls and preserves both mode and consent.
/// Pause/stop revoke logical claims only; stop appends canonical consent=false.
/// Preserve counters/funding/physical rows and originals. Actor/time come from
/// engine observations, not identity proof. SQL/codec/state/overflow failures keep
/// cause and require caller rollback; no commit, provider signal or process starts.
pub(crate) fn stage(
    db: &Connection,
    root: &std::path::Path,
    work: &mut Work,
    action: Action,
    r: &MissionControlRequest,
    settings: &EngineSettingsDocument,
    observed: (&str, &str),
) -> Result<(), E> {
    let (at, actor) = observed;
    if action == Action::Priority {
        return priority(db,work,r,settings,observed);
    }
    if matches!(action,Action::Start|Action::Resume) {
        return super::mission_start::stage(db,root,work,action,r,settings,observed);
    }
    settings.validate().map_err(E::Settings)?;
    crate::mission_supervision::parse(at)?;
    if !crate::text::nonempty(actor) {
        return Err(E::Refused("mission-control-actor-required"));
    }
    let previous = controls(db, &work.id)?;
    let (autonomy, slots) = if let Some(c) = previous {
        (mode(c.autonomy), c.slots)
    } else {
        if action == Action::Unpause {
            return Err(E::Refused("mission-controls-required"));
        }
        let policy = settings
            .values
            .mission_dispatch_policy()
            .map_err(E::Settings)?;
        ("manuel", policy.slots(None).map_err(E::Settings)?.0)
    };
    let paused = action != Action::Unpause;
    if paused {
        crate::mission_control_request::revoke(work)?;
    }
    let count=execute(db,"INSERT INTO cockpit_controls(work_id,paused,autonomy,slots) VALUES(?,?,?,?) ON CONFLICT(work_id) DO UPDATE SET paused=excluded.paused",params![work.id,paused,autonomy,slots])?;
    if count != 1 {
        return Err(E::Refused("mission-control-write-missing"));
    }
    if action == Action::Stop {
        // Exact historical consent schema: data from observed operator/time,
        // no extra authority field, synthetic approval or private receipt.
        let body =
            serde_json::to_string(&serde_json::json!({"enabled":false,"actor":actor,"at":at}))
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
        "opérateur local : départs suspendus={paused} ; événement={}",
        r.event_id
    );
    if execute(
        db,
        "INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,'pause',?)",
        params![work.id, at, message],
    )? != 1
    {
        return Err(E::Refused("mission-control-event-write-missing"));
    }
    Ok(())
}
/// Stage an exact ordinal change with its cockpit audit in the caller's Work
/// transaction. Validate unique task, saved settings, actual timestamp/operator;
/// task presence and ordinal are rechecked after strict envelope decoding. No
/// mode/consent/slots/profile/claims/attempt/consumption changes. Any SQL/audit
/// failure requires caller rollback; generic Work revision/replay/event/commit
/// rules remain authoritative. Local actor is attribution, not authentication.
fn priority(db: &Connection, work: &Work, r: &MissionControlRequest,
    settings: &EngineSettingsDocument, observed: (&str,&str)) -> Result<(),E> {
    settings.validate().map_err(E::Settings)?;
    let (at,actor)=observed;crate::mission_supervision::parse(at)?;
    if !crate::text::nonempty(actor) { return Err(E::Refused("mission-control-actor-required")); }
    let mut tasks=work.tasks.iter().flatten().filter(|t|t.id==r.task_id);
    tasks.next().ok_or(E::Refused("mission-priority-task-missing"))?;
    if tasks.next().is_some() { return Err(E::Refused("mission-priority-task-duplicate")); }
    let value=crate::mission_control_request::priority_value(r)?;
    if execute(db,"INSERT INTO cockpit_tasks(work_id,task_id,priority) VALUES(?,?,?) ON CONFLICT(work_id,task_id) DO UPDATE SET priority=excluded.priority",params![work.id,r.task_id,value])? != 1 {
        return Err(E::Refused("mission-priority-write-missing"));
    }
    let message=format!("opérateur local : tâche {} priorité {} ; événement={}",r.task_id,value,r.event_id);
    if execute(db,"INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,'priority',?)",params![work.id,at,message])? != 1 {
        return Err(E::Refused("mission-priority-audit-missing"));
    }
    Ok(())
}
impl RootStore {
    /// Read actual controls/consent and Work revision while holding the shared
    /// Root reservation; supported writers cannot interleave these reads. Preserve
    /// absence and typed read/close causes, exposing no private actor/host data.
    /// This is a local observation, not a future action lease, execution consent,
    /// authenticated operator or confinement against hostile same-UID SQL writes.
    /// Caller still owes Root.close. No SQL/configuration/process mutation occurs.
    pub fn mission_control_view(&self, work: &str) -> Result<MissionControlView, E> {
        let root = |e| E::Root(Box::new(e));
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(|e| root(RootStoreError::Storage(e)))?;
        let result = (|| {
            let actual = self.read_work(work).map_err(root)?;
            let observed = self.read_mission_controls(work)?;
            Ok(MissionControlView {
                revision: actual.revision,
                paused: observed.controls.as_ref().map(|c| c.paused),
                autonomy: observed.controls.as_ref().map(|c| mode(c.autonomy)),
                slots: observed.controls.as_ref().map(|c| c.slots),
                consent_enabled: observed.consent.as_ref().map(|c| c.enabled),
                archived: observed.archived,
            })
        })();
        crate::mission_supervision::finish(
            result,
            reservation
                .close()
                .map_err(|e| root(RootStoreError::Storage(e))),
        )
    }
}
