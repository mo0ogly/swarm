use super::*;
use rustix::{
    fd::AsFd,
    fs::{self as unix, FileType, Mode, OFlags},
};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};
#[cfg(feature = "test-fixtures")]
#[derive(Clone, Copy, Default)]
pub struct FixtureFaults {
    pub read: bool,
    pub post_read_check: bool,
    pub close: bool,
}
#[derive(Default)]
struct Hooks {
    #[cfg(feature = "test-fixtures")]
    faults: FixtureFaults,
}
impl Hooks {
    fn check(&self, operation: IoOperation) -> Result<(), Error> {
        #[cfg(feature = "test-fixtures")]
        if match operation {
            IoOperation::Read => self.faults.read,
            IoOperation::Stat => self.faults.post_read_check,
            IoOperation::Close => self.faults.close,
            _ => false,
        } {
            return Err(Error::Io {
                operation,
                cause: std::io::Error::from_raw_os_error(5),
            });
        }
        let _ = operation;
        Ok(())
    }
}
fn io(operation: IoOperation, cause: impl Into<std::io::Error>) -> Error {
    Error::Io {
        operation,
        cause: cause.into(),
    }
}
fn combined<T>(result: Result<T, Error>, cleanup: Result<(), Error>) -> Result<T, Error> {
    match (result, cleanup) {
        (r, Ok(())) => r,
        (Ok(_), Err(e)) => Err(e),
        (Err(cause), Err(cleanup)) => Err(Error::Joined {
            cause: Box::new(cause),
            cleanup: Box::new(cleanup),
        }),
    }
}
fn safe_file(file: impl AsFd, uid: u32) -> Result<(), Error> {
    let stat = unix::fstat(file).map_err(|e| io(IoOperation::Stat, e))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_mode & 0o7777 != 0o600
        || stat.st_uid != uid
        || stat.st_nlink != 1
    {
        return Err(Error::Path);
    }
    Ok(())
}
/// Read only; the digest must come from independent protected installation
/// control. This neither opens a Coordinator nor provisions a human/controller.
pub fn load(path: &str, expected_digest: &str) -> Result<Document, Error> {
    load_inner(
        path,
        expected_digest,
        Hooks::default(),
        decode,
        |d| d,
        installation_roots,
    )
}
/// Fixture-only v1 read/post-read/close failures on the actual protected path.
/// Requires isolated private fixture roots and independent fixture pin. No data,
/// lock, account or permission is created; every acquired descriptor still closes
/// and primary/secondary IO causes remain visible. Absent from normal profile.
#[cfg(feature = "test-fixtures")]
pub fn load_with_faults(
    path: &str,
    expected_digest: &str,
    faults: FixtureFaults,
) -> Result<Document, Error> {
    load_inner(
        path,
        expected_digest,
        Hooks { faults },
        decode,
        |d| d,
        installation_roots,
    )
}
/// Read the independently pinned v2 declaration and check protected existing
/// journal/configuration/witness roots, without opening SQLite or creating locks.
/// The checkpoint and human labels remain unverified data until current-source
/// control. Metadata checks do not claim OS confinement or hostile-UID isolation.
pub fn load_control(path: &str, expected_digest: &str) -> Result<ControlDocument, Error> {
    load_inner(
        path,
        expected_digest,
        Hooks::default(),
        decode_control,
        |d| &d.installation,
        control_roots,
    )
}
/// Fixture-only read/post-read/close failures, with actual descriptors still
/// explicitly closed. Does not inject a signature, grant or production policy.
#[cfg(feature = "test-fixtures")]
pub fn load_control_with_faults(
    path: &str,
    expected_digest: &str,
    faults: FixtureFaults,
) -> Result<ControlDocument, Error> {
    load_inner(
        path,
        expected_digest,
        Hooks { faults },
        decode_control,
        |d| &d.installation,
        control_roots,
    )
}
/// Read/hash/decode one pinned declaration while owning all observed handles.
/// Trusted typed layouts select v1/v2/v3 roots/files; no callable data comes from JSON.
/// Layout order preserves existing v1/v2 observation/cleanup order. No lock or
/// SQLite open occurs, and content/permission errors never create an empty success.
/// Close every handle after success/refusal and retain every cleanup failure.
fn load_inner<T>(
    path: &str,
    expected_digest: &str,
    hooks: Hooks,
    decode_document: fn(&[u8]) -> Result<T, Error>,
    installation: fn(&T) -> &Document,
    roots: fn(&T) -> Result<Vec<StorageRoot<'_>>, Error>,
) -> Result<T, Error> {
    if !swarm_identity_contract::digest(expected_digest.as_bytes()) {
        return Err(Error::Pin);
    }
    if !clean_path(path) {
        return Err(Error::Path);
    }
    let uid = rustix::process::geteuid().as_raw();
    let path = Path::new(path);
    let parent = path.parent().ok_or(Error::Path)?;
    let parent_fd = swarm_enrollment_config::store::open_private_directory(parent, uid)
        .map_err(Error::Protected)?;
    let mut files = vec![parent_fd];
    let result = (|| {
        let fd = unix::openat(
            &files[0],
            path.file_name().ok_or(Error::Path)?,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|e| io(IoOperation::Open, e))?;
        files.push(File::from(fd));
        safe_file(&files[1], uid)?;
        let mut raw = Vec::new();
        hooks.check(IoOperation::Read)?;
        files[1]
            .read_to_end(&mut raw)
            .map_err(|e| io(IoOperation::Read, e))?;
        hooks.check(IoOperation::Stat)?;
        safe_file(&files[1], uid)?;
        if format!("{:x}", Sha256::digest(&raw)) != expected_digest {
            return Err(Error::Pin);
        }
        let document = decode_document(&raw)?;
        let d = installation(&document);
        let layout = roots(&document)?;
        let bootstrap_parent = parent.to_str().ok_or(Error::Path)?;
        if d.service_uid != uid
            || layout
                .iter()
                .any(|root| overlaps(bootstrap_parent, root.directory))
        {
            return Err(Error::Path);
        }
        let mut directories = vec![0];
        let mut journals = vec![];
        for root in layout {
            let handle = swarm_enrollment_config::store::open_private_directory(
                Path::new(root.directory),
                uid,
            )
            .map_err(Error::Protected)?;
            files.push(handle);
            let index = files.len() - 1;
            distinct_root(&files, &directories, index)?;
            directories.push(index);
            if let Some(journal) = root.journal {
                journals.push((journal, index));
            }
        }
        for (journal, parent_index) in journals {
            let path = Path::new(journal);
            let fd = unix::openat(
                &files[parent_index],
                path.file_name().ok_or(Error::Path)?,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
                Mode::empty(),
            )
            .map_err(|e| io(IoOperation::Open, e))?;
            files.push(File::from(fd));
            safe_file(files.last().ok_or(Error::Path)?, uid)?;
        }
        Ok(document)
    })();
    // Run every close even after the original refusal or an earlier close failure.
    // File ownership is consumed; never retry close on a possibly reused descriptor.
    let mut closed = Ok(());
    for file in files {
        let actual = nix::unistd::close(file).map_err(|e| {
            io(
                IoOperation::Close,
                std::io::Error::from_raw_os_error(e as i32),
            )
        });
        closed = combined(closed, combined(actual, hooks.check(IoOperation::Close)));
    }
    combined(result, closed)
}

/// Borrowed names only, selected by trusted closed-schema codecs. Journal roots
/// are checked as directories and their final files separately, with no SQLite.
struct StorageRoot<'a> {
    directory: &'a str,
    journal: Option<&'a str>,
}
/// Existing v1 storage layout, no extra root or file inferred from input strings.
fn installation_roots(d: &Document) -> Result<Vec<StorageRoot<'_>>, Error> {
    Ok(vec![
        StorageRoot {
            directory: &d.configuration_directory,
            journal: None,
        },
        StorageRoot {
            directory: &d.witness_directory,
            journal: None,
        },
    ])
}
/// Existing v2 layout preserves config/witness then journal-parent order.
fn control_roots(d: &ControlDocument) -> Result<Vec<StorageRoot<'_>>, Error> {
    let mut roots = installation_roots(&d.installation)?;
    roots.push(StorageRoot {
        directory: d.journal_directory()?,
        journal: Some(&d.journal_path),
    });
    Ok(roots)
}
/// V3 adds explicit personal checkpoint and journal; no head/key/identity learned
/// from their contents. Complete logical validation already ran in the codec.
fn authenticated_roots(d: &AuthenticatedDocument) -> Result<Vec<StorageRoot<'_>>, Error> {
    let mut roots = control_roots(&d.control)?;
    roots.push(StorageRoot {
        directory: &d.personal.checkpoint_directory,
        journal: None,
    });
    roots.push(StorageRoot {
        directory: d.personal.journal_directory()?,
        journal: Some(&d.personal.journal_path),
    });
    Ok(roots)
}
/// Compare real retained descriptors, including bootstrap parent, to reject one
/// physical directory serving multiple domains. Owned FDs prevent inode reuse
/// during comparison. Invalid internal indices refuse; fstat failures keep cause.
fn distinct_root(files: &[File], previous: &[usize], current: usize) -> Result<(), Error> {
    let new = unix::fstat(files.get(current).ok_or(Error::Path)?)
        .map_err(|e| io(IoOperation::Stat, e))?;
    for index in previous {
        let old = unix::fstat(files.get(*index).ok_or(Error::Path)?)
            .map_err(|e| io(IoOperation::Stat, e))?;
        if new.st_dev == old.st_dev && new.st_ino == old.st_ino {
            return Err(Error::Path);
        }
    }
    Ok(())
}
/// Read the independently pinned v3 declaration and inspect all private existing
/// storage roots and both journal files. No SQLite/lock/account initialization or
/// authority adoption. Exact owner/mode/type/link checks and all close causes are
/// retained. This is metadata observation, not authentication or OS confinement.
pub fn load_authenticated(
    path: &str,
    expected_digest: &str,
) -> Result<AuthenticatedDocument, Error> {
    load_inner(
        path,
        expected_digest,
        Hooks::default(),
        decode_authenticated,
        |d| &d.control.installation,
        authenticated_roots,
    )
}
/// Same actual descriptor/cleanup path with the existing isolated read/stat/close
/// fixture hooks, absent in normal profile. No alias or authorization is injected.
#[cfg(feature = "test-fixtures")]
pub fn load_authenticated_with_faults(
    path: &str,
    expected_digest: &str,
    faults: FixtureFaults,
) -> Result<AuthenticatedDocument, Error> {
    load_inner(
        path,
        expected_digest,
        Hooks { faults },
        decode_authenticated,
        |d| &d.control.installation,
        authenticated_roots,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    /// Actual retained FD alias, not mocked metadata or privileged mount. A valid
    /// pair of distinct private directories precedes rejection of duplicate inode.
    #[test]
    fn distinct_root_real_owned_descriptor_alias_refuses_after_distinct_control() {
        let one = tempfile::tempdir().unwrap();
        let two = tempfile::tempdir().unwrap();
        let uid = rustix::process::geteuid().as_raw();
        for root in [one.path(), two.path()] {
            std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let a = swarm_enrollment_config::store::open_private_directory(one.path(), uid).unwrap();
        let b = swarm_enrollment_config::store::open_private_directory(two.path(), uid).unwrap();
        let alias =
            swarm_enrollment_config::store::open_private_directory(one.path(), uid).unwrap();
        let files = vec![a, b, alias];
        assert!(distinct_root(&files, &[0], 1).is_ok());
        assert_eq!(
            distinct_root(&files, &[0, 1], 2).unwrap_err().code(),
            "path"
        );
        for file in files {
            nix::unistd::close(file).unwrap()
        }
    }
}
