//! Failure-cause groups (LangWatch L11 "topics", deterministic): failed or
//! limit-stopped attempts grouped by the most specific cause the engine
//! recorded. No model, no text similarity, no prompt or output content: the
//! key is built from engine fields only, so the same cause always lands in
//! the same group and an unexplained failure stays visibly `unclassified`.
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// One group of attempts sharing a recorded cause.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FailureGroup {
    /// Stable key: `<source>:<value…>` (see [`failure_groups`]).
    pub key: String,
    /// Which engine field gave the key: `recovery`, `limit`, `usage`,
    /// `diagnostic`, `exit`, `stop` or `unclassified`.
    pub source: &'static str,
    /// Human-readable cause as recorded (category, limit reason, code…).
    pub label: String,
    /// Attempts in the group, newest `ended` first (unparsable last).
    pub agents: Vec<FailureMember>,
    /// Distinct `work/task` pairs hit by this cause.
    pub tasks: usize,
}

/// One attempt of a group, identifiers only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FailureMember {
    pub agent_id: String,
    pub work_id: String,
    pub task_id: String,
    pub ended: String,
}

/// Cause of one failed attempt as `(source, key, label)`, most specific
/// engine field first (used by the groups and the L8 dataset export):
/// recovery fingerprint (category + cause fingerprint, observed values when
/// the decided ones are empty), then the diagnostic limit reason, the first
/// usage error code from provider facts, the most frequent diagnostic item
/// category/label, the exit code, the stop kind; else `unclassified`.
pub fn failure_cause(a: &crate::Agent) -> (&'static str, String, String) {
    let r = &a.recovery;
    let (category, fingerprint) = if r.cause_fingerprint.is_empty() {
        (&r.observed_category, &r.observed_cause_fingerprint)
    } else {
        (&r.category, &r.cause_fingerprint)
    };
    if !fingerprint.is_empty() {
        let label = if category.is_empty() { "recovery" } else { category.as_str() };
        return ("recovery", format!("recovery:{category}:{fingerprint}"), label.to_owned());
    }
    let d = &a.diagnostic;
    if d.limit_reached && !d.limit_reason.is_empty() {
        return ("limit", format!("limit:{}", d.limit_reason), d.limit_reason.clone());
    }
    let usage_error = a
        .native_provider_facts
        .as_ref()
        .and_then(|raw| serde_json::from_str::<Value>(raw.0.get()).ok())
        .and_then(|v| v.get("first_usage_error").and_then(Value::as_str).map(str::to_owned))
        .filter(|code| !code.is_empty());
    if let Some(code) = usage_error {
        return ("usage", format!("usage:{code}"), code);
    }
    if let Some(item) = d.items.iter().flatten().filter(|i| !i.category.is_empty()).max_by_key(|i| i.count) {
        return ("diagnostic", format!("diagnostic:{}:{}", item.category, item.label), format!("{} — {}", item.category, item.label));
    }
    if let Some(code) = a.exit_code.as_deref() {
        return ("exit", format!("exit:{code}"), format!("exit {code}"));
    }
    if !a.stop_kind.is_empty() {
        return ("stop", format!("stop:{}", a.stop_kind), a.stop_kind.clone());
    }
    ("unclassified", "unclassified".to_owned(), String::new())
}

/// Group the failed attempts among `agents` (status `failed`, or any
/// terminal attempt whose diagnostic says a limit was reached) by recorded
/// cause, largest group first (ties by key). Pure, no IO; every attempt
/// lands in exactly one group.
pub fn failure_groups(agents: &[crate::Agent]) -> Vec<FailureGroup> {
    let terminal = |s: &str| !matches!(s, "queued" | "starting" | "running" | "stopping");
    let mut groups: BTreeMap<String, FailureGroup> = BTreeMap::new();
    for a in agents.iter().filter(|a| a.status == "failed" || (terminal(&a.status) && a.diagnostic.limit_reached)) {
        let (source, key, label) = failure_cause(a);
        let g = groups.entry(key.clone()).or_insert_with(|| FailureGroup { key, source, label, agents: Vec::new(), tasks: 0 });
        g.agents.push(FailureMember { agent_id: a.id.clone(), work_id: a.work_id.clone(), task_id: a.task_id.clone(), ended: a.ended.clone() });
    }
    let ended = |m: &FailureMember| swarm_wire_json::go_rfc3339::parse(&m.ended).ok();
    let mut out: Vec<_> = groups
        .into_values()
        .map(|mut g| {
            g.agents.sort_by(|x, y| ended(y).cmp(&ended(x)).then_with(|| x.agent_id.cmp(&y.agent_id)));
            g.tasks = g.agents.iter().map(|m| (&m.work_id, &m.task_id)).collect::<std::collections::BTreeSet<_>>().len();
            g
        })
        .collect();
    out.sort_by(|x, y| y.agents.len().cmp(&x.agents.len()).then_with(|| x.key.cmp(&y.key)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed(id: &str, task: &str, ended: &str) -> crate::Agent {
        crate::Agent { id: id.into(), work_id: "w".into(), task_id: task.into(), status: "failed".into(), ended: ended.into(), ..Default::default() }
    }

    #[test]
    fn failures_group_by_the_most_specific_recorded_cause() {
        let mut a = failed("a", "t1", "2026-10-22T10:00:00Z");
        a.recovery.category = "provider-quota".into();
        a.recovery.cause_fingerprint = "f1".into();
        a.diagnostic.limit_reached = true;
        a.diagnostic.limit_reason = "tool-calls".into(); // recovery wins over limit
        let mut b = failed("b", "t2", "2026-10-22T11:00:00Z");
        b.recovery.observed_category = "provider-quota".into();
        b.recovery.observed_cause_fingerprint = "f1".into(); // same cause via observed values
        let mut c = crate::Agent { status: "completed".into(), ..failed("c", "t1", "") };
        c.diagnostic.limit_reached = true;
        c.diagnostic.limit_reason = "tool-calls".into();
        let mut d = failed("d", "t3", "");
        d.exit_code = Some(Box::new(2));
        let e = failed("e", "t3", "");
        let running = crate::Agent { status: "running".into(), ..failed("r", "t9", "") };
        let ok = crate::Agent { status: "completed".into(), ..failed("ok", "t9", "") };
        let groups = failure_groups(&[a, b, c, d, e, running, ok]);
        let shape: Vec<_> = groups.iter().map(|g| (g.key.as_str(), g.source, g.agents.len(), g.tasks)).collect();
        assert_eq!(shape, [
            ("recovery:provider-quota:f1", "recovery", 2, 2),
            ("exit:2", "exit", 1, 1),
            ("limit:tool-calls", "limit", 1, 1),
            ("unclassified", "unclassified", 1, 1),
        ]);
        assert_eq!(groups[0].agents.iter().map(|m| m.agent_id.as_str()).collect::<Vec<_>>(), ["b", "a"], "newest first");
        assert!(failure_groups(&[]).is_empty());
    }
}
