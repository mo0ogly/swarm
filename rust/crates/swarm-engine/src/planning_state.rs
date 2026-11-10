//! Durable planning data validation, not proof freshness, authority or acceptance.
use crate::{encode_model, JudgeState, PlanningEvent, Work, WorkError};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanningRefusal {
    PrerequisiteUnknown,
    PrerequisiteCycle,
    PrerequisiteDuplicate,
    Policy,
    ReviewerContext,
    Judge,
    Scope,
    ScopeBudget,
    RequirementOwner,
    Root,
    Parent,
    Cycle,
    Depth,
    RequirementCoverage,
    TaskScope,
    TaskRole,
    TaskRequirements,
    TaskOwnership,
    Event,
}
pub enum PlanningStateError {
    Refused(PlanningRefusal),
    Encode(WorkError),
}
impl PlanningStateError {
    pub fn code(&self) -> &'static str {
        "format"
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Encode(_) => "planning-event-encoding",
            Self::Refused(reason) => match reason {
                PlanningRefusal::PrerequisiteUnknown => "planning-prerequisite-unknown",
                PlanningRefusal::PrerequisiteCycle => "planning-prerequisite-cycle",
                PlanningRefusal::PrerequisiteDuplicate => "planning-prerequisite-duplicate",
                PlanningRefusal::Policy => "planning-policy-invalid",
                PlanningRefusal::ReviewerContext => "planning-reviewer-context-invalid",
                PlanningRefusal::Judge => "planning-judge-history-invalid",
                PlanningRefusal::ScopeBudget => "planning-scope-budget-invalid",
                PlanningRefusal::Scope => "planning-scope-invalid",
                PlanningRefusal::RequirementOwner => "planning-requirement-multiple-owners",
                PlanningRefusal::Root => "planning-root-invalid",
                PlanningRefusal::Parent => "planning-parent-missing",
                PlanningRefusal::Cycle => "planning-ownership-cycle",
                PlanningRefusal::Depth => "planning-configured-depth-exceeded",
                PlanningRefusal::RequirementCoverage => "planning-requirement-coverage-invalid",
                PlanningRefusal::TaskScope => "planning-task-scope-missing",
                PlanningRefusal::TaskRole => "planning-task-role-invalid",
                PlanningRefusal::TaskRequirements => "planning-task-requirements-missing",
                PlanningRefusal::TaskOwnership => "planning-task-outside-ownership",
                PlanningRefusal::Event => "planning-event-invalid",
            },
        }
    }
}
impl fmt::Display for PlanningStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for PlanningStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PlanningStateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(e) => Some(e),
            _ => None,
        }
    }
}
fn refuse(reason: PlanningRefusal) -> PlanningStateError {
    PlanningStateError::Refused(reason)
}
use crate::text::nonempty;
fn exceeds(n: usize, limit: i64) -> bool {
    (n as u128) > (limit as u128)
}

/// Validate only configured prerequisite structure, without inferring accepted evidence.
pub fn validate_requirement_prerequisites(w: &Work) -> Result<(), PlanningStateError> {
    use PlanningRefusal as R;
    let Some(prerequisites) = w
        .requirement_prerequisites
        .as_ref()
        .filter(|p| !p.is_empty())
    else {
        return Ok(());
    };
    let known: BTreeSet<String> = (1..=w.criteria.as_ref().map_or(0, Vec::len))
        .map(|n| format!("req-{n}"))
        .collect();
    for task in w.tasks.as_deref().unwrap_or_default() {
        for id in task.requirements.as_deref().unwrap_or_default() {
            if !known.contains(id) {
                return Err(refuse(R::PrerequisiteUnknown));
            }
        }
    }
    let mut state: BTreeMap<&str, u8> = BTreeMap::new();
    // Explicit DFS stack; an input graph does not consume the native call stack.
    for root in prerequisites.keys() {
        let mut stack = vec![(root.as_str(), false)];
        while let Some((id, exit)) = stack.pop() {
            if exit {
                state.insert(id, 2);
                continue;
            }
            if !known.contains(id) {
                return Err(refuse(R::PrerequisiteUnknown));
            }
            match state.get(id) {
                Some(1) => return Err(refuse(R::PrerequisiteCycle)),
                Some(2) => continue,
                _ => {}
            }
            let deps = prerequisites
                .get(id)
                .and_then(Option::as_deref)
                .unwrap_or_default();
            let mut seen = BTreeSet::new();
            for dep in deps {
                if !seen.insert(dep) {
                    return Err(refuse(R::PrerequisiteDuplicate));
                }
            }
            state.insert(id, 1);
            stack.push((id, true));
            for dep in deps.iter().rev() {
                stack.push((dep, false));
            }
        }
    }
    Ok(())
}
/// Durable judge consistency only: no verdict is accepted as fresh or authorized.
/// Historical reason sizes are not rejudged against admission policy. New judge
/// decisions use the persisted decision reason cap; transport/context byte caps
/// remain separate caller boundaries. No IO, mutation or acceptance occurs.
pub fn validate_judge_state(j: Option<&JudgeState>) -> Result<(), PlanningStateError> {
    let Some(j) = j else { return Ok(()) };
    let invalid = || refuse(PlanningRefusal::Judge);
    if j.cycle < 1
        || j.max_cycles < 0
        || (j.max_cycles > 0 && j.cycle > j.max_cycles)
        || !matches!(j.verdict.as_str(), "" | "continue" | "end")
        || j.closed != (j.verdict == "end")
    {
        return Err(invalid());
    }
    let count = if j.verdict.is_empty() {
        j.cycle - 1
    } else {
        j.cycle
    };
    let history = j.history.as_deref().unwrap_or_default();
    if (history.len() as u128) != (count as u128)
        || (j.verdict.is_empty()
            && (!j.reason.is_empty() || !j.at.is_empty() || !j.evidence_ref.is_empty()))
    {
        return Err(invalid());
    }
    for (i, h) in history.iter().enumerate() {
        if (h.cycle as i128) != (i as i128) + 1
            || !nonempty(&h.reason)
            || swarm_wire_json::go_rfc3339::parse(&h.at).is_err()
        {
            return Err(invalid());
        }
        if h.cycle < j.cycle {
            if h.verdict != "continue" || swarm_wire_json::go_rfc3339::parse(&h.renewed_at).is_err()
            {
                return Err(invalid());
            }
        } else if h.verdict != j.verdict
            || h.reason != j.reason
            || h.at != j.at
            || h.evidence_ref != j.evidence_ref
            || !h.renewed_at.is_empty()
        {
            return Err(invalid());
        }
        if h.verdict == "end" && !nonempty(&h.evidence_ref) {
            return Err(invalid());
        }
    }
    Ok(())
}
/// Validate durable planning prerequisites, ownership tree and scalar contracts.
/// Scope consumption/limits must be nonnegative; zero local limits inherit global
/// policy. Positive consumed history above a tightened local cap stays readable:
/// admission must separately deny new use. Typed errors never repair/reset Work.
/// No IO, mutation, authenticated identity, authority or acceptance is established.
pub fn validate_planning_state(w: &Work) -> Result<(), PlanningStateError> {
    use PlanningRefusal as R;
    validate_requirement_prerequisites(w)?;
    let Some(p) = w.planning.as_ref() else {
        return Ok(());
    };
    let scopes = p.scopes.as_deref().unwrap_or_default();
    let tasks = w.tasks.as_deref().unwrap_or_default();
    if p.version != 1
        || scopes.is_empty()
        || p.max_scopes < 0
        || (p.max_scopes > 0 && exceeds(scopes.len(), p.max_scopes))
        || p.max_depth < 0
        || p.max_tasks < 1
        || exceeds(tasks.len(), p.max_tasks)
        || p.max_decisions < 1
        || p.max_activations < 1
        || p.activations < 0
        || p.activations > p.max_activations
        || p.decisions < 0
        || p.decisions > p.max_decisions
    {
        return Err(refuse(R::Policy));
    }
    if p.reviewer
        .as_ref()
        .is_some_and(|r| r.input_context_tokens < 0)
    {
        return Err(refuse(R::ReviewerContext));
    }
    validate_judge_state(p.judge.as_deref())?;
    let mut ids = BTreeMap::new();
    let mut requirements = BTreeSet::new();
    for (i, scope) in scopes.iter().enumerate() {
        if !swarm_identity_contract::name(scope.id.as_bytes())
            || ids.insert(scope.id.as_str(), i).is_some()
            || scope.revision < 1
        {
            return Err(refuse(R::Scope));
        }
        if scope.activations < 0 || scope.max_tasks < 0 || scope.max_activations < 0 {
            return Err(refuse(R::ScopeBudget));
        }
        for req in scope.requirements.as_deref().unwrap_or_default() {
            if !requirements.insert(req.as_str()) {
                return Err(refuse(R::RequirementOwner));
            }
        }
    }
    if ids
        .get("root")
        .is_none_or(|i| !scopes[*i].parent.is_empty())
    {
        return Err(refuse(R::Root));
    }
    let mut depths = BTreeMap::from([("root", 1usize)]);
    for scope in scopes {
        let mut seen = BTreeSet::new();
        let mut path = Vec::new();
        let mut node = scope;
        while !depths.contains_key(node.id.as_str()) {
            if !seen.insert(node.id.as_str()) {
                return Err(refuse(R::Cycle));
            }
            path.push(node.id.as_str());
            node = &scopes[*ids
                .get(node.parent.as_str())
                .ok_or_else(|| refuse(R::Parent))?];
        }
        let mut depth = depths[node.id.as_str()];
        for id in path.into_iter().rev() {
            depth = depth.checked_add(1).ok_or_else(|| refuse(R::Policy))?;
            depths.insert(id, depth);
        }
        if p.max_depth > 0 && exceeds(depths[scope.id.as_str()], p.max_depth) {
            return Err(refuse(R::Depth));
        }
    }
    let count = w.criteria.as_ref().map_or(0, Vec::len);
    if requirements.len() != count
        || (1..=count).any(|i| !requirements.contains(format!("req-{i}").as_str()))
    {
        return Err(refuse(R::RequirementCoverage));
    }
    for task in tasks {
        let scope = &scopes[*ids
            .get(task.scope_id.as_str())
            .ok_or_else(|| refuse(R::TaskScope))?];
        if !matches!(task.plan_role.as_str(), "" | "worker") {
            return Err(refuse(R::TaskRole));
        }
        let required = task.requirements.as_deref().unwrap_or_default();
        if required.is_empty() {
            return Err(refuse(R::TaskRequirements));
        }
        for req in required {
            if !scope
                .requirements
                .as_deref()
                .unwrap_or_default()
                .contains(req)
            {
                return Err(refuse(R::TaskOwnership));
            }
        }
    }
    let mut events = BTreeSet::new();
    for e in p.inbox.as_deref().unwrap_or_default() {
        if !swarm_identity_contract::name(e.id.as_bytes())
            || !events.insert(e.id.as_str())
            || !ids.contains_key(e.scope.as_str())
        {
            return Err(refuse(R::Event));
        }
    }
    Ok(())
}
/// Generate Go-compatible signal IDs. This is a deterministic data hash, not authority.
pub fn planning_validation_event_id(event: &str, task: &str) -> Result<String, PlanningStateError> {
    let bytes = encode_model(&[event, "validation", task]).map_err(PlanningStateError::Encode)?;
    Ok(format!("plan-{:x}", Sha256::digest(bytes)))
}
/// Validate before walking parents, stage signals, then validate the resulting plan.
/// Any failure leaves Work unchanged. Caller supplies time and ends its SQL transaction.
pub fn apply_planning_validation_signals(
    w: &mut Work,
    before: &BTreeMap<String, String>,
    event: &str,
    at: &str,
) -> Result<(), PlanningStateError> {
    validate_planning_state(w)?;
    let Some(p) = w.planning.as_ref() else {
        return Ok(());
    };
    let mut next = (**p).clone();
    let indexes: BTreeMap<String, usize> = next
        .scopes
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), i))
        .collect();
    for task in w.tasks.as_deref().unwrap_or_default() {
        let prior = before.get(&task.id).map(String::as_str).unwrap_or("");
        if task.scope_id.is_empty()
            || prior == task.status
            || (task.status != "accepted" && prior != "accepted")
        {
            continue;
        }
        // The pre-validation established every task owner and acyclic ancestry.
        let mut i = *indexes
            .get(&task.scope_id)
            .ok_or_else(|| refuse(PlanningRefusal::TaskScope))?;
        let attempt = task
            .attempts
            .as_deref()
            .and_then(|a| a.last())
            .map(|a| a.id.clone())
            .unwrap_or_default();
        next.inbox.get_or_insert_default().push(PlanningEvent {
            id: planning_validation_event_id(event, &task.id)?,
            scope: task.scope_id.clone(),
            kind: "validation_changed".into(),
            task: task.id.clone(),
            attempt,
            message: task.status.clone(),
            at: at.into(),
            ..Default::default()
        });
        loop {
            let scope = &mut next
                .scopes
                .as_mut()
                .ok_or_else(|| refuse(PlanningRefusal::Scope))?[i];
            scope.state = "ready".into();
            if scope.parent.is_empty() {
                break;
            }
            i = *indexes
                .get(&scope.parent)
                .ok_or_else(|| refuse(PlanningRefusal::Parent))?;
        }
    }
    let old = w.planning.replace(Box::new(next));
    if let Err(e) = validate_planning_state(w) {
        w.planning = old;
        return Err(e);
    }
    Ok(())
}
