//! Leased operator decisions. Declared holders are fences, not authenticated agents.
use crate::{
    AgentLaunchError as E, PlanningDecisionPolicy as P, PlanningOperation, PlanningRequest, Work,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

/// Check complete action applicability before any operation or acknowledgement.
/// Empty operations are an explicit observation; unrelated request metadata refuses.
fn envelope(r: &PlanningRequest) -> Result<(), E> {
    if r.schema_version != 1 {
        return Err(E::Refused("planning-schema-required"));
    }
    let raw = crate::encode_planning_request(r)
        .map_err(|e| E::caused("planning-decision-encoding", e))?;
    let fields: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&raw).map_err(|e| E::caused("planning-decision-encoding", e))?;
    if fields.keys().any(|k| {
        !matches!(
            k.as_str(),
            "schema_version"
                | "event_id"
                | "expected_revision"
                | "scope"
                | "scope_revision"
                | "holder"
                | "generation"
                | "input_events"
                | "operations"
                | "reason"
        )
    }) {
        return Err(E::Refused("planning-decision-field-not-applicable"));
    }
    if !swarm_identity_contract::name(r.scope.as_bytes())
        || !swarm_identity_contract::name(r.holder.as_bytes())
        || !crate::text::nonempty(&r.reason)
    {
        return Err(E::Refused("planning-decision-envelope-invalid"));
    }
    Ok(())
}
/// Validate current persisted lease/holder/generation/revision, including at the
/// final publication observation. No mutation, budget refund or time tolerance.
pub(crate) fn fence(w: &Work, r: &PlanningRequest, at: &str) -> Result<(), E> {
    let p = w
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    let s = p
        .scopes
        .iter()
        .flatten()
        .find(|s| s.id == r.scope)
        .ok_or(E::Refused("planning-scope-not-found"))?;
    if p.paused || !p.failure.is_empty() || s.state == "closed" {
        return Err(E::Refused("planning-decision-stopped"));
    }
    if s.revision != r.scope_revision {
        return Err(E::Refused("revision_conflict"));
    }
    if s.holder.is_empty() || s.holder != r.holder || s.generation != r.generation {
        return Err(E::Refused("stale_lease"));
    }
    let now = swarm_wire_json::go_rfc3339::parse(at)
        .map_err(|e| E::caused("planning-decision-clock-invalid", e))?;
    let until = swarm_wire_json::go_rfc3339::parse(&s.lease_until)
        .map_err(|e| E::caused("planning-decision-deadline-invalid", e))?;
    if now >= until {
        return Err(E::Refused("stale_lease"));
    }
    Ok(())
}
/// Derive Go-compatible stable event identifiers from complete serialized parts.
/// This is deduplication data, never cryptographic caller authentication.
pub(crate) fn event_id(parts: &[&str]) -> Result<String, E> {
    let bytes = crate::encode_model(&parts)
        .map_err(|e| E::caused("planning-decision-event-encoding", e))?;
    Ok(format!("plan-{:x}", Sha256::digest(bytes)))
}
/// Preserve original operator validation authority for all task requirements.
/// Incomplete automatic checks grant no automatic policy. Model criteria remain
/// suggestions and cannot replace authorized requirement text/control coverage.
fn inherit_checks(
    w: &Work,
    t: &mut crate::Task,
    settings: &crate::EngineSettingsDocument,
    at: &str,
) -> Result<(), E> {
    let p = w
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    if !p.human_review_authorized.is_empty() {
        t.validation_policy = Some(Box::new(crate::ValidationPolicy {
            mode: "human".into(),
            authorized_at: p.human_review_authorized.clone(),
            actor: "autorisation initiale de la mission".into(),
            ..Default::default()
        }));
        return Ok(());
    }
    let requirements = t.requirements.as_deref().unwrap_or_default();
    if requirements.iter().any(|r| {
        p.checks
            .as_ref()
            .and_then(|c| c.get(r))
            .and_then(Option::as_ref)
            .is_none_or(Vec::is_empty)
    }) {
        return Ok(());
    }
    let mut controls = Vec::new();
    let mut criteria = Vec::new();
    for (i, req) in requirements.iter().enumerate() {
        let owned = w
            .criteria
            .iter()
            .flatten()
            .enumerate()
            .find(|(n, _)| format!("req-{}", n + 1) == *req)
            .ok_or(E::Refused("planning-task-requirement-missing"))?;
        criteria.push(owned.1.clone());
        for control in p
            .checks
            .as_ref()
            .and_then(|c| c.get(req))
            .and_then(Option::as_ref)
            .into_iter()
            .flatten()
        {
            let mut control = control.clone();
            control.criteria = Some(vec![i64::try_from(i)
                .ok()
                .and_then(|v| v.checked_add(1))
                .ok_or(E::Refused(
                "planning-criterion-index-representation",
            ))?]);
            let hash = format!(
                "{:x}",
                Sha256::digest(format!("{req}:{}", control.id).as_bytes())
            );
            control.id = format!("check-{}", &hash[..24]);
            controls.push(control);
        }
    }
    if controls.is_empty() {
        return Ok(());
    }
    let limits = settings
        .values
        .mutation_limits()
        .map_err(|e| E::caused(e.reason(), e))?;
    let mut policy = crate::normalize_validation_policy(
        crate::ValidationPolicy {
            mode: "automatic".into(),
            authorized_at: at.into(),
            actor: "autorisation initiale de la mission".into(),
            controls: Some(controls),
        },
        &limits.policy,
    )
    .map_err(|e| E::caused("planning-task-checks-invalid", e))?;
    // Normalization deliberately strips supplied stamps. Restore only the
    // engine-owned time/attribution derived from original requirement controls.
    policy.authorized_at = at.into();
    policy.actor = "autorisation initiale de la mission".into();
    t.next.push_str(&format!(
        "\nPrécisions de réalisation proposées : {}",
        t.criteria.as_deref().unwrap_or_default().join(" ; ")
    ));
    t.criteria = Some(criteria);
    t.plan_checks = Some(
        crate::task_definition::task_policy_checks(
            t.plan_checks.as_ref(),
            t.criteria.as_deref().unwrap_or_default(),
            Some(&policy),
            limits.task.manual_gate_phases.as_ref(),
        )
        .map_err(|e| E::caused("planning-task-check-mapping-invalid", e))?,
    );
    crate::validation_policy_covers_task(&policy, t)
        .map_err(|e| E::caused("planning-task-check-coverage-invalid", e))?;
    t.validation_policy = Some(Box::new(policy));
    Ok(())
}
/// Check structural closure eligibility without IO or process observations.
/// The validated hierarchy must have closed children, accepted descendants and
/// own requirement coverage. This is prompt capability metadata only; decision
/// separately rereads original evidence, pending inputs and physical ownership.
pub(crate) fn closure_structure(w: &Work, scope: &str) -> Result<(), E> {
    let p = w.planning.as_deref().ok_or(E::Refused("planning-not-enabled"))?;
    let scopes = p.scopes.as_deref().unwrap_or_default();
    let owner = scopes.iter().find(|s| s.id == scope).ok_or(E::Refused("planning-scope-not-found"))?;
    if owner.state == "closed" || scopes.iter().any(|s| s.parent == scope && s.state != "closed") {
        return Err(E::Refused("planning-closure-child-open"));
    }
    let mut covered = BTreeSet::new();
    for task in w.tasks.iter().flatten() {
        if task.scope_id.is_empty() { return Err(E::Refused("planning-closure-task-unscoped")); }
        let ancestors = crate::planning_scope_ancestors(w, &task.scope_id)
            .map_err(|e| E::caused("planning-closure-tree-invalid", e))?;
        if (task.scope_id == scope || ancestors.iter().any(|s| s.id == scope)) && task.status != "accepted" {
            return Err(E::Refused("planning-closure-task-not-accepted"));
        }
        if task.scope_id == scope { covered.extend(task.requirements.iter().flatten().map(String::as_str)); }
    }
    if owner.requirements.iter().flatten().any(|r| !covered.contains(r.as_str())) {
        return Err(E::Refused("planning-closure-requirement-uncovered"));
    }
    Ok(())
}
/// Prepare logical closure only after all pending inputs and current descendant
/// proof chains pass. Saved quality/mutation policies apply without new defaults.
/// Original IO/contract errors retain causes. No process is stopped or committed;
/// WorkEngine separately checks registered physical ownership in its transaction.
fn close_scope(root: &Path, w: &mut Work, op: &PlanningOperation,
    r: &PlanningRequest, at: &str, settings: &crate::EngineSettingsDocument, policy: &P) -> Result<(), E> {
    let scope = r.scope.as_str();
    if !op.id.is_empty() || !op.next.is_empty() || !op.deliverable.is_empty()
        || op.requirements.as_ref().is_some_and(|v| !v.is_empty())
        || op.criteria.as_ref().is_some_and(|v| !v.is_empty())
        || op.depends.as_ref().is_some_and(|v| !v.is_empty())
        || !op.judge_verdict.is_empty() || op.judge_cycle != 0 || !op.expected_candidate.is_empty()
        || op.max_tasks != 0 || op.max_activations != 0 {
        return Err(E::Refused("planning-closure-field-not-applicable"));
    }
    P::check(policy.max_title_characters, op.title.chars().count(), "planning-operation-title-limit")?;
    closure_structure(w, scope)?;
    if w.planning.as_deref().unwrap().inbox.iter().flatten().any(|e|
        e.scope == scope && e.decision.is_empty() && !r.input_events.iter().flatten().any(|id| *id == e.id)) {
        return Err(E::Refused("planning-closure-input-pending"));
    }
    let limits = settings.values.mutation_limits().map_err(|e| E::caused(e.reason(), e))?;
    let quality = settings.quality_policy_snapshot().map_err(|e| E::caused(e.reason(), e))?;
    let context = crate::TaskLifecycleContext { root, quality_policy: Some(&quality), task_limits: &limits.task, validation_limits: &limits.policy };
    for task in w.tasks.iter().flatten() {
        let ancestors = crate::planning_scope_ancestors(w, &task.scope_id).map_err(|e| E::caused("planning-closure-tree-invalid", e))?;
        if task.scope_id == scope || ancestors.iter().any(|s| s.id == scope) {
            crate::task_lifecycle::check_scoped_accepted_freshness(w, &task.id, &context, settings)
                .map_err(|e| E::caused("planning-closure-evidence-invalid", e))?;
        }
    }
    let p = w.planning.as_deref_mut().unwrap();
    let s = p.scopes.as_mut().unwrap().iter_mut().find(|s| s.id == scope).unwrap();
    s.state = "closed".into(); let parent = s.parent.clone();
    if !parent.is_empty() {
        p.scopes.as_mut().unwrap().iter_mut().find(|s| s.id == parent).ok_or(E::Refused("planning-closure-parent-missing"))?.state = "ready".into();
        p.inbox.get_or_insert_default().push(crate::PlanningEvent { id: event_id(&[&r.event_id, "closed", scope])?, scope: parent, kind: "scope_closed".into(), message: op.title.clone(), at: at.into(), ..Default::default() });
    }
    Ok(())
}
/// Prepare correction on the SAME blocked native task using a current verified
/// changes-requested review or actual failed/interrupted native completion,
/// explicitly consumed by this decision. Only id/next
/// apply; saved instruction/attempt bounds and launch hold remain. No new task,
/// attempt, call, reservation or identity is created; Root admission rechecks the
/// persisted decision and original proof before any actual worker start.
fn retry_task(root: &Path, w: &mut Work, op: &PlanningOperation, r: &PlanningRequest, settings: &crate::EngineSettingsDocument, policy: &P) -> Result<(), E> {
    if !swarm_identity_contract::name(op.id.as_bytes()) || !crate::text::nonempty(&op.next)
        || !op.title.is_empty() || !op.deliverable.is_empty() || op.requirements.as_ref().is_some_and(|v| !v.is_empty())
        || op.criteria.as_ref().is_some_and(|v| !v.is_empty()) || op.depends.as_ref().is_some_and(|v| !v.is_empty())
        || op.max_tasks != 0 || op.max_activations != 0 || !op.judge_verdict.is_empty() || op.judge_cycle != 0 || !op.expected_candidate.is_empty() {
        return Err(E::Refused("planning-retry-field-not-applicable"));
    }
    P::check(policy.max_instruction_bytes, op.next.len(), "planning-operation-instruction-limit")?;
    let mut tasks = w.tasks.iter().flatten().filter(|t| t.id == op.id);
    let task = tasks.next().ok_or(E::Refused("planning-retry-task-missing"))?;
    if tasks.next().is_some() || task.scope_id != r.scope || task.plan_role != "worker" || task.status != "blocked"
        || task.launch_held || task.planning_retry || task.next == op.next || task.plan_max_attempts <= 0
        || task.attempts.as_ref().map_or(0, Vec::len) as u128 >= task.plan_max_attempts as u128 {
        return Err(E::Refused("planning-retry-unavailable"));
    }
    let input = if task.independent_review.is_some() {
        crate::review_evidence::check_current_native_change_request(root, w, &task.id, settings)
            .map_err(|e| E::caused("planning-retry-current-evidence", e))?.ok_or(E::Refused("planning-retry-current-review-required"))?.review_id
    } else {
        #[cfg(target_os = "linux")]
        { crate::RootStore::failed_worker_retry_input(root, w, &task.id).map_err(|e| E::caused("planning-retry-worker-evidence", e))?.event }
        #[cfg(not(target_os = "linux"))]
        { return Err(E::Refused("planning-retry-worker-platform-unported")); }
    };
    if !r.input_events.iter().flatten().any(|id| *id == input) { return Err(E::Refused("planning-retry-original-input-required")); }
    let task = w.tasks.as_mut().unwrap().iter_mut().find(|t| t.id == op.id).unwrap();
    task.planning_retry = true; task.next = op.next.clone(); Ok(())
}
/// Record a reviewer-only correction after a qualified original failure.
/// Root verifies the known failed response, unchanged report/contract/project and
/// actual original controls. Requires either the pending delivered review event
/// or its frozen unresolved source with fresh delivered inputs, plus saved reviewer
/// capacity. The old acknowledgement remains intact. Only next/new canonical
/// acknowledgement change: no worker/attempt,
/// new review, reservation, receipt or acceptance is created by this decision.
fn retry_review(root: &Path, w: &mut Work, op: &PlanningOperation, r: &PlanningRequest, settings: &crate::EngineSettingsDocument, policy: &P) -> Result<(), E> {
    if !swarm_identity_contract::name(op.id.as_bytes()) || !crate::text::nonempty(&op.next)
        || !op.title.is_empty() || !op.deliverable.is_empty() || op.requirements.as_ref().is_some_and(|v| !v.is_empty())
        || op.criteria.as_ref().is_some_and(|v| !v.is_empty()) || op.depends.as_ref().is_some_and(|v| !v.is_empty())
        || op.max_tasks != 0 || op.max_activations != 0 || !op.judge_verdict.is_empty() || op.judge_cycle != 0 || !op.expected_candidate.is_empty() {
        return Err(E::Refused("planning-review-retry-field-not-applicable"));
    }
    P::check(policy.max_instruction_bytes, op.next.len(), "planning-operation-instruction-limit")?;
    let task = w.tasks.iter().flatten().find(|t| t.id == op.id).ok_or(E::Refused("planning-review-retry-task-required"))?;
    let planning = w.planning.as_deref().ok_or(E::Refused("planning-review-retry-planning-required"))?;
    let delivery = planning.scopes.iter().flatten().find(|s| s.id == r.scope).and_then(|s| s.delivery.as_deref()).ok_or(E::Refused("planning-review-retry-delivery-required"))?;
    if task.scope_id != r.scope || task.status != "submitted" || task.plan_role != "worker" || task.launch_held || task.planning_retry || task.next == op.next
        || delivery.review_retry_unavailable || planning.reviewer.as_deref().is_none_or(|v| v.calls < 0 || v.calls >= v.max_calls) {
        return Err(E::Refused("planning-review-retry-unavailable"));
    }
    let observed = crate::review_evidence::check_current_native_reviewer_failure(root, w, &task.id, settings)
        .map_err(|e| E::caused("planning-review-retry-original", e))?.ok_or(E::Refused("planning-review-retry-original-required"))?;
    let direct = r.input_events.iter().flatten().any(|id| id == &observed.review_id);
    let received = delivery.unresolved_review_sources.as_ref().and_then(|m| m.get(&task.id));
    let fresh = r.input_events.as_ref().is_some_and(|ids| !ids.is_empty())
        && received.is_some_and(|v| v.result_event == observed.review_id && v.review_id == observed.review_id
            && v.attempt == observed.attempt_id
            && matches!((v.failure_code.as_str(),v.owner.as_str()), ("review_fragment_protocol","reviewer_protocol") | ("review_fragment_interrupted","reviewer_runtime"))
            && v.operation == "review-retry" && !v.producer_correction_required && v.candidate_commit.is_empty())
        && planning.inbox.iter().flatten().any(|e| e.id == observed.review_id && e.task == task.id && e.scope == r.scope && !e.decision.is_empty());
    if !direct && !fresh { return Err(E::Refused("planning-review-retry-input-required")); }
    let task = w.tasks.as_mut().unwrap().iter_mut().find(|t| t.id == op.id).unwrap();
    task.next = op.next.clone();
    Ok(())
}
/// Apply one supported task/delegation to a private candidate. Every new task
/// consumes global/ancestor count capacity. Delegation transfers only owned
/// requirements and bounded child budgets; no worker process or acceptance runs.
/// Unsupported operation/metadata refuses the entire caller transaction.
fn operation(
    w: &mut Work,
    scope_id: &str,
    op: &PlanningOperation,
    r: &PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
    policy: &P,
) -> Result<(), E> {
    if !matches!(op.kind.as_str(), "task" | "delegate") {
        return Err(E::Refused("planning-operation-unported"));
    }
    if !op.judge_verdict.is_empty()
        || op.judge_cycle != 0
        || !op.expected_candidate.is_empty()
        || op.kind == "task" && (op.max_tasks != 0 || op.max_activations != 0)
    {
        return Err(E::Refused("planning-operation-field-not-applicable"));
    }
    if !swarm_identity_contract::name(op.id.as_bytes()) || !crate::text::nonempty(&op.title) {
        return Err(E::Refused("planning-operation-id-title-invalid"));
    }
    P::check(
        policy.max_title_characters,
        op.title.chars().count(),
        "planning-operation-title-limit",
    )?;
    for text in [&op.next, &op.deliverable] {
        P::check(
            policy.max_instruction_bytes,
            text.len(),
            "planning-operation-instruction-limit",
        )?;
    }
    let p = w
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    let scopes = p.scopes.as_deref().unwrap_or_default();
    let index = scopes
        .iter()
        .position(|s| s.id == scope_id)
        .ok_or(E::Refused("planning-scope-not-found"))?;
    let scope = &scopes[index];
    let requirements = op.requirements.as_deref().unwrap_or_default();
    let mut seen = BTreeSet::new();
    if requirements.is_empty()
        || requirements.iter().any(|r| {
            !seen.insert(r)
                || !scope
                    .requirements
                    .as_deref()
                    .unwrap_or_default()
                    .contains(r)
        })
    {
        return Err(E::Refused("planning-operation-requirements-invalid"));
    }
    if op.kind == "delegate" {
        if !op.deliverable.is_empty()
            || op.criteria.as_ref().is_some_and(|v| !v.is_empty())
            || op.depends.as_ref().is_some_and(|v| !v.is_empty())
        {
            return Err(E::Refused("planning-delegation-task-fields-invalid"));
        }
        if scopes.iter().any(|s| s.id == op.id) {
            return Err(E::Refused("planning-scope-already-present"));
        }
        let depth = crate::planning_scope_ancestors(w, scope_id)
            .map_err(|e| E::caused("planning-delegation-tree-invalid", e))?
            .len()
            .checked_add(1)
            .ok_or(E::Refused("planning-depth-representation"))?;
        if p.max_depth > 0 && depth as u128 > p.max_depth as u128
            || p.max_scopes > 0 && scopes.len() as u128 >= p.max_scopes as u128
        {
            return Err(E::Refused("planning-delegation-configured-limit"));
        }
        if w.tasks.iter().flatten().any(|t| {
            t.scope_id == scope_id && t.requirements.iter().flatten().any(|r| seen.contains(r))
        }) {
            return Err(E::Refused("planning-requirement-already-assigned"));
        }
        let task_limit = if scope.max_tasks == 0 {
            p.max_tasks
        } else {
            scope.max_tasks
        };
        let activation_limit = if scope.max_activations == 0 {
            p.max_activations
        } else {
            scope.max_activations
        };
        if op.max_tasks < 0
            || op.max_tasks > task_limit
            || op.max_activations < 0
            || op.max_activations > activation_limit
        {
            return Err(E::Refused("planning-child-budget-invalid"));
        }
        let child = crate::PlanningScope {
            id: op.id.clone(),
            parent: scope_id.into(),
            objective: format!("{}\n{}", op.title, op.next),
            requirements: op.requirements.clone(),
            revision: 1,
            state: "ready".into(),
            max_tasks: if op.max_tasks == 0 {
                task_limit
            } else {
                op.max_tasks
            },
            max_activations: if op.max_activations == 0 {
                activation_limit
            } else {
                op.max_activations
            },
            ..Default::default()
        };
        let event = crate::PlanningEvent {
            id: event_id(&[&r.event_id, &op.id])?,
            scope: op.id.clone(),
            kind: "delegation".into(),
            message: op.title.clone(),
            at: at.into(),
            ..Default::default()
        };
        let p = w.planning.as_deref_mut().unwrap();
        let scopes = p.scopes.get_or_insert_default();
        scopes[index]
            .requirements
            .as_mut()
            .unwrap()
            .retain(|r| !seen.contains(r));
        scopes.push(child);
        p.inbox.get_or_insert_default().push(event);
        return Ok(());
    }
    if w.tasks.iter().flatten().any(|t| {
        t.scope_id == scope_id
            && t.status == "blocked"
            && t.requirements.iter().flatten().any(|r| seen.contains(r))
    }) {
        return Err(E::Refused("planning-blocked-requirement-requires-retry"));
    }
    crate::check_planning_task_budget(w, scope_id).map_err(|e| E::caused(e.reason(), e))?;
    P::check(
        policy.max_dependencies,
        op.depends.as_deref().unwrap_or_default().len(),
        "planning-task-dependency-limit",
    )?;
    for dep in op.depends.iter().flatten() {
        if !w
            .tasks
            .iter()
            .flatten()
            .any(|t| t.id == *dep && t.scope_id == scope_id)
        {
            return Err(E::Refused("planning-task-dependency-outside-scope"));
        }
    }
    let limits = settings
        .values
        .mutation_limits()
        .map_err(|e| E::caused(e.reason(), e))?;
    if policy.worker_default_attempts > limits.task.max_attempts {
        return Err(E::Refused("planning-worker-attempts-above-task-policy"));
    }
    let criteria = op.criteria.as_deref().unwrap_or_default();
    if criteria.len() > limits.task.max_criteria {
        return Err(E::Refused("planning-task-criteria-limit"));
    }
    for c in criteria {
        P::check(
            policy.max_criterion_bytes,
            c.len(),
            "planning-task-criterion-bytes-limit",
        )?;
    }
    let defaults = limits
        .task
        .manual_gate_phases
        .as_ref()
        .ok_or(E::Refused("task-gate-defaults-not-configured"))?;
    crate::work_definition::apply_task_addition_with_defaults(
        w,
        &crate::Request {
            schema_version: 1,
            id: op.id.clone(),
            title: op.title.clone(),
            deliverable: op.deliverable.clone(),
            criteria: op.criteria.clone(),
            depends: op.depends.clone(),
            next: op.next.clone(),
            owner: scope_id.into(),
            ..Default::default()
        },
        defaults,
    )
    .map_err(|e| E::caused("planning-task-definition-invalid", e))?;
    let mut task = w
        .tasks
        .as_mut()
        .unwrap()
        .pop()
        .ok_or(E::Refused("planning-created-task-missing"))?;
    task.scope_id = scope_id.into();
    task.requirements = op.requirements.clone();
    task.plan_role = "worker".into();
    task.plan_max_attempts = policy.worker_default_attempts;
    inherit_checks(w, &mut task, settings, at)?;
    crate::validate_task_dependency_graph(w, &task)
        .map_err(|e| E::caused("planning-task-dependency-invalid", e))?;
    w.tasks.as_mut().unwrap().push(task);
    Ok(())
}
/// Permit active operations from an exact current passed native review, or
/// same-task retry only from changes_requested. Re-read Root job/event/private/
/// public originals and current control/
/// report/contract/project proof; current last attempt and scoped inbox must match.
/// A known current original protocol error permits only reviewer correction;
/// historical/other non-verdict/ordinary/managed input refuses. No acknowledgement,
/// source lease, process, budget change or authenticated agent/human is inferred;
/// returns the complete explicitly referenced report after digest/text/saved-cap
/// validation, without fabricating a handoff event from the review.
/// caller retains its authoritative transaction, lease and final commit fences.
fn current_native_review_input(
    root: &Path,
    work: &Work,
    event: &crate::PlanningEvent,
    settings: &crate::EngineSettingsDocument,
    report_limit: Option<usize>,
    operations: &[PlanningOperation],
) -> Result<crate::PlanningReportContent, E> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, work, event, settings, report_limit, operations);
        Err(E::Refused("planning-decision-specialized-input-unported"))
    }
    #[cfg(target_os = "linux")]
    {
        let original = crate::RootStore::native_review_for_context(root, work, event)
            .map_err(|e| E::caused("planning-native-review-input-original", e))?;
        let correction = original.verdict_applicable && original.review.state == "changes_requested"
            && operations.iter().all(|op| op.kind == "retry") && operations.iter().any(|op| op.id == event.task);
        let reviewer_retry = original.retry_failure.is_some() && operations.iter().all(|op| op.kind == "review-retry")
            && operations.iter().any(|op| op.id == event.task);
        if !reviewer_retry && (!original.verdict_applicable || original.review.state != "passed" && !correction) {
            return Err(E::Refused("planning-native-review-current-passed-required"));
        }
        let evidence = if reviewer_retry {
            crate::review_evidence::check_current_native_reviewer_failure(root, work, &event.task, settings)
        } else if correction {
            crate::review_evidence::check_current_native_change_request(root, work, &event.task, settings)
        } else { crate::check_current_review_evidence(root, work, &event.task, settings) };
        let current = evidence.map_err(|e| E::caused("planning-native-review-input-current", e))?
            .ok_or(E::Refused("planning-native-review-current-required"))?;
        if current.review_id != event.id || current.task_id != event.task
            || current.attempt_id != event.attempt
        {
            return Err(E::Refused("planning-native-review-current-changed"));
        }
        let reference = crate::ExchangeArtifact {
            path: current.report,
            sha256: current.report_sha256,
        };
        if reviewer_retry && event.artifacts.is_some() || !reviewer_retry && event.artifacts.as_deref() != Some(std::slice::from_ref(&reference)) {
            return Err(E::Refused("planning-native-review-report-binding"));
        }
        crate::planning_report::read_reference(root, event, &reference, report_limit)
            .map_err(|e| E::caused("planning-native-review-input-report", e))
    }
}
/// Prepare an atomic leased decision from delivered, pending owned events.
/// Report content and current native review originals are reverified (passed
/// for ordinary actions; changes_requested for production retry, known protocol
/// error for reviewer-only correction)
/// before active operations; stale returns permit
/// acknowledgement only. Budget/revision/generation overflows, unsupported
/// operations and any failed preparation preserve the input Work. Caller owns
/// settings/Work reservations, replay before invocation, final fence and commit.
/// Managed repositories admit only acknowledgement, delegation and new tasks
/// after original copy/commit observation; future-task instruction additions also
/// preserve that repository observation. Other managed operations still refuse.
/// Holder equality is a concurrency fence, not authenticated agent authority.
pub(crate) fn decide(
    root: &Path,
    w: &Work,
    r: &PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
) -> Result<Work, E> {
    envelope(r)?;
    settings.validate().map_err(|e| E::caused(e.reason(), e))?;
    crate::validate_planning_state(w)
        .map_err(|e| E::caused("planning-decision-state-invalid", e))?;
    fence(w, r, at)?;
    let policy = settings
        .values
        .planning_decision_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    let p = w
        .planning
        .as_deref()
        .ok_or(E::Refused("planning-not-enabled"))?;
    if p.repository.is_some() {
        if r.operations.iter().flatten().any(|op|!matches!(op.kind.as_str(),"delegate"|"task"|"revise-instructions")) {
            return Err(E::Refused("planning-decision-managed-operation-unported"));
        }
        #[cfg(target_os="linux")]
        crate::managed_repository::observe(root,w,settings)?;
        #[cfg(not(target_os="linux"))]
        return Err(E::Refused("planning-managed-platform-unported"));
    }
    if p.decisions >= p.max_decisions {
        return Err(E::Refused("planning-decision-budget-exhausted"));
    }
    let scope = p
        .scopes
        .iter()
        .flatten()
        .find(|s| s.id == r.scope)
        .ok_or(E::Refused("planning-scope-not-found"))?;
    let delivery = scope
        .delivery
        .as_deref()
        .ok_or(E::Refused("planning-decision-delivery-required"))?;
    if !delivery.decision.is_empty() {
        return Err(E::Refused("planning-delivery-already-decided"));
    }
    let inputs = r.input_events.as_deref().unwrap_or_default();
    let operations = r.operations.as_deref().unwrap_or_default();
    if operations.iter().any(|op| op.kind == "delegate") && delivery.delegation_unavailable {
        return Err(E::Refused("planning-delegation-not-delivered"));
    }
    if inputs.is_empty() {
        return Err(E::Refused("planning-decision-inputs-required"));
    }
    P::check(
        policy.max_inputs,
        inputs.len(),
        "planning-decision-input-limit",
    )?;
    P::check(
        policy.max_operations,
        operations.len(),
        "planning-decision-operation-limit",
    )?;
    P::check(
        policy.max_reason_bytes,
        r.reason.len(),
        "planning-decision-reason-limit",
    )?;
    if delivery.historical_only && !operations.is_empty() {
        return Err(E::Refused("planning-historical-acknowledgement-only"));
    }
    let mut seen = BTreeSet::new();
    let report_limit = settings
        .values
        .planning_report_limit()
        .map_err(|e| E::caused(e.reason(), e))?;
    for id in inputs {
        if !seen.insert(id) || !delivery.events.as_deref().unwrap_or_default().contains(id) {
            return Err(E::Refused("planning-decision-input-not-delivered"));
        }
        let event = p
            .inbox
            .iter()
            .flatten()
            .find(|e| e.id == *id && e.scope == r.scope && e.decision.is_empty())
            .ok_or(E::Refused("planning-decision-input-not-pending"))?;
        if !operations.is_empty() {
            if delivery
                .unavailable_reports
                .as_ref()
                .and_then(|m| m.get(id))
                .is_some_and(|v| !v.is_empty())
            {
                return Err(E::Refused("planning-decision-input-unavailable"));
            }
            if event.exploration.is_some() || event.run_observation.is_some() {
                return Err(E::Refused("planning-decision-specialized-input-unported"));
            }
            let native_report = if event.kind == "independent_review" {
                Some(current_native_review_input(root, w, event, settings, report_limit, operations)?)
            } else {
                None
            };
            if !event.task.is_empty()
                && w.tasks
                    .iter()
                    .flatten()
                    .find(|t| t.id == event.task && t.scope_id == r.scope)
                    .is_none_or(|t| {
                        !event.attempt.is_empty()
                            && t.attempts
                                .as_deref()
                                .unwrap_or_default()
                                .last()
                                .is_none_or(|a| a.id != event.attempt)
                    })
            {
                return Err(E::Refused("planning-decision-stale-attempt"));
            }
            let report = match native_report {
                Some(report) => Some(report),
                None => crate::read_planning_report(root, event, report_limit)
                    .map_err(|e| E::caused("planning-decision-report-invalid", e))?,
            };
            if event.artifacts.iter().flatten().any(|a| {
                report
                    .as_ref()
                    .is_none_or(|r| r.path != a.path || r.sha256 != a.sha256)
            }) {
                return Err(E::Refused("planning-decision-artifact-unverified"));
            }
        }
    }
    let mut next = w.clone();
    for op in operations {
        if matches!(op.kind.as_str(), "judge" | "renew") {
            crate::planning_judge::apply(&mut next, op, r, at, &policy)?;
        } else if op.kind == "close" {
            if operations.len() != 1 { return Err(E::Refused("planning-closure-exclusive-operation")); }
            close_scope(root, &mut next, op, r, at, settings, &policy)?;
        } else if op.kind == "review-retry" {
            retry_review(root, &mut next, op, r, settings, &policy)?;
        } else if op.kind == "revise-instructions" {
            P::check(policy.max_instruction_bytes, op.next.len(), "planning-operation-instruction-limit")?;
            let task=next.tasks.iter_mut().flatten().find(|t|t.id==op.id).ok_or(E::Refused("instruction-task-not-found"))?;
            crate::instruction_learning::revise(task, &r.scope, op, r, at)?;
        } else if op.kind == "retry" {
            retry_task(root, &mut next, op, r, settings, &policy)?;
        } else {
            operation(&mut next, &r.scope, op, r, at, settings, &policy)?;
        }
    }
    let p = next.planning.as_deref_mut().unwrap();
    p.decisions = p
        .decisions
        .checked_add(1)
        .ok_or(E::Refused("planning-decision-count-overflow"))?;
    for event in p.inbox.iter_mut().flatten() {
        if seen.contains(&event.id) {
            event.decision = r.event_id.clone();
        }
    }
    let pending = p
        .inbox
        .iter()
        .flatten()
        .any(|e| e.scope == r.scope && e.decision.is_empty());
    let scope = p
        .scopes
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s.id == r.scope)
        .unwrap();
    scope.rejection_wait = None;
    scope.delivery.as_deref_mut().unwrap().decision = r.event_id.clone();
    scope.revision = scope
        .revision
        .checked_add(1)
        .ok_or(E::Refused("planning-scope-revision-overflow"))?;
    scope.generation = scope
        .generation
        .checked_add(1)
        .ok_or(E::Refused("planning-generation-overflow"))?;
    scope.holder.clear();
    scope.lease_until.clear();
    if scope.state != "closed" { scope.state = if pending { "ready" } else { "waiting" }.into(); }
    if !pending && delivery.historical_only {
        p.inbox.get_or_insert_default().push(crate::PlanningEvent{id:event_id(&[&r.event_id,"historical-acknowledged"])?,scope:r.scope.clone(),kind:"historical_acknowledged".into(),message:format!("Retours historiques acquittés sans opération par {}. Réexaminer le besoin et les exigences encore ouvertes sur l’état actuel ; aucune acceptation nouvelle n’est établie.",r.event_id),at:at.into(),..Default::default()});
        scope.state = "ready".into();
    }
    crate::validate_planning_state(&next)
        .map_err(|e| E::caused("planning-decision-result-invalid", e))?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Pure structural witness only: accepted status here does not invent review
    /// bytes or authorize closure. The public recipe exercises actual originals.
    fn closure_work() -> Work {
        let settings = crate::EngineSettingsDocument::initial(crate::EngineSettingsValues::installation_defaults().unwrap(), "fixture".into(), "2026-10-22T12:00:00Z".into()).unwrap();
        let w = Work { schema_version: 1, id: "closure-work".into(), criteria: Some(vec!["Requirement".into()]), ..Default::default() };
        crate::apply_planning_control(&w, crate::PlanningControlAction::Enable, &PlanningRequest { schema_version: 1, event_id: "closure-brief".into(), max_tasks: 3, max_activations: 3, max_decisions: 3, ..Default::default() }, "2026-10-22T12:00:00Z", &settings).unwrap()
    }
    /// Coverage is required independently of empty task lists or accepted labels.
    #[test]
    fn closure_structure_requires_own_requirement_coverage() {
        let mut w = closure_work();
        assert!(matches!(closure_structure(&w, "root"), Err(E::Refused("planning-closure-requirement-uncovered"))));
        w.tasks = Some(vec![crate::Task { id: "task".into(), scope_id: "root".into(), status: "accepted".into(), requirements: Some(vec!["req-1".into()]), ..Default::default() }]);
        closure_structure(&w, "root").unwrap();
        w.tasks.as_mut().unwrap()[0].status = "submitted".into();
        assert!(matches!(closure_structure(&w, "root"), Err(E::Refused("planning-closure-task-not-accepted"))));
    }
    /// Descendant closure cannot disappear from a parent's structural preflight.
    #[test]
    fn closure_structure_requires_closed_children() {
        let mut w = closure_work();
        let p = w.planning.as_mut().unwrap();
        p.scopes.as_mut().unwrap()[0].requirements = Some(vec![]);
        p.scopes.as_mut().unwrap().push(crate::PlanningScope { id: "child".into(), parent: "root".into(), state: "ready".into(), revision: 1, requirements: Some(vec!["req-1".into()]), ..Default::default() });
        assert!(matches!(closure_structure(&w, "root"), Err(E::Refused("planning-closure-child-open"))));
        w.planning.as_mut().unwrap().scopes.as_mut().unwrap()[1].state = "closed".into();
        closure_structure(&w, "root").unwrap();
    }
    /// A model suggestion cannot replace original authorized requirement checks;
    /// normalization must not erase the engine-owned authorization stamp.
    #[test]
    fn decision_inherited_checks_preserve_original_authorization() {
        let settings = crate::EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "fixture".into(),
            "2026-10-20T12:00:00Z".into(),
        )
        .unwrap();
        let work = Work {
            criteria: Some(vec!["Original requirement".into()]),
            planning: Some(Box::new(crate::PlanningState {
                checks: Some(std::collections::BTreeMap::from([(
                    "req-1".into(),
                    Some(vec![crate::ValidationControl {
                        id: "original".into(),
                        justification: "Verify original authorized requirement".into(),
                        command: Some(vec!["git".into(), "diff".into(), "--check".into()]),
                        criteria: Some(vec![1]),
                        timeout_seconds: 30,
                        ..Default::default()
                    }]),
                )])),
                ..Default::default()
            })),
            ..Default::default()
        };
        let mut task = crate::Task {
            requirements: Some(vec!["req-1".into()]),
            criteria: Some(vec!["MODEL SUGGESTION".into()]),
            ..Default::default()
        };
        inherit_checks(&work, &mut task, &settings, "2026-10-20T12:00:00Z").unwrap();
        assert_eq!(task.criteria, Some(vec!["Original requirement".into()]));
        assert_eq!(
            task.validation_policy.as_ref().unwrap().authorized_at,
            "2026-10-20T12:00:00Z"
        );
        assert!(!task.validation_policy.as_ref().unwrap().actor.is_empty());
    }
}
