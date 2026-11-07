//! Versioned-memory read surface: `swarm memory list` (Go `projectMemoryCLI`
//! list branch). Mutation subcommands and M20 summary-source remain unported —
//! they are refused with usage instead of inventing their wire behavior here.
use crate::{Arguments, CliError, json_bytes, write_output};
use std::io::Write;
use std::path::Path;
use swarm_engine::RootStore;

/// Run the guarded read-only catalog listing. No input, no output file, no
/// server route, delivery phase only. JSON stays a typed document; human output
/// adds a localized heading. Read/encode/close causes refuse, never empty
/// success; no admission, selection or authority follows.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    let kind = a.pos.get(1).map(String::as_str).ok_or(CliError::Usage)?;
    if kind != "list"
        || a.pos.len() != 2
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root_error = |e| CliError::Root(Box::new(e));
    let store = RootStore::open(Path::new(&a.root), false).map_err(root_error)?;
    let result = (|| {
        let bytes =
            json_bytes(&store.memory_catalog("").map_err(root_error)?).map_err(CliError::Output)?;
        if a.json {
            return Ok(bytes);
        }
        let mut human = if a.language == "en" {
            b"Memory catalog\n".to_vec()
        } else {
            "Catalogue mémoire\n".as_bytes().to_vec()
        };
        human.extend(bytes);
        Ok(human)
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(CliError::Root(Box::new(error))),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0)
}
