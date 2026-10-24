//! Read-only W4 verification consumer, not `mission status --json`.
//! Requires an explicit owned inactive fixture marker. Calls public Root APIs
//! and pure spending projection on the default engine, without fixture features,
//! SQL writes, provider calls, process actions, grants or task acceptance.
use serde::Deserialize;
use serde_json::json;
use std::{error::Error, fmt, fs, io::Write, os::unix::fs::MetadataExt, path::PathBuf};
use swarm_engine::{RootStore, RootStoreError as E};

/// Preserve the full typed error tree while printing only safe labels/codes.
/// This is a local verification diagnostic, not a retry or rollback assertion.
struct Failure(E);
/// Project every known typed cause through the common safe-label walker. No raw
/// Display, Work, Agent or path is rendered; cycle identity avoids diagnostic loops.
/// This is a safe summary, not evidence that an observation authorizes an action.
fn diagnostics(error: &(dyn Error + 'static)) -> Vec<serde_json::Value> {
    let mut pending: Vec<&(dyn Error + 'static)> = vec![error];
    let mut seen: Vec<*const dyn Error> = Vec::new();
    let mut records = Vec::new();
    while let Some(error) = pending.pop() {
        let pointer = error as *const dyn Error;
        if seen.iter().any(|p| std::ptr::eq(*p, pointer)) {
            continue;
        }
        seen.push(pointer);
        let links = swarm_engine::native_error_links(error);
        records.push(json!({"labels":links.labels,"io_errno":links.io_errno,"sqlite_code":links.sqlite_code}));
        pending.extend(links.children.into_iter().rev().map(|(_, child)| child));
    }
    records
}
impl fmt::Debug for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatusObservationFailure")
            .field("diagnostics", &diagnostics(&self.0))
            .finish()
    }
}
/// Explicit fixture coordinates; marker is attribution, never actor authentication.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u8,
    kind: String,
    root: PathBuf,
    work: String,
    operator: String,
}
/// Wrap local input/output errors without printing private text or losing errno.
fn io_error(error: std::io::Error) -> E {
    E::Runtime(swarm_engine::RuntimeError::new(
        swarm_engine::RuntimeOperation::Output,
        error,
    ))
}
/// Preserve input-filesystem IO under the platform operation; no private path
/// is rendered. Output failures use the separate Output operation above.
fn input_io(error: std::io::Error) -> E {
    E::Runtime(swarm_engine::RuntimeError::new(
        swarm_engine::RuntimeOperation::Platform,
        error,
    ))
}
/// Read the explicit marker from the owned Root and reject mismatched coordinates.
/// No marker can create permissions; fixture driver must prove quiescence separately.
fn input() -> Result<Input, E> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(E::Refused("status-observation-fixture-arguments"));
    }
    let root = fs::canonicalize(PathBuf::from(&args[0])).map_err(input_io)?;
    let work = args[1]
        .to_str()
        .ok_or(E::Refused("status-observation-fixture-work"))?;
    let operator = args[2]
        .to_str()
        .ok_or(E::Refused("status-observation-fixture-operator"))?;
    let path = root.join(".swarm/verification-status-contract.json");
    let metadata = fs::symlink_metadata(&path).map_err(input_io)?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::getuid().as_raw()
        || metadata.mode() & 0o077 != 0
    {
        return Err(E::Refused("status-observation-fixture-marker"));
    }
    let raw = fs::read(&path).map_err(input_io)?;
    let value: Input =
        serde_json::from_slice(&raw).map_err(|e| E::Encode(swarm_engine::WorkError::Json(e)))?;
    if value.schema_version != 1
        || value.kind != "owned-inactive-status-fixture"
        || value.root != root
        || value.work != work
        || value.operator != operator
    {
        return Err(E::Refused("status-observation-fixture-identity"));
    }
    Ok(value)
}
/// Observe real retained inputs, spending and ordinary no-Agent task results,
/// close Root, then publish only selected fields and safe semantic cause summaries.
/// Known freshness/result causes remain owned until after timestamp, spending,
/// encoding, Root close and output/flush finish; any late failure retains them.
/// Safe summaries may include separate repeated observations, never raw Display.
/// Both read/projection and close failures survive. No private Work/Agent/event
/// objects are serialized. Counts/revision describe this read, not a full status.
fn run() -> Result<(), E> {
    let mut observed_causes = Vec::new();
    let outcome = (|| {
        let input = input()?;
        let store = RootStore::open(&input.root, false)?;
        let result = (|| {
            let inputs = store.mission_status_inputs(&input.work, &input.operator)?;
            // Separate fenced observation, not one atomic whole-status snapshot.
            let mut freshness = store.mission_status_freshness(&input.work)?;
            for facts in freshness.values_mut() {
                observed_causes.append(&mut facts.causes);
            }
            let mut results = store.mission_status_results(&input.work)?;
            observed_causes.append(&mut results.causes);
            let result_causes: Vec<_> = observed_causes
                .iter()
                .map(|cause| diagnostics(cause))
                .collect();
            let at = swarm_engine::system_timestamp().map_err(E::Runtime)?;
            let spending = swarm_engine::mission_status_spending::mission_spending(&inputs, &at)?;
            let value = json!({"schema_version":1,"work":inputs.work.id,"revision":inputs.work.revision,
            "task_count":inputs.work.tasks.as_ref().map_or(0,Vec::len),"agent_count":inputs.agents.len(),
            "event_count":inputs.events.len(),"planning_call_count":inputs.planning_calls.len(),
            "exchange_count":inputs.exchanges.len(),"visit_present":inputs.visit.is_some(),"freshness_task_count":freshness.len(),"spending":spending,"task_results":results.results,"task_result_causes":result_causes,
            "complete_mission_status":false,"observation_is_permission":false});
            serde_json::to_vec(&value).map_err(|e| E::Encode(swarm_engine::WorkError::Json(e)))
        })();
        let bytes = match (result, store.close()) {
            (Ok(bytes), Ok(())) => bytes,
            (Err(error), Ok(())) => return Err(error),
            (Ok(_), Err(cleanup)) => return Err(cleanup),
            (Err(primary), Err(cleanup)) => {
                return Err(E::Cleanup {
                    primary: Some(Box::new(primary)),
                    cleanup: Box::new(cleanup),
                })
            }
        };
        let mut output = std::io::stdout().lock();
        output
            .write_all(&bytes)
            .and_then(|()| output.write_all(b"\n"))
            .and_then(|()| output.flush())
            .map_err(io_error)
    })();
    outcome.map_err(|mut primary| {
        for cause in observed_causes {
            primary = E::Cleanup {
                primary: Some(Box::new(primary)),
                cleanup: Box::new(cause),
            };
        }
        primary
    })
}
/// Verification entry point: errors retain typed causes in a safely rendered tree.
fn main() -> Result<(), Failure> {
    run().map_err(Failure)
}
