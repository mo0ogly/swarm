//! Quota candidate data; real Root/CLI/publication is verified separately.
use swarm_engine::{
    apply_quota_change, decode_quota_change, decode_work, encode_model, quota_view,
    EngineSettingsValues, QuotaChange, SettingCap, SettingCount, Work,
};
const AT: &str = "2026-10-21T00:00:00Z";
/// Existing valid native tree with nonzero consumption and explicit reviewer data.
/// This is supplied data, no actor/kernel/provider identity or execution proof.
fn fixture() -> Work {
    let work=decode_work(br#"{"schema_version":1,"id":"work","revision":1,"criteria":["Child","Other"],"tasks":[{"id":"task-child","scope_id":"child","requirements":["req-1"],"status":"accepted"},{"id":"task-other","scope_id":"other","requirements":["req-2"],"status":"failed"}],"planning":{"version":1,"max_tasks":10,"max_decisions":10,"max_activations":20,"activations":2,"decisions":1,"reviewer":{"provider":"fixture","max_calls":3,"calls":1,"authorized":"supplied","max_context_files":24,"max_source_bytes":98304,"max_context_bytes":131072,"max_files":100},"scopes":[{"id":"root","revision":1,"activations":2},{"id":"parent","parent":"root","revision":1,"max_tasks":2,"max_activations":2,"activations":1},{"id":"child","parent":"parent","revision":1,"requirements":["req-1"],"max_activations":1,"activations":1},{"id":"other","parent":"root","revision":1,"requirements":["req-2"]}]}}"#).unwrap();
    swarm_engine::validate_planning_state(&work).unwrap();
    work
}
/// Preserve full current values unless the test explicitly changes a field.
fn request(work: &Work) -> QuotaChange {
    QuotaChange {
        schema_version: 1,
        event_id: "quota-change".into(),
        expected_revision: work.revision,
        limits: quota_view(work).unwrap().limits,
        reason: "Explicit fixture quota change".into(),
        ..Default::default()
    }
}
/// Read the real saved defaults; never a helper's fallback.
fn policy() -> swarm_engine::QuotaReasonPolicy {
    EngineSettingsValues::installation_defaults()
        .unwrap()
        .quota_reason_policy()
        .unwrap()
}
/// Missing historical reviewer caps stay absent, and used counters are never defaults.
#[test]
fn quota_view_preserves_consumption_and_legacy_absence_without_fallback() {
    let mut work = fixture();
    let original = encode_model(&work).unwrap();
    let view = quota_view(&work).unwrap();
    assert_eq!(view.consumed.planning_activations, 2);
    assert_eq!(view.remaining.planning_activations, 18);
    assert_eq!(view.consumed.delegation_depth.as_deref(), Some(&3));
    assert!(view.remaining.delegation_depth.is_none());
    assert_eq!(encode_model(&work).unwrap(), original);
    work.planning
        .as_mut()
        .unwrap()
        .reviewer
        .as_mut()
        .unwrap()
        .max_context_bytes = None;
    assert!(quota_view(&work)
        .unwrap()
        .limits
        .review_context_bytes
        .is_none());
    work.planning.as_mut().unwrap().activations = -1;
    assert!(quota_view(&work).is_err());
}
/// Higher caps update only selected values and audited before/after; pauses,
/// failures, counts, attempts and unlisted scope allocations remain untouched.
#[test]
fn quota_change_preserves_history_consumption_and_unlisted_allocations() {
    let mut work = fixture();
    let mut change = request(&work);
    change.limits.planning_activations = 30;
    change.limits.review_calls = Some(Box::new(4));
    change.limits.review_context_bytes = Some(Box::new(0));
    work.planning.as_mut().unwrap().paused = true;
    work.planning.as_mut().unwrap().failure = "Retained failure".into();
    let old = quota_view(&work).unwrap();
    apply_quota_change(&mut work, &change, "operator/uid:1", AT, &policy()).unwrap();
    let next = quota_view(&work).unwrap();
    assert_eq!(next.consumed, old.consumed);
    assert_eq!(next.limits.review_context_bytes.as_deref(), Some(&0));
    assert_eq!(next.scopes, old.scopes);
    assert_eq!(work.revision, 1);
    let p = work.planning.as_ref().unwrap();
    assert!(p.paused);
    assert_eq!(p.failure, "Retained failure");
    let audit = p.quota_authorization.as_ref().unwrap();
    assert_eq!(audit.before, old.limits);
    assert_eq!(audit.after, next.limits);
    assert_eq!(audit.actor, "operator/uid:1");
    assert_eq!(audit.at, AT);
}
/// Every malformed/insufficient update follows a healthy witness and must leave
/// the complete supplied candidate byte-identical, even after late validation.
#[test]
fn quota_refusals_are_atomic_and_cannot_supply_authority_or_reset_usage() {
    let work = fixture();
    let control = request(&work);
    let mut healthy = work.clone();
    apply_quota_change(&mut healthy, &control, "operator", AT, &policy()).unwrap();
    for index in 0..7 {
        let mut candidate = work.clone();
        let mut change = control.clone();
        match index {
            0 => change.limits.planning_activations = 1,
            1 => change.limits.review_calls = Some(Box::new(0)),
            2 => change.limits.planning_tasks = Some(Box::new(1)),
            3 => change.limits.delegation_depth = Some(Box::new(2)),
            4 => change.reason = " ".into(),
            5 => change.scope_activations = Some([(String::from("absent"), 1)].into()),
            _ => change.expected_revision = 2,
        }
        let before = encode_model(&candidate).unwrap();
        assert!(apply_quota_change(&mut candidate, &change, "operator", AT, &policy()).is_err());
        assert_eq!(encode_model(&candidate).unwrap(), before);
    }
    let raw = serde_json::to_vec(&control).unwrap();
    assert!(decode_quota_change(&raw).is_ok());
    let mut raw: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    raw["actor"] = "invented".into();
    assert!(decode_quota_change(&serde_json::to_vec(&raw).unwrap()).is_err());
}
/// Clearing a local override restores inheritance but all ancestor/global
/// admission checks still apply; zero is never a refund of completed calls.
#[test]
fn quota_scope_zero_restores_inheritance_without_bypassing_ancestor_budget() {
    let mut work = fixture();
    let mut change = request(&work);
    change.scope_activations = Some([(String::from("child"), 0)].into());
    apply_quota_change(&mut work, &change, "operator", AT, &policy()).unwrap();
    let p = work.planning.as_mut().unwrap();
    let scopes = p.scopes.as_mut().unwrap();
    assert_eq!(scopes[2].max_activations, 0);
    assert_eq!(scopes[2].activations, 1);
    scopes[1].activations = 2;
    assert!(swarm_engine::check_planning_activation_budget(&work, "child").is_err());
    let mut work = fixture();
    let mut bad = request(&work);
    bad.scope_activations = Some([(String::from("child"), 3)].into());
    assert!(apply_quota_change(&mut work, &bad, "operator", AT, &policy()).is_err());
}
/// Scope ownership/review state is not cleared to update policy. Unimplemented
/// context-window upgrades refuse instead of invoking a guessed model/client.
#[test]
fn quota_live_owners_and_unported_window_refuse_without_clearing_state() {
    for kind in 0..3 {
        let mut work = fixture();
        let mut change = request(&work);
        match kind {
            0 => {
                work.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].holder =
                    "live-owner".into()
            }
            1 => {
                work.tasks.as_mut().unwrap()[0].independent_review =
                    Some(Box::new(swarm_engine::IndependentReview {
                        state: "running".into(),
                        ..Default::default()
                    }))
            }
            _ => change.limits.review_context_tokens = Some(Box::new(1000)),
        }
        let before = encode_model(&work).unwrap();
        assert!(apply_quota_change(&mut work, &change, "operator", AT, &policy()).is_err());
        assert_eq!(encode_model(&work).unwrap(), before);
    }
}
/// Quota reasons obey the actual saved byte policy, including UTF8, whitespace,
/// legacy absence and explicit unlimited; no private reason enters diagnostics.
#[test]
fn quota_saved_reason_policy_is_effective_and_legacy_absence_refuses() {
    let work = fixture();
    let mut request = request(&work);
    request.reason = "why".into();
    assert!(apply_quota_change(&mut work.clone(), &request, "operator", AT, &policy()).is_err());
    let mut values = EngineSettingsValues::installation_defaults().unwrap();
    values.planning_quota_reason_min_bytes = Some(SettingCount(3));
    values.planning_quota_reason_max_bytes = Some(SettingCap(Some(SettingCount(3))));
    assert!(apply_quota_change(
        &mut work.clone(),
        &request,
        "operator",
        AT,
        &values.quota_reason_policy().unwrap()
    )
    .is_ok());
    request.reason = " why ".into();
    let error = apply_quota_change(
        &mut work.clone(),
        &request,
        "operator",
        AT,
        &values.quota_reason_policy().unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code(), "quota-reason-outside-policy");
    assert!(!error.to_string().contains("why"));
    values.planning_quota_reason_min_bytes = Some(SettingCount(0));
    values.planning_quota_reason_max_bytes = Some(SettingCap(None));
    request.reason = "é".into();
    assert!(apply_quota_change(
        &mut work.clone(),
        &request,
        "operator",
        AT,
        &values.quota_reason_policy().unwrap()
    )
    .is_ok());
    values.planning_quota_reason_min_bytes = None;
    values.planning_quota_reason_max_bytes = None;
    assert!(values.quota_reason_policy().is_err());
}

/// A deep actual tree has no recursive-stack or repeated full-ancestry walk.
/// Test volume is fixture data, not a product bound or temporal performance claim.
/// A valid witness precedes cyclic, missing, duplicate and invalid root variants.
#[test]
fn quota_tree_deep_shared_ancestry_and_invalid_graphs() {
    let mut work = fixture();
    let root = swarm_engine::PlanningScope {
        id: "root".into(),
        revision: 1,
        ..Default::default()
    };
    let mut scopes = vec![root];
    for n in 1..=10000 {
        scopes.push(swarm_engine::PlanningScope {
            id: format!("s-{n}"),
            parent: if n == 1 {
                "root".into()
            } else {
                format!("s-{}", n - 1)
            },
            revision: 1,
            ..Default::default()
        });
    }
    work.planning.as_mut().unwrap().scopes = Some(scopes);
    assert_eq!(
        quota_view(&work)
            .unwrap()
            .consumed
            .delegation_depth
            .as_deref(),
        Some(&10001)
    );
    for variant in 0..4 {
        let mut invalid = work.clone();
        let scopes = invalid.planning.as_mut().unwrap().scopes.as_mut().unwrap();
        match variant {
            0 => scopes[1].parent = "s-10000".into(),
            1 => scopes[1].parent = "missing".into(),
            2 => scopes.push(scopes[0].clone()),
            _ => scopes[0].parent = "s-1".into(),
        }
        assert_eq!(
            quota_view(&invalid).err().unwrap().code(),
            "quota-hierarchy-invalid"
        );
    }
}

/// Reviewer duration is explicit saved data, not a new attempt/response cap.
/// A valid positive witness precedes invalid changes; zero removes the deadline
/// and omission preserves it, without changing calls, reservations or revision.
#[test]
fn quota_review_timeout_is_explicit_atomic_and_zero_removes_deadline() {
    let mut work=fixture();
    let used=quota_view(&work).unwrap().consumed;
    let mut change=request(&work);change.review_timeout_seconds=Some(Box::new(7));
    apply_quota_change(&mut work,&change,"fixture/operator",AT,&policy()).unwrap();
    assert_eq!(quota_view(&work).unwrap().review_timeout_seconds,Some(7));
    assert_eq!(work.planning.as_ref().unwrap().reviewer.as_ref().unwrap().timeout_seconds,7);
    let original=encode_model(&work).unwrap();
    change.review_timeout_seconds=Some(Box::new(-1));
    assert!(apply_quota_change(&mut work,&change,"fixture/operator",AT,&policy()).is_err());
    assert_eq!(encode_model(&work).unwrap(),original);
    change.review_timeout_seconds=None;apply_quota_change(&mut work,&change,"fixture/operator",AT,&policy()).unwrap();
    assert_eq!(quota_view(&work).unwrap().review_timeout_seconds,Some(7));
    change.review_timeout_seconds=Some(Box::new(0));apply_quota_change(&mut work,&change,"fixture/operator",AT,&policy()).unwrap();
    assert_eq!(quota_view(&work).unwrap().review_timeout_seconds,Some(0));
    assert_eq!(quota_view(&work).unwrap().consumed,used);
    assert_eq!(work.revision,1);
    let mut missing=work.clone();missing.planning.as_mut().unwrap().reviewer=None;
    let before=encode_model(&missing).unwrap();let mut change=request(&missing);change.review_timeout_seconds=Some(Box::new(1));
    assert!(apply_quota_change(&mut missing,&change,"fixture/operator",AT,&policy()).is_err());
    assert_eq!(encode_model(&missing).unwrap(),before);
}
