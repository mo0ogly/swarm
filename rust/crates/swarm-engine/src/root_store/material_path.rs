//! Association checks for material read through a project pathname.
//! Linux retains physical datastore witnesses; other Unix targets preserve the
//! historical reservation-only contract and establish no physical association.
use super::{finished, RootStore, RootStoreError};
#[cfg(target_os = "linux")]
use super::{LocalDirectory, RootBinding};

#[cfg(target_os = "linux")]
/// Require the project pathname used by artifact readers to name the retained
/// datastore directory and database. Reopen only the private .swarm directory;
/// database_identity uses an O_PATH witness, never a second SQLite data FD that
/// could disturb locks. Held original descriptors prevent original inode reuse.
/// Every observation/close cause survives. This checks association at this point,
/// not an atomic filesystem snapshot, pathname lock or same-UID confinement.
pub(super) fn verify_material_path(
    root: &RootStore,
    binding: &RootBinding,
) -> Result<(), RootStoreError> {
    let directory =
        LocalDirectory::open(&root.root.join(".swarm")).map_err(RootStoreError::Storage)?;
    let result = (|| {
        let expected = binding.instance()?;
        if directory.identity().map_err(RootStoreError::Storage)? != expected.directory
            || directory
                .database_identity()
                .map_err(RootStoreError::Storage)?
                != expected.database
        {
            return Err(RootStoreError::Refused("material-project-instance-changed"));
        }
        Ok(())
    })();
    finished(result, directory.close().map_err(RootStoreError::Storage))
}

/// Read under the saved-settings reservation and, on Linux, verify project
/// material still names the actual opened datastore before and after the read.
/// The callback may inspect files but must not mutate state or retain authority.
/// Returns its observation only when checks and explicit closes succeed; primary,
/// post-read and cleanup causes are preserved together. No retries, new policy,
/// atomic filesystem snapshot, pathname lock or later-action permission follow.
/// Other Unix targets retain their existing reservation-only behavior; Linux
/// physical identity support must not be inferred for those targets.
pub(super) fn with_material_read<T>(
    root: &RootStore,
    read: impl FnOnce() -> Result<T, RootStoreError>,
) -> Result<T, RootStoreError> {
    let reservation = root
        .directory
        .reserve(false, false)
        .map_err(RootStoreError::Storage)?;
    #[cfg(target_os = "linux")]
    let result = (|| {
        let binding = root.bind_root()?;
        let observed = (|| {
            verify_material_path(root, &binding)?;
            let value = read();
            let checked = (|| {
                verify_material_path(root, &binding)?;
                if !binding.matches(root)? {
                    return Err(RootStoreError::Refused("material-opened-instance-changed"));
                }
                Ok(())
            })();
            finished(value, checked)
        })();
        finished(observed, binding.close())
    })();
    #[cfg(not(target_os = "linux"))]
    let result = read();
    finished(result, reservation.close().map_err(RootStoreError::Storage))
}
