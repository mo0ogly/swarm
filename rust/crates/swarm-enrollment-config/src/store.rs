//! Linux protected Store, below the coordinated independent witness. No default
//! configuration or authenticated identity is inferred from filesystem data.
mod coordinator;
pub mod documents;
mod file_io;
mod guarded_read;
use crate::{
    decode, encode, snapshot_view, Change, Entry, Error, Head, IoClass, Pins, Projection, Snapshot,
    View,
};
pub use coordinator::Coordinator;
#[cfg(feature = "test-fixtures")]
pub use coordinator::{FixtureHook, FixturePhase};
use rustix::{
    fd::AsFd,
    fs::{self as unix, AtFlags, FileType, FlockOperation, Mode, OFlags, RenameFlags},
    io::Errno,
};
use std::{
    ffi::OsStr,
    fmt,
    fs::File,
    os::{fd::IntoRawFd, unix::ffi::OsStrExt},
    path::Path,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex, MutexGuard, TryLockError,
    },
};
use swarm_identity_contract::Principal;
use swarm_wire_json::timestamp;
use time::{OffsetDateTime, UtcOffset};
/// Explicit caller cancellation only; no implicit deadline or polling thread.
#[derive(Clone, Default)]
pub struct Context(Arc<AtomicU8>);
impl Context {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        // The first cancellation reason is immutable, like context.Context.Err.
        let _ = self
            .0
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst);
    }
    /// Record an external caller's deadline outcome; starts no timer and sets
    /// no duration or policy. The runtime owns deciding when its deadline fires.
    pub fn cancel_for_deadline(&self) {
        let _ = self
            .0
            .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst);
    }
    fn check(&self) -> Result<(), Error> {
        match self.0.load(Ordering::SeqCst) {
            0 => Ok(()),
            1 => Err(Error::Canceled),
            2 => Err(Error::DeadlineExceeded),
            _ => Err(Error::State),
        }
    }
}
/// Trusted callback receives owned copies while the reservation is held. It must
/// return authenticated authority, not echo the Change, and must not reenter or
/// wait for this Store (in particular Close). Matching data is not authentication.
pub type Authorizer =
    Arc<dyn Fn(&Context, Option<Snapshot>, Change) -> Result<Principal, Error> + Send + Sync>;
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
fn close(fd: impl IntoRawFd, class: IoClass) -> Result<(), Error> {
    nix::unistd::close(fd)
        .map_err(|e| Error::io(class, std::io::Error::from_raw_os_error(e as i32)))
}
fn safe_file(fd: impl AsFd, owner: u32) -> Result<(), Error> {
    let stat = unix::fstat(fd).map_err(|e| Error::io(IoClass::Path, e))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_mode & 0o7777 != 0o600
        || stat.st_uid != owner
        || stat.st_nlink != 1
    {
        return Err(Error::Path);
    }
    Ok(())
}
fn safe_directory(fd: impl AsFd, owner: u32) -> Result<(), Error> {
    let stat = unix::fstat(fd).map_err(|e| Error::io(IoClass::Path, e))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || stat.st_mode & 0o7777 != 0o700
        || stat.st_uid != owner
    {
        return Err(Error::Path);
    }
    Ok(())
}
/// Existing components only, no symlink following. Root/service ancestors must
/// not be group/other writable, except root-owned sticky directories. The final
/// directory is exactly 0700 owned by the actual service UID. No state created.
pub fn open_private_directory(path: &Path, owner: u32) -> Result<File, Error> {
    let raw = path.as_os_str().as_bytes();
    if rustix::process::geteuid().as_raw() != owner
        || !raw.starts_with(b"/")
        || raw.len() == 1
        || raw.contains(&0)
        || raw[1..]
            .split(|b| *b == b'/')
            .any(|p| p.is_empty() || p == b"." || p == b"..")
    {
        return Err(Error::Path);
    }
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW;
    let mut fd = unix::open("/", flags, Mode::empty()).map_err(|e| Error::io(IoClass::Path, e))?;
    let mut parts = raw[1..].split(|b| *b == b'/').peekable();
    while let Some(part) = parts.next() {
        let next = unix::openat(&fd, OsStr::from_bytes(part), flags, Mode::empty())
            .map_err(|e| Error::io(IoClass::Path, e));
        let old_close = close(fd, IoClass::Path);
        fd = match next {
            Ok(next) => match old_close {
                Ok(()) => next,
                Err(e) => return combined(Err(e), close(next, IoClass::Path)),
            },
            Err(e) => return combined(Err(e), old_close),
        };
        let check = (|| {
            let stat = unix::fstat(&fd).map_err(|e| Error::io(IoClass::Path, e))?;
            if parts.peek().is_none() {
                if stat.st_uid != owner || stat.st_mode & 0o7777 != 0o700 {
                    return Err(Error::Path);
                }
            } else if (stat.st_uid != 0 && stat.st_uid != owner)
                || (stat.st_mode & 0o22 != 0 && !(stat.st_uid == 0 && stat.st_mode & 0o1000 != 0))
            {
                return Err(Error::Path);
            }
            Ok(())
        })();
        if let Err(e) = check {
            return combined(Err(e), close(fd, IoClass::Path));
        }
    }
    Ok(File::from(fd))
}
struct Handles {
    dir: File,
    lock: File,
}
struct Inner {
    handles: Mutex<Option<Handles>>,
    owner: u32,
    pins: Pins,
    authorize: Authorizer,
    pending_cleanup: Mutex<Option<Error>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    faults: std::sync::atomic::AtomicU32,
}
/// Clones share the same handle/closed state. Open again for an independent flock.
#[derive(Clone)]
pub struct Store {
    inner: Arc<Inner>,
}
impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store")
            .field("owner", &self.inner.owner)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy)]
enum Stage {
    Entropy = 1,
    Write = 2,
    PartialWrite = 4,
    FileSync = 8,
    FileClose = 16,
    DirectorySync = 32,
    Unlink = 64,
    Unlock = 128,
    ReadClose = 256,
    StoreClose = 512,
}
#[cfg(feature = "test-fixtures")]
#[derive(Clone, Copy)]
pub enum FixtureFault {
    Entropy,
    Write,
    PartialWrite,
    FileSync,
    FileClose,
    DirectorySync,
    Unlink,
    Unlock,
    ReadClose,
    StoreClose,
}
impl Store {
    pub fn open(
        path: &Path,
        owner: u32,
        pins: Pins,
        authorize: Option<Authorizer>,
    ) -> Result<Self, Error> {
        pins.validate()?;
        let authorize = authorize.ok_or(Error::Unauthorized)?;
        let dir = open_private_directory(path, owner)?;
        let lock = unix::openat(
            &dir,
            "config.lock",
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|e| Error::io(IoClass::Path, e));
        let lock = match lock {
            Ok(fd) => File::from(fd),
            Err(e) => return combined(Err(e), close(dir, IoClass::Path)),
        };
        if let Err(e) = safe_file(&lock, owner) {
            return combined(
                combined(Err(e), close(lock, IoClass::Path)),
                close(dir, IoClass::Path),
            );
        }
        Ok(Self {
            inner: Arc::new(Inner {
                handles: Mutex::new(Some(Handles { dir, lock })),
                owner,
                pins,
                authorize,
                pending_cleanup: Mutex::new(None),
                #[cfg(any(test, feature = "test-fixtures"))]
                faults: std::sync::atomic::AtomicU32::new(0),
            }),
        })
    }
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
    pub fn close(&self) -> Result<(), Error> {
        let mut guard = self
            .inner
            .handles
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    fn retain_cleanup(&self, error: Error) {
        let mut pending = self
            .inner
            .pending_cleanup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *pending = Some(match pending.take() {
            Some(cause) => Error::Joined {
                cause: Box::new(cause),
                cleanup: Box::new(error),
            },
            None => error,
        });
    }
    /// Acquire immediate mutex/flock exclusion only on the still-named retained
    /// lock inode. Substitution, unsafe paths, cancellation, closed/poisoned or
    /// busy state refuse; after-lock validation releases before returning errors.
    /// No queue/retry or implicit authority/checkpoint adoption is introduced.
    fn reserve<'a>(&'a self, ctx: &Context) -> Result<Reservation<'a>, Error> {
        ctx.check()?;
        let guard = self.inner.handles.try_lock().map_err(|e| match e {
            TryLockError::WouldBlock => Error::Busy,
            TryLockError::Poisoned(_) => Error::State,
        })?;
        let h = guard.as_ref().ok_or(Error::Closed)?;
        file_io::check_handles(h, self.inner.owner, "config.lock")?;
        unix::flock(&h.lock, FlockOperation::NonBlockingLockExclusive).map_err(|e| {
            if e == Errno::WOULDBLOCK {
                Error::Busy
            } else {
                Error::io(IoClass::Storage, e)
            }
        })?;
        let r = Reservation {
            store: self,
            guard,
            locked: true,
        };
        if let Err(e) = file_io::check_handles(r.handles()?, self.inner.owner, "config.lock") {
            return r.finish(Err(e));
        }
        Ok(r)
    }
    pub fn read(&self, ctx: &Context, expected: &Head) -> Result<View, Error> {
        if expected.revision == 0 || !expected.valid() {
            return Err(Error::Head);
        }
        let r = self.reserve(ctx)?;
        let result = (|| {
            let snapshot = r.load()?;
            let view = snapshot_view(&snapshot)?;
            if &view.head != expected {
                return Err(Error::Head);
            }
            ctx.check()?;
            Ok(view)
        })();
        r.finish(result)
    }
    pub fn preview(
        &self,
        ctx: &Context,
        c: &Change,
        at: OffsetDateTime,
    ) -> Result<Projection, Error> {
        c.validate()?;
        if timestamp::zero(at) {
            return Err(Error::Format);
        }
        let r = self.reserve(ctx)?;
        let result = (|| {
            let current = match r.load() {
                Ok(s) => Some(s),
                Err(Error::NotConfigured) => None,
                Err(e) => return Err(e),
            };
            let base = match current.as_ref() {
                Some(s) => snapshot_view(s)?.head,
                None => Head::default(),
            };
            let (next, replay) = self.prepare(ctx, current.as_ref(), c, at)?;
            Ok(Projection {
                base,
                proposed: next,
                current_event_replay: replay,
            })
        })();
        r.finish(result)
    }
    pub fn create(&self, ctx: &Context, c: &Change, at: OffsetDateTime) -> Result<View, Error> {
        if c.expected_head != Head::default() {
            return Err(Error::Conflict);
        }
        self.change(ctx, c, at, true)
    }
    pub fn apply(&self, ctx: &Context, c: &Change, at: OffsetDateTime) -> Result<View, Error> {
        self.change(ctx, c, at, false)
    }
    fn change(
        &self,
        ctx: &Context,
        c: &Change,
        at: OffsetDateTime,
        create: bool,
    ) -> Result<View, Error> {
        c.validate()?;
        if timestamp::zero(at) {
            return Err(Error::Format);
        }
        let r = self.reserve(ctx)?;
        let result = (|| {
            let current = match r.load() {
                Ok(s) => Some(s),
                Err(Error::NotConfigured) if create => None,
                Err(e) => return Err(e),
            };
            let (next, replay) = self.prepare(ctx, current.as_ref(), c, at)?;
            if create && current.is_some() && !replay {
                return Err(Error::Exists);
            }
            if replay {
                r.sync_directory()?;
                return Ok(next);
            }
            let raw = encode(&next.snapshot)?;
            ctx.check()?;
            r.persist_document("config.json", &raw, create)?;
            Ok(next)
        })();
        r.finish(result)
    }
    fn prepare(
        &self,
        ctx: &Context,
        current: Option<&Snapshot>,
        c: &Change,
        at: OffsetDateTime,
    ) -> Result<(View, bool), Error> {
        let actor = (self.inner.authorize)(ctx, current.cloned(), c.clone())
            .map_err(|e| Error::Authorization(Box::new(e)))?;
        if actor != self.inner.pins.principal {
            return Err(Error::Unauthorized);
        }
        ctx.check()?;
        let mut snapshot = current.cloned().unwrap_or_else(|| Snapshot {
            schema_version: 1,
            pins: self.inner.pins.clone(),
            history: vec![],
        });
        let mut base = Head::default();
        if current.is_some() {
            let view = snapshot_view(&snapshot)?;
            base = view.head.clone();
            for (i, entry) in snapshot.history.iter().enumerate() {
                if entry.change.event_id == c.event_id {
                    if i == snapshot.history.len() - 1 && &entry.change == c {
                        return Ok((view, true));
                    }
                    return Err(Error::Event);
                }
            }
        }
        if c.expected_head != base {
            return Err(Error::Conflict);
        }
        if base.revision == u64::MAX {
            return Err(Error::Format);
        }
        snapshot.history.push(Entry {
            change: c.clone(),
            actor,
            at: at.checked_to_offset(UtcOffset::UTC).ok_or(Error::Format)?,
        });
        encode(&snapshot)?;
        Ok((snapshot_view(&snapshot)?, false))
    }
}
struct Reservation<'a> {
    store: &'a Store,
    guard: MutexGuard<'a, Option<Handles>>,
    locked: bool,
}
impl Reservation<'_> {
    fn handles(&self) -> Result<&Handles, Error> {
        self.guard.as_ref().ok_or(Error::Closed)
    }
    fn finish<T>(mut self, result: Result<T, Error>) -> Result<T, Error> {
        self.locked = false;
        let released = self.handles().and_then(|h| {
            unix::flock(&h.lock, FlockOperation::Unlock).map_err(|e| Error::io(IoClass::Storage, e))
        });
        combined(
            result,
            combined(released, self.store.fault(Stage::Unlock, IoClass::Storage)),
        )
    }
    fn load(&self) -> Result<Snapshot, Error> {
        let raw = self.read_document("config.json")?;
        let snapshot = decode(&raw)?;
        if snapshot.pins != self.store.inner.pins {
            return Err(Error::Pins);
        }
        Ok(snapshot)
    }
    /// Read exact private bytes with the shared protected descriptor path. Missing
    /// and explicit close failures remain errors; no default configuration.
    fn read_document(&self, name: &'static str) -> Result<Vec<u8>, Error> {
        file_io::read_document(
            &self.handles()?.dir,
            self.store.inner.owner,
            name,
            &|stage, class| self.store.fault(stage, class),
        )
    }
    /// Flush the retained directory; failure keeps publication indeterminate.
    fn sync_directory(&self) -> Result<(), Error> {
        file_io::sync_directory(&self.handles()?.dir, &|stage, class| {
            self.store.fault(stage, class)
        })
    }
    /// Same-directory temp/write/fsync/close/rename/fsync with original cleanup
    /// hooks and error ordering. Store keeps its existing result/replay contract.
    fn persist_document(
        &self,
        destination: &'static str,
        raw: &[u8],
        create: bool,
    ) -> Result<(), Error> {
        file_io::persist_document(
            &self.handles()?.dir,
            self.store.inner.owner,
            destination,
            raw,
            create,
            &mut file_io::Publication::NotAttempted,
            &|stage, class| self.store.fault(stage, class),
        )
    }
}
impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if self.locked {
            let result = self.handles().and_then(|h| {
                unix::flock(&h.lock, FlockOperation::Unlock)
                    .map_err(|e| Error::io(IoClass::Storage, e))
            });
            if let Err(e) = combined(result, self.store.fault(Stage::Unlock, IoClass::Storage)) {
                self.store.retain_cleanup(e)
            }
        }
    }
}
