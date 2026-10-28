//! Private exact bytes for a trusted coordinator, not a configuration/human
//! authority API. Use a dedicated existing private directory. No OS confinement,
//! automatic directory creation, checkpoint adoption or secret logging.
pub use super::file_io::Publication;
pub use super::guarded_read::{DurableReadError, PrivateBytes, ReadDurability, ReadValidation};
use super::*;
const LOCK: &str = "private-documents.lock";
/// A failed write with its observed publication phase and all IO/cleanup causes.
/// An error after publication never relabels the effect as absent. Safe ordinary
/// diagnostics omit filenames, document bytes and underlying private IO strings.
pub struct WriteError {
    /// Last actual rename/fsync acknowledgement, including uncertain rename.
    pub publication: Publication,
    /// Primary and any Joined secondary cleanup causes; authorized traversal only.
    pub cause: Error,
}
impl fmt::Display for WriteError {
    /// Fixed category and phase only; preserve structured cause via Error::source.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "private document: {:?} ({})",
            self.publication,
            self.cause.code()
        )
    }
}
impl fmt::Debug for WriteError {
    /// Same safe representation as Display, with no sensitive document data.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for WriteError {
    /// Preserve actual primary/secondary cause tree without formatting it.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}
struct DocumentInner {
    handles: Mutex<Option<Handles>>,
    owner: u32,
    pending_cleanup: Mutex<Option<Error>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    faults: std::sync::atomic::AtomicU32,
}
/// Retains directory and lock object descriptors; clones share closed/poison
/// state. Independent opens coordinate with immediate nonblocking flock. The
/// caller supplies a dedicated existing0700directory and actual effective UID.
#[derive(Clone)]
pub struct Documents {
    inner: Arc<DocumentInner>,
}
impl fmt::Debug for Documents {
    /// No private path, bytes, pins or identities are formatted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivateDocuments").finish_non_exhaustive()
    }
}
/// Validate internal protocol names, never paths: ASCII alphanumerics first,
/// then alphanumerics/dot/underscore/hyphen. Lock and hidden temp names excluded.
/// OS filename representation limits remain typed IO errors, no operator cap.
fn name(value: &str) -> Result<(), Error> {
    let raw = value.as_bytes();
    if raw.is_empty()
        || !raw[0].is_ascii_alphanumeric()
        || value == LOCK
        || !raw
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(b))
    {
        return Err(Error::Path);
    }
    Ok(())
}

impl Documents {
    /// Open an existing protected directory without learning any identity/root.
    /// Create/open only its private lockfile; reject foreign UID, links, modes,
    /// types or hardlinks. Close all acquired handles on a failed prerequisite,
    /// retaining secondary causes. Does not create/adopt a document or a journal.
    pub fn open(path: &Path, owner: u32) -> Result<Self, Error> {
        let dir = open_private_directory(path, owner)?;
        let lock = unix::openat(
            &dir,
            LOCK,
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::from_raw_mode(0o600),
        )
        .map(File::from)
        .map_err(|e| Error::io(IoClass::Path, e));
        let lock = match lock {
            Ok(lock) => lock,
            Err(e) => return combined(Err(e), close(dir, IoClass::Path)),
        };
        let h = Handles { dir, lock };
        if let Err(e) = file_io::check_handles(&h, owner, LOCK) {
            return combined(
                combined(Err(e), close(h.lock, IoClass::Path)),
                close(h.dir, IoClass::Path),
            );
        }
        Ok(Self {
            inner: Arc::new(DocumentInner {
                handles: Mutex::new(Some(h)),
                owner,
                pending_cleanup: Mutex::new(None),
                #[cfg(any(test, feature = "test-fixtures"))]
                faults: std::sync::atomic::AtomicU32::new(0),
            }),
        })
    }
    /// Acquire immediate in-process and interprocess exclusion, never queue or
    /// retry. Caller must finish before obtaining personal SQLite authority and
    /// before external business effects. Canceled/closed/poisoned/unsafe/busy
    /// state refuses. Dropped release failures forbid reuse until explicit close.
    pub fn reserve<'a>(&'a self, ctx: &'a Context) -> Result<Reservation<'a>, Error> {
        ctx.check()?;
        let guard = self.inner.handles.try_lock().map_err(|e| match e {
            TryLockError::WouldBlock => Error::Busy,
            TryLockError::Poisoned(_) => Error::State,
        })?;
        if self
            .inner
            .pending_cleanup
            .lock()
            .map_err(|_| Error::State)?
            .is_some()
        {
            return Err(Error::State);
        }
        let h = guard.as_ref().ok_or(Error::Closed)?;
        file_io::check_handles(h, self.inner.owner, LOCK)?;
        unix::flock(&h.lock, FlockOperation::NonBlockingLockExclusive).map_err(|e| {
            if e == Errno::WOULDBLOCK {
                Error::Busy
            } else {
                Error::io(IoClass::Storage, e)
            }
        })?;
        let r = Reservation {
            documents: self,
            ctx,
            guard,
            locked: true,
        };
        if let Err(e) = r.check() {
            return combined(Err(e), r.finish());
        }
        Ok(r)
    }
    /// Close this shared instance explicitly. Busy refuses without waiting or
    /// closing another reservation. Poison recovery only closes descriptors and
    /// reports retained release/close causes; it never enables data operations.
    /// Idempotent once closed. Drop alone is not a successful close proof.
    pub fn close(&self) -> Result<(), Error> {
        let mut guard = match self.inner.handles.try_lock() {
            Ok(g) => g,
            Err(TryLockError::WouldBlock) => return Err(Error::Busy),
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
        };
        let pending = self
            .inner
            .pending_cleanup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let result = if let Some(h) = guard.take() {
            combined(
                combined(
                    close(h.lock, IoClass::Storage),
                    close(h.dir, IoClass::Storage),
                ),
                self.fault(Stage::StoreClose, IoClass::Storage),
            )
        } else {
            Ok(())
        };
        self.inner.handles.clear_poison();
        combined(result, pending.map_or(Ok(()), Err))
    }
    /// Retain a dropped guard's release failure for explicit close; no log-only
    /// success or automatic reset. Further reservations refuse state.
    fn retain_cleanup(&self, e: Error) {
        let mut p = self
            .inner
            .pending_cleanup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *p = Some(match p.take() {
            None => e,
            Some(cause) => Error::Joined {
                cause: Box::new(cause),
                cleanup: Box::new(e),
            },
        })
    }
    /// Isolated IO hooks only, absent from the normal profile. Existing Stage
    /// and errno contracts match Store; no runtime retry/deadline policy.
    fn fault(&self, stage: Stage, class: IoClass) -> Result<(), Error> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.inner.faults.load(Ordering::SeqCst) & stage as u32 != 0 {
            return Err(Error::io(
                class,
                std::io::Error::from_raw_os_error(
                    if matches!(stage, Stage::Write | Stage::PartialWrite) {
                        28
                    } else {
                        5
                    },
                ),
            ));
        }
        let _ = (stage, class);
        Ok(())
    }
    /// Set only this disposable instance's fault mask. Test-fixtures feature
    /// only; Write/PartialWrite inject ENOSPC, other stages EIO. No live use.
    #[cfg(feature = "test-fixtures")]
    pub fn fixture_faults(&self, faults: &[FixtureFault]) {
        let mask = faults.iter().fold(0, |a, f| {
            a | match f {
                FixtureFault::Entropy => Stage::Entropy as u32,
                FixtureFault::Write => Stage::Write as u32,
                FixtureFault::PartialWrite => Stage::PartialWrite as u32,
                FixtureFault::FileSync => Stage::FileSync as u32,
                FixtureFault::FileClose => Stage::FileClose as u32,
                FixtureFault::DirectorySync => Stage::DirectorySync as u32,
                FixtureFault::Unlink => Stage::Unlink as u32,
                FixtureFault::Unlock => Stage::Unlock as u32,
                FixtureFault::ReadClose => Stage::ReadClose as u32,
                FixtureFault::StoreClose => Stage::StoreClose as u32,
            }
        });
        self.inner.faults.store(mask, Ordering::SeqCst)
    }
}
/// Borrowed exclusive private-file reservation. No authentication authority is
/// created. Finish explicitly and handle its result separately from any observed
/// write/callback outcome; a release failure cannot erase a published effect.
pub struct Reservation<'a> {
    documents: &'a Documents,
    ctx: &'a Context,
    guard: MutexGuard<'a, Option<Handles>>,
    locked: bool,
}
impl Reservation<'_> {
    /// Retained descriptor state, never reopen a configured directory pathname.
    fn handles(&self) -> Result<&Handles, Error> {
        self.guard.as_ref().ok_or(Error::Closed)
    }
    /// Revalidate cancellation and live directory/lock identity before IO.
    fn check(&self) -> Result<(), Error> {
        self.ctx.check()?;
        file_io::check_handles(self.handles()?, self.documents.inner.owner, LOCK)
    }
    /// Read exact bytes; missing/malformed physical file/close/cancellation errors
    /// refuse. Caller's schema/root validation remains mandatory before use.
    pub fn read(&self, n: &str) -> Result<Vec<u8>, Error> {
        name(n)?;
        self.check()?;
        let raw = file_io::read_document(
            &self.handles()?.dir,
            self.documents.inner.owner,
            n,
            &|s, c| self.documents.fault(s, c),
        )?;
        self.ctx.check()?;
        Ok(raw)
    }
    /// Read private bytes into one guarded allocation before the first byte;
    /// never reallocate secret-bearing storage or expose bytes through Debug.
    /// Retained directory/lock/name/file identity, exact stat-size and metadata
    /// are rechecked; missing, growth/shrink, unsafe files, cancellation and close
    /// errors refuse. No schema/pin/current-authority or OS-confinement claim.
    /// Caller owns borrowed copies; atime/OS/compiler buffers are not erased.
    pub fn read_private(&self, n: &str) -> Result<PrivateBytes, Error> {
        name(n)?;
        self.check()?;
        let raw = super::guarded_read::read(
            &self.handles()?.dir,
            self.documents.inner.owner,
            n,
            &|| self.check(),
            &|s, c| self.documents.fault(s, c),
        )?;
        self.check()?;
        Ok(raw)
    }
    /// Explicitly verify independently expected SHA256 bytes before file fsync,
    /// then directory fsync while the checked file descriptor is still held.
    /// Success includes both local FS acknowledgements and explicit file close;
    /// errors retain validation/ack progress, including late close/cancellation.
    /// No content is replaced, no hardware/freshness/human authority inferred.
    /// The observational read_private remains free of implicit fsync.
    pub fn read_private_durable(
        &self,
        n: &str,
        expected: &str,
    ) -> Result<PrivateBytes, DurableReadError> {
        let initial = |cause| DurableReadError {
            validation: ReadValidation::NotAttempted,
            durability: ReadDurability::NotAcknowledged,
            cause,
        };
        name(n).map_err(initial)?;
        if !swarm_identity_contract::digest(expected.as_bytes()) {
            return Err(initial(Error::Pins));
        }
        self.check().map_err(initial)?;
        let raw = super::guarded_read::read_durable(
            &self.handles().map_err(initial)?.dir,
            self.documents.inner.owner,
            n,
            expected,
            &|| self.check(),
            &|s, c| self.documents.fault(s, c),
        )?;
        self.check().map_err(|cause| DurableReadError {
            validation: ReadValidation::PinMatched,
            durability: ReadDurability::DirectoryAcknowledged,
            cause,
        })?;
        Ok(raw)
    }
    /// Acknowledge bytes from an independently PROTECTED LOCATOR, without an
    /// expected digest. The caller must establish that this directory/name is a
    /// trusted anchor before invoking; this method cannot establish that trust.
    /// Never use it to bless received package data or substitute for the pinned
    /// read. Ownership/0700/0600/singlelink/nofollow/retained identity and exact
    /// EOF checks, file+directory fsync and explicit read close still apply.
    /// Success acknowledges local FS bytes only; errors retain BytesRead (never
    /// PinMatched) and actual ack progress, with original/close causes. It neither
    /// validates anchor schema/context nor proves current authority/rollback/human
    /// or hardware survival. Caller must finish reservation and close Documents
    /// before using decoded anchor data; late cleanup can still refuse it.
    pub fn read_trusted_private_durable(&self, n: &str) -> Result<PrivateBytes, DurableReadError> {
        let initial = |cause| DurableReadError {
            validation: ReadValidation::NotAttempted,
            durability: ReadDurability::NotAcknowledged,
            cause,
        };
        name(n).map_err(initial)?;
        self.check().map_err(initial)?;
        let raw = super::guarded_read::read_trusted_durable(
            &self.handles().map_err(initial)?.dir,
            self.documents.inner.owner,
            n,
            &|| self.check(),
            &|s, c| self.documents.fault(s, c),
        )?;
        self.check().map_err(|cause| DurableReadError {
            validation: ReadValidation::BytesRead,
            durability: ReadDurability::DirectoryAcknowledged,
            cause,
        })?;
        Ok(raw)
    }
    /// Create exact bytes only when absent, using actual NOREPLACE. Existing
    /// safe documents refuse Exists; unsafe existing objects refuse their cause.
    /// Errors include publication phase. No schema/identity/default inferred.
    pub fn create(&self, n: &str, raw: &[u8]) -> Result<Publication, WriteError> {
        self.write(n, None, raw)
    }
    /// Replace only exact expected bytes under the held cooperative reservation.
    /// Missing is an error, stale bytes conflict. No in-place truncation. Does not
    /// protect against privileged/noncooperating writers or rollback by itself.
    pub fn replace(&self, n: &str, expected: &[u8], raw: &[u8]) -> Result<Publication, WriteError> {
        self.write(n, Some(expected), raw)
    }
    /// Validate/read existing bytes and publish once; preserve physical phase
    /// and all cleanup causes. No retry or side effect after a failed prerequisite.
    fn write(
        &self,
        n: &str,
        expected: Option<&[u8]>,
        raw: &[u8],
    ) -> Result<Publication, WriteError> {
        let mut publication = Publication::NotAttempted;
        let result = (|| {
            name(n)?;
            self.check()?;
            // Existing bytes may contain keys even when create only refuses.
            // Read them through the guarded path before absence/exact-CAS checks.
            match (expected, self.read_private(n)) {
                (None, Err(Error::NotConfigured)) => (),
                (None, Ok(_)) => return Err(Error::Exists),
                (Some(want), Ok(current)) if want == current.as_bytes() => (),
                (Some(_), Ok(_)) => return Err(Error::Conflict),
                (_, Err(e)) => return Err(e),
            }
            self.ctx.check()?;
            file_io::persist_document(
                &self.handles()?.dir,
                self.documents.inner.owner,
                n,
                raw,
                expected.is_none(),
                &mut publication,
                &|s, c| self.documents.fault(s, c),
            )
        })();
        result
            .map(|()| publication)
            .map_err(|cause| WriteError { publication, cause })
    }
    /// Explicit durability acknowledgement after authorized exact readback. Does
    /// not infer current checkpoint or repeat a failed write automatically.
    pub fn sync(&self) -> Result<(), Error> {
        self.check()?;
        file_io::sync_directory(&self.handles()?.dir, &|s, c| self.documents.fault(s, c))
    }
    /// Release once, even if canceled. Return both actual and injected release
    /// causes without consuming/relabeling the caller's already known outcome.
    pub fn finish(mut self) -> Result<(), Error> {
        self.locked = false;
        let released = self.handles().and_then(|h| {
            unix::flock(&h.lock, FlockOperation::Unlock).map_err(|e| Error::io(IoClass::Storage, e))
        });
        combined(
            released,
            self.documents.fault(Stage::Unlock, IoClass::Storage),
        )
    }
}
impl Drop for Reservation<'_> {
    /// Unwind/fallback release retains errors for close and forbids silent reuse.
    /// This path does not establish successful explicit release or durability.
    fn drop(&mut self) {
        if self.locked {
            let released = self.handles().and_then(|h| {
                unix::flock(&h.lock, FlockOperation::Unlock)
                    .map_err(|e| Error::io(IoClass::Storage, e))
            });
            if let Err(e) = combined(
                released,
                self.documents.fault(Stage::Unlock, IoClass::Storage),
            ) {
                self.documents.retain_cleanup(e)
            }
        }
    }
}
