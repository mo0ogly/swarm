//! Saved storage contention policy for native agent lifecycle operations, never process retry.
use crate::{
    local_storage::LocalDirectory, root_store::finished, EngineSettingsDocument, RootStore,
    RootStoreError, SettingCount,
};
use rusqlite::Connection;
use std::time::Duration;
/// Only direct, typed SQLite BUSY is eligible. Nested cleanup/rollback/commit
/// variants stay refused even when their primary cause happens to be BUSY.
/// Autocommit and reservation release are separate mandatory checks in execute.
fn retryable(error: &RootStoreError) -> bool {
    match error {
        #[cfg(target_os = "linux")]
        RootStoreError::Launch(error) => error.retryable_busy(),
        RootStoreError::Agent(crate::AgentStoreError::Sql(error)) => error
            .sqlite_error()
            .is_some_and(|e| e.code == rusqlite::ErrorCode::DatabaseBusy),
        _ => false,
    }
}
/// Only a clean nonblocking settings reservation conflict is retryable. A
/// failed close, changed publication state or unrelated permission/IO refusal
/// never becomes a retry. This check grants no callback/provider execution.
fn lock_busy(error: &crate::LocalStorageError) -> bool {
    use std::error::Error;
    error.operation == crate::LocalStorageOperation::Lock
        && error.cleanup.is_none()
        && error.publication == crate::PublicationState::NotPublished
        && error
            .source()
            .and_then(|e| e.downcast_ref::<rustix::io::Errno>())
            .is_some_and(|e| *e == rustix::io::Errno::AGAIN)
}
/// Preserve the preceding refusal when a later policy read/open also fails.
/// A fresh error is not evidence that the previous operation was rolled back.
fn prior(error: RootStoreError, previous: &mut Option<RootStoreError>) -> RootStoreError {
    match previous.take() {
        Some(primary) => RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(error),
        },
        None => error,
    }
}
/// Preserve the original domain/cause and the policy refusing another attempt.
/// Numeric metadata is safe; no supplied request, path or SQL is rendered.
fn exhausted(error: RootStoreError, attempts: u64, revision: SettingCount) -> RootStoreError {
    match error {
        #[cfg(target_os = "linux")]
        RootStoreError::Launch(error) => {
            RootStoreError::Launch(Box::new(crate::AgentLaunchError::Busy {
                cause: error,
                attempts,
                settings_revision: revision,
            }))
        }
        error => RootStoreError::AgentStorageBusy {
            cause: Box::new(error),
            attempts,
            settings_revision: revision,
        },
    }
}
/// Execute one lifecycle operation under fresh saved timeout/retries/delay.
/// Callback must own/finalize its transaction and return success only after its
/// advertised effects are recorded (or a read/preview is finished). Set recorded
/// only for durable operations so later lock-close errors preserve their effect.
/// Between safe SQLite BUSY or clean settings-lock conflicts, release settings
/// and reread the atomic saved document. Lock conflicts never run the callback.
/// The same additional-attempt budget and delay apply; no cap or
/// backoff beyond saved settings is invented. Null retries may wait indefinitely.
/// Retry requires autocommit and clean release. Commit/rollback/cleanup/nested
/// failures never retry; original and cleanup causes survive together. Storage
/// retries do not execute/relaunch providers, accept work or reset consumption.
pub(crate) fn execute<T>(
    db: &mut Connection,
    directory: &LocalDirectory,
    recorded: bool,
    operation: impl FnMut(&mut Connection, &EngineSettingsDocument) -> Result<T, RootStoreError>,
) -> Result<T, RootStoreError> {
    execute_with_boundary(db, directory, recorded, SettingsBoundary::Stable, operation)
}
/// A heartbeat writes cumulative facts under frozen execution policy. Only its
/// operational SQLite policy is reread; no launch/permission/budget decision uses
/// current settings here. Atomic settings snapshots suffice, so holding Admin's
/// shared reservation during the SQL pulse would create needless starvation.
/// Callback is fixed to pulse; this is not a caller-selectable grant bypass.
#[cfg(target_os = "linux")]
pub(crate) fn heartbeat(
    db: &mut Connection,
    directory: &LocalDirectory,
    expected: &crate::Agent,
    progress: &crate::AgentProgress,
    facts: &crate::agent_provider_facts::ProviderFactsSnapshot,
) -> Result<crate::Agent, RootStoreError> {
    execute_with_boundary(
        db,
        directory,
        true,
        SettingsBoundary::AtomicSnapshot,
        |db, _settings| {
            crate::agent_execution_store::pulse(db, expected, progress, facts)
                .map_err(|e| RootStoreError::Launch(Box::new(e)))
        },
    )
}
/// Read an owned Agent during private helper startup using current saved
/// contention settings. Fixed read-only callback preserves stop/ownership checks
/// without racing the helper's zero-wait Root bootstrap with a parent write.
/// Atomic settings snapshot is sufficient: no permission/budget decision occurs.
/// Original read/rollback/cleanup causes survive; never retries a provider.
#[cfg(target_os = "linux")]
pub(crate) fn pending_gate_owner(
    db: &mut Connection,
    directory: &LocalDirectory,
    expected: &crate::Agent,
) -> Result<crate::Agent, RootStoreError> {
    execute_with_boundary(db, directory, true, SettingsBoundary::AtomicSnapshot, |db, _settings| {
        crate::agent_execution_store::pending_gate_owner(db, expected)
            .map_err(|error| RootStoreError::Launch(Box::new(error)))
    })
}
/// Internal consistency boundary, not an operator execution choice. Startup and
/// settlement keep settings stable; frozen-policy heartbeat needs only a complete
/// atomic snapshot for storage contention policy. Neither mode retries a process.
#[derive(Clone, Copy)]
enum SettingsBoundary {
    Stable,
    #[cfg(target_os = "linux")]
    AtomicSnapshot,
}
/// Shared retry/accounting loop. Every transaction callback finishes before a
/// retry; locks close explicitly, and policy is freshly read on each attempt.
fn execute_with_boundary<T>(
    db: &mut Connection,
    directory: &LocalDirectory,
    recorded: bool,
    boundary: SettingsBoundary,
    operation: impl FnMut(&mut Connection, &EngineSettingsDocument) -> Result<T, RootStoreError>,
) -> Result<T, RootStoreError> {
    execute_context(db, |db| db, directory, recorded, boundary, operation)
}
/// Adapter for a Root-owned connection borrowed by its own native snapshots.
/// The callback must finalize its runtime-checked transaction before returning;
/// shared borrowing grants no nested transaction, second connection or process
/// retry. Uses the exact common saved contention/lock/cleanup loop below; normal
/// reviewer lifecycle callbacks must finalize their own transaction before return.
pub(crate) fn execute_borrowed<T>(
    db: &Connection,
    directory: &LocalDirectory,
    recorded: bool,
    mut operation: impl FnMut(&Connection, &EngineSettingsDocument) -> Result<T, RootStoreError>,
) -> Result<T, RootStoreError> {
    let mut context = db;
    execute_context(
        &mut context,
        |db| *db,
        directory,
        recorded,
        SettingsBoundary::Stable,
        |db, settings| operation(db, settings),
    )
}
/// One retry loop for mutable lifecycle and borrowed Root callbacks. The selector
/// observes the same original connection before/after each completed callback;
/// it neither opens another database nor changes policy. Autocommit, clean lock
/// release and direct typed BUSY remain mandatory; compound/commit refusals do
/// not retry. Callback output and errors preserve the existing recorded-state
/// boundary. No provider, callback permission or business acceptance is granted.
fn execute_context<T, C>(
    context: &mut C,
    connection: impl Fn(&C) -> &Connection,
    directory: &LocalDirectory,
    recorded: bool,
    boundary: SettingsBoundary,
    mut operation: impl FnMut(&mut C, &EngineSettingsDocument) -> Result<T, RootStoreError>,
) -> Result<T, RootStoreError> {
    let mut attempts = 0u64;
    let mut previous: Option<RootStoreError> = None;
    loop {
        let reservation = match boundary {
            #[cfg(target_os = "linux")]
            SettingsBoundary::AtomicSnapshot => None,
            SettingsBoundary::Stable => Some(match directory.reserve(false, false) {
                Ok(lock) => lock,
                Err(error) if lock_busy(&error) && connection(context).is_autocommit() => {
                    // Settings publish by atomic replacement. Read a complete old
                    // or new document without a reservation only to decide whether
                    // another lock attempt is allowed; never execute the callback.
                    let settings = RootStore::read_settings_from(directory)
                        .map_err(|e| prior(e, &mut previous))?;
                    attempts = attempts.checked_add(1).ok_or(RootStoreError::Refused(
                        "agent-storage-attempts-representation",
                    ))?;
                    let error = RootStoreError::Storage(error);
                    if settings
                        .values
                        .storage_busy_retries
                        .is_some_and(|max| attempts > max.0 as u64)
                    {
                        return Err(exhausted(error, attempts, settings.revision));
                    }
                    previous = Some(error);
                    std::thread::sleep(Duration::from_millis(
                        settings.values.storage_busy_retry_delay_ms.0 as u64,
                    ));
                    continue;
                }
                Err(error) => return Err(prior(RootStoreError::Storage(error), &mut previous)),
            }),
        };
        let prepared = (|| {
            let settings =
                RootStore::read_settings_from(directory).map_err(|e| prior(e, &mut previous))?;
            if attempts > 0
                && settings
                    .values
                    .storage_busy_retries
                    .is_some_and(|max| attempts > max.0 as u64)
            {
                let cause = previous
                    .take()
                    .ok_or(RootStoreError::Refused("agent-storage-retry-state"))?;
                return Err(exhausted(cause, attempts, settings.revision));
            }
            connection(context)
                .busy_timeout(Duration::from_millis(
                    settings.values.storage_busy_timeout_ms.0 as u64,
                ))
                .map_err(RootStoreError::Sql)?;
            attempts = attempts.checked_add(1).ok_or(RootStoreError::Refused(
                "agent-storage-attempts-representation",
            ))?;
            let result = operation(context, &settings);
            Ok((settings, result))
        })();
        let (settings, result) = match prepared {
            Ok(value) => value,
            Err(error) => {
                return finished(
                    Err(error),
                    reservation.map_or(Ok(()), |r| r.close().map_err(RootStoreError::Storage)),
                )
            }
        };
        let cleanup = reservation.map_or(Ok(()), |r| r.close().map_err(RootStoreError::Storage));
        let error = match (result, cleanup) {
            (Ok(value), Ok(())) => return Ok(value),
            (Ok(_), Err(primary)) if recorded => {
                return Err(RootStoreError::LifecycleRecorded {
                    primary: Box::new(primary),
                })
            }
            (Err(error), Ok(())) => error,
            (result, cleanup) => return finished(result, cleanup),
        };
        if !connection(context).is_autocommit() || !retryable(&error) {
            return Err(error);
        }
        if settings
            .values
            .storage_busy_retries
            .is_some_and(|max| attempts > max.0 as u64)
        {
            return Err(exhausted(error, attempts, settings.revision));
        }
        previous = Some(error);
        std::thread::sleep(Duration::from_millis(
            settings.values.storage_busy_retry_delay_ms.0 as u64,
        ));
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::{
        AgentLaunchError, AgentStoreError, EngineSettingsChange, EngineSettingsValues,
        SystemEnvironment,
    };
    /// Exercise the shared loop through both borrowing APIs. This fixture switch
    /// is test data, not a second operator policy or execution mode.
    fn fixture_execute<T>(
        borrowed: bool,
        db: &mut Connection,
        directory: &LocalDirectory,
        recorded: bool,
        mut operation: impl FnMut(&Connection, &EngineSettingsDocument) -> Result<T, RootStoreError>,
    ) -> Result<T, RootStoreError> {
        if borrowed {
            execute_borrowed(db, directory, recorded, operation)
        } else {
            execute(db, directory, recorded, |db, settings| {
                operation(db, settings)
            })
        }
    }
    /// Typed SQLite fixture, not a substring classification or actual OS fault.
    fn busy() -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY), None)
    }
    /// Compound/uncertain errors never become implicit retries by traversing a
    /// primary BUSY source. Direct safe storage is the explicit positive control.
    #[test]
    fn retry_classification_rejects_uncertain_and_compound_failures() {
        assert!(retryable(&RootStoreError::Agent(AgentStoreError::Sql(
            busy()
        ))));
        assert!(retryable(&RootStoreError::Launch(Box::new(
            AgentLaunchError::caused("agent-launch-storage", busy())
        ))));
        for rolled_back in [false, true] {
            assert!(!retryable(&RootStoreError::Agent(
                AgentStoreError::Commit {
                    cause: busy(),
                    rolled_back,
                    cleanup: None
                }
            )));
            assert!(!retryable(&RootStoreError::Launch(Box::new(
                AgentLaunchError::Commit {
                    cause: Box::new(AgentLaunchError::caused("agent-launch-storage", busy())),
                    rollback_confirmed: rolled_back,
                    cleanup: None
                }
            ))));
        }
        assert!(!retryable(&RootStoreError::Agent(
            AgentStoreError::Finalize {
                primary: Box::new(AgentStoreError::Sql(busy())),
                cleanup: busy()
            }
        )));
        assert!(!retryable(&RootStoreError::Launch(Box::new(
            AgentLaunchError::Cleanup {
                primary: Box::new(AgentLaunchError::caused("agent-launch-storage", busy())),
                cleanup: Box::new(AgentLaunchError::Refused("fixture-cleanup"))
            }
        ))));
        assert!(!retryable(&RootStoreError::Launch(Box::new(
            AgentLaunchError::caused("agent-launch-work-invalid", busy())
        ))));
        assert!(!retryable(&RootStoreError::Sql(busy())));
    }
    /// Explicit saved unlimited retries can recover without a hidden cap; valid
    /// externally replaced fixture policy is reread before another attempt. This
    /// is a deterministic private-file fault, not proof of concurrent public CAS.
    #[test]
    fn unlimited_policy_and_fresh_tightening_are_observed_without_deadlines() {
        use std::{fs, os::unix::fs::DirBuilderExt};
        for (tighten, borrowed) in [(false, false), (true, false), (false, true), (true, true)] {
            let root = std::env::temp_dir().join(crate::generate_identifier("storage77-").unwrap());
            fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            let store = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
            let doc = store.settings().unwrap();
            let mut values = EngineSettingsValues::installation_defaults().unwrap();
            values.storage_busy_timeout_ms = SettingCount(0);
            values.storage_busy_retries = None;
            values.storage_busy_retry_delay_ms = SettingCount(0);
            let mut change = EngineSettingsChange {
                schema_version: doc.schema_version,
                event_id: "unlimited-storage".into(),
                expected_revision: doc.revision,
                values,
                reason: "Explicit fixture unlimited storage retry".into(),
                preview_token: String::new(),
            };
            change.preview_token = store.preview_settings(&change).unwrap().preview_token;
            store
                .apply_settings(&change, &mut SystemEnvironment)
                .unwrap();
            let saved = store.settings().unwrap();
            store.close().unwrap();
            let directory = LocalDirectory::open(&root.join(".swarm")).unwrap();
            let mut db = Connection::open_in_memory().unwrap();
            db.execute_batch("CREATE TABLE witness(value INTEGER); INSERT INTO witness VALUES(1)")
                .unwrap();
            let mut calls = 0;
            let outcome = fixture_execute(borrowed, &mut db, &directory, true, |db, settings| {
                calls += 1;
                if calls == 1 {
                    assert_eq!(settings.values.storage_busy_retries, None);
                    if tighten {
                        let mut values = saved.values.clone();
                        values.storage_busy_retries = Some(SettingCount(0));
                        let mut change = EngineSettingsChange {
                            schema_version: saved.schema_version,
                            event_id: "tighten-fixture".into(),
                            expected_revision: saved.revision,
                            values,
                            reason: "Private valid replacement fault".into(),
                            preview_token: String::new(),
                        };
                        change.preview_token = saved.preview(&change).unwrap().preview_token;
                        let (next, _) = saved
                            .apply(&change, "fixture".into(), "2026-10-19T23:00:00Z".into())
                            .unwrap();
                        directory
                            .publish("engine-settings.json", &serde_json::to_vec(&next).unwrap())
                            .unwrap();
                    }
                    return Err(RootStoreError::Agent(AgentStoreError::Sql(busy())));
                }
                db.execute("INSERT INTO witness VALUES(2)", []).unwrap();
                Ok(())
            });
            assert_eq!(
                db.pragma_query_value::<i64, _>(None, "busy_timeout", |r| r.get(0))
                    .unwrap(),
                0
            );
            if tighten {
                assert_eq!(calls, 1);
                match outcome.err().unwrap() {
                    RootStoreError::AgentStorageBusy {
                        attempts,
                        settings_revision,
                        ..
                    } => {
                        assert_eq!(attempts, 1);
                        assert_eq!(settings_revision.0, 3);
                    }
                    _ => panic!("tightened policy was not observed"),
                };
            } else {
                outcome.unwrap();
                assert_eq!(calls, 2);
            }
            assert_eq!(
                db.query_row("SELECT count(*) FROM witness", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                if tighten { 1 } else { 2 }
            );
            db.close().unwrap();
            directory.close().unwrap();
            fs::remove_dir_all(root).unwrap();
        }
    }

    /// Use the actual values generation, never relabel current settings as an
    /// older schema. Valid callback precedes real exclusive-lock injection. Zero/one saved
    /// retries mean one/two acquisition attempts, no callback while blocked;
    /// checked lock release recovers without sleeps or a global test watchdog.
    /// The public settings reader must use that same saved budget and recover
    /// without changing settings/history; a raw immediate EAGAIN is insufficient.
    #[test]
    fn real_settings_lock_budget_refuses_without_callback_then_recovers() {
        use std::{fs, os::unix::fs::DirBuilderExt};
        for (retries, borrowed) in [(0, false), (1, false), (0, true), (1, true)] {
            let root = std::env::temp_dir().join(crate::generate_identifier("lock78-").unwrap());
            fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            let store = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
            let current = store.settings().unwrap();
            let mut values = current.values;
            values.storage_busy_retries = Some(SettingCount(retries));
            values.storage_busy_retry_delay_ms = SettingCount(0);
            let mut request = EngineSettingsChange {
                schema_version: values.schema_version(),
                event_id: "lock-policy".into(),
                expected_revision: current.revision,
                values,
                reason: "Actual private lock refusal fixture".into(),
                preview_token: String::new(),
            };
            request.preview_token = store.preview_settings(&request).unwrap().preview_token;
            store
                .apply_settings(&request, &mut SystemEnvironment)
                .unwrap();
            store.close().unwrap();
            let directory = LocalDirectory::open(&root.join(".swarm")).unwrap();
            let mut db = Connection::open_in_memory().unwrap();
            let mut calls = 0;
            fixture_execute(borrowed, &mut db, &directory, false, |_, _| {
                calls += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(calls, 1);
            let reader = RootStore::open(&root, false).unwrap();
            let settings_witness = reader.settings().unwrap();
            let lock = directory.reserve(true, false).unwrap();
            calls = 0;
            let result = fixture_execute(borrowed, &mut db, &directory, false, |_, _| {
                calls += 1;
                Ok(())
            });
            match result.unwrap_err() {
                RootStoreError::AgentStorageBusy {
                    cause, attempts, ..
                } => {
                    assert_eq!(attempts, retries as u64 + 1);
                    assert!(matches!(*cause,RootStoreError::Storage(ref e)if lock_busy(e)));
                }
                e => panic!("wrong refusal: {e}"),
            };
            assert_eq!(calls, 0);
            match reader.settings().unwrap_err() {
                RootStoreError::AgentStorageBusy { cause, attempts, .. } => {
                    assert_eq!(attempts, retries as u64 + 1);
                    assert!(matches!(*cause, RootStoreError::Storage(ref e) if lock_busy(e)));
                }
                e => panic!("public settings reader ignored saved lock policy: {e}"),
            }
            lock.close().unwrap();
            fixture_execute(borrowed, &mut db, &directory, false, |_, _| {
                calls += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(calls, 1);
            assert_eq!(reader.settings().unwrap(), settings_witness);
            reader.close().unwrap();
            db.close().unwrap();
            directory.close().unwrap();
            fs::remove_dir_all(root).unwrap();
        }
    }
}
