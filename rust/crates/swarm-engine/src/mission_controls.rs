//! Read-only declared mission consent and persisted controls, never launch authority.
use super::{
    mission_supervision_store::{caused, optional, read_transaction},
    RootStore, RootStoreError,
};
use crate::{mission_supervision::parse, MissionSupervisionError as E};
/// Existing Go control vocabulary, not a default or permission level inferred
/// from missing data. Automatic still requires explicit consent and other gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionAutonomy {
    Manual,
    Assisted,
    Automatic,
}
impl MissionAutonomy {
    /// Canonical Go-shaped saved token for this observed mode. A recorded
    /// token is observed vocabulary, never a default or permission level.
    fn token(self) -> &'static str {
        match self {
            MissionAutonomy::Manual => "manuel",
            MissionAutonomy::Assisted => "assiste",
            MissionAutonomy::Automatic => "autonome",
        }
    }
}
/// Latest canonical Go-shaped receipt. Actor/time are declarations, not
/// authenticated identity or fresh authority. This is not an approved HTTP DTO.
#[derive(Clone, PartialEq, Eq)]
pub struct MissionConsentObservation {
    pub enabled: bool,
    pub declared_actor: String,
    pub declared_at: String,
    pub recorded_at: String,
    pub event_sequence: i64,
}
impl std::fmt::Debug for MissionConsentObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MissionConsentObservation(<private>)")
    }
}
/// Actual saved controls without fallback/clamping. Positive slots are observed
/// data, not validation against a current operator capacity policy or budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionControlSettings {
    pub autonomy: MissionAutonomy,
    pub paused: bool,
    pub slots: i64,
}
/// One SQL snapshot; None means no such record, never automatic consent.
/// Archive presence is observed only, not proof of its external archive bytes.
/// Facts are not a transferable token or an authorization to launch/accept work.
#[derive(Clone, PartialEq, Eq)]
pub struct MissionControlsObservation {
    pub consent: Option<MissionConsentObservation>,
    pub controls: Option<MissionControlSettings>,
    pub archived: bool,
}
impl std::fmt::Debug for MissionControlsObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MissionControlsObservation(<private>)")
    }
}
/// Canonical fields emitted by Go json.Marshal(MissionPolicy). No inferred bool,
/// merge, null/default or unknown extension is accepted for this consent record.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedConsent {
    enabled: bool,
    actor: String,
    at: String,
}
/// Decode the newest actual record; malformed newest data cannot disappear behind
/// an earlier authorization. No IO outside SQL, mutation, fallback or human/clock
/// authentication occurs. Parser/SQL/finalize causes remain typed and private.
pub(super) fn consent(db: &rusqlite::Connection, work: &str) -> Result<Option<MissionConsentObservation>, E> {
    let Some((seq,at,raw))=optional(db,"SELECT seq,at,message FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1",[work],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?else{return Ok(None)};
    if seq < 1 {
        return Err(E::Refused("mission-consent-sequence-invalid"));
    }
    let value: SavedConsent =
        serde_json::from_str(&raw).map_err(|e| caused("mission-consent-invalid", e))?;
    if crate::text::trim_space(&value.actor).is_empty()
        || crate::agent_terminal_text::terminal_text(&value.actor) != value.actor
    {
        return Err(E::Refused("mission-consent-actor-invalid"));
    }
    parse(&value.at).map_err(|e| caused("mission-consent-time-invalid", e))?;
    parse(&at).map_err(|e| caused("mission-consent-time-invalid", e))?;
    Ok(Some(MissionConsentObservation {
        enabled: value.enabled,
        declared_actor: value.actor,
        declared_at: value.at,
        recorded_at: at,
        event_sequence: seq,
    }))
}
/// Read actual controls and explicit vocabulary. SQL/boolean/type errors refuse;
/// absence remains None and positive slots are never reduced by the old Go cap.
pub(super) fn controls(db: &rusqlite::Connection, work: &str) -> Result<Option<MissionControlSettings>, E> {
    let Some((paused, mode, slots)) = optional(
        db,
        "SELECT paused,autonomy,slots FROM cockpit_controls WHERE work_id=?",
        [work],
        |r| {
            Ok((
                crate::sqlite_value::boolean(r.get_ref(0)?, 0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        },
    )?
    else {
        return Ok(None);
    };
    let autonomy = match mode.as_str() {
        "manuel" => MissionAutonomy::Manual,
        "assiste" => MissionAutonomy::Assisted,
        "autonome" => MissionAutonomy::Automatic,
        _ => return Err(E::Refused("mission-controls-autonomy-invalid")),
    };
    if slots < 1 {
        return Err(E::Refused("mission-controls-slots-invalid"));
    }
    Ok(Some(MissionControlSettings {
        autonomy,
        paused,
        slots,
    }))
}
/// Controls plus an effective slot observation under one held settings
/// reservation. Missing controls remain missing: selected slots are not consent,
/// an inferred autonomy mode or a transferable execution permit. Not an HTTP DTO.
#[derive(Clone)]
pub struct MissionDispatchObservation {
    pub controls: MissionControlsObservation,
    pub slots: crate::SettingCount,
    pub settings_revision: crate::SettingCount,
    pub policy: crate::MissionDispatchPolicy,
}
impl std::fmt::Debug for MissionDispatchObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MissionDispatchObservation(<private>)")
    }
}
/// Safe narrow serializable cockpit projection of the user-approved saved
/// initial autonomy choice. Additive observation; no Go key-parity claim is
/// made for these keys.
///
/// Inputs: the actual saved mission controls and newest consent record of a
/// valid existing Work, the saved installation default exposed by the
/// parent-owned settings accessor, and the settings revision observed with it.
/// Preconditions: unknown Work, malformed newest consent, malformed or invalid
/// saved controls, invalid settings values and reservation/SQL/release
/// failures already refuse with original typed causes; nothing is defaulted
/// past an error.
/// Errors: none after construction — every refusal happens during
/// construction and never degrades into a configured value or empty success.
/// Effects: none; inert data with no IO, no writes and no events.
/// Limits: existing mission controls always win over the installation choice;
/// a Work with no saved controls and no installation default serializes as
/// `autonomy: null` with `source: "not_configured"` so historical cockpits
/// stay readable and are never granted autonomy; `consent_enabled` mirrors
/// only the newest consent record and a missing record is `false` (absent
/// consent is never consent); `launch_authorized` is always `false`, so this
/// value grants no launch, acceptance, capacity or other authority; no actor,
/// timestamp or prompt data is carried.
#[derive(Clone, serde::Serialize)]
pub struct CockpitAutonomyObservation {
    /// Recorded token ("manuel", "assiste", "autonome") or `null` when no
    /// saved choice exists anywhere; never inferred, defaulted or clamped.
    pub autonomy: Option<String>,
    /// Exactly "mission", "installation" or "not_configured".
    pub source: &'static str,
    /// Settings revision observed alongside the installation default.
    pub settings_revision: crate::SettingCount,
    /// Newest actual consent record's enabled flag; a missing record is
    /// `false`, never consent.
    pub consent_enabled: bool,
    /// Always `false`: an observation never authorizes a launch.
    pub launch_authorized: bool,
}
impl std::fmt::Debug for CockpitAutonomyObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CockpitAutonomyObservation(<private>)")
    }
}
/// Require canonical mission pause/archive state before an activation mutation.
/// Read inside the caller's Work write transaction. Missing controls are legacy
/// absence, not consent; malformed controls/SQL/finalization failures refuse with
/// preserved causes. Any archive row blocks mutation without trusting its path.
/// No policy/default adoption, permission, process or business acceptance follows.
pub(crate) fn require_planning_mission_active(
    db: &rusqlite::Connection,
    work: &str,
) -> Result<(), E> {
    let observed = controls(db, work).map_err(|e| caused("planning-controls-invalid", e))?;
    if observed.is_some_and(|c| c.paused) {
        return Err(E::Refused("planning-mission-paused"));
    }
    let count = optional(
        db,
        "SELECT count(*) FROM main.mission_lifecycle WHERE work_id=?",
        [work],
        |r| r.get::<_, i64>(0),
    )
    .map_err(|e| caused("planning-archive-observation-invalid", e))?
    .ok_or(E::Refused("planning-archive-observation-missing"))?;
    if count < 0 {
        return Err(E::Refused("planning-archive-count-invalid"));
    }
    if count > 0 {
        return Err(E::Refused("mission_archived"));
    }
    Ok(())
}

impl RootStore {
    /// Read saved slot policy and actual mission controls, holding the Root
    /// settings reservation through the read-only SQL transaction and selection.
    /// Legacy/partial policy, out-of-policy saved count, malformed controls,
    /// missing Work and read/release failures refuse with original typed causes.
    /// No settings/SQL mutation, clamping, consent, capacity, launch or acceptance
    /// follows; a future dispatcher must recheck its own authoritative transaction.
    /// Caller still owes Root.close; no reservation survives this return.
    pub fn read_mission_dispatch_controls(
        &self,
        work: &str,
    ) -> Result<MissionDispatchObservation, E> {
        let root = |e| E::Root(Box::new(e));
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(|e| root(RootStoreError::Storage(e)))?;
        let result = (|| {
            let settings = self.read_settings().map_err(root)?;
            let policy = settings
                .values
                .mission_dispatch_policy()
                .map_err(E::Settings)?;
            let controls = self.read_mission_controls(work)?;
            let slots = policy
                .slots(controls.controls.as_ref().map(|c| c.slots))
                .map_err(E::Settings)?;
            Ok(MissionDispatchObservation {
                controls,
                slots,
                settings_revision: settings.revision,
                policy,
            })
        })();
        crate::mission_supervision::finish(
            result,
            reservation
                .close()
                .map_err(|e| root(RootStoreError::Storage(e))),
        )
    }

    /// Read the cockpit autonomy observation for one valid existing Work,
    /// holding the Root settings reservation through the read-only SQL
    /// transaction and closing it via the shared finish helper so primary and
    /// cleanup errors are both retained.
    ///
    /// Inputs: `work`, saved settings and actual mission controls/newest
    /// consent. Missing historical choice remains None; a present choice uses
    /// the closed typed settings accessor and every validation error propagates.
    /// Preconditions: the settings reservation is acquirable and the Work
    /// exists; the caller still owes Root.close.
    /// Errors: unknown Work, malformed newest consent, malformed or invalid
    /// saved controls, invalid settings values and reservation/read/release
    /// failures refuse with their original typed causes. Malformed saved
    /// controls are authoritative and are never replaced by the installation
    /// default; no error is converted into a configured value.
    /// Effects: read-only SQL plus in-memory selection only; no settings or SQL
    /// mutation, no events, no launch, acceptance or permission change.
    /// Limits: existing mission controls always win over the installation
    /// choice; with no saved controls and no installation default the result
    /// serializes as `autonomy: null`, `source: "not_configured"`, so
    /// historical cockpits remain readable and are never granted autonomy;
    /// consent_enabled mirrors only the newest consent record and a missing
    /// record is false; launch_authorized is always false; no actor, timestamp
    /// or prompt data is exposed.
    pub fn read_cockpit_autonomy(&self, work: &str) -> Result<CockpitAutonomyObservation, E> {
        let root = |e| E::Root(Box::new(e));
        let reservation = self
            .directory
            .reserve(false, false)
            .map_err(|e| root(RootStoreError::Storage(e)))?;
        let result = (|| {
            let settings = self.read_settings().map_err(root)?;
            // Historical absence is explicit data, not an accessor error that
            // may hide an invalid value. A present choice must validate.
            let saved_default = if settings.values.cockpit_default_autonomy.is_some() {
                Some(settings.values.cockpit_default_autonomy_mode().map_err(E::Settings)?)
            } else {
                None
            };
            // Existing mission controls always win; malformed saved controls
            // already refused above and are never replaced by the default.
            let observed = self.read_mission_controls(work)?;
            let (autonomy, source) = match observed.controls.as_ref().map(|c| c.autonomy) {
                Some(mode) => (Some(mode.token().to_owned()), "mission"),
                None => match saved_default {
                    Some(token) => (Some(token.to_owned()), "installation"),
                    None => (None, "not_configured"),
                },
            };
            Ok(CockpitAutonomyObservation {
                autonomy,
                source,
                settings_revision: settings.revision,
                consent_enabled: observed.consent.as_ref().is_some_and(|c| c.enabled),
                launch_authorized: false,
            })
        })();
        crate::mission_supervision::finish(
            result,
            reservation
                .close()
                .map_err(|e| root(RootStoreError::Storage(e))),
        )
    }

    /// Observe consent, controls and archive presence for a valid existing Work
    /// in one read-only SQL snapshot. Unknown Work, malformed newest consent,
    /// invalid controls and SQL/finalize/rollback failures refuse with typed cause.
    /// Missing records stay None; no defaults, cap, migrations or permission write.
    /// This does not validate budget/organization/agent identity, process liveness,
    /// operator capacity or automatic execution. A future launch must recheck its
    /// own authoritative transaction. Root.close remains owed; no settings lock
    /// is needed because this reader consumes only stored SQL facts.
    pub fn read_mission_controls(&self, work: &str) -> Result<MissionControlsObservation, E> {
        read_transaction(&self.db, || {
            crate::read_recorded_work(&self.db, work)
                .map_err(|e| E::Root(Box::new(RootStoreError::Work(e))))?;
            let consent = consent(&self.db, work)?;
            let controls = controls(&self.db, work)?;
            let count: i64 = optional(
                &self.db,
                "SELECT count(*) FROM mission_lifecycle WHERE work_id=?",
                [work],
                |r| r.get(0),
            )?
            .ok_or(E::Refused("mission-controls-row-missing"))?;
            Ok(MissionControlsObservation {
                consent,
                controls,
                archived: count != 0,
            })
        })
    }
}
