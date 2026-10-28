//! Public local mission quota configuration; no provider run or authenticated transport.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::{self, Write};
/// Show recorded quotas, preview without writes, or apply one versioned
/// change. Strict input precedes Root opening; durable replay/commit guards are
/// shared with other Work mutations. Close/output/flush errors preserve committed
/// state and causes. Consumption and unlisted allocations survive; local actor/time is not authentication or launch permission.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
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
    let root_error = |e| CliError::Root(Box::new(e));
    if let Some(raw) = input.as_deref() {
        swarm_engine::decode_quota_change(raw)
            .map_err(|e| root_error(swarm_engine::RootStoreError::Launch(Box::new(e))))?;
    }
    let mut store = swarm_engine::RootStore::open(std::path::Path::new(&a.root), op == "apply")
        .map_err(root_error)?;
    let mut committed = None;
    let result = (|| match op {
        "show" => {
            json_bytes(&store.quotas(&a.pos[2]).map_err(root_error)?).map_err(CliError::Output)
        }
        "preview" => json_bytes(
            &store
                .preview_quotas(
                    &a.pos[2],
                    input.as_deref().ok_or(CliError::Usage)?,
                    &mut swarm_engine::SystemEnvironment,
                )
                .map_err(root_error)?,
        )
        .map_err(CliError::Output),
        "apply" => {
            let outcome = store
                .execute_external(
                    &a.pos[2],
                    "quotas.configure",
                    input.as_deref().ok_or(CliError::Usage)?,
                    &mut swarm_engine::SystemEnvironment,
                )
                .map_err(root_error)?;
            let work = outcome.result.work;
            committed = Some((work.id.clone(), work.revision));
            json_bytes(&work).map_err(CliError::Output)
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
        if let Some((work, revision)) = committed.or_else(|| primary.recorded_work()) {
            CliError::PlanningStatePreserved {
                primary: Box::new(primary),
                work,
                revision,
                english: a.language == "en",
            }
        } else {
            primary
        }
    })
}
