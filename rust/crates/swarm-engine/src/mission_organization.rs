//! Structural mission prerequisites, not authenticated consent or execution authority.
use crate::{PlanningState, PlanningStateError, ReviewerConfig, ValidationPolicyError, Work};
use std::{error::Error, fmt};

/// Safe refusal with original typed state/coverage causes and private reviewer
/// failure retained. None of these variants permits an agent, provider or commit.
pub enum MissionOrganizationError {
    Structure(&'static str),
    State(Box<PlanningStateError>),
    Coverage(Box<ValidationPolicyError>),
    ReviewerRequired,
    ReviewerUnavailable {
        calls: i64,
        limit: i64,
        failure: String,
    },
}
impl MissionOrganizationError {
    /// Existing Go-facing family; no task identifiers, provider or private failure.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Structure(_) | Self::State(_) | Self::Coverage(_) => "organization_incomplete",
            Self::ReviewerRequired => "reviewer_required",
            Self::ReviewerUnavailable { .. } => "reviewer_unavailable",
        }
    }
    /// Static actionable boundary. Budget values remain configured observations,
    /// not a policy default; invalid negative consumption is never reset/clamped.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Structure(reason) => reason,
            Self::State(_) => "organization-state-invalid",
            Self::Coverage(_) => "organization-task-validation-incomplete",
            Self::ReviewerRequired => "organization-reviewer-missing",
            Self::ReviewerUnavailable { calls, .. } if *calls < 0 => "reviewer-counter-invalid",
            Self::ReviewerUnavailable { failure, .. } if !failure.is_empty() => {
                "reviewer-recorded-failure"
            }
            Self::ReviewerUnavailable { .. } => "reviewer-budget-exhausted",
        }
    }
    /// Preserve a recorded diagnostic for trusted local recovery only. The caller
    /// must not publish it through an unapproved HTTP DTO or unsafe logs.
    pub fn recorded_failure(&self) -> Option<&str> {
        match self {
            Self::ReviewerUnavailable { failure, .. } if !failure.is_empty() => Some(failure),
            _ => None,
        }
    }
}
impl fmt::Display for MissionOrganizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for MissionOrganizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for MissionOrganizationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::State(e) => Some(e.as_ref()),
            Self::Coverage(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}
/// Read configured metadata, not a verified identity, fresh permission or actual
/// provider availability. Positive explicit max_calls is the existing protocol.
fn configured_reviewer(planning: &PlanningState) -> Option<&ReviewerConfig> {
    planning
        .reviewer
        .as_deref()
        .filter(|r| !r.provider.is_empty() && !r.authorized.is_empty() && r.max_calls > 0)
}
/// Validate hierarchical structural prerequisites on supplied Work data without IO.
/// Requires planning, independent-review metadata even with declared human review,
/// valid state/root/parents, planner provider, requirement controls or declared human
/// review, task scope ownership and explicitly authorized covered validation.
/// Returns typed original state/coverage errors or a safe missing boundary. Does
/// not mutate, authenticate declarations, verify external review/controls/provider
/// digests/cooldowns, inspect budget/capacity or authorize a launch/acceptance. A
/// runtime consumer must load authoritative Work and recheck its own transaction.
pub fn validate_mission_organization(work: &Work) -> Result<(), MissionOrganizationError> {
    use MissionOrganizationError as E;
    let p = work
        .planning
        .as_deref()
        .ok_or(E::Structure("organization-planning-missing"))?;
    if configured_reviewer(p).is_none() {
        return Err(E::Structure("organization-reviewer-missing"));
    }
    crate::validate_planning_state(work).map_err(|e| E::State(Box::new(e)))?;
    let scopes = p.scopes.as_deref().unwrap_or_default();
    if !scopes.iter().any(|s| s.id == "root" && s.parent.is_empty()) {
        return Err(E::Structure("organization-root-invalid"));
    }
    if p.provider.is_empty() {
        return Err(E::Structure("organization-planner-missing"));
    }
    if p.human_review_authorized.is_empty() {
        for i in 0..work.criteria.as_ref().map_or(0, Vec::len) {
            let key = format!("req-{}", i + 1);
            if p.checks
                .as_ref()
                .and_then(|m| m.get(&key))
                .and_then(Option::as_ref)
                .is_none_or(Vec::is_empty)
            {
                return Err(E::Structure("organization-requirement-controls-missing"));
            }
        }
    }
    for scope in scopes {
        if scope.id != "root" && !scopes.iter().any(|s| s.id == scope.parent) {
            return Err(E::Structure("organization-parent-missing"));
        }
    }
    for task in work.tasks.iter().flatten() {
        if !scopes.iter().any(|s| s.id == task.scope_id) {
            return Err(E::Structure("organization-task-scope-missing"));
        }
        let policy = task
            .validation_policy
            .as_deref()
            .filter(|p| {
                matches!(p.mode.as_str(), "automatic" | "human") && !p.authorized_at.is_empty()
            })
            .ok_or(E::Structure("organization-task-validation-unauthorized"))?;
        crate::validation_policy_covers_task(policy, task).map_err(|e| E::Coverage(Box::new(e)))?;
    }
    Ok(())
}
/// Check declared reviewer call capacity after structural validation, without IO.
/// Missing configured reviewer, negative consumption, calls at/above the explicit
/// budget or recorded failure refuses. Original counters/failure are retained;
/// no reset, clamp, hidden retry/cap or provider probe occurs. Structural readiness
/// may remain valid after exhaustion; callers must require both guards and actual
/// provider/digest/cooldown/identity checks before any launch. This is not a permit.
pub fn validate_mission_reviewer_capacity(
    planning: &PlanningState,
) -> Result<(), MissionOrganizationError> {
    use MissionOrganizationError as E;
    let r = configured_reviewer(planning).ok_or(E::ReviewerRequired)?;
    if r.calls < 0 || r.calls >= r.max_calls || !r.failure.is_empty() {
        return Err(E::ReviewerUnavailable {
            calls: r.calls,
            limit: r.max_calls,
            failure: r.failure.clone(),
        });
    }
    Ok(())
}
