//! Imported historical telemetry, never a current Agent or execution command.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::{io::Write, path::Path};

/// Validate one archive id before opening Root; emit historical Go JSON even in
/// plain CLI mode. The Root reader owns private descriptor/association checks,
/// validates the full Bundle type graph and returns only historical cockpit data.
/// No Work lookup/adoption, bookmark, provider, signal or SQL write occurs. Missing
/// or invalid archives refuse, not an empty successful list. Decode/encode/Root
/// close/write/flush errors preserve their causes, including both cleanup paths.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[1] != "history"
        || !swarm_identity_contract::name(a.pos[2].as_bytes())
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
    let result = store
        .archived_agent_history(&a.pos[2])
        .map_err(root)
        .and_then(|v| json_bytes(&v).map_err(CliError::Output));
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => Ok(bytes),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cleanup)) => Err(root(cleanup)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    write_output(out, &result?).map(|()| 0)
}
