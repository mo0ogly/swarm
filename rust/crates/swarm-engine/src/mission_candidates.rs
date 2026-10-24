//! Structural contenders before capacity and workspace occupancy. This projection
//! is deliberately NOT eligibility, a FIFO registration, a launch or a permit.
use crate::{LaunchProfile, MissionSupervisionError as E, MissionTaskOrder, Work};
use std::collections::BTreeMap;

/// One ordered task and its structural result. Rejected tasks remain visible;
/// the original typed profile/branch error survives under `profile`. An `Ok`
/// still requires current prerequisite evidence, recovery/operator-stop history,
/// cost, consent, lease, provider and workspace checks in the writer transaction.
pub(crate) struct StructuralCandidate {
    pub task_id: String,
    pub profile: Result<LaunchProfile, E>,
}

/// Project every task in one coherent supplied order without IO or mutation.
/// Reject mismatched Work/revision, missing/repeated/orphan order entries, invalid
/// exact priorities/depths and graph cycles before returning any contenders.
/// Ordering reuses the shared DAG comparator: priority descending, longest depth
/// ascending, lexical ID. Supplied priorities prove no persisted observation.
///
/// Per-task refusal preserves unsupported branches, held starts, missing profiles
/// and non-worker roles. A blocked hierarchical task must carry the existing
/// scoped retry gate; blocked manual recovery is not ported here. `Ok` means only
/// structurally possible. No slots/occupancy filtering, prerequisite freshness,
/// recovery decision, process history or provider check is inferred. The consumer
/// must observe those authoritative inputs before registering even a waiting turn
/// and recheck all launch protections when acquiring/releasing it.
pub(crate) fn select(work: &Work, order: &MissionTaskOrder) -> Result<Vec<StructuralCandidate>, E> {
    if order.work != work.id || order.revision != work.revision {
        return Err(E::Refused("mission-candidates-order-stale"));
    }
    let mut priorities = BTreeMap::new();
    for position in &order.tasks {
        let priority = position.priority.parse::<i64>().map_err(|cause| E::Cause {
            code: "mission-candidates-priority-invalid",
            cause: Box::new(cause),
        })?;
        if priority.to_string() != position.priority
            || priorities
                .insert(position.task_id.clone(), priority)
                .is_some()
        {
            return Err(E::Refused("mission-candidates-order-invalid"));
        }
    }
    let expected = super::mission_task_order::ordered(work, &priorities)?;
    if &expected != order {
        return Err(E::Refused("mission-candidates-order-invalid"));
    }
    let tasks = work.tasks.as_deref().unwrap_or_default();
    let indices: BTreeMap<_, _> = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.as_str(), index))
        .collect();
    order
        .tasks
        .iter()
        .map(|position| {
            let index = *indices
                .get(position.task_id.as_str())
                .ok_or(E::Refused("mission-candidates-task-missing"))?;
            let task = &tasks[index];
            let profile = (|| {
                if !matches!(task.status.as_str(), "todo" | "blocked") {
                    return Err(E::Refused("mission-candidates-status-ineligible"));
                }
                if task.status == "blocked" && work.planning.is_none() {
                    return Err(E::Refused("mission-candidates-manual-recovery-unported"));
                }
                crate::agent_launch::supported_work(work, index).map_err(|cause| E::Cause {
                    code: "mission-candidates-work-branch-refused",
                    cause: Box::new(cause),
                })?;
                let parent = work
                    .launch_profile
                    .as_deref()
                    .ok_or(E::Refused("mission-profile-required"))?;
                let profile = super::mission_agent::effective_profile(work, &task.id)?;
                if parent.role != "worker" || profile.role != "worker" {
                    return Err(E::Refused("mission-worker-profile-role-unported"));
                }
                Ok(profile)
            })();
            Ok(StructuralCandidate {
                task_id: task.id.clone(),
                profile,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Data-only control; no fixture is presented as current SQL or permission.
    fn work() -> Work {
        Work {
            id: "mission".into(),
            revision: 7,
            launch_profile: Some(Box::new(LaunchProfile {
                role: "worker".into(),
                provider: "parent".into(),
                workspace: "/fixture".into(),
                ..Default::default()
            })),
            tasks: Some(
                ["a", "b", "c"]
                    .into_iter()
                    .map(|id| crate::Task {
                        id: id.into(),
                        status: "todo".into(),
                        ..Default::default()
                    })
                    .collect(),
            ),
            ..Default::default()
        }
    }

    /// Exact priority sorting and saved override survive the structural projection;
    /// all statuses/held/missing-profile refusals remain visible without mutation.
    #[test]
    fn structural_candidates_keep_order_profiles_and_visible_refusals() {
        let mut work = work();
        work.tasks.as_mut().unwrap()[1].launch_profile = Some(Box::new(LaunchProfile {
            provider: "override".into(),
            role: "worker".into(),
            workspace: "/other".into(),
            ..Default::default()
        }));
        let order = super::super::mission_task_order::ordered(
            &work,
            &BTreeMap::from([("b".into(), i64::MAX), ("c".into(), i64::MIN)]),
        )
        .unwrap();
        let before = crate::encode_model(&work).unwrap();
        let projected = select(&work, &order).unwrap();
        assert_eq!(
            projected
                .iter()
                .map(|c| c.task_id.as_str())
                .collect::<Vec<_>>(),
            ["b", "a", "c"]
        );
        assert_eq!(projected[0].profile.as_ref().unwrap().provider, "override");
        assert_eq!(projected[1].profile.as_ref().unwrap().provider, "parent");
        assert_eq!(before, crate::encode_model(&work).unwrap());
        work.tasks.as_mut().unwrap()[1].model_selection = Some(Box::new(crate::TaskModel {
            provider: "model-provider".into(),
            route: crate::ModelRoute {
                level: "fixture-level".into(),
                ..Default::default()
            },
            ..Default::default()
        }));
        let selected = select(&work, &order).unwrap();
        assert_eq!(
            selected[0].profile.as_ref().unwrap().provider,
            "model-provider"
        );
        assert_eq!(selected[0].profile.as_ref().unwrap().level, "fixture-level");
        work.launch_profile.as_mut().unwrap().role = "planner".into();
        assert!(select(&work, &order)
            .unwrap()
            .iter()
            .all(|c| c.profile.is_err()));
        work.launch_profile.as_mut().unwrap().role = "worker".into();
        for status in [
            "running",
            "submitted",
            "accepted",
            "waived",
            "abandoned",
            "unknown",
        ] {
            work.tasks.as_mut().unwrap()[0].status = status.into();
            let projected = select(&work, &order).unwrap();
            assert_eq!(projected.len(), 3);
            assert_eq!(
                projected[1].profile.as_ref().unwrap_err().code(),
                "mission-candidates-status-ineligible"
            );
        }
        work.tasks.as_mut().unwrap()[0].status = "blocked".into();
        assert_eq!(
            select(&work, &order).unwrap()[1]
                .profile
                .as_ref()
                .unwrap_err()
                .code(),
            "mission-candidates-manual-recovery-unported"
        );
        work.tasks.as_mut().unwrap()[0].status = "todo".into();
        work.tasks.as_mut().unwrap()[0].launch_held = true;
        assert!(select(&work, &order).unwrap()[1].profile.is_err());
        work.tasks.as_mut().unwrap()[0].launch_held = false;
        work.launch_profile = None;
        assert!(select(&work, &order)
            .unwrap()
            .iter()
            .all(|c| c.profile.is_err()));
    }

    /// Mismatched observations, omission and malformed DAGs fail the whole read;
    /// no empty-success fallback or mutation masks a bad input. Restoration works.
    #[test]
    fn structural_candidates_refuse_stale_incomplete_and_cyclic_order() {
        let mut work = work();
        let order = super::super::mission_task_order::ordered(&work, &BTreeMap::new()).unwrap();
        assert_eq!(select(&work, &order).unwrap().len(), 3);
        work.revision += 1;
        assert!(select(&work, &order).is_err());
        work.revision -= 1;
        let mut incomplete = order.clone();
        incomplete.tasks.pop();
        assert!(select(&work, &incomplete).is_err());
        let mut duplicate = order.clone();
        duplicate.tasks.push(order.tasks[0].clone());
        assert!(select(&work, &duplicate).is_err());
        let mut depth = order.clone();
        depth.tasks[0].depth = "1".into();
        assert!(select(&work, &depth).is_err());
        let mut invalid = order.clone();
        invalid.tasks[0].priority = "00".into();
        assert!(select(&work, &invalid).is_err());
        invalid.tasks[0].priority = "9223372036854775808".into();
        let error = match select(&work, &invalid) {
            Ok(_) => panic!("overflow accepted"),
            Err(error) => error,
        };
        assert_eq!(error.code(), "mission-candidates-priority-invalid");
        assert!(std::error::Error::source(&error)
            .unwrap()
            .is::<std::num::ParseIntError>());
        for parent in ["a", "missing"] {
            work.tasks.as_mut().unwrap()[0].depends = Some(vec![parent.into()]);
            assert!(select(&work, &order).is_err());
        }
        work.tasks.as_mut().unwrap()[0].depends = None;
        assert_eq!(select(&work, &order).unwrap().len(), 3);
    }
}
