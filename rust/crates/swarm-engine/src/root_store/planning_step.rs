//! One explicit native activation, preserving inbox FIFO and original scope guards.
use super::{finished, RootStore, RootStoreError as E};
use crate::{
    AgentLaunchError as A, PlanningCallPreviewRequest, PlanningRequest, SettingCount, Work,
};
use serde::Serialize;
use std::collections::BTreeSet;

/// One observed activation, never a complete workflow, acceptance or invoice.
/// Idle means no open-scope input exists; denied/busy/invalid input is an error.
#[derive(Serialize)]
pub struct PlanningStepOutcome {
    pub schema_version: i64,
    pub status: &'static str,
    pub accepted: bool,
    pub selected_scope: Option<String>,
    pub input_event: Option<String>,
    pub settings_revision: SettingCount,
    pub lease_seconds: Option<i64>,
    pub claim_id: Option<String>,
    pub run: Option<super::PlanningRunOutcome>,
}
/// Preserve typed preparation/SQL/state errors under their static safe reason.
fn launch(e: A) -> E {
    E::Launch(Box::new(e))
}
/// Selection data is observed in the same read transaction as Work and registry.
/// It is not a lease/exec permit; authoritative admission repeats current guards.
struct Choice {
    scope: String,
    input: String,
}
/// Choose the oldest unacknowledged input's eligible scope, not first role order.
/// Closed scopes are ignored; each scope is checked once. Busy/exhausted/replan
/// conditions may yield to another eligible scope, but malformed state/SQL never
/// becomes an empty result. All-blocked is an error; only no open input is idle.
/// This preserves the existing FIFO contract, not a new configurable priority.
fn choose(db: &rusqlite::Connection, work: &Work, at: &str) -> Result<Option<Choice>, A> {
    crate::validate_planning_state(work)
        .map_err(|e| A::caused("planning-step-state-invalid", e))?;
    let p = work
        .planning
        .as_deref()
        .ok_or(A::Refused("planning-not-enabled"))?;
    if p.paused {
        return Err(A::Refused("planning_paused"));
    }
    if !p.failure.is_empty() {
        return Err(A::Refused("planning-stopped"));
    }
    if p.repository.is_some() {
        return Err(A::Refused("planning-claim-repository-unported"));
    }
    let now =
        swarm_wire_json::go_rfc3339::parse(at).map_err(|e| A::caused("planning-step-clock", e))?;
    let mut seen = BTreeSet::new();
    let mut blocked = None;
    for event in p.inbox.iter().flatten() {
        if !event.decision.is_empty() || !seen.insert(&event.scope) {
            continue;
        }
        let scope = p
            .scopes
            .iter()
            .flatten()
            .find(|s| s.id == event.scope)
            .ok_or(A::Refused("planning-scope-not-found"))?;
        if scope.state == "closed" {
            continue;
        }
        let runs = crate::planning_run_store::scope_runs(db, &work.id, &scope.id)?;
        if runs.iter().any(|r| r.phase.retains_ownership()) {
            blocked.get_or_insert(A::Refused("planning-claim-call-active"));
            continue;
        }
        // Unnotified/legacy or superseded terminal calls cannot auto-repeat in
        // their old generation. Delivered causal feedback advances generation
        // separately; this guard never fabricates that delivery or a retry grant.
        if runs.iter().any(|r| {
            r.phase == crate::planning_run_store::Phase::Refused && r.generation == scope.generation
        }) {
            blocked.get_or_insert(A::Refused("planning-step-replan-required"));
            continue;
        }
        if !scope.holder.is_empty() {
            let until = swarm_wire_json::go_rfc3339::parse(&scope.lease_until)
                .map_err(|e| A::caused("planning-scope-lease-invalid", e))?;
            if now < until {
                blocked.get_or_insert(A::Refused("lease_busy"));
                continue;
            }
        }
        if crate::planning_rejection_unchanged(work, &scope.id)
            .map_err(|e| A::caused(e.code(), e))?
        {
            blocked.get_or_insert(A::Refused("rejection_requires_change"));
            continue;
        }
        if let Err(e) = crate::check_planning_activation_budget(work, &scope.id) {
            let exhausted = matches!(e, crate::PlanningBudgetError::Exhausted { .. });
            let error = A::caused(e.reason(), e);
            if exhausted {
                blocked.get_or_insert(error);
                continue;
            } else {
                return Err(error);
            }
        }
        return Ok(Some(Choice {
            scope: scope.id.clone(),
            input: event.id.clone(),
        }));
    }
    match blocked {
        Some(error) => Err(error),
        None => Ok(None),
    }
}
impl RootStore {
    /// Perform at most the one activation explicitly requested by this command.
    /// Select current inbox FIFO under a settings reservation, consume saved27
    /// default lease, then use normal authoritative claim and protected run.
    /// New claims spend the original budgets; no hidden CAS/inference retry,
    /// quota reset, acceptance or background supervision is performed. Missing
    /// policy/source/ownership refuses. Caller must use the foreground CLI image
    /// and close Root; automatic incident-driven recovery remains unported.
    pub fn step_planning(&mut self, work: &str) -> Result<PlanningStepOutcome, E> {
        if !self.writable {
            return Err(E::Refused("root-read-only"));
        }
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let prepared = crate::agent_storage_retry::execute(
            &mut self.db,
            &self.directory,
            false,
            |db, settings| {
                let mut tx = db.transaction().map_err(E::Sql)?;
                tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
                let value = (|| {
                    let value = crate::read_recorded_work(&tx, work).map_err(E::Work)?;
                    let at = crate::system_timestamp().map_err(E::Runtime)?;
                    let choice = choose(&tx, &value, &at).map_err(launch)?;
                    let policy = settings
                        .values
                        .planning_wait_policy()
                        .map_err(E::Settings)?;
                    Ok((
                        value,
                        choice,
                        settings.revision,
                        policy.default_lease_seconds.0,
                    ))
                })();
                finished(value, tx.rollback().map_err(E::Sql))
            },
        );
        let (old, choice, settings_revision, lease) = match prepared {
            Ok(v) => v,
            Err(e) => return finished(Err(e), reservation.close().map_err(E::Storage)),
        };
        let Some(choice) = choice else {
            reservation.close().map_err(E::Storage)?;
            return Ok(PlanningStepOutcome {
                schema_version: 1,
                status: "idle",
                accepted: false,
                selected_scope: None,
                input_event: None,
                settings_revision,
                lease_seconds: None,
                claim_id: None,
                run: None,
            });
        };
        let prepared = (|| {
            let scope = old
                .planning
                .as_deref()
                .and_then(|p| p.scopes.as_deref())
                .and_then(|s| s.iter().find(|s| s.id == choice.scope))
                .ok_or(E::Refused("planning-scope-not-found"))?;
            let claim = PlanningRequest {
                schema_version: 1,
                event_id: crate::generate_identifier("planning-claim-").map_err(E::Runtime)?,
                expected_revision: old.revision,
                scope: scope.id.clone(),
                scope_revision: scope.revision,
                holder: crate::generate_identifier("native-planner-").map_err(E::Runtime)?,
                lease_seconds: lease,
                ..Default::default()
            };
            let raw = crate::encode_planning_request(&claim).map_err(E::Encode)?;
            Ok((claim, raw))
        })();
        let (claim, raw) = match prepared {
            Ok(v) => v,
            Err(e) => return finished(Err(e), reservation.close().map_err(E::Storage)),
        };
        // Use ordinary admission, including the existing mission boundary, rather
        // than replacing it with a weaker selector-only mutation fence.
        let claimed =
            self.execute_external(work, "planning.claim", &raw, &mut crate::SystemEnvironment);
        let known = claimed
            .as_ref()
            .ok()
            .map(|v| (v.result.work.id.clone(), v.result.work.revision));
        let claimed =
            finished(claimed, reservation.close().map_err(E::Storage)).map_err(|primary| {
                match known {
                    Some((work, revision)) => E::WorkRecorded {
                        primary: Box::new(primary),
                        work,
                        revision,
                    },
                    None => primary,
                }
            })?;
        let raw = (|| {
            let scope = claimed
                .result
                .work
                .planning
                .as_deref()
                .and_then(|p| p.scopes.as_deref())
                .and_then(|s| s.iter().find(|s| s.id == choice.scope))
                .ok_or(E::Refused("planning-scope-not-found"))?;
            let request = PlanningCallPreviewRequest {
                schema_version: 1,
                claim_id: claim.event_id.clone(),
                expected_revision: claimed.result.work.revision,
                scope: scope.id.clone(),
                scope_revision: scope.revision,
                holder: scope.holder.clone(),
                generation: scope.generation,
            };
            serde_json::to_vec(&request)
                .map_err(|e| E::Launch(Box::new(A::caused("planning-step-call-encoding", e))))
        })()
        .map_err(|primary| E::WorkRecorded {
            primary: Box::new(primary),
            work: claimed.result.work.id.clone(),
            revision: claimed.result.work.revision,
        })?;
        let run = self.run_planning(work, &raw).map_err(|primary| {
            if primary.recorded_work().is_some() {
                primary
            } else {
                E::WorkRecorded {
                    primary: Box::new(primary),
                    work: claimed.result.work.id.clone(),
                    revision: claimed.result.work.revision,
                }
            }
        })?;
        Ok(PlanningStepOutcome {
            schema_version: 1,
            status: "applied",
            accepted: false,
            selected_scope: Some(choice.scope),
            input_event: Some(choice.input),
            settings_revision,
            lease_seconds: Some(lease),
            claim_id: Some(claim.event_id),
            run: Some(run),
        })
    }
}
