//! Existing-only service connection. It pins the opened SQLite object, not a
//! pathname or UID. Bootstrap and directory protection remain caller duties.
use super::*;
use std::sync::{Mutex, MutexGuard, TryLockError};

/// A retained connection to an already initialized, signed WAL journal. It never
/// creates a database or follows its pathname again after opening. Operations
/// and close refuse contention immediately; there is no queue or retry policy.
/// This is local object binding, not OS confinement or rollback protection.
pub struct BoundJournal {
    journal: Journal,
    state: Mutex<ConnectionState>,
}
struct ConnectionState {
    connection: Option<Connection>,
    closed: bool,
}
impl BoundJournal {
    /// Open an existing WAL database, validate its signed history and retain its
    /// actual connection. Pins/path must come from independent protected startup;
    /// NOFOLLOW rejects links in the final component only. No schema is created.
    /// SQLite/I/O/cancellation failures retain their typed cause.
    pub fn open(
        ctx: &Context,
        path: impl AsRef<Path>,
        key: &[u8],
        project: [u8; 32],
    ) -> Result<Self, Error> {
        if key.len() != 32 || !path.as_ref().is_absolute() {
            return Err(Error::Context);
        }
        ctx.check()?;
        let journal = Journal {
            path: path.as_ref().to_path_buf(),
            key: key.try_into().map_err(|_| Error::Context)?,
            key_id: hash(key),
            project: hex(&project),
            closed: RwLock::new(false),
        };
        let c = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        let initialized = (|| {
            Journal::configure(ctx, &c)?;
            let mode: String = c.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
            if mode != "wal" {
                return Err(Error::Context);
            }
            journal.transaction(ctx, &c, |c| journal.load(ctx, c).map(|_| ()))
        })();
        if let Err(cause) = initialized {
            return match c.close() {
                Ok(()) => Err(cause),
                Err((_, close)) => Err(Error::Close {
                    cause: Box::new(cause),
                    close,
                }),
            };
        }
        Ok(Self {
            journal,
            state: Mutex::new(ConnectionState {
                connection: Some(c),
                closed: false,
            }),
        })
    }
    /// Acquire the one connection without waiting. Poisoned state is refused;
    /// cancellation and authority validation remain part of each transaction.
    fn lock(&self) -> Result<MutexGuard<'_, ConnectionState>, Error> {
        self.state.try_lock().map_err(|e| match e {
            TryLockError::WouldBlock => Error::Busy,
            TryLockError::Poisoned(_) => Error::State,
        })
    }
    /// Resolve an active record at the independently supplied current head and
    /// hold the same SQLite reservation until callback completion and COMMIT.
    /// No reentry/wait on another writer. COMMIT/cancellation may fail after the
    /// callback: consumers must retain external outcome uncertainty themselves.
    pub fn with_active<T>(
        &self,
        ctx: &Context,
        expected: &Head,
        id: &str,
        f: impl FnOnce(Record) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if !expected.valid() {
            return Err(Error::Head);
        }
        let state = self.lock()?;
        if state.closed {
            return Err(Error::Closed);
        }
        let c = state.connection.as_ref().ok_or(Error::Closed)?;
        Journal::configure(ctx, c)?;
        self.journal.active_on(ctx, c, expected, id, f)
    }
    /// Reject future operations and explicitly close SQLite. Contention returns
    /// Busy without closing; a failed close retains the connection for a later
    /// explicit close, while operations remain Closed. Drop is not a close proof.
    pub fn close(&self) -> Result<(), Error> {
        let mut state = self.lock()?;
        state.closed = true;
        if let Some(c) = state.connection.take() {
            if let Err((c, cause)) = c.close() {
                state.connection = Some(c);
                return Err(Error::Sqlite(cause));
            }
        }
        Ok(())
    }
}
