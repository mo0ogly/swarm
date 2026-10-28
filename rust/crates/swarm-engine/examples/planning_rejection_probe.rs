//! Test-fixtures only rejection-state data probe. Reads trusted fixture JSONL, not a public
//! transport. No provider, SQL, agent, permission or reservation is executed.
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};
use swarm_engine::{decode_work, planning_rejection_state, planning_rejection_unchanged};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    work: Box<serde_json::value::RawValue>,
    scope: String,
}
/// Observe actual pure fingerprint/wait checks; invalid durable/scope inputs
/// become explicit refusal data. Codec/frame/IO failures retain original causes.
/// No actor authenticity, wait persistence or retry permission is established.
fn one(frame: Frame) -> Result<serde_json::Value, Box<dyn Error>> {
    let work = decode_work(frame.work.get().as_bytes())?;
    match planning_rejection_state(&work, &frame.scope) {
        Ok(state) => Ok(
            serde_json::json!({"code":"ok","state_sha256":state,"unchanged":planning_rejection_unchanged(&work,&frame.scope)?}),
        ),
        Err(swarm_engine::PlanningRejectionError::State(_)) => {
            Ok(serde_json::json!({"code":"refused"}))
        }
        Err(error) => Err(error.into()),
    }
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
        eprintln!("planning-rejection-fixture: {error}");
        std::process::exit(1)
    }
}
