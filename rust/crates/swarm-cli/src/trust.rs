//! Local trust entry point. No native mission launch or external effect.
use crate::{write_output, Arguments, CliError};
use std::{
    io::{self, Write},
    path::Path,
};
/// Validate the fixed namespace, read bounded input, run the shared engine and
/// close Root before writing. Output/close failure retains known publication;
/// the bearer comes only from SWARM_TRUST_TOKEN and is never rendered.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 2
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    let op = a.pos[1].as_str();
    if !matches!(
        op,
        "show"
            | "export"
            | "anchor"
            | "verify"
            | "config-preview"
            | "config-apply"
            | "direct-write"
            | "elevate"
            | "elevate-integrity"
            | "mandate-create"
            | "mandate-revoke"
            | "request"
            | "approve"
            | "commit"
            | "reconcile"
            | "read-object"
            | "read-original"
            | "audit-transaction"
            | "public-anchor" | "public-verify" | "evidence-key-prepare" | "evidence-key-activate" | "evidence-key-revoke"
    ) {
        return Err(CliError::Usage);
    }
    let read = matches!(op, "show" | "export" | "anchor");
    let raw = if read {
        if !a.input.is_empty() {
            return Err(CliError::Usage);
        }
        vec![]
    } else {
        if a.input.is_empty() {
            return Err(CliError::Usage);
        }
        (if a.input == "-" {
            swarm_engine::read_command_document(&mut io::stdin().lock())
        } else {
            swarm_engine::read_command_file(Path::new(&a.input))
        })
        .map_err(CliError::CommandInput)?
    };
    if matches!(op,"verify"|"public-verify") {
        let bytes = (if op=="verify"{swarm_engine::verify_trust(&raw)}else{swarm_engine::verify_public_trust(&raw)}).map_err(|e| CliError::Trust(Box::new(e)))?;
        write_output(out, &bytes)?;
        return Ok(0);
    }
    let readonly = read || matches!(op, "config-preview" | "read-object" | "read-original" | "public-anchor");
    let store = swarm_engine::RootStore::open(Path::new(&a.root), !readonly)
        .map_err(|e| CliError::Root(Box::new(e)))?;
    let token = std::env::var("SWARM_TRUST_TOKEN").unwrap_or_default();
    let result = store
        .trust(op, &raw, &token)
        .map_err(|e| CliError::Root(Box::new(e)));
    let publication = result.as_ref().ok().and_then(|r| r.publication);
    let closed = store.close();
    let completed = match (result, closed) {
        (Ok(reply), Ok(())) => Ok(reply),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(CliError::Root(Box::new(e))),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    };
    let reply = completed.map_err(|e| preserve(e, publication))?;
    write_output(out, &reply.bytes).map_err(|e| preserve(e, publication))?;
    Ok(0)
}
/// Attach exact known outcome without converting a late failure into success.
fn preserve(e: CliError, publication: Option<&'static str>) -> CliError {
    match publication {
        Some(publication) => CliError::TrustPublication {
            primary: Box::new(e),
            publication,
        },
        None => e,
    }
}
