//! Explicit synthetic legacy-memory fixture writer; never a product command.
use std::{
    error::Error,
    path::{Path, PathBuf},
};

type Failure = Box<dyn Error>;
#[derive(Debug)]
struct Cleanup {
    primary: Option<Failure>,
    cleanup: Failure,
}
impl std::fmt::Display for Cleanup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fixture cleanup {:?}; primary {:?}",
            self.cleanup, self.primary
        )
    }
}
impl Error for Cleanup {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.primary.as_deref().unwrap_or(self.cleanup.as_ref()))
    }
}
/// Preserve the operation and close/rollback causes; never retry a fixture write.
fn finish<T>(value: Result<T, Failure>, closed: Result<(), Failure>) -> Result<T, Failure> {
    match (value, closed) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (value, Err(cleanup)) => Err(Box::new(Cleanup {
            primary: value.err(),
            cleanup,
        })),
    }
}

/// Seed only two fixture fields on a marked temporary Root. Decode the complete
/// Work, require the original envelope/revision, prohibit all nonmemory changes,
/// and use the actual identity UDF/triggers and a checked transaction/readback.
/// This does not admit a version, alter budgets, launch an agent or confer rights.
/// All failures propagate; the original row survives an uncommitted transaction.
/// An owner marker is a fixture precondition, not hostile same-UID confinement.
/// Arguments are root, work ID, candidate and explicit configured fixture area;
/// both paths are canonicalized and /tmp (including descendants) is forbidden.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err("fixture requires root, work, candidate, configured area".into());
    }
    let root = PathBuf::from(&args[0]).canonicalize()?;
    let work = args[1].to_str().ok_or("fixture work must be UTF8")?;
    let area = PathBuf::from(&args[3]).canonicalize()?;
    if !owned_fixture_path(&root, &area) {
        return Err("temporary fixture root required".into());
    }
    let marker = std::fs::read(root.join(".swarm/memory-source-fixture-owner"))?;
    if marker
        != format!(
            "swarm-memory-source-fixture-v1\n{}\n{work}\n",
            root.display()
        )
        .as_bytes()
    {
        return Err("memory fixture marker required".into());
    }
    let raw = std::fs::read(&args[2])?;
    let candidate = swarm_engine::decode_work(&raw)?;
    let mut db = rusqlite::Connection::open_with_flags(
        root.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let result = (|| -> Result<(), Failure> {
        swarm_engine::register_identity_plan_projection(&db)?;
        let mut tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = (|| -> Result<(), Failure> {
            let original = swarm_engine::read_recorded_work(&tx, work)?;
            let mut ordinary = candidate.clone();
            ordinary.retex = original.retex.clone();
            ordinary.project_memory = original.project_memory.clone();
            if ordinary != original {
                return Err("fixture only permits memory and RETEX changes".into());
            }
            let body = swarm_engine::encode_model(&candidate)?;
            if swarm_engine::identity_plan_digest(&body)?
                != swarm_engine::identity_plan_digest(&swarm_engine::encode_model(&original)?)?
            {
                return Err("fixture identity plan changed".into());
            }
            if tx.execute(
                "UPDATE works SET body=? WHERE id=? AND revision=?",
                rusqlite::params![body, work, original.revision],
            )? != 1
                || swarm_engine::read_recorded_work(&tx, work)? != candidate
            {
                return Err("fixture readback differs".into());
            }
            Ok(())
        })();
        match result {
            Err(error) => finish(
                Err(error),
                tx.rollback().map_err(|e| Box::new(e) as Failure),
            ),
            Ok(()) => match tx.commit() {
                Ok(()) => Ok(()),
                Err(error) => {
                    let primary = Err(Box::new(error) as Failure);
                    if db.is_autocommit() {
                        primary
                    } else {
                        finish(
                            primary,
                            db.execute_batch("ROLLBACK")
                                .map_err(|e| Box::new(e) as Failure),
                        )
                    }
                }
            },
        }
    })();
    finish(result, db.close().map_err(|(_, e)| Box::new(e) as Failure))
}

/// Check canonical fixture placement before reading a marker or opening SQL.
/// The explicit area comes from the test harness's Admin setting, not a product
/// capability. Prefix and direct-parent checks prevent accidental live-root use;
/// /tmp is always refused. This pure guard grants no hostile-writer confinement.
fn owned_fixture_path(root: &Path, area: &Path) -> bool {
    !area.starts_with("/tmp") && root.parent() == Some(area)
        && root.file_name().and_then(|n|n.to_str()).is_some_and(|n|n.starts_with("swarm-model83-"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_fixture_area_guard_retains_refusals() {
        let area=Path::new("/configured-fixtures");
        assert!(owned_fixture_path(&area.join("swarm-model83-valid"),area));
        for (root,area) in [("/tmp/swarm-model83-old","/tmp"),
            ("/tmp/nested/swarm-model83-old","/tmp/nested"),
            ("/elsewhere/swarm-model83-wrong","/configured-fixtures"),
            ("/configured-fixtures/live-root","/configured-fixtures"),
            ("/configured-fixtures/nested/swarm-model83-wrong","/configured-fixtures")] {
            assert!(!owned_fixture_path(Path::new(root),Path::new(area)));
        }
    }
}
