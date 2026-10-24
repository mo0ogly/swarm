//! Pure task contracts and metadata. Supplied stamps/limits grant no authority.
//! Running/acceptance transitions require actual Store evidence and are separate.
use crate::{
    mutation_guard::edits_definition, normalize_validation_policy, task_review_contract,
    text::nonempty, update_task_instructions, validate_work_request, validation_policy_covers_task,
    InstructionError, Request, Task, ValidationPolicyError, ValidationPolicyLimits, Work,
    WorkDefinitionError, WorkError,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
#[derive(Clone, Debug)]
pub struct TaskDefinitionLimits {
    pub max_attempts: i64,
    pub max_tool_calls: i64,
    pub max_criteria: usize,
    /// Explicit manual creation defaults; None is legacy absence, not a fallback.
    pub manual_gate_phases: Option<crate::ManualGatePhaseDefaults>,
}
impl TaskDefinitionLimits {
    /// Legacy data fixture, never a hidden runtime fallback.
    pub fn legacy_go() -> Self {
        Self {
            max_attempts: 3,
            max_tool_calls: 100,
            max_criteria: 12,
            manual_gate_phases: Some(crate::ManualGatePhaseDefaults::legacy_go()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct DefinitionContext<'a> {
    pub authorized_at: &'a str,
    pub actor: &'a str,
    pub task_limits: TaskDefinitionLimits,
    pub policy_limits: ValidationPolicyLimits,
}
pub enum TaskDefinitionError {
    Refused(&'static str),
    Policy(ValidationPolicyError),
    Instruction(InstructionError),
    Request(WorkDefinitionError),
    Contract(WorkError),
}
impl TaskDefinitionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request(e) => e.code(),
            _ => "format",
        }
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(r) => r,
            Self::Policy(e) => e.reason(),
            Self::Instruction(e) => e.reason(),
            Self::Request(e) => e.reason(),
            Self::Contract(_) => "task-contract-encoding-failed",
        }
    }
}
impl fmt::Display for TaskDefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for TaskDefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for TaskDefinitionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Refused(_) => None,
            Self::Policy(e) => Some(e),
            Self::Instruction(e) => Some(e),
            Self::Request(e) => Some(e),
            Self::Contract(e) => Some(e),
        }
    }
}
fn fail(reason: &'static str) -> TaskDefinitionError {
    TaskDefinitionError::Refused(reason)
}
pub fn validate_task_dependency_graph(
    w: &Work,
    replacement: &Task,
) -> Result<(), TaskDefinitionError> {
    let tasks = w.tasks.as_deref().unwrap_or_default();
    let mut first = BTreeMap::new();
    for t in tasks {
        first.entry(t.id.as_str()).or_insert(t);
    }
    let mut state = BTreeMap::<&str, u8>::new();
    for root in tasks {
        let mut pending = vec![(root.id.as_str(), false)];
        while let Some((id, exit)) = pending.pop() {
            if exit {
                state.insert(id, 2);
                continue;
            }
            match state.get(id) {
                Some(1) => return Err(fail("task-dependency-cycle")),
                Some(2) => continue,
                _ => {}
            }
            let mut node = *first
                .get(id)
                .ok_or_else(|| fail("task-dependency-missing"))?;
            if id == replacement.id {
                node = replacement;
            }
            state.insert(id, 1);
            pending.push((id, true));
            let mut seen = BTreeSet::new();
            for dep in node.depends.iter().flatten() {
                if !seen.insert(dep.as_str()) {
                    return Err(fail("task-dependency-repeated"));
                }
            }
            for dep in node.depends.as_deref().unwrap_or_default().iter().rev() {
                pending.push((dep.as_str(), false));
            }
        }
    }
    Ok(())
}
/// Derive checks for validated criteria. Existing non-criterion requirements
/// survive unchanged; numbered criterion IDs are rebuilt. Only a missing mapping
/// adopts supplied creation defaults, refusing absent configuration. Returns a
/// new map or a static configuration error; no IO, mutation or authority follows.
pub(crate) fn task_plan_checks(
    previous: Option<&BTreeMap<String, String>>,
    criteria: &[String],
    defaults: Option<&crate::ManualGatePhaseDefaults>,
) -> Result<BTreeMap<String, String>, TaskDefinitionError> {
    let mut checks = BTreeMap::new();
    if previous.is_none() {
        let defaults = defaults.ok_or_else(|| fail("task-gate-defaults-not-configured"))?;
        for phase in defaults.iter() {
            checks.insert(format!("plan-{phase}"), phase.clone());
        }
    }
    for (id, phase) in previous.into_iter().flatten() {
        if !id.starts_with("plan-criterion-") {
            checks.insert(id.clone(), phase.clone());
        }
    }
    for i in 0..criteria.len() {
        checks.insert(format!("plan-criterion-{}", i + 1), "validation".into());
    }
    Ok(checks)
}
/// Bind automatic gate requirements to the observed authorized control IDs.
/// Replaces only synthetic manual-template IDs/phases; custom requirements remain
/// subject to the unchanged policy-coverage guard. A first manual mapping with
/// no saved defaults returns a static configuration error; existing mappings need
/// no new default adoption. Returns a new map without IO,
/// command authorization, mutation of supplied data or any execution claim.
pub(crate) fn task_policy_checks(
    previous: Option<&BTreeMap<String, String>>,
    criteria: &[String],
    policy: Option<&crate::ValidationPolicy>,
    defaults: Option<&crate::ManualGatePhaseDefaults>,
) -> Result<BTreeMap<String, String>, TaskDefinitionError> {
    let Some(policy) = policy.filter(|p| p.mode == "automatic") else {
        return task_plan_checks(previous, criteria, defaults);
    };
    let mut checks = BTreeMap::new();
    for (id, phase) in previous.into_iter().flatten() {
        let synthetic = matches!(
            (id.as_str(), phase.as_str()),
            ("plan-entry", "entry")
                | ("plan-validation", "validation")
                | ("plan-delivery", "delivery")
        ) || id.starts_with("plan-criterion-") && phase == "validation";
        if !synthetic {
            checks.insert(id.clone(), phase.clone());
        }
    }
    for control in policy.controls.iter().flatten() {
        let phase = previous
            .and_then(|p| p.get(&control.id))
            .filter(|p| !p.is_empty())
            .cloned()
            .unwrap_or_else(|| "delivery".into());
        checks.insert(control.id.clone(), phase);
    }
    Ok(checks)
}
/// Validate and prepare a definition edit on an open manual task using the
/// supplied observed limits/stamps. Invalid data leaves inputs unchanged; changed
/// contracts archive prior review and clear stale gate/control/waiver evidence.
/// Returns a task copy or a typed validation/encoding error. Supplied actor/time
/// are data, never authentication; Root owns reservations and persistence.
pub fn update_task_definition(
    w: &Work,
    t: &Task,
    r: &Request,
    c: &DefinitionContext<'_>,
) -> Result<Task, TaskDefinitionError> {
    if !edits_definition(r) {
        return Ok(t.clone());
    }
    if t.independent_review
        .as_ref()
        .is_some_and(|v| v.state == "running")
    {
        return Err(fail("task-review-running"));
    }
    if !matches!(t.status.as_str(), "todo" | "blocked")
        || (!r.status.is_empty() && r.status != t.status)
    {
        return Err(fail("task-definition-reopen-without-transition"));
    }
    if w.tasks.iter().flatten().any(|t| t.status == "running") {
        return Err(fail("task-definition-active-task"));
    }
    let mut next = t.clone();
    if let Some(p) = &r.validation_policy {
        let mut p = normalize_validation_policy(p.clone(), &c.policy_limits)
            .map_err(TaskDefinitionError::Policy)?;
        p.authorized_at = c.authorized_at.into();
        p.actor = c.actor.into();
        next.validation_policy = Some(Box::new(p));
    }
    if r.max_attempts != 0 {
        if r.max_attempts < 1 || r.max_attempts > c.task_limits.max_attempts {
            return Err(fail("task-max-attempts-invalid"));
        }
        next.plan_max_attempts = r.max_attempts;
    }
    if r.max_tool_calls != 0 {
        if r.max_tool_calls < 1 || r.max_tool_calls > c.task_limits.max_tool_calls {
            return Err(fail("task-max-tool-calls-invalid"));
        }
        next.plan_tool_limit = r.max_tool_calls;
    }
    if !r.title.is_empty() {
        if !nonempty(&r.title) {
            return Err(fail("task-title-blank"));
        }
        next.title = r.title.clone();
    }
    if !r.deliverable.is_empty() {
        if !nonempty(&r.deliverable) {
            return Err(fail("task-deliverable-blank"));
        }
        next.deliverable = r.deliverable.clone();
    }
    if let Some(criteria) = &r.criteria {
        if criteria.is_empty()
            || criteria.len() > c.task_limits.max_criteria
            || criteria.iter().any(|s| !nonempty(s))
        {
            return Err(fail("task-criteria-invalid"));
        }
        next.criteria = Some(criteria.clone());
    }
    if let Some(deps) = &r.depends {
        next.depends = Some(deps.clone());
    }
    if next.plan_checks.is_some() || r.criteria.is_some() || r.validation_policy.is_some() {
        next.plan_checks = Some(task_policy_checks(
            next.plan_checks.as_ref(),
            next.criteria.as_deref().unwrap_or_default(),
            next.validation_policy.as_deref(),
            c.task_limits.manual_gate_phases.as_ref(),
        )?);
    }
    if let Some(p) = &next.validation_policy {
        validation_policy_covers_task(p, &next).map_err(TaskDefinitionError::Policy)?;
    }
    validate_task_dependency_graph(w, &next)?;
    if next.title == t.title
        && next.deliverable == t.deliverable
        && next.criteria == t.criteria
        && next.depends == t.depends
        && next.plan_max_attempts == t.plan_max_attempts
        && next.plan_tool_limit == t.plan_tool_limit
        && next.validation_policy == t.validation_policy
        && next.plan_checks == t.plan_checks
    {
        return Ok(t.clone());
    }
    if task_review_contract(&next).map_err(TaskDefinitionError::Contract)?
        != task_review_contract(t).map_err(TaskDefinitionError::Contract)?
    {
        if let Some(review) = next.independent_review.take() {
            next.previous_reviews.get_or_insert_default().push(*review);
        }
    }
    next.gate = None;
    next.automatic_validation = None;
    next.manual_override = None;
    next.revalidation = None;

    Ok(next)
}
/// Metadata-only task.update. Full status transitions remain a Store operation.
pub fn apply_task_metadata(
    w: &mut Work,
    r: &Request,
    c: &DefinitionContext<'_>,
) -> Result<(), TaskDefinitionError> {
    validate_work_request("task.update", r).map_err(TaskDefinitionError::Request)?;
    if !r.status.is_empty() {
        return Err(fail("task-metadata-status-transition-forbidden"));
    }
    let tasks = w.tasks.as_deref().unwrap_or_default();
    let i = tasks
        .iter()
        .position(|t| t.id == r.id)
        .ok_or_else(|| fail("task-unknown"))?;
    let mut next = update_task_definition(w, &tasks[i], r, c)?;
    if !r.owner.is_empty() {
        next.owner = r.owner.clone();
    }
    if !r.next.is_empty() {
        update_task_instructions(&mut next, &r.next).map_err(TaskDefinitionError::Instruction)?;
    }
    if !r.blocker.is_empty() {
        next.blocker = r.blocker.clone();
    }
    w.tasks.as_mut().ok_or_else(|| fail("task-unknown"))?[i] = next;
    Ok(())
}
