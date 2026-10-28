//! Physical preparation provenance for new native planner runs. Historical
//! receipts without this journal remain historical, never new exec authority.
use super::super::{observe_instance, RootInstanceIdentity};
use crate::{agent_launch::query, local_storage::LocalDirectory, AgentLaunchError as E};
use rusqlite::{params, Connection};

// Internal normalized representation, not an operator policy or schema37 rewrite.
const TABLE: &str = "CREATE TABLE native_planning_run_bindings(claim_id TEXT PRIMARY KEY REFERENCES native_planning_runs(claim_id),schema_version INTEGER NOT NULL CHECK(schema_version=1),directory_device TEXT NOT NULL,directory_inode TEXT NOT NULL,database_device TEXT NOT NULL,database_inode TEXT NOT NULL)";

/// Validate the optional exact auxiliary schema without writes. Reserved-name
/// collisions, unexpected DDL/index/trigger and query/finalize failures propagate.
fn schema(db: &Connection) -> Result<bool, E> {
    super::super::mission_supervision_store::auxiliary_schema(
        db,
        "native_planning_run_bindings",
        TABLE,
        "planning-binding-schema-invalid",
    )
    .map_err(|e| E::caused("planning-binding-schema", e))
}

/// Canonical u64 text retains the full filesystem representation without an
/// i64 cast or permissive numeric coercion. These are observations, not rights.
fn number(raw: String) -> Result<u64, E> {
    let n: u64 = raw
        .parse()
        .map_err(|e| E::caused("planning-binding-identity-invalid", e))?;
    if raw != n.to_string() {
        return Err(E::Refused("planning-binding-identity-invalid"));
    }
    Ok(n)
}

/// Decode one immutable original. Absence is explicit legacy unknown; neither
/// absence nor malformed rows supplies a current instance or permission.
fn load(db: &Connection, claim: &str) -> Result<Option<RootInstanceIdentity>, E> {
    if !schema(db)? {
        return Ok(None);
    }
    let row: Option<(i64, String, String, String, String)> = query(db,
        "SELECT schema_version,directory_device,directory_inode,database_device,database_inode FROM native_planning_run_bindings WHERE claim_id=?", [claim], |rows| {
            rows.next()?.map(|r| Ok::<_,rusqlite::Error>((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).transpose().map_err(E::from)
        })?;
    row.map(|(version, dd, di, bd, bi)| {
        if version != 1 {
            return Err(E::Refused("planning-binding-schema-version"));
        }
        Ok(RootInstanceIdentity {
            directory: (number(dd)?, number(di)?),
            database: (number(bd)?, number(bi)?),
        })
    })
    .transpose()
}

/// Require exact directory AND connected SQLite identity. The observation also
/// checks the directory's state.db against the real connection. Callers retaining
/// authority must hold a RootBinding across execution to prevent inode reuse.
fn equal(expected: &RootInstanceIdentity, actual: &RootInstanceIdentity) -> Result<(), E> {
    if expected != actual {
        return Err(E::Refused("planning-run-instance-changed"));
    }
    Ok(())
}

/// Insert once inside the actual run-claim transaction, after its original Run
/// row exists. The expected identity comes from the held parent RootBinding,
/// never a public request. No ALTER/UPDATE/DELETE, grant or legacy upgrade occurs.
/// SQL/schema/identity/finalize failures roll back with the caller transaction.
pub(super) fn stage(
    db: &Connection,
    directory: &LocalDirectory,
    claim: &str,
    expected: &RootInstanceIdentity,
) -> Result<(), E> {
    if db.is_autocommit() {
        return Err(E::Refused("planning-binding-transaction-required"));
    }
    let actual = observe_instance(db, directory)
        .map_err(|e| E::caused("planning-binding-observation", e))?;
    equal(expected, &actual)?;
    if !schema(db)? {
        db.execute_batch(TABLE)?;
    }
    let changed = super::super::mission_supervision_store::execute(
        db,
        "INSERT INTO native_planning_run_bindings VALUES(?,1,?,?,?,?)",
        params![
            claim,
            expected.directory.0.to_string(),
            expected.directory.1.to_string(),
            expected.database.0.to_string(),
            expected.database.1.to_string()
        ],
    )
    .map_err(|e| E::caused("planning-binding-insert", e))?;
    if changed != 1 || load(db, claim)?.as_ref() != Some(expected) {
        return Err(E::Refused("planning-binding-not-recorded"));
    }
    Ok(())
}

/// Verify the prepared physical instance before parent permission, private child
/// material reads or new decision application. Terminal historical replay may
/// explicitly tolerate missing legacy provenance, never a changed present row.
/// Read-only; observes no identity/liveness/rights of an agent from stored labels.
pub(super) fn check(
    db: &Connection,
    directory: &LocalDirectory,
    claim: &str,
    historical_only: bool,
) -> Result<(), E> {
    match load(db, claim)? {
        Some(expected) => equal(
            &expected,
            &observe_instance(db, directory)
                .map_err(|e| E::caused("planning-binding-observation", e))?,
        ),
        None if historical_only => Ok(()),
        None => Err(E::Refused("planning-run-instance-required")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Valid physical witness precedes each different-db/same-directory and
    /// different-directory case. Equal values prove consistency, not ownership.
    #[test]
    fn planner_binding_requires_both_actual_storage_identities() {
        let good = RootInstanceIdentity {
            directory: (1, 2),
            database: (3, 4),
        };
        equal(&good, &good).unwrap();
        for bad in [
            RootInstanceIdentity {
                database: (3, 5),
                ..good.clone()
            },
            RootInstanceIdentity {
                directory: (1, 9),
                ..good.clone()
            },
        ] {
            assert!(matches!(
                equal(&good, &bad),
                Err(E::Refused("planning-run-instance-changed"))
            ));
            equal(&good, &good).unwrap();
        }
    }
    /// Canonical representation has no platform i64 truncation or hidden bound.
    #[test]
    fn planner_binding_rejects_noncanonical_identity_without_narrowing_u64() {
        assert_eq!(number(u64::MAX.to_string()).unwrap(), u64::MAX);
        for bad in ["", "-1", "01", "+1", "18446744073709551616"] {
            assert!(number(bad.into()).is_err());
            assert_eq!(number(u64::MAX.to_string()).unwrap(), u64::MAX);
        }
    }
}
