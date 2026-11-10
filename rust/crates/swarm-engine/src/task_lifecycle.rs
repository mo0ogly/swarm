//! Task lifecycle preparation, never process launch, identity or review authority.
//! Manual entry remains separate; scoped workers require explicit current policies.
use crate::{
    check_gate_evidence, text, update_task_definition, validate_requirement_prerequisites,
    validate_work_request, Attempt, DefinitionContext, GateContractError, PlanningStateError,
    QualityPolicySnapshot, Request, Revalidation, RuntimeError, Task, TaskDefinitionError,
    TaskDefinitionLimits, ValidationPolicyLimits, Work, WorkEnvironment,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    path::Path,
};
pub struct TaskLifecycleContext<'a> {
    pub root: &'a Path,
    pub quality_policy: Option<&'a QualityPolicySnapshot>,
    pub task_limits: &'a TaskDefinitionLimits,
    pub validation_limits: &'a ValidationPolicyLimits,
}
pub enum TaskLifecycleError {
    Refused(&'static str),
    Transition,
    Attempt,
    PolicyMissing,
    HierarchyNotPorted,
    Hierarchy(PlanningStateError),
    Review(crate::ReviewEvidenceError),
    Definition(TaskDefinitionError),
    Planning(PlanningStateError),
    Gate(GateContractError),
    Runtime(RuntimeError),
    Prerequisite(Option<Box<Self>>),
}
impl TaskLifecycleError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Transition => "invalid_transition",
            Self::Attempt => "attempt_state_invalid",
            Self::PolicyMissing => "settings-quality-policy-not-configured",
            Self::HierarchyNotPorted => "command_not_ported",
            Self::Hierarchy(e) => e.code(),
            Self::Review(e) => e.code(),
            Self::Definition(e) => e.code(),
            Self::Planning(e) => e.code(),
            Self::Gate(e) => e.code(),
            Self::Runtime(e) => e.code(),
            Self::Prerequisite(_) => "requirement_prerequisite",
            Self::Refused(_) => "format",
        }
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(r) => r,
            Self::Transition => "task-transition-forbidden",
            Self::Attempt => "running-attempt-missing",
            Self::PolicyMissing => "quality-policy-not-configured",
            Self::HierarchyNotPorted => "hierarchical-lifecycle-not-ported",
            Self::Hierarchy(_) => "hierarchical-state-invalid",
            Self::Review(_) => "current-independent-review-required",
            Self::Definition(_) => "task-definition",
            Self::Planning(_) => "requirement-prerequisites",
            Self::Gate(_) => "current-gate-required",
            Self::Runtime(_) => "runtime-observation",
            Self::Prerequisite(_) => "fresh-prerequisite-evidence-required",
        }
    }
}
impl fmt::Display for TaskLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for TaskLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for TaskLifecycleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Hierarchy(e) => Some(e),
            Self::Review(e) => Some(e),
            Self::Definition(e) => Some(e),
            Self::Planning(e) => Some(e),
            Self::Gate(e) => Some(e),
            Self::Runtime(e) => Some(e),
            Self::Prerequisite(Some(e)) => Some(e.as_ref()),
            _ => None,
        }
    }
}
type Result<T> = std::result::Result<T, TaskLifecycleError>;
fn fail(reason: &'static str) -> TaskLifecycleError {
    TaskLifecycleError::Refused(reason)
}
fn gate(task: &Task, c: &TaskLifecycleContext<'_>) -> Result<()> {
    check_gate_evidence(
        task,
        c.root,
        c.quality_policy.ok_or(TaskLifecycleError::PolicyMissing)?,
    )
    .map(|_| ())
    .map_err(TaskLifecycleError::Gate)
}
/// A traversal-local observation, never retained as a cache/permit across calls.
pub fn check_manual_task_freshness(w: &Work, id: &str, c: &TaskLifecycleContext<'_>) -> Result<()> {
    if w.planning.is_some() {
        return Err(TaskLifecycleError::HierarchyNotPorted);
    }
    check_freshness(w, id, c, None)
}
/// Require an accepted scoped task and fresh gate, review, dependency and
/// requirement-prerequisite evidence under current saved settings. Reads actual
/// originals, retains causes and performs no mutation, execution or acceptance.
pub(crate) fn check_scoped_accepted_freshness(w: &Work, id: &str, c: &TaskLifecycleContext<'_>, settings: &crate::EngineSettingsDocument) -> Result<()> {
    let task = w.tasks.iter().flatten().find(|t| t.id == id).ok_or_else(|| fail("task-dependency-missing"))?;
    if w.planning.is_none() || task.status != "accepted" { return Err(fail("planning-closure-task-not-accepted")); }
    check_freshness(w, id, c, Some(settings))?;
    prerequisites(w, task, c, Some(settings))
}
/// Check the existing scoped dependency and requirement-prerequisite contract
/// before creating a worker copy. Uses actual Work and saved settings, reads
/// review/gate originals and preserves their causes; no task transition, permit,
/// reservation or filesystem write follows. Admission must recheck these inputs
/// in its writer: this observation never grants future execution authority.
pub(crate) fn check_scoped_worker_prerequisites(w: &Work, task: &Task, c: &TaskLifecycleContext<'_>, settings: &crate::EngineSettingsDocument) -> Result<()> {
    if w.planning.is_none() { return Err(TaskLifecycleError::HierarchyNotPorted); }
    prerequisites(w, task, c, Some(settings))?;
    for dependency in task.depends.as_deref().unwrap_or_default() {
        check_freshness(w, dependency, c, Some(settings))?;
    }
    Ok(())
}
/// One traversal of actual gate and dependency evidence. A hierarchy additionally
/// needs explicit current settings and fresh independent-review bytes. No cache
/// or permit survives this call; original review/graph/artifact causes propagate.
fn check_freshness(
    w: &Work,
    id: &str,
    c: &TaskLifecycleContext<'_>,
    settings: Option<&crate::EngineSettingsDocument>,
) -> Result<()> {
    if w.planning.is_some() && settings.is_none() {
        return Err(TaskLifecycleError::HierarchyNotPorted);
    }
    let mut tasks = BTreeMap::new();
    for t in w.tasks.as_deref().unwrap_or_default() {
        if tasks.insert(t.id.as_str(), t).is_some() {
            return Err(fail("task-id-duplicate"));
        }
    }
    let mut visiting = BTreeSet::new();
    let mut done = BTreeSet::new();
    let mut pending = vec![(id, false)];
    while let Some((id, exit)) = pending.pop() {
        if exit {
            visiting.remove(id);
            done.insert(id);
            continue;
        }
        if visiting.contains(id) {
            return Err(fail("task-dependency-cycle"));
        }
        if done.contains(id) {
            continue;
        }
        let task = tasks
            .get(id)
            .ok_or_else(|| fail("task-dependency-missing"))?;
        if w.planning.is_some() {
            let settings = settings.ok_or(TaskLifecycleError::HierarchyNotPorted)?;
            crate::check_current_review_evidence(c.root, w, &task.id, settings)
                .map_err(TaskLifecycleError::Review)?
                .ok_or_else(|| fail("task-independent-review-missing"))?;
        }
        if task.status == "accepted" {
            gate(task, c)?;
        } else if !(task.status == "waived"
            && task
                .manual_override
                .as_ref()
                .is_some_and(|v| text::nonempty(&v.reason)))
        {
            return Err(fail("task-dependency-not-accepted"));
        }
        visiting.insert(id);
        pending.push((id, true));
        for dep in task.depends.as_deref().unwrap_or_default().iter().rev() {
            pending.push((dep.as_str(), false))
        }
    }
    Ok(())
}
fn prerequisites(
    w: &Work,
    t: &Task,
    c: &TaskLifecycleContext<'_>,
    settings: Option<&crate::EngineSettingsDocument>,
) -> Result<()> {
    validate_requirement_prerequisites(w).map_err(TaskLifecycleError::Planning)?;
    let Some(prereqs) = w
        .requirement_prerequisites
        .as_ref()
        .filter(|v| !v.is_empty())
    else {
        return Ok(());
    };
    let requirements = t.requirements.as_deref().unwrap_or_default();
    if requirements.is_empty() {
        return Err(TaskLifecycleError::Prerequisite(None));
    }
    let mut needed = BTreeSet::new();
    let mut pending: Vec<_> = requirements.iter().map(String::as_str).collect();
    while let Some(id) = pending.pop() {
        for dep in prereqs
            .get(id)
            .and_then(Option::as_deref)
            .unwrap_or_default()
        {
            if needed.insert(dep.as_str()) {
                pending.push(dep.as_str())
            }
        }
    }
    for id in needed {
        let mut covered = false;
        let mut cause = None;
        for prior in w.tasks.as_deref().unwrap_or_default() {
            if prior.id == t.id
                || prior.status != "accepted"
                || !prior
                    .requirements
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .any(|r| r == id)
            {
                continue;
            }
            match check_freshness(w, &prior.id, c, settings) {
                Ok(()) => {
                    covered = true;
                    break;
                }
                Err(error) => {
                    if cause.is_none() {
                        cause = Some(Box::new(error))
                    }
                }
            }
        }
        if !covered {
            return Err(TaskLifecycleError::Prerequisite(cause));
        }
    }
    Ok(())
}
fn transition(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    match from {
        "todo" => matches!(to, "running" | "blocked" | "abandoned"),
        "running" => matches!(to, "submitted" | "blocked" | "todo" | "abandoned"),
        "blocked" => matches!(to, "todo" | "running" | "abandoned"),
        "submitted" => matches!(to, "accepted" | "running" | "blocked" | "abandoned"),
        "accepted" | "waived" | "abandoned" => to == "todo",
        _ => false,
    }
}
/// Prepare a Task copy from supplied manual Work data. Root must provide the
/// recorded Work/current policy and retain its transaction/reservations. This
/// records an attempt; it does not prove a provider process actually ran.
pub fn transition_manual_task<E: WorkEnvironment>(
    w: &Work,
    r: &Request,
    c: &TaskLifecycleContext<'_>,
    environment: &mut E,
) -> Result<Task> {
    validate_work_request("task.update", r)
        .map_err(|e| TaskLifecycleError::Definition(crate::TaskDefinitionError::Request(e)))?;
    if w.planning.is_some() {
        return Err(TaskLifecycleError::HierarchyNotPorted);
    }
    transition_task(w, r, c, None, environment)
}
/// Inspect the persisted scoped-worker restart signal before any new attempt.
/// Todo/running keep initial/idempotent lifecycle behavior; every other start
/// needs blocked state and the engine-owned planning_retry signal. Review
/// protocol/active review states take diagnostic precedence over a missing
/// production signal, even while a failed native review leaves its report
/// submitted. They require review recovery, never another production attempt.
/// The saved task attempt maximum is also retained. This pure structural guard
/// authenticates no supervisor: Root callers still verify original decision,
/// source, physical ownership, budgets and transaction before any admission.
pub(crate) fn require_scoped_worker_restart(task: &Task) -> Result<()> {
    if matches!(task.status.as_str(), "todo" | "running") { return Ok(()); }
    if task.independent_review.as_deref().is_some_and(|review| matches!(review.state.as_str(), "error" | "running" | "queued")) {
        return Err(fail("scoped-worker-review-recovery-required"));
    }
    if task.status != "blocked" || !task.planning_retry {
        return Err(fail("scoped-worker-retry-decision-required"));
    }
    if task.plan_max_attempts > 0 && task.attempts.as_ref().map_or(0, Vec::len) as u128 >= task.plan_max_attempts as u128 {
        return Err(fail("scoped-worker-plan-attempts-exhausted"));
    }
    Ok(())
}
/// Prepare only running/blocked transitions for an actual scoped worker in a
/// supplied hierarchical Work, using explicit current settings. Validates state,
/// unique task/ownership, a nonclosed scope for starts, and fresh review+gate evidence for
/// prerequisites. Restart requires the persisted retry gate and retains the
/// task attempt maximum; protocol incidents cannot start another worker. A
/// signalled native correction additionally rereads its review or actual failed
/// completion chain and
/// canonical retry decision before archiving the old review/creating an attempt.
/// Contract edits, acceptance and other statuses remain refused.
/// Returns a Task copy with attributed attempt timestamps/outcomes, or original
/// typed state/review/runtime errors; no Work/SQL mutation or process launch.
/// Caller must hold its actual Root/settings transaction, enforce reservations,
/// provider/attempt budgets and identity, persist matching events and retain all
/// stop/report evidence. Supplied data and policies grant no execution authority.
pub fn transition_scoped_worker_task<E: WorkEnvironment>(
    w: &Work,
    r: &Request,
    c: &TaskLifecycleContext<'_>,
    settings: &crate::EngineSettingsDocument,
    environment: &mut E,
) -> Result<Task> {
    validate_work_request("task.update", r)
        .map_err(|e| TaskLifecycleError::Definition(crate::TaskDefinitionError::Request(e)))?;
    let planning = w
        .planning
        .as_deref()
        .ok_or(TaskLifecycleError::HierarchyNotPorted)?;
    crate::validate_planning_state(w).map_err(TaskLifecycleError::Hierarchy)?;
    settings
        .validate()
        .map_err(|e| TaskLifecycleError::Review(crate::ReviewEvidenceError::Settings(e)))?;
    if !matches!(r.status.as_str(), "running" | "blocked")
        || crate::mutation_guard::edits_definition(r)
    {
        return Err(TaskLifecycleError::HierarchyNotPorted);
    }
    let mut tasks = w.tasks.iter().flatten().filter(|task| task.id == r.id);
    let task = tasks.next().ok_or_else(|| fail("task-unknown"))?;
    if tasks.next().is_some() {
        return Err(fail("task-id-duplicate"));
    }
    let scope = planning
        .scopes
        .iter()
        .flatten()
        .find(|scope| scope.id == task.scope_id)
        .ok_or_else(|| fail("task-scope-missing"))?;
    if r.status == "running" {
        if scope.state == "closed" { return Err(fail("task-scope-closed")); }
        require_scoped_worker_restart(task)?;
        if task.planning_retry && task.status == "blocked" {
            if task.independent_review.is_some() {
                crate::review_evidence::check_current_native_change_request(c.root, w, &task.id, settings)
                    .map_err(TaskLifecycleError::Review)?.ok_or_else(|| fail("native-change-request-required"))?;
            } else {
                #[cfg(target_os = "linux")]
                crate::RootStore::failed_worker_retry_input(c.root, w, &task.id)
                    .map_err(|e| TaskLifecycleError::Review(crate::ReviewEvidenceError::Native(Box::new(e))))?;
                #[cfg(not(target_os = "linux"))]
                return Err(fail("native-worker-retry-platform-unported"));
            }
        }
    }
    transition_task(w, r, c, Some(settings), environment)
}
/// Prepare a local operator's unstarted scoped abandonment or reopening.
/// Caller owns current Root/settings, event/revision/permission transaction and
/// final persistence, and verifies absence of native worker/review/validation
/// ownership in that same transaction. Only status/next are supported, with todo/abandoned source
/// and target, open owned scope and no attempt/review/recovery history. Validate
/// structural state and current settings; reuse normal lifecycle bookkeeping.
/// Returns a Task copy or original typed errors without changing supplied Work,
/// launching a process, authenticating the operator or refunding consumption.
/// Attempted/reviewed tasks and definition/ownership edits remain unported here.
pub(crate) fn transition_scoped_initial_decision<E: WorkEnvironment>(
    w: &Work, r: &Request, c: &TaskLifecycleContext<'_>,
    settings: &crate::EngineSettingsDocument, environment: &mut E,
) -> Result<Task> {
    validate_work_request("task.update", r)
        .map_err(|e| TaskLifecycleError::Definition(crate::TaskDefinitionError::Request(e)))?;
    if !matches!(r.status.as_str(), "todo" | "abandoned")
        || crate::mutation_guard::edits_definition(r) || !r.owner.is_empty()
        || !r.blocker.is_empty() || !r.outcome.is_empty()
    { return Err(fail("scoped-initial-decision-fields-invalid")); }
    let planning = w.planning.as_deref().ok_or(TaskLifecycleError::HierarchyNotPorted)?;
    crate::validate_planning_state(w).map_err(TaskLifecycleError::Hierarchy)?;
    settings.validate().map_err(|e| TaskLifecycleError::Review(crate::ReviewEvidenceError::Settings(e)))?;
    let mut tasks = w.tasks.iter().flatten().filter(|t| t.id == r.id);
    let task = tasks.next().ok_or_else(|| fail("task-unknown"))?;
    if tasks.next().is_some() { return Err(fail("task-id-duplicate")); }
    if !matches!(task.status.as_str(), "todo" | "abandoned") || task.plan_role != "worker"
        || task.attempts.as_ref().is_some_and(|v| !v.is_empty())
        || task.independent_review.is_some() || task.previous_reviews.as_ref().is_some_and(|v| !v.is_empty())
        || task.corrective_recovery.is_some() || task.planning_retry
        || task.review_coordination.is_some() || task.batch_review_resume.is_some()
        || task.recovered_result.is_some()
    { return Err(fail("scoped-initial-decision-history-unported")); }
    let scope = planning.scopes.iter().flatten().find(|s| s.id == task.scope_id)
        .ok_or_else(|| fail("task-scope-missing"))?;
    if scope.state == "closed" { return Err(fail("task-scope-closed")); }
    transition_task(w, r, c, Some(settings), environment)
}
/// Prepare explicit acceptance of one submitted native scoped task, without
/// widening the worker's running/blocked transition API. Caller must hold the
/// authoritative Root/settings transaction and apply its normal admission/commit
/// guards. A declared local operator field proves no human/agent authentication.
/// Only status/next metadata is supported; source/owner/contract edits refuse.
/// Requires an open owned scope, passed authoritative native result plus current
/// report/artifact/contract/context, prerequisite and deterministic gate evidence.
/// Rechecks review sources after transition preparation IO; no result/file is
/// mutated, process launched, reservation refunded or mission accepted here.
/// Typed errors leave the supplied Work unchanged; source checks grant no lease.
pub(crate) fn transition_scoped_reviewed_task<E: WorkEnvironment>(
    w: &Work,
    r: &Request,
    c: &TaskLifecycleContext<'_>,
    settings: &crate::EngineSettingsDocument,
    environment: &mut E,
) -> Result<Task> {
    transition_reviewed(w, r, c, settings, environment, None)
}
/// Prepare the same accepted transition using current managed originals checked
/// by the publication writer in this transaction. Opaque evidence must match the
/// exact inputs; all scope, prerequisite and deterministic gate guards remain.
#[cfg(target_os = "linux")]
pub(crate) fn transition_managed_publication<E: WorkEnvironment>(
    w: &Work, r: &Request, c: &TaskLifecycleContext<'_>,
    settings: &crate::EngineSettingsDocument, environment: &mut E,
    proof: &crate::root_store::PrecheckedAcceptance<'_>,
) -> Result<Task> {
    if !proof.matches(w, settings, &r.id) { return Err(fail("managed-publication-inputs-changed")); }
    transition_reviewed(w, r, c, settings, environment, Some(()))
}
/// Shared admission mechanics. The internal discriminator is never supplied by
/// requests; only the matching opaque-proof wrapper may select managed evidence.
fn transition_reviewed<E: WorkEnvironment>(
    w: &Work, r: &Request, c: &TaskLifecycleContext<'_>,
    settings: &crate::EngineSettingsDocument, environment: &mut E,
    managed: Option<()>,
) -> Result<Task> {
    validate_work_request("task.update", r)
        .map_err(|e| TaskLifecycleError::Definition(crate::TaskDefinitionError::Request(e)))?;
    if r.status != "accepted" || crate::mutation_guard::edits_definition(r)
        || !r.owner.is_empty() || !r.blocker.is_empty() || !r.outcome.is_empty()
    {
        return Err(fail("native-scoped-acceptance-fields-invalid"));
    }
    let planning = w.planning.as_deref().ok_or(TaskLifecycleError::HierarchyNotPorted)?;
    crate::validate_planning_state(w).map_err(TaskLifecycleError::Hierarchy)?;
    settings.validate().map_err(|e| TaskLifecycleError::Review(crate::ReviewEvidenceError::Settings(e)))?;
    let mut tasks = w.tasks.iter().flatten().filter(|t| t.id == r.id);
    let task = tasks.next().ok_or_else(|| fail("task-unknown"))?;
    if tasks.next().is_some() { return Err(fail("task-id-duplicate")); }
    if task.status != "submitted" || task.plan_role != "worker"
        || task.independent_review.as_deref().is_none_or(|review|
            !crate::review_evidence::native_receipt_reference(&review.receipt))
    {
        return Err(fail("native-scoped-submitted-review-required"));
    }
    let scope = planning.scopes.iter().flatten().find(|s| s.id == task.scope_id)
        .ok_or_else(|| fail("task-scope-missing"))?;
    if scope.state == "closed" { return Err(fail("task-scope-closed")); }
    if managed.is_none() {
        crate::check_current_review_evidence(c.root, w, &r.id, settings)
            .map_err(TaskLifecycleError::Review)?.ok_or_else(|| fail("task-independent-review-missing"))?;
    }
    let next = transition_task(w, r, c, Some(settings), environment)?;
    if managed.is_none() {
        crate::check_current_review_evidence(c.root, w, &r.id, settings)
            .map_err(TaskLifecycleError::Review)?.ok_or_else(|| fail("task-independent-review-missing"))?;
    }
    Ok(next)
}
/// Shared mechanics keep manual byte/status behavior; optional settings are an
/// actual review-policy dependency for scoped callers, never an allow JSON flag.
fn transition_task<E: WorkEnvironment>(
    w: &Work,
    r: &Request,
    c: &TaskLifecycleContext<'_>,
    settings: Option<&crate::EngineSettingsDocument>,
    environment: &mut E,
) -> Result<Task> {
    let task = w
        .tasks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|t| t.id == r.id)
        .ok_or_else(|| fail("task-unknown"))?;
    let (at, actor) = if r.validation_policy.is_some() {
        (
            environment
                .timestamp()
                .map_err(TaskLifecycleError::Runtime)?,
            environment
                .operator()
                .map_err(TaskLifecycleError::Runtime)?,
        )
    } else {
        (String::new(), String::new())
    };
    let definition = DefinitionContext {
        authorized_at: &at,
        actor: &actor,
        task_limits: c.task_limits.clone(),
        policy_limits: c.validation_limits.clone(),
    };
    let mut next =
        update_task_definition(w, task, r, &definition).map_err(TaskLifecycleError::Definition)?;
    if !matches!(
        r.status.as_str(),
        "todo" | "running" | "blocked" | "submitted" | "accepted" | "waived" | "abandoned"
    ) {
        return Err(fail("task-status-unknown"));
    }
    if !transition(&next.status, &r.status) {
        return Err(TaskLifecycleError::Transition);
    }
    if matches!(r.status.as_str(), "running" | "accepted") {
        prerequisites(w, &next, c, settings)?;
        for dep in next.depends.as_deref().unwrap_or_default() {
            check_freshness(w, dep, c, settings)?
        }
    }
    if r.status == "accepted" {
        gate(&next, c)?
    }
    if r.status == "blocked" && !text::nonempty(&r.blocker) {
        return Err(fail("task-blocker-required"));
    }
    if next.status == "running" && r.status != "running" {
        if !matches!(r.outcome.as_str(), "completed" | "interrupted" | "failed") {
            return Err(fail("task-outcome-required"));
        }
        if r.status == "submitted" && r.outcome != "completed" {
            return Err(fail("submission-requires-completed-outcome"));
        }
        if next.attempts.as_ref().is_none_or(Vec::is_empty) {
            return Err(TaskLifecycleError::Attempt);
        }
    }
    if r.status == "running"
        && next.status != "running"
        && next
            .independent_review
            .as_ref()
            .is_some_and(|v| matches!(v.state.as_str(), "running" | "queued"))
    {
        return Err(fail("task-independent-review-active"));
    }
    // Observe runtime only after validation. Every mutation below affects the copy.
    if next.status == "running" && r.status != "running" {
        let ended = environment
            .timestamp()
            .map_err(TaskLifecycleError::Runtime)?;
        if !text::nonempty(&ended) {
            return Err(fail("attempt-end-time-required"));
        }
        let attempt = next
            .attempts
            .as_mut()
            .and_then(|a| a.last_mut())
            .ok_or(TaskLifecycleError::Attempt)?;
        attempt.status = r.outcome.clone();
        attempt.ended = ended;
    }
    if r.status == "running" && next.status != "running" {
        let id = environment
            .identifier("a-")
            .map_err(TaskLifecycleError::Runtime)?;
        let started = environment
            .timestamp()
            .map_err(TaskLifecycleError::Runtime)?;
        if !id.starts_with("a-")
            || !swarm_identity_contract::name(id.as_bytes())
            || !text::nonempty(&started)
        {
            return Err(fail("attempt-identity-and-start-required"));
        }
        if let Some(review) = next.independent_review.take() {
            next.previous_reviews.get_or_insert_default().push(*review)
        }
        next.batch_review_resume = None;
        next.review_coordination = None;
        next.attempts.get_or_insert_default().push(Attempt {
            id,
            status: "recorded".into(),
            started,
            ended: String::new(),
        });
        next.gate = None;
        next.evidence_stale_reason.clear();
        next.automatic_validation = None;
    }
    if r.status == "todo" && next.status != r.status {
        if next.status == "accepted" && check_freshness(w, &next.id, c, settings).is_err() {
            if let Some(gate) = &next.gate {
                next.revalidation = Some(Box::new(Revalidation {
                    previous_artifacts: gate.evaluation.artifacts.clone(),
                    config_digest: gate.evaluation.config_digest.clone(),
                    ..Default::default()
                }))
            }
        }
        next.gate = None;
        next.evidence_stale_reason.clear();
        next.manual_override = None;
    }
    next.status = r.status.clone();
    next.blocker = r.blocker.clone();
    if !r.next.is_empty() {
        next.next = r.next.clone()
    }
    if !r.owner.is_empty() {
        next.owner = r.owner.clone()
    }
    Ok(next)
}
