#![cfg(target_os = "linux")]
use std::{error::Error, fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{EngineSettingsValues, MissionLeaseClock, RootStore, SystemEnvironment};
/// Private real Root fixture, removed only on success; failures preserve originals.
struct Fixture(PathBuf);
impl Fixture {
    /// Initialize through the supported Root API; no live DB or provider is used.
    fn new() -> (Self, RootStore, String) {
        let path = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("swarm-mission100-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut env = SystemEnvironment;
        let mut store = RootStore::initialize(&path, &mut env).unwrap();
        let raw=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Mission fixture","objective":"Lease guard","scope":"Isolated","criteria":["Ownership"]})).unwrap();
        let id = store
            .execute_external("", "work.create", &raw, &mut env)
            .unwrap()
            .result
            .work
            .id;
        (Self(path), store, id)
    }
    /// Observe every lease/epoch field for refusal-without-mutation assertions.
    fn state(&self) -> (Vec<String>, Vec<String>) {
        let db = rusqlite::Connection::open(self.0.join(".swarm/state.db")).unwrap();
        let lease=db.query_row("SELECT conductor_id,source,started_at,heartbeat_at,reconciled_at,last_check_at,last_error,next_check_at,stopped_at FROM mission_supervision",[],|r| (0..9).map(|i|r.get(i)).collect()).unwrap();
        let epoch=db.query_row("SELECT CAST(generation AS TEXT),conductor_id,CAST(pid AS TEXT),process_stamp,host,CAST(poll_ms AS TEXT),CAST(stale_ms AS TEXT),CAST(future_ms AS TEXT) FROM native_mission_lease_epochs",[],|r| (0..8).map(|i|r.get(i)).collect()).unwrap();
        db.close().unwrap();
        (lease, epoch)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Go inclusive expiry, nanoseconds, timezone offsets, clock refusal and exact
/// millisecond settings. No wall-clock sleeps or arbitrary global watchdog.
#[test]
fn mission_clock_boundaries_invalid_and_large_values() {
    let mut policy = EngineSettingsValues::installation_defaults()
        .unwrap()
        .mission_supervision_policy()
        .unwrap();
    let heartbeat = "2026-10-20T00:00:00Z";
    assert_eq!(
        policy.observe(heartbeat, "2026-10-20T00:00:30Z").unwrap(),
        MissionLeaseClock::Present
    );
    assert_eq!(
        policy
            .observe(heartbeat, "2026-10-20T00:00:30.000000001Z")
            .unwrap(),
        MissionLeaseClock::Stale
    );
    assert_eq!(
        policy.observe(heartbeat, "2026-10-19T23:59:58Z").unwrap(),
        MissionLeaseClock::Present
    );
    assert_eq!(
        policy
            .observe(heartbeat, "2026-10-19T23:59:57.999999999Z")
            .unwrap(),
        MissionLeaseClock::Future
    );
    assert_eq!(
        policy
            .observe("2026-10-20T01:00:00+01:00", heartbeat)
            .unwrap(),
        MissionLeaseClock::Present
    );
    let err = policy.observe("malformed PRIVATE", heartbeat).unwrap_err();
    assert_eq!(err.code(), "mission-clock-invalid");
    assert!(err.source().is_some());
    assert!(!err.to_string().contains("PRIVATE"));
    policy.future_tolerance_ms.0 = 0;
    assert_eq!(
        policy
            .observe(heartbeat, "2026-10-19T23:59:59.999999999Z")
            .unwrap(),
        MissionLeaseClock::Future
    );
    policy.stale_after_ms.0 = i64::MAX;
    policy.future_tolerance_ms.0 = i64::MAX;
    policy.validate().unwrap();
    assert_eq!(
        policy.observe(heartbeat, "9999-12-31T23:59:59Z").unwrap(),
        MissionLeaseClock::Present
    );
}
/// A valid handle renews; the same label cannot reset it. Stopped and replaced
/// generations never revive; effects survive a checked close/reopen.
#[test]
fn mission_lease_same_label_stop_reclaim_and_persistence() {
    let (f, store, id) = Fixture::new();
    let first = store
        .claim_mission_supervision(&id, "conductor", "2026-10-20T00:00:00Z")
        .unwrap();
    assert_eq!(first.generation(), 1);
    assert_eq!(first.settings_revision().0, 1);
    store
        .update_mission_supervision(&first, "2026-10-20T00:00:01Z", false)
        .unwrap();
    let state = f.state();
    assert_eq!(state.0[3], "2026-10-20T00:00:01Z");
    assert_eq!(state.0[7], "2026-10-20T00:00:03Z");
    assert_eq!(
        store
            .claim_mission_supervision(&id, "conductor", "2026-10-20T00:00:02Z")
            .unwrap_err()
            .code(),
        "mission-lease-already-held"
    );
    assert_eq!(state, f.state());
    store
        .update_mission_supervision(&first, "2026-10-20T00:00:02Z", true)
        .unwrap();
    let stopped = f.state();
    assert_eq!(stopped.0[8], "2026-10-20T00:00:02Z");
    assert!(stopped.0[7].is_empty());
    assert!(store
        .update_mission_supervision(&first, "2026-10-20T00:00:03Z", false)
        .is_err());
    assert_eq!(stopped, f.state());
    let second = store
        .claim_mission_supervision(&id, "conductor", "2026-10-20T00:00:03Z")
        .unwrap();
    assert_eq!(second.generation(), 2);
    let replacement = f.state();
    assert!(store
        .update_mission_supervision(&first, "2026-10-20T00:00:04Z", true)
        .is_err());
    assert_eq!(replacement, f.state());
    first.close().unwrap();
    store.close().unwrap();
    let reopened = RootStore::open(&f.0, true).unwrap();
    reopened
        .update_mission_supervision(&second, "2026-10-20T00:00:04Z", false)
        .unwrap();
    assert_eq!(f.state().0[3], "2026-10-20T00:00:04Z");
    second.close().unwrap();
    reopened.close().unwrap();
}
/// Expired owners cannot renew; replacement uses persisted old policy. Invalid or
/// excessive future clocks never masquerade as permission to steal a lease.
#[test]
fn mission_lease_expiry_future_corruption_and_recovery() {
    let (f, store, id) = Fixture::new();
    let first = store
        .claim_mission_supervision(&id, "one", "2026-10-20T00:00:00Z")
        .unwrap();
    let valid = f.state();
    assert_eq!(
        store
            .update_mission_supervision(&first, "2026-10-20T00:00:31Z", false)
            .unwrap_err()
            .code(),
        "mission-lease-not-present"
    );
    assert_eq!(valid, f.state());
    assert_eq!(
        store
            .claim_mission_supervision(&id, "two", "2026-10-19T23:59:57Z")
            .unwrap_err()
            .code(),
        "mission-lease-clock-future"
    );
    assert_eq!(valid, f.state());
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute("UPDATE mission_supervision SET heartbeat_at='invalid'", [])
        .unwrap();
    let corrupt = f.state();
    assert_eq!(
        store
            .claim_mission_supervision(&id, "two", "2026-10-20T00:00:31Z")
            .unwrap_err()
            .code(),
        "mission-clock-invalid"
    );
    assert_eq!(corrupt, f.state());
    db.execute(
        "UPDATE mission_supervision SET heartbeat_at='2026-10-20T00:00:00Z'",
        [],
    )
    .unwrap();
    db.close().unwrap();
    let second = store
        .claim_mission_supervision(&id, "two", "2026-10-20T00:00:30.000000001Z")
        .unwrap();
    assert_eq!(second.generation(), 2);
    let state = f.state();
    assert!(store
        .update_mission_supervision(&first, "2026-10-20T00:00:31Z", false)
        .is_err());
    assert_eq!(state, f.state());
    first.close().unwrap();
    second.close().unwrap();
    store.close().unwrap();
}
/// Root bindings reject cross-datastore handles before writes; read-only stores
/// refuse. A real SQL abort rolls the epoch and lease back, retains the cause,
/// and recovery succeeds after removing only the injected fixture trigger.
#[test]
fn mission_lease_root_readonly_sql_refusal_and_recovery() {
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "one", "2026-10-20T00:00:00Z")
        .unwrap();
    let (other, other_store, _) = Fixture::new();
    assert_eq!(
        other_store
            .update_mission_supervision(&lease, "2026-10-20T00:00:01Z", false)
            .unwrap_err()
            .code(),
        "mission-lease-root-mismatch"
    );
    let readonly = RootStore::open(&f.0, false).unwrap();
    let state = f.state();
    assert_eq!(
        readonly
            .update_mission_supervision(&lease, "2026-10-20T00:00:01Z", false)
            .unwrap_err()
            .code(),
        "mission-root-read-only"
    );
    assert_eq!(state, f.state());
    readonly.close().unwrap();
    store
        .update_mission_supervision(&lease, "2026-10-20T00:00:01Z", false)
        .unwrap();
    let valid = f.state();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER fixture_abort BEFORE UPDATE ON mission_supervision BEGIN SELECT RAISE(ABORT,'PRIVATE fixture'); END;").unwrap();
    let error = store
        .update_mission_supervision(&lease, "2026-10-20T00:00:02Z", false)
        .unwrap_err();
    assert_eq!(error.code(), "mission-storage-failed");
    assert!(error
        .source()
        .unwrap()
        .downcast_ref::<rusqlite::Error>()
        .is_some());
    assert!(!error.to_string().contains("PRIVATE"));
    assert_eq!(valid, f.state());
    db.execute_batch("DROP TRIGGER fixture_abort;").unwrap();
    db.close().unwrap();
    store
        .update_mission_supervision(&lease, "2026-10-20T00:00:02Z", false)
        .unwrap();
    assert_eq!(f.state().0[3], "2026-10-20T00:00:02Z");
    lease.close().unwrap();
    store.close().unwrap();
    other_store.close().unwrap();
    drop(other);
}

/// Frozen old policy governs replacement; saving settings does not retroactively
/// expire a conductor. A subsequent claim consumes the new persisted values.
#[test]
fn mission_lease_settings_save_freeze_and_new_claim_consumer() {
    let (f, store, id) = Fixture::new();
    let old = store
        .claim_mission_supervision(&id, "one", "2026-10-20T00:00:00Z")
        .unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.mission_supervision_poll_interval_ms = Some(swarm_engine::SettingCount(4000));
    values.mission_supervision_stale_after_ms = Some(swarm_engine::SettingCount(1000));
    values.mission_supervision_future_tolerance_ms = Some(swarm_engine::SettingCount(0));
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "mission-policy".into(),
        expected_revision: doc.revision,
        values,
        reason: "Fixture new lease policy".into(),
        preview_token: String::new(),
    };
    let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    assert_eq!(old.policy().stale_after_ms.0, 30000);
    assert_eq!(
        store
            .claim_mission_supervision(&id, "two", "2026-10-20T00:00:02Z")
            .unwrap_err()
            .code(),
        "mission-lease-already-held"
    );
    store
        .update_mission_supervision(&old, "2026-10-20T00:00:02Z", false)
        .unwrap();
    assert_eq!(f.state().0[7], "2026-10-20T00:00:04Z");
    store
        .update_mission_supervision(&old, "2026-10-20T00:00:03Z", true)
        .unwrap();
    let new = store
        .claim_mission_supervision(&id, "two", "2026-10-20T00:00:04Z")
        .unwrap();
    assert_eq!(new.settings_revision().0, 2);
    assert_eq!(new.policy().poll_interval_ms.0, 4000);
    assert_eq!(f.state().0[7], "2026-10-20T00:00:08Z");
    let before = f.state();
    assert_eq!(
        store
            .update_mission_supervision(&new, "2026-10-20T00:00:05.000000001Z", false)
            .unwrap_err()
            .code(),
        "mission-lease-not-present"
    );
    assert_eq!(before, f.state());
    new.close().unwrap();
    old.close().unwrap();
    store.close().unwrap();
}
/// Two independently opened Roots compete after a successful stopped witness.
/// Exactly one claims the next epoch; the other observes held, with no sleeps.
#[test]
fn mission_lease_concurrent_claim_has_one_winner() {
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "witness", "2026-10-20T00:00:00Z")
        .unwrap();
    store
        .update_mission_supervision(&lease, "2026-10-20T00:00:01Z", true)
        .unwrap();
    lease.close().unwrap();
    store.close().unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let path = f.0.clone();
        let id = id.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            let root = RootStore::open(&path, true).unwrap();
            barrier.wait();
            let won =
                match root.claim_mission_supervision(&id, "same-label", "2026-10-20T00:00:02Z") {
                    Ok(lease) => {
                        assert_eq!(lease.generation(), 2);
                        lease.close().unwrap();
                        true
                    }
                    Err(e) => {
                        assert_eq!(e.code(), "mission-lease-already-held");
                        false
                    }
                };
            root.close().unwrap();
            won
        }));
    }
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|x| **x).count(), 1);
    assert_eq!(f.state().1[0], "2");
}
/// Real deferred-FK commit refusal rolls back both tables and retains the original
/// SQLite cause. Recovery changes the injected prerequisite; it is not a retry of
/// an unchanged failing command. A valid renewal precedes injection.
#[test]
fn mission_lease_commit_refusal_and_checked_rollback_recovery() {
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "witness", "2026-10-20T00:00:00Z")
        .unwrap();
    store
        .update_mission_supervision(&lease, "2026-10-20T00:00:01Z", false)
        .unwrap();
    let before = f.state();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TABLE fixture_parent(id INTEGER PRIMARY KEY); CREATE TABLE fixture_child(parent INTEGER REFERENCES fixture_parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fixture_commit BEFORE UPDATE ON mission_supervision BEGIN INSERT INTO fixture_child VALUES(1); END;").unwrap();
    let error = store
        .update_mission_supervision(&lease, "2026-10-20T00:00:02Z", false)
        .unwrap_err();
    assert!(matches!(
        error,
        swarm_engine::MissionSupervisionError::Commit {
            rollback_confirmed: true,
            cleanup: None,
            ..
        }
    ));
    assert_eq!(error.code(), "mission-commit-refused");
    assert!(error
        .source()
        .unwrap()
        .downcast_ref::<rusqlite::Error>()
        .is_some());
    assert_eq!(before, f.state());
    assert_eq!(
        db.query_row("SELECT count(*) FROM fixture_child", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    db.execute_batch(
        "DROP TRIGGER fixture_commit; DROP TABLE fixture_child; DROP TABLE fixture_parent;",
    )
    .unwrap();
    db.close().unwrap();
    store
        .update_mission_supervision(&lease, "2026-10-20T00:00:02Z", false)
        .unwrap();
    assert_eq!(f.state().0[3], "2026-10-20T00:00:02Z");
    lease.close().unwrap();
    store.close().unwrap();
}
/// Native epochs survive deletion of the lease row, and exhaustion refuses
/// without wrapping to an earlier generation. Legacy rows never gain defaults.
#[test]
fn mission_lease_epoch_retention_exhaustion_and_legacy_refusal() {
    let (f, store, id) = Fixture::new();
    let first = store
        .claim_mission_supervision(&id, "witness", "2026-10-20T00:00:00Z")
        .unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute("DELETE FROM mission_supervision", []).unwrap();
    let second = store
        .claim_mission_supervision(&id, "replacement", "2026-10-20T00:00:01Z")
        .unwrap();
    assert_eq!(second.generation(), 2);
    let state = f.state();
    assert!(store
        .update_mission_supervision(&first, "2026-10-20T00:00:02Z", false)
        .is_err());
    assert_eq!(state, f.state());
    store
        .update_mission_supervision(&second, "2026-10-20T00:00:02Z", true)
        .unwrap();
    db.execute(
        "UPDATE native_mission_lease_epochs SET generation=?",
        [i64::MAX],
    )
    .unwrap();
    let state = f.state();
    assert_eq!(
        store
            .claim_mission_supervision(&id, "exhausted", "2026-10-20T00:00:03Z")
            .unwrap_err()
            .code(),
        "mission-epoch-exhausted"
    );
    assert_eq!(state, f.state());
    db.execute("DELETE FROM native_mission_lease_epochs", [])
        .unwrap();
    let before: Vec<u8> = fs::read(f.0.join(".swarm/state.db")).unwrap();
    assert_eq!(
        store
            .claim_mission_supervision(&id, "legacy", "2026-10-20T00:00:40Z")
            .unwrap_err()
            .code(),
        "mission-legacy-lease-requires-migration"
    );
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    db.close().unwrap();
    first.close().unwrap();
    second.close().unwrap();
    store.close().unwrap();
}
/// v20 bytes/history remain readable and unchanged until explicit upgrade. Partial,
/// null and invalid v21 groups refuse; the catalogue advertises schema21 support.
#[test]
fn mission_settings_history_absence_partial_null_and_invalid() {
    // Freeze the actual v21 configuration rather than cloning future defaults.
    let defaults = swarm_engine::EngineSettingsDocument::decode(include_bytes!(
        "fixtures/legacy-v21-document.json"
    ))
    .unwrap()
    .values;
    let mut old = serde_json::to_value(&defaults).unwrap();
    let names = [
        "mission_supervision_poll_interval_ms",
        "mission_supervision_stale_after_ms",
        "mission_supervision_future_tolerance_ms",
    ];
    for name in names {
        old.as_object_mut().unwrap().remove(name);
    }
    let old = EngineSettingsValues::decode(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(old.schema_version(), 20);
    assert!(old.mission_supervision_policy().is_err());
    let doc = swarm_engine::EngineSettingsDocument::initial(
        old,
        "fixture".into(),
        "2026-10-20T00:00:00Z".into(),
    )
    .unwrap();
    let bytes = swarm_wire_json::encode(&doc).unwrap();
    assert_eq!(
        swarm_wire_json::encode(&swarm_engine::EngineSettingsDocument::decode(&bytes).unwrap())
            .unwrap(),
        bytes
    );
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: 21,
        event_id: "upgrade".into(),
        expected_revision: doc.revision,
        values: defaults.clone(),
        reason: "Explicit schema21 adoption".into(),
        preview_token: String::new(),
    };
    change.preview_token = doc.preview(&change).unwrap().preview_token;
    let (upgraded, replayed) = doc
        .apply(&change, "fixture".into(), "2026-10-20T00:00:01Z".into())
        .unwrap();
    assert!(!replayed);
    assert_eq!(upgraded.schema_version, 21);
    assert_eq!(upgraded.history[0], doc.history[0]);
    assert_eq!(
        upgraded
            .apply(&change, "ignored".into(), "invalid".into())
            .unwrap(),
        (upgraded.clone(), true)
    );
    for name in names {
        for invalid in [serde_json::Value::Null, serde_json::json!("-1")] {
            let mut value = serde_json::to_value(&defaults).unwrap();
            value[name] = invalid;
            assert!(EngineSettingsValues::decode(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        let mut value = serde_json::to_value(&defaults).unwrap();
        value.as_object_mut().unwrap().remove(name);
        assert!(EngineSettingsValues::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let catalog = swarm_engine::EngineSettingsCatalog::installation().unwrap();
    assert!(catalog.supported_document_versions.contains(&21));
}

/// Valid Work absence is explicit and readonly, while unknown Work is a typed
/// refusal. The reader must not create an epoch schema or publish settings.
#[test]
fn mission_reader_absence_unknown_work_and_readonly_nonmutation() {
    let (f, store, id) = Fixture::new();
    store.close().unwrap();
    let root = RootStore::open(&f.0, false).unwrap();
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    assert!(root
        .read_mission_supervision(&id, "2026-10-20T00:00:00Z")
        .unwrap()
        .is_none());
    let error = root
        .read_mission_supervision("unknown", "2026-10-20T00:00:00Z")
        .unwrap_err();
    assert!(error.source().is_some());
    assert_eq!(error.code(), "mission-root-unavailable");
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        settings
    );
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name='native_mission_lease_epochs'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    db.close().unwrap();
    root.close().unwrap();
}
/// Native facts retain policy/epoch after reopen, without inventing settings
/// provenance. Effective deadlines disappear for stale/future/stopped leases.
#[test]
fn mission_reader_native_clock_deadline_stop_and_private_debug() {
    use swarm_engine::MissionSupervisionState as S;
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "PRIVATE label", "2026-10-20T00:00:00Z")
        .unwrap();
    let before = f.state();
    let first = store
        .read_mission_supervision(&id, "2026-10-20T00:00:02Z")
        .unwrap()
        .unwrap();
    assert_eq!(first.state, S::Present);
    assert_eq!(first.generation, Some(1));
    assert_eq!(first.settings_revision, Some(swarm_engine::SettingCount(1)));
    assert_eq!(first.policy.as_ref().unwrap(), lease.policy());
    assert_eq!(first.late, Some(false));
    assert_eq!(first.next_check_at.as_deref(), Some("2026-10-20T00:00:02Z"));
    assert!(!format!("{first:?}").contains("PRIVATE"));
    let late = store
        .read_mission_supervision(&id, "2026-10-20T00:00:02.000000001Z")
        .unwrap()
        .unwrap();
    assert_eq!(late.late, Some(true));
    for (now, state) in [
        ("2026-10-20T00:00:30.000000001Z", S::Stale),
        ("2026-10-19T23:59:57Z", S::Future),
    ] {
        let row = store.read_mission_supervision(&id, now).unwrap().unwrap();
        assert_eq!(row.state, state);
        assert!(row.next_check_at.is_none());
        assert!(row.late.is_none());
        assert_eq!(row.recorded_next_check_at, "2026-10-20T00:00:02Z");
    }
    assert_eq!(f.state(), before);
    store
        .update_mission_supervision(&lease, "2026-10-20T00:00:03Z", true)
        .unwrap();
    lease.close().unwrap();
    store.close().unwrap();
    let root = RootStore::open(&f.0, false).unwrap();
    let row = root
        .read_mission_supervision(&id, "2026-10-20T00:00:04Z")
        .unwrap()
        .unwrap();
    assert_eq!(row.state, S::Stopped);
    assert_eq!(row.generation, Some(1));
    assert!(row.next_check_at.is_none());
    root.close().unwrap();
}
/// A valid native witness precedes corruption/legacy fixtures. Refusals retain
/// causes and schema errors; repair changes the prerequisite before recovery.
#[test]
fn mission_reader_corruption_schema_and_legacy_do_not_adopt_policy() {
    use swarm_engine::MissionSupervisionState as S;
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "witness", "2026-10-20T00:00:00Z")
        .unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
            .unwrap()
            .unwrap()
            .state,
        S::Present
    );
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute(
        "UPDATE mission_supervision SET next_check_at='PRIVATE invalid'",
        [],
    )
    .unwrap();
    let invalid = f.state();
    let error = store
        .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
        .unwrap_err();
    assert_eq!(error.code(), "mission-clock-invalid");
    assert!(error.source().is_some());
    assert!(!error.to_string().contains("PRIVATE"));
    assert_eq!(invalid, f.state());
    db.execute(
        "UPDATE mission_supervision SET next_check_at='2026-10-20T00:00:02Z'",
        [],
    )
    .unwrap();
    assert!(store
        .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
        .is_ok());
    db.execute_batch("CREATE INDEX fixture_epoch_index ON native_mission_lease_epochs(generation)")
        .unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
            .unwrap_err()
            .code(),
        "mission-epoch-schema-invalid"
    );
    db.execute_batch("DROP INDEX fixture_epoch_index").unwrap();
    assert!(store
        .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
        .is_ok());
    db.execute_batch(
        "DROP TABLE native_mission_claim_receipts; DROP TABLE native_mission_lease_epochs",
    )
    .unwrap();
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let row = store
        .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
        .unwrap()
        .unwrap();
    assert_eq!(row.state, S::LegacyUnverified);
    assert!(
        row.generation.is_none()
            && row.policy.is_none()
            && row.settings_revision.is_none()
            && row.next_check_at.is_none()
            && row.late.is_none()
    );
    assert_eq!(before, fs::read(f.0.join(".swarm/state.db")).unwrap());
    db.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}
/// A real missing-core-table SQL failure never becomes None or empty success.
/// Restoring only the captured fixture table/data proves reader recovery.
#[test]
fn mission_reader_sql_failure_retains_cause_and_recovers() {
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "witness", "2026-10-20T00:00:00Z")
        .unwrap();
    assert!(store
        .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
        .unwrap()
        .is_some());
    let row = f.state().0;
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    let ddl: String = db
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='mission_supervision'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    db.execute_batch("DROP TABLE mission_supervision").unwrap();
    let error = store
        .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
        .unwrap_err();
    assert_eq!(error.code(), "mission-storage-failed");
    assert!(error
        .source()
        .unwrap()
        .downcast_ref::<rusqlite::Error>()
        .is_some());
    db.execute_batch(&ddl).unwrap();
    let values: Vec<String> = std::iter::once(id.clone()).chain(row).collect();
    db.execute(
        "INSERT INTO mission_supervision VALUES(?,?,?,?,?,?,?,?,?,?)",
        rusqlite::params_from_iter(values),
    )
    .unwrap();
    db.close().unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
            .unwrap()
            .unwrap()
            .generation,
        Some(1)
    );
    lease.close().unwrap();
    store.close().unwrap();
}

/// After a valid native witness, a view occupying the reserved epoch table name
/// must refuse as malformed schema, never downgrade ownership to legacy absence.
#[test]
fn mission_reader_epoch_object_kind_refuses_before_query() {
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "witness", "2026-10-20T00:00:00Z")
        .unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
            .unwrap()
            .unwrap()
            .state,
        swarm_engine::MissionSupervisionState::Present
    );
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("DROP TABLE native_mission_lease_epochs; CREATE VIEW native_mission_lease_epochs AS SELECT 1 AS private_marker").unwrap();
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
            .unwrap_err()
            .code(),
        "mission-epoch-schema-invalid"
    );
    assert_eq!(before, fs::read(f.0.join(".swarm/state.db")).unwrap());
    db.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}

/// Controlled environment clock; optional channels expose an observation boundary
/// without sleeps. The budget's actual operator observation can advance time,
/// independently of how many timestamp calls the implementation performs.
struct MissionClock {
    at: String,
    expired: Option<String>,
    operator_observed: bool,
    gate: Option<(std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>)>,
}
impl MissionClock {
    /// Explicit fixture observation only; it authenticates no operator or clock.
    fn fixed(at: String) -> Self {
        Self {
            at,
            expired: None,
            operator_observed: false,
            gate: None,
        }
    }
}
impl swarm_engine::WorkEnvironment for MissionClock {
    fn identifier(&mut self, prefix: &str) -> Result<String, swarm_engine::RuntimeError> {
        swarm_engine::generate_identifier(prefix)
    }
    /// One explicit fixture handshake, then a clock selected by actual operator
    /// observation. Channel closure fails the fixture; there is no timed watchdog.
    fn timestamp(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        if let Some((sent, resume)) = self.gate.take() {
            sent.send(()).unwrap();
            resume.recv().unwrap();
        }
        Ok(if self.operator_observed {
            self.expired.as_ref().unwrap_or(&self.at)
        } else {
            &self.at
        }
        .clone())
    }
    fn operator(&mut self) -> Result<String, swarm_engine::RuntimeError> {
        self.operator_observed = true;
        Ok("fixture-observation".into())
    }
    fn git(
        &mut self,
        root: &std::path::Path,
    ) -> Result<swarm_engine::GitObservation, swarm_engine::RuntimeError> {
        swarm_engine::WorkEnvironment::git(&mut SystemEnvironment, root)
    }
}
/// Advance a fixture's actual creation timestamp, with checked representation.
/// This is a virtual observation, not a real-time test deadline or product policy.
fn mission_at(start: &str, millis: i64) -> String {
    let t = swarm_wire_json::go_rfc3339::parse(start)
        .unwrap()
        .checked_add(time::Duration::milliseconds(millis))
        .unwrap();
    swarm_engine::format_utc_timestamp(t.unix_timestamp(), i64::from(t.nanosecond())).unwrap()
}
/// Build the existing external DTO; revision remains a JSON number as required.
fn mission_title(event: &str, revision: i64, title: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":revision,"title":title})).unwrap()
}
/// Current ownership permits a real Work edit. Stopped/replaced handles cannot
/// edit or replay; current owner can continue without renewing history implicitly.
#[test]
fn mission_fence_current_stop_replacement_and_exact_work_effect() {
    let (f, mut store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store.claim_mission_supervision(&id, "one", &start).unwrap();
    let unchanged_lease = f.state();
    let mut env = MissionClock::fixed(mission_at(&start, 1));
    let raw = mission_title("fenced-witness", 1, "Fenced witness");
    let out = store
        .execute_mission_external(&lease, "work.update", &raw, &mut env)
        .unwrap();
    assert_eq!(out.result.work.title, "Fenced witness");
    assert_eq!(out.result.work.revision, 2);
    assert_eq!(f.state(), unchanged_lease);
    store
        .update_mission_supervision(&lease, &mission_at(&start, 2), true)
        .unwrap();
    let before = swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap();
    let events = store.events(&id).unwrap();
    assert_mission_refused(
        store.execute_mission_external(
            &lease,
            "work.update",
            &mission_title("stopped", 2, "Wrong"),
            &mut env,
        ),
        "mission-lease-lost",
    );
    assert_eq!(
        before,
        swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap()
    );
    assert_eq!(events, store.events(&id).unwrap());
    let next = store
        .claim_mission_supervision(&id, "two", &mission_at(&start, 3))
        .unwrap();
    let state = f.state();
    assert_mission_refused(
        store.execute_mission_external(&lease, "work.update", &raw, &mut env),
        "mission-lease-lost",
    );
    assert_eq!(state, f.state());
    assert_eq!(
        before,
        swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap()
    );
    env.at = mission_at(&start, 4);
    let out = store
        .execute_mission_external(
            &next,
            "work.update",
            &mission_title("next-owner", 2, "Next owner"),
            &mut env,
        )
        .unwrap();
    assert_eq!(out.result.work.revision, 3);
    assert_eq!(out.result.work.title, "Next owner");
    next.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}
/// A different datastore and malformed clock refuse before any durable Work or
/// event changes; a valid witness prevents unrelated setup rejection masking it.
#[test]
fn mission_fence_other_root_and_bad_clock_refuse_without_mutation() {
    let (_f, mut store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store.claim_mission_supervision(&id, "one", &start).unwrap();
    let mut env = MissionClock::fixed(mission_at(&start, 1));
    store
        .execute_mission_external(
            &lease,
            "work.update",
            &mission_title("witness", 1, "Witness"),
            &mut env,
        )
        .unwrap();
    let (_other, mut other, _) = Fixture::new();
    assert_mission_refused(
        other.execute_mission_external(
            &lease,
            "work.update",
            &mission_title("wrong-root", 2, "Wrong"),
            &mut env,
        ),
        "mission-lease-root-mismatch",
    );
    other.close().unwrap();
    let before = swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap();
    env.at = "PRIVATE invalid clock".into();
    let error = match store.execute_mission_external(
        &lease,
        "work.update",
        &mission_title("clock", 2, "Wrong"),
        &mut env,
    ) {
        Err(error) => error,
        Ok(_) => panic!("invalid clock unexpectedly accepted"),
    };
    assert_eq!(error.code(), "mission-clock-invalid");
    assert!(!error.to_string().contains("PRIVATE"));
    assert!(error.source().is_some());
    assert_eq!(
        before,
        swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap()
    );
    lease.close().unwrap();
    store.close().unwrap();
}
/// Budget's actual operator observation advances the virtual clock after a
/// successful witness. Expiry before commit must roll back policy, Work and logs.
#[test]
fn mission_fence_expiry_during_budget_mutation_rolls_back_all_effects() {
    let (f, mut store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store.claim_mission_supervision(&id, "one", &start).unwrap();
    let mut env = MissionClock::fixed(mission_at(&start, 1));
    let request = |event: &str, revision: i64, limit: i64| {
        serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":revision,"budget":{"limit_usd":limit,"reserve_per_launch_usd":1,"estimate_source":"Isolated fixture","reference_date":"2026-10-20"}})).unwrap()
    };
    store
        .execute_mission_external(
            &lease,
            "budget.configure",
            &request("witness", 1, 10),
            &mut env,
        )
        .unwrap();
    let before = swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap();
    let events = store.events(&id).unwrap();
    let state = f.state();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    let budget: Vec<u8> = db
        .query_row("SELECT body FROM budgets WHERE work_id=?", [&id], |r| {
            r.get(0)
        })
        .unwrap();
    let cockpit: i64 = db
        .query_row("SELECT count(*) FROM cockpit_events", [], |r| r.get(0))
        .unwrap();
    env.operator_observed = false;
    env.expired = Some(mission_at(
        &start,
        lease.policy().stale_after_ms.0.checked_add(1).unwrap(),
    ));
    assert_mission_refused(
        store.execute_mission_external(
            &lease,
            "budget.configure",
            &request("expires", 2, 20),
            &mut env,
        ),
        "mission-lease-not-present",
    );
    assert!(env.operator_observed);
    assert_eq!(
        before,
        swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap()
    );
    assert_eq!(events, store.events(&id).unwrap());
    assert_eq!(state, f.state());
    assert_eq!(
        db.query_row("SELECT body FROM budgets WHERE work_id=?", [&id], |r| r
            .get::<_, Vec<u8>>(0))
            .unwrap(),
        budget
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM cockpit_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        cockpit
    );
    db.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}
/// Another Root replaces the lease after time observation but before SQL writer
/// admission. Channel handshakes expose the race without arbitrary timing guards.
#[test]
fn mission_fence_replacement_between_clock_and_writer_refuses() {
    // Preserve command assembler mobility to the native worker thread; the
    // additional borrowed fence must not accidentally remove the Send contract.
    fn require_send<T: Send>() {}
    require_send::<swarm_engine::WorkEngine<'_, SystemEnvironment>>();
    let (f, mut store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store.claim_mission_supervision(&id, "one", &start).unwrap();
    let mut env = MissionClock::fixed(mission_at(&start, 1));
    store
        .execute_mission_external(
            &lease,
            "work.update",
            &mission_title("witness", 1, "Witness"),
            &mut env,
        )
        .unwrap();
    let before = swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap();
    let (arrive_tx, arrive_rx) = std::sync::mpsc::channel();
    let (resume_tx, resume_rx) = std::sync::mpsc::channel();
    let path = f.0.clone();
    let at = mission_at(&start, 4);
    std::thread::scope(|scope| {
        let lease_ref = &lease;
        let child = scope.spawn(move || {
            let mut root = RootStore::open(&path, true).unwrap();
            let mut env = MissionClock::fixed(at);
            env.gate = Some((arrive_tx, resume_rx));
            let outcome = root.execute_mission_external(
                lease_ref,
                "work.update",
                &mission_title("stale-race", 2, "Wrong"),
                &mut env,
            );
            root.close().unwrap();
            outcome.err().map(|error| error.code())
        });
        arrive_rx.recv().unwrap();
        store
            .update_mission_supervision(&lease, &mission_at(&start, 2), true)
            .unwrap();
        let next = store
            .claim_mission_supervision(&id, "two", &mission_at(&start, 3))
            .unwrap();
        resume_tx.send(()).unwrap();
        assert_eq!(child.join().unwrap(), Some("mission-lease-lost"));
        assert_eq!(
            before,
            swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap()
        );
        next.close().unwrap();
    });
    lease.close().unwrap();
    store.close().unwrap();
}

/// Require the intended refusal family without demanding Debug on private success
/// data or accepting an unrelated validation/IO refusal as the tested protection.
fn assert_mission_refused<T>(result: Result<T, swarm_engine::RootStoreError>, expected: &str) {
    match result {
        Err(error) => assert_eq!(error.code(), expected),
        Ok(_) => panic!("operation unexpectedly succeeded"),
    }
}

/// Persisted provenance follows the exact historical claim revision after Admin
/// changes and reopen. A changed receipt must refuse, never adopt current values.
#[test]
fn mission_claim_provenance_roundtrip_and_history_mismatch_refusal() {
    let (f, store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store
        .claim_mission_supervision(&id, "declared-label", &start)
        .unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, &start)
            .unwrap()
            .unwrap()
            .settings_revision,
        Some(swarm_engine::SettingCount(1))
    );
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.mission_supervision_poll_interval_ms = Some(swarm_engine::SettingCount(4000));
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "after-claim".into(),
        expected_revision: doc.revision,
        values,
        reason: "Fixture policy change after claim".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    lease.close().unwrap();
    store.close().unwrap();
    let root = RootStore::open(&f.0, false).unwrap();
    let row = root
        .read_mission_supervision(&id, &mission_at(&start, 1))
        .unwrap()
        .unwrap();
    assert_eq!(row.settings_revision, Some(swarm_engine::SettingCount(1)));
    assert_eq!(row.policy.unwrap().poll_interval_ms.0, 2000);
    assert_eq!(root.settings().unwrap().revision.0, 2);
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute(
        "UPDATE native_mission_claim_receipts SET settings_revision=2 WHERE work_id=?",
        [&id],
    )
    .unwrap();
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let error = root
        .read_mission_supervision(&id, &mission_at(&start, 1))
        .unwrap_err();
    assert_eq!(error.code(), "mission-claim-provenance-invalid");
    assert_eq!(before, fs::read(f.0.join(".swarm/state.db")).unwrap());
    db.execute(
        "UPDATE native_mission_claim_receipts SET settings_revision=1 WHERE work_id=?",
        [&id],
    )
    .unwrap();
    assert_eq!(
        root.read_mission_supervision(&id, &start)
            .unwrap()
            .unwrap()
            .settings_revision,
        Some(swarm_engine::SettingCount(1))
    );
    db.close().unwrap();
    root.close().unwrap();
}
/// A real new command links its own revision and both settings domains. Replay
/// preserves that actor/generation and creates no duplicate or retrospective link.
#[test]
fn mission_work_receipt_tracks_two_revisions_replay_and_reopen() {
    let (f, mut store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store
        .claim_mission_supervision(&id, "conductor", &start)
        .unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.mission_supervision_poll_interval_ms = Some(swarm_engine::SettingCount(4000));
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "command-policy".into(),
        expected_revision: doc.revision,
        values,
        reason: "Distinct command policy fixture".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    let mut env = MissionClock::fixed(mission_at(&start, 1));
    let raw = mission_title("linked-action", 1, "Linked action");
    let outcome = store
        .execute_mission_external(&lease, "work.update", &raw, &mut env)
        .unwrap();
    assert_eq!(outcome.settings_revision.0, 2);
    assert_eq!(lease.settings_revision().0, 1);
    let receipts = store.read_mission_work_receipts(&id).unwrap();
    assert_eq!(receipts.len(), 1);
    let first = &receipts[0];
    assert_eq!(first.event_id, "linked-action");
    assert_eq!(first.work_revision, 2);
    assert_eq!(first.kind, "work.update");
    assert_eq!(first.generation, 1);
    assert_eq!(first.claim_settings_revision.0, 1);
    assert_eq!(first.command_settings_revision.0, 2);
    assert!(!outcome.result.replayed);
    assert!(
        store
            .execute_mission_external(&lease, "work.update", &raw, &mut env)
            .unwrap()
            .result
            .replayed
    );
    assert_eq!(store.read_mission_work_receipts(&id).unwrap(), receipts);
    lease.close().unwrap();
    store.close().unwrap();
    let root = RootStore::open(&f.0, false).unwrap();
    assert_eq!(root.read_mission_work_receipts(&id).unwrap(), receipts);
    root.close().unwrap();
}
/// Journal schema failure after a valid witness must roll back the staged Work
/// command. Repair the injected schema before recovery with the same event.
#[test]
fn mission_work_receipt_failure_rolls_back_and_recovers() {
    let (f, mut store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store
        .claim_mission_supervision(&id, "conductor", &start)
        .unwrap();
    let mut env = MissionClock::fixed(mission_at(&start, 1));
    store
        .execute_mission_external(
            &lease,
            "work.update",
            &mission_title("witness", 1, "Witness"),
            &mut env,
        )
        .unwrap();
    assert_eq!(store.read_mission_work_receipts(&id).unwrap().len(), 1);
    let before = swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap();
    let events = store.events(&id).unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER fixture_receipt BEFORE INSERT ON native_mission_work_receipts BEGIN SELECT RAISE(ABORT,'PRIVATE fixture'); END").unwrap();
    let raw = mission_title("journal-refusal", 2, "Refused");
    assert_mission_refused(
        store.execute_mission_external(&lease, "work.update", &raw, &mut env),
        "mission-command-schema-invalid",
    );
    assert_eq!(
        before,
        swarm_engine::encode_model(&store.read_work(&id).unwrap()).unwrap()
    );
    assert_eq!(events, store.events(&id).unwrap());
    db.execute_batch("DROP TRIGGER fixture_receipt").unwrap();
    db.close().unwrap();
    store
        .execute_mission_external(&lease, "work.update", &raw, &mut env)
        .unwrap();
    assert_eq!(store.read_mission_work_receipts(&id).unwrap().len(), 2);
    lease.close().unwrap();
    store.close().unwrap();
}

/// A valid provenance witness precedes deletion of its epoch anchor. Known proof
/// must not silently downgrade to legacy unknown when authoritative state breaks.
#[test]
fn mission_claim_detached_epoch_refuses_without_legacy_downgrade() {
    let (f, store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store
        .claim_mission_supervision(&id, "witness", &start)
        .unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, &start)
            .unwrap()
            .unwrap()
            .settings_revision,
        Some(swarm_engine::SettingCount(1))
    );
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("DROP TABLE native_mission_lease_epochs")
        .unwrap();
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, &start)
            .unwrap_err()
            .code(),
        "mission-claim-provenance-invalid"
    );
    assert_eq!(before, fs::read(f.0.join(".swarm/state.db")).unwrap());
    db.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}

/// A genuine v1 epoch row has policy values but no provenance ledger. Readonly
/// observation keeps revision unknown without manufacturing it from current data.
#[test]
fn mission_claim_v1_revision_remains_unknown_without_mutation() {
    let (f, store, id) = Fixture::new();
    let start = store.read_work(&id).unwrap().created;
    let lease = store
        .claim_mission_supervision(&id, "witness", &start)
        .unwrap();
    assert_eq!(
        store
            .read_mission_supervision(&id, &start)
            .unwrap()
            .unwrap()
            .settings_revision,
        Some(swarm_engine::SettingCount(1))
    );
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("DROP TABLE native_mission_claim_receipts")
        .unwrap();
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let observed = store
        .read_mission_supervision(&id, &start)
        .unwrap()
        .unwrap();
    assert_eq!(observed.generation, Some(1));
    assert!(observed.settings_revision.is_none());
    assert_eq!(observed.policy.as_ref(), Some(lease.policy()));
    assert_eq!(before, fs::read(f.0.join(".swarm/state.db")).unwrap());
    db.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}

/// Persisted cycle diagnostics retain the first successful reconciliation, emit
/// only changed-error/recovery events and reopen without inventing acceptance.
#[test]
fn mission_cycle_error_transition_recovery_and_reopen() {
    use swarm_engine::MissionSupervisionState as S;
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "cycles", "2026-10-20T00:00:00Z")
        .unwrap();
    assert!(store
        .record_mission_cycle(&lease, "2026-10-20T00:00:01Z", None)
        .unwrap()
        .is_none());
    let first = store
        .read_mission_supervision(&id, "2026-10-20T00:00:01Z")
        .unwrap()
        .unwrap();
    assert_eq!(first.reconciled_at, "2026-10-20T00:00:01Z");
    let event = store
        .record_mission_cycle(
            &lease,
            "2026-10-20T00:00:02Z",
            Some("Provider\n indisponible\u{202e}"),
        )
        .unwrap()
        .unwrap();
    let value = store
        .read_mission_supervision(&id, "2026-10-20T00:00:02Z")
        .unwrap()
        .unwrap();
    assert_eq!(value.state, S::CycleError);
    assert_eq!(value.last_error, "Provider  indisponible");
    assert_eq!(value.last_check_at, "2026-10-20T00:00:02Z");
    assert!(store
        .record_mission_cycle(
            &lease,
            "2026-10-20T00:00:03Z",
            Some("Provider  indisponible")
        )
        .unwrap()
        .is_none());
    let changed = store
        .record_mission_cycle(&lease, "2026-10-20T00:00:03.5Z", Some("Different failure"))
        .unwrap()
        .unwrap();
    let recovery = store
        .record_mission_cycle(&lease, "2026-10-20T00:00:04Z", None)
        .unwrap()
        .unwrap();
    assert_ne!(event, recovery);
    assert!(store
        .record_mission_cycle(&lease, "2026-10-20T00:00:05Z", None)
        .unwrap()
        .is_none());
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    let rows:Vec<(String,String,String)>=db.prepare("SELECT event_key,kind,detail FROM mission_coordination_events WHERE work_id=? ORDER BY at,event_key").unwrap().query_map([&id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0],
        (event, "cycle-error".into(), "Provider  indisponible".into())
    );
    assert_eq!(
        rows[1],
        (changed, "cycle-error".into(), "Different failure".into())
    );
    assert_eq!(rows[2].0, recovery);
    assert_eq!(rows[2].1, "cycle-recovered");
    db.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
    let reopened = RootStore::open(&f.0, false).unwrap();
    let value = reopened
        .read_mission_supervision(&id, "2026-10-20T00:00:05Z")
        .unwrap()
        .unwrap();
    assert_eq!(value.state, S::Present);
    assert!(value.last_error.is_empty());
    assert_eq!(value.reconciled_at, first.reconciled_at);
    assert_eq!(value.next_check_at.as_deref(), Some("2026-10-20T00:00:07Z"));
    reopened.close().unwrap();
}
/// A valid cycle witness precedes malformed/expired/stopped/foreign-root cases;
/// refusals keep every lease field and event unchanged, and a new owner recovers.
#[test]
fn mission_cycle_refusal_preserves_state_and_new_owner_recovers() {
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "cycles", "2026-10-20T00:00:00Z")
        .unwrap();
    store
        .record_mission_cycle(&lease, "2026-10-20T00:00:01Z", None)
        .unwrap();
    let before = f.state();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    let no_events = || {
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM mission_coordination_events",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        )
    };
    no_events();
    for (at, error, code) in [
        ("malformed", None, "mission-clock-invalid"),
        (
            "2026-10-20T00:00:02Z",
            Some("\u{202e}\n\t"),
            "mission-cycle-error-empty",
        ),
        (
            "2026-10-20T00:00:31.000000001Z",
            None,
            "mission-lease-not-present",
        ),
    ] {
        assert_eq!(
            store
                .record_mission_cycle(&lease, at, error)
                .unwrap_err()
                .code(),
            code
        );
        assert_eq!(f.state(), before);
        no_events();
    }
    let (other, other_store, _) = Fixture::new();
    assert_eq!(
        other_store
            .record_mission_cycle(&lease, "2026-10-20T00:00:02Z", None)
            .unwrap_err()
            .code(),
        "mission-lease-root-mismatch"
    );
    other_store.close().unwrap();
    drop(other);
    assert_eq!(f.state(), before);
    store
        .update_mission_supervision(&lease, "2026-10-20T00:00:02Z", true)
        .unwrap();
    let stopped = f.state();
    assert_eq!(
        store
            .record_mission_cycle(&lease, "2026-10-20T00:00:03Z", None)
            .unwrap_err()
            .code(),
        "mission-lease-lost"
    );
    assert_eq!(f.state(), stopped);
    let new = store
        .claim_mission_supervision(&id, "cycles", "2026-10-20T00:00:03Z")
        .unwrap();
    let replaced = f.state();
    assert_eq!(
        store
            .record_mission_cycle(&lease, "2026-10-20T00:00:04Z", None)
            .unwrap_err()
            .code(),
        "mission-lease-lost"
    );
    assert_eq!(f.state(), replaced);
    no_events();
    store
        .record_mission_cycle(&new, "2026-10-20T00:00:04Z", Some("Owned diagnostic"))
        .unwrap();
    db.close().unwrap();
    new.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}
/// An actual journal INSERT failure rolls back the heartbeat and diagnostic;
/// dropping only the fixture trigger permits the same cycle to be recorded.
#[test]
fn mission_cycle_journal_failure_rolls_back_then_recovers() {
    let (f, store, id) = Fixture::new();
    let lease = store
        .claim_mission_supervision(&id, "cycles", "2026-10-20T00:00:00Z")
        .unwrap();
    store
        .record_mission_cycle(&lease, "2026-10-20T00:00:01Z", Some("Initial failure"))
        .unwrap();
    let before = f.state();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER fixture_cycle_failure BEFORE INSERT ON mission_coordination_events BEGIN SELECT RAISE(ABORT,'fixture cycle journal');END;").unwrap();
    let error = store
        .record_mission_cycle(&lease, "2026-10-20T00:00:02Z", None)
        .unwrap_err();
    assert_eq!(error.code(), "mission-storage-failed");
    assert!(error.source().is_some());
    assert_eq!(f.state(), before);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM mission_coordination_events",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    db.execute_batch("DROP TRIGGER fixture_cycle_failure")
        .unwrap();
    assert!(store
        .record_mission_cycle(&lease, "2026-10-20T00:00:02Z", None)
        .unwrap()
        .is_some());
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM mission_coordination_events",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    db.close().unwrap();
    lease.close().unwrap();
    store.close().unwrap();
}

/// Fixture-only canonical Go-shaped rows; neither a supported permission
/// mutation nor evidence of authenticated consent or automatic launch.
fn mission_controls_rows(db: &rusqlite::Connection, work: &str) {
    db.execute(
        "INSERT INTO cockpit_controls(work_id,paused,autonomy,slots) VALUES(?,0,'autonome',17)",
        [work],
    )
    .unwrap();
    let raw = serde_json::to_string(&serde_json::json!({"enabled":true,"actor":"declared operator","at":"2026-10-20T00:00:00Z"})).unwrap();
    db.execute("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,'2026-10-20T00:00:00Z','mission-policy',?)", rusqlite::params![work,raw]).unwrap();
}
/// Historical absence remains unknown, actual slots are not clamped and the
/// newest explicit revocation survives readonly reopen. Reads never write.
#[test]
fn mission_controls_absence_latest_revocation_and_reopen() {
    use swarm_engine::MissionAutonomy;
    let (f, root, id) = Fixture::new();
    let path = f.0.join(".swarm/state.db");
    let before = fs::read(&path).unwrap();
    let absent = root.read_mission_controls(&id).unwrap();
    assert!(absent.consent.is_none());
    assert!(absent.controls.is_none());
    assert!(!absent.archived);
    assert_eq!(fs::read(&path).unwrap(), before);
    let db = rusqlite::Connection::open(&path).unwrap();
    mission_controls_rows(&db, &id);
    let before = fs::read(&path).unwrap();
    let valid = root.read_mission_controls(&id).unwrap();
    let consent = valid.consent.unwrap();
    assert!(consent.enabled);
    assert_eq!(consent.declared_actor, "declared operator");
    assert_eq!(consent.declared_at, "2026-10-20T00:00:00Z");
    assert_eq!(consent.recorded_at, "2026-10-20T00:00:00Z");
    let controls = valid.controls.unwrap();
    assert_eq!(controls.slots, 17);
    assert_eq!(controls.autonomy, MissionAutonomy::Automatic);
    assert!(!controls.paused);
    assert_eq!(fs::read(&path).unwrap(), before);
    db.execute("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,'2026-10-20T00:00:01Z','mission-policy',?)",
        rusqlite::params![id,r#"{"enabled":false,"actor":"declared revoker","at":"2026-10-20T00:00:01Z"}"#]).unwrap();
    db.execute("INSERT INTO mission_lifecycle(work_id,archived_at,archive_path,archive_digest) VALUES(?,'2026-10-20T00:00:02Z','fixture','unverified')",[&id]).unwrap();
    db.execute(
        "UPDATE cockpit_controls SET paused=1,autonomy='manuel' WHERE work_id=?",
        [&id],
    )
    .unwrap();
    let before = fs::read(&path).unwrap();
    let last = root.read_mission_controls(&id).unwrap();
    assert!(!last.consent.as_ref().unwrap().enabled);
    assert!(last.consent.as_ref().unwrap().event_sequence > consent.event_sequence);
    assert!(last.archived);
    assert!(last.controls.as_ref().unwrap().paused);
    assert_eq!(
        last.controls.as_ref().unwrap().autonomy,
        MissionAutonomy::Manual
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(!format!("{:?}", last).contains("declared revoker"));
    db.close().unwrap();
    root.close().unwrap();
    let reopened = RootStore::open(&f.0, false).unwrap();
    assert_eq!(reopened.read_mission_controls(&id).unwrap(), last);
    assert!(reopened.read_mission_controls("unknown-work").is_err());
    reopened.close().unwrap();
}
/// A valid newest consent controls malformed JSON/type/time variants; an invalid
/// newer row cannot silently fall back to the previous enabled receipt.
#[test]
fn mission_controls_bad_consent_refuses_without_fallback_or_mutation() {
    let (f, root, id) = Fixture::new();
    let path = f.0.join(".swarm/state.db");
    let db = rusqlite::Connection::open(&path).unwrap();
    mission_controls_rows(&db, &id);
    assert!(
        root.read_mission_controls(&id)
            .unwrap()
            .consent
            .unwrap()
            .enabled
    );
    db.execute("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,'2026-10-20T00:00:01Z','mission-policy','null')",[&id]).unwrap();
    let seq = db.last_insert_rowid();
    for raw in [
        "null",
        r#"{"enabled":true,"enabled":false,"actor":"PRIVATE","at":"2026-10-20T00:00:00Z"}"#,
        r#"{"enabled":null,"actor":"PRIVATE","at":"2026-10-20T00:00:00Z"}"#,
        r#"{"enabled":true,"actor":"PRIVATE"}"#,
        r#"{"enabled":true,"actor":"PRIVATE","at":"2026-10-20T00:00:00Z","unknown":true}"#,
    ] {
        db.execute(
            "UPDATE cockpit_events SET message=? WHERE seq=?",
            rusqlite::params![raw, seq],
        )
        .unwrap();
        let before = fs::read(&path).unwrap();
        let error = root.read_mission_controls(&id).unwrap_err();
        assert_eq!(error.code(), "mission-consent-invalid");
        assert!(error.source().is_some());
        assert!(!error.to_string().contains("PRIVATE"));
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    for (actor, at, code) in [
        (
            "\u{202e}",
            "2026-10-20T00:00:00Z",
            "mission-consent-actor-invalid",
        ),
        (
            "declared",
            "PRIVATE invalid time",
            "mission-consent-time-invalid",
        ),
    ] {
        let raw = serde_json::to_string(&serde_json::json!({"enabled":true,"actor":actor,"at":at}))
            .unwrap();
        db.execute(
            "UPDATE cockpit_events SET message=? WHERE seq=?",
            rusqlite::params![raw, seq],
        )
        .unwrap();
        let before = fs::read(&path).unwrap();
        assert_eq!(root.read_mission_controls(&id).unwrap_err().code(), code);
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    db.execute("DELETE FROM cockpit_events WHERE seq=?", [seq])
        .unwrap();
    assert!(
        root.read_mission_controls(&id)
            .unwrap()
            .consent
            .unwrap()
            .enabled
    );
    db.execute(
        "UPDATE cockpit_events SET at='PRIVATE invalid' WHERE work_id=?",
        [&id],
    )
    .unwrap();
    let before = fs::read(&path).unwrap();
    assert_eq!(
        root.read_mission_controls(&id).unwrap_err().code(),
        "mission-consent-time-invalid"
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    db.execute(
        "UPDATE cockpit_events SET at='2026-10-20T00:00:00Z' WHERE work_id=?",
        [&id],
    )
    .unwrap();
    assert!(
        root.read_mission_controls(&id)
            .unwrap()
            .consent
            .unwrap()
            .enabled
    );
    db.close().unwrap();
    root.close().unwrap();
}
/// Corrupt controls and a missing SQL table are errors, not defaults. Restore
/// the original fixture DDL/data, then verify SDK recovery and readonly reopen.
#[test]
fn mission_controls_corruption_sql_refusal_and_recovery() {
    let (f, root, id) = Fixture::new();
    let path = f.0.join(".swarm/state.db");
    let db = rusqlite::Connection::open(&path).unwrap();
    mission_controls_rows(&db, &id);
    assert_eq!(
        root.read_mission_controls(&id)
            .unwrap()
            .controls
            .unwrap()
            .slots,
        17
    );
    for (sql, code) in [
        (
            "UPDATE cockpit_controls SET paused='PRIVATE malformed'",
            "mission-storage-failed",
        ),
        (
            "UPDATE cockpit_controls SET paused=0,autonomy='unknown'",
            "mission-controls-autonomy-invalid",
        ),
        (
            "UPDATE cockpit_controls SET autonomy='autonome',slots=0",
            "mission-controls-slots-invalid",
        ),
    ] {
        db.execute_batch(sql).unwrap();
        let before = fs::read(&path).unwrap();
        assert_eq!(root.read_mission_controls(&id).unwrap_err().code(), code);
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    db.execute_batch("UPDATE cockpit_controls SET paused=0,autonomy='assiste',slots=17")
        .unwrap();
    assert_eq!(
        root.read_mission_controls(&id)
            .unwrap()
            .controls
            .unwrap()
            .autonomy,
        swarm_engine::MissionAutonomy::Assisted
    );
    let ddl: String = db
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='cockpit_controls'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    db.execute_batch("DROP TABLE cockpit_controls").unwrap();
    let before = fs::read(&path).unwrap();
    let error = root.read_mission_controls(&id).unwrap_err();
    assert_eq!(error.code(), "mission-storage-failed");
    assert!(error.source().is_some());
    assert_eq!(fs::read(&path).unwrap(), before);
    db.execute_batch(&ddl).unwrap();
    db.execute(
        "INSERT INTO cockpit_controls(work_id,paused,autonomy,slots) VALUES(?,0,'assiste',17)",
        [&id],
    )
    .unwrap();
    assert_eq!(
        root.read_mission_controls(&id)
            .unwrap()
            .controls
            .unwrap()
            .slots,
        17
    );
    db.close().unwrap();
    root.close().unwrap();
    let reopened = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        reopened
            .read_mission_controls(&id)
            .unwrap()
            .controls
            .unwrap()
            .slots,
        17
    );
    reopened.close().unwrap();
}

/// Explicit default is an observation only; saving new Admin policy changes the
/// next read and survives reopen without creating controls or consent.
#[test]
fn mission_dispatch_saved_settings_consumer_absence_and_reopen() {
    let (f, root, id) = Fixture::new();
    let first = root.read_mission_dispatch_controls(&id).unwrap();
    assert_eq!(first.slots, swarm_engine::SettingCount(2));
    assert!(first.controls.controls.is_none() && first.controls.consent.is_none());
    let doc = root.settings().unwrap();
    let mut values = doc.values.clone();
    values.mission_dispatch_default_slots = Some(swarm_engine::SettingCount(3));
    values.mission_dispatch_max_slots = Some(swarm_engine::SettingCap(Some(
        swarm_engine::SettingCount(4),
    )));
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "dispatch-policy".into(),
        expected_revision: doc.revision,
        values,
        reason: "Fixture configured slots".into(),
        preview_token: String::new(),
    };
    let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    change.preview_token = root.preview_settings(&change).unwrap().preview_token;
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    root.apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    let next = root.read_mission_dispatch_controls(&id).unwrap();
    assert_eq!(next.slots, swarm_engine::SettingCount(3));
    assert!(next.controls.controls.is_none());
    assert!(next.controls.consent.is_none());
    assert_eq!(
        next.settings_revision,
        swarm_engine::SettingCount(doc.revision.0 + 1)
    );
    root.close().unwrap();
    let reopened = RootStore::open(&f.0, true).unwrap();
    assert_eq!(
        reopened.read_mission_dispatch_controls(&id).unwrap().slots,
        next.slots
    );
    reopened.close().unwrap();
}
/// An actual count17 is readable as fact but refuses cap16 without SQL/settings
/// mutation. Removing the operator cap recovers the same row without rewriting.
#[test]
fn mission_dispatch_outside_policy_refuses_without_clamp_and_recovers() {
    let (f, root, id) = Fixture::new();
    assert_eq!(
        root.read_mission_dispatch_controls(&id).unwrap().slots,
        swarm_engine::SettingCount(2)
    );
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    mission_controls_rows(&db, &id);
    let before = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let settings_before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let error = root.read_mission_dispatch_controls(&id).unwrap_err();
    assert_eq!(error.code(), "mission-settings-invalid");
    let original = error
        .source()
        .unwrap()
        .downcast_ref::<swarm_engine::EngineSettingsError>()
        .unwrap();
    assert_eq!(
        original.reason(),
        "mission-dispatch-saved-slots-outside-policy"
    );
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        settings_before
    );
    assert_eq!(
        root.read_mission_controls(&id)
            .unwrap()
            .controls
            .unwrap()
            .slots,
        17
    );
    let doc = root.settings().unwrap();
    let mut values = doc.values.clone();
    values.mission_dispatch_max_slots = Some(swarm_engine::SettingCap(None));
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "remove-dispatch-cap".into(),
        expected_revision: doc.revision,
        values,
        reason: "Fixture removes only operator cap".into(),
        preview_token: String::new(),
    };
    change.preview_token = root.preview_settings(&change).unwrap().preview_token;
    root.apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    let recovered = root.read_mission_dispatch_controls(&id).unwrap();
    assert_eq!(recovered.slots, swarm_engine::SettingCount(17));
    assert_eq!(recovered.controls.controls.unwrap().slots, 17);
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    db.close().unwrap();
    root.close().unwrap();
}
