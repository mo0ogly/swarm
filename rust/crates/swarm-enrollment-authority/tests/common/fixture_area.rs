//! Governed operator fallback fixture area for tests in this crate.
//!
//! Authority fixtures traverse `open_private_directory` and the private
//! provisioning/installation loaders, whose ancestor walks require every
//! existing ancestor to be private (no group/other writes). The repository
//! fixture area lives inside the Swarm checkout whose ancestors are
//! group-writable, so it cannot host them; the operator fallback area
//! (`~/.local/state/swarm-fixtures`, 0700 chain) realizes the precondition.
//! This module replicates `swarm_engine::fallback_area()` inline because
//! this crate must not depend on swarm-engine.

use std::path::PathBuf;

/// Create (if absent) and return the private operator fallback fixture area.
pub(crate) fn area() -> PathBuf {
    use std::os::unix::fs::DirBuilderExt;
    let home = std::env::var_os("HOME")
        .unwrap_or_else(|| panic!("HOME required for the operator fallback fixture area"));
    let area = std::path::Path::new(&home).join(".local/state/swarm-fixtures");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&area)
        .unwrap_or_else(|e| panic!("fallback fixture area {} cannot be created: {e}", area.display()));
    area
}
