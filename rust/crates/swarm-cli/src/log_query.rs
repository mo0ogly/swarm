//! Explicit filtered log read, not provider/process control or retention.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::{
    io::{self, Write},
    path::Path,
};
/// Validate Work/Agent and query syntax before Root IO; obtain one saved-policy
/// snapshot page, then checked Root close, encode/output/flush. Typed failures
/// retain causes and never become an empty successful history. No provider,
/// mission, log publication, retention, budget or permission change occurs.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 4
        || a.pos[1] != "query"
        || !swarm_identity_contract::name(a.pos[2].as_bytes())
        || !swarm_identity_contract::name(a.pos[3].as_bytes())
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root = |e| CliError::Root(Box::new(e));
    let q = if a.input.is_empty() {
        swarm_engine::LogSearchInput {
            schema_version: 1,
            after: 0,
            query: String::new(),
            kind: String::new(),
        }
    } else {
        let raw = if a.input == "-" {
            swarm_engine::read_command_document(&mut io::stdin().lock())
        } else {
            swarm_engine::read_command_file(Path::new(&a.input))
        }
        .map_err(CliError::CommandInput)?;
        swarm_engine::decode_log_search_input(&raw)
            .map_err(|e| root(swarm_engine::RootStoreError::Launch(Box::new(e))))?
    };
    let store = swarm_engine::RootStore::open(Path::new(&a.root), false).map_err(root)?;
    let result = store
        .query_logs(&a.pos[2], &a.pos[3], &q.query, &q.kind, q.after)
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
