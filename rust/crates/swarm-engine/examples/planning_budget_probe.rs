//! Test-fixtures only numeric-data probe. Reads trusted fixture JSONL, not a public
//! transport. No provider, SQL, agent, permission or reservation is executed.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
use swarm_engine::{
    check_planning_activation_budget, check_planning_task_budget, decode_work,
    planning_scope_ancestors, planning_scope_task_count, PlanningBudgetError,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    work: Box<serde_json::value::RawValue>,
    scope: String,
}
/// Observe actual pure functions; invalid state/selection becomes safe refusal
/// data, while unexpected count/codec/IO failures retain their cause as errors.
/// False availability means exhausted numeric budget only, never denied authority.
fn one(frame: Frame) -> Result<serde_json::Value, Box<dyn Error>> {
    let work = decode_work(frame.work.get().as_bytes())?;
    let Ok(ancestors) = planning_scope_ancestors(&work, &frame.scope) else {
        return Ok(serde_json::json!({"code":"refused"}));
    };
    let available = |value: Result<(), PlanningBudgetError>| -> Result<bool, PlanningBudgetError> {
        match value {
            Ok(()) => Ok(true),
            Err(PlanningBudgetError::Exhausted { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    };
    Ok(
        serde_json::json!({"code":"ok","ancestors":ancestors.iter().map(|s|s.id.as_str()).collect::<Vec<_>>(),"task_count":planning_scope_task_count(&work,&frame.scope)?,"activation_budget_available":available(check_planning_activation_budget(&work,&frame.scope))?,"task_budget_available":available(check_planning_task_budget(&work,&frame.scope))?}),
    )
}
/// Decode complete fixture lines and flush complete observations; any framing,
/// codec/output failure stops with preserved original cause, never a blank PASS.
/// Input-sized allocations have no product-policy or aggregate-memory guarantee.
fn run() -> Result<(), Box<dyn Error>> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let line = line?;
        let frame = serde_json::from_str(&line)?;
        let raw = serde_json::to_vec(&one(frame)?)?;
        out.write_all(&raw)?;
        out.write_all(b"\n")?;
    }
    out.flush()?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("planning-budget-fixture: {error}");
        std::process::exit(1)
    }
}
