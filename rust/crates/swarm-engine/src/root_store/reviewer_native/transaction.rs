//! Candidate Root transaction finalization. Never a provider or gate retry.
use crate::{
    root_store::{finished, RootStore, RootStoreError as E},
    AgentLaunchError as A,
};
use rusqlite::Transaction;

/// Internal SQL outcome: read-only replay rolls back its read transaction, while
/// staged data requires commit. This is not configurable execution policy.
pub(super) enum Change<T> {
    Observe(T),
    Mutate(T),
}
/// Execute/finalize a same-connection IMMEDIATE transaction. Runtime nested BEGIN
/// failure leaves the caller's original transaction untouched. Every callback
/// error rolls back explicitly; read-only replay also rolls back. Commit failure
/// preserves cause, actual autocommit and any rollback error; autocommit already
/// true stays uncertain. Direct typed SQL BUSY may follow the outer saved retry
/// policy only after clean rollback and autocommit; compound/commit failures never
/// retry. This helper neither retries, releases settings, grants a socket
/// byte, changes a budget policy nor invokes a process. Callback filesystem probe
/// effects are independent and cannot be erased by SQL rollback.
pub(super) fn apply<T>(
    root: &RootStore,
    operation: impl FnOnce(&mut Transaction<'_>) -> Result<Change<T>, E>,
) -> Result<T, E> {
    if !root.writable {
        return Err(E::Refused("root-read-only"));
    }
    let mut tx = Transaction::new_unchecked(&root.db, rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| E::Launch(Box::new(A::from(error))))?;
    let result = operation(&mut tx);
    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
    let rollback = |error| E::Launch(Box::new(A::caused("review-transaction-rollback", error)));
    match result {
        Err(error) => finished(Err(error), tx.rollback().map_err(rollback)),
        Ok(Change::Observe(value)) => finished(Ok(value), tx.rollback().map_err(rollback)),
        Ok(Change::Mutate(value)) => match tx.commit() {
            Ok(()) => Ok(value),
            Err(cause) => {
                let active = !root.db.is_autocommit();
                let cleanup = if active {
                    root.db
                        .execute_batch("ROLLBACK")
                        .err()
                        .map(|error| Box::new(A::caused("review-transaction-rollback", error)))
                } else {
                    None
                };
                Err(E::Launch(Box::new(A::Commit {
                    cause: Box::new(A::caused("review-transaction-commit", cause)),
                    rollback_confirmed: active && cleanup.is_none() && root.db.is_autocommit(),
                    cleanup,
                })))
            }
        },
    }
}
