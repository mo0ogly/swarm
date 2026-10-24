//! Writer reservation and replay observation only. This does not authorize,
//! validate or apply a new command. The caller must retain the same transaction
//! through all remaining guards/mutations and end it explicitly, including errors.
use crate::{read_recorded_work, RecordedWorkError, Work, WorkError};
use rusqlite::{types::ValueRef, Connection, Statement, Transaction};
use std::{error::Error, fmt};
/// Technical storage epoch, bound to the actual Go producer by qualification.
/// Identity plan projection versions are independent of this global epoch.
pub const RECORDED_COMMAND_SCHEMA_VERSION: i64 = 37;
pub enum RecordedCommandError {
    Identity,
    InactiveTransaction,
    Version {
        observed: i64,
        expected: i64,
    },
    Conflict,
    Sql(rusqlite::Error),
    Work(RecordedWorkError),
    Finalize {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
}
impl RecordedCommandError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Identity => "event-identity",
            Self::InactiveTransaction => "command-transaction-inactive",
            Self::Version { .. } => "storage_version_conflict",
            Self::Conflict => "event_conflict",
            Self::Sql(_) => "command-storage",
            Self::Work(e) => e.code(),
            Self::Finalize { .. } => "command-storage-cleanup",
        }
    }
    /// Preserve extended diagnostics. Classification grants no retry policy;
    /// BUSY_SNAPSHOT requires ending the stale transaction, not sleeping in it.
    pub fn sqlite_extended_code(&self) -> Option<i32> {
        let mut cause = self.source();
        while let Some(e) = cause {
            if let Some(e) = e.downcast_ref::<rusqlite::Error>() {
                return e.sqlite_error().map(|e| e.extended_code);
            }
            cause = e.source();
        }
        None
    }
    pub fn is_busy(&self) -> bool {
        self.sqlite_extended_code()
            .is_some_and(|code| code & 255 == 5)
    }
}
impl fmt::Display for RecordedCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for RecordedCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for RecordedCommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sql(e) => Some(e),
            Self::Work(e) => Some(e),
            Self::Finalize { primary, .. } => Some(primary),
            _ => None,
        }
    }
}
impl From<rusqlite::Error> for RecordedCommandError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<RecordedWorkError> for RecordedCommandError {
    fn from(e: RecordedWorkError) -> Self {
        Self::Work(e)
    }
}
fn statement<T>(
    db: &Connection,
    sql: &str,
    read: impl FnOnce(&mut Statement<'_>) -> Result<T, RecordedCommandError>,
) -> Result<T, RecordedCommandError> {
    let mut stmt = db.prepare(sql)?;
    let result = read(&mut stmt);
    match (result, stmt.finalize()) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e.into()),
        (Err(primary), Err(cleanup)) => Err(RecordedCommandError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}
/// Reserve the writer before reading the storage epoch and idempotency key.
/// `Some` is the current validated Work for identical work/kind/request bytes;
/// an empty `work` matches Go creation replay. `None` means only "not recorded".
/// No expected-revision/business/hook/permission checks or writes are completed.
/// No transaction end, retry, sleep or connection policy is performed here.
pub fn reserve_recorded_command(
    tx: &Transaction<'_>,
    work: &str,
    kind: &str,
    event: &str,
    request: &[u8],
) -> Result<Option<Work>, RecordedCommandError> {
    reserve_recorded_command_on(tx, work, kind, event, request)
}

// Internal savepoint callers retain the same writer reservation and snapshot.
pub(crate) fn reserve_recorded_command_on(
    tx: &Connection,
    work: &str,
    kind: &str,
    event: &str,
    request: &[u8],
) -> Result<Option<Work>, RecordedCommandError> {
    if !swarm_identity_contract::name(event.as_bytes()) {
        return Err(RecordedCommandError::Identity);
    }
    if tx.is_autocommit() {
        return Err(RecordedCommandError::InactiveTransaction);
    }
    statement(
        tx,
        "UPDATE main.works SET revision=revision WHERE id=?",
        |stmt| {
            stmt.execute([work])?;
            Ok(())
        },
    )?;
    let observed = statement(tx, "PRAGMA main.user_version", |stmt| {
        let mut rows = stmt.query([])?;
        let row = rows.next()?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        Ok(row.get::<_, i64>(0)?)
    })?;
    if observed != RECORDED_COMMAND_SCHEMA_VERSION {
        return Err(RecordedCommandError::Version {
            observed,
            expected: RECORDED_COMMAND_SCHEMA_VERSION,
        });
    }
    let prior = statement(
        tx,
        "SELECT id,work_id,kind,request FROM main.events WHERE id=?",
        |stmt| {
            let mut rows = stmt.query([event])?;
            let Some(row) = rows.next()? else {
                return Ok(None);
            };
            // SQL collation equality is not byte identity for an idempotency key.
            if crate::sqlite_value::string(row.get_ref(0)?, 0)? != event.as_bytes() {
                return Err(RecordedCommandError::Conflict);
            }
            // Unlike named json.RawMessage, Go's *[]byte destination accepts NULL,
            // strings and numeric scalars. Its string comparison equates nil/empty.
            let old_request = match row.get_ref(3)? {
                ValueRef::Null => Vec::new(),
                value => crate::sqlite_value::string(value, 3)?,
            };
            Ok(Some((
                crate::sqlite_value::string(row.get_ref(1)?, 1)?,
                crate::sqlite_value::string(row.get_ref(2)?, 2)?,
                old_request,
            )))
        },
    )?;
    let Some((old_work, old_kind, old_request)) = prior else {
        return Ok(None);
    };
    if (!work.is_empty() && old_work != work.as_bytes())
        || old_kind != kind.as_bytes()
        || old_request != request
    {
        return Err(RecordedCommandError::Conflict);
    }
    if !swarm_identity_contract::name(&old_work) {
        return Err(RecordedWorkError::from(WorkError::Identity).into());
    }
    let old_work =
        std::str::from_utf8(&old_work).map_err(|e| RecordedWorkError::from(WorkError::Utf8(e)))?;
    Ok(Some(read_recorded_work(tx, old_work)?))
}
