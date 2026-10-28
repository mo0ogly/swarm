//! Cumulative control data planning, never native admission or acceptance.
use crate::{
    AgentLaunchError as E, ValidationControl, ValidationPolicy, ValidationPolicyLimits, Work,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One original task policy and its separately normalized command contract.
/// Normalization clears authority metadata; the original must therefore remain
/// alongside it. Neither stored actor strings nor this DTO establish a grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CohortPolicy {
    pub task_id: String,
    pub original: ValidationPolicy,
    pub normalized: ValidationPolicy,
}
/// A cumulative slot keeps its actual task/control names separate from the
/// transport key. Equal control names in distinct tasks are valid. No command
/// is renamed or authorized by the transport key, ordinal or serialization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ControlSlot {
    pub ordinal: usize,
    pub task_id: String,
    pub control: ValidationControl,
    pub transport_id: String,
}
/// Supplied-data plan for the selected task and every already accepted task, in
/// stored Work order. The digest reuses managed_review_contract, excluding the
/// controller's own optimistic Work revision. This is not an execution source
/// hash: current defaults may change normalized policies without changing this
/// business digest. A native job must bind normalized policies/slots, settings
/// and owned candidate witnesses separately. Commands are private engine data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ControlPlan {
    pub contract: String,
    pub policies: Vec<CohortPolicy>,
    pub slots: Vec<ControlSlot>,
}

/// Validate cumulative controls against explicit current saved policy limits.
///
/// Inputs are supplied Work data, the selected task ID and actual validation
/// limits. Every participating task needs an original authorized automatic
/// policy; each is normalized and coverage-checked independently. There is no
/// aggregate policy ceiling, fallback or implicit timeout. Existing limits keep
/// their per-task meaning. Task/control order is preserved, and a canonical
/// tuple hash distinguishes same-named controls belonging to different tasks.
///
/// Returns a complete plan or a safe static refusal/typed codec/policy cause.
/// Missing selection, duplicate/invalid participating IDs, missing authority,
/// nonautomatic mode, invalid policy/coverage or transport collision refuse the
/// whole plan. This allocates only: no filesystem, SQL, command, lease, mutation,
/// execution permission, source observation or acceptance is established. Actual
/// producer, Root, candidate witnesses and gate permissions remain caller duties.
pub(crate) fn plan(
    work: &Work,
    selected: &str,
    limits: &ValidationPolicyLimits,
) -> Result<ControlPlan, E> {
    if !swarm_identity_contract::name(selected.as_bytes()) {
        return Err(E::Refused("managed-controls-task-invalid"));
    }
    let mut tasks = BTreeSet::new();
    let mut transports = BTreeSet::new();
    let mut policies = Vec::new();
    let mut slots = Vec::new();
    let mut found = false;
    for task in work.tasks.iter().flatten() {
        if task.id != selected && task.status != "accepted" {
            continue;
        }
        if !swarm_identity_contract::name(task.id.as_bytes()) || !tasks.insert(task.id.clone()) {
            return Err(E::Refused("managed-controls-task-invalid-or-duplicate"));
        }
        found |= task.id == selected;
        let original = task
            .validation_policy
            .as_deref()
            .ok_or(E::Refused("managed-controls-policy-required"))?;
        if crate::text::trim_space(&original.actor).is_empty()
            || crate::text::trim_space(&original.authorized_at).is_empty()
        {
            return Err(E::Refused("managed-controls-policy-unauthorized"));
        }
        let normalized = crate::normalize_validation_policy(original.clone(), limits)
            .map_err(|e| E::caused("managed-controls-policy-invalid", e))?;
        if normalized.mode != "automatic" {
            return Err(E::Refused("managed-controls-automatic-required"));
        }
        crate::validation_policy_covers_task(&normalized, task)
            .map_err(|e| E::caused("managed-controls-coverage", e))?;
        for control in normalized.controls.iter().flatten() {
            let raw = crate::encode_model(&(&task.id, &control.id))
                .map_err(|e| E::caused("managed-controls-slot-encode", e))?;
            let transport_id = super::hash(&raw);
            if !transports.insert(transport_id.clone()) {
                return Err(E::Refused("managed-controls-slot-collision"));
            }
            slots.push(ControlSlot {
                ordinal: slots.len(),
                task_id: task.id.clone(),
                control: control.clone(),
                transport_id,
            });
        }
        policies.push(CohortPolicy {
            task_id: task.id.clone(),
            original: original.clone(),
            normalized,
        });
    }
    if !found {
        return Err(E::Refused("managed-controls-task-missing"));
    }
    let contract = crate::managed_review_contract(work, selected)
        .map_err(|e| E::caused("managed-controls-contract", e))?;
    Ok(ControlPlan {
        contract,
        policies,
        slots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Explicit data fixture; settings limits are supplied by the test, never a
    /// production fallback. No actual SQL producer, process or grant is created.
    fn task(id: &str, status: &str) -> crate::Task {
        crate::Task {
            id: id.into(),
            status: status.into(),
            criteria: Some(vec!["criterion".into()]),
            validation_policy: Some(Box::new(ValidationPolicy {
                mode: "automatic".into(),
                authorized_at: "fixture-at".into(),
                actor: "fixture-actor".into(),
                controls: Some(vec![ValidationControl {
                    id: "same-check".into(),
                    command: Some(vec![
                        "python3".into(),
                        "-c".into(),
                        "print('fixture')".into(),
                    ]),
                    criteria: Some(vec![1]),
                    justification: "Verify the declared fixture criterion".into(),
                    timeout_seconds: 1,
                    ..Default::default()
                }]),
            })),
            ..Default::default()
        }
    }
    #[test]
    fn managed_controls_preserve_cohort_and_per_task_limits_without_aggregate_cap() {
        let mut limits = ValidationPolicyLimits::legacy_go();
        limits.max_controls = 1;
        limits.max_total_timeout_seconds = 1;
        let mut work = Work {
            tasks: Some(vec![
                task("prior", "accepted"),
                task("current", "submitted"),
                task("unrelated", "todo"),
            ]),
            ..Default::default()
        };
        let p = plan(&work, "current", &limits).unwrap();
        assert_eq!(p.policies.len(), 2);
        assert_eq!(p.slots.len(), 2);
        assert_eq!(p.slots[0].task_id, "prior");
        assert_eq!(p.slots[1].task_id, "current");
        assert_eq!(p.slots[0].control.id, p.slots[1].control.id);
        assert_ne!(p.slots[0].transport_id, p.slots[1].transport_id);
        assert_eq!(
            p.slots.iter().map(|s| s.ordinal).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            p.contract,
            crate::managed_review_contract(&work, "current").unwrap()
        );
        assert!(!p.policies[0].original.actor.is_empty());
        assert!(p.policies[0].normalized.actor.is_empty());
        work.revision += 1;
        assert_eq!(plan(&work, "current", &limits).unwrap(), p);
        work.tasks.as_mut().unwrap()[0]
            .validation_policy
            .as_mut()
            .unwrap()
            .actor = "changed-actor".into();
        assert_ne!(
            plan(&work, "current", &limits).unwrap().contract,
            p.contract
        );
    }
    #[test]
    fn managed_controls_valid_witness_then_invalid_policies_and_recovery() {
        let limits = ValidationPolicyLimits::legacy_go();
        let valid = Work {
            tasks: Some(vec![
                task("current", "submitted"),
                task("prior", "accepted"),
            ]),
            ..Default::default()
        };
        let original = plan(&valid, "current", &limits).unwrap();
        for fault in 0..6 {
            let mut work = valid.clone();
            let task = &mut work.tasks.as_mut().unwrap()[1];
            match fault {
                0 => task.validation_policy = None,
                1 => task.validation_policy.as_mut().unwrap().actor.clear(),
                2 => task
                    .validation_policy
                    .as_mut()
                    .unwrap()
                    .authorized_at
                    .clear(),
                3 => task.validation_policy.as_mut().unwrap().mode = "human".into(),
                4 => {
                    task.validation_policy
                        .as_mut()
                        .unwrap()
                        .controls
                        .as_mut()
                        .unwrap()[0]
                        .command = Some(vec!["not-allowed".into()])
                }
                _ => task.criteria.as_mut().unwrap().push("uncovered".into()),
            }
            assert!(plan(&work, "current", &limits).is_err());
            assert_eq!(plan(&valid, "current", &limits).unwrap(), original);
        }
    }
    #[test]
    fn managed_controls_valid_witness_then_missing_duplicate_or_invalid_selection() {
        let limits = ValidationPolicyLimits::legacy_go();
        let mut work = Work {
            tasks: Some(vec![task("current", "submitted")]),
            ..Default::default()
        };
        assert!(plan(&work, "current", &limits).is_ok());
        assert_eq!(
            plan(&work, "absent", &limits).err().unwrap().code(),
            "managed-controls-task-missing"
        );
        assert_eq!(
            plan(&work, "../invalid", &limits).err().unwrap().code(),
            "managed-controls-task-invalid"
        );
        work.tasks
            .as_mut()
            .unwrap()
            .push(task("current", "accepted"));
        assert_eq!(
            plan(&work, "current", &limits).err().unwrap().code(),
            "managed-controls-task-invalid-or-duplicate"
        );
    }
    /// Business candidate identity does not authorize effective runtime commands.
    /// A saved default change must remain visible in the plan consumed by the job.
    #[test]
    fn managed_controls_effective_default_changes_plan_without_changing_business_digest() {
        let mut limits = ValidationPolicyLimits::legacy_go();
        let mut current = task("current", "submitted");
        current
            .validation_policy
            .as_mut()
            .unwrap()
            .controls
            .as_mut()
            .unwrap()[0]
            .timeout_seconds = 0;
        let work = Work {
            tasks: Some(vec![current]),
            ..Default::default()
        };
        limits.default_timeout_seconds = 1;
        let first = plan(&work, "current", &limits).unwrap();
        limits.default_timeout_seconds = 2;
        let changed = plan(&work, "current", &limits).unwrap();
        assert_eq!(first.contract, changed.contract);
        assert_eq!(first.policies[0].original, changed.policies[0].original);
        assert_eq!(first.slots[0].transport_id, changed.slots[0].transport_id);
        assert_ne!(first.policies[0].normalized, changed.policies[0].normalized);
        assert_eq!(first.slots[0].control.timeout_seconds, 1);
        assert_eq!(changed.slots[0].control.timeout_seconds, 2);
    }
}
