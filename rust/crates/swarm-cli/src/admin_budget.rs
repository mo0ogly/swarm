//! Dedicated Admin envelope CLI; no provider invocation or execution grant.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::{self, Write};
/// Use Root's shared budget show/preview/apply contract with bounded explicit
/// input. Known/replayed or uncertain publication survives encoding, Root close,
/// output and flush errors. No shadow validation, default budget or retry.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 2
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let op = a.pos[1].as_str();
    if !matches!(op, "show" | "preview" | "apply") {
        return Err(CliError::Usage);
    }
    let input = if op == "show" {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        None
    } else {
        if a.input.is_empty() {
            return Err(CliError::Usage);
        }
        Some(
            if a.input == "-" {
                swarm_engine::read_command_document(&mut io::stdin().lock())
            } else {
                swarm_engine::read_command_file(std::path::Path::new(&a.input))
            }
            .map_err(CliError::CommandInput)?,
        )
    };
    let root_error = |error| CliError::Root(Box::new(error));
    let mut store = swarm_engine::RootStore::open(std::path::Path::new(&a.root), op == "apply")
        .map_err(root_error)?;
    let mut known = false;
    let result = (|| match op {
        "show" => {
            json_bytes(&store.admin_advice_budget().map_err(root_error)?).map_err(CliError::Output)
        }
        "preview" => json_bytes(
            &store
                .preview_admin_advice_budget(input.as_deref().ok_or(CliError::Usage)?)
                .map_err(root_error)?,
        )
        .map_err(CliError::Output),
        "apply" => {
            let reply = store
                .apply_admin_advice_budget(
                    input.as_deref().ok_or(CliError::Usage)?,
                    &mut swarm_engine::SystemEnvironment,
                )
                .map_err(root_error)?;
            known = true;
            json_bytes(&reply).map_err(CliError::Output)
        }
        _ => Err(CliError::Usage),
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(root_error(error)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if known {
            CliError::ConfigurationStatePreserved {
                primary: Box::new(primary),
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
