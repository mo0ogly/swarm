//! Reader for agent histories retained in the import archive.
//!
//! [`RootStore::archived_agent_history`] decodes a retained
//! `imports/<id>/manifest.json` as a historical record. The lookup consults
//! no current `Work`, so an entry stays readable after its `Work` is removed.
//! Reads are scoped by `material_path::with_material_read`; the
//! physical-association witnesses that module can establish are Linux-only,
//! and this reader claims nothing about any other platform. It grants no
//! authority over the archive or current state and permits no later action.

use super::material_path::with_material_read;
use super::{finished, RootStore, RootStoreError};

impl RootStore {
    /// Return the archived agent history retained for `id`.
    ///
    /// `id` is validated with `swarm_identity_contract::name` before any
    /// archive access; an invalid name is refused without opening anything.
    /// The history is then read from `imports/<id>/manifest.json` under
    /// `material_path::with_material_read` and decoded with
    /// `crate::archive_history::decode`; decode failures surface as
    /// [`RootStoreError::Encode`]. Nothing is created, no database is opened,
    /// and no archive state is mutated. Memory follows manifest size, with no
    /// hidden read ceiling. Returned history is immediately stale, not a grant.
    ///
    /// Each directory opened for the lookup is closed on every path following
    /// a successful open, with primary and cleanup causes preserved together
    /// by `finished`. Files opened by `LocalDirectory::read` are checked and
    /// closed by that call itself.
    pub fn archived_agent_history(
        &self,
        id: &str,
    ) -> Result<crate::archive_history::ArchivedAgentHistory, RootStoreError> {
        if !swarm_identity_contract::name(id.as_bytes()) {
            return Err(RootStoreError::Refused("agent-history-id-invalid"));
        }
        with_material_read(self, || {
            let imports = self
                .directory
                .child("imports", false)
                .map_err(RootStoreError::Storage)?;
            let result = (|| {
                let entry = imports.child(id, false).map_err(RootStoreError::Storage)?;
                let decoded = (|| {
                    let raw = entry
                        .read("manifest.json")
                        .map_err(RootStoreError::Storage)?;
                    crate::archive_history::decode(&raw).map_err(RootStoreError::Encode)
                })();
                finished(decoded, entry.close().map_err(RootStoreError::Storage))
            })();
            finished(result, imports.close().map_err(RootStoreError::Storage))
        })
    }
}
