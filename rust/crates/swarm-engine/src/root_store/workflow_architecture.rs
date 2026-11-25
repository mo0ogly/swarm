//! Deterministic structural advice and revision-bound operator receipts.
use super::{finished, material_path, native_transaction, RootStore, RootStoreError as E};
use crate::{SystemEnvironment, Work, WorkEnvironment};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const TABLE: &str = "workflow_architecture_approvals";
const SCHEMA: &str = "CREATE TABLE workflow_architecture_approvals(work_id TEXT NOT NULL REFERENCES works(id), revision INTEGER NOT NULL, proposal TEXT NOT NULL, recommendation TEXT NOT NULL, approved_at TEXT NOT NULL, PRIMARY KEY(work_id, revision))";

#[derive(Serialize)]
struct Assessment {
    recommendation: &'static str,
    tasks: usize,
    dependencies: usize,
    delegated_scopes: usize,
    competing_choices: bool,
    separate_ready_scopes: bool,
    cross_scope_dependencies: bool,
}

#[derive(Serialize)]
struct Approval {
    revision: String,
    proposal: String,
    recommendation: String,
    approved_at: String,
}

/// Classify precedence conservatively. A unique complete topological order
/// proves a serial chain. Hierarchy requires a ready choice between separate
/// delegated scopes and an actual cross-scope prerequisite.
fn assess(work: &Work) -> Result<Assessment, E> {
    let tasks = work.tasks.as_deref().unwrap_or_default();
    let delegated: BTreeSet<_> = work
        .planning
        .iter()
        .flat_map(|planning| planning.scopes.iter().flatten())
        .filter(|scope| !scope.parent.is_empty())
        .map(|scope| scope.id.as_str())
        .collect();
    let mut pending: BTreeMap<_, _> = tasks
        .iter()
        .map(|task| (task.id.as_str(), (0_usize, task.scope_id.as_str())))
        .collect();
    if pending.len() != tasks.len() {
        return Err(E::Refused("workflow-architecture-graph-invalid"));
    }
    let mut edges = BTreeSet::new();
    let mut producers = BTreeMap::<&str, Vec<&str>>::new();
    for task in tasks {
        for requirement in task.requirements.iter().flatten() {
            producers.entry(requirement).or_default().push(&task.id);
        }
        for prerequisite in task.depends.iter().flatten() {
            if !pending.contains_key(prerequisite.as_str()) {
                return Err(E::Refused("workflow-architecture-graph-invalid"));
            }
            edges.insert((prerequisite.as_str(), task.id.as_str()));
        }
    }
    let mut unresolved = false;
    if let Some(requirements) = work
        .requirement_prerequisites
        .as_ref()
        .filter(|map| !map.is_empty())
    {
        for task in tasks {
            let owned = task.requirements.as_deref().unwrap_or_default();
            unresolved |= owned.is_empty();
            for requirement in owned {
                for prerequisite in requirements
                    .get(requirement)
                    .into_iter()
                    .flatten()
                    .flatten()
                {
                    let inputs: Vec<_> = producers
                        .get(prerequisite.as_str())
                        .into_iter()
                        .flatten()
                        .copied()
                        .filter(|producer| *producer != task.id)
                        .collect();
                    if let [producer] = inputs.as_slice() {
                        edges.insert((*producer, &task.id));
                    } else {
                        unresolved = true;
                    }
                }
            }
        }
    }
    let mut followers = BTreeMap::<&str, Vec<&str>>::new();
    let mut cross_scope_dependencies = false;
    for (from, to) in &edges {
        let previous = pending[from].1;
        let (count, scope) = pending
            .get_mut(to)
            .ok_or(E::Refused("workflow-architecture-graph-invalid"))?;
        *count += 1;
        cross_scope_dependencies |=
            delegated.contains(previous) && delegated.contains(*scope) && previous != *scope;
        followers.entry(from).or_default().push(to);
    }
    let mut ready: VecDeque<_> = pending
        .iter()
        .filter(|(_, (count, _))| *count == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut ready_scopes = BTreeMap::<&str, usize>::new();
    for id in &ready {
        let scope = pending[id].1;
        if delegated.contains(scope) {
            *ready_scopes.entry(scope).or_default() += 1;
        }
    }
    let mut competing_choices = false;
    let mut separate_ready_scopes = false;
    let mut seen = 0;
    while let Some(id) = ready.pop_front() {
        competing_choices |= !ready.is_empty();
        separate_ready_scopes |= ready_scopes.len() > 1;
        let scope = pending[id].1;
        if let Some(count) = ready_scopes.get_mut(scope) {
            *count -= 1;
            if *count == 0 {
                ready_scopes.remove(scope);
            }
        }
        seen += 1;
        for child in followers.get(id).into_iter().flatten() {
            let (count, scope) = pending
                .get_mut(child)
                .ok_or(E::Refused("workflow-architecture-graph-invalid"))?;
            *count -= 1;
            if *count == 0 {
                ready.push_back(child);
                if delegated.contains(*scope) {
                    *ready_scopes.entry(*scope).or_default() += 1;
                }
            }
        }
    }
    if seen != tasks.len() {
        return Err(E::Refused("workflow-architecture-graph-invalid"));
    }
    let recommendation = if unresolved {
        "insufficient"
    } else if tasks.len() > 1 && !competing_choices {
        "sequential"
    } else if separate_ready_scopes && cross_scope_dependencies {
        "hierarchical"
    } else {
        "insufficient"
    };
    Ok(Assessment {
        recommendation,
        dependencies: edges.len(),
        tasks: tasks.len(),
        delegated_scopes: delegated.len(),
        competing_choices,
        separate_ready_scopes,
        cross_scope_dependencies,
    })
}

/// Verify the precise optional schema. An absent table means no receipts yet;
/// a substituted or unsupported schema is an error.
fn has_approvals(db: &Connection) -> Result<bool, E> {
    let sql: Option<Option<String>> = db
        .query_row(
            "SELECT sql FROM main.sqlite_schema WHERE name=?1 COLLATE NOCASE",
            [TABLE],
            |r| r.get(0),
        )
        .optional()
        .map_err(E::Sql)?;
    match sql {
        None => Ok(false),
        Some(Some(found)) if found == SCHEMA => Ok(true),
        _ => Err(E::Refused("workflow-architecture-approval-schema")),
    }
}

/// Read the last receipt for this Work, preserving typed database errors.
fn last_approval(db: &Connection, work: &str) -> Result<Option<Approval>, E> {
    db.query_row(
        "SELECT revision, proposal, recommendation, approved_at FROM main.workflow_architecture_approvals WHERE work_id=?1 ORDER BY revision DESC LIMIT 1",
        [work], |r| Ok(Approval {
            revision: r.get::<_, i64>(0)?.to_string(),
            proposal: r.get(1)?,
            recommendation: r.get(2)?,
            approved_at: r.get(3)?,
        })
    ).optional().map_err(E::Sql)
}

/// Build advice from a single recorded Work and SQL snapshot. The proposal
/// digest is a stale-reference guard, not a signature or an execution grant.
fn observation(db: &Connection, work: &Work) -> Result<(Assessment, String, Option<Approval>), E> {
    let assessment = assess(work)?;
    let topology: BTreeMap<_, _> = work
        .tasks
        .iter()
        .flatten()
        .map(|task| {
            (
                &task.id,
                (&task.depends, &task.scope_id, &task.requirements),
            )
        })
        .collect();
    let scopes: BTreeMap<_, _> = work
        .planning
        .iter()
        .flat_map(|p| p.scopes.iter().flatten())
        .map(|scope| (&scope.id, &scope.parent))
        .collect();
    let encoded = crate::encode_model(&(
        1,
        &work.id,
        work.revision,
        topology,
        scopes,
        &work.requirement_prerequisites,
        &assessment,
    ))
    .map_err(E::Encode)?;
    let proposal = format!("{:x}", Sha256::digest(encoded));
    let approval = if has_approvals(db)? {
        last_approval(db, &work.id)?
    } else {
        None
    };
    Ok((assessment, proposal, approval))
}

/// Expose the recorded revision and distinction between current and stale
/// receipts. No plan fields, model calls or effective mode are synthesized.
fn render(
    work: &Work,
    assessment: Assessment,
    proposal: String,
    approval: Option<Approval>,
) -> Value {
    let revision = work.revision.to_string();
    let approved = approval.as_ref().is_some_and(|value| {
        value.revision == revision
            && value.proposal == proposal
            && value.recommendation == assessment.recommendation
    });
    json!({"schema_version":1,"work":work.id,"revision":revision,"proposal":proposal,
        "assessment":assessment,"approval":approval,"approved":approved})
}

impl RootStore {
    /// Observe advice for one existing Work, or acknowledge the exact displayed
    /// revision and proposal in an IMMEDIATE transaction. Repeating that
    /// acknowledgement returns the original receipt. A later revision makes
    /// its prior receipt stale. No Work, launch, budget, task, consent or plan is
    /// modified. On uncertain write/close failure, read the receipt before retry.
    pub fn workflow_architecture(
        &self,
        id: &str,
        approve: Option<(&str, &str)>,
    ) -> Result<Value, E> {
        if !swarm_identity_contract::name(id.as_bytes()) {
            return Err(E::Refused("workflow-architecture-work-invalid"));
        }
        match approve {
            None => material_path::with_material_read(self, || {
                let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
                tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
                let result = (|| {
                    let work = crate::read_recorded_work(&tx, id).map_err(E::Work)?;
                    let (assessment, proposal, approval) = observation(&tx, &work)?;
                    Ok(render(&work, assessment, proposal, approval))
                })();
                finished(result, tx.rollback().map_err(E::Sql))
            }),
            Some((revision, digest)) => material_path::with_material_operation(self, || {
                native_transaction::apply_observed(self, |tx| {
                    let work = crate::read_recorded_work(tx, id).map_err(E::Work)?;
                    if revision != work.revision.to_string() {
                        return Err(E::Refused("workflow-architecture-stale"));
                    }
                    let (assessment, proposal, previous) = observation(tx, &work)?;
                    if digest != proposal {
                        return Err(E::Refused("workflow-architecture-stale"));
                    }
                    if assessment.recommendation == "insufficient" {
                        return Err(E::Refused("workflow-architecture-insufficient"));
                    }
                    if let Some(value) = previous.filter(|approval| approval.revision == revision) {
                        if value.proposal != proposal || value.recommendation != assessment.recommendation {
                            return Err(E::Refused("workflow-architecture-approval-conflict"));
                        }
                        return Ok(native_transaction::Change::Observe(render(&work, assessment, proposal, Some(value))));
                    }
                    if !has_approvals(tx)? {
                        tx.execute_batch(SCHEMA).map_err(E::Sql)?;
                    }
                    let at = SystemEnvironment.timestamp().map_err(E::Runtime)?;
                    tx.execute(
                        "INSERT INTO main.workflow_architecture_approvals(work_id,revision,proposal,recommendation,approved_at) VALUES (?1,?2,?3,?4,?5)",
                        rusqlite::params![id, work.revision, proposal, assessment.recommendation, at],
                    ).map_err(E::Sql)?;
                    let saved = last_approval(tx, id)?.ok_or(E::Refused("workflow-architecture-approval-missing"))?;
                    if saved.revision != revision || saved.proposal != proposal || saved.recommendation != assessment.recommendation || saved.approved_at != at {
                        return Err(E::Refused("workflow-architecture-approval-conflict"));
                    }
                    Ok(native_transaction::Change::Mutate(render(&work, assessment, proposal, Some(saved))))
                }).map_err(native_transaction::ObservedSqlFailure::into_root)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlanningScope, PlanningState, Task};

    /// A chain stays sequential even with delegated ownership. Branches require
    /// both delegated work and an explicit coordination witness.
    #[test]
    fn architecture_conservative_classification() {
        let mut work = Work {
            tasks: Some(vec![
                Task {
                    id: "a".into(),
                    scope_id: "alpha".into(),
                    requirements: Some(vec!["req-1".into()]),
                    ..Default::default()
                },
                Task {
                    id: "b".into(),
                    scope_id: "beta".into(),
                    requirements: Some(vec!["req-2".into()]),
                    depends: Some(vec!["a".into()]),
                    ..Default::default()
                },
            ]),
            planning: Some(Box::new(PlanningState {
                scopes: Some(vec![
                    PlanningScope {
                        id: "root".into(),
                        ..Default::default()
                    },
                    PlanningScope {
                        id: "alpha".into(),
                        parent: "root".into(),
                        ..Default::default()
                    },
                    PlanningScope {
                        id: "beta".into(),
                        parent: "root".into(),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            })),
            ..Default::default()
        };
        assert_eq!(assess(&work).unwrap().recommendation, "sequential");
        work.tasks.as_mut().unwrap()[1].depends = None;
        assert_eq!(assess(&work).unwrap().recommendation, "insufficient");
        work.requirement_prerequisites = Some(BTreeMap::from([(
            "req-2".into(),
            Some(vec!["req-1".into()]),
        )]));
        assert_eq!(assess(&work).unwrap().recommendation, "sequential");
        work.tasks.as_mut().unwrap().push(Task {
            id: "c".into(),
            scope_id: "beta".into(),
            requirements: Some(vec!["req-3".into()]),
            ..Default::default()
        });
        assert_eq!(assess(&work).unwrap().recommendation, "hierarchical");
        work.tasks.as_mut().unwrap()[2].requirements = Some(vec!["req-1".into()]);
        assert_eq!(assess(&work).unwrap().recommendation, "insufficient");
        work.tasks.as_mut().unwrap()[0].depends = Some(vec!["b".into()]);
        work.tasks.as_mut().unwrap()[1].depends = Some(vec!["a".into()]);
        assert!(matches!(
            assess(&work),
            Err(E::Refused("workflow-architecture-graph-invalid"))
        ));
    }
}
