//! Report retained worker cost declarations; never price inference or a debit.
use crate::{json_bytes, write_output, Arguments, CliError};
use std::io::Write;
/// Scope metadata prevents a worker-only projection from being read as a full
/// mission invoice. Flatten preserves the Go-shaped totals and grouped keys.
#[derive(serde::Serialize)]
struct View<'a> {
    scope: &'static str,
    #[serde(flatten)]
    costs: &'a swarm_engine::CostSummary,
}

/// Read costs show WORK from a read-only Root and emit the structured totals.
/// Reject unsupported arguments before opening. Encode and close before writing;
/// read+close failures retain both causes and write/flush errors propagate.
/// Zero reported USD with silent attempts is unknown cost, never free execution.
/// No mutation, provider call, policy adoption or automatic launch is performed.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 3
        || a.pos[1] != "show"
        || !a.input.is_empty()
        || !a.output.is_empty()
        || a.server
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let root_error = |e| CliError::Root(Box::new(e));
    let store =
        swarm_engine::RootStore::open(std::path::Path::new(&a.root), false).map_err(root_error)?;
    let result = store
        .reported_costs(&a.pos[2])
        .map_err(root_error)
        .and_then(|value| {
            json_bytes(&View {
                scope: "retained-worker-attempts",
                costs: &value,
            })
            .map_err(CliError::Output)
        });
    match (result, store.close()) {
        (Ok(bytes), Ok(())) => write_output(out, &bytes).map(|()| 0),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(root_error(error)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
