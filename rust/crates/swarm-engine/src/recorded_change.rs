//! Atomic persistence with an optional trusted SQL hook before the event.
//! Business guards, clocks, IDs, authority and final commit belong to the Store.
use crate::{
    encode_model, read_recorded_work, RecordedCommandError, RecordedWorkError, Work, WorkError,
};
use rusqlite::{params, types::ValueRef, Connection, DropBehavior, Statement, Transaction};
use std::{error::Error, fmt};
const SAVEPOINT: &str = "swarm_recorded_change";
pub struct RecordedChange<'a> {
    /// Final candidate, including next revision and timestamps supplied by caller.
    pub work: &'a Work,
    pub expected_revision: i64,
    pub event: &'a str,
    pub kind: &'a str,
    pub request: &'a [u8],
}
pub enum RecordedChangeOutcome {
    Staged,
    Replayed(Box<Work>),
}
impl fmt::Debug for RecordedChangeOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Staged => "Staged (outer commit pending)",
            Self::Replayed(_) => "Replayed (data redacted)",
        })
    }
}
pub enum RecordedChangeError {
    Command(RecordedCommandError),
    Work(RecordedWorkError),
    Encode(WorkError),
    Sql(rusqlite::Error),
    Revision {
        expected: i64,
        observed: Option<i64>,
    },
    Envelope,
    Overflow,
    Postcondition,
    Hook(Box<dyn Error + Send + Sync>),
    Finalize {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
    Recovery {
        primary: Box<Self>,
        cleanup: Option<rusqlite::Error>,
        transaction_lost: bool,
    },
}
impl RecordedChangeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Command(e) => e.code(),
            Self::Work(e) => e.code(),
            Self::Encode(e) => e.code(),
            Self::Sql(_) => "recorded-change-storage",
            Self::Revision { .. } => "revision_conflict",
            Self::Envelope => "recorded-change-envelope",
            Self::Overflow => "recorded-change-overflow",
            Self::Postcondition => "recorded-change-postcondition",
            Self::Hook(_) => "recorded-change-hook",
            Self::Finalize { .. } => "recorded-change-finalize",
            Self::Recovery {
                transaction_lost: true,
                ..
            } => "recorded-change-transaction-lost",
            Self::Recovery { .. } => "recorded-change-recovery",
        }
    }
    /// True only when SQLite was observed outside the caller's transaction.
    pub fn outer_transaction_lost(&self) -> bool {
        matches!(
            self,
            Self::Command(RecordedCommandError::InactiveTransaction)
                | Self::Recovery {
                    transaction_lost: true,
                    ..
                }
        )
    }
}
impl fmt::Display for RecordedChangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for RecordedChangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for RecordedChangeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Command(e) => Some(e),
            Self::Work(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Sql(e) => Some(e),
            Self::Hook(e) => Some(e.as_ref()),
            Self::Finalize { primary, .. } | Self::Recovery { primary, .. } => Some(primary),
            _ => None,
        }
    }
}
impl From<rusqlite::Error> for RecordedChangeError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<RecordedCommandError> for RecordedChangeError {
    fn from(e: RecordedCommandError) -> Self {
        Self::Command(e)
    }
}
impl From<RecordedWorkError> for RecordedChangeError {
    fn from(e: RecordedWorkError) -> Self {
        Self::Work(e)
    }
}
fn statement<T>(
    db: &Connection,
    sql: &str,
    run: impl FnOnce(&mut Statement<'_>) -> Result<T, RecordedChangeError>,
) -> Result<T, RecordedChangeError> {
    let mut stmt = db.prepare(sql)?;
    let result = run(&mut stmt);
    match (result, stmt.finalize()) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e.into()),
        (Err(primary), Err(cleanup)) => Err(RecordedChangeError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}
fn verify_work(
    db: &Connection,
    c: &RecordedChange<'_>,
    body: &[u8],
) -> Result<(), RecordedChangeError> {
    statement(
        db,
        "SELECT id,revision,body FROM main.works WHERE id=?",
        |stmt| {
            let mut rows = stmt.query([&c.work.id])?;
            let row = rows.next()?.ok_or(RecordedChangeError::Postcondition)?;
            if !matches!(row.get_ref(0)?,ValueRef::Text(v)if v==c.work.id.as_bytes())
                || !matches!(row.get_ref(1)?,ValueRef::Integer(v)if v==c.work.revision)
                || !matches!(row.get_ref(2)?,ValueRef::Blob(v)if v==body)
            {
                return Err(RecordedChangeError::Postcondition);
            }
            if rows.next()?.is_some() {
                return Err(RecordedChangeError::Postcondition);
            }
            Ok(())
        },
    )
}
fn verify(db: &Connection, c: &RecordedChange<'_>, body: &[u8]) -> Result<(), RecordedChangeError> {
    verify_work(db, c, body)?;
    statement(
        db,
        "SELECT id,work_id,revision,kind,at,payload,request FROM main.events WHERE id=?",
        |stmt| {
            let mut rows = stmt.query([c.event])?;
            let row = rows.next()?.ok_or(RecordedChangeError::Postcondition)?;
            for (column, expected) in [
                (0, c.event.as_bytes()),
                (1, c.work.id.as_bytes()),
                (3, c.kind.as_bytes()),
                (4, c.work.updated.as_bytes()),
            ] {
                if !matches!(row.get_ref(column)?,ValueRef::Text(v)if v==expected) {
                    return Err(RecordedChangeError::Postcondition);
                }
            }
            if !matches!(row.get_ref(2)?,ValueRef::Integer(v)if v==c.work.revision) {
                return Err(RecordedChangeError::Postcondition);
            }
            for column in [5, 6] {
                if !matches!(row.get_ref(column)?,ValueRef::Blob(v)if v==c.request) {
                    return Err(RecordedChangeError::Postcondition);
                }
            }
            if rows.next()?.is_some() {
                return Err(RecordedChangeError::Postcondition);
            }
            Ok(())
        },
    )
}
fn write<F>(
    db: &Connection,
    c: RecordedChange<'_>,
    hook: F,
) -> Result<RecordedChangeOutcome, RecordedChangeError>
where
    F: FnOnce(&Connection, &Work) -> Result<(), Box<dyn Error + Send + Sync>>,
{
    let create = c.kind == "work.create";
    if let Some(work) = crate::recorded_command::reserve_recorded_command_on(
        db,
        if create { "" } else { &c.work.id },
        c.kind,
        c.event,
        c.request,
    )? {
        return Ok(RecordedChangeOutcome::Replayed(Box::new(work)));
    }
    if create {
        if c.expected_revision != 0 {
            return Err(RecordedChangeError::Revision {
                expected: c.expected_revision,
                observed: None,
            });
        }
    } else {
        let current = read_recorded_work(db, &c.work.id)?;
        if current.revision != c.expected_revision {
            return Err(RecordedChangeError::Revision {
                expected: c.expected_revision,
                observed: Some(current.revision),
            });
        }
    }
    let next = c
        .expected_revision
        .checked_add(1)
        .ok_or(RecordedChangeError::Overflow)?;
    if c.work.schema_version != 1
        || c.work.revision < 1
        || c.work.revision != next
        || !swarm_identity_contract::name(c.work.id.as_bytes())
    {
        return Err(RecordedChangeError::Envelope);
    }
    let body = encode_model(c.work).map_err(RecordedChangeError::Encode)?;
    let changed = if create {
        statement(
            db,
            "INSERT INTO main.works(id,revision,body) VALUES(?,?,?)",
            |stmt| Ok(stmt.execute(params![c.work.id, c.work.revision, body])?),
        )?
    } else {
        statement(
            db,
            "UPDATE main.works SET revision=?,body=? WHERE id=? AND revision=?",
            |stmt| {
                Ok(stmt.execute(params![
                    c.work.revision,
                    body,
                    c.work.id,
                    c.expected_revision
                ])?)
            },
        )?
    };
    if changed != 1 {
        return Err(RecordedChangeError::Postcondition);
    }
    // A hook observes the written candidate, with no new event yet. Silent
    // triggers must not expose a substituted Work to that hook.
    verify_work(db, &c, &body)?;
    hook(db, c.work).map_err(RecordedChangeError::Hook)?;
    if db.is_autocommit() {
        return Err(RecordedCommandError::InactiveTransaction.into());
    }
    let version: i64 = statement(db, "PRAGMA main.user_version", |stmt| {
        Ok(stmt.query_row([], |r| r.get(0))?)
    })?;
    if version != crate::RECORDED_COMMAND_SCHEMA_VERSION {
        return Err(RecordedCommandError::Version {
            observed: version,
            expected: crate::RECORDED_COMMAND_SCHEMA_VERSION,
        }
        .into());
    }
    verify_work(db, &c, &body)?;
    let changed=statement(db,"INSERT INTO main.events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,?,?,?,?)",|stmt|Ok(stmt.execute(params![c.event,c.work.id,c.work.revision,c.kind,c.work.updated,c.request,c.request])?))?;
    if changed != 1 {
        return Err(RecordedChangeError::Postcondition);
    }
    verify(db, &c, &body)?;
    Ok(RecordedChangeOutcome::Staged)
}
/// Stage Work and event atomically in a recoverable savepoint. `Staged` still
/// needs outer commit; hooks and all business/authority checks are caller-owned.
/// Replay precedes candidate/revision checks, returning the current Work data.
/// Existing triggers run; detected silent changes are rejected. No retry policy.
pub fn stage_recorded_change(
    tx: &mut Transaction<'_>,
    change: RecordedChange<'_>,
) -> Result<RecordedChangeOutcome, RecordedChangeError> {
    stage_recorded_change_with_hook(tx, change, |_, _| Ok(()))
}
/// Stage a candidate, invoke a trusted engine SQL hook, then insert the event.
/// The hook must not end the transaction or perform external/FS/provider effects.
/// It receives immutable Work and grants no authority. Replay never invokes it.
/// A contract-violating early commit can persist Work without an event: loss is
/// reported, never described as rollback, and must be reconciled by the caller.
pub fn stage_recorded_change_with_hook<F>(
    tx: &mut Transaction<'_>,
    change: RecordedChange<'_>,
    hook: F,
) -> Result<RecordedChangeOutcome, RecordedChangeError>
where
    F: FnOnce(&Connection, &Work) -> Result<(), Box<dyn Error + Send + Sync>>,
{
    if tx.is_autocommit() {
        return Err(RecordedCommandError::InactiveTransaction.into());
    }
    let mut sp = tx.savepoint_with_name(SAVEPOINT)?;
    // Keep RAII rollback while executing. Disable it before explicit cleanup so
    // an observed cleanup failure is never silently retried by Drop.
    let result = write(&sp, change, hook).and_then(|outcome| {
        sp.execute_batch("RELEASE SAVEPOINT swarm_recorded_change")?;
        Ok(outcome)
    });
    sp.set_drop_behavior(DropBehavior::Ignore);
    match result {
        Ok(outcome) => Ok(outcome),
        Err(primary) => {
            if sp.is_autocommit() {
                return Err(RecordedChangeError::Recovery {
                    primary: Box::new(primary),
                    cleanup: None,
                    transaction_lost: true,
                });
            }
            let cleanup = sp
                .execute_batch("ROLLBACK TO SAVEPOINT swarm_recorded_change")
                .and_then(|()| sp.execute_batch("RELEASE SAVEPOINT swarm_recorded_change"));
            match cleanup {
                Ok(()) => Err(primary),
                Err(cleanup) => Err(RecordedChangeError::Recovery {
                    primary: Box::new(primary),
                    cleanup: Some(cleanup),
                    transaction_lost: sp.is_autocommit(),
                }),
            }
        }
    }
}
