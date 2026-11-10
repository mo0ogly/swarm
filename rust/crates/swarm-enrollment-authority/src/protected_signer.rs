//! Private startup-only Ed25519 seed loading. This is key custody, not current
//! human/account authority, a service, OS confinement or a signing endpoint.
use ed25519_dalek::{SigningKey, VerifyingKey};
use rustix::fs::{self as unix, FileType, Mode, OFlags};
use std::{
    error::Error as StdError,
    ffi::OsStr,
    fmt,
    fs::File,
    io::Read,
    path::{Component, Path},
};
use zeroize::Zeroizing;
/// Actual IO phase; no path, key or secret is formatted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Open,
    Stat,
    Read,
    FileClose,
    DirectoryClose,
}
/// Safe refusal with typed causes and all independent close failures retained.
/// source() selects the primary; Joined.cleanup must also be examined.
pub enum Error {
    Path,
    Encoding,
    Binding,
    Storage(swarm_enrollment_config::Error),
    Io {
        stage: Stage,
        cause: std::io::Error,
    },
    Joined {
        cause: Box<Error>,
        cleanup: Box<Error>,
    },
}
impl Error {
    /// Stable safe category. No secret/path/OS error text is included.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Encoding => "encoding",
            Self::Binding => "binding",
            Self::Storage(e) => e.code(),
            Self::Io { stage, .. } => match stage {
                Stage::Open => "open",
                Stage::Stat => "stat",
                Stage::Read => "read",
                Stage::FileClose | Stage::DirectoryClose => "close",
            },
            Self::Joined { cause, .. } => cause.code(),
        }
    }
}
impl fmt::Display for Error {
    /// Keep even nested IO diagnostics private; typed causes remain inspectable.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "protected signing key: {}", self.code())
    }
}
impl fmt::Debug for Error {
    /// Ordinary debug is as redacted as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl StdError for Error {
    /// Primary cause only. Joined keeps its independent cleanup as structured data.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Storage(e) => Some(e),
            Self::Io { cause, .. } => Some(cause),
            Self::Joined { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}
/// Owned, non-clonable/non-serializable key snapshot. No seed export or public
/// arbitrary-signing method. Drop uses dalek's explicitly enabled zeroize feature;
/// compiler/OS copies, swap/disk and hostile same-UID access are not confined.
pub struct ProtectedSigner {
    key: SigningKey,
}
impl ProtectedSigner {
    /// Internal trusted lifecycle borrow only. No public signing-key/seed export or
    /// arbitrary-signing endpoint is created; caller still verifies signed context.
    pub(crate) fn signing_key(&self) -> &SigningKey {
        &self.key
    }
    /// Public identity of the loaded snapshot, not proof of current authorization.
    /// Later file changes do not rotate an already loaded signer.
    pub fn public_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }
}
impl fmt::Debug for ProtectedSigner {
    /// Never print key material, including public installation identity.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProtectedSigner { <private key> }")
    }
}
/// Preserve primary and cleanup without ever converting a failed close to success.
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
/// Retain exact OS cause without embedding private paths or bytes in diagnostics.
fn io(stage: Stage, cause: impl Into<std::io::Error>) -> Error {
    Error::Io {
        stage,
        cause: cause.into(),
    }
}
/// Check actual open regular0600single-link/current-euid inode and current name.
/// Directory is separately validated before and after read. No pathname adoption.
fn checked_file(dir: &File, file: &File, name: &OsStr, uid: u32) -> Result<unix::Stat, Error> {
    let s = unix::fstat(file).map_err(|e| io(Stage::Stat, e))?;
    let named =
        unix::statat(dir, name, unix::AtFlags::SYMLINK_NOFOLLOW).map_err(|e| io(Stage::Stat, e))?;
    if FileType::from_raw_mode(s.st_mode) != FileType::RegularFile
        || s.st_mode & 0o7777 != 0o600
        || s.st_uid != uid
        || s.st_nlink != 1
        || s.st_dev != named.st_dev
        || s.st_ino != named.st_ino
    {
        return Err(Error::Path);
    }
    // 32bytes is the Ed25519 seed format, not an operator limit.
    if s.st_size != 32 {
        return Err(Error::Encoding);
    }
    Ok(s)
}
/// Recheck the retained directory's actual owner/type/mode after reading. The
/// last pathname's ancestors and hostile same-UID processes are not confined.
fn checked_directory(dir: &File, uid: u32) -> Result<(), Error> {
    let s = unix::fstat(dir).map_err(|e| io(Stage::Stat, e))?;
    if FileType::from_raw_mode(s.st_mode) != FileType::Directory
        || s.st_mode & 0o7777 != 0o700
        || s.st_uid != uid
    {
        return Err(Error::Path);
    }
    Ok(())
}
/// Unit-only phase failures/substitution. Normal build is zero-sized, no public
/// fixture surface or production fault activation/environment option.
#[derive(Default)]
struct Hooks {
    #[cfg(test)]
    fail: u32,
    #[cfg(test)]
    after_read: Option<std::sync::Arc<dyn Fn()>>,
    #[cfg(test)]
    before_read: Option<std::sync::Arc<dyn Fn()>>,
}
impl Hooks {
    /// Fixture EIO after actual closes; never skip/duplicate close on a reused FD.
    fn check(&self, stage: Stage) -> Result<(), Error> {
        #[cfg(test)]
        {
            if stage == Stage::Read {
                if let Some(f) = &self.before_read {
                    f()
                }
            }
            if stage == Stage::Stat {
                if let Some(f) = &self.after_read {
                    f()
                }
            }
            let bit = match stage {
                Stage::Read => 1,
                Stage::Stat => 2,
                Stage::FileClose => 4,
                Stage::DirectoryClose => 8,
                _ => 0,
            };
            if self.fail & bit != 0 {
                return Err(io(stage, std::io::Error::from_raw_os_error(5)));
            }
        }
        let _ = stage;
        Ok(())
    }
}
/// Consume each owned descriptor exactly once, preserving actual and independent
/// post-close fixture failures. Never retry EINTR or a possibly reused FD.
fn close(file: File, stage: Stage, hooks: &Hooks) -> Result<(), Error> {
    let actual = nix::unistd::close(file)
        .map_err(|e| io(stage, std::io::Error::from_raw_os_error(e as i32)));
    combined(actual, hooks.check(stage))
}
/// Load exactly32raw seed bytes from an existing absolute private startup path,
/// with independently pinned public key. Opens no/create/write/SQLite/lock files.
/// Shared private-directory loader walks all existing ancestors without symlinks,
/// requires root/service ownership and no group/other writes except root-sticky
/// ancestors. Final parent0700euid and regular0600single-link file are checked on
/// owned descriptors; parent and current named file rechecked after read. All
/// descriptors close before success, all close failures remain structured.
/// Owned read buffers and dalek key drop zeroize. No real key generation, human
/// presence, current authorization, same-UID/OS confinement or durable key
/// rotation is established. Caller must protect ancestor resolution and supply
/// the path/pin outside requests; ancestor checks are not OS confinement. This is
/// a snapshot until dropped/reloaded.
pub fn load(path: &Path, expected_public_key: [u8; 32]) -> Result<ProtectedSigner, Error> {
    load_inner(path, expected_public_key, &Hooks::default())
}
/// Shared actual read/close path, with unit-only faults. Validation precedes key
/// construction; closing failure discards/zeroizes any successful key snapshot.
fn load_inner(path: &Path, expected: [u8; 32], hooks: &Hooks) -> Result<ProtectedSigner, Error> {
    VerifyingKey::from_bytes(&expected).map_err(|_| Error::Binding)?;
    if !path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
        || path
            .as_os_str()
            .as_encoded_bytes()
            .split(|b| *b == b'/')
            .any(|p| p == b".")
    {
        return Err(Error::Path);
    }
    let parent = path.parent().ok_or(Error::Path)?;
    let name = path.file_name().ok_or(Error::Path)?;
    let uid = rustix::process::geteuid().as_raw();
    let dir = swarm_enrollment_config::store::open_private_directory(parent, uid)
        .map_err(Error::Storage)?;
    let mut files = vec![];
    let result = (|| {
        let fd = unix::openat(
            &dir,
            name,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|e| io(Stage::Open, e))?;
        files.push(File::from(fd));
        let file = &mut files[0];
        let before = checked_file(&dir, file, name, uid)?;
        let mut seed = Zeroizing::new([0u8; 32]);
        let mut extra = Zeroizing::new([0u8; 1]);
        hooks.check(Stage::Read)?;
        file.read_exact(seed.as_mut())
            .map_err(|e| io(Stage::Read, e))?;
        if file.read(extra.as_mut()).map_err(|e| io(Stage::Read, e))? != 0 {
            return Err(Error::Encoding);
        }
        hooks.check(Stage::Stat)?;
        checked_directory(&dir, uid)?;
        let after = checked_file(&dir, file, name, uid)?;
        if before.st_mtime != after.st_mtime
            || before.st_mtime_nsec != after.st_mtime_nsec
            || before.st_ctime != after.st_ctime
            || before.st_ctime_nsec != after.st_ctime_nsec
        {
            return Err(Error::Path);
        }
        let key = SigningKey::from_bytes(&seed);
        if key.verifying_key().to_bytes() != expected {
            return Err(Error::Binding);
        }
        Ok(ProtectedSigner { key })
    })();
    let mut closed = Ok(());
    for f in files {
        closed = combined(closed, close(f, Stage::FileClose, hooks));
    }
    closed = combined(closed, close(dir, Stage::DirectoryClose, hooks));
    combined(result, closed)
}
#[cfg(test)]
mod tests;
