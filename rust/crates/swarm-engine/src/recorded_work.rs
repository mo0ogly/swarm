//! SQL data readers on a caller-owned connection/transaction. No root opening,
//! migration, retry, adoption, filesystem setup or authority is performed here.
use crate::{decode_work_row, Work, WorkError};
use rusqlite::{types::ValueRef, Connection, Params, Rows};
use std::fmt;

pub enum RecordedWorkError {
    Sql(rusqlite::Error),
    Work(WorkError),
    Missing,
    Finalize {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
}
impl RecordedWorkError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sql(_) => "work-storage",
            Self::Work(e) => e.code(),
            Self::Missing => "work-missing",
            Self::Finalize { .. } => "work-storage-cleanup",
        }
    }
}
impl fmt::Display for RecordedWorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for RecordedWorkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for RecordedWorkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sql(e) => Some(e),
            Self::Work(e) => Some(e),
            Self::Finalize { primary, .. } => Some(primary),
            Self::Missing => None,
        }
    }
}
impl From<rusqlite::Error> for RecordedWorkError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<WorkError> for RecordedWorkError {
    fn from(e: WorkError) -> Self {
        Self::Work(e)
    }
}

fn bytes<'a>(row: &'a rusqlite::Row<'_>, column: usize) -> rusqlite::Result<&'a [u8]> {
    match row.get_ref(column)? {
        ValueRef::Blob(v) | ValueRef::Text(v) => Ok(v),
        value => Err(rusqlite::Error::InvalidColumnType(
            column,
            "work data".into(),
            value.data_type(),
        )),
    }
}
fn decode(row: &rusqlite::Row<'_>) -> Result<Work, RecordedWorkError> {
    let id = std::str::from_utf8(bytes(row, 0)?).map_err(WorkError::Utf8)?;
    Ok(decode_work_row(
        id,
        crate::sqlite_value::integer(row.get_ref(1)?, 1)?,
        bytes(row, 2)?,
    )?)
}
fn query<T>(
    connection: &Connection,
    sql: &str,
    parameters: impl Params,
    read: impl FnOnce(&mut Rows<'_>) -> Result<T, RecordedWorkError>,
) -> Result<T, RecordedWorkError> {
    let mut statement = connection.prepare(sql)?;
    let result = match statement.query(parameters) {
        Ok(mut rows) => read(&mut rows),
        Err(e) => Err(e.into()),
    };
    match (result, statement.finalize()) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e.into()),
        (Err(primary), Err(cleanup)) => Err(RecordedWorkError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}
/// Check the persisted schema/id/revision envelope, not permission to read it.
/// A transaction dereferences to Connection and retains its existing snapshot.
pub fn read_recorded_work(connection: &Connection, id: &str) -> Result<Work, RecordedWorkError> {
    if !swarm_identity_contract::name(id.as_bytes()) {
        return Err(WorkError::Identity.into());
    }
    query(
        connection,
        "SELECT id,revision,body FROM main.works WHERE id=?",
        [id],
        |rows| {
            let row = rows.next()?.ok_or(RecordedWorkError::Missing)?;
            decode(row)
        },
    )
}
/// Actual Go order: newest rowid first. Any invalid row rejects the whole list;
/// a successfully read empty table is the only successful empty result.
pub fn list_recorded_works(connection: &Connection) -> Result<Vec<Work>, RecordedWorkError> {
    query(
        connection,
        "SELECT id,revision,body FROM main.works ORDER BY rowid DESC",
        [],
        |rows| {
            let mut result = Vec::new();
            while let Some(row) = rows.next()? {
                result.push(decode(row)?)
            }
            Ok(result)
        },
    )
}
