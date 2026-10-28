//! Supervisor alert conditions (LangWatch L5), computed from engine-recorded
//! state only: agent status, diagnostic and progress marks, declared usage
//! state, task blockers, and (settings v34) the gap between declared and
//! telemetry output tokens. Every threshold is a saved setting: the engine's
//! own marks, or `agent_usage_mismatch_percent` for the gap rule. Durable
//! history (fired / resolved) lives in the console store, see
//! [`crate::TelemetryStore::sync_alerts`].

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type Rule = (&'static str, &'static str);
const AGENT_FAILED: Rule = ("agent-failed", "critical");
const AGENT_ERROR_LIMIT: Rule = ("agent-error-limit", "critical");
const AGENT_TIME_THRESHOLD: Rule = ("agent-time-threshold", "warning");
const AGENT_TOOL_WAIT: Rule = ("agent-tool-wait", "warning");
const AGENT_USAGE_INVALID: Rule = ("agent-usage-invalid", "warning");
const TASK_BLOCKED: Rule = ("task-blocked", "warning");
const AGENT_USAGE_MISMATCH: Rule = ("agent-usage-mismatch", "warning");
const WORK_BUDGET_THRESHOLD: Rule = ("work-budget-threshold", "warning");
const WORK_BUDGET_OVERSPENT: Rule = ("work-budget-overspent", "critical");

/// Fixed rule set `(id, level)`; ids are stable public codes.
pub const ALERT_RULES: [Rule; 9] = [
    AGENT_FAILED,
    AGENT_ERROR_LIMIT,
    AGENT_TIME_THRESHOLD,
    AGENT_TOOL_WAIT,
    AGENT_USAGE_INVALID,
    TASK_BLOCKED,
    AGENT_USAGE_MISMATCH,
    WORK_BUDGET_THRESHOLD,
    WORK_BUDGET_OVERSPENT,
];

/// One condition currently true. `subject` is an agent id, `work/task` for
/// `task-blocked`, or `budget:<work>` for the budget rules; `occurred` is the engine time when known (agent `ended`),
/// else empty (the store then uses detection time).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AlertCondition {
    pub rule: &'static str,
    pub level: &'static str,
    pub subject: String,
    pub work_id: String,
    pub occurred: String,
}

fn condition((rule, level): Rule, subject: String, work_id: &str, occurred: &str) -> AlertCondition {
    AlertCondition { rule, level, subject, work_id: work_id.to_owned(), occurred: occurred.to_owned() }
}

fn task_done(status: &str) -> bool {
    matches!(status, "accepted" | "waived")
}

/// Latest attempt of a task that is neither accepted nor waived (see
/// [`alert_conditions`] for the `started` comparison rules).
fn is_current(a: &crate::Agent, works: &[crate::Work], agents: &[crate::Agent]) -> bool {
    let started = |a: &crate::Agent| swarm_wire_json::go_rfc3339::parse(&a.started).ok();
    let superseded = started(a).is_some_and(|mine| {
        agents.iter().any(|b| b.id != a.id && b.work_id == a.work_id && b.task_id == a.task_id && started(b).is_some_and(|t| t > mine))
    });
    let done = works
        .iter()
        .filter(|w| w.id == a.work_id)
        .flat_map(|w| w.tasks.iter().flatten())
        .any(|t| t.id == a.task_id && task_done(&t.status));
    !superseded && !done
}

/// Conditions true now for these Works and their agents (pure, no IO).
///
/// Terminal agent rules (`agent-failed`, `agent-error-limit`,
/// `agent-usage-invalid`) apply only to the latest attempt of a task that is
/// not accepted or waived: a retried task (a newer agent on the same task) or
/// an accepted one clears them, so a failed attempt does not stay alerted for
/// as long as its record exists. "Latest" compares the recorded `started`
/// times; an agent whose time does not parse is kept (never hidden by a
/// guess), and an agent whose task is not found in the Works is judged alone.
/// Running-agent rules read the engine's progress marks on active agents only.
/// Usage state is read from `native_provider_facts.usage_state`
/// (`invalid`/`incomplete`); an undecodable facts document is not guessed.
/// Duplicate (rule, subject) pairs are not produced.
pub fn alert_conditions(works: &[crate::Work], agents: &[crate::Agent]) -> Vec<AlertCondition> {
    let mut out = Vec::new();
    for a in agents {
        let current = is_current(a, works, agents);
        if current && a.status == "failed" {
            out.push(condition(AGENT_FAILED, a.id.clone(), &a.work_id, &a.ended));
        }
        if current && a.diagnostic.limit_reached {
            out.push(condition(AGENT_ERROR_LIMIT, a.id.clone(), &a.work_id, ""));
        }
        let active = matches!(a.status.as_str(), "queued" | "starting" | "running" | "stopping");
        if active && a.progress.time_threshold_exceeded {
            out.push(condition(AGENT_TIME_THRESHOLD, a.id.clone(), &a.work_id, ""));
        }
        if active && a.progress.tool_time_threshold_exceeded {
            out.push(condition(AGENT_TOOL_WAIT, a.id.clone(), &a.work_id, ""));
        }
        let usage = a
            .native_provider_facts
            .as_ref()
            .and_then(|raw| serde_json::from_str::<Value>(raw.0.get()).ok())
            .and_then(|v| v.get("usage_state").and_then(Value::as_str).map(str::to_owned));
        if current && matches!(usage.as_deref(), Some("invalid" | "incomplete")) {
            out.push(condition(AGENT_USAGE_INVALID, a.id.clone(), &a.work_id, ""));
        }
    }
    for w in works {
        for t in w.tasks.iter().flatten() {
            if !task_done(&t.status) && (t.status == "blocked" || !t.blocker.is_empty()) {
                out.push(condition(TASK_BLOCKED, format!("{}/{}", w.id, t.id), &w.id, ""));
            }
        }
    }
    let mut seen = BTreeSet::new();
    out.retain(|c| seen.insert((c.rule, c.subject.clone())));
    out
}

/// `agent-usage-mismatch` (settings v34): for each finished current attempt
/// whose declared usage is observed and that has telemetry calls, compare the
/// declared output tokens with `telemetry_output` (sum of its calls' output
/// tokens, from [`crate::TelemetryStore::output_tokens_by_agent`]); the
/// condition holds when the gap exceeds `percent` of the declared value. Two
/// unauthenticated declarations: the alert says they differ, not which is
/// right (often telemetry lost at process exit).
pub fn usage_mismatch_conditions(
    works: &[crate::Work],
    agents: &[crate::Agent],
    telemetry_output: &BTreeMap<String, u64>,
    percent: i64,
) -> Vec<AlertCondition> {
    let mut out = Vec::new();
    for a in agents {
        let active = matches!(a.status.as_str(), "queued" | "starting" | "running" | "stopping");
        let Some(usage) = a.usage.as_ref() else { continue };
        let Some(&telemetry) = telemetry_output.get(&a.id) else { continue };
        let invalid = a
            .native_provider_facts
            .as_ref()
            .and_then(|raw| serde_json::from_str::<Value>(raw.0.get()).ok())
            .and_then(|v| v.get("usage_state").and_then(Value::as_str).map(str::to_owned))
            .is_some_and(|s| s != "observed");
        if active || invalid || !is_current(a, works, agents) {
            continue;
        }
        let declared = u64::try_from(usage.output_tokens).unwrap_or(0);
        let gap = declared.abs_diff(telemetry);
        if u128::from(gap) * 100 > u128::from(declared) * u128::try_from(percent.max(0)).unwrap_or(0) {
            out.push(condition(AGENT_USAGE_MISMATCH, a.id.clone(), &a.work_id, &a.ended));
        }
    }
    out
}

/// One Work's budget figures for the L7 rules (settings v35), read by
/// [`crate::RootStore::work_budget_state`]. All amounts are USD.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkBudgetState {
    pub work_id: String,
    /// Saved limit; zero is the unlimited contract (no rule applies).
    pub limit_usd: f64,
    /// Saved per-launch reserve, for display.
    pub reserve_per_launch_usd: f64,
    /// Engine committed estimates: active agent, assist, preparation and
    /// planning ledgers (the launch admission figure).
    pub committed_usd: f64,
    /// Sum of provider-declared worker costs (unauthenticated declarations).
    pub reported_usd: f64,
    /// Attempts without a declared cost: `reported_usd` is then partial.
    pub attempts_without_cost: i64,
}

/// Budget rules (LangWatch L7) for Works with a positive limit:
/// `work-budget-overspent` (critical) when the declared cost exceeds the
/// limit — the after-the-fact gap admission cannot prevent, since launches
/// are admitted on estimates — else `work-budget-threshold` (warning) when
/// the higher of committed estimates and declared cost reaches
/// `warning_percent` of the limit (`None` = warning off). At most one rule
/// per Work; subject `budget:<work>`. Pure, no IO.
pub fn budget_conditions(states: &[WorkBudgetState], warning_percent: Option<i64>) -> Vec<AlertCondition> {
    let mut out = Vec::new();
    for b in states.iter().filter(|b| b.limit_usd > 0.0) {
        let subject = format!("budget:{}", b.work_id);
        if b.reported_usd > b.limit_usd {
            out.push(condition(WORK_BUDGET_OVERSPENT, subject, &b.work_id, ""));
        } else if let Some(p) = warning_percent {
            if b.committed_usd.max(b.reported_usd) * 100.0 >= b.limit_usd * p as f64 {
                out.push(condition(WORK_BUDGET_THRESHOLD, subject, &b.work_id, ""));
            }
        }
    }
    out
}

/// Every subject that exists now (agent ids, `work/task` and `budget:<work>`), so the store can
/// tell a cleared condition from a subject that disappeared
/// (see [`crate::TelemetryStore::sync_alerts`]).
pub fn alert_subjects(works: &[crate::Work], agents: &[crate::Agent]) -> BTreeSet<String> {
    agents
        .iter()
        .map(|a| a.id.clone())
        .chain(works.iter().flat_map(|w| w.tasks.iter().flatten().map(move |t| format!("{}/{}", w.id, t.id))))
        .chain(works.iter().map(|w| format!("budget:{}", w.id)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_rules_warn_at_the_saved_share_and_flag_overspend() {
        let state = |work: &str, limit: f64, committed: f64, reported: f64| WorkBudgetState {
            work_id: work.into(), limit_usd: limit, reserve_per_launch_usd: 0.0, committed_usd: committed, reported_usd: reported, attempts_without_cost: 0,
        };
        let rules = |states: &[WorkBudgetState], p| budget_conditions(states, p).into_iter().map(|c| (c.rule, c.subject)).collect::<Vec<_>>();
        // Witness: below the share, nothing.
        assert!(rules(&[state("w", 10.0, 7.9, 1.0)], Some(80)).is_empty());
        assert_eq!(rules(&[state("w", 10.0, 8.0, 1.0)], Some(80)), [("work-budget-threshold", "budget:w".to_owned())], "committed at exactly 80 %");
        assert_eq!(rules(&[state("w", 10.0, 0.0, 8.5)], Some(80)), [("work-budget-threshold", "budget:w".to_owned())], "declared cost counts too");
        assert_eq!(rules(&[state("w", 10.0, 3.0, 10.01)], Some(80)), [("work-budget-overspent", "budget:w".to_owned())], "overspend replaces the warning");
        assert_eq!(rules(&[state("w", 10.0, 3.0, 10.01)], None), [("work-budget-overspent", "budget:w".to_owned())], "overspend does not need the warning setting");
        assert!(rules(&[state("w", 10.0, 9.9, 9.9)], None).is_empty(), "warning off");
        assert!(rules(&[state("w", 0.0, 50.0, 50.0)], Some(80)).is_empty(), "zero limit is unlimited");
        let works = [crate::Work { id: "w".into(), ..Default::default() }];
        assert!(alert_subjects(&works, &[]).contains("budget:w"), "a deleted Work resolves as gone");
    }

    fn agent(id: &str, task: &str, status: &str, started: &str) -> crate::Agent {
        crate::Agent { id: id.into(), work_id: "w".into(), task_id: task.into(), status: status.into(), started: started.into(), ..Default::default() }
    }

    #[test]
    fn rules_read_engine_marks_only() {
        let mut failed = agent("f", "t3", "failed", "2026-10-22T09:00:00Z");
        failed.ended = "2026-10-22T10:00:00Z".into();
        let mut slow = agent("s", "t4", "running", "2026-10-22T09:00:00Z");
        slow.progress.time_threshold_exceeded = true;
        slow.progress.tool_time_threshold_exceeded = true;
        slow.diagnostic.limit_reached = true;
        slow.native_provider_facts = Some(crate::GoRaw(serde_json::value::RawValue::from_string(r#"{"usage_state":"invalid"}"#.into()).unwrap()));
        let mut done = agent("d", "t5", "completed", "2026-10-22T09:00:00Z");
        done.progress.time_threshold_exceeded = true; // inactive: not alerted
        let mut stopped = agent("x", "t6", "interrupted", "2026-10-22T09:00:00Z");
        stopped.diagnostic.limit_reached = true; // terminal mark kept on the latest attempt
        let blocked = crate::Task { id: "t1".into(), blocker: "waiting for access".into(), ..Default::default() };
        let accepted = crate::Task { id: "t2".into(), status: "accepted".into(), blocker: "stale".into(), ..Default::default() };
        let work = crate::Work { id: "w".into(), tasks: Some(vec![blocked, accepted]), ..Default::default() };
        let got: Vec<_> = alert_conditions(&[work], &[failed, slow, done, stopped]).into_iter().map(|c| (c.rule, c.subject, c.level)).collect();
        assert_eq!(got, vec![
            ("agent-failed", "f".to_owned(), "critical"),
            ("agent-error-limit", "s".to_owned(), "critical"),
            ("agent-time-threshold", "s".to_owned(), "warning"),
            ("agent-tool-wait", "s".to_owned(), "warning"),
            ("agent-usage-invalid", "s".to_owned(), "warning"),
            ("agent-error-limit", "x".to_owned(), "critical"),
            ("task-blocked", "w/t1".to_owned(), "warning"),
        ]);
    }

    #[test]
    fn usage_gap_compares_output_tokens_of_finished_current_attempts() {
        let finished = |id: &str, task: &str, out: i64| {
            let mut a = agent(id, task, "completed", "2026-10-22T09:00:00Z");
            a.usage = Some(Box::new(crate::models::Usage { output_tokens: out, ..Default::default() }));
            a
        };
        let close = finished("close", "t1", 100); // telemetry 95: 5 % gap, tolerated at 10 %
        let far = finished("far", "t2", 100); // telemetry 40: 60 % gap
        let mut running = finished("run", "t3", 100);
        running.status = "running".into(); // still active: not judged
        let silent = finished("silent", "t4", 100); // no telemetry call: not judged
        let mut invalid = finished("invalid", "t5", 100);
        invalid.native_provider_facts = Some(crate::GoRaw(serde_json::value::RawValue::from_string(r#"{"usage_state":"invalid"}"#.into()).unwrap()));
        let sums: BTreeMap<String, u64> = [("close", 95), ("far", 40), ("run", 0), ("invalid", 0)].into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
        let agents = [close, far, running, silent, invalid];
        let got: Vec<_> = usage_mismatch_conditions(&[], &agents, &sums, 10).into_iter().map(|c| (c.rule, c.subject)).collect();
        assert_eq!(got, [("agent-usage-mismatch", "far".to_owned())]);
        assert_eq!(usage_mismatch_conditions(&[], &agents, &sums, 0).len(), 2, "zero tolerance: any gap");
    }

    #[test]
    fn retried_or_accepted_tasks_clear_terminal_alerts() {
        let old = agent("old", "t1", "failed", "2026-10-22T09:00:00Z");
        let retry = agent("new", "t1", "running", "2026-10-22T09:30:00.5Z");
        let unparsed = agent("odd", "t1", "failed", "not-a-time"); // kept: no guess
        let accepted_fail = agent("acc", "t2", "failed", "2026-10-22T09:00:00Z");
        let t1 = crate::Task { id: "t1".into(), status: "running".into(), ..Default::default() };
        let t2 = crate::Task { id: "t2".into(), status: "accepted".into(), ..Default::default() };
        let work = crate::Work { id: "w".into(), tasks: Some(vec![t1, t2]), ..Default::default() };
        let agents = [old, retry, unparsed, accepted_fail];
        let got: Vec<_> = alert_conditions(std::slice::from_ref(&work), &agents).into_iter().map(|c| (c.rule, c.subject)).collect();
        assert_eq!(got, vec![("agent-failed", "odd".to_owned())]);
        let subjects = alert_subjects(&[work], &agents);
        assert!(subjects.contains("old") && subjects.contains("w/t2") && !subjects.contains("w/t9"));
    }
}
