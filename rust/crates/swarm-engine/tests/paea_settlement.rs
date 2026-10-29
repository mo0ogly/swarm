//! Real signed current authority and two physically distinct isolated Root stores.
//! Fictitious SQLite effects only; no protected daemon, real payment or human claim.
#![cfg(target_os = "linux")]
#[path = "../../swarm-enrollment-authority/tests/common/authenticated_fixture.rs"]
mod fixture;
use serde_json::{json, Value};
use swarm_engine::{
    PaeaSettlementError, PaeaSettlementObservation as Observation,
    PaeaSettlementOutcome as Outcome, PaeaSettlementRequest, RootStore, SystemEnvironment,
};
use swarm_enrollment_authority::{
    authenticated_control::Context as UseContext, authenticated_startup, paea_action::Operation,
};
use swarm_enrollment_state::{sign_change, Context};
use swarm_identity_contract::Scope;
use swarm_protection_contract::{Action, AdmissionContext, Proposal};

/// Create one owned native Root; no existing user/project database is opened.
fn root() -> (tempfile::TempDir, RootStore) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, store)
}
/// Actual public work.create and persisted plan stamp, no fabricated generation.
fn project() -> (tempfile::TempDir, RootStore, Scope) {
    let (dir, mut store) = root();
    let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Settlement plan","objective":"Exact fictitious effect","scope":"Fixture","criteria":["No duplicate invoice"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
    let config = json!({"schema_version":1,"event_id":"effect-config","expected_revision":0,"reason":"Explicit fixture effect role","policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},"identities":[{"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}]});
    store
        .trust("config-apply", &serde_json::to_vec(&config).unwrap(), "")
        .unwrap_or_else(|e| {
            let reason = match &e {
                swarm_engine::RootStoreError::Trust(cause) => cause.reason(),
                _ => "non-Trust failure",
            };
            panic!("explicit fixture config refused: {}: {reason}", e.code());
        });
    let scope = scope(dir.path(), &work);
    (dir, store, scope)
}
/// Read only a fixture plan, finalizing the independent connection before use.
fn scope(dir: &std::path::Path, work: &str) -> Scope {
    let db = rusqlite::Connection::open_with_flags(
        dir.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let version: i64 = db
        .query_row(
            "SELECT projection_version FROM agent_identity_plan_heads WHERE work_id=?",
            [work],
            |r| r.get(0),
        )
        .unwrap();
    let projection = match version {
        1 => swarm_engine::IdentityPlanProjection::V1,
        2 => swarm_engine::IdentityPlanProjection::V2,
        _ => panic!("unknown fixture projection"),
    };
    let stamp = swarm_engine::read_identity_plan_stamp(&db, work, projection).unwrap();
    db.close().unwrap();
    Scope {
        work_id: stamp.work_id,
        plan_version: stamp.generation,
        plan_digest: stamp.digest,
    }
}
/// Canonical interchange fixture remains declared data, never admission evidence.
fn action() -> (Proposal, AdmissionContext) {
    let doc = swarm_protection_contract::decode_document(include_bytes!(
        "../../../../web/paea-examples/v1/payment-document.json"
    ))
    .unwrap();
    (doc.proposal, doc.context)
}
/// Count actual durable ledger effects through a separately opened read-only DB.
fn count(dir: &std::path::Path) -> i64 {
    let db = rusqlite::Connection::open_with_flags(
        dir.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let n = db
        .query_row("SELECT count(*) FROM paea_settlement_effects", [], |r| {
            r.get(0)
        })
        .unwrap();
    db.close().unwrap();
    n
}
/// Mint a separately signed fixture permission for this exact retained request,
/// open actual personal/business kernels, consume once and close both explicitly.
fn invoke(
    project: &RootStore,
    ledger: &RootStore,
    request: &PaeaSettlementRequest,
    scope: &Scope,
) -> Result<Outcome, PaeaSettlementError> {
    invoke_revision(project, ledger, request, scope, 1)
}
/// Explicit current Trust revision for mapping-change refusal/recovery fixtures.
fn invoke_revision(
    project: &RootStore,
    ledger: &RootStore,
    request: &PaeaSettlementRequest,
    scope: &Scope,
    revision: i64,
) -> Result<Outcome, PaeaSettlementError> {
    let f =
        fixture::Fixture::build_for_trust(request.request().permission(), scope.clone(), revision);
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let result = ledger
        .call_paea_settlement(
            project,
            request,
            &app,
            &UseContext::new(),
            &head,
            b"STARTUP_FIXTURE_PASSWORD",
        )
        .map_err(|e| match e {
            swarm_engine::PaeaSettlementCallError::Action(error) => error,
            other => panic!("unexpected current authority refusal: {}", other.code()),
        });
    app.close().unwrap();
    f.close();
    result
}
/// Read known receipt fields while distinguishing replay from first publication.
fn recorded(outcome: Outcome, replayed: bool) -> swarm_engine::PaeaSettlementReceipt {
    match outcome {
        Outcome::Recorded {
            receipt,
            replayed: actual,
        } => {
            assert_eq!(actual, replayed);
            receipt
        }
        _ => panic!("expected recorded effect"),
    }
}

/// Actual publication, new transport key replay, close/reopen and reconciliation.
#[test]
fn settlement_current_authority_unique_transport_reopen_and_reconcile() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let request = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    let historical_context = request.context().clone();
    assert_eq!(historical_context.policy_revision, 1);
    assert_ne!(historical_context.policy_digest, c.policy_digest);
    assert_eq!(c.policy_digest, "aa".repeat(32));
    let first = recorded(invoke(&project, &ledger, &request, &scope).unwrap(), false);
    assert_eq!(first.authority_principal.id, "operator");
    assert_eq!(first.authority_scope, scope);
    assert_eq!(first.authority_project_digest, "0b".repeat(32));
    assert_eq!(first.invoice_id, "invoice-17");
    assert_eq!(count(ld.path()), 1);
    let replay = recorded(invoke(&project, &ledger, &request, &scope).unwrap(), true);
    assert_eq!(replay.action_digest, first.action_digest);
    assert_eq!(replay.commit_requested_at, first.commit_requested_at);
    let mut new_key = p.clone();
    new_key.request_id = "another-transport-key".into();
    let other = project
        .paea_settlement_request(&ledger, &new_key, &c, Operation::Commit)
        .unwrap();
    assert_ne!(
        request.request().root_digest(),
        other.request().root_digest()
    );
    assert_eq!(request.canonical().digest(), other.canonical().digest());
    let replay = recorded(invoke(&project, &ledger, &other, &scope).unwrap(), true);
    assert_eq!(replay.intent_id, first.intent_id);
    assert_eq!(count(ld.path()), 1);
    other.close().unwrap();
    request.close().unwrap();
    ledger.close().unwrap();
    let reopened = RootStore::open(ld.path(), true).unwrap();
    let reconcile = project
        .paea_settlement_request(&reopened, &p, &historical_context, Operation::Reconcile)
        .unwrap();
    let observed = recorded(
        invoke(&project, &reopened, &reconcile, &scope).unwrap(),
        true,
    );
    assert_eq!(observed.commit_requested_at, first.commit_requested_at);
    assert_eq!(count(ld.path()), 1);
    reconcile.close().unwrap();
    reopened.close().unwrap();
    project.close().unwrap();
}

/// Never infer permission from another root, and keep known previous effects when
/// changed canonical content attempts the same business invoice under a new grant.
#[test]
fn settlement_rejects_wrong_roots_and_preserves_previous_effect() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (_od, other) = root();
    let (p, c) = action();
    for store in [&ledger, &other] {
        store
            .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
            .unwrap();
    }
    let request = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &request, &scope).unwrap(), false);
    match project.paea_settlement_request(&project, &p, &c, Operation::Commit) {
        Err(e) => assert_eq!(e.code, "paea-settlement-separate-ledger-required"),
        Ok(_) => panic!("same storage adopted as separate service"),
    }
    let e = invoke(&project, &other, &request, &scope).unwrap_err();
    assert_eq!(e.code, "paea-settlement-binding");
    assert!(matches!(e.observation, Observation::NotAttempted));
    assert_eq!(count(ld.path()), 1);
    let mut changed = p.clone();
    if let Action::Payment { amount_minor, .. } = &mut changed.action {
        *amount_minor = "430000".into();
    }
    changed.request_id = "new-key-new-content".into();
    let request2 = project
        .paea_settlement_request(&ledger, &changed, &c, Operation::Commit)
        .unwrap();
    let e = invoke(&project, &ledger, &request2, &scope).unwrap_err();
    assert_eq!(e.code, "paea-settlement-business-intent-conflict");
    match e.observation {
        Observation::Known(old) => {
            recorded(*old, true);
        }
        _ => panic!("lost old effect"),
    };
    assert_eq!(count(ld.path()), 1);
    recorded(invoke(&project, &ledger, &request, &scope).unwrap(), true);
    request2.close().unwrap();
    request.close().unwrap();
    other.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Explicit negative reconciliation is a typed observation, never an empty
/// success; wrong debt terms/unknown invoice refuse without publishing a row.
#[test]
fn settlement_debt_refusal_negative_reconciliation_and_recovery() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let negative = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Reconcile)
        .unwrap();
    match invoke(&project, &ledger, &negative, &scope).unwrap() {
        Outcome::ReconciledNotCommitted {
            intent_id,
            invoice_id,
        } => {
            assert_eq!(intent_id, negative.canonical().intent_id());
            assert_eq!(invoice_id, "invoice-17");
        }
        _ => panic!("negative observation missing"),
    };
    assert_eq!(count(ld.path()), 0);
    negative.close().unwrap();
    for unknown in [false, true] {
        let mut bad = p.clone();
        if let Action::Payment {
            invoice_id,
            invoice_version,
            ..
        } = &mut bad.action
        {
            if unknown {
                *invoice_id = "unknown-invoice".into();
            } else {
                *invoice_version = "stale-version".into();
            }
        }
        let req = project
            .paea_settlement_request(&ledger, &bad, &c, Operation::Commit)
            .unwrap();
        let e = invoke(&project, &ledger, &req, &scope).unwrap_err();
        assert_eq!(e.code, "paea-settlement-debt-incompatible");
        assert_eq!(count(ld.path()), 0);
        req.close().unwrap();
    }
    let good = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &good, &scope).unwrap(), false);
    assert_eq!(count(ld.path()), 1);
    good.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Public plan mutation invalidates a still-signed request. Fresh scope/grant
/// recovers without overwriting the original known invoice publication.
#[test]
fn settlement_rechecks_real_current_plan_and_recovers() {
    let (_pd, mut project, old_scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let req = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &req, &old_scope).unwrap(), false);
    project.execute_external(&old_scope.work_id,"task.add",br#"{"schema_version":1,"event_id":"new-plan","expected_revision":1,"id":"task","title":"Changed plan","deliverable":"Proof","criteria":["Fresh scope"]}"#,&mut SystemEnvironment).unwrap();
    let e = invoke(&project, &ledger, &req, &old_scope).unwrap_err();
    assert_eq!(e.code, "paea-settlement-plan-stale");
    assert_eq!(count(ld.path()), 1);
    let fresh_scope = scope(_pd.path(), &old_scope.work_id);
    recorded(invoke(&project, &ledger, &req, &fresh_scope).unwrap(), true);
    req.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// First a valid call; then revoke a separately issued exact second-invoice
/// grant before any effect use. Both stale and current revoked heads refuse.
#[test]
fn settlement_revocation_before_first_effect_consumption_refuses() {
    use ed25519_dalek::SigningKey;
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    let mut second = p.clone();
    if let Action::Payment { invoice_id, .. } = &mut second.action {
        *invoice_id = "second-invoice".into();
    }
    ledger
        .paea_initialize_settlement(
            &c.project_id,
            &"0b".repeat(32),
            &[p.clone(), second.clone()],
        )
        .unwrap();
    let control = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &control, &scope).unwrap(), false);
    control.close().unwrap();
    let req = project
        .paea_settlement_request(&ledger, &second, &c, Operation::Commit)
        .unwrap();
    let f = fixture::Fixture::build_for_trust(req.request().permission(), scope, 1);
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let raw = sign_change(
        &SigningKey::from_bytes(&[7; 32]),
        [11; 32],
        &head,
        "revoke",
        "business-grant",
        &[],
    )
    .unwrap();
    let revoked = f.business.apply(&Context::new(), &head, &raw).unwrap();
    for h in [&head, &revoked] {
        let result = app
            .with_paea_action(
                &UseContext::new(),
                h,
                b"STARTUP_FIXTURE_PASSWORD",
                req.request(),
                |_| -> Result<(), ()> { panic!("revoked grant reached effect") },
            )
            .unwrap();
        assert!(result.authority.is_err());
        assert!(result.action.is_none());
    }
    assert_eq!(count(ld.path()), 1);
    app.close().unwrap();
    f.close();
    req.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Real SQLITE_BUSY at COMMIT, explicit rollback and negative reconciliation.
/// The timeout is a saved Admin setting (0ms fixture), never a new product bound.
#[test]
fn settlement_commit_uncertainty_reconciles_then_recovers() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    let mut second = p.clone();
    if let Action::Payment { invoice_id, .. } = &mut second.action {
        *invoice_id = "second-invoice".into();
    }
    let document = ledger.settings().unwrap();
    let mut values = document.values.clone();
    values.storage_busy_timeout_ms = swarm_engine::SettingCount(0);
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: document.schema_version,
        event_id: "zero-busy-fixture".into(),
        expected_revision: document.revision,
        values,
        reason: "Immediate observable contention fixture".into(),
        preview_token: String::new(),
    };
    change.preview_token = ledger.preview_settings(&change).unwrap().preview_token;
    ledger
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    ledger
        .paea_initialize_settlement(
            &c.project_id,
            &"0b".repeat(32),
            &[p.clone(), second.clone()],
        )
        .unwrap();
    let control = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &control, &scope).unwrap(), false);
    control.close().unwrap();
    let req = project
        .paea_settlement_request(&ledger, &second, &c, Operation::Commit)
        .unwrap();
    let reader = rusqlite::Connection::open_with_flags(
        ld.path().join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mode: String = reader
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        mode, "delete",
        "real COMMIT contention fixture requires rollback journal"
    );
    reader.execute_batch("BEGIN").unwrap();
    let n: i64 = reader
        .query_row("SELECT count(*) FROM paea_settlement_effects", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 1);
    let e = invoke(&project, &ledger, &req, &scope).unwrap_err();
    assert_eq!(e.code, "paea-settlement-commit-uncertain");
    assert!(matches!(e.observation, Observation::Uncertain));
    assert_eq!(
        e.cause
            .downcast_ref::<rusqlite::Error>()
            .unwrap()
            .sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy)
    );
    assert!(e.cleanup.is_empty());
    call_wrappers(swarm_engine::PaeaSettlementCallError::Action(e), None, true);
    reader.execute_batch("ROLLBACK").unwrap();
    reader.close().unwrap();
    assert_eq!(count(ld.path()), 1);
    let reconcile = project
        .paea_settlement_request(&ledger, &second, req.context(), Operation::Reconcile)
        .unwrap();
    assert!(matches!(
        invoke(&project, &ledger, &reconcile, &scope).unwrap(),
        Outcome::ReconciledNotCommitted { .. }
    ));
    reconcile.close().unwrap();
    recorded(invoke(&project, &ledger, &req, &scope).unwrap(), false);
    assert_eq!(count(ld.path()), 2);
    req.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Stored role removal and configuration revision changes cannot be bypassed by
/// a still-valid signed grant; explicit restored current mapping recovers.
#[test]
fn settlement_current_trust_revision_and_role_refuse_then_recover() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let req = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &req, &scope).unwrap(), false);
    let view: Value =
        serde_json::from_slice(&project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let mut ids = view["identities"].clone();
    for id in ids.as_array_mut().unwrap() {
        if id["enrollment_principal"].is_null() {
            id["token"] = format!("fixture-{}-token", id["subject"].as_str().unwrap()).into();
        }
    }
    let restored_ids = ids.clone();
    ids[0]["disabled"] = true.into();
    let change = json!({"schema_version":1,"event_id":"disable-effect","expected_revision":1,"reason":"Explicit fixture disabled effect mapping","policy":view["policy"],"identities":ids});
    project
        .trust("config-apply", &serde_json::to_vec(&change).unwrap(), "")
        .unwrap();
    let stale = invoke(&project, &ledger, &req, &scope).unwrap_err();
    assert_eq!(stale.code, "paea-settlement-trust-revision");
    let disabled = invoke_revision(&project, &ledger, &req, &scope, 2).unwrap_err();
    assert_eq!(disabled.code, "paea-settlement-effect-role");
    assert_eq!(count(ld.path()), 1);
    let restore = json!({"schema_version":1,"event_id":"restore-effect","expected_revision":2,"reason":"Explicit fixture restored effect mapping","policy":view["policy"],"identities":restored_ids});
    project
        .trust("config-apply", &serde_json::to_vec(&restore).unwrap(), "")
        .unwrap();
    let reconcile = project
        .paea_settlement_request(&ledger, &p, req.context(), Operation::Reconcile)
        .unwrap();
    recorded(
        invoke_revision(&project, &ledger, &reconcile, &scope, 3).unwrap(),
        true,
    );
    reconcile.close().unwrap();
    assert_eq!(count(ld.path()), 1);
    req.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Unsupported schema extras and a malformed receipt refuse rather than produce
/// an empty success. Restore exact fixture inputs between faults and recover.
#[test]
fn settlement_schema_and_receipt_corruption_refuse_then_recover() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let req = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &req, &scope).unwrap(), false);
    let db = rusqlite::Connection::open(ld.path().join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER corrupt_settlement BEFORE INSERT ON paea_settlement_effects BEGIN SELECT RAISE(ABORT,'fixture trigger'); END").unwrap();
    let error = invoke(&project, &ledger, &req, &scope).unwrap_err();
    assert_eq!(error.code, "paea-settlement-schema");
    assert_eq!(count(ld.path()), 1);
    db.execute_batch("DROP TRIGGER corrupt_settlement").unwrap();
    recorded(invoke(&project, &ledger, &req, &scope).unwrap(), true);
    let raw: String = db
        .query_row("SELECT receipt FROM paea_settlement_effects", [], |r| {
            r.get(0)
        })
        .unwrap();
    db.execute("UPDATE paea_settlement_effects SET receipt='{}'", [])
        .unwrap();
    let error = invoke(&project, &ledger, &req, &scope).unwrap_err();
    assert_eq!(error.code, "paea-settlement-receipt-decode");
    assert_eq!(count(ld.path()), 1);
    db.execute("UPDATE paea_settlement_effects SET receipt=?", [raw])
        .unwrap();
    db.close().unwrap();
    recorded(invoke(&project, &ledger, &req, &scope).unwrap(), true);
    req.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Two independently authorized connections compete on one real business intent.
/// Per-worker readiness/release channels avoid sleeps and global deadlines; all
/// workers are joined before assertions, including a failed readiness/release.
#[test]
fn settlement_two_current_authorities_publish_exactly_once() {
    let (pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let mut workers = Vec::new();
    let mut ready = Vec::new();
    let mut releases = Vec::new();
    for index in 0..2 {
        let project_path = pd.path().to_path_buf();
        let ledger_path = ld.path().to_path_buf();
        let mut proposal = p.clone();
        proposal.request_id = format!("concurrent-transport-{index}");
        let context = c.clone();
        let scope = scope.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        ready.push(ready_rx);
        releases.push(release_tx);
        workers.push(std::thread::spawn(move || {
            let project = RootStore::open(&project_path, true).unwrap();
            let ledger = RootStore::open(&ledger_path, true).unwrap();
            let req = project
                .paea_settlement_request(&ledger, &proposal, &context, Operation::Commit)
                .unwrap();
            let f = fixture::Fixture::build_for_trust(req.request().permission(), scope, 1);
            let (app, head) =
                authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin)
                    .unwrap();
            ready_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            let used = app
                .with_paea_action(
                    &UseContext::new(),
                    &head,
                    b"STARTUP_FIXTURE_PASSWORD",
                    req.request(),
                    |identity| ledger.paea_settle(&project, &req, identity),
                )
                .unwrap();
            app.close().unwrap();
            f.close();
            req.close().unwrap();
            ledger.close().unwrap();
            project.close().unwrap();
            used
        }));
    }
    let readiness: Vec<_> = ready.into_iter().map(|r| r.recv()).collect();
    let releases_result: Vec<_> = if readiness.iter().all(Result::is_ok) {
        releases.into_iter().map(|s| s.send(())).collect()
    } else {
        drop(releases);
        Vec::new()
    };
    let joined: Vec<_> = workers.into_iter().map(|w| w.join()).collect();
    assert!(readiness.iter().all(Result::is_ok));
    assert!(releases_result.iter().all(Result::is_ok));
    let mut recorded_count = 0;
    let mut replay_count = 0;
    let mut digests = std::collections::BTreeSet::new();
    for result in joined {
        let used = result.unwrap();
        assert!(used.authority.is_ok());
        match used.action.unwrap().unwrap() {
            Outcome::Recorded { receipt, replayed } => {
                digests.insert(receipt.action_digest);
                if replayed {
                    replay_count += 1;
                } else {
                    recorded_count += 1;
                }
            }
            _ => panic!("concurrent commitment returned absence"),
        }
    }
    assert_eq!((recorded_count, replay_count), (1, 1));
    assert_eq!(digests.len(), 1);
    assert_eq!(count(ld.path()), 1);
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Exercise the established concrete Box/custom-IO/Root and secondary-cleanup
/// matrix without relying on source() alone or rendering any receipt/password.
fn call_wrappers(
    error: swarm_engine::PaeaSettlementCallError,
    expected: Option<&str>,
    uncertain: bool,
) {
    fn check(error: &(dyn std::error::Error + 'static), expected: Option<&str>, uncertain: bool) {
        let typed = swarm_engine::PaeaSettlementCallError::find(error)
            .expect("typed PAEA envelope must survive supported wrapper");
        assert_eq!(typed.commit_uncertain(), uncertain);
        match (expected, typed.known_outcome()) {
            (
                Some(digest),
                Some(Outcome::Recorded {
                    receipt,
                    replayed: false,
                }),
            ) => assert_eq!(receipt.action_digest, digest),
            (None, None) => (),
            _ => panic!("known result changed through error wrapper"),
        }
        let safe = format!("{typed:?}");
        assert!(!safe.contains("STARTUP_FIXTURE_PASSWORD"));
        assert!(!safe.contains("second-invoice"));
    }
    check(&error, expected, uncertain);
    let boxed = Box::new(error);
    check(&boxed, expected, uncertain);
    let io = std::io::Error::other(boxed);
    check(&io, expected, uncertain);
    let root = swarm_engine::RootStoreError::SqlTransaction(Box::new(io));
    check(&root, expected, uncertain);
    let boxed = Box::new(root);
    check(&boxed, expected, uncertain);
    let secondary = swarm_engine::RootStoreError::Cleanup {
        primary: Some(Box::new(swarm_engine::RootStoreError::Refused(
            "fixture-primary",
        ))),
        cleanup: boxed,
    };
    check(&secondary, expected, uncertain);
    let io = std::io::Error::other(Box::new(secondary));
    check(&io, expected, uncertain);
    let root = swarm_engine::RootStoreError::SqlTransaction(Box::new(io));
    check(&root, expected, uncertain);
}

/// Real ledger publication followed by cancellation of both retained authority
/// layers. Preserve the real AfterUse through the same Personal adapter variant
/// used by call_paea_settlement, then reconcile without a third publication.
#[test]
fn settlement_known_effect_survives_after_use_and_supported_wrappers() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    let mut second = p.clone();
    if let Action::Payment { invoice_id, .. } = &mut second.action {
        *invoice_id = "second-invoice".into();
    }
    ledger
        .paea_initialize_settlement(
            &c.project_id,
            &"0b".repeat(32),
            &[p.clone(), second.clone()],
        )
        .unwrap();
    let control = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &control, &scope).unwrap(), false);
    control.close().unwrap();
    let req = project
        .paea_settlement_request(&ledger, &second, &c, Operation::Commit)
        .unwrap();
    let f = fixture::Fixture::build_for_trust(req.request().permission(), scope.clone(), 1);
    let (app, head) =
        authenticated_startup::open(&Context::new(), f.path.to_str().unwrap(), &f.pin).unwrap();
    let ctx = UseContext::new();
    let raw = app.with_paea_action(
        &ctx,
        &head,
        b"STARTUP_FIXTURE_PASSWORD",
        req.request(),
        |identity| {
            let result = ledger.paea_settle(&project, &req, identity);
            ctx.cancel();
            result
        },
    );
    let error = match raw {
        Err(error) => swarm_engine::PaeaSettlementCallError::Personal(error),
        Ok(_) => panic!("actual after-use cancellation did not refuse"),
    };
    assert!(error.branches().len() >= 2);
    assert_eq!(count(ld.path()), 2);
    call_wrappers(error, Some(req.canonical().digest()), false);
    app.close().unwrap();
    f.close();
    let historical_context = req.context().clone();
    req.close().unwrap();
    let reconcile = project
        .paea_settlement_request(&ledger, &second, &historical_context, Operation::Reconcile)
        .unwrap();
    recorded(invoke(&project, &ledger, &reconcile, &scope).unwrap(), true);
    assert_eq!(count(ld.path()), 2);
    reconcile.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Change only explicit fixture policy through the public operator configuration
/// path, retaining tokens only for local fixture identities, never enrolled ones.
fn fixture_policy(project: &RootStore, require_separation: bool, role_separation: bool) -> i64 {
    let view: Value =
        serde_json::from_slice(&project.trust("show", b"{}", "").unwrap().bytes).unwrap();
    let mut ids = view["identities"].clone();
    for id in ids.as_array_mut().unwrap() {
        if id["enrollment_principal"].is_null() {
            id["token"] = format!("fixture-{}-token", id["subject"].as_str().unwrap()).into();
        }
    }
    let mut policy = view["policy"].clone();
    policy["require_separation"] = require_separation.into();
    let revision = view["revision"].as_i64().unwrap();
    let change = json!({"schema_version":1,"event_id":format!("fixture-policy-{}",revision+1),"expected_revision":revision,"reason":"Explicit fixture separation policy transition","policy":policy,"identities":ids,"role_separation":{"enabled":role_separation,"incompatible_roles":[["requester","approver"]],"principals":{"enrolled-effect":"effect-person","requester":"requester-person","approver":"approver-person"}}});
    project
        .trust("config-apply", &serde_json::to_vec(&change).unwrap(), "")
        .unwrap();
    revision + 1
}

/// A reference-profile control precedes each policy fault. Without attributed
/// participants/approval the new service must refuse enabled separation, never
/// silently bypass either persisted policy. Reconciliation remains read-only.
#[test]
fn settlement_enabled_separation_requires_attributed_admission() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    let mut second = p.clone();
    let mut third = p.clone();
    if let Action::Payment { invoice_id, .. } = &mut second.action {
        *invoice_id = "second-invoice".into();
    }
    if let Action::Payment { invoice_id, .. } = &mut third.action {
        *invoice_id = "third-control-invoice".into();
    }
    ledger
        .paea_initialize_settlement(
            &c.project_id,
            &"0b".repeat(32),
            &[p.clone(), second.clone(), third.clone()],
        )
        .unwrap();
    let mut revision = fixture_policy(&project, false, false);
    for (index, (require_separation, role_separation)) in
        [(true, false), (false, true)].into_iter().enumerate()
    {
        let control = project
            .paea_settlement_request(
                &ledger,
                if index == 0 { &p } else { &third },
                &c,
                Operation::Commit,
            )
            .unwrap();
        recorded(
            invoke_revision(&project, &ledger, &control, &scope, revision).unwrap(),
            false,
        );
        control.close().unwrap();
        let before = count(ld.path());
        revision = fixture_policy(&project, require_separation, role_separation);
        let blocked = project
            .paea_settlement_request(&ledger, &second, &c, Operation::Commit)
            .unwrap();
        let error = invoke_revision(&project, &ledger, &blocked, &scope, revision).unwrap_err();
        assert_eq!(error.code, "paea-settlement-attributed-admission-required");
        assert_eq!(count(ld.path()), before);
        let read = project
            .paea_settlement_request(&ledger, &second, blocked.context(), Operation::Reconcile)
            .unwrap();
        assert!(matches!(
            invoke_revision(&project, &ledger, &read, &scope, revision).unwrap(),
            Outcome::ReconciledNotCommitted { .. }
        ));
        read.close().unwrap();
        blocked.close().unwrap();
        revision = fixture_policy(&project, false, false);
    }
    let recovered = project
        .paea_settlement_request(&ledger, &second, &c, Operation::Commit)
        .unwrap();
    recorded(
        invoke_revision(&project, &ledger, &recovered, &scope, revision).unwrap(),
        false,
    );
    assert_eq!(count(ld.path()), 3);
    recovered.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// Policy fields come from the real configured project, not the declared input.
/// A newer configuration refuses a previously prepared commitment even with a
/// fresh grant; historical reconciliation and newly prepared reference use work.
#[test]
fn settlement_policy_attribution_freshness_and_historical_reconciliation() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    let original = c.clone();
    let mut second = p.clone();
    if let Action::Payment { invoice_id, .. } = &mut second.action {
        *invoice_id = "second-invoice".into();
    }
    ledger
        .paea_initialize_settlement(
            &c.project_id,
            &"0b".repeat(32),
            &[p.clone(), second.clone()],
        )
        .unwrap();
    let old = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    assert_eq!(c, original);
    assert_eq!(old.context().policy_revision, 1);
    assert_ne!(old.context().policy_digest, c.policy_digest);
    assert_eq!(old.context().sources, c.sources);
    assert_eq!(old.context().principal_id, c.principal_id);
    recorded(invoke(&project, &ledger, &old, &scope).unwrap(), false);
    let revision = fixture_policy(&project, false, false);
    let stale = invoke_revision(&project, &ledger, &old, &scope, revision).unwrap_err();
    assert_eq!(stale.code, "paea-settlement-policy-stale");
    assert_eq!(count(ld.path()), 1);
    let historical = project
        .paea_settlement_request(&ledger, &p, old.context(), Operation::Reconcile)
        .unwrap();
    assert_eq!(historical.canonical().digest(), old.canonical().digest());
    recorded(
        invoke_revision(&project, &ledger, &historical, &scope, revision).unwrap(),
        true,
    );
    historical.close().unwrap();
    let fresh = project
        .paea_settlement_request(&ledger, &second, &c, Operation::Commit)
        .unwrap();
    assert_eq!(fresh.context().policy_revision, revision);
    assert_ne!(fresh.context().policy_digest, old.context().policy_digest);
    recorded(
        invoke_revision(&project, &ledger, &fresh, &scope, revision).unwrap(),
        false,
    );
    assert_eq!(count(ld.path()), 2);
    let mut bad = c.clone();
    bad.policy_digest = "invalid-digest".into();
    match project.paea_settlement_request(&ledger, &second, &bad, Operation::Commit) {
        Err(e) => assert_eq!(e.code, "paea-settlement-context"),
        Ok(_) => panic!("invalid declared representation silently normalized"),
    };
    assert_eq!(count(ld.path()), 2);
    fresh.close().unwrap();
    old.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}

/// A real stored effect is the control before each isolated metadata corruption.
/// Restore exact original bytes and observe recovery; no live datastore is used.
#[test]
fn settlement_receipt_metadata_refuses_then_recovers() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let req = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    recorded(invoke(&project, &ledger, &req, &scope).unwrap(), false);
    let db = rusqlite::Connection::open(ld.path().join(".swarm/state.db")).unwrap();
    let raw: String = db
        .query_row("SELECT receipt FROM paea_settlement_effects", [], |r| {
            r.get(0)
        })
        .unwrap();
    let mut accepted = Vec::new();
    for variant in ["enrollment", "timestamp", "project", "policy"] {
        recorded(invoke(&project, &ledger, &req, &scope).unwrap(), true);
        let mut changed: Value = serde_json::from_str(&raw).unwrap();
        match variant {
            "enrollment" => changed["authority_enrollment_id"] = "".into(),
            "timestamp" => changed["commit_requested_at"] = "not-a-timestamp".into(),
            "project" => changed["authority_project_digest"] = "ab".repeat(32).into(),
            "policy" => {
                changed["trust_revision"] = (changed["trust_revision"].as_i64().unwrap() + 1).into()
            }
            _ => unreachable!(),
        }
        db.execute(
            "UPDATE paea_settlement_effects SET receipt=?",
            [serde_json::to_string(&changed).unwrap()],
        )
        .unwrap();
        match invoke(&project, &ledger, &req, &scope) {
            Ok(_) => accepted.push(variant),
            Err(e) => assert_eq!(e.code, "paea-settlement-receipt-incompatible"),
        }
        assert_eq!(count(ld.path()), 1);
        db.execute("UPDATE paea_settlement_effects SET receipt=?", [&raw])
            .unwrap();
        recorded(invoke(&project, &ledger, &req, &scope).unwrap(), true);
    }
    db.close().unwrap();
    req.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
    assert!(
        accepted.is_empty(),
        "incoherent metadata accepted: {accepted:?}"
    );
}

/// The declared tool manifest is transported, not engine-attributed: settlement
/// consumes no provider catalogue (the admitted-tool barrier is the agent-launch
/// preflight, Increment101), so the effective binding is canonical-bytes→signed
/// permission→frozen prepared request→existing-intent refusal. A manifest change
/// after preparation yields a different action digest for the same business
/// intent and the recorded outcome refuses any reauthorization without a new
/// effect; reconciliation retains the historical declared field. This proves the
/// honest linkage and its limit: no engine recomputation, no ambient-provider
/// barrier is claimed for this field.
#[test]
fn settlement_declared_tool_manifest_binding_and_existing_intent_refusal() {
    let (_pd, project, scope) = project();
    let (ld, ledger) = root();
    let (p, c) = action();
    assert_eq!(c.tool_manifest_digest, "b".repeat(64));
    ledger
        .paea_initialize_settlement(&c.project_id, &"0b".repeat(32), std::slice::from_ref(&p))
        .unwrap();
    let witness = project
        .paea_settlement_request(&ledger, &p, &c, Operation::Commit)
        .unwrap();
    // Declaration is preserved verbatim at preparation (no engine attribution),
    // while the canonical digest — which includes the manifest — is frozen into
    // the exact signed permission resource and the in-memory prepared request.
    assert_eq!(witness.context().tool_manifest_digest, c.tool_manifest_digest);
    let witness_digest = witness.canonical().digest().to_owned();
    let resource = witness.request().permission().resource;
    assert!(
        resource.contains(&format!("/commit/{witness_digest}")),
        "signed permission must bind the canonical digest containing the manifest"
    );
    let receipt = recorded(invoke(&project, &ledger, &witness, &scope).unwrap(), false);
    assert_eq!(receipt.action_digest, witness_digest);
    assert_eq!(count(ld.path()), 1);
    // Mutating only the manifest after publication: re-preparation is possible
    // (declaration stays validated form-only), but the new canonical digest for
    // the same business intent refuses against the recorded outcome with no new
    // effect and the old known receipt retained — the consumption-time barrier.
    let mut changed = c.clone();
    changed.tool_manifest_digest = "c".repeat(64);
    let attempt = project
        .paea_settlement_request(&ledger, &p, &changed, Operation::Commit)
        .unwrap();
    assert_eq!(attempt.context().tool_manifest_digest, changed.tool_manifest_digest);
    assert_ne!(attempt.canonical().digest(), witness_digest);
    assert_eq!(attempt.canonical().intent_id(), witness.canonical().intent_id());
    let error = invoke(&project, &ledger, &attempt, &scope).unwrap_err();
    assert_eq!(error.code, "paea-settlement-business-intent-conflict");
    match error.observation {
        Observation::Known(old) => {
            assert_eq!(recorded(*old, true).action_digest, witness_digest);
        }
        _ => panic!("lost recorded outcome on manifest mutation"),
    }
    assert_eq!(count(ld.path()), 1);
    // Reconciliation keeps the historical declared manifest and replays the
    // recorded effect without any new commitment.
    let historical = project
        .paea_settlement_request(&ledger, &p, witness.context(), Operation::Reconcile)
        .unwrap();
    assert_eq!(
        historical.context().tool_manifest_digest,
        c.tool_manifest_digest
    );
    assert_eq!(historical.canonical().digest(), witness_digest);
    recorded(invoke(&project, &ledger, &historical, &scope).unwrap(), true);
    assert_eq!(count(ld.path()), 1);
    attempt.close().unwrap();
    historical.close().unwrap();
    witness.close().unwrap();
    ledger.close().unwrap();
    project.close().unwrap();
}
