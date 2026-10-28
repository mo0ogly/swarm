//! Compatible activity display policy and original history; no provider contact.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::{self, Write};
/// Validate syntax/input before Root, then read/history/preview/apply through the
/// same producer as HTTP. Recorded native Change excludes native preview token.
/// Late output/close errors preserve known application/replay state; no Work,
/// provider execution, credential or implicit default adoption is introduced.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[1] != "activity"
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let op = a.pos[2].as_str();
    if !matches!(op, "show" | "history" | "preview" | "apply") {
        return Err(CliError::Usage);
    }
    let root_error = |e| CliError::Root(Box::new(e));
    let change = if matches!(op,"show"|"history") {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        None
    } else {
        if a.input.is_empty() {
            return Err(CliError::Usage);
        }
        let raw = if a.input == "-" {
            swarm_engine::read_command_document(&mut io::stdin().lock())
        } else {
            swarm_engine::read_command_file(std::path::Path::new(&a.input))
        }
        .map_err(CliError::CommandInput)?;
        Some(
            swarm_engine::decode_api_activity_read_change(&raw)
                .map_err(|e| root_error(swarm_engine::RootStoreError::Launch(Box::new(e))))?,
        )
    };
    let store = swarm_engine::RootStore::open(std::path::Path::new(&a.root), op == "apply")
        .map_err(root_error)?;
    let mut changed = false;
    let result = (|| match op {
        "history" => json_bytes(&store.activity_read_policy().map_err(root_error)?.config.history).map_err(CliError::Output),
        "show" => json_bytes(&store.activity_read_policy().map_err(root_error)?)
            .map_err(CliError::Output),
        "preview" => json_bytes(
            &store
                .preview_activity_read_policy(change.as_ref().ok_or(CliError::Usage)?)
                .map_err(root_error)?,
        )
        .map_err(CliError::Output),
        "apply" => {
            let view = store
                .apply_activity_read_policy(change.as_ref().ok_or(CliError::Usage)?)
                .map_err(root_error)?;
            changed = view.applied;
            json_bytes(&view).map_err(CliError::Output)
        }
        _ => Err(CliError::Usage),
    })();
    let result = match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes),
        (Err(e), Ok(())) => Err(e),
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
