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
    load_inner(path, expected_digest, Hooks::default())
}
#[cfg(feature = "test-fixtures")]
pub fn load_with_faults(
    path: &str,
    expected_digest: &str,
    faults: FixtureFaults,
) -> Result<Document, Error> {
    load_inner(path, expected_digest, Hooks { faults })
}
fn load_inner(path: &str, expected_digest: &str, hooks: Hooks) -> Result<Document, Error> {
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
        let d = decode(&raw)?;
        if d.service_uid != uid
            || overlaps(
                parent.to_str().ok_or(Error::Path)?,
                &d.configuration_directory,
            )
            || overlaps(parent.to_str().ok_or(Error::Path)?, &d.witness_directory)
        {
            return Err(Error::Path);
        }
        let mut directories = vec![0];
        for directory in [&d.configuration_directory, &d.witness_directory] {
            let handle =
                swarm_enrollment_config::store::open_private_directory(Path::new(directory), uid)
                    .map_err(Error::Protected)?;
            files.push(handle);
            let index = files.len() - 1;
            let stat = unix::fstat(&files[index]).map_err(|e| io(IoOperation::Stat, e))?;
            for previous in &directories {
                let old = unix::fstat(&files[*previous]).map_err(|e| io(IoOperation::Stat, e))?;
                if stat.st_dev == old.st_dev && stat.st_ino == old.st_ino {
                    return Err(Error::Path);
                }
            }
            directories.push(index);
        }
        Ok(d)
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
