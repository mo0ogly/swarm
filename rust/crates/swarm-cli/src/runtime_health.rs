//! Explicit storage/version observation; no business or launch decision.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::{io::Write, path::Path};
/// Bootstrap a readonly actual Root/binding, close the SQL connection, then reuse
/// the same descriptor-bound HTTP producer. The initial local CLI open requires
/// readable SQLite; an already-bound HTTP server does not. No query/input/output
/// file or server substitution is accepted. Safe diagnostics are serialized and
/// original ancillary causes survive encoding, binding-close and output/flush
/// errors. Observation is not permission, SQL integrity or a mission completion.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 1
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root = |e| CliError::Root(Box::new(e));
    let store = swarm_engine::RootStore::open(Path::new(&a.root), false).map_err(root)?;
    let binding = match (store.bind_root(), store.close()) {
        (Ok(binding), Ok(())) => binding,
        (Err(e), Ok(())) => return Err(root(e)),
        (Err(primary), Err(cleanup)) => {
            return Err(CliError::RootCleanup {
                primary: Box::new(root(primary)),
                cleanup: Box::new(cleanup),
            })
        }
        (Ok(binding), Err(primary)) => {
            return match binding.close() {
                Ok(()) => Err(root(primary)),
                Err(cleanup) => Err(CliError::RootCleanup {
                    primary: Box::new(root(primary)),
                    cleanup: Box::new(cleanup),
                }),
            }
        }
    };
    let read = binding.runtime_health(Path::new(&a.root), &mut swarm_engine::SystemEnvironment);
    let mut causes = Vec::new();
    let bytes = read.map_err(root).and_then(|read| {
        causes = read.causes;
        json_bytes(&read.view).map_err(CliError::Output)
    });
    let result = match (bytes, binding.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(root(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    crate::mission_status::retain_observation_causes(result, causes)
}
