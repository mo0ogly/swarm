//! Pure rejected-proposal precondition fingerprint. No diagnostic is delivered,
//! wait persisted, retry authorized or authenticated agent established here.
use crate::{encode_model, planning_scope_ancestors, PlanningBudgetError, Work, WorkError};
use serde::Serialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

/// Safe fingerprint failure retaining invalid-state/codec causes. Never substitutes
/// an empty hash or false unchanged result for a failed prerequisite.
pub enum PlanningRejectionError {
    State(PlanningBudgetError),
    Encode(WorkError),
    Fragment(serde_json::Error),
}
impl PlanningRejectionError {
    /// Stable safe family; supplied values are never rendered as diagnostics.
    pub fn code(&self) -> &'static str {
        "planning-rejection-state-invalid"
    }
}
impl fmt::Display for PlanningRejectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for PlanningRejectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PlanningRejectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::State(e) => e,
            Self::Encode(e) => e,
            Self::Fragment(e) => e,
        })
    }
}
type Result<T> = std::result::Result<T, PlanningRejectionError>;
/// Serialize typed data without converting nested struct fields to sorted maps.
/// Go string formatting is applied by the final encoder; errors preserve causes.
fn fragment(value: &impl Serialize) -> Result<Box<RawValue>> {
    let raw = encode_model(value).map_err(PlanningRejectionError::Encode)?;
    serde_json::from_slice(&raw).map_err(PlanningRejectionError::Fragment)
}
/// Fingerprint Go's rejected-proposal relevant state after durable validation.
/// Includes owned descendant tasks, all global task IDs, pending own non-diagnostic
/// inputs, immediate children and explicit model/reviewer/repository/budget policy.
/// Ignores leases, planning activations/decisions, reviewer Calls and the diagnostic.
/// Preserves Go map sorting, typed field order and nil/empty distinctions. Returns
/// lowercase SHA256 data only, with typed state/codec errors. No IO/mutation,
/// authority, automatic retry, signature, freshness or disclosure right is implied.
/// Standard input-proportional allocations provide no aggregate memory guarantee.
pub fn planning_rejection_state(work: &Work, scope: &str) -> Result<String> {
    let chain = planning_scope_ancestors(work, scope).map_err(PlanningRejectionError::State)?;
    // A successful ancestor chain always includes its selected first scope.
    let owner = chain[0];
    let p = work
        .planning
        .as_deref()
        .ok_or(PlanningRejectionError::State(
            PlanningBudgetError::MissingPlanning,
        ))?;
    let scopes = p.scopes.as_deref().unwrap_or_default();
    let mut children_by_parent: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for child in scopes {
        children_by_parent
            .entry(&child.parent)
            .or_default()
            .push(&child.id);
    }
    let mut owned = BTreeSet::new();
    let mut pending = vec![scope];
    while let Some(id) = pending.pop() {
        owned.insert(id);
        if let Some(children) = children_by_parent.get(id) {
            pending.extend(children.iter().copied());
        }
    }
    let tasks: Vec<_> = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|t| owned.contains(t.scope_id.as_str()))
        .collect();
    let existing_ids: Vec<_> = work
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|t| &t.id)
        .collect();
    let inputs: Vec<_> = p
        .inbox
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|e| e.scope == scope && e.decision.is_empty() && e.kind != "planning_rejected")
        .collect();
    let mut children = Vec::new();
    for child in scopes.iter().filter(|s| s.parent == scope) {
        children.push(BTreeMap::from([
            ("id", fragment(&child.id)?),
            ("state", fragment(&child.state)?),
            ("requirements", fragment(&child.requirements)?),
        ]));
    }
    let mut reviewer = p.reviewer.clone();
    if let Some(value) = reviewer.as_mut() {
        value.calls = 0;
    }
    let values = BTreeMap::from([
        ("existing_task_ids", fragment(&existing_ids)?),
        ("objective", fragment(&owner.objective)?),
        ("requirements", fragment(&owner.requirements)?),
        ("criteria", fragment(&work.criteria)?),
        ("tasks", fragment(&tasks)?),
        ("inputs", fragment(&inputs)?),
        ("children", fragment(&children)?),
        ("provider", fragment(&p.provider)?),
        ("provider_digest", fragment(&p.provider_digest)?),
        ("route", fragment(&p.model_route)?),
        ("scope_model", fragment(&owner.model_selection)?),
        ("reviewer", fragment(&reviewer)?),
        ("repository", fragment(&p.repository)?),
        ("max_tasks", fragment(&p.max_tasks)?),
        ("max_activations", fragment(&p.max_activations)?),
        ("max_decisions", fragment(&p.max_decisions)?),
        ("scope_task_limit", fragment(&owner.max_tasks)?),
        ("scope_activation_limit", fragment(&owner.max_activations)?),
    ]);
    let raw = encode_model(&values).map_err(PlanningRejectionError::Encode)?;
    Ok(format!("{:x}", Sha256::digest(raw)))
}
/// Compare the selected scope's stored wait to the current pure fingerprint.
/// Validates even when no wait exists: invalid/unknown state is an error, not false.
/// False means absent/different data, never permission to claim or launch a model.
/// No wait/diagnostic is persisted; transaction owner must recheck before acting.
pub fn planning_rejection_unchanged(work: &Work, scope: &str) -> Result<bool> {
    let state = planning_rejection_state(work, scope)?;
    let chain = planning_scope_ancestors(work, scope).map_err(PlanningRejectionError::State)?;
    Ok(chain[0]
        .rejection_wait
        .as_deref()
        .is_some_and(|wait| wait.state_sha256 == state))
}
