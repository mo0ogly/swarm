//! Atomic cycle observations and changed-error journal under the current lease.
//! This records the trusted caller's report, not proof of an autonomous decision.
use super::{
    bound, caused, execute, next, optional, owned_heartbeat, write, MissionLease, RootStore, E,
};
use crate::MissionLeaseClock;
use rusqlite::params;
impl RootStore {
    /// Record one completed or failed local supervision cycle under its live lease.
    /// None reports completion; Some carries private diagnostic text, normalized
    /// with the shared terminal Unicode rules. A diagnostic empty after cleaning
    /// refuses rather than becoming success. The caller must exclude secrets;
    /// normalization is not redaction and this is not an approved HTTP input.
    /// Root/current process, epoch/owner/frozen policy and time are checked before
    /// updating heartbeat, last check, next check and first successful reconciliation.
    /// Changed errors and recovery stage one coordination event in the same SQL
    /// transaction. Repeated equal diagnostics stage none, but renew observation.
    /// Returns the new event key only on transition. SQL/clock/entropy/commit and
    /// reservation cleanup causes remain typed; no implicit retry, provider launch,
    /// business acceptance, authenticated actor or actual repair is inferred.
    pub fn record_mission_cycle(
        &self,
        lease: &MissionLease,
        now: &str,
        cycle_error: Option<&str>,
    ) -> Result<Option<String>, E> {
        bound(self, lease)?;
        super::parse(now)?;
        let message = match cycle_error {
            None => String::new(),
            Some(raw) => {
                let value = crate::agent_terminal_text::terminal_text(raw);
                if value.trim().is_empty() {
                    return Err(E::Refused("mission-cycle-error-empty"));
                }
                value
            }
        };
        write(self, |_| {
            let heartbeat = owned_heartbeat(&self.db, lease)?;
            if lease.policy.observe(&heartbeat, now)? != MissionLeaseClock::Present {
                return Err(E::Refused("mission-lease-not-present"));
            }
            let deadline = next(now, &lease.policy)?;
            let previous:String=optional(&self.db,"SELECT last_error FROM mission_supervision WHERE work_id=? AND conductor_id=? AND stopped_at=''",params![lease.work,lease.conductor],|r|r.get(0))?.ok_or(E::Refused("mission-lease-lost"))?;
            let changed=execute(&self.db,"UPDATE mission_supervision SET heartbeat_at=?,reconciled_at=CASE WHEN reconciled_at='' AND ?='' THEN ? ELSE reconciled_at END,last_check_at=?,last_error=?,next_check_at=? WHERE work_id=? AND conductor_id=? AND stopped_at=''",params![now,message,now,now,message,deadline,lease.work,lease.conductor])?;
            if changed != 1 {
                return Err(E::Refused("mission-lease-lost"));
            }
            if previous == message {
                return Ok(None);
            }
            let event = crate::generate_identifier("cycle-")
                .map_err(|e| caused("mission-cycle-event-unavailable", e))?;
            let (kind, detail) = if message.is_empty() {
                ("cycle-recovered", "Cycle repris après erreur")
            } else {
                ("cycle-error", message.as_str())
            };
            let written=execute(&self.db,"INSERT INTO mission_coordination_events(event_key,work_id,agent_id,conductor_id,kind,at,detail) VALUES(?,?,?,?,?,?,?)",params![event,lease.work,"",lease.conductor,kind,now,detail])?;
            if written != 1 {
                return Err(E::Refused("mission-cycle-event-missing"));
            }
            Ok(Some(event))
        })
    }
}
