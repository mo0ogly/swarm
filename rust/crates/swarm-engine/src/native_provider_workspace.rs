//! Engine-labelled I0 output directories for tool-free registered helpers.
use crate::{local_storage::LocalDirectory, AgentLaunchError as E};
use crate::agent_launch_error::finish;
/// Fixed protocol namespaces, not an operator path or execution policy.
pub(crate) enum OutputRole { Planner, Reviewer }
impl OutputRole {
    /// Stable namespace under the owned private Root; grants no authority.
    fn name(&self) -> &'static str { match self { Self::Planner => "planning", Self::Reviewer => "reviewer" } }
}
/// Open an engine-labelled I0 output leaf relative to the owned private Root.
/// Only the verified request/claim id selects a leaf; no client path is accepted. The
/// initial run may create private directories; gate/controller rechecks only open
/// existing leaves. Existing leaf bytes remain I0 and confer no authority. Host
/// preparation records its inode in the run scope; later replacement must refuse.
/// Creation/sync/open/close failures retain causes; an error can leave I0 output
/// visible, never a provider launch, accepted result or implied rollback.
pub(crate) fn output_workspace(directory: &LocalDirectory, role: OutputRole, claim: &str, create: bool) -> Result<String, E> {
    let cause = |e| E::caused("native-i0-workspace", e);
    let base = directory.child("biba-i0", create).map_err(cause)?;
    let result = (|| {
        let scope = base.child(role.name(), create).map_err(cause)?;
        let result = (|| {
            let leaf = scope.child(claim, create).map_err(cause)?;
            leaf.close().map_err(cause)
        })();
        finish(result, scope.close().map_err(cause))
    })();
    finish(result, base.close().map_err(cause))?;
    Ok(format!(".swarm/biba-i0/{}/{claim}", role.name()))
}
