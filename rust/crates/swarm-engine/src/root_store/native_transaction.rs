//! Root SQL finalization, including explicit auxiliary-write observation.
//! No provider, business completion, policy fallback or automatic retry.
use crate::{
    root_store::{RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use rusqlite::Transaction;
use std::{error::Error, fmt};

/// Callback's SQL intent, not operator policy. Filesystem effects are independent.
pub(super) enum Change<T> {
    Observe(T),
    Mutate(T),
}
/// Actual SQL-only result after a failed transaction; unknown is never rollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SqlEffect {
    NoMutation,
    Uncertain,
}
/// Preserve phase and every original SQL/callback/rollback cause. No error text
/// is parsed to infer effects; the actual rollback and autocommit decide them.
pub(crate) enum ObservedSqlFailure {
    Refused(E),
    Begin(rusqlite::Error),
    Callback {
        cause: E,
        rollback: Option<rusqlite::Error>,
        rollback_confirmed: bool,
    },
    ReadRollback(rusqlite::Error),
    Commit {
        cause: rusqlite::Error,
        cleanup: Option<rusqlite::Error>,
        rollback_confirmed: bool,
    },
}
impl ObservedSqlFailure {
    /// SQL-only mutation uncertainty, not filesystem rollback or retry permission.
    pub(crate) fn effect(&self) -> SqlEffect {
        match self {
            Self::Callback {
                rollback_confirmed: false,
                ..
            }
            | Self::Commit {
                rollback_confirmed: false,
                ..
            } => SqlEffect::Uncertain,
            _ => SqlEffect::NoMutation,
        }
    }
    /// Closed phase vocabulary for the established safe diagnostic graph.
    pub(crate) fn phase(&self) -> &'static str {
        match self {
            Self::Refused(_) => "sql-phase:admission",
            Self::Begin(_) => "sql-phase:begin",
            Self::Callback { .. } => "sql-phase:callback",
            Self::ReadRollback(_) => "sql-phase:read-rollback",
            Self::Commit { .. } => "sql-phase:commit",
        }
    }
    /// Original primary and independent rollback causes, without private messages.
    pub(crate) fn children(&self) -> Vec<(&'static str, &(dyn Error + 'static))> {
        let primary: &(dyn Error + 'static) = match self {
            Self::Refused(e) => e,
            Self::Begin(e) | Self::ReadRollback(e) => e,
            Self::Callback { cause, .. } => cause,
            Self::Commit { cause, .. } => cause,
        };
        let mut out = vec![("primary", primary)];
        let cleanup = match self {
            Self::Callback { rollback, .. } => rollback.as_ref(),
            Self::Commit { cleanup, .. } => cleanup.as_ref(),
            _ => None,
        };
        if let Some(e) = cleanup {
            out.push(("cleanup", e as &(dyn Error + 'static)))
        }
        out
    }
    /// Retain this typed SQL failure in Root without calling it an Agent effect.
    pub(crate) fn into_root(self) -> E {
        match self {
            Self::Refused(error)
            | Self::Callback {
                cause: error,
                rollback: None,
                rollback_confirmed: true,
            } => error,
            failure => E::SqlTransaction(Box::new(failure)),
        }
    }
    /// Preserve the existing review helper's exact families and cleanup envelopes.
    /// New auxiliary callers use into_root instead; no review/agent claim follows.
    fn into_legacy(self) -> E {
        let rollback = |e| E::Launch(Box::new(A::caused("review-transaction-rollback", e)));
        match self {
            Self::Refused(e) => e,
            Self::Begin(e) => E::Launch(Box::new(A::from(e))),
            Self::Callback {
                cause,
                rollback: Some(e),
                ..
            } => E::Cleanup {
                primary: Some(Box::new(cause)),
                cleanup: Box::new(rollback(e)),
            },
            Self::Callback {
                cause,
                rollback: None,
                ..
            } => cause,
            Self::ReadRollback(e) => rollback(e),
            Self::Commit {
                cause,
                cleanup,
                rollback_confirmed,
            } => E::Launch(Box::new(A::Commit {
                cause: Box::new(A::caused("review-transaction-commit", cause)),
                rollback_confirmed,
                cleanup: cleanup.map(|e| Box::new(A::caused("review-transaction-rollback", e))),
            })),
        }
    }
}
impl fmt::Display for ObservedSqlFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.phase())
    }
}
impl fmt::Debug for ObservedSqlFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ObservedSqlFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.children()[0].1)
    }
}

/// Same-connection IMMEDIATE transaction with explicit SQL outcome. Callback
/// must not finalize the transaction itself or claim Observe after writes. Nested
/// BEGIN failure leaves the caller's transaction untouched. Every callback/read
/// failure rolls back explicitly; failed commits preserve actual autocommit,
/// rollback and all causes. A failed rollback/unknown commit is uncertain, never
/// a basis for a blind retry. Callback filesystem effects are independent. No
/// policy, rights, budget, process or business completion is inferred.
pub(super) fn apply_observed<T>(
    root: &RootStore,
    operation: impl FnOnce(&mut Transaction<'_>) -> Result<Change<T>, E>,
) -> Result<T, ObservedSqlFailure> {
    if !root.writable {
        return Err(ObservedSqlFailure::Refused(E::Refused("root-read-only")));
    }
    let mut tx = Transaction::new_unchecked(&root.db, rusqlite::TransactionBehavior::Immediate)
        .map_err(ObservedSqlFailure::Begin)?;
    let result = operation(&mut tx);
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    match result {
        Err(cause) => {
            let rollback = tx.rollback().err();
            let rollback_confirmed = rollback.is_none() && root.db.is_autocommit();
            Err(ObservedSqlFailure::Callback {
                cause,
                rollback,
                rollback_confirmed,
            })
        }
        Ok(Change::Observe(value)) => tx
            .rollback()
            .map(|()| value)
            .map_err(ObservedSqlFailure::ReadRollback),
        Ok(Change::Mutate(value)) => match tx.commit() {
            Ok(()) => Ok(value),
            Err(cause) => {
                let active = !root.db.is_autocommit();
                let cleanup = if active {
                    root.db.execute_batch("ROLLBACK").err()
                } else {
                    None
                };
                let rollback_confirmed = active && cleanup.is_none() && root.db.is_autocommit();
                Err(ObservedSqlFailure::Commit {
                    cause,
                    cleanup,
                    rollback_confirmed,
                })
            }
        },
    }
}
/// Existing review adapter: same IMMEDIATE/observe/commit semantics and original
/// safe families. Direct callback SQL BUSY may reach the saved outer retry policy
/// only after clean rollback/autocommit; commit/compound errors never retry.
/// This helper itself performs no retry, resource release or permission grant.
pub(super) fn apply<T>(
    root: &RootStore,
    operation: impl FnOnce(&mut Transaction<'_>) -> Result<Change<T>, E>,
) -> Result<T, E> {
    apply_observed(root, operation).map_err(ObservedSqlFailure::into_legacy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemEnvironment;
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
    /// Owned inactive SQLite Root; failed cases retain original bytes. No mission,
    /// provider, settings adoption or real-project SQL mutation is involved.
    struct Fixture {
        root: Option<RootStore>,
        path: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(crate::generate_identifier("visit-sql-").unwrap());
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            let root = RootStore::initialize(&path, &mut SystemEnvironment).unwrap();
            root.db
                .execute_batch("CREATE TABLE visit_test_effect(value INTEGER NOT NULL)")
                .unwrap();
            Self {
                root: Some(root),
                path,
            }
        }
        fn store(&self) -> &RootStore {
            self.root.as_ref().unwrap()
        }
        fn count(&self) -> i64 {
            self.store()
                .db
                .query_row("SELECT count(*) FROM visit_test_effect", [], |r| r.get(0))
                .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Some(root) = self.root.take() {
                root.close().unwrap()
            }
            if std::thread::panicking() {
                eprintln!("owned SQL fixture retained: {}", self.path.display())
            } else {
                fs::remove_dir_all(&self.path).unwrap()
            }
        }
    }
    #[test]
    fn actual_commit_callback_and_observe_rollback() {
        let f = Fixture::new();
        apply_observed(f.store(), |tx| {
            tx.execute("INSERT INTO visit_test_effect VALUES(1)", [])
                .map_err(E::Sql)?;
            Ok(Change::Mutate(()))
        })
        .unwrap();
        assert_eq!(f.count(), 1);
        let error = apply_observed::<()>(f.store(), |tx| {
            tx.execute("INSERT INTO visit_test_effect VALUES(2)", [])
                .map_err(E::Sql)?;
            Err(E::Refused("visit-test-refusal"))
        })
        .unwrap_err();
        assert_eq!(error.effect(), SqlEffect::NoMutation);
        assert_eq!(error.into_root().code(), "visit-test-refusal");
        assert_eq!(f.count(), 1);
        // Inject a callback violating Observe's contract: helper still rolls back
        // its SQL writes. This is a fixture, not supported callback behavior.
        apply_observed(f.store(), |tx| {
            tx.execute("INSERT INTO visit_test_effect VALUES(3)", [])
                .map_err(E::Sql)?;
            Ok(Change::Observe(()))
        })
        .unwrap();
        assert_eq!(f.count(), 1);
    }
    #[test]
    fn nested_begin_preserves_callers_transaction_and_skips_callback() {
        let f = Fixture::new();
        f.store()
            .db
            .execute_batch("BEGIN; INSERT INTO visit_test_effect VALUES(1)")
            .unwrap();
        let mut called = false;
        let error = apply_observed(f.store(), |_| {
            called = true;
            Ok(Change::Mutate(()))
        })
        .unwrap_err();
        assert!(!called);
        assert!(matches!(error, ObservedSqlFailure::Begin(_)));
        assert_eq!(error.effect(), SqlEffect::NoMutation);
        assert!(!f.store().db.is_autocommit());
        assert_eq!(f.count(), 1);
        f.store().db.execute_batch("ROLLBACK").unwrap();
        assert_eq!(f.count(), 0);
    }
    #[test]
    fn actual_deferred_commit_refusal_rolls_back_and_legacy_keeps_family() {
        let f = Fixture::new();
        f.store().db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE visit_test_parent(id INTEGER PRIMARY KEY); CREATE TABLE visit_test_child(id INTEGER REFERENCES visit_test_parent(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
        let operation = |tx: &mut Transaction<'_>| {
            tx.execute("INSERT INTO visit_test_child VALUES(404)", [])
                .map_err(E::Sql)?;
            Ok(Change::Mutate(()))
        };
        let error = apply_observed(f.store(), operation).unwrap_err();
        assert!(matches!(
            error,
            ObservedSqlFailure::Commit {
                rollback_confirmed: true,
                ..
            }
        ));
        assert_eq!(error.effect(), SqlEffect::NoMutation);
        assert_eq!(
            f.store()
                .db
                .query_row("SELECT count(*) FROM visit_test_child", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let legacy = apply(f.store(), operation).unwrap_err();
        assert!(
            matches!(legacy,E::Launch(ref e) if matches!(e.as_ref(),A::Commit{rollback_confirmed:true,..}))
        );
        assert!(f.store().db.is_autocommit());
    }
    #[test]
    fn externally_finalized_callback_retains_uncertainty_and_observed_row() {
        let f = Fixture::new();
        // Deliberate contract violation in this owned fixture: simulates an
        // already-ended SQL transaction, not actual disk/IO failure evidence.
        let error = apply_observed(f.store(), |tx| {
            tx.execute_batch("INSERT INTO visit_test_effect VALUES(7); COMMIT")
                .map_err(E::Sql)?;
            Ok(Change::Mutate(()))
        })
        .unwrap_err();
        assert!(matches!(
            error,
            ObservedSqlFailure::Commit {
                rollback_confirmed: false,
                ..
            }
        ));
        assert_eq!(error.effect(), SqlEffect::Uncertain);
        assert_eq!(f.count(), 1);
        assert!(f.store().db.is_autocommit());
    }
}
