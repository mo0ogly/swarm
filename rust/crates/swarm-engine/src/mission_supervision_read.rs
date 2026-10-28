//! Read-only native lease facts for recovery, without ownership or policy adoption.
use super::{
    mission_supervision_store::{
        claim_revision, epoch_schema, legacy_unproven, optional, previous, read_transaction,
    },
    RootStore, RootStoreError,
};
use crate::{
    mission_supervision::parse, MissionLeaseClock, MissionSupervisionError as E,
    MissionSupervisionPolicy, SettingCount,
};
/// Observed state only, never a decision to start, stop, accept or signal a worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionSupervisionState {
    LegacyUnverified,
    Stopped,
    Present,
    CycleError,
    Stale,
    Future,
}
/// Consistent SQL facts with effective scheduling separated from retained values.
/// Private stored source/error text is returned to the trusted local caller only;
/// this is not an approved HTTP DTO. No PID/host/conductor credential is exposed.
/// Debug deliberately omits content. No actor or clock authentication follows.
pub struct MissionSupervisionObservation {
    pub state: MissionSupervisionState,
    pub source: String,
    pub started_at: String,
    pub observed_at: String,
    pub reconciled_at: String,
    pub last_check_at: String,
    pub last_error: String,
    pub stopped_at: String,
    /// Raw retained timestamp; never advertise it as a current deadline alone.
    pub recorded_next_check_at: String,
    /// Only a present native lease with a valid nonempty deadline has an effective
    /// next check. Stopped/stale/future/legacy states always return None.
    pub next_check_at: Option<String>,
    pub late: Option<bool>,
    pub generation: Option<i64>,
    pub policy: Option<MissionSupervisionPolicy>,
    /// Exact claim revision from its optional normalized receipt and validated
    /// settings history. Legacy None is unproven, never a current/default guess.
    pub settings_revision: Option<SettingCount>,
}
impl std::fmt::Debug for MissionSupervisionObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MissionSupervisionObservation(<private>)")
    }
}
/// Decode one record with authoritative columns. This does not classify time,
/// infer a policy, expose a conductor token or substitute missing row values.
fn record(row: &rusqlite::Row<'_>) -> rusqlite::Result<(MissionSupervisionObservation, String)> {
    Ok((
        MissionSupervisionObservation {
            state: MissionSupervisionState::LegacyUnverified,
            source: row.get(0)?,
            started_at: row.get(1)?,
            observed_at: row.get(2)?,
            reconciled_at: row.get(3)?,
            last_check_at: row.get(4)?,
            last_error: row.get(5)?,
            recorded_next_check_at: row.get(6)?,
            stopped_at: row.get(7)?,
            next_check_at: None,
            late: None,
            generation: None,
            policy: None,
            settings_revision: None,
        },
        row.get(8)?,
    ))
}
impl RootStore {
    /// Read an existing Work's lease in one read-only SQL snapshot, on writable
    /// or read-only Root. A valid Work with no record returns None; unknown/corrupt
    /// Work, SQL/schema/clock/rollback failures retain typed causes and refuse.
    /// Legacy rows have explicitly unverified policy and no effective deadline.
    /// Supplied RFC3339 time is an observation, never authenticated. The read
    /// creates/migrates no epoch table, acquires no handle, changes no settings,
    /// retries nothing and starts/signals/accepts no agent. Root.close remains owed.
    pub fn read_mission_supervision(
        &self,
        work: &str,
        now: &str,
    ) -> Result<Option<MissionSupervisionObservation>, E> {
        let observed = parse(now)?;
        read_transaction(&self.db, || {
            crate::read_recorded_work(&self.db, work)
                .map_err(|e| E::Root(Box::new(RootStoreError::Work(e))))?;
            let has_schema = epoch_schema(&self.db)?;
            let Some((mut value,conductor))=optional(&self.db,"SELECT source,started_at,heartbeat_at,reconciled_at,last_check_at,last_error,next_check_at,stopped_at,conductor_id FROM mission_supervision WHERE work_id=?",[work],record)? else{return Ok(None)};
            // Validate every stored temporal claim; empty optional stamps retain
            // their protocol absence. Invalid text never becomes an absent lease.
            parse(&value.started_at)?;
            parse(&value.observed_at)?;
            for raw in [
                &value.reconciled_at,
                &value.last_check_at,
                &value.recorded_next_check_at,
                &value.stopped_at,
            ] {
                if !raw.is_empty() {
                    parse(raw)?;
                }
            }
            let native = has_schema
                && optional(
                    &self.db,
                    "SELECT 1 FROM native_mission_lease_epochs WHERE work_id=?",
                    [work],
                    |r| r.get::<_, i64>(0),
                )?
                .is_some();
            if !native {
                legacy_unproven(&self.db, work, &conductor)?;
            }
            if native {
                let (generation, policy) = previous(&self.db, work, &conductor)?;
                value.settings_revision =
                    claim_revision(self, work, generation, &conductor, &policy)?;
                value.generation = Some(generation);
                value.state = if !value.stopped_at.is_empty() {
                    MissionSupervisionState::Stopped
                } else {
                    match policy.observe(&value.observed_at, now)? {
                        MissionLeaseClock::Stale => MissionSupervisionState::Stale,
                        MissionLeaseClock::Future => MissionSupervisionState::Future,
                        MissionLeaseClock::Present if value.last_error.is_empty() => {
                            MissionSupervisionState::Present
                        }
                        MissionLeaseClock::Present => MissionSupervisionState::CycleError,
                    }
                };
                if matches!(
                    value.state,
                    MissionSupervisionState::Present | MissionSupervisionState::CycleError
                ) && !value.recorded_next_check_at.is_empty()
                {
                    value.late = Some(observed > parse(&value.recorded_next_check_at)?);
                    value.next_check_at = Some(value.recorded_next_check_at.clone());
                }
                value.policy = Some(policy);
            }
            Ok(Some(value))
        })
    }
}
