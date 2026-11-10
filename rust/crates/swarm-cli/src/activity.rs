//! Explicit historical activity page using saved policy, no visit or execution.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::{
    io::{self, Write},
    path::Path,
};
/// Validate explicit Work/query before opening readonly Root, then obtain a
/// source-bound page. Typed query/read/codec/rollback/Root-close/output/flush
/// failures stay errors, never empty history or inferred business permission.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[1] != "show"
        || !swarm_identity_contract::name(a.pos[2].as_bytes())
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root = |e| CliError::Root(Box::new(e));
    let q = if a.input.is_empty() {
        swarm_engine::ActivityReadInput {
            schema_version: 1,
            before: String::new(),
            limit: None,
            decisions_only: false,
        }
    } else {
        let raw = if a.input == "-" {
            swarm_engine::read_command_document(&mut io::stdin().lock())
        } else {
            swarm_engine::read_command_file(Path::new(&a.input))
        }
        .map_err(CliError::CommandInput)?;
        swarm_engine::decode_activity_read_input(&raw)
            .map_err(|e| root(swarm_engine::RootStoreError::Launch(Box::new(e))))?
    };
    if q.schema_version != 1 || q.limit.is_some_and(|n| n < 0) {
        return Err(CliError::Usage);
    }
    let store = swarm_engine::RootStore::open(Path::new(&a.root), false).map_err(root)?;
    let result = store
        .activity(&a.pos[2], &q.before, q.limit, q.decisions_only)
        .map_err(root)
        .and_then(|v| json_bytes(&v).map_err(CliError::Output));
    match (result, store.close()) {
        (Ok(v), Ok(())) => write_output(out, &v).map(|()| 0),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
