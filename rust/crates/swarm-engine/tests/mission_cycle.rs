#![cfg(target_os = "linux")]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCount, SystemEnvironment};

/// Owned API-created fixture. Preserve failed roots; successful tests close all
/// handles explicitly before removing their own directory. No live DB/provider.
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> (Self, RootStore, String) {
        let path = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("swarm-cycle184-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut store = RootStore::initialize(&path, &mut SystemEnvironment).unwrap();
        let doc = store.settings().unwrap();
        let mut values = doc.values.clone();
        // Deliberately short persisted test cadence; no global test deadline.
        values.mission_supervision_poll_interval_ms = Some(SettingCount(5));
        let mut change = EngineSettingsChange {
            schema_version: doc.schema_version,
            event_id: "fixture-cadence".into(),
            expected_revision: doc.revision,
            values,
            reason: "Observe real heartbeat transitions in isolated test".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store
            .apply_settings(&change, &mut SystemEnvironment)
            .unwrap();
        let request = serde_json::json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Owned cycle","objective":"Real lease maintenance","scope":"Isolated","criteria":["Keep known outcomes"]});
        let work = store
            .execute_external(
                "",
                "work.create",
                &serde_json::to_vec(&request).unwrap(),
                &mut SystemEnvironment,
            )
            .unwrap()
            .result
            .work
            .id;
        (Self(path), store, work)
    }
    /// Read actual persisted observation; never write positive provenance.
    fn heartbeat(&self) -> String {
        let db = rusqlite::Connection::open(self.0.join(".swarm/state.db")).unwrap();
        let value = db
            .query_row("SELECT heartbeat_at FROM mission_supervision", [], |row| {
                row.get(0)
            })
            .unwrap();
        db.close().unwrap();
        value
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

/// Wait for observed transitions, not an arbitrary sleep, while the operation is
/// still active. The driver must renew from another descriptor-bound connection.
#[test]
fn cycle_renews_during_operation_and_returns_exact_result() {
    let (fixture, mut store, work) = Fixture::new();
    let lease = store
        .claim_mission_supervision(
            &work,
            "cycle-test",
            &swarm_engine::system_timestamp().unwrap(),
        )
        .unwrap();
    let before = store.read_work(&work).unwrap();
    let result = store
        .run_mission_cycle(&lease, |store, _, monitor| {
            let first = fixture.heartbeat();
            let mut seen = first.clone();
            let mut transitions = 0;
            while transitions < 2 {
                assert!(monitor.is_running(), "heartbeat exited before witness");
                let current = fixture.heartbeat();
                if current != seen {
                    transitions += 1;
                    seen = current;
                }
                std::thread::yield_now();
            }
            assert_ne!(seen, first);
            store.read_work(&work)
        })
        .unwrap();
    assert_eq!(result, before);
    lease.close().unwrap();
    store.close().unwrap();
}

/// A real callback refusal remains an error, is journalled, and a separately
/// requested healthy cycle recovers without replaying the failed callback.
#[test]
fn cycle_retains_action_error_and_explicit_next_cycle_recovers() {
    let (_fixture, mut store, work) = Fixture::new();
    let lease = store
        .claim_mission_supervision(
            &work,
            "cycle-test",
            &swarm_engine::system_timestamp().unwrap(),
        )
        .unwrap();
    let failure = store
        .run_mission_cycle(&lease, |store, _, _| store.read_work("missing-work"))
        .unwrap_err();
    assert!(matches!(failure.action, Some(Err(_))));
    assert!(failure.supervision.is_none() && failure.journal.is_none());
    let result = store
        .run_mission_cycle(&lease, |store, _, _| store.read_work(&work))
        .unwrap();
    assert_eq!(result.id, work);
    lease.close().unwrap();
    store.close().unwrap();
}

/// A valid witness precedes negative SQL injection on this already opened fixture.
/// Heartbeat failure and failed journalling cannot erase the known callback value.
#[test]
fn cycle_retains_completed_value_and_both_heartbeat_and_journal_errors() {
    let (fixture, mut store, work) = Fixture::new();
    let lease = store
        .claim_mission_supervision(
            &work,
            "cycle-test",
            &swarm_engine::system_timestamp().unwrap(),
        )
        .unwrap();
    store
        .run_mission_cycle(&lease, |store, _, _| store.read_work(&work))
        .unwrap();
    let db = rusqlite::Connection::open(fixture.0.join(".swarm/state.db")).unwrap();
    let before = store.read_work(&work).unwrap();
    let failure = store.run_mission_cycle(&lease, |store, _, monitor| {
        db.execute_batch("CREATE TRIGGER owned_heartbeat_fault BEFORE UPDATE ON mission_supervision BEGIN SELECT RAISE(ABORT, 'owned heartbeat injection'); END;").unwrap();
        while monitor.is_running() { std::thread::yield_now(); }
        store.read_work(&work)
    }).unwrap_err();
    assert_eq!(failure.action.as_ref().unwrap().as_ref().unwrap(), &before);
    assert!(failure.supervision.is_some() && failure.journal.is_some());
    let report = failure.diagnostics();
    assert!(report.action_started && report.action_succeeded);
    assert!(report.causes.iter().any(|c| c.phase == "supervision" && c.sqlite_code.is_some()));
    assert!(report.causes.iter().any(|c| c.phase == "journal" && c.sqlite_code.is_some()));
    assert!(!report.causes.iter().any(|c| c.phase == "action" || c.recorded_work.is_some()));
    assert!(!serde_json::to_string(&report).unwrap().contains("owned heartbeat injection"));
    db.execute_batch("DROP TRIGGER owned_heartbeat_fault;")
        .unwrap();
    db.close().unwrap();
    assert_eq!(
        store
            .run_mission_cycle(&lease, |store, _, _| store.read_work(&work))
            .unwrap(),
        before
    );
    lease.close().unwrap();
    store.close().unwrap();
}

/// A genuine lease from another Root must refuse before any callback execution.
#[test]
fn cycle_wrong_root_never_starts_callback() {
    let (_first, first, work) = Fixture::new();
    let (_second, mut second, _) = Fixture::new();
    let lease = first
        .claim_mission_supervision(
            &work,
            "cycle-test",
            &swarm_engine::system_timestamp().unwrap(),
        )
        .unwrap();
    let mut called = false;
    let failure = second
        .run_mission_cycle(&lease, |_, _, _| {
            called = true;
            Ok(())
        })
        .unwrap_err();
    assert!(!called && failure.action.is_none());
    assert_eq!(
        failure.supervision.unwrap().code(),
        "mission-lease-root-mismatch"
    );
    lease.close().unwrap();
    first.close().unwrap();
    second.close().unwrap();
}

/// Public lease revocation while the callback is active preserves its known
/// value, denies another cycle before the callback, then a new claim recovers.
#[test]
fn cycle_revocation_retains_result_and_requires_new_claim() {
    let (_fixture, mut store, work) = Fixture::new();
    let lease = store
        .claim_mission_supervision(
            &work,
            "cycle-test",
            &swarm_engine::system_timestamp().unwrap(),
        )
        .unwrap();
    store
        .run_mission_cycle(&lease, |store, _, _| store.read_work(&work))
        .unwrap();
    let failure = store
        .run_mission_cycle(&lease, |store, lease, monitor| {
            store
                .update_mission_supervision(lease, &swarm_engine::system_timestamp().unwrap(), true)
                .unwrap();
            while monitor.is_running() {
                std::thread::yield_now();
            }
            store.read_work(&work)
        })
        .unwrap_err();
    assert_eq!(failure.action.as_ref().unwrap().as_ref().unwrap().id, work);
    assert_eq!(failure.supervision.unwrap().code(), "mission-lease-lost");
    assert_eq!(failure.journal.unwrap().code(), "mission-lease-lost");
    let mut called = false;
    let failure = store
        .run_mission_cycle(&lease, |_, _, _| {
            called = true;
            Ok(())
        })
        .unwrap_err();
    assert!(!called && failure.action.is_none());
    let replacement = store
        .claim_mission_supervision(
            &work,
            "cycle-recovery",
            &swarm_engine::system_timestamp().unwrap(),
        )
        .unwrap();
    assert!(replacement.generation() > lease.generation());
    assert_eq!(
        store
            .run_mission_cycle(&replacement, |store, _, _| store.read_work(&work))
            .unwrap()
            .id,
        work
    );
    replacement.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}

/// Unwinding a callback must disconnect and join the heartbeat before propagating
/// the original panic; the same Root and lease remain usable after caller recovery.
#[test]
fn cycle_callback_panic_stops_heartbeat_and_preserves_payload() {
    let (_fixture, mut store, work) = Fixture::new();
    let lease = store
        .claim_mission_supervision(
            &work,
            "cycle-test",
            &swarm_engine::system_timestamp().unwrap(),
        )
        .unwrap();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        store.run_mission_cycle(
            &lease,
            |_, _, _| -> Result<(), swarm_engine::RootStoreError> {
                panic!("owned cycle panic fixture");
            },
        )
    }))
    .unwrap_err();
    assert_eq!(
        failure.downcast_ref::<&str>(),
        Some(&"owned cycle panic fixture")
    );
    assert_eq!(
        store
            .run_mission_cycle(&lease, |store, _, _| store.read_work(&work))
            .unwrap()
            .id,
        work
    );
    lease.close().unwrap();
    store.close().unwrap();
}

/// A writable witness precedes reopening the same datastore read-only. An owned
/// lease must not let the helper upgrade that explicitly selected Root mode.
#[test]
fn cycle_read_only_root_never_starts_callback_or_renews() {
    let (fixture, mut store, work) = Fixture::new();
    let lease = store
        .claim_mission_supervision(
            &work,
            "cycle-test",
            &swarm_engine::system_timestamp().unwrap(),
        )
        .unwrap();
    store
        .run_mission_cycle(&lease, |store, _, _| store.read_work(&work))
        .unwrap();
    let mut readonly = RootStore::open(&fixture.0, false).unwrap();
    let before = fixture.heartbeat();
    let mut called = false;
    let result = readonly.run_mission_cycle(&lease, |store, _, _| {
        called = true;
        store.read_work(&work)
    });
    assert!(
        result.is_err() && !called,
        "read-only cycle started its callback"
    );
    assert_eq!(fixture.heartbeat(), before);
    let failure = result.unwrap_err();
    assert!(failure.action.is_none());
    assert_eq!(
        failure.supervision.unwrap().code(),
        "mission-root-read-only"
    );
    readonly.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}

/// Typed projection keeps every phase and independent cleanup cause, traversing
/// the established custom-IO/mission/Root matrix without publishing private text.
/// These are diagnostic fixtures, not a claim that a transaction really committed.
#[test]
fn cycle_diagnostics_keep_all_phases_wrappers_and_recorded_revisions() {
    use swarm_engine::{MissionCycleFailure, MissionSupervisionError as M, RootStoreError as R};
    use std::io;
    let failure = MissionCycleFailure::<()> {
        action: Some(Err(R::Cleanup {
            primary: Some(Box::new(R::WorkRecorded {
                primary: Box::new(R::Refused("action-refused")),
                work: "known-work".into(), revision: 7,
            })),
            cleanup: Box::new(R::WorkRecorded {
                primary: Box::new(R::Refused("cleanup-refused")),
                work: "known-work".into(), revision: 8,
            }),
        })),
        supervision: Some(M::Cause { code: "supervision-wrapper",
            cause: Box::new(io::Error::other(M::Root(Box::new(R::WorkRecorded {
                primary: Box::new(R::Refused("peer-refused")),
                work: "peer-work".into(), revision: 3,
            })))),
        }),
        journal: Some(M::Cleanup {
            primary: Box::new(M::Cause { code: "journal-io",
                cause: Box::new(io::Error::other("PRIVATE_CYCLE_MUST_NOT_LEAK")),
            }),
            cleanup: Box::new(M::Cause { code: "journal-close",
                cause: Box::new(rustix::io::Errno::AGAIN),
            }),
        }),
    };
    let report = failure.diagnostics();
    assert!(report.action_started && !report.action_succeeded);
    let known: Vec<_> = report.causes.iter().filter_map(|c|
        c.recorded_work.as_ref().map(|w| (c.phase, c.path.clone(), w.clone()))).collect();
    assert_eq!(known.len(), 3);
    assert_eq!(known[0].2, ("known-work".into(), 7));
    assert_eq!(known[1].2, ("known-work".into(), 8));
    assert!(known[0].1.contains(&"primary".into()));
    assert!(known[1].1.contains(&"cleanup".into()));
    assert_eq!(known[2].0, "supervision");
    assert_eq!(known[2].2, ("peer-work".into(), 3));
    assert!(report.causes.iter().any(|c| c.phase == "journal"
        && c.path.contains(&"cleanup".into())
        && c.io_errno == Some(rustix::io::Errno::AGAIN.raw_os_error())));
    let raw = serde_json::to_string(&report).unwrap();
    assert!(!raw.contains("PRIVATE_CYCLE_MUST_NOT_LEAK"));
    assert!(raw.contains("io-kind:Other"));
    assert!(failure.action.as_ref().unwrap().is_err());
    assert_eq!(report, failure.diagnostics());
}

/// Not-started and successful-value states stay distinct without fabricating a
/// Work identity from T, requesting serialization of T, or retrying any callback.
#[test]
fn cycle_diagnostics_do_not_infer_work_from_success_or_absence() {
    use swarm_engine::{MissionCycleFailure, MissionSupervisionError as M};
    struct PrivateValue;
    let before = MissionCycleFailure::<PrivateValue> {
        action: None, supervision: Some(M::Refused("not-started")), journal: None,
    };
    let report = before.diagnostics();
    assert!(!report.action_started && !report.action_succeeded);
    assert_eq!(report.causes.len(), 1);
    assert_eq!(report.causes[0].phase, "supervision");
    let late = MissionCycleFailure {
        action: Some(Ok(PrivateValue)), supervision: None,
        journal: Some(M::Refused("late-journal")),
    };
    let report = late.diagnostics();
    assert!(report.action_started && report.action_succeeded);
    assert_eq!(report.causes.len(), 1);
    assert_eq!(report.causes[0].phase, "journal");
    assert!(report.causes.iter().all(|c| c.recorded_work.is_none()));
    assert!(late.action.unwrap().is_ok());
}
