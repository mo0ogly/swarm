//! Request validation and pure definition edits. Data transformations grant no
//! authority and perform no SQL, clock, ID generation or Git subprocess.
use crate::{
    validate_requirement_prerequisites, GitState, PlanningStateError, Request, Task, Work,
};
use std::{error::Error, fmt};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinitionRefusal {
    Schema,
    Command,
    RequiredCreationFields,
    RequiredWorkFields,
    TaskFields,
    PrerequisitesBeforeTasks,
    TasksMustBeReopened,
    BlankWorkField,
    EmptyCriteria,
    RequiredTaskFields,
    DuplicateTask,
    UnknownDependency,
    RequiredOodaFields,
}
pub enum WorkDefinitionError {
    RequestFields(crate::RequestFieldsError),
    Refused(DefinitionRefusal),
    Planning(PlanningStateError),
    /// Preserve any typed check-derivation cause without a panic or empty fallback.
    GateDefaults(Box<crate::TaskDefinitionError>),
}
impl WorkDefinitionError {
    /// Return the stable public class, preserving a nested derivation class.
    /// No private input, IO details or mutation is exposed by this lookup.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(DefinitionRefusal::Schema) => "invalid_schema",
            Self::Refused(DefinitionRefusal::Command) => "unknown_command",
            Self::GateDefaults(e) => e.code(),
            Self::RequestFields(e) => e.code(),
            _ => "format",
        }
    }
    /// Return only a static contract reason, retaining nested typed causes for
    /// authorized diagnosis through Error::source. This never echoes input data.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Planning(e) => e.reason(),
            Self::GateDefaults(e) => e.reason(),
            Self::RequestFields(e) => e.reason(),
            Self::Refused(r) => match r {
                DefinitionRefusal::Schema => "request-schema-must-equal-one",
                DefinitionRefusal::Command => "request-command-unknown",
                DefinitionRefusal::RequiredCreationFields => "work-creation-fields-required",
                DefinitionRefusal::RequiredWorkFields => "work-definition-fields-required",
                DefinitionRefusal::TaskFields => "work-definition-task-fields-forbidden",
                DefinitionRefusal::PrerequisitesBeforeTasks => {
                    "work-prerequisites-configure-before-tasks"
                }
                DefinitionRefusal::TasksMustBeReopened => "work-definition-reopen-tasks-first",
                DefinitionRefusal::BlankWorkField => "work-definition-field-blank",
                DefinitionRefusal::EmptyCriteria => "work-criteria-nonblank-required",
                DefinitionRefusal::RequiredTaskFields => "task-definition-fields-required",
                DefinitionRefusal::DuplicateTask => "task-identity-already-present",
                DefinitionRefusal::UnknownDependency => "task-dependency-missing",
                DefinitionRefusal::RequiredOodaFields => "ooda-fields-required",
            },
        }
    }
}
impl fmt::Display for WorkDefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for WorkDefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
/// Retain planning/check-derivation causes without printing private source data.
impl Error for WorkDefinitionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Planning(e) => Some(e),
            Self::GateDefaults(e) => Some(e.as_ref()),
            Self::RequestFields(e) => Some(e),
            _ => None,
        }
    }
}
fn refused(r: DefinitionRefusal) -> WorkDefinitionError {
    WorkDefinitionError::Refused(r)
}
use crate::text::nonempty;
/// Go executeRequest checks these before reservation/replay. No action is applied.
pub fn validate_work_request(kind: &str, r: &Request) -> Result<(), WorkDefinitionError> {
    if r.schema_version != 1 {
        return Err(refused(DefinitionRefusal::Schema));
    }
    if !matches!(
        kind,
        "work.create" | "work.update" | "task.add" | "task.update" | "checkpoint" | "ooda"
    ) {
        return Err(refused(DefinitionRefusal::Command));
    }
    Ok(())
}
/// Internal mutation identity encoding. Unlike the legacy public DTO codec,
/// explicit empty collections remain distinguishable from absence. Schema,
/// command, authority and reservation checks still belong to the caller.
pub fn encode_work_mutation_intent(r: &Request) -> Result<Vec<u8>, crate::Error> {
    let mut raw = crate::encode_request(r)?;
    let markers = [
        (
            r.criteria.as_ref().is_some_and(Vec::is_empty),
            b",\"criteria\":[]".as_slice(),
        ),
        (
            r.memory.as_ref().is_some_and(Vec::is_empty),
            b",\"memory\":[]".as_slice(),
        ),
        (
            r.requirement_prerequisites
                .as_ref()
                .is_some_and(std::collections::BTreeMap::is_empty),
            b",\"requirement_prerequisites\":{}".as_slice(),
        ),
    ];
    if !markers.iter().any(|(include, _)| *include) {
        return Ok(raw);
    }
    if raw.pop() != Some(b'}') {
        return Err(crate::Error::Projection(
            swarm_wire_json::projection::Error::Invalid("mutation request object required"),
        ));
    }
    for (include, bytes) in markers {
        if include {
            raw.extend_from_slice(bytes);
        }
    }
    raw.push(b'}');
    Ok(raw)
}
/// Fill an initial Work supplied by its future Store (ID/created/revision untouched).
/// Title/objective/scope and every criterion must contain non-whitespace text;
/// invalid input or prerequisites retain the original candidate. Git is a supplied
/// observation, not authenticated repository state. No IO, policy fallback or
/// authority is introduced; callers own replay, transaction and persistence.
pub fn apply_work_creation(
    w: &mut Work,
    r: &Request,
    git: GitState,
) -> Result<(), WorkDefinitionError> {
    validate_work_request("work.create", r)?;
    if !nonempty(&r.title)
        || !nonempty(&r.objective)
        || !nonempty(&r.scope)
        || r.criteria.as_ref().is_none_or(Vec::is_empty)
    {
        return Err(refused(DefinitionRefusal::RequiredCreationFields));
    }
    if r.criteria
        .iter()
        .flatten()
        .any(|criterion| !nonempty(criterion))
    {
        return Err(refused(DefinitionRefusal::EmptyCriteria));
    }
    let mut next = w.clone();
    next.title = r.title.clone();
    next.objective = r.objective.clone();
    next.scope = r.scope.clone();
    next.criteria = r.criteria.clone();
    next.requirement_prerequisites = r.requirement_prerequisites.clone();
    validate_requirement_prerequisites(&next).map_err(WorkDefinitionError::Planning)?;
    next.next = r.next.clone();
    next.git = git;
    *w = next;
    Ok(())
}
/// Reframe an unstarted Work. Any refusal retains the exact original Work.
pub fn apply_work_definition(w: &mut Work, r: &Request) -> Result<(), WorkDefinitionError> {
    use DefinitionRefusal as R;
    validate_work_request("work.update", r)?;
    if r.title.is_empty()
        && r.objective.is_empty()
        && r.scope.is_empty()
        && r.criteria.is_none()
        && r.requirement_prerequisites.is_none()
    {
        return Err(refused(R::RequiredWorkFields));
    }
    if !r.status.is_empty()
        || !r.id.is_empty()
        || r.depends.is_some()
        || !r.deliverable.is_empty()
        || r.max_attempts != 0
        || r.max_tool_calls != 0
    {
        return Err(refused(R::TaskFields));
    }
    let tasks = w.tasks.as_deref().unwrap_or_default();
    if (r.requirement_prerequisites.is_some()
        || (r.criteria.is_some()
            && w.requirement_prerequisites
                .as_ref()
                .is_some_and(|p| !p.is_empty())))
        && !tasks.is_empty()
    {
        return Err(refused(R::PrerequisitesBeforeTasks));
    }
    if tasks
        .iter()
        .any(|t| !matches!(t.status.as_str(), "todo" | "blocked"))
    {
        return Err(refused(R::TasksMustBeReopened));
    }
    if [&r.title, &r.objective, &r.scope]
        .iter()
        .any(|s| !s.is_empty() && !nonempty(s))
    {
        return Err(refused(R::BlankWorkField));
    }
    if r.criteria
        .as_ref()
        .is_some_and(|v| v.is_empty() || v.iter().any(|s| !nonempty(s)))
    {
        return Err(refused(R::EmptyCriteria));
    }
    let mut next = w.clone();
    if !r.title.is_empty() {
        next.title = r.title.clone();
    }
    if !r.objective.is_empty() {
        next.objective = r.objective.clone();
    }
    if !r.scope.is_empty() {
        next.scope = r.scope.clone();
    }
    if r.criteria.is_some() {
        next.criteria = r.criteria.clone();
    }
    if r.requirement_prerequisites.is_some() {
        next.requirement_prerequisites = r.requirement_prerequisites.clone();
    }
    validate_requirement_prerequisites(&next).map_err(WorkDefinitionError::Planning)?;
    if let Some(tasks) = next.tasks.as_mut() {
        for t in tasks {
            t.gate = None;
            t.manual_override = None;
            t.revalidation = None;
        }
    }
    *w = next;
    Ok(())
}
/// Append a todo task with an explicitly empty attempts list, matching Go.
/// Prepare a new manual task with nonblank declared criteria and their existing
/// mandatory gate identifiers. References must name existing tasks; invalid input
/// leaves Work unchanged. This pure edit performs no IO, enforces no supplied
/// runtime count policy and grants no execution/review/acceptance authority.
/// This compatibility API uses the historical Go phase profile only; native Root
/// calls the explicit-policy constructor and never uses this as a fallback.
pub fn apply_task_addition(w: &mut Work, r: &Request) -> Result<(), WorkDefinitionError> {
    apply_task_addition_with_defaults(w, r, &crate::ManualGatePhaseDefaults::legacy_go())
}
/// Prepare a new task using explicitly observed manual defaults. Caller validates
/// runtime counts and reserves/persists the result; this pure constructor returns
/// a validation/encoding error without mutation, execution or actor authorization.
pub(crate) fn apply_task_addition_with_defaults(
    w: &mut Work,
    r: &Request,
    defaults: &crate::ManualGatePhaseDefaults,
) -> Result<(), WorkDefinitionError> {
    use DefinitionRefusal as R;
    validate_work_request("task.add", r)?;
    if !swarm_identity_contract::name(r.id.as_bytes())
        || !nonempty(&r.title)
        || !nonempty(&r.deliverable)
        || r.criteria
            .as_ref()
            .is_none_or(|c| c.is_empty() || c.iter().any(|v| !nonempty(v)))
    {
        return Err(refused(R::RequiredTaskFields));
    }
    let tasks = w.tasks.as_deref().unwrap_or_default();
    if tasks.iter().any(|t| t.id == r.id) {
        return Err(refused(R::DuplicateTask));
    }
    for dep in r.depends.as_deref().unwrap_or_default() {
        if !tasks.iter().any(|t| t.id == *dep) {
            return Err(refused(R::UnknownDependency));
        }
    }
    w.tasks.get_or_insert_default().push(Task {
        id: r.id.clone(),
        title: r.title.clone(),
        deliverable: r.deliverable.clone(),
        criteria: r.criteria.clone(),
        plan_checks: Some(
            crate::task_definition::task_plan_checks(
                None,
                r.criteria.as_deref().unwrap_or_default(),
                Some(defaults),
            )
            .map_err(|e| WorkDefinitionError::GateDefaults(Box::new(e)))?,
        ),
        owner: r.owner.clone(),
        depends: r.depends.clone(),
        status: "todo".into(),
        next: r.next.clone(),
        attempts: Some(Vec::new()),
        ..Default::default()
    });
    Ok(())
}
/// Record an OODA decision's next step and optional result, without execution.
pub fn apply_ooda(w: &mut Work, r: &Request) -> Result<(), WorkDefinitionError> {
    validate_work_request("ooda", r)?;
    if [
        &r.observation,
        &r.orientation,
        &r.decision,
        &r.owner,
        &r.next,
    ]
    .iter()
    .any(|s| !nonempty(s))
    {
        return Err(refused(DefinitionRefusal::RequiredOodaFields));
    }
    w.next = r.next.clone();
    if !r.result.is_empty() {
        w.summary = r.result.clone();
    }
    Ok(())
}
