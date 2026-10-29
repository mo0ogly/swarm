//! Mandatory I0 filesystem boundary for an already-authorized private helper.
//! This module grants no identity, labels, launch permission or acceptance.
use crate::{AgentLaunchError, RootStoreError as E};
use std::path::Path;
use swarm_biba_linux::Sandbox;
#[path = "native_provider_workspace.rs"]
mod workspace;
pub(crate) use workspace::{output_workspace, OutputRole};

/// Retain the original kernel/IO cause and every independent close failure.
/// Static library stages are safe diagnostics; paths and credentials are not
/// copied into an output message. No failure becomes an unconfined fallback.
fn convert(error: swarm_biba_linux::Error) -> E {
    let (stage, cause, cleanup) = error.into_parts();
    let mut primary = E::Launch(Box::new(AgentLaunchError::caused(stage, cause)));
    for error in cleanup {
        primary = E::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(convert(error)),
        };
    }
    primary
}

/// Prepare inode-bound rules for the validated Root and I0 output workspace.
/// The private gate must have checked ownership/provenance and current rights;
/// a client path is not such proof. This performs filesystem/kernel observations
/// but does not restrict the caller yet. Unsupported hosts and unsafe overlaps
/// refuse with original causes; caller still owes transport/Root cleanup.
pub(crate) fn prepare(root: &Path, workspace: &Path, expected: (u64, u64)) -> Result<Sandbox, E> {
    Sandbox::prepare_bound(root, workspace, expected).map_err(convert)
}

/// Irreversibly restrict this single-threaded, unprivileged helper before exec.
/// Descendants inherit the boundary. Any returned error requires helper exit;
/// retrying or executing without the boundary is forbidden. No effect is accepted.
pub(crate) fn enforce(sandbox: Sandbox) -> Result<(), E> {
    sandbox.enforce().map_err(convert)
}

/// Explicitly release an unused ruleset after transport setup refuses.
/// Preserve a close failure alongside the caller's primary error.
pub(crate) fn close(sandbox: Sandbox) -> Result<(), E> {
    sandbox.close().map_err(convert)
}
