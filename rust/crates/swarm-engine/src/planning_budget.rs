//! Count-based planning admission only; no financial reservation. Immutable Work is supplied by the transaction
//! owner; these checks grant no identity, lease, execution, acceptance or rights.
use crate::{PlanningScope, PlanningState, PlanningStateError, Task, Work};
use std::{collections::BTreeMap, error::Error, fmt};

/// Budget dimension exhausted by existing consumption; not a new operator policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanningBudgetDimension {
    Activations,
    Decisions,
    Tasks,
}
/// Safe numeric-admission refusal. Exhaustion retains its ancestor/global context
/// as typed data, while Display/Debug never echo supplied private identifiers.
pub enum PlanningBudgetError {
    State(PlanningStateError),
    MissingPlanning,
    UnknownScope,
    CountOverflow,
    Exhausted {
        dimension: PlanningBudgetDimension,
        scope: Option<String>,
    },
}
impl PlanningBudgetError {
    /// Stable failure family; numeric success is not an execution permit.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Exhausted { .. } => "budget_exhausted",
            _ => "planning-budget-invalid",
        }
    }
    /// Static reason only; original state failure remains available as a cause.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::State(e) => e.reason(),
            Self::MissingPlanning => "planning-not-enabled",
            Self::UnknownScope => "planning-scope-not-found",
            Self::CountOverflow => "planning-task-count-overflow",
            Self::Exhausted { dimension, .. } => match dimension {
                PlanningBudgetDimension::Activations => "planning-activation-budget-exhausted",
                PlanningBudgetDimension::Decisions => "planning-decision-budget-exhausted",
                PlanningBudgetDimension::Tasks => "planning-task-budget-exhausted",
            },
        }
    }
}
impl fmt::Display for PlanningBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for PlanningBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for PlanningBudgetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::State(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
type Result<T> = std::result::Result<T, PlanningBudgetError>;
/// Index one validated immutable tree and its tasks; no IO, state mutation or
/// authority. Standard Rust allocations are not an aggregate memory guarantee.
struct Index<'a> {
    planning: &'a PlanningState,
    scopes: &'a [PlanningScope],
    tasks: &'a [Task],
    positions: BTreeMap<&'a str, usize>,
}
impl<'a> Index<'a> {
    /// Reject invalid/absent planning before traversal; retain the state cause.
    fn new(work: &'a Work) -> Result<Self> {
        crate::validate_planning_state(work).map_err(PlanningBudgetError::State)?;
        let planning = work
            .planning
            .as_deref()
            .ok_or(PlanningBudgetError::MissingPlanning)?;
        let scopes = planning.scopes.as_deref().unwrap_or_default();
        Ok(Self {
            planning,
            scopes,
            tasks: work.tasks.as_deref().unwrap_or_default(),
            positions: scopes
                .iter()
                .enumerate()
                .map(|(i, s)| (s.id.as_str(), i))
                .collect(),
        })
    }
    /// Walk selected scope then parents iteratively. Unknown never becomes an
    /// empty chain; tree validity supplies termination without a hidden cutoff.
    fn ancestors(&self, id: &str) -> Result<Vec<usize>> {
        let mut out = Vec::new();
        let mut current = id;
        loop {
            let i = *self
                .positions
                .get(current)
                .ok_or(PlanningBudgetError::UnknownScope)?;
            out.push(i);
            let parent = &self.scopes[i].parent;
            if parent.is_empty() {
                break;
            }
            current = parent;
        }
        Ok(out)
    }
    /// Count every task once in each owner/ancestor, regardless of status. One
    /// shared pass avoids repeating all task walks for every requested ancestor.
    /// Checked overflow refuses; no task, history or reservation is changed.
    fn task_counts(&self) -> Result<Vec<usize>> {
        let mut counts = vec![0usize; self.scopes.len()];
        for task in self.tasks {
            for i in self.ancestors(&task.scope_id)? {
                counts[i] = counts[i]
                    .checked_add(1)
                    .ok_or(PlanningBudgetError::CountOverflow)?;
            }
        }
        Ok(counts)
    }
}
/// Return selected scope and ancestors in Go order after full durable validation.
/// Missing planning, unknown scope, cycles/ownership/counter violations refuse,
/// never a successful empty chain. No IO/mutation or future freshness is implied.
pub fn planning_scope_ancestors<'a>(work: &'a Work, id: &str) -> Result<Vec<&'a PlanningScope>> {
    let index = Index::new(work)?;
    Ok(index
        .ancestors(id)?
        .into_iter()
        .map(|i| &index.scopes[i])
        .collect())
}
/// Count all descendant/self tasks, including accepted/failed history, in the
/// supplied scope. Invalid state/unknown scope/overflow refuse instead of zero.
/// This is a snapshot, not a consumed-budget reset, admission permit or reservation.
pub fn planning_scope_task_count(work: &Work, id: &str) -> Result<usize> {
    let index = Index::new(work)?;
    let chain = index.ancestors(id)?;
    let selected = chain[0];
    Ok(index.task_counts()?[selected])
}
/// Check numeric claim prerequisites from actual global activation/decision counts
/// and every ancestor's activation count. Zero local max_activations inherits the
/// GLOBAL limit; ancestor guards still apply. Equality/excess refuses without
/// mutation. Invalid state/unknown scope preserve safe causes. This does not check
/// paused/failure state, pending events, lease/holder, workflow/capacity/provider,
/// identity/authority or replay. Caller must recheck in its actual transaction.
pub fn check_planning_activation_budget(work: &Work, id: &str) -> Result<()> {
    use PlanningBudgetDimension as D;
    let index = Index::new(work)?;
    let chain = index.ancestors(id)?;
    let p = index.planning;
    for (used, limit, dimension) in [
        (p.activations, p.max_activations, D::Activations),
        (p.decisions, p.max_decisions, D::Decisions),
    ] {
        if used >= limit {
            return Err(PlanningBudgetError::Exhausted {
                dimension,
                scope: None,
            });
        }
    }
    for i in chain {
        let s = &index.scopes[i];
        let limit = if s.max_activations == 0 {
            p.max_activations
        } else {
            s.max_activations
        };
        if s.activations >= limit {
            return Err(PlanningBudgetError::Exhausted {
                dimension: D::Activations,
                scope: Some(s.id.clone()),
            });
        }
    }
    Ok(())
}
/// Check global task capacity then each ancestor's complete descendant task count.
/// Zero local max_tasks inherits GLOBAL max_tasks, not the nearest parent; every
/// ancestor's own guard still applies. Equality/excess/invalid state/unknown scope
/// or checked-count overflow refuses without mutation, reset or empty success.
/// No operation/decision budget, authority, ownership receipt or reservation is
/// created; caller owns freshness, concurrency, transaction and other admission.
pub fn check_planning_task_budget(work: &Work, id: &str) -> Result<()> {
    use PlanningBudgetDimension as D;
    let index = Index::new(work)?;
    let chain = index.ancestors(id)?;
    let p = index.planning;
    if index.tasks.len() as u128 >= p.max_tasks as u128 {
        return Err(PlanningBudgetError::Exhausted {
            dimension: D::Tasks,
            scope: None,
        });
    }
    let counts = index.task_counts()?;
    for i in chain {
        let s = &index.scopes[i];
        let limit = if s.max_tasks == 0 {
            p.max_tasks
        } else {
            s.max_tasks
        };
        if counts[i] as u128 >= limit as u128 {
            return Err(PlanningBudgetError::Exhausted {
                dimension: D::Tasks,
                scope: Some(s.id.clone()),
            });
        }
    }
    Ok(())
}
