//! Mission quota data and atomic candidate updates; no execution or identity grant.
use crate::{AgentLaunchError as E, QuotaAuthorization, QuotaValues, Work};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Explicit operator change. No actor/time, usage reset or provider identity can
/// be supplied. Optional fields preserve existing data; zero semantics differ
/// between budgets, inherited scope limits and nullable context caps.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QuotaChange {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_activations: Option<BTreeMap<String, i64>>,
    pub limits: QuotaValues,
    /// Native extension: absent/null preserves the saved mission value; explicit
    /// zero removes the reviewer deadline. This is separate from call quotas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_timeout_seconds: Option<Box<i64>>,
    pub reason: String,
}
/// Observe persisted values without substituting initial Go/installation caps.
/// None for a context field means unconfigured, not unlimited; explicit zero is
/// unlimited there. Remaining None for zero scope/depth/judge caps is unlimited.
/// Manifest transport is absent because native managed review is not connected.
#[derive(Clone, Serialize)]
pub struct QuotaView {
    pub revision: i64,
    pub limits: QuotaValues,
    pub consumed: QuotaValues,
    pub remaining: QuotaValues,
    pub bounds: QuotaValues,
    pub last_authorization: Option<Box<QuotaAuthorization>>,
    pub scopes: Vec<crate::PlanningScope>,
    pub review_manifest_transport_bytes: Option<i64>,
    pub review_manifest_supported: bool,
    pub context_window_override_supported: bool,
    /// Actual mission reviewer duration; None means no reviewer configuration,
    /// not unlimited. Some(0) explicitly means no total reviewer deadline.
    pub review_timeout_seconds: Option<i64>,
}
/// Strict inherited envelope decoding. Unknown authority fields, invalid names,
/// version/revision or blank audit reason refuse before IO. Nonblank reason is a
/// protocol audit invariant; no hidden8/2000 text policy is introduced. Caller
/// retains authoritative transport-size checks. Original codec causes survive.
pub fn decode_quota_change(raw: &[u8]) -> Result<QuotaChange, E> {
    use crate::model_wire::Kind;
    let fields = &[
        ("schema_version", Kind::Int64),
        ("event_id", Kind::String),
        ("expected_revision", Kind::Int64),
        ("scope_activations", Kind::Map(&Kind::Int64)),
        ("limits", Kind::Object(crate::models::shape_quota_values)),
        ("reason", Kind::String),
        ("review_timeout_seconds", Kind::Pointer(&Kind::Int64)),
    ];
    let value: QuotaChange = crate::work::decode_mode(raw, fields, true)
        .map_err(|e| E::caused("quota-request-invalid", e))?;
    if value.schema_version != 1
        || value.expected_revision < 1
        || !swarm_identity_contract::name(value.event_id.as_bytes())
        || !crate::text::nonempty(&value.reason)
    {
        return Err(E::Refused("quota-request-invalid"));
    }
    Ok(value)
}
/// Interpret the actual saved mission duration: zero has no total deadline.
/// Positive seconds must be representable by this platform's monotonic clock;
/// this is a technical runtime bound, not a hidden operator ceiling. Negative
/// conversion retains its typed cause. Reads the monotonic clock only; starts
/// no timer/process and performs no persistence, provider call or policy fallback.
/// Runtime callers must create their actual deadline again at the call boundary.
pub(crate) fn review_timeout_duration(seconds: i64) -> Result<Option<std::time::Duration>, E> {
    let seconds = u64::try_from(seconds).map_err(|e| E::caused("quota-review-timeout-invalid", e))?;
    if seconds == 0 { return Ok(None); }
    let duration = std::time::Duration::from_secs(seconds);
    if std::time::Instant::now().checked_add(duration).is_none() {
        return Err(E::Refused("quota-review-timeout-unrepresentable"));
    }
    Ok(Some(duration))
}
/// Convert actual collection cardinality without truncation or an operator cap.
fn count(value: usize) -> Result<i64, E> {
    i64::try_from(value).map_err(|e| E::caused("quota-count-representation", e))
}
/// Index the actual tree without inferring budgets from invalid ancestry.
/// Depths are memoized without recursion or a walk of every ancestor per node.
/// Config edits may repair insufficient positive caps, so no admission/exhaustion check
/// is used as a prerequisite to reading them. Missing/duplicate/cyclic IDs refuse.
fn tree(work: &Work) -> Result<i64, E> {
    let planning = work
        .planning
        .as_deref()
        .ok_or(E::Refused("quota-planning-required"))?;
    let scopes = planning.scopes.as_deref().unwrap_or_default();
    let mut index = BTreeMap::new();
    for scope in scopes {
        if !swarm_identity_contract::name(scope.id.as_bytes())
            || index.insert(scope.id.as_str(), scope).is_some()
        {
            return Err(E::Refused("quota-hierarchy-invalid"));
        }
    }
    if index.get("root").is_none_or(|s| !s.parent.is_empty()) {
        return Err(E::Refused("quota-hierarchy-invalid"));
    }
    let mut depths = BTreeMap::from([("root", 1_i64)]);
    let mut maximum = 1;
    for scope in scopes {
        let mut seen = BTreeSet::new();
        let mut path = Vec::new();
        let mut node = scope;
        while !depths.contains_key(node.id.as_str()) {
            if !seen.insert(node.id.as_str()) {
                return Err(E::Refused("quota-hierarchy-invalid"));
            }
            path.push(node.id.as_str());
            node = index
                .get(node.parent.as_str())
                .copied()
                .ok_or(E::Refused("quota-hierarchy-invalid"))?;
        }
        let mut depth = depths[node.id.as_str()];
        for id in path.into_iter().rev() {
            depth = depth
                .checked_add(1)
                .ok_or(E::Refused("quota-hierarchy-depth-overflow"))?;
            depths.insert(id, depth);
        }
        maximum = maximum.max(depth);
    }
    Ok(maximum)
}
/// Snapshot stored limits/consumption and derived remaining capacity, including
/// exhausted budgets. Negative consumption refuses, never becomes credit. Missing
/// reviewer context remains absent; no configured cap is clamped or adopted.
/// Input is immutable; no IO, reservation, inference or acceptance occurs.
pub fn quota_view(work: &Work) -> Result<QuotaView, E> {
    let depth = tree(work)?;
    let p = work
        .planning
        .as_deref()
        .ok_or(E::Refused("quota-planning-required"))?;
    if p.activations < 0
        || p.decisions < 0
        || p.reviewer.as_deref().is_some_and(|r| r.calls < 0)
        || p.judge.as_deref().is_some_and(|j| j.cycle < 0)
    {
        return Err(E::Refused("quota-consumption-invalid"));
    }
    let scopes = p.scopes.as_deref().unwrap_or_default();
    let task_count = count(work.tasks.as_deref().unwrap_or_default().len())?;
    let scope_count = count(scopes.len())?;
    if scopes.iter().any(|s| s.activations < 0) {
        return Err(E::Refused("quota-consumption-invalid"));
    }
    let mut limits = QuotaValues {
        planning_activations: p.max_activations,
        planning_decisions: p.max_decisions,
        planning_tasks: Some(Box::new(p.max_tasks)),
        planning_scopes: Some(Box::new(p.max_scopes)),
        delegation_depth: Some(Box::new(p.max_depth)),
        ..Default::default()
    };
    let mut used = QuotaValues {
        planning_activations: p.activations,
        planning_decisions: p.decisions,
        planning_tasks: Some(Box::new(task_count)),
        planning_scopes: Some(Box::new(scope_count)),
        delegation_depth: Some(Box::new(depth)),
        ..Default::default()
    };
    if let Some(r) = p.reviewer.as_deref() {
        limits.review_calls = Some(Box::new(r.max_calls));
        used.review_calls = Some(Box::new(r.calls));
        limits.review_context_tokens = Some(Box::new(r.input_context_tokens));
        limits.review_context_files = r.max_context_files.clone();
        limits.review_source_bytes = r.max_source_bytes.clone();
        limits.review_context_bytes = r.max_context_bytes.clone();
        limits.review_delivery_files = r.max_files.clone();
    }
    if let Some(j) = p.judge.as_deref() {
        limits.judge_cycles = Some(Box::new(j.max_cycles));
        used.judge_cycles = Some(Box::new(j.cycle));
    }
    let remaining_count = |limit: i64, consumed: i64| limit.saturating_sub(consumed).max(0);
    let remaining = QuotaValues {
        planning_activations: remaining_count(p.max_activations, p.activations),
        planning_decisions: remaining_count(p.max_decisions, p.decisions),
        planning_tasks: Some(Box::new(remaining_count(p.max_tasks, task_count))),
        planning_scopes: (p.max_scopes > 0)
            .then(|| Box::new(remaining_count(p.max_scopes, scope_count))),
        delegation_depth: (p.max_depth > 0).then(|| Box::new(remaining_count(p.max_depth, depth))),
        review_calls: p
            .reviewer
            .as_deref()
            .map(|r| Box::new(remaining_count(r.max_calls, r.calls))),
        judge_cycles: p
            .judge
            .as_deref()
            .filter(|j| j.max_cycles > 0)
            .map(|j| Box::new(remaining_count(j.max_cycles, j.cycle))),
        ..Default::default()
    };
    Ok(QuotaView {
        revision: work.revision,
        limits,
        consumed: used,
        remaining,
        bounds: QuotaValues::default(),
        last_authorization: p.quota_authorization.clone(),
        scopes: scopes.to_vec(),
        review_manifest_transport_bytes: None,
        review_manifest_supported: false,
        context_window_override_supported: false,
        review_timeout_seconds: p.reviewer.as_deref().map(|r| r.timeout_seconds),
    })
}
/// Apply quota data on a private candidate, swapping it only after every check.
/// Positive count budgets cannot erase consumption; optional fields preserve
/// current values. Zero scope/depth/context/judge ceilings remove only that cap;
/// explicit per-scope zero restores global inheritance under ancestor admission.
/// Live planner holders/running reviews refuse. New positive client-window
/// overrides are unported and refuse; unchanged historical values remain data.
/// The explicit saved reason policy applies before mutation; no text truncation
/// or hidden minimum/maximum is used. Actor/time are caller-observed audit data,
/// not authenticated human identity.
/// Optional reviewer duration is validated and saved independently; omission
/// preserves it and zero removes only its deadline. Platform clock bounds are
/// technical, not a second operator policy. No provider or timer starts.
/// No Work revision, attempt, reservation, pause/failure or consumed counter is
/// changed here; transaction owner owns publication/replay/revision and freshness.
pub fn apply_quota_change(
    work: &mut Work,
    request: &QuotaChange,
    actor: &str,
    at: &str,
    reason_policy: &crate::QuotaReasonPolicy,
) -> Result<(), E> {
    if request.schema_version != 1
        || !swarm_identity_contract::name(request.event_id.as_bytes())
        || !crate::text::nonempty(&request.reason)
        || !crate::text::nonempty(actor)
    {
        return Err(E::Refused("quota-request-invalid"));
    }
    if request.expected_revision != work.revision {
        return Err(E::Refused("quota-revision-conflict"));
    }
    reason_policy.check(&request.reason)?;
    swarm_wire_json::go_rfc3339::parse(at).map_err(|e| E::caused("quota-time-invalid", e))?;
    let current = quota_view(work)?;
    if let Some(seconds) = request.review_timeout_seconds.as_deref() {
        if work.planning.as_deref().and_then(|p| p.reviewer.as_deref()).is_none() {
            return Err(E::Refused("quota-reviewer-required"));
        }
        review_timeout_duration(*seconds)?;
    }
    let task_count = count(work.tasks.as_deref().unwrap_or_default().len())?;
    let mut candidate = work.clone();
    let p = candidate
        .planning
        .as_mut()
        .ok_or(E::Refused("quota-planning-required"))?;
    let change = &request.limits;
    if change.planning_activations < p.activations.max(1)
        || change.planning_decisions < p.decisions.max(1)
        || change
            .planning_tasks
            .as_deref()
            .is_some_and(|n| *n < task_count.max(1))
    {
        return Err(E::Refused("quota-below-consumption"));
    }
    for (requested, consumed) in [
        (
            change.planning_scopes.as_deref(),
            current.consumed.planning_scopes.as_deref(),
        ),
        (
            change.delegation_depth.as_deref(),
            current.consumed.delegation_depth.as_deref(),
        ),
    ] {
        if requested.is_some_and(|n| *n < 0 || (*n > 0 && consumed.is_some_and(|used| n < used))) {
            return Err(E::Refused("quota-below-hierarchy"));
        }
    }
    let contexts = [
        &change.review_context_tokens,
        &change.review_context_files,
        &change.review_source_bytes,
        &change.review_context_bytes,
        &change.review_delivery_files,
    ];
    if contexts
        .iter()
        .any(|v| v.as_deref().is_some_and(|n| *n < 0 || p.reviewer.is_none()))
    {
        return Err(E::Refused("quota-reviewer-context-invalid"));
    }
    match p.reviewer.as_deref() {
        None if change.review_calls.is_some() => return Err(E::Refused("quota-reviewer-required")),
        Some(r) => {
            if change
                .review_calls
                .as_deref()
                .is_none_or(|n| *n < r.calls.max(1))
            {
                return Err(E::Refused("quota-reviewer-below-consumption"));
            }
            if change
                .review_context_tokens
                .as_deref()
                .is_some_and(|n| *n > 0 && *n != r.input_context_tokens)
            {
                return Err(E::Refused("quota-client-window-unported"));
            }
        }
        _ => {}
    }
    if let Some(n) = change.judge_cycles.as_deref() {
        let judge = p
            .judge
            .as_deref()
            .ok_or(E::Refused("quota-judge-required"))?;
        if *n < 0 || (*n > 0 && *n < judge.cycle) {
            return Err(E::Refused("quota-judge-below-consumption"));
        }
    }
    if p.scopes.iter().flatten().any(|s| !s.holder.is_empty()) {
        return Err(E::Refused("quota-planner-owner-active"));
    }
    if candidate.tasks.iter().flatten().any(|t| {
        t.independent_review
            .as_deref()
            .is_some_and(|r| r.state == "running")
    }) {
        return Err(E::Refused("quota-reviewer-running"));
    }
    let mut before_scopes = BTreeMap::new();
    let mut after_scopes = BTreeMap::new();
    for (id, n) in request.scope_activations.iter().flatten() {
        let scope = p
            .scopes
            .iter()
            .flatten()
            .find(|s| s.id == *id)
            .ok_or(E::Refused("quota-scope-missing"))?;
        if *n < 0
            || (*n > 0 && *n < scope.activations.max(1))
            || *n > change.planning_activations
            || (*n == 0 && scope.activations > change.planning_activations)
        {
            return Err(E::Refused("quota-scope-below-consumption"));
        }
        before_scopes.insert(id.clone(), scope.max_activations);
        after_scopes.insert(id.clone(), *n);
    }
    for scope in p.scopes.iter().flatten().filter(|s| !s.parent.is_empty()) {
        let parent = p
            .scopes
            .iter()
            .flatten()
            .find(|s| s.id == scope.parent)
            .ok_or(E::Refused("quota-hierarchy-invalid"))?;
        let overrides = request.scope_activations.as_ref();
        let selected = overrides.and_then(|m| m.get(&scope.id));
        let parent_selected = overrides.and_then(|m| m.get(&parent.id));
        let limit = selected.copied().unwrap_or(scope.max_activations);
        let parent_limit = parent_selected.copied().unwrap_or(parent.max_activations);
        let parent_limit = if parent_limit == 0 {
            change.planning_activations
        } else {
            parent_limit
        };
        if (selected.is_some() || parent_selected.is_some()) && limit > parent_limit {
            return Err(E::Refused("quota-child-exceeds-parent"));
        }
    }
    p.max_activations = change.planning_activations;
    p.max_decisions = change.planning_decisions;
    if let Some(n) = change.planning_tasks.as_deref() {
        p.max_tasks = *n;
    }
    if let Some(n) = change.planning_scopes.as_deref() {
        p.max_scopes = *n;
    }
    if let Some(n) = change.delegation_depth.as_deref() {
        p.max_depth = *n;
    }
    if let Some(r) = p.reviewer.as_mut() {
        if let Some(seconds) = request.review_timeout_seconds.as_deref() { r.timeout_seconds = *seconds; }
        r.max_calls = **change
            .review_calls
            .as_ref()
            .ok_or(E::Refused("quota-reviewer-budget-required"))?;
        if let Some(n) = change.review_context_tokens.as_deref() {
            r.input_context_tokens = *n;
        }
        for (value, target) in [
            (&change.review_context_files, &mut r.max_context_files),
            (&change.review_source_bytes, &mut r.max_source_bytes),
            (&change.review_context_bytes, &mut r.max_context_bytes),
            (&change.review_delivery_files, &mut r.max_files),
        ] {
            if let Some(value) = value {
                *target = Some(value.clone());
            }
        }
    }
    if let Some(n) = change.judge_cycles.as_deref() {
        p.judge
            .as_mut()
            .ok_or(E::Refused("quota-judge-required"))?
            .max_cycles = *n;
    }
    for (id, n) in &after_scopes {
        p.scopes
            .as_mut()
            .and_then(|s| s.iter_mut().find(|s| s.id == *id))
            .ok_or(E::Refused("quota-scope-missing"))?
            .max_activations = *n;
    }
    let after = quota_view(&candidate)?.limits;
    candidate
        .planning
        .as_mut()
        .ok_or(E::Refused("quota-planning-required"))?
        .quota_authorization = Some(Box::new(QuotaAuthorization {
        before_scope_activations: Some(before_scopes),
        after_scope_activations: Some(after_scopes),
        before: current.limits,
        after,
        actor: actor.into(),
        at: at.into(),
        reason: crate::text::trim_space(&request.reason).into(),
    }));
    *work = candidate;
    Ok(())
}
