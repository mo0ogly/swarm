//! Explicit provider-file bootstrap, metadata, preview and source-bound update.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::{self, Write};
/// Dispatch only local file operations. Strict bounded documents are decoded
/// before opening the Root; current policy, CAS/token and storage enforce writes.
/// Configuration/snapshot metadata grants no readiness, identity or execution.
/// Close/serialization/write/flush failures remain visible and preserve a real
/// completed mutation. No-op/preview does not pretend a new file was written.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() < 3
        || a.pos[1] != "config"
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let op = a.pos[2].as_str();
    if !matches!(
        op,
        "show" | "init" | "inspect" | "backups" | "preview" | "apply"
    ) {
        return Err(CliError::Unsupported);
    }
    if a.pos.len() != 3 && !(op == "inspect" && a.pos.len() == 4) {
        return Err(CliError::Usage);
    }
    let root_error = |e| CliError::Root(Box::new(e));
    let document = if matches!(op, "init" | "preview" | "apply") {
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
    } else {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        None
    };
    let format_error = |e| root_error(swarm_engine::RootStoreError::Providers(e));
    let config = if op == "init" {
        Some(
            swarm_engine::decode_provider_configuration(
                document.as_deref().ok_or(CliError::Usage)?,
            )
            .map_err(format_error)?,
        )
    } else {
        None
    };
    let change = if matches!(op, "preview" | "apply") {
        Some(
            swarm_engine::decode_provider_configuration_change(
                document.as_deref().ok_or(CliError::Usage)?,
            )
            .map_err(format_error)?,
        )
    } else {
        None
    };
    let store = swarm_engine::RootStore::open(
        std::path::Path::new(&a.root),
        matches!(op, "init" | "apply"),
    )
    .map_err(root_error)?;
    let mut changed = false;
    let result = (|| match op {
        "init" => {
            let view = store
                .initialize_provider_configuration(config.as_ref().ok_or(CliError::Usage)?)
                .map_err(root_error)?;
            changed = true;
            json_bytes(&view).map_err(CliError::Output)
        }
        "show" => json_bytes(&store.provider_configuration().map_err(root_error)?)
            .map_err(CliError::Output),
        "inspect" => json_bytes(
            &store
                .inspect_provider_configuration(a.pos.get(3).map(String::as_str))
                .map_err(root_error)?,
        )
        .map_err(CliError::Output),
        "backups" => json_bytes(&store.provider_configuration_backups().map_err(root_error)?)
            .map_err(CliError::Output),
        "preview" => json_bytes(
            &store
                .preview_provider_configuration(change.as_ref().ok_or(CliError::Usage)?)
                .map_err(root_error)?,
        )
        .map_err(CliError::Output),
        "apply" => {
            let view = store
                .apply_provider_configuration(change.as_ref().ok_or(CliError::Usage)?)
                .map_err(root_error)?;
            changed = view.changed;
            json_bytes(&view).map_err(CliError::Output)
        }
        _ => Err(CliError::Unsupported),
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(e)) => Err(root_error(e)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    result.map(|()| 0).map_err(|primary| {
        if changed {
            CliError::ConfigurationStatePreserved {
                primary: Box::new(primary),
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
