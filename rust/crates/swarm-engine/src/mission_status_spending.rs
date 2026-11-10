//! Pure Go mission spending/attempt projections at commit
//! 553593a2ffa3823d16784a6fedc291eebf390837. Inputs are the complete retained
//! read snapshot, not a recent-agent window. No SQL, Root, provider, pricing,
//! clock, reservation mutation or task acceptance is performed here.
use crate::mission_status_contract::{AttemptLedger, MissionSpending, SpendingCall, SpendingRow};
use crate::{
    Agent, AutomaticValidation, CostTotal, GoFloat, MissionStatusInputs, RecordedEvent,
    RootStoreError as E, Usage, ValidationControlResult, Work,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use swarm_wire_json::go_rfc3339::parse as parse_go_timestamp;

// Go loop_guard.go:20: retained categorised metrics schema, not a policy ceiling.
const ATTEMPT_METRICS_VERSION: i64 = 1;
const TESTS_UNKNOWN: &str =
    "inconnu : aucun signal fiable ne distingue un test dans les commandes observées";

/// Checked Linux64 Go counter addition. Representation overflow refuses with a
/// static safe code; no business cap, retry or partial projection is published.
fn sum(left: i64, right: i64) -> Result<i64, E> {
    left.checked_add(right)
        .ok_or(E::Refused("mission-spending-integer-overflow"))
}

/// Convert an actually recorded collection length into the Go int wire width.
fn count(value: usize) -> Result<i64, E> {
    i64::try_from(value).map_err(|_| E::Refused("mission-spending-integer-overflow"))
}

/// Start the exact Go zero accumulator; missing costs are added separately and
/// never interpreted as a provider-reported zero or measured invoice.
fn cost_total() -> CostTotal {
    CostTotal {
        reported_usd: GoFloat(0.0),
        attempts_with_cost: 0,
        attempts_without_cost: 0,
    }
}

/// Add a retained provider declaration without prices/rounding. No declaration
/// increments unknown, while a genuine Some(0) increments measured. Arithmetic
/// or nonfinite JSON representation failure refuses; signs remain supplied Go
/// facts (reader admission/usage validation is not replaced by this projection).
fn add_cost(total: &mut CostTotal, usage: Option<&Usage>) -> Result<(), E> {
    if let Some(cost) = usage.and_then(|usage| usage.provider_reported_cost_usd.as_deref()) {
        let next = total.reported_usd.0 + cost.0;
        if !cost.0.is_finite() || !next.is_finite() {
            return Err(E::Refused("mission-spending-cost-overflow"));
        }
        total.reported_usd = GoFloat(next);
        total.attempts_with_cost = sum(total.attempts_with_cost, 1)?;
    } else {
        total.attempts_without_cost = sum(total.attempts_without_cost, 1)?;
    }
    Ok(())
}

/// Known zero stays Some(0); missing, invalid and reversed timestamps stay None.
/// Go Time.Sub saturates at i64 nanoseconds before truncating milliseconds. No
/// elapsed time is fabricated for still-running/never-started processes. Parsing
/// uses the shared Go RFC3339 parser, including historical comma fractions and
/// numeric-zone normalization; original stored timestamps remain unchanged.
pub fn recorded_process_duration(started: &str, ended: &str) -> Option<i64> {
    let start = parse_go_timestamp(started).ok()?;
    let end = parse_go_timestamp(ended).ok()?;
    let nanos = end.unix_timestamp_nanos() - start.unix_timestamp_nanos();
    if nanos < 0 {
        return None;
    }
    Some((nanos.min(i128::from(i64::MAX)) / 1_000_000) as i64)
}

/// Produce a recorded worker attempt declaration, never proof inference was
/// sent. Task scope and this agent's persisted requested/observed models alone
/// supply attribution; no current provider/model configuration is consulted.
pub fn spending_agent_call(work: &Work, agent: &Agent) -> SpendingCall {
    let scope = work
        .tasks
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .find(|task| task.id == agent.task_id)
        .map(|task| task.scope_id.clone())
        .unwrap_or_default();
    SpendingCall {
        id: format!("agent/{}", agent.id),
        record_type: "producer_attempt".into(),
        kind: "worker".into(),
        role: agent.role.clone(),
        scope,
        task: agent.task_id.clone(),
        attempt: agent.attempt_id.clone(),
        provider: agent.provider.clone(),
        requested_model: agent
            .model_route
            .as_ref()
            .map(|route| route.model.clone())
            .unwrap_or_default(),
        observed_model: agent
            .reported_model
            .as_ref()
            .map(|model| model.model.clone())
            .unwrap_or_default(),
        state: agent.status.clone(),
        registered_at: agent.started.clone(),
        usage: agent.usage.clone(),
    }
}

/// Per-attempt bilan, sorted task/agent/attempt as Go. Input order is not changed.
/// Task acceptance is the recorded task status independently of process ending;
/// it is not fresh acceptance evidence. Metrics v1 nonterminal mode permits
/// observed metrics, pending/degraded becomes partial, legacy/terminal unknown.
/// Unknown usage and unknown reported cost remain separate from measured zero.
pub fn attempt_ledgers(work: &Work, agents: &[Agent]) -> Result<Vec<AttemptLedger>, E> {
    attempt_ledgers_from(work, agents.iter())
}

/// Reduce borrowed complete-history agents without cloning their private prompts
/// or environment data. Public output remains limited to actual ledger fields.
fn attempt_ledgers_from<'a>(
    work: &Work,
    agents: impl Iterator<Item = &'a Agent>,
) -> Result<Vec<AttemptLedger>, E> {
    let mut out = Vec::new();
    for agent in agents {
        let task = work
            .tasks
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|task| task.id == agent.task_id);
        let measured =
            agent.progress.metrics_version == ATTEMPT_METRICS_VERSION && agent.mode != "terminal";
        let measurement = if measured {
            if !agent.progress.degraded.is_empty() || agent.progress.pending_tools > 0 {
                "partial"
            } else {
                "observed"
            }
        } else {
            "unknown"
        };
        let mut cost = cost_total();
        add_cost(&mut cost, agent.usage.as_deref())?;
        out.push(AttemptLedger {
            task: agent.task_id.clone(),
            label: task
                .map(|task| task.title.clone())
                .unwrap_or_else(|| agent.task_id.clone()),
            agent: agent.id.clone(),
            attempt: agent.attempt_id.clone(),
            role: agent.role.clone(),
            requested_model: agent
                .model_route
                .as_ref()
                .map(|route| route.model.clone())
                .unwrap_or_default(),
            observed_model: agent
                .reported_model
                .as_ref()
                .map(|model| model.model.clone())
                .unwrap_or_default(),
            process_state: agent.status.clone(),
            accepted: task.is_some_and(|task| task.status == "accepted"),
            measurement: measurement.into(),
            calls: 1,
            tools: agent.progress.tool_calls,
            measured,
            reads: if measured {
                agent.progress.tool_reads
            } else {
                0
            },
            writes: if measured {
                agent.progress.tool_writes
            } else {
                0
            },
            unclassified: if measured {
                agent.progress.tool_unclassified
            } else {
                0
            },
            tests: TESTS_UNKNOWN.into(),
            errors: if measured {
                agent.progress.tool_errors_total
            } else {
                0
            },
            repeats: if measured {
                agent.progress.tool_repeats_total
            } else {
                0
            },
            degraded: agent.progress.degraded.clone(),
            input: agent.usage.as_ref().map_or(0, |usage| usage.input_tokens),
            output: agent.usage.as_ref().map_or(0, |usage| usage.output_tokens),
            missing_usage: agent.usage.is_none(),
            cost,
            started: agent.started.clone(),
            ended: agent.ended.clone(),
            duration_ms: recorded_process_duration(&agent.started, &agent.ended).map(Box::new),
        });
    }
    out.sort_by(|left, right| {
        (&left.task, &left.agent, &left.attempt).cmp(&(&right.task, &right.agent, &right.attempt))
    });
    Ok(out)
}

/// Allocate the Go spending group accumulator only when a real call belongs to
/// it. Zero counters are arithmetic identities, not missing-data substitutes.
fn spending_row(kind: &str, id: &str, label: String) -> SpendingRow {
    SpendingRow {
        kind: kind.into(),
        id: id.into(),
        label,
        calls: 0,
        tools: 0,
        unknown_tools: 0,
        input: 0,
        output: 0,
        missing_usage: 0,
        cost: cost_total(),
        reviews: 0,
        task_retries: 0,
        duration_ms: None,
        missing_durations: 0,
    }
}

/// Aggregate usage declarations. Absence increments missing; explicit zero token
/// counts remain measured. Cost absence has its own counter independently.
fn add_measured_usage(row: &mut SpendingRow, usage: Option<&Usage>) -> Result<(), E> {
    if let Some(usage) = usage {
        row.input = sum(row.input, usage.input_tokens)?;
        row.output = sum(row.output, usage.output_tokens)?;
    } else {
        row.missing_usage = sum(row.missing_usage, 1)?;
    }
    add_cost(&mut row.cost, usage)
}

/// Decode only the Go controls envelope; nil controls/entries remain nil. All
/// ValidationControlResult fields are decoded with their real Go shapes so an
/// invalid ignored-looking field still refuses as json.Unmarshal would. No
/// arbitrary JSON payload or secret value enters a diagnostic message.
#[derive(Default, Deserialize)]
#[serde(default)]
struct ManagedControls {
    controls: Option<BTreeMap<String, Option<Vec<ValidationControlResult>>>>,
}

/// Count executed controls, not controls passed or arrays merely present. Only
/// these two event kinds are consumed; unrelated malformed payloads stay ignored
/// as in Go's SQL selection. Go-compatible decoder errors retain their original
/// typed codec cause through RootStoreError::Encode; no success-empty fallback.
fn executed_controls(events: &[RecordedEvent], work: &str) -> Result<i64, E> {
    use crate::model_wire::Kind;
    let mut executed = 0;
    for event in events.iter().filter(|event| {
        event.work_id == work.as_bytes()
            && matches!(
                event.kind.as_slice(),
                b"task.auto-validation" | b"managed.integrated"
            )
    }) {
        let raw = event.payload.as_deref().unwrap_or(&[]);
        let results: Vec<ValidationControlResult> = if event.kind == b"managed.integrated" {
            let record: ManagedControls = crate::work::decode(
                raw,
                &[(
                    "controls",
                    Kind::Map(&Kind::Array(&Kind::Object(
                        crate::models::shape_validation_control_result,
                    ))),
                )],
            )
            .map_err(E::Encode)?;
            record
                .controls
                .into_iter()
                .flat_map(|map| map.into_values())
                .flatten()
                .flatten()
                .collect()
        } else {
            let record: AutomaticValidation =
                crate::work::decode(raw, crate::models::shape_automatic_validation())
                    .map_err(E::Encode)?;
            record.controls.unwrap_or_default()
        };
        for control in results {
            if control.executed {
                executed = sum(executed, 1)?;
            }
        }
    }
    Ok(executed)
}

/// Complete pure Go spending reduction over a complete retained input snapshot.
/// Agents from another explicit Work are excluded, empty legacy Work ID included;
/// planner rows are already nonreleased per the reader, and are never assigned a
/// present-day model. Sorted rows/calls and nonnil arrays match Go. The supplied
/// observed_at is copied, with no clock IO, estimates or hidden history limit.
/// Checked integer/nonfinite arithmetic and malformed selected control payloads
/// return safe typed errors without changing inputs or publishing a partial DTO.
/// The caller retains Root close/output error handling and binary parity checks.
pub fn mission_spending(
    inputs: &MissionStatusInputs,
    observed_at: &str,
) -> Result<MissionSpending, E> {
    let work = &inputs.work;
    let mut rows: BTreeMap<String, SpendingRow> = BTreeMap::new();
    let mut calls = Vec::new();
    let mut attempts_by_task: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut explicit_retries: BTreeMap<&str, i64> = BTreeMap::new();
    for agent in &inputs.agents {
        if !agent.work_id.is_empty() && agent.work_id != work.id {
            continue;
        }
        calls.push(spending_agent_call(work, agent));
        let task = work
            .tasks
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|task| task.id == agent.task_id);
        let row = rows
            .entry(format!("worker:{}", agent.task_id))
            .or_insert_with(|| {
                spending_row(
                    "worker",
                    &agent.task_id,
                    task.map(|task| task.title.clone())
                        .unwrap_or_else(|| agent.task_id.clone()),
                )
            });
        row.calls = sum(row.calls, 1)?;
        if let Some(duration) = recorded_process_duration(&agent.started, &agent.ended) {
            row.duration_ms = Some(Box::new(sum(
                row.duration_ms.as_deref().copied().unwrap_or(0),
                duration,
            )?));
        } else {
            row.missing_durations = sum(row.missing_durations, 1)?;
        }
        row.tools = sum(row.tools, agent.progress.tool_calls)?;
        if agent.mode == "terminal"
            || !agent.progress.degraded.is_empty()
            || agent.progress.metrics_version != ATTEMPT_METRICS_VERSION
        {
            row.unknown_tools = sum(row.unknown_tools, 1)?;
        }
        if let Some(task) = task {
            row.reviews = sum(
                count(task.previous_reviews.as_ref().map_or(0, Vec::len))?,
                i64::from(task.independent_review.is_some()),
            )?;
        }
        if !agent.previous.is_empty() {
            let n = explicit_retries.entry(&agent.task_id).or_default();
            *n = sum(*n, 1)?;
        }
        if !agent.attempt_id.is_empty() {
            attempts_by_task
                .entry(&agent.task_id)
                .or_default()
                .insert(&agent.attempt_id);
        }
        add_measured_usage(row, agent.usage.as_deref())?;
    }
    let mut retries = 0;
    for row in rows.values_mut() {
        let distinct = attempts_by_task
            .get(row.id.as_str())
            .map_or(0, BTreeSet::len)
            .saturating_sub(1);
        row.task_retries =
            count(distinct)?.max(*explicit_retries.get(row.id.as_str()).unwrap_or(&0));
        retries = sum(retries, row.task_retries)?;
    }
    for planned in &inputs.planning_calls {
        let (kind, label) = if planned.scope_id == "reviewer" {
            ("reviewer", "Vérificateur indépendant".into())
        } else {
            ("planner", format!("Responsable : {}", planned.scope_id))
        };
        let row = rows
            .entry(format!("{kind}:{}", planned.scope_id))
            .or_insert_with(|| spending_row(kind, &planned.scope_id, label));
        row.calls = sum(row.calls, 1)?;
        add_measured_usage(row, planned.usage.as_ref())?;
        calls.push(SpendingCall {
            id: format!("planning/{}", planned.id),
            record_type: "planning_reservation".into(),
            kind: kind.into(),
            role: kind.into(),
            scope: planned.scope_id.clone(),
            task: String::new(),
            attempt: String::new(),
            provider: String::new(),
            requested_model: String::new(),
            observed_model: String::new(),
            state: planned.state.clone(),
            registered_at: planned.created_at.clone(),
            usage: planned.usage.clone().map(Box::new),
        });
    }
    calls.sort_by(|left, right| {
        (&left.registered_at, &left.id).cmp(&(&right.registered_at, &right.id))
    });
    Ok(MissionSpending { calls: Some(calls), observed_at: observed_at.into(), source: "agent.lifecycle + agent.progress + provider.result + task.review-history + planning_calls + events".into(), rows: Some(rows.into_values().collect()), controls: executed_controls(&inputs.events, &work.id)?, retries, note: "Les appels d’outils, les appels IA et les contrôles du moteur sont des mesures différentes. Une mesure absente n’est pas un zéro ; les relances internes du fournisseur ne sont pas toutes observables. Une réservation d’appel ne prouve pas son envoi au fournisseur.".into(), attempts: Some(attempt_ledgers_from(work, inputs.agents.iter().filter(|agent| agent.work_id.is_empty() || agent.work_id == work.id))?) })
}

#[cfg(test)]
mod tests;
