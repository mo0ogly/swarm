#![cfg(target_os = "linux")]
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
};
use swarm_engine::{
    AdminBudgetChange, AdminBudgetPolicy, AdminBudgetReservationRequest, RootStore, SettingCount,
    SystemEnvironment,
};
/// Only this isolated fixture is owned; keep the original on a test panic.
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
/// Initialize a real native Root through its public API; no SQL/data fabrication.
fn fixture() -> Fixture {
    let path =
        std::env::temp_dir().join(swarm_engine::generate_identifier("admin-budget-").unwrap());
    fs::create_dir(&path).unwrap();
    RootStore::initialize(&path, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    Fixture(path)
}
/// Explicit money/provenance fixture, never a product default or real tariff.
fn change(revision: i64, event: &str, limit: Option<i64>, reserve: i64) -> AdminBudgetChange {
    AdminBudgetChange {
        schema_version: 1,
        event_id: event.into(),
        expected_revision: SettingCount(revision),
        policy: AdminBudgetPolicy {
            limit_micro_usd: limit.map(SettingCount),
            reserve_per_call_micro_usd: SettingCount(reserve),
            estimate_source: "Owned test estimate; not a model price".into(),
        },
        reason: "Actual native Admin budget integration".into(),
        preview_token: String::new(),
    }
}
/// Preview through Root and retain its token in the exact original intention.
fn preview(store: &RootStore, mut change: AdminBudgetChange) -> AdminBudgetChange {
    change.preview_token = store
        .preview_admin_advice_budget(&serde_json::to_vec(&change).unwrap())
        .unwrap()
        .preview_token;
    change
}
/// Publish through the same public Root operation consumed by CLI/HTTP.
fn save(store: &mut RootStore, change: &AdminBudgetChange) {
    let reply = store
        .apply_admin_advice_budget(&serde_json::to_vec(change).unwrap(), &mut SystemEnvironment)
        .unwrap();
    assert!(reply.applied)
}
/// Standalone real accounting consumer. It supplies no provider/exec permission.
fn charge(
    path: &Path,
    id: &str,
) -> Result<swarm_engine::AdminBudgetReservation, swarm_engine::RootStoreError> {
    let mut store = RootStore::open(path, true)?;
    let result = store.reserve_admin_advice_budget(&AdminBudgetReservationRequest {
        call_id: id.into(),
        request_sha256: "a".repeat(64),
        expected_budget_revision: SettingCount(1),
    });
    let close = store.close();
    match (result, close) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(primary), Err(cleanup)) => Err(swarm_engine::RootStoreError::Cleanup {
            primary: Some(Box::new(primary)),
            cleanup: Box::new(cleanup),
        }),
    }
}
#[test]
fn actual_root_budget_preview_save_reopen_replay_preserves_settings() {
    let root = fixture();
    let mut store = RootStore::open(&root.0, true).unwrap();
    let settings = serde_json::to_vec(&store.settings().unwrap()).unwrap();
    let absent = store.admin_advice_budget().unwrap();
    assert!(!absent.configured);
    assert_eq!(absent.reserved_estimate_micro_usd.0, 0);
    let request = preview(&store, change(0, "first", Some(9_007_199_254_740_993), 1));
    assert_eq!(store.admin_advice_budget().unwrap(), absent);
    save(&mut store, &request);
    store.close().unwrap();
    let mut store = RootStore::open(&root.0, true).unwrap();
    let saved = store.admin_advice_budget().unwrap();
    assert_eq!(
        saved.policy.unwrap().limit_micro_usd.unwrap().0,
        9_007_199_254_740_993
    );
    assert_eq!(saved.history.len(), 1);
    let replay = store
        .apply_admin_advice_budget(
            &serde_json::to_vec(&request).unwrap(),
            &mut SystemEnvironment,
        )
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.document.history.len(), 1);
    assert_eq!(
        serde_json::to_vec(&store.settings().unwrap()).unwrap(),
        settings
    );
    store.close().unwrap();
}
#[test]
fn real_roots_budget_reservations_are_exclusive_without_reset_or_free_replay() {
    let root = fixture();
    let mut store = RootStore::open(&root.0, true).unwrap();
    let request = preview(&store, change(0, "first", Some(100), 60));
    save(&mut store, &request);
    store.close().unwrap();
    let gate = Arc::new(Barrier::new(3));
    let handles: Vec<_> = ["call-a", "call-b"]
        .into_iter()
        .map(|id| {
            let gate = Arc::clone(&gate);
            let path = root.0.clone();
            std::thread::spawn(move || {
                gate.wait();
                charge(&path, id)
            })
        })
        .collect();
    gate.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .next()
            .unwrap()
            .code(),
        "admin-budget-exhausted"
    );
    let original = results.into_iter().find_map(Result::ok).unwrap();
    assert!(!original.execution_authorized);
    let replay = charge(&root.0, &original.call_id).unwrap();
    assert!(replay.replayed);
    let mut store = RootStore::open(&root.0, true).unwrap();
    assert_eq!(
        store
            .admin_advice_budget()
            .unwrap()
            .reserved_estimate_micro_usd
            .0,
        60
    );
    let lowered = preview(&store, change(1, "lowered", Some(0), 20));
    save(&mut store, &lowered);
    assert_eq!(
        store
            .admin_advice_budget()
            .unwrap()
            .reserved_estimate_micro_usd
            .0,
        60
    );
    let old = store
        .reserve_admin_advice_budget(&AdminBudgetReservationRequest {
            call_id: original.call_id,
            request_sha256: "a".repeat(64),
            expected_budget_revision: SettingCount(1),
        })
        .unwrap();
    assert!(old.replayed);
    assert_eq!(old.reserved_estimate_micro_usd.0, 60);
    store.close().unwrap();
}
#[test]
fn public_budget_stale_preview_and_changed_source_refuse_then_recover() {
    let root = fixture();
    let mut store = RootStore::open(&root.0, true).unwrap();
    let request = preview(&store, change(0, "first", Some(100), 10));
    save(&mut store, &request);
    let next = preview(&store, change(1, "next", None, 20));
    let first = store
        .reserve_admin_advice_budget(&AdminBudgetReservationRequest {
            call_id: "one".into(),
            request_sha256: "a".repeat(64),
            expected_budget_revision: SettingCount(1),
        })
        .unwrap();
    assert!(!first.replayed);
    let before = store.admin_advice_budget().unwrap();
    assert_eq!(
        store
            .apply_admin_advice_budget(&serde_json::to_vec(&next).unwrap(), &mut SystemEnvironment)
            .unwrap_err()
            .code(),
        "admin-budget-preview-stale"
    );
    assert_eq!(store.admin_advice_budget().unwrap(), before);
    let fresh = preview(&store, next);
    save(&mut store, &fresh);
    assert_eq!(
        store
            .admin_advice_budget()
            .unwrap()
            .reserved_estimate_micro_usd
            .0,
        10
    );
    store.close().unwrap();
}
