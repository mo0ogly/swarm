//! Historical budget reads and advisory preview; apply remains in its own path.
use super::{json_bytes, write_output, Arguments, CliError};
use std::{io::{self, Write}, path::Path};

/// Validate transport before readonly Root access; preview consumes only bounded
/// input and validates the monetary intent before opening. Emit historical JSON
/// in both modes, matching the prior CLI. Close before publishing and preserve
/// simultaneous read/close plus write/flush errors. No mutation or permission.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3 || a.pos[0] != "budget" || !matches!(a.pos[1].as_str(), "show" | "preview")
        || !a.output.is_empty() || a.server || !a.task.is_empty() || a.phase != "delivery" {
        return Err(CliError::Usage);
    }
    let root = |e| CliError::Root(Box::new(e));
    let raw = if a.pos[1] == "preview" {
        if a.input.is_empty() { return Err(CliError::Usage); }
        let raw = if a.input == "-" { swarm_engine::read_command_document(&mut io::stdin().lock()) }
            else { swarm_engine::read_command_file(Path::new(&a.input)) }.map_err(CliError::CommandInput)?;
        let change = swarm_engine::decode_budget_change(&raw)
            .and_then(|v| { swarm_engine::validate_budget_change(&v)?; Ok(v) })
            .map_err(|e| root(swarm_engine::RootStoreError::Mutation(swarm_engine::WorkEngineError::Budget(e))))?;
        drop(change);
        Some(raw)
    } else {
        if !a.input.is_empty() { return Err(CliError::Usage); }
        None
    };
    let store = swarm_engine::RootStore::open(Path::new(&a.root), false).map_err(root)?;
    let result = if let Some(raw) = raw {
        store.preview_historical_budget(&a.pos[2], &raw).map_err(root)
            .and_then(|v| json_bytes(&v).map_err(CliError::Output))
    } else {
        store.historical_budget_show(&a.pos[2]).map_err(root)
            .and_then(|v| json_bytes(&v).map_err(CliError::Output))
    };
    match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup { primary: Box::new(primary), cleanup: Box::new(cleanup) }),
    }
}
