//! Go-compatible explicit historical exchange list; never an exchange command.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::{io::Write, path::Path};
/// List one explicit Work with no input/output redirection/server/task/phase
/// ambiguity. Root opens readonly; SQL snapshot and Root close complete before
/// writing/flushing Go-compatible JSON values; byte identity is not claimed. No send/ACK, provider, consent or mutation.
/// Failed input/read/encode/close/output preserves its cause, not an empty list.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[1] != "list"
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    if !swarm_identity_contract::name(a.pos[2].as_bytes()) {
        return Err(CliError::Usage);
    }
    let root = |e| CliError::Root(Box::new(e));
    let store = swarm_engine::RootStore::open(Path::new(&a.root), false).map_err(root)?;
    let result = store
        .agent_exchanges(&a.pos[2])
        .map_err(root)
        .and_then(|v| json_bytes(&v).map_err(CliError::Output));
    match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
