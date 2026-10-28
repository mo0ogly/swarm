//! Private bytes from an already reserved directory, without schema/authority
//! inference. No secret-bearing allocation growth; caller/OS/compiler copies and
//! hostile same-UID/root mutation are outside the protection/erasure claim.
use super::*;
use sha2::{Digest, Sha256};
use std::io::{self, Read};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Observed read validation, never caller/human/business authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadValidation {
    NotAttempted,
    ReadUnknown,
    BytesRead,
    PinMatched,
}
/// Local filesystem acknowledgements only, not hardware/power-loss guarantees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadDurability {
    NotAcknowledged,
    FileAcknowledged,
    DirectoryAcknowledged,
}
/// Failure retains actual validation and fsync progress, even after close/cancel.
/// No private byte buffer is returned after any refusal.
pub struct DurableReadError {
    pub validation: ReadValidation,
    pub durability: ReadDurability,
    pub cause: Error,
}
impl fmt::Display for DurableReadError {
    /// Safe observed stages/category only, no pin/path/data/private IO text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "private durable read: {:?} {:?} ({})",
            self.validation,
            self.durability,
            self.cause.code()
        )
    }
}
impl fmt::Debug for DurableReadError {
    /// Same redaction as Display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for DurableReadError {
    /// Actual cause including any Joined independent file-close branch.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}
/// Internal explicit acknowledgement request, not an operator execution policy.
struct DurabilityRequest<'a> {
    // Explicit pin comparison or trusted-locator read; no implicit fallback.
    expected: Option<&'a str>,
    validation: &'a mut ReadValidation,
    durability: &'a mut ReadDurability,
}

/// Owned private bytes with redacted Debug and no Clone/Serialize. Only this
/// allocation zeroizes on drop; borrowed copies remain their owner's duty.
pub struct PrivateBytes(Zeroizing<Vec<u8>>);
impl PrivateBytes {
    /// Borrow exact file bytes, not an authenticity/pin/freshness/human proof.
    /// Callers must validate protected format/context before any use.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for PrivateBytes {
    /// Neither bytes nor size/path/identity is included in ordinary diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrivateBytes { <private> }")
    }
}
/// Bind the retained private regular0600single-link descriptor to its current
/// name. No symlink follow or directory pathname reopen; not OS confinement.
fn checked_file(dir: &File, file: &File, owner: u32, name: &str) -> Result<unix::Stat, Error> {
    safe_file(file, owner)?;
    let held = unix::fstat(file).map_err(|e| Error::io(IoClass::Path, e))?;
    let named = unix::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|e| Error::io(IoClass::Path, e))?;
    if held.st_dev != named.st_dev || held.st_ino != named.st_ino {
        return Err(Error::Path);
    }
    Ok(held)
}
/// Reserve actual stat-size plus one sentinel before reading any private byte.
/// Reject shrink/growth rather than truncate or reallocate. File-size/usize and
/// allocation failures are representation/resource errors, no operator cap.
/// Only EINTR byte reads continue; cancellation checks never close borrowed IO.
fn bytes(
    reader: &mut dyn Read,
    size: usize,
    check: &impl Fn() -> Result<(), Error>,
) -> Result<PrivateBytes, Error> {
    check()?;
    let capacity = size.checked_add(1).ok_or(Error::Path)?;
    let mut raw = Zeroizing::new(Vec::new());
    raw.try_reserve_exact(capacity)
        .map_err(|e| Error::io(IoClass::Storage, io::Error::other(e)))?;
    raw.resize(capacity, 0);
    let mut used = 0;
    loop {
        check()?;
        let remaining = raw.len() - used;
        let count = match reader.read(&mut raw[used..]) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(Error::io(IoClass::Storage, e)),
        };
        if count > remaining {
            return Err(Error::io(
                IoClass::Storage,
                io::Error::new(io::ErrorKind::InvalidData, "invalid reader count"),
            ));
        }
        check()?;
        if count == 0 {
            break;
        }
        used += count;
        if used > size {
            return Err(Error::Conflict);
        }
    }
    if used != size {
        return Err(Error::Conflict);
    }
    // The unused sentinel is still its initial zero on every successful path.
    raw.truncate(size);
    Ok(PrivateBytes(raw))
}
/// Existing reserved-directory entry point. Missing remains NotConfigured;
/// all opened files explicitly close, with independent close/hook errors joined.
pub(super) fn read(
    dir: &File,
    owner: u32,
    name: &str,
    check: &impl Fn() -> Result<(), Error>,
    fault: &impl Fn(Stage, IoClass) -> Result<(), Error>,
) -> Result<PrivateBytes, Error> {
    read_inner(dir, owner, name, check, fault, &|| Ok(()), &|| Ok(()))
}
/// Before/after callbacks are internal synchronous test seams only. Production
/// passes no-op callbacks, no environment/feature/runtime policy is introduced.
/// Revalidate physical binding/metadata after EOF before explicit close. Atime
/// may change; read errors never establish no filesystem observation side effect.
fn read_inner(
    dir: &File,
    owner: u32,
    name: &str,
    check: &impl Fn() -> Result<(), Error>,
    fault: &impl Fn(Stage, IoClass) -> Result<(), Error>,
    before_read: &impl Fn() -> Result<(), Error>,
    after_read: &impl Fn() -> Result<(), Error>,
) -> Result<PrivateBytes, Error> {
    read_core(
        dir,
        owner,
        name,
        check,
        fault,
        before_read,
        after_read,
        None,
    )
}
/// Verify expected digest before any fsync, then acknowledge held file and
/// directory with the file FD retained through final identity/metadata checks.
/// Caller validates canonical pin/name before entry. No implicit retry/duration.
pub(super) fn read_durable(
    dir: &File,
    owner: u32,
    name: &str,
    expected: &str,
    check: &impl Fn() -> Result<(), Error>,
    fault: &impl Fn(Stage, IoClass) -> Result<(), Error>,
) -> Result<PrivateBytes, DurableReadError> {
    read_durable_kind(dir, owner, name, Some(expected), check, fault)
}
/// Explicit trusted-locator acknowledgement, with no digest comparison or
/// PinMatched claim. Caller owns locator trust and later format/context checks.
/// Identical protected FD/read/fsync/close checks and error progress still apply.
pub(super) fn read_trusted_durable(
    dir: &File,
    owner: u32,
    name: &str,
    check: &impl Fn() -> Result<(), Error>,
    fault: &impl Fn(Stage, IoClass) -> Result<(), Error>,
) -> Result<PrivateBytes, DurableReadError> {
    read_durable_kind(dir, owner, name, None, check, fault)
}
/// Mechanical explicit durability implementation, independent of operator policy.
/// Some requires an independently supplied canonical digest (validated by the
/// public boundary); None is only the separately named trusted-locator entry.
/// An absent comparison never upgrades BytesRead to PinMatched or grants trust.
fn read_durable_kind(
    dir: &File,
    owner: u32,
    name: &str,
    expected: Option<&str>,
    check: &impl Fn() -> Result<(), Error>,
    fault: &impl Fn(Stage, IoClass) -> Result<(), Error>,
) -> Result<PrivateBytes, DurableReadError> {
    let mut validation = ReadValidation::NotAttempted;
    let mut durability = ReadDurability::NotAcknowledged;
    let result = read_core(
        dir,
        owner,
        name,
        check,
        fault,
        &|| Ok(()),
        &|| Ok(()),
        Some(DurabilityRequest {
            expected,
            validation: &mut validation,
            durability: &mut durability,
        }),
    );
    result.map_err(|cause| DurableReadError {
        validation,
        durability,
        cause,
    })
}
/// Require stable size/mtime/ctime. Atime is intentionally not a mutation guard.
fn unchanged_metadata(before: &unix::Stat, after: &unix::Stat) -> Result<(), Error> {
    if before.st_size != after.st_size
        || before.st_mtime != after.st_mtime
        || before.st_mtime_nsec != after.st_mtime_nsec
        || before.st_ctime != after.st_ctime
        || before.st_ctime_nsec != after.st_ctime_nsec
    {
        Err(Error::Conflict)
    } else {
        Ok(())
    }
}
/// Shared exact read with optional explicit durability and separate pin choice.
/// Trusted-locator mode preserves BytesRead without a digest comparison. Observe mode
/// performs no fsync; sync mode keeps acknowledged progress through every error.
/// Each owned leaf FD is consumed once, including after any earlier refusal.
fn read_core(
    dir: &File,
    owner: u32,
    name: &str,
    check: &impl Fn() -> Result<(), Error>,
    fault: &impl Fn(Stage, IoClass) -> Result<(), Error>,
    before_read: &impl Fn() -> Result<(), Error>,
    after_read: &impl Fn() -> Result<(), Error>,
    mut durable: Option<DurabilityRequest<'_>>,
) -> Result<PrivateBytes, Error> {
    if let Some(d) = durable.as_mut() {
        *d.validation = ReadValidation::ReadUnknown;
    }
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
        let before = checked_file(dir, &file, owner, name)?;
        let size = usize::try_from(before.st_size).map_err(|_| Error::Path)?;
        check()?;
        before_read()?;
        let raw = bytes(&mut file, size, check)?;
        after_read()?;
        check()?;
        let after = checked_file(dir, &file, owner, name)?;
        unchanged_metadata(&before, &after)?;
        if let Some(d) = durable.as_mut() {
            *d.validation = ReadValidation::BytesRead;
            if let Some(expected) = d.expected {
                let actual = format!("{:x}", Sha256::digest(raw.as_bytes()));
                if !bool::from(actual.as_bytes().ct_eq(expected.as_bytes())) {
                    return Err(Error::Pins);
                }
                *d.validation = ReadValidation::PinMatched;
            }
            check()?;
            fault(Stage::FileSync, IoClass::Storage)?;
            file.sync_all()
                .map_err(|e| Error::io(IoClass::Storage, e))?;
            *d.durability = ReadDurability::FileAcknowledged;
            check()?;
            file_io::sync_directory(dir, fault)?;
            *d.durability = ReadDurability::DirectoryAcknowledged;
            check()?;
            let after_sync = checked_file(dir, &file, owner, name)?;
            unchanged_metadata(&before, &after_sync)?;
        }
        Ok(raw)
    })();
    combined(
        result,
        combined(
            close(file, IoClass::Storage),
            fault(Stage::ReadClose, IoClass::Storage),
        ),
    )
}

#[cfg(test)]
mod tests;
