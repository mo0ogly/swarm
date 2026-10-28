//! Pure declared-event guard fixtures, no process, actual tools or authorization.
use std::collections::BTreeMap;
use swarm_engine::{
    AgentGuardSnapshot, AgentGuardStop, AgentGuardVisibility, AgentToolGuard, RunLimits,
    SettingCap, SettingCount,
};
/// Explicit fixture limits, not installation defaults or a policy fallback.
fn limits() -> RunLimits {
    RunLimits {
        silence_seconds: 180,
        tool_seconds: 300,
        max_tool_calls: 100,
        max_repeated_calls: 4,
        max_consecutive_errors: 3,
        observation_mode: 0,
    }
}
/// Declared fixture snapshot; production construction belongs to Root admission.
fn snapshot(multiplier: Option<i64>) -> AgentGuardSnapshot {
    AgentGuardSnapshot {
        schema_version: 1,
        settings_revision: "1".into(),
        lookback_multiplier: SettingCap(multiplier.map(SettingCount)),
        extensions: BTreeMap::new(),
    }
}
/// Valid frozen values work; incomplete/negative limits and unknown snapshots
/// refuse instead of inheriting current settings or interpreting absent facts.
#[test]
fn guard_requires_frozen_valid_limits_and_explicit_policy() {
    assert!(AgentToolGuard::new(limits(), &snapshot(Some(4))).is_ok());
    let mut invalid = limits();
    invalid.max_tool_calls = 0;
    assert_eq!(
        AgentToolGuard::new(invalid, &snapshot(Some(4)))
            .unwrap_err()
            .code(),
        "agent-guard-frozen-limits-invalid"
    );
    let mut future = snapshot(Some(4));
    future.schema_version = 2;
    assert!(AgentToolGuard::new(limits(), &future).is_err());
    assert!(AgentToolGuard::new(limits(), &snapshot(Some(0))).is_err());
    let mut large = limits();
    large.max_tool_calls = i64::MAX;
    large.max_repeated_calls = i64::MAX;
    large.silence_seconds = i64::MAX;
    assert!(AgentToolGuard::new(large, &snapshot(Some(i64::MAX))).is_ok());
}
/// JSON ordering/float spelling/escaping and duplicate object keys normalize as
/// Go any. A changed name/input under one call ID refuses without time/counters.
#[test]
fn guard_canonical_dedup_and_conflicting_ids_are_atomic() {
    let mut g = AgentToolGuard::new(limits(), &snapshot(Some(4))).unwrap();
    g.call(0, "A", "Read", br#"{"n":1,"s":"<x>","dup":0,"dup":2}"#)
        .unwrap();
    g.call(
        1,
        "A",
        "Read",
        br#"{"dup":2.0,"s":"\u003cx\u003e","n":1.0}"#,
    )
    .unwrap();
    assert_eq!(g.observation(1).unwrap().calls, 1);
    g.call(2, "B", "Read", br#"{"n":1e0,"dup":2,"s":"<x>"}"#)
        .unwrap();
    g.result(3, "A", false).unwrap();
    g.result(4, "A", false).unwrap();
    let before = g.observation(4).unwrap();
    assert_eq!(
        (before.calls, before.results, before.total_repeats),
        (2, 1, 1)
    );
    assert_eq!(
        g.call(5, "A", "Write", b"null").unwrap_err().code(),
        "agent-guard-call-id-conflict"
    );
    assert_eq!(
        g.result(5, "A", true).unwrap_err().code(),
        "agent-guard-result-id-conflict"
    );
    assert_eq!(g.observation(4).unwrap(), before);
}
/// The last call within the total budget can deliver its result. Silence/tool
/// time only set observations; vitality stays unknown, with no temporal stop.
#[test]
fn guard_last_authorized_result_and_duration_observations_do_not_fake_termination() {
    let mut l = limits();
    l.max_tool_calls = 1;
    let mut g = AgentToolGuard::new(l, &snapshot(Some(4))).unwrap();
    g.call(0, "A", "Read", b"null").unwrap();
    let waiting = g.observation(1_000_000).unwrap();
    assert_eq!(waiting.stop_reason, None);
    assert!(waiting.silence_threshold_exceeded);
    assert!(waiting.tool_time_threshold_exceeded);
    assert_eq!(waiting.provider_vitality, "unknown");
    assert_eq!(waiting.tool_state, "waiting");
    g.result(1_000_001, "A", false).unwrap();
    assert_eq!(
        g.observation(1_000_001).unwrap().stop_reason,
        Some(AgentGuardStop::ToolCalls)
    );
    g.call(1_000_002, "B", "Write", b"null").unwrap();
    let after = g.observation(1_000_002).unwrap();
    assert_eq!(after.calls, 2);
    assert_eq!(after.stop_reason, Some(AgentGuardStop::ToolCalls));
}
/// Isolate interleaved identical failures from consecutive errors/repetitions.
/// The explicit multiplier/null policy changes only this detector's history.
fn interleaved(multiplier: Option<i64>, gap: usize) -> swarm_engine::AgentGuardObservation {
    let mut l = limits();
    l.max_repeated_calls = 2;
    let mut g = AgentToolGuard::new(l, &snapshot(multiplier)).unwrap();
    g.call(0, "A1", "Read", br#"{"path":"same"}"#).unwrap();
    g.result(1, "A1", true).unwrap();
    let mut time = 2;
    for i in 0..gap {
        let id = format!("B{i}");
        let input = format!("{{\"n\":{i}}}");
        g.call(time, &id, "Write", input.as_bytes()).unwrap();
        time += 1;
        g.result(time, &id, false).unwrap();
        time += 1;
    }
    g.call(time, "A2", "Read", br#"{"path":"same"}"#).unwrap();
    time += 1;
    g.result(time, "A2", true).unwrap();
    g.observation(time).unwrap()
}
#[test]
fn guard_interleaved_window_is_explicit_and_unlimited_preserves_history() {
    assert_eq!(interleaved(Some(1), 1).stop_reason, None);
    assert_eq!(
        interleaved(Some(4), 1).stop_reason,
        Some(AgentGuardStop::InterleavedFailures)
    );
    assert_eq!(interleaved(Some(4), 10).stop_reason, None);
    assert_eq!(
        interleaved(None, 10).stop_reason,
        Some(AgentGuardStop::InterleavedFailures)
    );
}
/// Observation mode changes enforcement, not cumulative facts. A visibility gap
/// invalidates streak assumptions but never turns unobserved pending tools idle.
#[test]
fn guard_visibility_and_observation_keep_cumulative_facts_separate() {
    let mut g = AgentToolGuard::new(limits(), &snapshot(Some(4))).unwrap();
    g.call(0, "A", "Read", b"{}").unwrap();
    g.result(1, "A", true).unwrap();
    g.call(2, "B", "Read", b"{}").unwrap();
    g.lose_visibility(3, AgentGuardVisibility::MissingEvents)
        .unwrap();
    let loss = g.observation(1_000_000).unwrap();
    assert_eq!(
        (loss.calls, loss.total_errors, loss.total_repeats),
        (2, 1, 1)
    );
    assert_eq!(
        (loss.consecutive_errors, loss.consecutive_identical),
        (0, 0)
    );
    assert_eq!(loss.tool_state, "unknown");
    assert!(!loss.tool_time_threshold_exceeded);
    assert_eq!(loss.provider_vitality, "unknown");
    let mut l = limits();
    l.observation_mode = 1;
    l.max_tool_calls = 1;
    l.max_repeated_calls = 1;
    l.max_consecutive_errors = 1;
    let mut o = AgentToolGuard::new(l, &snapshot(Some(4))).unwrap();
    o.call(0, "A", "Read", b"{}").unwrap();
    o.result(1, "A", true).unwrap();
    o.call(2, "B", "Read", b"{}").unwrap();
    o.result(3, "B", true).unwrap();
    let report = o.observation(3).unwrap();
    assert_eq!(
        (report.calls, report.total_errors, report.total_repeats),
        (2, 2, 1)
    );
    assert_eq!(report.stop_reason, None);
}
/// No invalid event mutates counters. The complete JSON must be float64-valid,
/// including overwritten duplicate values; errors never echo private arguments.
#[test]
fn guard_invalid_input_clock_and_unmatched_result_refuse_atomically() {
    for input in [b"PRIVATE".as_slice(), b"1e400", br#"{"n":1e400,"n":0}"#] {
        let mut g = AgentToolGuard::new(limits(), &snapshot(Some(4))).unwrap();
        g.call(0, "control", "Read", b"null").unwrap();
        let before = g.observation(0).unwrap();
        let e = g
            .call(1, "fault", "Read", input)
            .expect_err("invalid Go number accepted");
        assert!(!format!("{e:?}").contains("PRIVATE"));
        assert_eq!(g.observation(0).unwrap(), before);
    }
    let mut g = AgentToolGuard::new(limits(), &snapshot(Some(4))).unwrap();
    g.call(2, "A", "Read", b"null").unwrap();
    let before = g.observation(2).unwrap();
    assert!(g.call(1, "B", "Read", b"null").is_err());
    assert!(g.result(3, "unknown", true).is_err());
    assert_eq!(g.observation(2).unwrap(), before);
}
/// Deep input hashes without recursive Value construction/drop or an invented
/// depth policy. The shared Go representation limit still refuses >10000 levels.
#[test]
fn guard_deep_input_normalization_has_no_recursive_value_drop() {
    let mut g = AgentToolGuard::new(limits(), &snapshot(Some(4))).unwrap();
    let a = format!("{}1{}", "[".repeat(512), "]".repeat(512));
    let b = format!("{}1.0{}", "[".repeat(512), "]".repeat(512));
    g.call(0, "A", "Read", a.as_bytes()).unwrap();
    g.call(1, "A", "Read", b.as_bytes()).unwrap();
    assert_eq!(g.observation(1).unwrap().calls, 1);
    let invalid = format!("{}null{}", "[".repeat(10001), "]".repeat(10001));
    assert!(g.call(2, "B", "Read", invalid.as_bytes()).is_err());
    assert_eq!(g.observation(1).unwrap().calls, 1);
}
