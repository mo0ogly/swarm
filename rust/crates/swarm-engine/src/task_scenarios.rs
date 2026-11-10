//! Task scenarios (LangWatch L6), deterministic: operator expectations on a
//! task, judged against what was recorded for its latest attempt — engine
//! status and declared usage, and the provider telemetry of tool results —
//! never against the agent's own claims. An expectation that needs telemetry
//! the attempt does not have is "not judgeable" (`passed: None`), never a
//! guess. Specification and results carry no prompt or tool content.
use serde::{Deserialize, Serialize};

/// One operator expectation. `tool` names are matched exactly against the
/// telemetry tool name; amounts are USD.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Expectation {
    /// The attempt ended with this engine status (e.g. `completed`).
    Outcome { status: String },
    /// At least one result of this tool was declared.
    ToolUsed { tool: String },
    /// No result of this tool was declared.
    ToolNotUsed { tool: String },
    /// This tool was used and none of its results failed.
    ToolSucceeded { tool: String },
    /// At most this many failed tool results, all tools together.
    MaxToolFailures { value: u64 },
    /// Declared cost at most this amount.
    MaxCostUsd { value: f64 },
    /// The provider declared usage the engine could read (`observed`).
    UsageObserved,
}

/// Saved scenario of one task (schema_version 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSpec {
    pub schema_version: i64,
    pub expectations: Vec<Expectation>,
}

/// Result of one expectation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExpectationResult {
    pub expectation: Expectation,
    /// `None` when the recorded facts cannot decide it.
    pub passed: Option<bool>,
    /// What was observed, factual and short.
    pub observed: String,
}

/// Usage state as the console shows it: `native_provider_facts.usage_state`
/// when recorded, else `observed` when usage exists, else `absent`.
fn usage_state(a: &crate::Agent) -> String {
    a.native_provider_facts
        .as_ref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw.0.get()).ok())
        .and_then(|v| v.get("usage_state").and_then(serde_json::Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| if a.usage.is_some() { "observed" } else { "absent" }.to_owned())
}

/// Check a scenario specification: version 1, at least one expectation,
/// non-blank tool names and statuses without control characters, finite
/// non-negative amounts. Errors are stable codes. Neither the number of
/// expectations nor name lengths have a bound of their own: the saved HTTP
/// body limit bounds the request. A status is compared as written (an
/// unknown status simply never matches).
pub fn validate_scenario(spec: &ScenarioSpec) -> Result<(), &'static str> {
    if spec.schema_version != 1 {
        return Err("scenario-version-unsupported");
    }
    if spec.expectations.is_empty() {
        return Err("scenario-empty");
    }
    for e in &spec.expectations {
        match e {
            Expectation::Outcome { status: s } if s.trim().is_empty() || s.chars().any(char::is_control) => return Err("scenario-value-invalid"),
            Expectation::ToolUsed { tool } | Expectation::ToolNotUsed { tool } | Expectation::ToolSucceeded { tool }
                if tool.trim().is_empty() || tool.chars().any(char::is_control) =>
            {
                return Err("scenario-value-invalid")
            }
            Expectation::MaxCostUsd { value } if !value.is_finite() || *value < 0.0 => return Err("scenario-value-invalid"),
            _ => {}
        }
    }
    Ok(())
}

/// Judge `spec` against the latest attempt of the task (`None` when the
/// task has no attempt: every expectation is then not judgeable) and its
/// telemetry (`None` when none was received). Tool expectations read the
/// telemetry tool aggregates (complete counts, not a page). Pure, no IO.
pub fn evaluate_scenario(
    spec: &ScenarioSpec,
    agent: Option<&crate::Agent>,
    telemetry: Option<&crate::AgentTelemetry>,
) -> Vec<ExpectationResult> {
    let tool = |name: &str| telemetry.map(|t| t.tools.iter().find(|x| x.name == name).map_or((0, 0), |x| (x.count, x.failures)));
    spec.expectations
        .iter()
        .map(|e| {
            let (passed, observed) = match (agent, e) {
                (None, _) => (None, "no attempt".to_owned()),
                (Some(a), Expectation::Outcome { status }) => (Some(&a.status == status), format!("status={}", a.status)),
                (Some(a), Expectation::UsageObserved) => {
                    let state = usage_state(a);
                    (Some(state == "observed"), format!("usage={state}"))
                }
                (Some(a), Expectation::MaxCostUsd { value }) => match a.usage.as_ref().and_then(|u| u.provider_reported_cost_usd.as_deref()) {
                    Some(cost) => (Some(cost.0 <= *value), format!("cost={}", cost.0)),
                    None => (None, "no declared cost".to_owned()),
                },
                (Some(_), Expectation::MaxToolFailures { value }) => match telemetry {
                    Some(t) => {
                        let failures: u64 = t.tools.iter().map(|x| x.failures).sum();
                        (Some(failures <= *value), format!("failures={failures}"))
                    }
                    None => (None, "no telemetry".to_owned()),
                },
                (Some(_), Expectation::ToolUsed { tool: name }) => match tool(name) {
                    Some((n, _)) => (Some(n > 0), format!("results={n}")),
                    None => (None, "no telemetry".to_owned()),
                },
                (Some(_), Expectation::ToolNotUsed { tool: name }) => match tool(name) {
                    Some((n, _)) => (Some(n == 0), format!("results={n}")),
                    None => (None, "no telemetry".to_owned()),
                },
                (Some(_), Expectation::ToolSucceeded { tool: name }) => match tool(name) {
                    Some((n, f)) => (Some(n > 0 && f == 0), format!("results={n} failures={f}")),
                    None => (None, "no telemetry".to_owned()),
                },
            };
            ExpectationResult { expectation: e.clone(), passed, observed }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(raw: &str) -> ScenarioSpec {
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn scenarios_judge_recorded_facts_and_never_guess() {
        let s = spec(r#"{"schema_version":1,"expectations":[
            {"kind":"outcome","status":"completed"},{"kind":"tool-used","tool":"Bash"},{"kind":"tool-succeeded","tool":"Bash"},
            {"kind":"tool-not-used","tool":"Write"},{"kind":"max-tool-failures","value":0},{"kind":"max-cost-usd","value":0.5},{"kind":"usage-observed"}]}"#);
        validate_scenario(&s).unwrap();
        let mut agent = crate::Agent { id: "a".into(), status: "completed".into(), ..Default::default() };
        agent.usage = Some(Box::new(crate::Usage { output_tokens: 1, provider_reported_cost_usd: Some(Box::new(crate::GoFloat(0.4))), ..Default::default() }));
        let t = crate::AgentTelemetry {
            tools: vec![crate::TelemetryTool { name: "Bash".into(), count: 3, failures: 1, ..Default::default() }],
            ..Default::default()
        };
        let verdicts = |r: &[ExpectationResult]| r.iter().map(|x| x.passed).collect::<Vec<_>>();
        let judged = evaluate_scenario(&s, Some(&agent), Some(&t));
        assert_eq!(verdicts(&judged), [Some(true), Some(true), Some(false), Some(true), Some(false), Some(true), Some(true)]);
        assert_eq!(judged[2].observed, "results=3 failures=1");
        // Without telemetry: tool expectations are not judgeable, the rest still is.
        let bare = evaluate_scenario(&s, Some(&agent), None);
        assert_eq!(verdicts(&bare), [Some(true), None, None, None, None, Some(true), Some(true)]);
        assert!(evaluate_scenario(&s, None, None).iter().all(|r| r.passed.is_none() && r.observed == "no attempt"));
        for (raw, code) in [
            (r#"{"schema_version":2,"expectations":[{"kind":"usage-observed"}]}"#, "scenario-version-unsupported"),
            (r#"{"schema_version":1,"expectations":[]}"#, "scenario-empty"),
            (r#"{"schema_version":1,"expectations":[{"kind":"tool-used","tool":" "}]}"#, "scenario-value-invalid"),
            (r#"{"schema_version":1,"expectations":[{"kind":"max-cost-usd","value":-1}]}"#, "scenario-value-invalid"),
            (r#"{"schema_version":1,"expectations":[{"kind":"outcome","status":"done\u0007"}]}"#, "scenario-value-invalid"),
        ] {
            assert_eq!(validate_scenario(&spec(raw)), Err(code), "{raw}");
        }
        assert!(serde_json::from_str::<ScenarioSpec>(r#"{"schema_version":1,"expectations":[{"kind":"guess"}]}"#).is_err(), "unknown kind refused");
    }
}
