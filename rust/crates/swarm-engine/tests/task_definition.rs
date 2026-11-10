use std::collections::BTreeMap;
use swarm_engine::{
    apply_task_metadata, composed_task_instructions, decode_work, normalize_validation_policy,
    update_task_definition, update_task_instructions, validate_task_dependency_graph,
    validation_policy_covers_task, DefinitionContext, Request, Task, TaskDefinitionLimits,
    ValidationControl, ValidationPolicy, ValidationPolicyLimits, Work,
};
fn work() -> Work {
    decode_work(br#"{"schema_version":1,"id":"work","revision":3,"tasks":[{"id":"task","title":"Old","deliverable":"report","criteria":["proof"],"status":"todo","attempts":[{"id":"prior"}],"gate":{},"automatic_validation":{},"manual_override":{},"revalidation":{},"plan_checks":{"plan-criterion-9":"validation","custom":"delivery"},"independent_review":{"id":"review","state":"passed"}}]}"#).unwrap()
}
fn context() -> DefinitionContext<'static> {
    DefinitionContext {
        authorized_at: "supplied-time",
        actor: "supplied-actor",
        task_limits: TaskDefinitionLimits::legacy_go(),
        policy_limits: ValidationPolicyLimits::legacy_go(),
    }
}
fn request() -> Request {
    Request {
        schema_version: 1,
        id: "task".into(),
        ..Default::default()
    }
}
fn policy() -> ValidationPolicy {
    ValidationPolicy {
        mode: " automatic\u{3000}".into(),
        controls: Some(vec![ValidationControl {
            id: " check ".into(),
            command: Some(vec!["go".into(), "version".into()]),
            criteria: Some(vec![1]),
            justification: " objective evidence ".into(),
            ..Default::default()
        }]),
        authorized_at: "untrusted".into(),
        actor: "untrusted".into(),
    }
}
/// Explicit definition edits invalidate prior evidence and rebuild ordinal
/// criteria while preserving the adopted custom/phase requirements. These pure
/// supplied stamps/limits do not authorize execution, review or acceptance.
#[test]
fn definition_changes_archive_only_changed_review_contract_and_rebuild_checks() {
    let w = work();
    let t = &w.tasks.as_ref().unwrap()[0];
    let budget = update_task_definition(
        &w,
        t,
        &Request {
            max_tool_calls: 20,
            ..request()
        },
        &context(),
    )
    .unwrap();
    assert_eq!(budget.independent_review, t.independent_review);
    assert!(budget.gate.is_none() && budget.automatic_validation.is_none());
    let changed = update_task_definition(
        &w,
        t,
        &Request {
            criteria: Some(vec!["new".into(), "second".into()]),
            ..request()
        },
        &context(),
    )
    .unwrap();
    assert!(changed.independent_review.is_none());
    assert_eq!(changed.previous_reviews.as_ref().unwrap()[0].id, "review");
    assert_eq!(changed.attempts, t.attempts);
    assert!(changed.manual_override.is_none() && changed.revalidation.is_none());
    let checks = changed.plan_checks.as_ref().unwrap();
    // Preserve the adopted custom requirement, rebuild two criterion IDs and
    // do not inject current creation-phase defaults into the existing contract.
    assert_eq!(checks.len(), 3);
    assert_eq!(checks["custom"], "delivery");
    assert!(!checks.contains_key("plan-criterion-9"));
    let mut updated = w.clone();
    updated.tasks.as_mut().unwrap()[0] = changed.clone();
    assert_eq!(
        update_task_definition(
            &updated,
            &changed,
            &Request {
                criteria: changed.criteria.clone(),
                ..request()
            },
            &context()
        )
        .unwrap(),
        changed
    );
}
#[test]
fn invalid_definition_and_instruction_changes_leave_entire_work_unchanged() {
    let mut w = work();
    let original = w.clone();
    for r in [
        Request {
            criteria: Some(vec![]),
            ..request()
        },
        Request {
            depends: Some(vec!["task".into()]),
            ..request()
        },
        Request {
            depends: Some(vec!["absent".into()]),
            ..request()
        },
        Request {
            max_attempts: 4,
            ..request()
        },
        Request {
            title: "\u{0085}".into(),
            ..request()
        },
    ] {
        assert!(apply_task_metadata(&mut w, &r, &context()).is_err());
        assert_eq!(w, original);
    }
    w.tasks.as_mut().unwrap()[0]
        .independent_review
        .as_mut()
        .unwrap()
        .state = "running".into();
    let before = w.clone();
    assert!(apply_task_metadata(
        &mut w,
        &Request {
            title: "new".into(),
            ..request()
        },
        &context()
    )
    .is_err());
    assert_eq!(w, before);
    apply_task_metadata(
        &mut w,
        &Request {
            owner: "owner".into(),
            ..request()
        },
        &context(),
    )
    .unwrap();
    assert_eq!(w.tasks.as_ref().unwrap()[0].owner, "owner");
}
#[test]
fn graph_validation_handles_deep_paths_duplicates_missing_nodes_and_cycles() {
    let mut w = Work {
        tasks: Some(
            (0..20000)
                .map(|i| Task {
                    id: format!("t{i}"),
                    depends: if i == 19999 {
                        None
                    } else {
                        Some(vec![format!("t{}", i + 1)])
                    },
                    ..Default::default()
                })
                .collect(),
        ),
        ..Default::default()
    };
    let replacement = w.tasks.as_ref().unwrap()[19999].clone();
    validate_task_dependency_graph(&w, &replacement).unwrap();
    for deps in [
        vec!["t0".into()],
        vec!["missing".into()],
        vec!["t0".into(), "t0".into()],
    ] {
        let mut bad = replacement.clone();
        bad.depends = Some(deps);
        assert!(validate_task_dependency_graph(&w, &bad).is_err());
    }
    w.tasks.as_mut().unwrap()[0].depends = Some(vec!["t1".into(), "t1".into()]);
    assert!(validate_task_dependency_graph(&w, &replacement).is_err());
}
#[test]
fn policy_normalization_requires_explicit_limits_and_does_not_authenticate_actor() {
    let p = normalize_validation_policy(policy(), &ValidationPolicyLimits::legacy_go()).unwrap();
    assert_eq!(p.mode, "automatic");
    assert!(p.actor.is_empty() && p.authorized_at.is_empty());
    let c = &p.controls.as_ref().unwrap()[0];
    assert_eq!(c.id, "check");
    assert_eq!(c.timeout_seconds, 60);
    let t = Task {
        criteria: Some(vec!["proof".into()]),
        ..Default::default()
    };
    validation_policy_covers_task(&p, &t).unwrap();
    let mut limits = ValidationPolicyLimits::legacy_go();
    limits.default_timeout_seconds = 17;
    let mut rust_policy = policy();
    rust_policy.controls.as_mut().unwrap()[0].command = Some(vec!["cargo".into(), "check".into()]);
    assert!(normalize_validation_policy(rust_policy.clone(), &limits).is_err());
    limits.allowed_programs.insert("cargo".into());
    assert!(normalize_validation_policy(rust_policy, &limits).is_ok());
    assert_eq!(
        normalize_validation_policy(policy(), &limits)
            .unwrap()
            .controls
            .unwrap()[0]
            .timeout_seconds,
        17
    );
    for bad in [
        ValidationControl {
            command: Some(vec!["sh".into()]),
            ..c.clone()
        },
        ValidationControl {
            inputs: Some(vec!["../out".into()]),
            ..c.clone()
        },
        ValidationControl {
            environment_exit_codes: Some(vec![7, 7]),
            ..c.clone()
        },
        ValidationControl {
            timeout_seconds: 301,
            ..c.clone()
        },
        ValidationControl {
            justification: "short".into(),
            ..c.clone()
        },
    ] {
        assert!(normalize_validation_policy(
            ValidationPolicy {
                controls: Some(vec![bad]),
                ..p.clone()
            },
            &limits
        )
        .is_err());
    }
    let t = Task {
        criteria: Some(vec!["proof".into(), "uncovered".into()]),
        ..t
    };
    assert!(validation_policy_covers_task(&p, &t).is_err());
    let mut human = p.clone();
    human.mode = "human".into();
    validation_policy_covers_task(&human, &t).unwrap();
    let t = Task {
        criteria: Some(vec!["proof".into()]),
        plan_checks: Some(BTreeMap::from([("custom".into(), "delivery".into())])),
        ..Default::default()
    };
    assert!(validation_policy_covers_task(&p, &t).is_err());
}
#[test]
fn instructions_preserve_attributed_learning_and_reject_ambiguous_relocation_atomically() {
    let mut t=swarm_engine::decode_task(br#"{"id":"task","status":"todo","instruction_base":"original","next":"learning\n\noriginal","instruction_revisions":[{"state":"applied","addition":"learning"},{"state":"rejected","addition":"rejected"}]}"#).unwrap();
    let original = t.instruction_base.clone();
    update_task_instructions(&mut t, "operator\n\nlearning\n\noriginal").unwrap();
    assert_eq!(composed_task_instructions(&t), t.next);
    assert_eq!(t.instruction_base, original);
    update_task_instructions(&mut t, "replacement").unwrap();
    assert_eq!(t.next, "learning\n\nreplacement");
    let before = t.clone();
    assert!(update_task_instructions(&mut t, "new\n\nlearning\n\nmisplaced").is_err());
    assert_eq!(t, before);
    t.instruction_revisions.as_mut().unwrap()[0].state = "rejected".into();
    assert_eq!(composed_task_instructions(&t), "replacement");
}
#[test]
fn definition_then_metadata_failure_is_atomic_and_supplied_stamps_are_data_only() {
    let mut w = work();
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.instruction_base = Some(Box::new("base".into()));
    t.next = "unrecognizable edit".into();
    let before = w.clone();
    let r = Request {
        title: "new".into(),
        owner: "other".into(),
        next: "replacement".into(),
        ..request()
    };
    assert!(apply_task_metadata(&mut w, &r, &context()).is_err());
    assert_eq!(w, before);
    let mut w = work();
    w.tasks.as_mut().unwrap()[0].plan_checks = None;
    apply_task_metadata(
        &mut w,
        &Request {
            validation_policy: Some(policy()),
            ..request()
        },
        &context(),
    )
    .unwrap();
    let p = w.tasks.as_ref().unwrap()[0]
        .validation_policy
        .as_ref()
        .unwrap();
    assert_eq!(p.actor, "supplied-actor");
    assert_eq!(p.authorized_at, "supplied-time");
    assert_eq!(w.revision, 3);
}

/// Automatic command identities come from the explicitly normalized policy;
/// custom mandatory requirements survive and cannot silently become optional.
#[test]
fn automatic_policy_uses_authorized_control_ids_without_discarding_custom_requirements() {
    let mut w = Work::default();
    swarm_engine::apply_task_addition(
        &mut w,
        &Request {
            id: "task".into(),
            title: "Task".into(),
            deliverable: "Report".into(),
            criteria: Some(vec!["Proof".into()]),
            ..request()
        },
    )
    .unwrap();
    let original = &w.tasks.as_ref().unwrap()[0];
    let automatic = update_task_definition(
        &w,
        original,
        &Request {
            validation_policy: Some(policy()),
            ..request()
        },
        &context(),
    )
    .unwrap();
    assert_eq!(
        automatic.plan_checks.as_ref().unwrap(),
        &BTreeMap::from([("check".into(), "delivery".into())])
    );
    let mut protected = automatic.clone();
    protected
        .plan_checks
        .as_mut()
        .unwrap()
        .insert("security-review".into(), "delivery".into());
    let before = protected.clone();
    assert!(update_task_definition(
        &w,
        &protected,
        &Request {
            validation_policy: Some(policy()),
            ..request()
        },
        &context()
    )
    .is_err());
    assert_eq!(protected, before);
    let mut authorized = policy();
    let mut extra = authorized.controls.as_ref().unwrap()[0].clone();
    extra.id = "security-review".into();
    authorized.controls.as_mut().unwrap().push(extra);
    let allowed = update_task_definition(
        &w,
        &protected,
        &Request {
            validation_policy: Some(authorized),
            ..request()
        },
        &context(),
    )
    .unwrap();
    assert_eq!(
        allowed.plan_checks.as_ref().unwrap()["security-review"],
        "delivery"
    );
}
/// A meaningful mapping adoption is not discarded as text-identical metadata.
/// This is historical fixture data only; the caller still owns authorization.
#[test]
fn identical_criteria_adopt_legacy_mapping_and_clear_old_gate_without_acceptance() {
    let mut w = work();
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.plan_checks = None;
    let before = t.clone();
    let adopted = update_task_definition(
        &w,
        &before,
        &Request {
            criteria: before.criteria.clone(),
            ..request()
        },
        &context(),
    )
    .unwrap();
    assert_eq!(adopted.plan_checks.as_ref().unwrap().len(), 4);
    assert!(
        adopted.gate.is_none()
            && adopted.automatic_validation.is_none()
            && adopted.manual_override.is_none()
    );
    assert_eq!(adopted.status, "todo");
    assert_eq!(adopted.criteria, before.criteria);
    assert_eq!(w.tasks.as_ref().unwrap()[0], before);
}
