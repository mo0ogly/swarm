//! Read-only Linux diagnostic; no Root, input selection, subprocess or activation.
use super::{json_bytes, write_output, Arguments, CliError};
use std::io::Write;
/// Emit the shared version2 report for this CLI process only. Require explicit
/// JSON, no positional/input/project/output options; parser also rejects explicit
/// default-valued forbidden flags and duplicates. Missing/invalid prerequisites
/// remain visible report data, never launch permission. Encode before write;
/// original serialization/write/flush errors propagate without a success claim.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 2
        || !a.json
        || !a.input.is_empty()
        || a.root != "."
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let report = swarm_engine::linux_prerequisites::observe();
    let bytes = json_bytes(&report).map_err(CliError::Output)?;
    write_output(out, &bytes)?;
    Ok(0)
}
