//! Readonly lifecycle inventory, never permission to restore/delete a mission.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::Write;
/// No target or command document, phase, server or output file.
/// Root owns snapshot/physical checks; preserve original read/close/output/flush
/// errors without a successful empty list or any mutation/provider operation.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 2
        || a.pos[1] != "list"
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root = |e| CliError::Root(Box::new(e));
    let store =
        swarm_engine::RootStore::open(std::path::Path::new(&a.root), false).map_err(root)?;
    let result = store
        .lifecycle_missions()
        .map_err(root)
        .and_then(|value| json_bytes(&value).map_err(CliError::Output));
    let result = match (result, store.close()) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    write_output(out, &result?).map(|()| 0)
}
