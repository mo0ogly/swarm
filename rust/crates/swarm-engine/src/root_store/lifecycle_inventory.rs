//! Readonly lifecycle metadata, never archive integrity or permission to restore.
use super::{finished, material_path, RootStore, RootStoreError as E};
use rusqlite::{types::ValueRef, Connection, Row};

/// Ordered Go lifecycleMissions view. States describe recorded metadata only;
/// `trash` does not establish snapshot integrity, restorability or authorization.
/// Changed-at is omitted when empty, including an archived row with empty time.
#[derive(serde::Serialize)]
pub struct LifecycleMission {
    pub work_id: String,
    pub title: String,
    pub revision: i64,
    pub state: String,
    pub state_label: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub changed_at: String,
}
/// Match database/sql string conversion, then Go JSON invalid-UTF substitution.
/// NULL is an error here; only the explicit archive optional column admits NULL.
fn text(row: &Row<'_>, column: usize) -> Result<String, E> {
    let value = crate::sqlite_value::string(row.get_ref(column).map_err(E::Sql)?, column)
        .map_err(E::Sql)?;
    Ok(crate::text::go_utf8(&value))
}
/// Fully finish one statement without discarding a primary read/decode failure.
/// No cap, skip, retry or partial-success return is introduced.
fn collect(
    db: &Connection,
    sql: &str,
    mut decode: impl FnMut(&Row<'_>) -> Result<LifecycleMission, E>,
) -> Result<Vec<LifecycleMission>, E> {
    let mut statement = db.prepare(sql).map_err(E::Sql)?;
    let result = (|| {
        let mut rows = statement.query([]).map_err(E::Sql)?;
        let mut items = Vec::new();
        while let Some(row) = rows.next().map_err(E::Sql)? {
            items.push(decode(row)?);
        }
        Ok(items)
    })();
    finished(result, statement.finalize().map_err(E::Sql))
}
impl RootStore {
    /// Read active/archived Works newest rowid first, then trash by deleted_at
    /// descending, in one SQL snapshot under shared settings/physical Root checks.
    /// Validate every real Work envelope; trash is metadata, never a decoded or
    /// verified recovery snapshot. Empty tables are the only empty success.
    /// Preserve SQL, decoder, statement/transaction finalization, post-check and
    /// release errors; no mutation/provider, implicit lifecycle action or policy.
    /// Memory grows with retained metadata; archive/trash files are not inspected.
    pub fn lifecycle_missions(&self) -> Result<Vec<LifecycleMission>, E> {
        material_path::with_material_read(self, || {
            let mut tx = self.db.unchecked_transaction().map_err(E::Sql)?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let result = (|| {
                let mut items = collect(&tx, "SELECT w.id,w.revision,w.body,l.archived_at FROM main.works w LEFT JOIN main.mission_lifecycle l ON l.work_id=w.id ORDER BY w.rowid DESC", |row| {
                    let id = text(row, 0)?;
                    let revision = crate::sqlite_value::integer(row.get_ref(1).map_err(E::Sql)?, 1).map_err(E::Sql)?;
                    let raw = crate::sqlite_value::string(row.get_ref(2).map_err(E::Sql)?, 2).map_err(E::Sql)?;
                    let work = crate::decode_work_row(&id, revision, &raw).map_err(E::Encode)?;
                    let archived = !matches!(row.get_ref(3).map_err(E::Sql)?, ValueRef::Null);
                    let changed_at = if archived { text(row, 3)? } else { String::new() };
                    Ok(LifecycleMission {work_id:id,title:work.title,revision,state:if archived {"archived"} else {"active"}.into(),state_label:if archived {"Archivée"} else {"Active"}.into(),changed_at})
                })?;
                items.extend(collect(&tx, "SELECT work_id,title,revision,deleted_at FROM main.mission_trash ORDER BY deleted_at DESC", |row| {
                    Ok(LifecycleMission {work_id:text(row,0)?,title:text(row,1)?,revision:crate::sqlite_value::integer(row.get_ref(2).map_err(E::Sql)?,2).map_err(E::Sql)?,state:"trash".into(),state_label:"Dans la corbeille".into(),changed_at:text(row,3)?})
                })?);
                Ok(items)
            })();
            finished(result, tx.rollback().map_err(E::Sql))
        })
    }
}
