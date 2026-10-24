//! Transactional component conversion. Not a root Store/schema migration and
//! not an authority, publication or anti-rollback witness.
use crate::{
    identity_plan_digest_for, IdentityPlanError, IdentityPlanProjection, IDENTITY_PLAN_HEADS_SQL,
};
use rusqlite::{params, types::ValueRef, Connection, DropBehavior, Transaction};
use std::{collections::BTreeMap, fmt};
use swarm_identity_contract::{name, Scope};

const UPGRADE_SQL: &str = include_str!("identity_plan_upgrade.sql");
const SAVEPOINT: &str = "swarm_identity_plan_projection_upgrade";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityPlanStamp {
    pub work_id: String,
    pub generation: i64,
    pub digest: String,
    pub projection: IdentityPlanProjection,
}
impl IdentityPlanStamp {
    /// Data equality only. This verifies no grant, actor, adoption or witness.
    pub fn matches_scope(&self, scope: &Scope) -> bool {
        self.work_id == scope.work_id
            && self.generation == scope.plan_version
            && self.digest == scope.plan_digest
    }
}

pub enum IdentityPlanUpgradeError {
    Sql(rusqlite::Error),
    Projection(IdentityPlanError),
    Schema,
    InactiveTransaction,
    Stamp,
    Overflow,
    Cleanup {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
}
impl IdentityPlanUpgradeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sql(_) => "identity_plan_storage",
            Self::Projection(_) => "agent_identity_plan_invalid",
            Self::Schema => "identity_plan_projection_schema",
            Self::InactiveTransaction => "identity_plan_transaction_inactive",
            Self::Stamp => "agent_identity_plan_stamp_changed",
            Self::Overflow => "identity_plan_generation_overflow",
            Self::Cleanup { .. } => "identity_plan_upgrade_cleanup",
        }
    }
}
impl fmt::Display for IdentityPlanUpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for IdentityPlanUpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for IdentityPlanUpgradeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sql(e) => Some(e),
            Self::Projection(e) => Some(e),
            Self::Cleanup { primary, .. } => Some(primary),
            _ => None,
        }
    }
}
impl From<rusqlite::Error> for IdentityPlanUpgradeError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<IdentityPlanError> for IdentityPlanUpgradeError {
    fn from(e: IdentityPlanError) -> Self {
        Self::Projection(e)
    }
}

fn body(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<Vec<u8>> {
    match row.get_ref(column)? {
        ValueRef::Blob(v) | ValueRef::Text(v) => Ok(v.to_vec()),
        value => Err(rusqlite::Error::InvalidColumnType(
            column,
            "body".into(),
            value.data_type(),
        )),
    }
}

/// Verify a data stamp at an explicitly selected format. Historical v1 reads
/// must never be used as a substitute for the current consumer's v2 expectation.
pub fn read_identity_plan_stamp(
    connection: &Connection,
    work: &str,
    expected: IdentityPlanProjection,
) -> Result<IdentityPlanStamp, IdentityPlanUpgradeError> {
    if !name(work.as_bytes()) {
        return Err(IdentityPlanUpgradeError::Stamp);
    }
    let (generation, stored, version, raw): (i64, String, i64, Vec<u8>) = connection.query_row(
        "SELECT p.generation,p.digest,p.projection_version,w.body FROM main.works w JOIN main.agent_identity_plan_heads p ON p.work_id=w.id WHERE w.id=?",
        [work], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,body(r,3)?)))?;
    if generation < 1
        || version != expected as i64
        || stored != identity_plan_digest_for(&raw, expected)?
    {
        return Err(IdentityPlanUpgradeError::Stamp);
    }
    Ok(IdentityPlanStamp {
        work_id: work.into(),
        generation,
        digest: stored,
        projection: expected,
    })
}

// Match the actual v37 producer's schema, rather than replacing unknown objects.
// SQLite stores CREATE statements with IF NOT EXISTS normalized; derive that
// representation using the same SQLite library, without touching the caller DB.
fn component_schema(connection: &Connection) -> rusqlite::Result<BTreeMap<String, String>> {
    let mut query = connection.prepare("SELECT name,sql FROM main.sqlite_schema WHERE name IN ('agent_identity_plan_heads','agent_identity_plan_insert','agent_identity_plan_update_guard','agent_identity_plan_update') OR (tbl_name='agent_identity_plan_heads' AND sql IS NOT NULL)")?;
    let result = query
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect();
    result
}
fn verify_legacy_schema(connection: &Connection) -> Result<(), IdentityPlanUpgradeError> {
    let foreign_keys: i64 = connection.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if foreign_keys != 1 || version != 37 {
        return Err(IdentityPlanUpgradeError::Schema);
    }
    // SQLite resolves temporary objects before unqualified main objects. Never
    // let a caller's temp table/view/trigger redirect this component conversion.
    let temporary: i64 = connection.query_row("SELECT count(*) FROM temp.sqlite_schema WHERE name IN ('works','agent_identity_plan_heads','agent_identity_plan_heads_v2','agent_identity_plan_projection_history','agent_identity_plan_insert','agent_identity_plan_update_guard','agent_identity_plan_update','agent_identity_plan_history_update','agent_identity_plan_history_delete')", [], |r| r.get(0))?;
    if temporary != 0 {
        return Err(IdentityPlanUpgradeError::Schema);
    }
    let reference = Connection::open_in_memory()?;
    let check = (|| -> Result<(), IdentityPlanUpgradeError> {
        crate::register_identity_plan_projection(&reference)?;
        reference.execute_batch("CREATE TABLE works(id TEXT PRIMARY KEY,body BLOB);")?;
        reference.execute_batch(IDENTITY_PLAN_HEADS_SQL)?;
        if component_schema(connection)? != component_schema(&reference)? {
            return Err(IdentityPlanUpgradeError::Schema);
        }
        // DROP TABLE must not cascade into another contract's records.
        let tables: Vec<String> = connection
            .prepare("SELECT name FROM main.sqlite_schema WHERE type='table'")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for table in tables {
            let count: i64 = connection.query_row("SELECT count(*) FROM pragma_foreign_key_list(?,'main') WHERE \"table\"='agent_identity_plan_heads'", [table], |r|r.get(0))?;
            if count != 0 {
                return Err(IdentityPlanUpgradeError::Schema);
            }
        }
        Ok(())
    })();
    let close = reference.close().map_err(|(_, e)| e);
    match (check, close) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err(e)) => Err(e.into()),
        (Err(primary), Err(cleanup)) => Err(IdentityPlanUpgradeError::Cleanup {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}

fn convert(connection: &Connection) -> Result<Vec<IdentityPlanStamp>, IdentityPlanUpgradeError> {
    verify_legacy_schema(connection)?;
    let orphans: i64 = connection.query_row("SELECT count(*) FROM main.agent_identity_plan_heads p LEFT JOIN main.works w ON w.id=p.work_id WHERE w.id IS NULL", [], |r|r.get(0))?;
    if orphans != 0 {
        return Err(IdentityPlanUpgradeError::Stamp);
    }
    let ids: Vec<String> = connection
        .prepare("SELECT id FROM main.works ORDER BY id")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut transitions = Vec::new();
    for id in ids {
        let old = read_identity_plan_stamp(connection, &id, IdentityPlanProjection::V1)?;
        let raw = connection.query_row("SELECT body FROM main.works WHERE id=?", [&id], |r| {
            body(r, 0)
        })?;
        let generation = old
            .generation
            .checked_add(1)
            .ok_or(IdentityPlanUpgradeError::Overflow)?;
        let digest = identity_plan_digest_for(&raw, IdentityPlanProjection::V2)?;
        transitions.push((
            old,
            IdentityPlanStamp {
                work_id: id,
                generation,
                digest,
                projection: IdentityPlanProjection::V2,
            },
        ));
    }
    // All old stamps checked before schema or data changes. The savepoint also
    // covers later SQL/constraint failures, including name collisions.
    connection.execute_batch(UPGRADE_SQL)?;
    for (old, new) in &transitions {
        connection.execute(
            "INSERT INTO main.agent_identity_plan_projection_history VALUES(?,?,?,?,?,?,?)",
            params![
                old.work_id,
                old.generation,
                old.digest,
                old.projection as i64,
                new.generation,
                new.digest,
                new.projection as i64
            ],
        )?;
        connection.execute(
            "INSERT INTO main.agent_identity_plan_heads_v2 VALUES(?,?,?,?)",
            params![
                new.work_id,
                new.generation,
                new.digest,
                new.projection as i64
            ],
        )?;
    }
    connection.execute_batch(include_str!("identity_plan_upgrade_finish.sql"))?;
    for (_, new) in &transitions {
        if read_identity_plan_stamp(connection, &new.work_id, IdentityPlanProjection::V2)? != *new {
            return Err(IdentityPlanUpgradeError::Stamp);
        }
    }
    Ok(transitions.into_iter().map(|(_, new)| new).collect())
}

/// Convert only the identity-head component in a caller-owned transaction.
/// Does not commit that transaction, update user_version, issue grants or write
/// an external rollback witness. Root Store integration must do those separately
/// before exposing the DB. Repeated conversion is refused, never a downgrade.
pub fn upgrade_identity_plan_projection(
    transaction: &mut Transaction<'_>,
) -> Result<Vec<IdentityPlanStamp>, IdentityPlanUpgradeError> {
    if transaction.is_autocommit() {
        return Err(IdentityPlanUpgradeError::InactiveTransaction);
    }
    let mut savepoint = transaction.savepoint_with_name(SAVEPOINT)?;
    let result = convert(&savepoint).and_then(|stamps| {
        // Retain the savepoint on RELEASE error to observe cleanup errors too.
        savepoint.execute_batch("RELEASE SAVEPOINT swarm_identity_plan_projection_upgrade")?;
        Ok(stamps)
    });
    match result {
        Ok(stamps) => {
            savepoint.set_drop_behavior(DropBehavior::Ignore);
            Ok(stamps)
        }
        Err(primary) => match savepoint.finish() {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(IdentityPlanUpgradeError::Cleanup {
                primary: Box::new(primary),
                cleanup,
            }),
        },
    }
}
