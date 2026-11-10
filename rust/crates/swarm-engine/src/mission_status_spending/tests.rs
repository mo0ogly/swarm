//! Pure spending counterexamples; no Root, SQL, clock or provider IO.
use super::*;
use crate::{IndependentReview, MissionStatusPlanningCall, ModelRoute, ReportedModel, Task};
use std::error::Error;

/// Full retained input fixture with explicit Work identity and empty history.
fn inputs() -> MissionStatusInputs {
    let work = Work {
        id: "work".into(),
        ..Work::default()
    };
    MissionStatusInputs {
        work,
        agents: vec![],
        events: vec![],
        visit: None,
        planning_calls: vec![],
        exchanges: vec![],
    }
}

/// Retained agent fixture; unset fields keep defaults and grant no permission.
fn agent(id: &str, task: &str, attempt: &str) -> Agent {
    Agent {
        id: id.into(),
        work_id: "work".into(),
        task_id: task.into(),
        attempt_id: attempt.into(),
        role: "worker".into(),
        status: "completed".into(),
        mode: "pipe".into(),
        ..Agent::default()
    }
}

/// Explicit recorded usage, including genuine zero and missing cost distinctions.
fn usage(input: i64, output: i64, cost: Option<f64>) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: output,
        provider_reported_cost_usd: cost.map(|cost| Box::new(GoFloat(cost))),
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        cached_input_tokens: None,
        source: "provider fixture".into(),
        scope: "call".into(),
        at: "recorded-time".into(),
    }
}

/// Complete raw event fixture; payload stays data until the selected decoder runs.
fn event(kind: &str, payload: &[u8]) -> RecordedEvent {
    RecordedEvent {
        id: b"event".to_vec(),
        work_id: b"work".to_vec(),
        revision: 1,
        kind: kind.as_bytes().to_vec(),
        at: b"recorded-time".to_vec(),
        payload: Some(payload.to_vec()),
    }
}

/// Retained nonreleased planner reservation, not evidence of provider execution.
fn planning(id: &str, scope: &str, at: &str) -> MissionStatusPlanningCall {
    MissionStatusPlanningCall {
        id: id.into(),
        scope_id: scope.into(),
        state: "reserved".into(),
        created_at: at.into(),
        usage: None,
    }
}

#[test]
fn empty_retained_history_has_explicit_arrays_without_measured_calls() {
    let value = mission_spending(&inputs(), "caller-observation").unwrap();
    let json = serde_json::to_value(&value).unwrap();
    assert_eq!(json["calls"], serde_json::json!([]));
    assert_eq!(json["rows"], serde_json::json!([]));
    assert_eq!(json["attempts"], serde_json::json!([]));
    assert_eq!(value.controls, 0);
    assert_eq!(value.retries, 0);
    assert_eq!(value.observed_at, "caller-observation");
}

#[test]
fn absent_usage_and_reported_zero_are_distinct_without_cross_work_leak() {
    let mut input = inputs();
    let absent = agent("absent", "missing", "one");
    let mut zero = agent("zero", "zero", "one");
    zero.usage = Some(Box::new(usage(0, 0, Some(0.0))));
    let mut excluded = agent("excluded", "missing", "other");
    excluded.work_id = "different".into();
    let mut legacy = agent("legacy", "legacy", "one");
    legacy.work_id.clear();
    legacy.usage = Some(Box::new(usage(0, 0, None)));
    input.agents = vec![absent, zero, excluded, legacy];
    let value = mission_spending(&input, "now").unwrap();
    let rows = value.rows.as_ref().unwrap();
    let missing = rows.iter().find(|row| row.id == "missing").unwrap();
    assert_eq!(missing.calls, 1);
    assert_eq!(missing.missing_usage, 1);
    assert_eq!(missing.cost.attempts_without_cost, 1);
    let zero = rows.iter().find(|row| row.id == "zero").unwrap();
    assert_eq!(zero.missing_usage, 0);
    assert_eq!(zero.cost.attempts_with_cost, 1);
    assert_eq!(zero.cost.reported_usd.0, 0.0);
    let legacy = rows.iter().find(|row| row.id == "legacy").unwrap();
    assert_eq!(legacy.missing_usage, 0);
    assert_eq!(legacy.cost.attempts_without_cost, 1);
    assert_eq!(value.attempts.as_ref().unwrap().len(), 3);
}

#[test]
fn task_retries_use_distinct_attempts_or_explicit_links_not_agent_count() {
    let mut input = inputs();
    let t = Task {
        id: "task".into(),
        title: "Actual task".into(),
        previous_reviews: Some(vec![
            IndependentReview::default(),
            IndependentReview::default(),
        ]),
        independent_review: Some(Box::new(IndependentReview::default())),
        ..Task::default()
    };
    input.work.tasks = Some(vec![t]);
    let a = agent("one", "task", "same");
    let mut b = agent("two", "task", "same");
    b.previous = "one".into();
    let mut c = agent("three", "task", "new");
    c.previous = "two".into();
    input.agents = vec![
        a,
        b,
        c,
        agent("x", "other", "a"),
        agent("y", "other", "b"),
        agent("z", "other", "c"),
    ];
    let value = mission_spending(&input, "now").unwrap();
    let rows = value.rows.as_ref().unwrap();
    let row = rows.iter().find(|row| row.id == "task").unwrap();
    assert_eq!(row.task_retries, 2);
    assert_eq!(row.reviews, 3);
    assert_eq!(row.calls, 3);
    assert_eq!(value.retries, 4);
}

#[test]
fn duration_zero_absence_reversal_fraction_and_go_saturation_are_distinct() {
    let at = "2026-10-23T12:00:00Z";
    assert_eq!(recorded_process_duration(at, at), Some(0));
    assert_eq!(recorded_process_duration(at, ""), None);
    assert_eq!(recorded_process_duration("invalid", at), None);
    assert_eq!(recorded_process_duration(at, "2026-10-23T11:59:59Z"), None);
    assert_eq!(
        recorded_process_duration(at, "2026-10-23T12:00:00.000999999Z"),
        Some(0)
    );
    assert_eq!(
        recorded_process_duration("0001-01-01T00:00:00Z", at),
        Some(i64::MAX / 1_000_000)
    );
    let mut input = inputs();
    let mut known = agent("known", "task", "one");
    known.started = at.into();
    known.ended = at.into();
    input.agents = vec![known, agent("unknown", "task", "two")];
    let value = mission_spending(&input, "now").unwrap();
    let row = &value.rows.as_ref().unwrap()[0];
    assert_eq!(row.duration_ms.as_deref(), Some(&0));
    assert_eq!(row.missing_durations, 1);
}

#[test]
fn metrics_schema_mode_degradation_and_pending_keep_go_distinctions() {
    let work = inputs().work;
    let mut a = agent("a", "task", "attempt");
    a.progress.metrics_version = 1;
    a.progress.tool_calls = 4;
    a.progress.tool_reads = 2;
    let value = attempt_ledgers(&work, &[a.clone()]).unwrap();
    assert!(value[0].measured);
    assert_eq!(value[0].measurement, "observed");
    a.progress.pending_tools = 1;
    assert_eq!(
        attempt_ledgers(&work, &[a.clone()]).unwrap()[0].measurement,
        "partial"
    );
    let mut input = inputs();
    input.agents = vec![a.clone()];
    assert_eq!(
        mission_spending(&input, "now")
            .unwrap()
            .rows
            .as_ref()
            .unwrap()[0]
            .unknown_tools,
        0
    );
    a.progress.degraded = "incomplete source".into();
    input.agents = vec![a.clone()];
    assert_eq!(
        mission_spending(&input, "now")
            .unwrap()
            .rows
            .as_ref()
            .unwrap()[0]
            .unknown_tools,
        1
    );
    a.mode = "terminal".into();
    let value = attempt_ledgers(&work, &[a.clone()]).unwrap();
    assert!(!value[0].measured);
    assert_eq!(value[0].measurement, "unknown");
    assert_eq!(value[0].reads, 0);
    assert_eq!(value[0].tools, 4);
    a.mode = "pipe".into();
    a.progress.metrics_version = 0;
    assert!(!attempt_ledgers(&work, &[a]).unwrap()[0].measured);
}

#[test]
fn process_completion_does_not_imply_acceptance_and_recorded_acceptance_is_separate() {
    let mut input = inputs();
    let t = Task {
        id: "task".into(),
        status: "blocked".into(),
        ..Task::default()
    };
    input.work.tasks = Some(vec![t]);
    input.agents = vec![agent("completed", "task", "attempt")];
    let value = mission_spending(&input, "now").unwrap();
    let row = &value.attempts.as_ref().unwrap()[0];
    assert_eq!(row.process_state, "completed");
    assert!(!row.accepted);
    input.work.tasks.as_mut().unwrap()[0].status = "accepted".into();
    input.agents[0].status = "failed".into();
    let value = mission_spending(&input, "now").unwrap();
    let row = &value.attempts.as_ref().unwrap()[0];
    assert_eq!(row.process_state, "failed");
    assert!(row.accepted);
}

#[test]
fn stored_worker_models_and_cache_zero_survive_without_planner_model_invention() {
    let mut input = inputs();
    let mut a = agent("a", "task", "attempt");
    let route = ModelRoute {
        model: "requested historical".into(),
        ..ModelRoute::default()
    };
    a.model_route = Some(Box::new(route));
    a.reported_model = Some(Box::new(ReportedModel {
        model: "observed historical".into(),
        source: "provider".into(),
        at: "stored".into(),
    }));
    let mut u = usage(1, 2, None);
    u.cache_read_input_tokens = Some(Box::new(0));
    a.usage = Some(Box::new(u));
    input.agents = vec![a];
    input.planning_calls = vec![planning("planner", "root", "stored")];
    let value = mission_spending(&input, "now").unwrap();
    let calls = value.calls.as_ref().unwrap();
    let worker = calls.iter().find(|call| call.kind == "worker").unwrap();
    assert_eq!(worker.requested_model, "requested historical");
    assert_eq!(worker.observed_model, "observed historical");
    let json = serde_json::to_value(worker).unwrap();
    assert_eq!(json["usage"]["cache_read_input_tokens"], 0);
    assert!(json["usage"].get("cached_input_tokens").is_none());
    let planner = calls.iter().find(|call| call.kind == "planner").unwrap();
    assert!(
        planner.provider.is_empty()
            && planner.requested_model.is_empty()
            && planner.observed_model.is_empty()
    );
    assert_eq!(planner.record_type, "planning_reservation");
    assert!(planner.usage.is_none());
}

#[test]
fn rows_calls_and_attempts_follow_distinct_go_sort_orders() {
    let mut input = inputs();
    let mut b = agent("b", "z-task", "2");
    b.started = "same".into();
    let mut a = agent("a", "a-task", "1");
    a.started = "same".into();
    input.agents = vec![b, a];
    input.planning_calls = vec![planning("z", "reviewer", "same"), planning("a", "root", "")];
    let value = mission_spending(&input, "now").unwrap();
    assert_eq!(
        value
            .rows
            .as_ref()
            .unwrap()
            .iter()
            .map(|row| row.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["planner", "reviewer", "worker", "worker"]
    );
    assert_eq!(
        value
            .calls
            .as_ref()
            .unwrap()
            .iter()
            .map(|call| call.id.as_str())
            .collect::<Vec<_>>(),
        vec!["planning/a", "agent/a", "agent/b", "planning/z"]
    );
    assert_eq!(
        value
            .attempts
            .as_ref()
            .unwrap()
            .iter()
            .map(|attempt| attempt.task.as_str())
            .collect::<Vec<_>>(),
        vec!["a-task", "z-task"]
    );
}

#[test]
fn executed_control_counts_respect_go_null_case_merge_and_selected_events() {
    let mut input = inputs();
    input.events = vec![
        event("task.auto-validation", br#"{"controls":[{"executed":true},{"executed":false},null,{"ExEcUtEd":true,"unknown":"ignored"}]}"#),
        event("managed.integrated", br#"{"controls":{"a":[{"executed":true,"passed":false}],"b":null}}"#),
        event("unrelated", b"not JSON"), event("task.auto-validation", b"null")];
    let mut foreign = event("task.auto-validation", b"not JSON");
    foreign.work_id = b"other".to_vec();
    input.events.push(foreign);
    let value = mission_spending(&input, "now").unwrap();
    assert_eq!(value.controls, 3);
    input.events = vec![
        event("task.auto-validation", br#"{"controls":null}"#),
        event("managed.integrated", b"{}"),
    ];
    assert_eq!(mission_spending(&input, "now").unwrap().controls, 0);
}

#[test]
fn malformed_selected_control_preserves_safe_cause_and_valid_recovery() {
    let mut input = inputs();
    let good = event(
        "task.auto-validation",
        br#"{"controls":[{"executed":true}]}"#,
    );
    input.events = vec![good.clone()];
    assert_eq!(mission_spending(&input, "now").unwrap().controls, 1);
    for (kind, raw) in [
        (
            "task.auto-validation",
            br#"{"controls":[{"executed":"PRIVATE-CONTROL-INPUT"}]}"#.as_slice(),
        ),
        (
            "managed.integrated",
            br#"{"controls":{"task":{}}}"#.as_slice(),
        ),
        ("task.auto-validation", b"".as_slice()),
    ] {
        input.events = vec![event(kind, raw)];
        let before = input.events.clone();
        let error = mission_spending(&input, "now")
            .err()
            .expect("invalid selected payload must refuse");
        assert!(matches!(error, E::Encode(_)));
        assert!(!error.to_string().contains("PRIVATE-CONTROL-INPUT"));
        assert!(error.source().is_some());
        assert!(input.events == before);
    }
    input.events = vec![good];
    assert_eq!(mission_spending(&input, "now").unwrap().controls, 1);
}

#[test]
fn integer_and_float_overflow_refuse_without_input_mutation_then_recover() {
    let mut input = inputs();
    let mut a = agent("a", "task", "1");
    a.progress.tool_calls = i64::MAX;
    let mut b = agent("b", "task", "2");
    input.agents = vec![a, b.clone()];
    assert_eq!(
        mission_spending(&input, "now")
            .unwrap()
            .rows
            .as_ref()
            .unwrap()[0]
            .tools,
        i64::MAX
    );
    input.agents[1].progress.tool_calls = 1;
    let before = input.agents.clone();
    let error = mission_spending(&input, "now").err().unwrap();
    assert_eq!(error.to_string(), "mission-spending-integer-overflow");
    assert!(input.agents == before);
    input.agents[1].progress.tool_calls = 0;
    assert!(mission_spending(&input, "now").is_ok());
    input.agents[0].usage = Some(Box::new(usage(0, 0, Some(f64::MAX))));
    b.usage = Some(Box::new(usage(0, 0, Some(f64::MAX))));
    input.agents[1] = b;
    let error = mission_spending(&input, "now").err().unwrap();
    assert_eq!(error.to_string(), "mission-spending-cost-overflow");
    input.agents[1]
        .usage
        .as_mut()
        .unwrap()
        .provider_reported_cost_usd = Some(Box::new(GoFloat(0.0)));
    assert!(mission_spending(&input, "now").is_ok());
}

#[test]
fn absent_attempt_ids_use_explicit_retry_links_and_partial_usage_stays_missing_cost() {
    let mut input = inputs();
    let mut a = agent("a", "task", "");
    a.previous = "legacy-source".into();
    a.usage = Some(Box::new(usage(7, 8, None)));
    input.agents = vec![a];
    let value = mission_spending(&input, "now").unwrap();
    let row = &value.rows.as_ref().unwrap()[0];
    assert_eq!(row.task_retries, 1);
    assert_eq!(row.input, 7);
    assert_eq!(row.missing_usage, 0);
    assert_eq!(row.cost.attempts_without_cost, 1);
    assert!(!value.attempts.as_ref().unwrap()[0].missing_usage);
}

/// Historical Go timestamps permit comma fractions. A valid canonical witness
/// precedes the noncanonical spelling; invalid and reversed remain unknown.
#[test]
fn mission_status_duration_accepts_actual_go_timestamp_grammar() {
    assert_eq!(
        recorded_process_duration("2026-10-23T00:00:00.1Z", "2026-10-23T00:00:00.2Z"),
        Some(100)
    );
    assert_eq!(
        recorded_process_duration("2026-10-23T00:00:00,1Z", "2026-10-23T00:00:00,2Z"),
        Some(100)
    );
    assert_eq!(
        recorded_process_duration("bad-private-stamp", "2026-10-23T00:00:00,2Z"),
        None
    );
    assert_eq!(
        recorded_process_duration("2026-10-23T00:00:00,2Z", "2026-10-23T00:00:00,1Z"),
        None
    );
    assert_eq!(
        recorded_process_duration("2026-10-23T00:00:00,1Z", "2026-10-23T00:00:00,2Z"),
        Some(100)
    );
}
