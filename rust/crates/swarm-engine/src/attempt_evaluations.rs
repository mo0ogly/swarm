//! Asynchronous attempt evaluations (LangWatch L4 "monitors"), deterministic:
//! measurements of a finished attempt computed from engine records and its
//! provider telemetry, stored once by the console store. A verdict (`passed`)
//! is given only where an engine mark or a saved policy defines it; otherwise
//! the evaluation is a score without verdict, never a hidden threshold. No
//! model is called here: a model judge would have to go through the engine's
//! launch gates and budget reservations, which this module does not own.
use serde::Serialize;

/// One evaluator's result for one attempt.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AttemptEvaluation {
    /// Stable evaluator id (see [`evaluate_attempt`]).
    pub evaluator: &'static str,
    /// Measurement in [0, 1] when defined (1 = best), else `None`.
    pub score: Option<f64>,
    /// Verdict when an engine mark or saved policy defines one, else `None`.
    pub passed: Option<bool>,
    /// Short factual detail (counts), never provider content.
    pub detail: String,
}

/// Saved policies the evaluators may use; `None` = not configured.
#[derive(Clone, Copy, Debug, Default)]
pub struct EvaluationPolicy {
    /// v31 cache health: (miss ratio percent, minimum input tokens).
    pub cache: Option<(i64, i64)>,
    /// v34 tolerated declared-vs-telemetry output gap, percent.
    pub usage_mismatch_percent: Option<i64>,
}

fn field(f: &std::collections::BTreeMap<String, serde_json::Value>, key: &str) -> u64 {
    f.get(key).and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or_else(|| v.as_u64())).unwrap_or(0)
}

/// Evaluate one finished attempt. Evaluators:
/// - `outcome`: passed = status `completed` (engine status; no score);
/// - `limits`: passed = no run limit reached (engine diagnostic mark);
/// - `tool-reliability`: score = 1 − failed / declared tool results
///   (telemetry); no verdict (no saved policy);
/// - `cache-reuse`: score = cache-read share of total input over the
///   attempt's calls; passed = no call is a v31 cache miss (policy needed);
/// - `usage-consistency`: score = 1 − |declared − telemetry| / declared
///   output tokens (floored at 0); passed = gap within the v34 tolerance.
///
/// Telemetry-based evaluators are omitted when the attempt has no
/// telemetry calls/tools (nothing to measure, not a zero). When the call
/// page is truncated (`calls_more`), `cache-reuse` and `usage-consistency`
/// are measured on the kept calls, marked `partial`, and give no verdict.
/// Pure, no IO.
pub fn evaluate_attempt(
    agent: &crate::Agent,
    telemetry: Option<&crate::AgentTelemetry>,
    policy: EvaluationPolicy,
) -> Vec<AttemptEvaluation> {
    let mut out = vec![
        AttemptEvaluation { evaluator: "outcome", score: None, passed: Some(agent.status == "completed"), detail: format!("status={}", agent.status) },
        AttemptEvaluation {
            evaluator: "limits",
            score: None,
            passed: Some(!agent.diagnostic.limit_reached),
            detail: if agent.diagnostic.limit_reason.is_empty() { String::new() } else { format!("limit={}", agent.diagnostic.limit_reason) },
        },
    ];
    let Some(t) = telemetry else { return out };
    let (results, failures) = t.tools.iter().fold((0u64, 0u64), |(n, f), x| (n + x.count, f + x.failures));
    if results > 0 {
        out.push(AttemptEvaluation {
            evaluator: "tool-reliability",
            score: Some(1.0 - failures as f64 / results as f64),
            passed: None,
            detail: format!("results={results} failures={failures}"),
        });
    }
    let calls: Vec<_> = t.calls.iter().map(|c| (field(&c.fields, "input_tokens"), field(&c.fields, "cache_read_tokens"), field(&c.fields, "cache_creation_tokens"), field(&c.fields, "output_tokens"))).collect();
    let total_input: u64 = calls.iter().map(|(i, r, c, _)| i + r + c).sum();
    if total_input > 0 {
        let read: u64 = calls.iter().map(|(_, r, _, _)| r).sum();
        let misses = policy.cache.map(|(ratio, min)| {
            calls
                .iter()
                .filter(|(i, r, c, _)| {
                    let total = i + r + c;
                    total >= u64::try_from(min).unwrap_or(u64::MAX) && (*r as f64) * 100.0 < (total as f64) * ratio as f64
                })
                .count()
        });
        out.push(AttemptEvaluation {
            evaluator: "cache-reuse",
            score: Some(read as f64 / total_input as f64),
            passed: misses.filter(|_| !t.calls_more).map(|m| m == 0),
            detail: format!("calls={}{}{}", calls.len(), misses.map_or(String::new(), |m| format!(" misses={m}")), if t.calls_more { " partial" } else { "" }),
        });
    }
    if let (Some(usage), false) = (agent.usage.as_ref(), calls.is_empty()) {
        let declared = u64::try_from(usage.output_tokens).unwrap_or(0);
        let observed: u64 = calls.iter().map(|(_, _, _, o)| o).sum();
        if declared > 0 {
            let gap = declared.abs_diff(observed);
            out.push(AttemptEvaluation {
                evaluator: "usage-consistency",
                score: Some((1.0 - gap as f64 / declared as f64).max(0.0)),
                passed: policy.usage_mismatch_percent.filter(|_| !t.calls_more).map(|p| u128::from(gap) * 100 <= u128::from(declared) * u128::try_from(p.max(0)).unwrap_or(0)),
                detail: format!("declared={declared} telemetry={observed}{}", if t.calls_more { " partial" } else { "" }),
            });
        }
    }
    out
}

/// Deterministic sampling: whether `agent_id` falls in the first `percent`
/// of 100 buckets (SHA-256 of the id), so a sample is stable across runs and
/// servers. 100 keeps every attempt; out-of-range values are clamped.
pub fn evaluation_sampled(agent_id: &str, percent: i64) -> bool {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(agent_id.as_bytes());
    let bucket = u16::from_be_bytes([digest[0], digest[1]]) % 100;
    i64::from(bucket) < percent.clamp(0, 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn call(input: &str, read: &str, output: &str) -> crate::TelemetryCall {
        let mut fields = BTreeMap::new();
        for (k, v) in [("input_tokens", input), ("cache_read_tokens", read), ("output_tokens", output)] {
            fields.insert(k.to_owned(), serde_json::Value::String(v.to_owned()));
        }
        crate::TelemetryCall { at_ms: 1, fields }
    }

    #[test]
    fn evaluators_measure_and_judge_only_with_a_mark_or_saved_policy() {
        let mut agent = crate::Agent { id: "a".into(), status: "completed".into(), ..Default::default() };
        agent.usage = Some(Box::new(crate::Usage { output_tokens: 100, ..Default::default() }));
        let names = |v: &[AttemptEvaluation]| v.iter().map(|e| e.evaluator).collect::<Vec<_>>();
        // Witness without telemetry: engine marks only.
        let bare = evaluate_attempt(&agent, None, EvaluationPolicy::default());
        assert_eq!(names(&bare), ["outcome", "limits"]);
        assert_eq!((bare[0].passed, bare[1].passed), (Some(true), Some(true)));
        let mut t = crate::AgentTelemetry {
            calls: vec![call("1000", "3000", "60"), call("2000", "0", "30")],
            tools: vec![crate::TelemetryTool { name: "Bash".into(), count: 4, failures: 1, ..Default::default() }],
            ..Default::default()
        };
        let no_policy = evaluate_attempt(&agent, Some(&t), EvaluationPolicy::default());
        let by = |v: &[AttemptEvaluation], n: &str| v.iter().find(|e| e.evaluator == n).cloned().unwrap();
        assert_eq!(names(&no_policy), ["outcome", "limits", "tool-reliability", "cache-reuse", "usage-consistency"]);
        assert_eq!((by(&no_policy, "tool-reliability").score, by(&no_policy, "tool-reliability").passed), (Some(0.75), None));
        assert_eq!((by(&no_policy, "cache-reuse").score, by(&no_policy, "cache-reuse").passed), (Some(0.5), None), "no saved cache policy: score only");
        assert_eq!((by(&no_policy, "usage-consistency").score, by(&no_policy, "usage-consistency").passed), (Some(0.9), None));
        let policy = EvaluationPolicy { cache: Some((50, 1000)), usage_mismatch_percent: Some(10) };
        let judged = evaluate_attempt(&agent, Some(&t), policy);
        assert_eq!(by(&judged, "cache-reuse").passed, Some(false), "second call reads 0 % of 2000 ≥ 1000 tokens: a miss");
        assert_eq!(by(&judged, "cache-reuse").detail, "calls=2 misses=1");
        assert_eq!(by(&judged, "usage-consistency").passed, Some(true), "gap 10 of 100 is within 10 %");
        agent.status = "failed".into();
        agent.diagnostic.limit_reached = true;
        agent.diagnostic.limit_reason = "tool-calls".into();
        t.calls_more = true;
        let partial = evaluate_attempt(&agent, Some(&t), policy);
        assert_eq!((by(&partial, "cache-reuse").passed, by(&partial, "usage-consistency").passed), (None, None), "no verdict on a truncated page");
        assert!(by(&partial, "cache-reuse").detail.ends_with("partial"));
        let failed = evaluate_attempt(&agent, None, policy);
        assert_eq!((failed[0].passed, failed[1].passed, failed[1].detail.as_str()), (Some(false), Some(false), "limit=tool-calls"));
    }

    #[test]
    fn sampling_is_stable_and_bounded() {
        let ids: Vec<String> = (0..400).map(|i| format!("agent-{i}")).collect();
        let kept = |p| ids.iter().filter(|id| evaluation_sampled(id, p)).count();
        assert_eq!((kept(0), kept(100)), (0, 400));
        let half = kept(50);
        assert!((150..=250).contains(&half), "about half: {half}");
        assert!(ids.iter().all(|id| evaluation_sampled(id, 30) == evaluation_sampled(id, 30)), "stable");
        assert!(ids.iter().filter(|id| evaluation_sampled(id, 30)).all(|id| evaluation_sampled(id, 60)), "a larger share keeps the smaller sample");
    }
}
