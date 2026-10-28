//! Shared descriptor-bound private document IO. Authentication and interpretation
//! remain consumers' responsibility; these functions never infer authority.
use super::*;
use std::io::{Read, Write};
/// Observed publication phase, never inferred from an Err alone. Directory fsync
/// acknowledges durability to the local filesystem, not hardware/remote recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    /// This operation has not called rename; original destination is untouched.
    NotAttempted,
    /// Rename was attempted but did not acknowledge success. Inspect/recover;
    /// this conservative category makes no claim of non-publication.
    Unknown,
    /// Rename acknowledged success; directory durability is not acknowledged.
    Visible,
    /// Rename and directory fsync acknowledged success. Later cleanup may fail.
    Durable,
}
/// Require the retained lock to remain the current named regular0600single-link
/// inode. A pathname alone cannot bind exclusion after lockfile substitution.
pub(super) fn check_handles(h: &Handles, owner: u32, lock_name: &str) -> Result<(), Error> {
    safe_directory(&h.dir, owner)?;
    safe_file(&h.lock, owner)?;
    let held = unix::fstat(&h.lock).map_err(|e| Error::io(IoClass::Path, e))?;
    let current = unix::statat(&h.dir, lock_name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|e| Error::io(IoClass::Path, e))?;
    if held.st_dev != current.st_dev || held.st_ino != current.st_ino {
        return Err(Error::Path);
    }
    Ok(())
}
/// Read an existing regular0600/single-link file under the retained directory.
/// Missing is NotConfigured; preserve read and explicit close/fixture failures.
/// No empty fallback, schema interpretation, automatic size policy or path reopen.
pub(super) fn read_document(
    dir: &File,
    owner: u32,
    name: &str,
    hook: &impl Fn(Stage, IoClass) -> Result<(), Error>,
) -> Result<Vec<u8>, Error> {
    let fd = unix::openat(
        dir,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|e| {
        if e == Errno::NOENT {
            Error::NotConfigured
        } else {
            Error::io(IoClass::Path, e)
        }
    })?;
    let mut file = File::from(fd);
    let result = (|| {
        safe_file(&file, owner)?;
        let mut raw = vec![];
        file.read_to_end(&mut raw)
            .map_err(|e| Error::io(IoClass::Storage, e))?;
        Ok(raw)
    })();
    let closed = combined(
        close(file, IoClass::Storage),
        hook(Stage::ReadClose, IoClass::Storage),
    );
    combined(result, closed)
}
/// Flush this retained directory. Refusals preserve an indeterminate IO cause;
/// no global deadline, retry or cancellation thread is introduced.
pub(super) fn sync_directory(
    dir: &File,
    hook: &impl Fn(Stage, IoClass) -> Result<(), Error>,
) -> Result<(), Error> {
    hook(Stage::DirectorySync, IoClass::Indeterminate)?;
    unix::fsync(dir).map_err(|e| Error::io(IoClass::Indeterminate, e))
}
/// Publish exact bytes with a private same-directory random temporary. The
/// caller holds its own reservation and validates destination/name/schema.
/// create uses NOREPLACE. Always attempt explicit close and temp cleanup; preserve
/// both causes. phase changes only at actual rename/fsync acknowledgements and
/// remains meaningful when the returned result is Err. No original is truncated.
pub(super) fn persist_document(
    dir: &File,
    owner: u32,
    destination: &str,
    raw: &[u8],
    create: bool,
    phase: &mut Publication,
    hook: &impl Fn(Stage, IoClass) -> Result<(), Error>,
) -> Result<(), Error> {
    *phase = Publication::NotAttempted;
    safe_directory(dir, owner)?;
    let mut nonce = [0; 16];
    hook(Stage::Entropy, IoClass::Storage)?;
    getrandom::getrandom(&mut nonce)
        .map_err(|e| Error::io(IoClass::Storage, std::io::Error::other(e)))?;
    let name = format!(
        ".config-{}",
        nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let fd = unix::openat(
        dir,
        name.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|e| Error::io(IoClass::Storage, e))?;
    let mut file = File::from(fd);
    let result = (|| {
        safe_file(&file, owner)?;
        hook(Stage::Write, IoClass::Storage)?;
        if hook(Stage::PartialWrite, IoClass::Storage).is_err() {
            file.write_all(&raw[..raw.len() / 2])
                .map_err(|e| Error::io(IoClass::Storage, e))?;
            hook(Stage::PartialWrite, IoClass::Storage)?;
        }
        file.write_all(raw)
            .map_err(|e| Error::io(IoClass::Storage, e))?;
        hook(Stage::FileSync, IoClass::Storage)?;
        file.sync_all().map_err(|e| Error::io(IoClass::Storage, e))
    })();
    let written = combined(
        result,
        combined(
            close(file, IoClass::Storage),
            hook(Stage::FileClose, IoClass::Storage),
        ),
    );
    let result = written.and_then(|()| {
        *phase = Publication::Unknown;
        let rename = if create {
            unix::renameat_with(dir, name.as_str(), dir, destination, RenameFlags::NOREPLACE)
        } else {
            unix::renameat(dir, name.as_str(), dir, destination)
        };
        rename.map_err(|e| {
            if e == Errno::EXIST {
                Error::Exists
            } else {
                Error::io(IoClass::Storage, e)
            }
        })?;
        *phase = Publication::Visible;
        sync_directory(dir, hook)?;
        *phase = Publication::Durable;
        Ok(())
    });
    let cleaned = hook(Stage::Unlink, IoClass::Storage).and_then(|()| {
        match unix::unlinkat(dir, name.as_str(), AtFlags::empty()) {
            Ok(()) | Err(Errno::NOENT) => Ok(()),
            Err(e) => Err(Error::io(IoClass::Storage, e)),
        }
    });
    combined(result, cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    /// Real NOREPLACE refusal after a valid write: unknown acknowledgement stays
    /// explicit and exact existing bytes remain unchanged. No fault hook needed.
    #[test]
    fn real_noreplace_refusal_keeps_original_and_reports_attempted_publication() {
        let root = tempfile::TempDir::new().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let owner = rustix::process::geteuid().as_raw();
        let dir = open_private_directory(root.path(), owner).unwrap();
        let hook = |_, _| Ok(());
        let mut phase = Publication::NotAttempted;
        persist_document(
            &dir,
            owner,
            "checkpoint",
            b"witness",
            true,
            &mut phase,
            &hook,
        )
        .unwrap();
        assert_eq!(phase, Publication::Durable);
        assert_eq!(
            read_document(&dir, owner, "checkpoint", &hook).unwrap(),
            b"witness"
        );
        let e = persist_document(&dir, owner, "checkpoint", b"bad", true, &mut phase, &hook)
            .unwrap_err();
        assert_eq!(e.code(), "exists");
        assert_eq!(phase, Publication::Unknown);
        assert_eq!(
            read_document(&dir, owner, "checkpoint", &hook).unwrap(),
            b"witness"
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        close(dir, IoClass::Storage).unwrap();
    }
}
