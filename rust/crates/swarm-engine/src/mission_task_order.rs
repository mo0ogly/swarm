//! Read-only deterministic ordering, deliberately separate from launch eligibility.
use super::{mission_supervision_store::read_transaction, RootStore, RootStoreError};
use crate::{mission_supervision::finish, MissionSupervisionError as E, Work};
use std::collections::{BTreeMap, BTreeSet};

/// One task's saved priority and structural depth. Decimal strings preserve exact
/// signed priorities and platform-sized counts across JSON consumers. Inclusion
/// says nothing about readiness, consent, available slots or launch permission.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct MissionTaskPosition {
    pub task_id: String,
    pub priority: String,
    pub depth: String,
}
/// One coherent Work/priority SQL observation, including all task statuses.
/// Revision identifies the observed Work, not a settings reservation or permit.
/// This document is not an approved HTTP DTO or an executable dispatch plan.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct MissionTaskOrder {
    pub work: String,
    pub revision: i64,
    pub tasks: Vec<MissionTaskPosition>,
    pub scope: &'static str,
}

/// Validate a complete unique DAG and order all nodes by priority descending,
/// longest prerequisite depth ascending, then lexical task ID. Missing saved
/// priority means the existing schema's neutral zero, not an execution policy.
/// Missing/repeated dependencies, cycles, malformed/duplicate IDs and orphan
/// priority keys refuse. Iterative traversal has no recursion/attempt cutoff;
/// count overflow is a representation error. No IO, mutation or authority.
pub(super) fn ordered(work: &Work, priorities: &BTreeMap<String, i64>) -> Result<MissionTaskOrder, E> {
    let mut tasks = BTreeMap::new();
    for task in work.tasks.iter().flatten() {
        if !swarm_identity_contract::name(task.id.as_bytes())
            || tasks.insert(task.id.as_str(), task).is_some()
        {
            return Err(E::Refused("mission-order-task-id-invalid"));
        }
        let mut seen = BTreeSet::new();
        for parent in task.depends.iter().flatten() {
            if !seen.insert(parent.as_str()) {
                return Err(E::Refused("mission-order-dependency-repeated"));
            }
        }
    }
    if priorities
        .keys()
        .any(|key| !tasks.contains_key(key.as_str()))
    {
        return Err(E::Refused("mission-order-priority-task-missing"));
    }
    let mut depths = BTreeMap::<&str, usize>::new();
    let mut visiting = BTreeSet::new();
    for root in tasks.keys() {
        let mut pending = vec![(*root, false)];
        while let Some((id, exit)) = pending.pop() {
            if depths.contains_key(id) {
                continue;
            }
            let task = tasks
                .get(id)
                .ok_or(E::Refused("mission-order-dependency-missing"))?;
            if exit {
                let mut depth = 0;
                for parent in task.depends.iter().flatten() {
                    let next = depths
                        .get(parent.as_str())
                        .ok_or(E::Refused("mission-order-dependency-missing"))?
                        .checked_add(1)
                        .ok_or(E::Refused("mission-order-depth-overflow"))?;
                    depth = depth.max(next);
                }
                visiting.remove(id);
                depths.insert(id, depth);
            } else {
                if !visiting.insert(id) {
                    return Err(E::Refused("mission-order-dependency-cycle"));
                }
                pending.push((id, true));
                for parent in task.depends.iter().flatten().rev() {
                    pending.push((parent.as_str(), false));
                }
            }
        }
    }
    let mut ids: Vec<_> = tasks.keys().copied().collect();
    // Compare signed priorities directly; negating i64::MIN would overflow.
    ids.sort_by(|a, b| {
        priorities
            .get(*b)
            .unwrap_or(&0)
            .cmp(priorities.get(*a).unwrap_or(&0))
            .then_with(|| depths[a].cmp(&depths[b]))
            .then_with(|| a.cmp(b))
    });
    Ok(MissionTaskOrder {
        work: work.id.clone(),
        revision: work.revision,
        tasks: ids
            .into_iter()
            .map(|id| MissionTaskPosition {
                task_id: id.into(),
                priority: priorities.get(id).copied().unwrap_or(0).to_string(),
                depth: depths[id].to_string(),
            })
            .collect(),
        scope: "order_only_not_eligibility_or_permission",
    })
}

/// Read strict persisted priorities and order a supplied Work in the caller's
/// existing SQL snapshot. Malformed rows/graph and finalization retain causes;
/// this creates no transaction, mutation, capacity observation or permission.
pub(super) fn read_order(db: &rusqlite::Connection, saved: &Work) -> Result<MissionTaskOrder, E> {
    let work = saved.id.as_str();
            let mut statement = db
                .prepare(
                    "SELECT task_id,priority FROM cockpit_tasks WHERE work_id=? ORDER BY task_id",
                )
                .map_err(E::Sql)?;
            let result = (|| {
                let mut rows = statement.query([work]).map_err(E::Sql)?;
                let mut priorities = BTreeMap::new();
                while let Some(row) = rows.next().map_err(E::Sql)? {
                    let task = row.get::<_, String>(0).map_err(E::Sql)?;
                    let value = row.get::<_, i64>(1).map_err(E::Sql)?;
                    if priorities.insert(task, value).is_some() {
                        return Err(E::Refused("mission-order-priority-duplicate"));
                    }
                }
                ordered(saved, &priorities)
            })();
            finish(result, statement.finalize().map_err(E::Sql))
}

impl RootStore {
    /// Read complete deterministic task order from one Work/priority SQL snapshot.
    /// Existing priority rows are untrusted signed integers; malformed/orphan rows
    /// and graph errors refuse rather than disappearing into an empty order.
    /// Explicit statement finalization and rollback preserve independent causes.
    /// No settings adoption, event, reservation, worker, provider or permission.
    /// Caller retains Root close/JSON/output ownership; no setters or UX implied.
    pub fn mission_task_order(&self, work: &str) -> Result<MissionTaskOrder, E> {
        read_transaction(&self.db, || {
            let saved = crate::read_recorded_work(&self.db, work)
                .map_err(|e| E::Root(Box::new(RootStoreError::Work(e))))?;
            read_order(&self.db, &saved)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Data-only DAG control then malformed variants; no SQL or launch authority.
    #[test]
    fn complete_order_refuses_invalid_graph_and_preserves_exact_priorities() {
        let task = |id: &str, parents: &[&str]| crate::Task {
            id: id.into(),
            depends: Some(parents.iter().map(|p| (*p).into()).collect()),
            ..Default::default()
        };
        let mut work = Work {
            id: "work".into(),
            revision: 1,
            tasks: Some(vec![
                task("c", &["b"]),
                task("a", &[]),
                task("b", &["a"]),
                task("z", &[]),
            ]),
            ..Default::default()
        };
        let priorities = BTreeMap::new();
        let valid = ordered(&work, &priorities).unwrap();
        assert_eq!(
            valid
                .tasks
                .iter()
                .map(|t| t.task_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "z", "b", "c"]
        );
        assert_eq!(valid.tasks[3].depth, "2");
        let p = BTreeMap::from([("a".into(), i64::MIN), ("c".into(), i64::MAX)]);
        let exact = ordered(&work, &p).unwrap();
        assert_eq!(exact.tasks[0].priority, i64::MAX.to_string());
        assert_eq!(exact.tasks[3].priority, i64::MIN.to_string());
        let original = work.clone();
        for parents in [vec!["missing"], vec!["a", "a"], vec!["c"]] {
            work.tasks.as_mut().unwrap()[0].depends =
                Some(parents.into_iter().map(String::from).collect());
            assert!(ordered(&work, &priorities).is_err());
            work = original.clone();
        }
        work.tasks.as_mut().unwrap()[1].depends = Some(vec!["c".into()]);
        assert!(ordered(&work, &priorities).is_err());
        work = original.clone();
        work.tasks.as_mut().unwrap().push(task("a", &[]));
        assert!(ordered(&work, &priorities).is_err());
        work = original.clone();
        assert!(ordered(&work, &BTreeMap::from([("missing".into(), 1)])).is_err());
        work.tasks.as_mut().unwrap()[0].id = String::new();
        assert!(ordered(&work, &priorities).is_err());
        assert!(ordered(&Work::default(), &priorities)
            .unwrap()
            .tasks
            .is_empty());
    }
}
