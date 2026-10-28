//! Durable reservations on isolated Roots, without protected publication claims.
#![cfg(target_os = "linux")]
use swarm_engine::{
    PaeaGitIntentObservation as Observation, PaeaGitIntentReservation as Reservation, RootStore,
    SystemEnvironment,
};
use swarm_protection_contract::{canonicalize, Action, AdmissionContext, Proposal};
const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// Build a private disposable native Root, never an existing mission database.
fn root() -> (tempfile::TempDir, RootStore) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, store)
}
/// Valid data declarations only: neither review nor human authority is claimed.
fn action() -> (Proposal, AdmissionContext) {
    (
        Proposal {
            schema_version: 1,
            request_id: "transport-one".into(),
            action: Action::Git {
                repository_id: "repository".into(),
                target_ref: "refs/heads/target".into(),
                expected_old_oid: "1".repeat(40),
                candidate_oid: "2".repeat(40),
            },
        },
        AdmissionContext {
            project_id: "project".into(),
            principal_id: "person".into(),
            instance_id: "instance".into(),
            mandate_id: "mandate".into(),
            mandate_revision: 1,
            policy_revision: 1,
            policy_digest: "a".repeat(64),
            tool_manifest_digest: "b".repeat(64),
            sources: vec![],
        },
    )
}
/// A valid reservation is an explicit prerequisite before each negative branch.
fn control(store: &RootStore) -> swarm_engine::PaeaGitIntent {
    let (p, c) = action();
    let record = match store.paea_reserve_git_intent(&p, &c, PIN).unwrap() {
        Reservation::New(r) => r,
        _ => panic!("new fixture required"),
    };
    assert_eq!(
        store.paea_read_git_intent(&record.intent_id, PIN).unwrap(),
        Some(record.clone())
    );
    record
}
/// Persisted identity is independent of transport, candidate and policy. Reopen
/// retains the old exact bytes and refuses reinitialization instead of adoption.
#[test]
fn git_intent_durable_alias_conflict_and_restart() {
    let (dir, store) = root();
    store.paea_initialize_git_intents("project", PIN).unwrap();
    let record = control(&store);
    let (mut p, mut c) = action();
    p.request_id = "transport-two".into();
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap(),
        Reservation::Existing(record.clone())
    );
    if let Action::Git { candidate_oid, .. } = &mut p.action {
        *candidate_oid = "3".repeat(40);
    }
    c.policy_revision += 1;
    assert_ne!(canonicalize(&p, &c).unwrap().digest(), record.action_digest);
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap(),
        Reservation::Existing(record.clone())
    );
    store.close().unwrap();
    let store = RootStore::open(dir.path(), true).unwrap();
    assert_eq!(
        store.paea_read_git_intent(&record.intent_id, PIN).unwrap(),
        Some(record.clone())
    );
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap(),
        Reservation::Existing(record)
    );
    assert_eq!(
        store
            .paea_initialize_git_intents("project", PIN)
            .unwrap_err()
            .code,
        "paea-git-intent-already-initialized"
    );
    store.close().unwrap();
}
/// Missing provisioning/pin mismatch/payment/invalid ref/read-only all refuse;
/// an independent read keeps the valid original unchanged and absence explicit.
#[test]
fn git_intent_refusals_preserve_original_and_recover() {
    let (dir, store) = root();
    let (mut p, c) = action();
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap_err().code,
        "paea-git-intent-uninitialized"
    );
    store.paea_initialize_git_intents("project", PIN).unwrap();
    let original = control(&store);
    assert!(store
        .paea_read_git_intent(&"c".repeat(64), PIN)
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .paea_reserve_git_intent(&p, &c, &"b".repeat(64))
            .unwrap_err()
            .code,
        "paea-git-intent-project-pin"
    );
    let mut wrong = c.clone();
    wrong.project_id = "other".into();
    assert_eq!(
        store
            .paea_reserve_git_intent(&p, &wrong, PIN)
            .unwrap_err()
            .code,
        "paea-git-intent-project-pin"
    );
    if let Action::Git { target_ref, .. } = &mut p.action {
        *target_ref = "refs/heads/target\ncommit".into();
    }
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap_err().code,
        "paea-git-intent-proposal"
    );
    p.action = Action::Payment {
        invoice_id: "invoice".into(),
        invoice_version: "1".into(),
        amount_minor: "5".into(),
        currency: "EUR".into(),
        beneficiary_account_id: "beneficiary".into(),
    };
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap_err().code,
        "paea-git-intent-git-required"
    );
    store.close().unwrap();
    let store = RootStore::open(dir.path(), false).unwrap();
    let (p, c) = action();
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap_err().code,
        "paea-git-intent-write-binding"
    );
    assert_eq!(
        store
            .paea_read_git_intent(&original.intent_id, PIN)
            .unwrap(),
        Some(original)
    );
    store.close().unwrap();
}
/// Persist 0ms via public Admin preview/apply for real deterministic SQL contention.
/// This is fixture policy, not a global watchdog or changed production default.
fn zero_busy(store: &RootStore) {
    let document = store.settings().unwrap();
    let mut values = document.values.clone();
    values.storage_busy_timeout_ms = swarm_engine::SettingCount(0);
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: document.schema_version,
        event_id: "zero-busy".into(),
        expected_revision: document.revision,
        values,
        reason: "Observable contention fixture".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
}
/// Successful first intent, then actual SQLITE_BUSY on COMMIT. Failed publication
/// remains uncertain, cause graph preserves code across Box/IO/Root wrappers;
/// independent read after rollback establishes absence before a fresh reserve.
#[test]
fn git_intent_commit_uncertain_then_read_and_recover() {
    let (dir, store) = root();
    zero_busy(&store);
    store.paea_initialize_git_intents("project", PIN).unwrap();
    control(&store);
    let reader = rusqlite::Connection::open_with_flags(
        dir.path().join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mode: String = reader
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "delete");
    reader.execute_batch("BEGIN").unwrap();
    let n: i64 = reader
        .query_row("SELECT count(*) FROM paea_git_intents", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
    let (mut p, c) = action();
    if let Action::Git { target_ref, .. } = &mut p.action {
        *target_ref = "refs/heads/second".into();
    }
    let intent = canonicalize(&p, &c).unwrap().intent_id().to_string();
    let e = store.paea_reserve_git_intent(&p, &c, PIN).unwrap_err();
    assert_eq!(e.code, "paea-git-intent-commit-uncertain");
    assert_eq!(e.observation, Observation::Uncertain);
    assert!(e.cleanup.is_empty());
    assert_eq!(
        e.cause
            .downcast_ref::<rusqlite::Error>()
            .unwrap()
            .sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy)
    );
    let wrapped = std::io::Error::other(Box::new(e));
    let root_error = swarm_engine::RootStoreError::SqlTransaction(Box::new(wrapped));
    let mut pending: Vec<&(dyn std::error::Error + 'static)> = vec![&root_error];
    let mut uncertain = false;
    let mut busy = false;
    while let Some(error) = pending.pop() {
        let links = swarm_engine::native_error_links(error);
        uncertain |= links
            .labels
            .iter()
            .any(|s| s == "paea-git-intent-uncertain");
        busy |= links.sqlite_code.is_some();
        pending.extend(links.children.into_iter().map(|(_, e)| e));
    }
    assert!(uncertain && busy);
    reader.execute_batch("ROLLBACK").unwrap();
    reader.close().unwrap();
    assert!(store.paea_read_git_intent(&intent, PIN).unwrap().is_none());
    assert!(matches!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap(),
        Reservation::New(_)
    ));
    store.close().unwrap();
}
/// Corrupted canonical bytes are not accepted as a replay; repair only a
/// disposable fixture's exact old row, then demonstrate recovery unchanged.
#[test]
fn git_intent_corrupt_record_refuses_and_recovers() {
    let (dir, store) = root();
    store.paea_initialize_git_intents("project", PIN).unwrap();
    let original = control(&store);
    let db = rusqlite::Connection::open(dir.path().join(".swarm/state.db")).unwrap();
    db.execute(
        "UPDATE paea_git_intents SET action_digest=?",
        ["d".repeat(64)],
    )
    .unwrap();
    assert_eq!(
        store
            .paea_read_git_intent(&original.intent_id, PIN)
            .unwrap_err()
            .code,
        "paea-git-intent-binding"
    );
    let (p, c) = action();
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap_err().code,
        "paea-git-intent-binding"
    );
    db.execute(
        "UPDATE paea_git_intents SET action_digest=?",
        [&original.action_digest],
    )
    .unwrap();
    db.close().unwrap();
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap(),
        Reservation::Existing(original)
    );
    store.close().unwrap();
}

/// Two separate connections race for one new business key. Exactly one insert
/// succeeds; the other returns the exact original rather than another permit.
#[test]
fn git_intent_two_connections_have_one_new_reservation() {
    let (dir, store) = root();
    store.paea_initialize_git_intents("project", PIN).unwrap();
    control(&store);
    let (mut p, c) = action();
    if let Action::Git { target_ref, .. } = &mut p.action {
        *target_ref = "refs/heads/race".into();
    }
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let mut handles = vec![];
        for _ in 0..2 {
            let path = dir.path();
            let barrier = &barrier;
            let p = &p;
            let c = &c;
            handles.push(scope.spawn(move || {
                let other = RootStore::open(path, true).unwrap();
                barrier.wait();
                let r = other.paea_reserve_git_intent(p, c, PIN).unwrap();
                other.close().unwrap();
                r
            }));
        }
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Reservation::New(_)))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Reservation::Existing(_)))
            .count(),
        1
    );
    let records: Vec<_> = results
        .iter()
        .map(|r| match r {
            Reservation::New(r) | Reservation::Existing(r) => r,
        })
        .collect();
    assert_eq!(records[0], records[1]);
    store.close().unwrap();
}
/// Unexpected schema triggers refuse both read and reservation. Only a fixture
/// removes its own injected trigger, then observes the unchanged original row.
#[test]
fn git_intent_schema_change_refuses_and_recovers() {
    let (dir, store) = root();
    store.paea_initialize_git_intents("project", PIN).unwrap();
    let original = control(&store);
    let db = rusqlite::Connection::open(dir.path().join(".swarm/state.db")).unwrap();
    db.execute_batch(
        "CREATE TRIGGER fixture_intent AFTER INSERT ON paea_git_intents BEGIN SELECT 1; END;",
    )
    .unwrap();
    assert!(store
        .paea_read_git_intent(&original.intent_id, PIN)
        .is_err());
    let (p, c) = action();
    assert!(store.paea_reserve_git_intent(&p, &c, PIN).is_err());
    db.execute_batch("DROP TRIGGER fixture_intent;").unwrap();
    db.close().unwrap();
    assert_eq!(
        store.paea_reserve_git_intent(&p, &c, PIN).unwrap(),
        Reservation::Existing(original)
    );
    store.close().unwrap();
}
