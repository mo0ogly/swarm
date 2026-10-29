//! Two-phase real CLI fixture for the guarded PAEA fictitious settlement
//! second domain, composed through one externally designated delivered binary
//! (see `delivered_binary`): one actual project Root with a real plan and Trust
//! configuration, one physically separate ledger Root provisioned through the
//! documented fixture initialization (`paea_initialize_settlement`), then the
//! public CLI `paea settlement-call` commit, an identical replay and the
//! read-only `paea reconcile-effects` report — every business spawn targeting
//! the same designated executable. Fictitious SQLite effects only; no
//! protected daemon, real payment, UID separation, human presence or accepted
//! service is claimed. The authority stays the existing explicitly simulated
//! enrollment fixture. Without `SWARM_PAEA_DELIVERED_BIN` the harness runs the
//! cargo-launched binary exactly like the settlement harness
//! (`tests/paea_settlement.rs`), which owns the same CLI contract; this file
//! adds only the delivered-binary composition and its witnesses.
#![cfg(target_os = "linux")]
#[path = "../../swarm-enrollment-authority/tests/common/authenticated_fixture.rs"]
mod fixture;
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Barrier},
};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_identity_contract::Scope;
use swarm_protection_contract::{AdmissionContext, Proposal};

/// Canonical interchange fixture remains declared data, never admission
/// evidence; the same retained document feeds the ledger initialization and
/// the CLI's `--input` request file (motif of `tests/paea_settlement.rs`).
fn document() -> (Proposal, AdmissionContext, Vec<u8>) {
    let raw: &[u8] = include_bytes!("../../../../web/paea-examples/v1/payment-document.json");
    let doc = swarm_protection_contract::decode_document(raw).unwrap();
    (doc.proposal, doc.context, raw.to_vec())
}

/// Create one owned native Root; no existing user/project database is opened.
fn root() -> (tempfile::TempDir, RootStore) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let store = RootStore::initialize(dir.path(), &mut SystemEnvironment).unwrap();
    (dir, store)
}

/// Actual public work.create and persisted plan stamp, no fabricated generation
/// (motif of `tests/paea_settlement.rs::project`).
fn project() -> (tempfile::TempDir, RootStore, Scope) {
    let (dir, mut store) = root();
    let work = store
        .execute_external(
            "",
            "work.create",
            br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Delivered settlement plan","objective":"Exact fictitious effect","scope":"Fixture","criteria":["No duplicate invoice"]}"#,
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work
        .id;
    let config = serde_json::json!({
        "schema_version":1,"event_id":"effect-config","expected_revision":0,
        "reason":"Explicit fixture effect role",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":[
            {"subject":"enrolled-effect","integrity":3,"roles":["admin","effect"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}},
            {"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},
            {"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}
        ]
    });
    store
        .trust("config-apply", &serde_json::to_vec(&config).unwrap(), "")
        .unwrap_or_else(|e| panic!("explicit fixture config refused: {}", e.code()));
    let fixture_scope = scope(dir.path(), &work);
    (dir, store, fixture_scope)
}

/// Read only a fixture plan, finalizing the independent connection before use.
fn scope(dir: &Path, work: &str) -> Scope {
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

/// Read the current Trust revision from the project's own bounded state file.
fn current_trust_revision(root: &Path) -> i64 {
    let state: Value =
        serde_json::from_slice(&fs::read(root.join(".swarm/trust-actions.json")).unwrap())
            .unwrap();
    state["revision"].as_i64().expect("fixture Trust revision")
}

/// Count actual durable ledger effects through a separately opened read-only DB.
fn count(dir: &Path) -> i64 {
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

/// Read one provisioned debt's exact terms through a separately opened
/// read-only DB; `None` proves the debt row is absent rather than assuming it.
fn debt_for(dir: &Path, invoice: &str) -> Option<(String, i64, String, String)> {
    let db = rusqlite::Connection::open_with_flags(
        dir.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let row = db
        .query_row(
            "SELECT invoice_version,amount_minor,currency,beneficiary FROM paea_settlement_debts WHERE invoice_id=?",
            [invoice],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    db.close().unwrap();
    row
}

/// The original provisioned debt's terms (`invoice-17`).
fn debt(dir: &Path) -> Option<(String, i64, String, String)> {
    debt_for(dir, "invoice-17")
}

/// Read the single durable effect row's exact columns through a separately
/// opened read-only DB; the receipt JSON stays producer bytes, never a
/// harness-invented shape.
fn effect(dir: &Path) -> (String, String, String, Value) {
    let db = rusqlite::Connection::open_with_flags(
        dir.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (intent_id, invoice_id, action_digest, raw): (String, String, String, String) = db
        .query_row(
            "SELECT intent_id,invoice_id,action_digest,receipt FROM paea_settlement_effects",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    db.close().unwrap();
    let receipt: Value = serde_json::from_str(&raw).unwrap();
    (intent_id, invoice_id, action_digest, receipt)
}

/// Full owned fixture set for one delivered CLI settlement run: the actual
/// project Root (writable, plan scope read, Trust applied), the physically
/// separate ledger Root provisioned for the document's declared project with
/// the fictitious debt terms, and the exact request document bytes. All owners
/// must be explicitly closed by the caller.
struct Owned {
    project_dir: tempfile::TempDir,
    project: RootStore,
    ledger_dir: tempfile::TempDir,
    ledger: RootStore,
    trust_revision: i64,
    scope: Scope,
    document: Vec<u8>,
}

fn owned() -> Owned {
    let (project_dir, project, fixture_scope) = project();
    let trust_revision = current_trust_revision(project_dir.path());
    let (ledger_dir, ledger) = root();
    let (proposal, context, raw) = document();
    // Documented fixture provisioning path only: the library initialization is
    // the supported way to create the fictitious ledger (debts provisioned
    // before any CLI call; never adopted from first-request data).
    ledger
        .paea_initialize_settlement(
            &context.project_id,
            &"0b".repeat(32),
            std::slice::from_ref(&proposal),
        )
        .unwrap();
    Owned {
        project_dir,
        project,
        ledger_dir,
        ledger,
        trust_revision,
        scope: fixture_scope,
        document: raw,
    }
}

/// CLI binary exercised by the delivered settlement chain. Motif of
/// `tests/paea_promote.rs::delivered_binary` (external-test hash guard), kept
/// as a bounded local copy instead of a wide refactor of that harness: by
/// default every spawn targets the cargo-launched `CARGO_BIN_EXE_swarm-rust`
/// and the default is never inferred or replaced. When an explicit test
/// environment designates a delivered production binary via
/// `SWARM_PAEA_DELIVERED_BIN`, that exact file is used instead and its actual
/// SHA-256 must equal the mandatory `SWARM_PAEA_DELIVERED_BIN_SHA256` witness
/// at every spawn: a silently swapped or rebuilt binary refuses the chain.
/// Test-environment selection only: no product setting, no build-profile
/// qualification and no claim that the designated file was produced by a
/// particular compilation or delivery are made here.
fn delivered_binary() -> Command {
    match std::env::var_os("SWARM_PAEA_DELIVERED_BIN") {
        None => Command::new(env!("CARGO_BIN_EXE_swarm-rust")),
        Some(path) => {
            use sha2::Digest;
            let expected = std::env::var("SWARM_PAEA_DELIVERED_BIN_SHA256").expect(
                "SWARM_PAEA_DELIVERED_BIN_SHA256 must accompany SWARM_PAEA_DELIVERED_BIN",
            );
            let raw = fs::read(&path).expect("designated delivered binary readable");
            let actual = format!("{:x}", sha2::Sha256::digest(&raw));
            assert_eq!(
                actual, expected,
                "designated delivered binary hash mismatch: {path:?}"
            );
            println!(
                "delivered-binary-witness path={} sha256={actual}",
                path.to_string_lossy()
            );
            Command::new(path)
        }
    }
}

/// Full two-phase delivered CLI invocation of the settlement operation: the
/// prepared event names the exact permission, the fixture mints it into a
/// declaration, then the frame answers with the fixture pin and personal
/// password. `bad_frame` replaces the frame with a wrong header so the refusal
/// happens before any consumption. The exact spawn argv is printed as a
/// witness (it carries no secret: the settlement command has no pin argument
/// and the password never enters argv). Outputs: exit status, stdout after the
/// prepared event, stderr. Refusals stay the CLI's own; none is hidden.
fn invoke(owned: &Owned, operation: &str, bad_frame: bool) -> (std::process::ExitStatus, String, String) {
    invoke_between(owned, operation, bad_frame, owned.trust_revision, &|| {})
}

/// Same real two-phase invocation with the provisioning window explicitly
/// open: `between` runs after the public prepared event and before the private
/// frame, while the CLI process holds both prepared physical witnesses and no
/// reservation, mirroring an out-of-band change of the current authority. The
/// provisioned grant binds the explicitly supplied current Trust revision, not
/// the preparation-time one; no other parameter changes. Motif of
/// `tests/paea_promote.rs::invoke_between` on the settlement operation.
fn invoke_between(
    owned: &Owned,
    operation: &str,
    bad_frame: bool,
    revision: i64,
    between: &dyn Fn(),
) -> (std::process::ExitStatus, String, String) {
    let project_dir = owned.project_dir.path();
    let ledger_dir = owned.ledger_dir.path();
    let input = ledger_dir.join("settlement-request.json");
    fs::write(&input, &owned.document).unwrap();
    let declaration_dir = ledger_dir.join("operator-declaration");
    fs::create_dir_all(&declaration_dir).unwrap();
    fs::set_permissions(&declaration_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let declaration = declaration_dir.join("declaration.json");
    let args = [
        "--json",
        "paea",
        "settlement-call",
        operation,
        project_dir.to_str().unwrap(),
        ledger_dir.to_str().unwrap(),
        declaration.to_str().unwrap(),
        "--input",
        input.to_str().unwrap(),
    ];
    println!("delivered-settlement-argv {}", args.join(" "));
    let mut child = delivered_binary()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut first = String::new();
    let read = stdout.read_line(&mut first).unwrap();
    if read == 0 {
        let output = child.wait_with_output().unwrap();
        return (
            output.status,
            String::new(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        );
    }
    let prepared: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(prepared["event"], "paea-settlement-prepared");
    assert_eq!(prepared["service"], "payment.settle");
    assert_eq!(prepared["operation"], operation);
    assert_eq!(prepared["authority"], false);
    assert_eq!(prepared["restart_reusable"], false);
    let permission: swarm_identity_contract::Permission =
        serde_json::from_value(prepared["permission"].clone()).unwrap();
    between();
    let f = fixture::Fixture::build_for_trust(permission, owned.scope.clone(), revision);
    if !bad_frame {
        fs::copy(&f.path, &declaration).unwrap();
        fs::set_permissions(&declaration, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let packet = if bad_frame {
        b"wrong-header\n".to_vec()
    } else {
        let mut packet = format!("swarm-trust-source-v1\n{}\n", f.pin).into_bytes();
        packet.extend_from_slice(b"STARTUP_FIXTURE_PASSWORD");
        packet
    };
    child.stdin.take().unwrap().write_all(&packet).unwrap();
    let mut rest = String::new();
    stdout.read_to_string(&mut rest).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!rest.contains("STARTUP_FIXTURE_PASSWORD"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("STARTUP_FIXTURE_PASSWORD"));
    f.close();
    (
        output.status,
        rest,
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// One composed real second-domain chain through one externally designated
/// delivered binary (see `delivered_binary`, re-proven at each spawn): with
/// fresh isolated owned project/ledger roots and the fictitious debt
/// provisioned through the documented fixture initialization, the guarded CLI
/// `paea settlement-call commit` records exactly one durable ledger effect
/// whose receipt binds the invoice, the provisioned debt terms and the
/// canonical payment action; an identical CLI commit replays the same durable
/// intent without a second effect or any debt change; the public read-only
/// `paea reconcile-effects` CLI reports the settlement journal `consistent`
/// with exactly one verified finding bound to that intent and, on the project
/// root never configured for Git, the explicit `uninitialized` Git journal
/// state. Test-environment composition only: the agent identity is the
/// existing explicitly simulated fixture, no UID separation, human presence,
/// real payment, accepted service or delivery provenance is proven here;
/// without `SWARM_PAEA_DELIVERED_BIN` the chain runs on the cargo-launched
/// binary exactly as before. Direct ledger witnesses (effect count, debt
/// terms, receipt binding) are read through separately opened read-only DBs,
/// never through the CLI's own report alone.
#[test]
#[ignore = "guarded full-root settlement scenario; run explicitly via its recipe"]
fn delivered_settlement_commit_replay_reconcile_once() {
    let owned = owned();
    // --- Step 1: guarded CLI commit records exactly one bound ledger effect. ---
    let (status, rest, stderr) = invoke(&owned, "commit", false);
    assert!(status.success(), "stderr: {stderr}");
    let first: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(first["event"], "paea-settlement-result");
    assert_eq!(first["outcome"], "recorded");
    assert_eq!(first["replayed"], false);
    assert_eq!(first["invoice_id"], "invoice-17");
    let intent = first["intent_id"].as_str().unwrap().to_owned();
    assert_eq!(intent.len(), 64);
    let digest = first["action_digest"].as_str().unwrap().to_owned();
    assert_eq!(digest.len(), 64);
    assert_eq!(first["trust_revision"], owned.trust_revision);
    assert!(!first["commit_requested_at"].as_str().unwrap().is_empty());
    // Durable effect: one row, provisioned debt unchanged, receipt binds the
    // invoice, the exact debt terms and the canonical payment action.
    assert_eq!(count(owned.ledger_dir.path()), 1);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some((
            "v7".into(),
            420000,
            "EUR".into(),
            "account-17".into()
        ))
    );
    let (row_intent, row_invoice, row_digest, receipt) = effect(owned.ledger_dir.path());
    assert_eq!(row_intent, intent);
    assert_eq!(row_invoice, "invoice-17");
    assert_eq!(row_digest, digest);
    assert_eq!(receipt["invoice_id"], "invoice-17");
    assert_eq!(receipt["intent_id"], intent.as_str());
    assert_eq!(receipt["action_digest"], digest.as_str());
    assert_eq!(receipt["trust_revision"], owned.trust_revision);
    assert_eq!(receipt["authority_principal"]["principal_kind"], "human");
    let canonical: Value = serde_json::from_slice(
        &receipt["canonical_action"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b.as_u64().unwrap() as u8)
            .collect::<Vec<u8>>(),
    )
    .unwrap();
    assert_eq!(canonical["canonicalization"], "paea-json-v1");
    assert_eq!(canonical["action"]["kind"], "payment.settle");
    assert_eq!(canonical["action"]["invoice_id"], "invoice-17");
    assert_eq!(canonical["action"]["invoice_version"], "v7");
    assert_eq!(canonical["action"]["amount_minor"], "420000");
    assert_eq!(canonical["action"]["currency"], "EUR");
    assert_eq!(canonical["action"]["beneficiary_account_id"], "account-17");
    // --- Step 2: identical CLI commit replays one durable intent. ---
    let (status, rest, stderr) = invoke(&owned, "commit", false);
    assert!(status.success(), "stderr: {stderr}");
    let replay: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(replay["event"], "paea-settlement-result");
    assert_eq!(replay["outcome"], "recorded");
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["intent_id"], first["intent_id"]);
    assert_eq!(replay["action_digest"], first["action_digest"]);
    // The ledger never grew and the debt never changed: no second payment.
    assert_eq!(count(owned.ledger_dir.path()), 1);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some((
            "v7".into(),
            420000,
            "EUR".into(),
            "account-17".into()
        ))
    );
    let (replay_intent, _, replay_digest, replay_receipt) = effect(owned.ledger_dir.path());
    assert_eq!(replay_intent, intent);
    assert_eq!(replay_digest, digest);
    assert_eq!(replay_receipt, receipt);
    // --- Step 3: public read-only CLI reconciliation report. ---
    let mut command = delivered_binary();
    let report_raw = command
        .args([
            "paea",
            "reconcile-effects",
            owned.ledger_dir.path().to_str().unwrap(),
            "--json",
            "--root",
            owned.project_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        report_raw.status.success(),
        "{}",
        String::from_utf8_lossy(&report_raw.stderr)
    );
    assert!(report_raw.stderr.is_empty());
    let report: Value = serde_json::from_slice(&report_raw.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    // The settlement journal holds exactly one verified effect.
    assert_eq!(report["settlement"]["state"], "consistent");
    assert_eq!(report["settlement"]["effectCount"], 1);
    assert_eq!(report["settlement"]["findings"][0]["intent_id"], intent.as_str());
    assert_eq!(
        report["settlement"]["findings"][0]["action_digest"],
        digest.as_str()
    );
    assert_eq!(
        report["settlement"]["findings"][0]["verified"],
        true,
        "unexpected refusals: {:?}",
        report["settlement"]["findings"][0]["refusals"]
    );
    // The project root was never configured for Git: explicit journal state.
    assert_eq!(report["git"]["state"], "uninitialized");
    owned.ledger.close().unwrap();
    owned.project.close().unwrap();
}

/// A wrong frame refuses through the designated delivered binary without any
/// preparation consumption and without any business mutation: the project
/// Trust state is unchanged and the ledger has no effect row and keeps its
/// provisioned debt. Motif of
/// `tests/paea_settlement.rs::settlement_call_cli_refuses_bad_frame_without_consumption`
/// on the one designated executable.
#[test]
#[ignore = "guarded full-root settlement scenario; run explicitly via its recipe"]
fn delivered_settlement_refuses_bad_frame_without_consumption() {
    let owned = owned();
    let before = fs::read(owned.project_dir.path().join(".swarm/trust-actions.json")).unwrap();
    let (status, rest, stderr) = invoke(&owned, "commit", true);
    assert!(!status.success());
    assert!(rest.is_empty(), "unexpected stdout: {rest}");
    assert!(
        stderr.contains("paea-settlement-call-frame"),
        "stderr: {stderr}"
    );
    let after = fs::read(owned.project_dir.path().join(".swarm/trust-actions.json")).unwrap();
    assert_eq!(before, after);
    assert_eq!(count(owned.ledger_dir.path()), 0);
    // The provisioned fictitious debt row is untouched by the refusal.
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    owned.ledger.close().unwrap();
    owned.project.close().unwrap();
}

/// One real revocation window between the guarded CLI settlement preparation
/// and its consumption: after the public prepared event (witnesses held, no
/// reservation), the operator performs the supported public Trust
/// configuration change `config-apply` on the project through the library,
/// disabling the effect identity; this advances the current Trust revision.
/// The still-signed grant then refuses twice under current storage state —
/// `paea-settlement-trust-revision` for the preparation-time revision and,
/// with a revision-matched grant, `paea-settlement-effect-role` for the
/// disabled effect identity — with no ledger effect row and the provisioned
/// debt untouched. After the public restore configuration the explicitly
/// re-provisioned current grant consumes a fresh legitimate preparation
/// exactly once: `recorded`, replayed=false, exactly one durable effect with
/// the provisioned debt terms and the consistent read-only reconciliation
/// journal. Motif of
/// `tests/paea_promote.rs::promote_call_cli_revoked_current_authority_refuses_without_effect_then_recovers`
/// on the settlement path of the one designated executable. Authority stays
/// the explicit simulated fixture; no UID separation, human presence, real
/// payment or accepted service is proven.
#[test]
#[ignore = "guarded full-root settlement scenario; run explicitly via its recipe"]
fn delivered_settlement_revoked_current_authority_refuses_without_effect_then_recovers() {
    let owned = owned();
    // Snapshot the configured identity set so the revocation disables the
    // effect mapping and the restore applies the identical set enabled again,
    // both through the same supported public configuration operation.
    let view: Value = serde_json::from_slice(
        &owned
            .project
            .trust("show", b"{}", "")
            .unwrap()
            .bytes,
    )
    .unwrap();
    let mut identities = view["identities"].clone();
    for id in identities.as_array_mut().unwrap() {
        if id["enrollment_principal"].is_null() {
            id["token"] = format!("fixture-{}-token", id["subject"].as_str().unwrap()).into();
        }
    }
    let restored = identities.clone();
    for id in identities.as_array_mut().unwrap() {
        if id["subject"] == "enrolled-effect" {
            id["disabled"] = true.into();
        }
    }
    let revision_before = owned.trust_revision;
    // The event id binds the revision it applies to, matching the established
    // promote-harness pattern: fresh events per run, never a conflicting
    // replay of an older retained fingerprint.
    let change = |event: &str, revision: i64, ids: &Value| {
        serde_json::to_vec(&json!({
            "schema_version":1,
            "event_id":format!("{event}-{revision}"),
            "expected_revision":revision,
            "reason":"Explicit fixture current-authority change",
            "policy":view["policy"],
            "identities":ids
        }))
        .unwrap()
    };
    // --- Window 1: revocation between preparation and consumption. ---
    let (status, rest, stderr) = invoke_between(&owned, "commit", false, owned.trust_revision, &|| {
        let applied: Value = serde_json::from_slice(
            &owned
                .project
                .trust(
                    "config-apply",
                    &change("settlement-cli-revoke-effect", revision_before, &identities),
                    "",
                )
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert_eq!(applied["revision"], revision_before + 1);
    });
    assert!(!status.success());
    assert!(rest.is_empty(), "unexpected stdout: {rest}");
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    println!("revoked-window-1-refusal {}", serde_json::to_string(&error).unwrap());
    // The settlement CLI surfaces the exact backend refusal as the top-level
    // failure code (no promote-style action envelope): the stale preparation-
    // time revision is refused by the current-authority recheck.
    assert_eq!(error["failure"]["code"], "paea-settlement-trust-revision");
    // Refusal left no current change beyond the explicit revocation itself and
    // no durable effect: the debt row keeps its provisioned terms.
    let current = current_trust_revision(owned.project_dir.path());
    assert_eq!(current, revision_before + 1);
    assert_eq!(count(owned.ledger_dir.path()), 0);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    // --- Window 2: same revoked current state with a revision-matched grant. ---
    let revoked_revision = revision_before + 1;
    let (status, rest, stderr) =
        invoke_between(&owned, "commit", false, revoked_revision, &|| {});
    assert!(!status.success());
    assert!(rest.is_empty(), "unexpected stdout: {rest}");
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    println!("revoked-window-2-refusal {}", serde_json::to_string(&error).unwrap());
    // Same revoked state with a revision-matched grant: the disabled effect
    // identity is refused by the exact role mapping recheck.
    assert_eq!(error["failure"]["code"], "paea-settlement-effect-role");
    assert_eq!(current_trust_revision(owned.project_dir.path()), revoked_revision);
    assert_eq!(count(owned.ledger_dir.path()), 0);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    // --- Legitimate recovery: public restore, then one exact consumption. ---
    let applied: Value = serde_json::from_slice(
        &owned
            .project
            .trust(
                "config-apply",
                &change("settlement-cli-restore-effect", revoked_revision, &restored),
                "",
            )
            .unwrap()
            .bytes,
    )
    .unwrap();
    assert_eq!(applied["revision"], revoked_revision + 1);
    let restored_revision = current_trust_revision(owned.project_dir.path());
    // The re-provisioned grant consumes a fresh legitimate preparation exactly
    // once: the earlier revoked preparation was closed by its refused process.
    let (status, rest, stderr) =
        invoke_between(&owned, "commit", false, restored_revision, &|| {});
    assert!(status.success(), "stderr: {stderr}");
    let reply: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(reply["event"], "paea-settlement-result");
    assert_eq!(reply["outcome"], "recorded");
    assert_eq!(reply["replayed"], false);
    assert_eq!(reply["invoice_id"], "invoice-17");
    assert_eq!(reply["trust_revision"], restored_revision);
    let intent = reply["intent_id"].as_str().unwrap().to_owned();
    let digest = reply["action_digest"].as_str().unwrap().to_owned();
    // Exactly one durable effect bound to the provisioned debt terms.
    assert_eq!(count(owned.ledger_dir.path()), 1);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    let (row_intent, row_invoice, row_digest, receipt) = effect(owned.ledger_dir.path());
    assert_eq!(row_intent, intent);
    assert_eq!(row_invoice, "invoice-17");
    assert_eq!(row_digest, digest);
    assert_eq!(receipt["invoice_id"], "invoice-17");
    assert_eq!(receipt["trust_revision"], restored_revision);
    // Public read-only reconciliation reports the consistent journal.
    let mut command = delivered_binary();
    let report_raw = command
        .args([
            "paea",
            "reconcile-effects",
            owned.ledger_dir.path().to_str().unwrap(),
            "--json",
            "--root",
            owned.project_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        report_raw.status.success(),
        "{}",
        String::from_utf8_lossy(&report_raw.stderr)
    );
    let report: Value = serde_json::from_slice(&report_raw.stdout).unwrap();
    assert_eq!(report["settlement"]["state"], "consistent");
    assert_eq!(report["settlement"]["effectCount"], 1);
    assert_eq!(report["settlement"]["findings"][0]["intent_id"], intent.as_str());
    assert_eq!(
        report["settlement"]["findings"][0]["verified"],
        true,
        "unexpected refusals: {:?}",
        report["settlement"]["findings"][0]["refusals"]
    );
    owned.ledger.close().unwrap();
    owned.project.close().unwrap();
}

/// One spawned delivered settlement CLI process holding its live preparation:
/// the public `paea-settlement-prepared` event was observed and validated, no
/// frame was answered and no consumption happened. The child stays alive until
/// `consume` answers its private stdin frame or the harness drops it (the pipe
/// close makes the CLI refuse its frame before any consumption).
struct Prepared {
    child: std::process::Child,
    /// Taken by the harness when it must close its own reader end (real
    /// channel break) before consumption; `consume` refuses a taken reader.
    stdout: Option<BufReader<std::process::ChildStdout>>,
    permission: swarm_identity_contract::Permission,
    declaration: PathBuf,
}

impl Prepared {
    /// Spawn one normal `paea settlement-call commit` process with its own
    /// declaration path, read and validate its public prepared event, and hold
    /// the preparation alive. The exact spawn argv is printed as a witness and
    /// the designated executable is re-hashed at this spawn. No frame is
    /// answered and no authority is established by this function.
    fn prepare(owned: &Owned, declaration: PathBuf) -> Prepared {
        let request = owned
            .ledger_dir
            .path()
            .join("settlement-request.json")
            .into_os_string()
            .into_string()
            .unwrap();
        let args = [
            "--json",
            "paea",
            "settlement-call",
            "commit",
            owned.project_dir.path().to_str().unwrap(),
            owned.ledger_dir.path().to_str().unwrap(),
            declaration.to_str().unwrap(),
            "--input",
            request.as_str(),
        ];
        println!("delivered-settlement-argv {}", args.join(" "));
        let mut child = delivered_binary()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut first = String::new();
        let read = stdout.read_line(&mut first).unwrap();
        if read == 0 {
            let output = child.wait_with_output().unwrap();
            panic!(
                "preparation produced no event: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let prepared: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(prepared["event"], "paea-settlement-prepared");
        assert_eq!(prepared["service"], "payment.settle");
        assert_eq!(prepared["operation"], "commit");
        assert_eq!(prepared["authority"], false);
        assert_eq!(prepared["restart_reusable"], false);
        let permission: swarm_identity_contract::Permission =
            serde_json::from_value(prepared["permission"].clone()).unwrap();
        Prepared {
            child,
            stdout: Some(stdout),
            permission,
            declaration,
        }
    }
    /// Observe the live child's exact identifier: bytes never confirm liveness
    /// by themselves, so the lost-response harness checks the actual process.
    fn pid(&self) -> u32 {
        self.child.id()
    }
    /// Answer this preparation's private stdin frame with the provisioned
    /// declaration pin and personal password, then drain the full reply.
    /// Outputs: exit status, stdout after the prepared event, stderr. The
    /// credential packet never appears in any returned text and no refusal is
    /// hidden: the exact stderr bytes stay the caller's own evidence. Requires
    /// the harness to still hold the stdout reader; for the broken-channel
    /// consumption path use `consume_without_stdout` instead.
    fn consume(mut self, packet: &[u8]) -> (std::process::ExitStatus, String, String) {
        self.child
            .stdin
            .take()
            .unwrap()
            .write_all(packet)
            .unwrap();
        let mut rest = String::new();
        let mut stdout = self.stdout.take().expect("stdout reader already taken");
        stdout.read_to_string(&mut rest).unwrap();
        let output = self.child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(!rest.contains("STARTUP_FIXTURE_PASSWORD"));
        assert!(!stderr.contains("STARTUP_FIXTURE_PASSWORD"));
        (output.status, rest, stderr)
    }
    /// Test-only consumption path for the real broken-channel scenario: the
    /// harness has ALREADY taken and dropped the stdout reader (the child's
    /// final write meets the actually closed pipe), so no stdout reading
    /// happens here — unlike `consume`, which requires that reader. The frame
    /// is answered, then the child's stdin handle is explicitly dropped so the
    /// CLI sees its private stdin EOF, and the child is awaited (reaped) with
    /// its exact stderr retained. There is no drain before the frame and no
    /// kill, simulated error or state faking: the harness observes the real
    /// channel order (closed stdout → answered frame → closed stdin → exit)
    /// and the CLI's own code and durable state decide the verdict.
    fn consume_without_stdout(
        mut self,
        packet: &[u8],
    ) -> (std::process::ExitStatus, String) {
        assert!(
            self.stdout.is_none(),
            "the stdout reader must already be taken (channel broken) before \
             consume_without_stdout"
        );
        self.child
            .stdin
            .take()
            .unwrap()
            .write_all(packet)
            .expect("frame write through stdin");
        // The taken stdin handle is dropped immediately: the child's read of
        // its private frame returns EOF as soon as the packet ends, exactly
        // like `invoke_between`.
        let output = self.child.wait_with_output().expect("reap the CLI child");
        assert!(
            output.stdout.is_empty(),
            "unexpected bytes on the broken stdout channel: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (output.status, stderr)
    }
}

/// Provision the exact fixture declaration for one live preparation's prepared
/// permission at the preparation's own declaration path and return the owned
/// fixture together with the private frame packet (pin64 + personal password).
/// Same documented provisioning path as the sequential harness; fixture-only
/// simulated authority, same-UID. The returned fixture must stay owned by the
/// caller until AFTER the CLI consumed the frame and exited: the v3 declaration
/// references its protected layout directories (bootstrap/configuration/
/// witness/business/personal), which live inside the fixture's own tempdir —
/// dropping the fixture earlier deletes those directories and the CLI's
/// protected startup refuses with `paea-settlement-call-startup` before any
/// consumption (verified reproducible defect of the first contention harness
/// revision; the sequential harness keeps `f` alive through `invoke_between`).
fn provision(owned: &Owned, prepared: &Prepared) -> (fixture::Fixture, Vec<u8>) {
    let f = fixture::Fixture::build_for_trust(
        prepared.permission.clone(),
        owned.scope.clone(),
        owned.trust_revision,
    );
    fs::copy(&f.path, &prepared.declaration).unwrap();
    fs::set_permissions(&prepared.declaration, fs::Permissions::from_mode(0o600)).unwrap();
    let mut packet = format!("swarm-trust-source-v1\n{}\n", f.pin).into_bytes();
    packet.extend_from_slice(b"STARTUP_FIXTURE_PASSWORD");
    (f, packet)
}

/// Build one alternate legitimate preparation request by re-encoding the same
/// fixture document with distinct exact payment terms: same project context
/// (the ledger stays provisioned for the document's declared project) but a
/// different invoice, version, amount and beneficiary account. The document is
/// re-decoded so every field stays inside the real protocol grammar; no field
/// outside the retained `Document` shape is invented. Returns the exact bytes
/// to feed the CLI `--input` file plus the terms witness for assertions.
fn alternate_document() -> (Vec<u8>, (String, i64, String, String)) {
    let (mut proposal, context, _) = document();
    proposal.request_id = "request-alternate-terms".into();
    proposal.action = swarm_protection_contract::Action::Payment {
        invoice_id: "invoice-18".into(),
        invoice_version: "v8".into(),
        amount_minor: "420001".into(),
        currency: "EUR".into(),
        beneficiary_account_id: "account-18".into(),
    };
    let doc = swarm_protection_contract::Document {
        proposal,
        context: context.clone(),
    };
    // Round-trip through the real decoder: the alternate terms are proven
    // representable before any CLI sees them, and the bytes are canonical.
    let raw = serde_json::to_vec(&doc).unwrap();
    let decoded = swarm_protection_contract::decode_document(&raw).unwrap();
    assert_eq!(decoded.proposal, doc.proposal);
    assert_eq!(decoded.context, doc.context);
    (
        raw,
        (
            "v8".into(),
            420001,
            "EUR".into(),
            "account-18".into(),
        ),
    )
}

/// One real signed-agreement mismatch window on the delivered settlement CLI:
/// the project and Trust state are identical for both preparations, the ledger
/// holds ONE provisioned debt with the original document's exact terms, and two
/// individually legitimate preparations exist — preparation A for the original
/// terms (invoice-17/v7/420000/EUR/account-17) and preparation B for the
/// alternate terms (invoice-17/v8/420001/EUR/account-18), both against the same
/// project/Trust and the same ledger, both with distinct current signed fixture
/// grants available. A (holding its live prepared witnesses) is consumed with
/// B's exact current signed agreement provisioned at its declaration: the
/// current authority is valid, current and matching — only the agreement is
/// exactly a different preparation's — so the refusal must be the exact
/// permission-mismatch branch, not a stale revision, an expired grant, a bad
/// frame, a startup, format or project failure. The refusal leaves zero durable
/// ledger effects and the provisioned debt untouched. Then a NEW preparation
/// for the alternate terms B is provisioned with its own exact agreement and
/// consumes exactly once: `recorded`, replayed=false, exactly one durable
/// effect bound to the alternate terms with a consistent read-only
/// reconciliation journal — and the original debt row keeps its provisioned
/// terms. The pre-existing valid witness on equivalent separate data is
/// `delivered_settlement_commit_replay_reconcile_once` (same fixture grants and
/// provisioning path, run in the same recipe). Fixture authority only,
/// same-UID, fictitious SQLite: no UID separation, human presence, real payment
/// or independent acceptance is proven here.
#[test]
#[ignore = "guarded full-root settlement agreement-mismatch scenario; run explicitly via its recipe"]
fn delivered_settlement_wrong_agreement_refuses_without_effect_then_recovers() {
    let (alternate_raw, alternate_terms) = alternate_document();
    let owned = owned();
    // Both preparations are individually legitimate against the SAME project,
    // Trust configuration and current revision. The ledger is provisioned by
    // `owned()` with the original `invoice-17` debt; the alternate terms live
    // on a different invoice (`invoice-18`), so they need their own ledger
    // root: the documented initialization refuses an initialized ledger and
    // debts are never rewritten, so a second dedicated ledger for the same
    // project is the supported composition.
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    assert_eq!(debt_for(owned.ledger_dir.path(), "invoice-18"), None);
    let (alt_proposal, alt_context, _) = {
        let doc = swarm_protection_contract::decode_document(&alternate_raw).unwrap();
        (doc.proposal, doc.context, alternate_raw.clone())
    };
    let (alternate_ledger_dir, alternate_ledger) = root();
    alternate_ledger
        .paea_initialize_settlement(
            &alt_context.project_id,
            &"0b".repeat(32),
            std::slice::from_ref(&alt_proposal),
        )
        .unwrap();
    assert_eq!(debt_for(alternate_ledger_dir.path(), "invoice-18"), Some(alternate_terms.clone()));
    // Preparation A consumes the original document through the same request
    // file the other delivered scenarios use (`Prepared::prepare` reads it).
    let request = owned.ledger_dir.path().join("settlement-request.json");
    fs::write(&request, &owned.document).unwrap();
    // --- Legitimate fresh preparation A for the original provisioned terms. ---
    let p1 = Prepared::prepare(
        &owned,
        owned.ledger_dir.path().join("operator-declaration-p1.json"),
    );
    assert_eq!(count(owned.ledger_dir.path()), 0);
    // Its exact permission binds the original canonical action; the mismatch
    // must later refuse through this agreement, never through another cause.
    let resource = p1.permission.resource.clone();
    assert!(resource.contains("/payment.settle/commit/"));
    // --- Fresh legitimate preparation B for the alternate terms. ---
    let alternate_input = alternate_ledger_dir.path().join("settlement-request.json");
    fs::write(&alternate_input, &alternate_raw).unwrap();
    let alternate = {
        let request = alternate_input.to_str().unwrap().to_owned();
        let declaration = alternate_ledger_dir
            .path()
            .join("operator-declaration-p2.json");
        let args = [
            "--json",
            "paea",
            "settlement-call",
            "commit",
            owned.project_dir.path().to_str().unwrap(),
            alternate_ledger_dir.path().to_str().unwrap(),
            declaration.to_str().unwrap(),
            "--input",
            request.as_str(),
        ];
        println!("delivered-settlement-argv {}", args.join(" "));
        let mut child = delivered_binary()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut first = String::new();
        let read = stdout.read_line(&mut first).unwrap();
        if read == 0 {
            let output = child.wait_with_output().unwrap();
            panic!(
                "alternate preparation produced no event: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let prepared: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(prepared["event"], "paea-settlement-prepared");
        assert_eq!(prepared["service"], "payment.settle");
        assert_eq!(prepared["operation"], "commit");
        assert_eq!(prepared["authority"], false);
        assert_eq!(prepared["restart_reusable"], false);
        let permission: swarm_identity_contract::Permission =
            serde_json::from_value(prepared["permission"].clone()).unwrap();
        // The two live preparations bind exactly different canonical actions
        // (distinct exact terms) for the same project: neither agreement can
        // substitute for the other.
        assert_ne!(
            p1.permission.resource.rsplit('/').next().unwrap(),
            permission.resource.rsplit('/').next().unwrap(),
            "prepared canonical action digests are equal: the alternate terms did not change the action"
        );
        (child, stdout, permission)
    };
    let (_alternate_child, _alternate_stdout, alternate_permission) = alternate;
    // --- Mismatched consumption: preparation A answered with agreement B. ---
    let (fixture1, packet1) = {
        let f = fixture::Fixture::build_for_trust(
            alternate_permission.clone(),
            owned.scope.clone(),
            owned.trust_revision,
        );
        fs::copy(
            &f.path,
            owned
                .ledger_dir
                .path()
                .join("operator-declaration-p1.json"),
        )
        .unwrap();
        let mut packet = format!("swarm-trust-source-v1\n{}\n", f.pin).into_bytes();
        packet.extend_from_slice(b"STARTUP_FIXTURE_PASSWORD");
        (f, packet)
    };
    let (status1, rest1, stderr1) = {
        let out = {
            let _keep = fixture1;
            p1.consume(&packet1)
        };
        out
    };
    assert!(!status1.success(), "unexpected stdout: {rest1}");
    assert!(rest1.is_empty(), "unexpected stdout: {rest1}");
    let error: Value = serde_json::from_str(stderr1.trim()).unwrap();
    println!(
        "agreement-mismatch-refusal {}",
        serde_json::to_string(&error).unwrap()
    );
    // The current authority chain refused the exact permission mismatch: the
    // top-level settlement-call branch is the held-authority refusal and the
    // safe cause names the exact signed permission check.
    assert_eq!(error["failure"]["code"], "paea-settlement-authority");
    assert!(
        error["failure"]["causes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "enrollment-control:permission"),
        "expected the exact permission-mismatch cause, got: {:?}",
        error["failure"]["causes"]
    );
    // No effect, no replay and no debt mutation anywhere: the refusal left the
    // provisioned debts exactly as initialized and both ledgers empty.
    assert_eq!(count(owned.ledger_dir.path()), 0);
    assert_eq!(count(alternate_ledger_dir.path()), 0);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    assert_eq!(
        debt_for(alternate_ledger_dir.path(), "invoice-18"),
        Some(("v8".into(), 420001, "EUR".into(), "account-18".into()))
    );
    // --- Recovery: new compliant preparation A with its own exact agreement. ---
    let request = owned.ledger_dir.path().join("settlement-request.json");
    fs::write(&request, &owned.document).unwrap();
    let declaration = owned
        .ledger_dir
        .path()
        .join("operator-declaration-p3.json");
    let args = [
        "--json",
        "paea",
        "settlement-call",
        "commit",
        owned.project_dir.path().to_str().unwrap(),
        owned.ledger_dir.path().to_str().unwrap(),
        declaration.to_str().unwrap(),
        "--input",
        request.to_str().unwrap(),
    ];
    println!("delivered-settlement-argv {}", args.join(" "));
    let mut child = delivered_binary()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut first = String::new();
    let read = stdout.read_line(&mut first).unwrap();
    if read == 0 {
        let output = child.wait_with_output().unwrap();
        panic!(
            "recovery preparation produced no event: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let prepared: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(prepared["event"], "paea-settlement-prepared");
    let permission: swarm_identity_contract::Permission =
        serde_json::from_value(prepared["permission"].clone()).unwrap();
    // A fresh preparation holds a fresh root nonce, so its exact permission
    // resource differs from the refused window-1 preparation; only the bound
    // canonical action digest must be the ORIGINAL agreement's own.
    assert_ne!(permission.resource, resource);
    assert_eq!(
        permission.resource.rsplit('/').next().unwrap(),
        resource.rsplit('/').next().unwrap(),
        "recovery preparation did not bind the original canonical action"
    );
    let (fixture2, packet2) = {
        let f = fixture::Fixture::build_for_trust(
            permission,
            owned.scope.clone(),
            owned.trust_revision,
        );
        fs::copy(&f.path, &declaration).unwrap();
        let mut packet = format!("swarm-trust-source-v1\n{}\n", f.pin).into_bytes();
        packet.extend_from_slice(b"STARTUP_FIXTURE_PASSWORD");
        (f, packet)
    };
    let (status2, rest2, stderr2) = {
        let _keep = fixture2;
        let mut rest = String::new();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&packet2)
            .unwrap();
        stdout.read_to_string(&mut rest).unwrap();
        let output = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(!rest.contains("STARTUP_FIXTURE_PASSWORD"));
        assert!(!stderr.contains("STARTUP_FIXTURE_PASSWORD"));
        (output.status, rest, stderr)
    };
    assert!(status2.success(), "stderr: {stderr2}");
    let reply: Value = serde_json::from_str(rest2.trim()).unwrap();
    assert_eq!(reply["event"], "paea-settlement-result");
    assert_eq!(reply["outcome"], "recorded");
    assert_eq!(reply["replayed"], false);
    assert_eq!(reply["invoice_id"], "invoice-17");
    let intent = reply["intent_id"].as_str().unwrap().to_owned();
    let digest = reply["action_digest"].as_str().unwrap().to_owned();
    // Exactly one durable effect, bound to the ORIGINAL provisioned terms; the
    // provisioned debt rows themselves keep their exact terms on both ledgers
    // (debts are never rewritten) and the alternate ledger stays at zero.
    assert_eq!(count(owned.ledger_dir.path()), 1);
    assert_eq!(count(alternate_ledger_dir.path()), 0);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    let (row_intent, row_invoice, row_digest, receipt) = effect(owned.ledger_dir.path());
    assert_eq!(row_intent, intent);
    assert_eq!(row_invoice, "invoice-17");
    assert_eq!(row_digest, digest);
    assert_eq!(receipt["intent_id"], intent.as_str());
    assert_eq!(receipt["action_digest"], digest.as_str());
    assert_eq!(receipt["trust_revision"], owned.trust_revision);
    let canonical: Value = serde_json::from_slice(
        &receipt["canonical_action"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b.as_u64().unwrap() as u8)
            .collect::<Vec<u8>>(),
    )
    .unwrap();
    assert_eq!(canonical["canonicalization"], "paea-json-v1");
    assert_eq!(canonical["action"]["invoice_id"], "invoice-17");
    assert_eq!(canonical["action"]["invoice_version"], "v7");
    assert_eq!(canonical["action"]["amount_minor"], "420000");
    assert_eq!(canonical["action"]["currency"], "EUR");
    assert_eq!(canonical["action"]["beneficiary_account_id"], "account-17");
    // Public read-only reconciliation reports the consistent journal.
    let mut command = delivered_binary();
    let report_raw = command
        .args([
            "paea",
            "reconcile-effects",
            owned.ledger_dir.path().to_str().unwrap(),
            "--json",
            "--root",
            owned.project_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        report_raw.status.success(),
        "{}",
        String::from_utf8_lossy(&report_raw.stderr)
    );
    assert!(report_raw.stderr.is_empty());
    let report: Value = serde_json::from_slice(&report_raw.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["settlement"]["state"], "consistent");
    assert_eq!(report["settlement"]["effectCount"], 1);
    assert_eq!(report["settlement"]["findings"][0]["intent_id"], intent.as_str());
    assert_eq!(
        report["settlement"]["findings"][0]["action_digest"],
        digest.as_str()
    );
    assert_eq!(
        report["settlement"]["findings"][0]["verified"],
        true,
        "unexpected refusals: {:?}",
        report["settlement"]["findings"][0]["refusals"]
    );
    owned.ledger.close().unwrap();
    alternate_ledger.close().unwrap();
    owned.project.close().unwrap();
}

/// One real lost settlement response on the delivered CLI: a legitimate live
/// preparation is answered with its own exact current agreement, but the
/// harness closes its OWN reader end of the child's stdout pipe BEFORE writing
/// the private frame. The preparation event was already observed on that pipe,
/// so the loss is real and dated: the child's final receipt write meets a
/// broken pipe (Rust ignores SIGPIPE by default, so the process survives to
/// report its own loss; no signal is sent and no DB/sleep/error is simulated).
/// Whether the durable commit happened stays unknown to the client at that
/// moment; the ledger is then observed independently (separately opened
/// read-only DB plus the public read-only `paea reconcile-effects` CLI). If and
/// only if exactly one durable effect exists, a NEW preparation with fresh
/// root nonce and the same exact business terms is consumed: it must replay
/// the same durable intent (`recorded`, replayed=true) without a second effect
/// or debt change. A refusal BEFORE the commit is not invented into this
/// recovery: with no effect the exact post-authorization stdout loss keeps its
/// own terminal evidence and the replay branch stays unproven. Fixture
/// authority only, same-UID, fictitious SQLite: no UID separation, human
/// presence, real payment or independent acceptance is proven here.
#[test]
#[ignore = "guarded full-root lost-response scenario; run explicitly via its recipe"]
fn delivered_settlement_lost_stdout_after_commit_replays_without_second_effect() {
    let owned = owned();
    let request = owned.ledger_dir.path().join("settlement-request.json");
    fs::write(&request, &owned.document).unwrap();
    let mut prepared = Prepared::prepare(
        &owned,
        owned.ledger_dir.path().join("operator-declaration-p1.json"),
    );
    assert_eq!(
        count(owned.ledger_dir.path()),
        0,
        "an effect exists before any consumption"
    );
    let pid = prepared.pid();
    println!("lost-response-child-pid {pid}");
    println!("lost-response-t0 try_wait={:?}", prepared.child.try_wait().map(|s| s.map(|s| s.to_string())));
    let (fixture, packet) = provision(&owned, &prepared);
    println!("lost-response-t1 provisioned try_wait={:?}", prepared.child.try_wait().map(|s| s.map(|s| s.to_string())));
    // Real channel break owned by the harness: take the reader the harness
    // itself used for the prepared event (keeping the process alive while its
    // other witnesses are held, like `Prepared::consume`) and drop it WITHOUT
    // draining — a read here would deadlock, because the CLI child is waiting
    // for the private stdin frame before writing anything else to stdout
    // (verified harness deadlock of the first revision; the child's stdin is
    // the other side of this exact channel order). The child keeps its own
    // write end; no signal, no kill and no state faking: its final receipt
    // write fails at the actually broken pipe.
    drop(prepared.stdout.take().expect("stdout reader already taken"));
    // Consumption after the break: the same exact agreement, nothing else
    // changed. The frame write itself must still succeed through stdin; the
    // harness holds no stdout reader anymore, so this is the test-only
    // `consume_without_stdout` path, not `Prepared::consume`.
    println!("lost-response-t2 pre-consume try_wait={:?}", prepared.child.try_wait().map(|s| s.map(|s| s.to_string())));
    let (status, stderr) = {
        let _keep = &fixture;
        prepared.consume_without_stdout(&packet)
    };
    // The client saw NOTHING after the prepared event: no receipt line reached
    // the (already closed) stdout, so the reply is empty regardless of outcome.
    let rest = String::new();
    println!("lost-response-t3 post-consume status={status}");
    // The client saw NOTHING after the prepared event: no receipt line reached
    // the (already closed) stdout, so the reply is empty regardless of outcome.
    assert!(rest.is_empty(), "unexpected stdout after lost response: {rest}");
    let error: Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|e| panic!("undecodable CLI stderr: {e}: {stderr}"));
    println!("lost-response-stderr {}", serde_json::to_string(&error).unwrap());
    let code = error["failure"]["code"].as_str().unwrap().to_owned();
    let durable = count(owned.ledger_dir.path());
    println!("lost-response-code {code} durable-effects {durable}");
    if durable == 0 {
        // No observable post-commit loss to prove here: the exact refusal stays
        // terminal evidence and the replay branch stays unproven (no invention).
        panic!(
            "no durable effect after the lost response; terminal refusal {code}: {stderr}"
        );
    }
    // The durable commit happened but the client never got the receipt: the
    // exact output loss must be the top-level failure, not a hidden success.
    assert!(!status.success());
    assert_eq!(code, "paea-settlement-call-output", "stderr: {stderr}");
    // Independent durable observation: exactly one effect bound to the
    // provisioned terms; the debt never changed.
    assert_eq!(durable, 1);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    let (row_intent, row_invoice, row_digest, receipt) = effect(owned.ledger_dir.path());
    assert_eq!(row_invoice, "invoice-17");
    assert_eq!(receipt["invoice_id"], "invoice-17");
    // Public read-only reconciliation from the delivered binary: the journal is
    // consistent with exactly one verified effect for that intent.
    let mut command = delivered_binary();
    let report_raw = command
        .args([
            "paea",
            "reconcile-effects",
            owned.ledger_dir.path().to_str().unwrap(),
            "--json",
            "--root",
            owned.project_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        report_raw.status.success(),
        "{}",
        String::from_utf8_lossy(&report_raw.stderr)
    );
    assert!(report_raw.stderr.is_empty());
    let report: Value = serde_json::from_slice(&report_raw.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["settlement"]["state"], "consistent");
    assert_eq!(report["settlement"]["effectCount"], 1);
    assert_eq!(report["settlement"]["findings"][0]["intent_id"], row_intent.as_str());
    assert_eq!(report["settlement"]["findings"][0]["action_digest"], row_digest.as_str());
    assert_eq!(
        report["settlement"]["findings"][0]["verified"],
        true,
        "unexpected refusals: {:?}",
        report["settlement"]["findings"][0]["refusals"]
    );
    // Recovery WITHOUT a second effect: a new preparation (fresh root nonce,
    // same canonical terms) is consumed and replays the same durable intent.
    let request = owned.ledger_dir.path().join("settlement-request.json");
    fs::write(&request, &owned.document).unwrap();
    let mut recovery = Prepared::prepare(
        &owned,
        owned.ledger_dir.path().join("operator-declaration-p2.json"),
    );
    let recovery_pid = recovery.pid();
    assert_ne!(recovery_pid, pid, "recovery reused the lost child's pid");
    let (replay_fixture, replay_packet) = provision(&owned, &recovery);
    let (status2, rest2, stderr2) = { let _keep = replay_fixture; recovery.consume(&replay_packet) };
    assert!(status2.success(), "stderr: {stderr2}");
    let replay: Value = serde_json::from_str(rest2.trim())
        .unwrap_or_else(|e| panic!("undecodable replay reply: {e}: {rest2}"));
    assert_eq!(replay["event"], "paea-settlement-result");
    assert_eq!(replay["outcome"], "recorded");
    assert_eq!(replay["replayed"], true, "recovery claimed a new commitment");
    assert_eq!(replay["intent_id"], row_intent.as_str());
    assert_eq!(replay["action_digest"], row_digest.as_str());
    // The ledger never grew and the debt never changed: no second payment.
    assert_eq!(count(owned.ledger_dir.path()), 1);
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    let (same_intent, _, same_digest, same_receipt) = effect(owned.ledger_dir.path());
    assert_eq!(same_intent, row_intent);
    assert_eq!(same_digest, row_digest);
    assert_eq!(same_receipt, receipt);
    owned.ledger.close().unwrap();
    owned.project.close().unwrap();
}

/// Two normal delivered settlement CLI processes prepare concurrently against
/// the SAME fictitious debt/invoice/ledger with the same current fixture
/// authority and the same exact business terms, then consume their live
/// preparations in explicitly synchronized real contention (a std::sync::Barrier
/// releases both private frames together — no sleep or assumed ordering). Both
/// prepared events are validated first: the same canonical `action_digest`
/// (identical business terms, deterministic canonicalization) with distinct
/// `root_digest`s (independent per-preparation nonce witnesses). No
/// config-apply or revocation happens inside the window. The verdict accepts
/// exactly one business effect, whichever ordering wins: one process records
/// the single durable effect and the other either replays the same durable
/// intent (`recorded`, replayed=true, same intent) or refuses explicitly; two
/// distinct effects are always a failure. A busy-reservation refusal is a
/// legitimate loss of the race but is kept with its exact cause — the ledger
/// then holds one effect from the winner and the loser's refusal stays its own
/// terminal evidence, never converted into a replay it did not report. The
/// public read-only `paea reconcile-effects` CLI and separately opened
/// read-only DB reads verify effectCount 1, the exact provisioned terms and
/// the debt settled once. The pre-existing valid witness on equivalent
/// separate data is
/// `delivered_settlement_commit_replay_reconcile_once` (same fixture grants,
/// same provisioning, fresh isolated roots, run in the same recipe). Fixture
/// authority only, same-UID, fictitious SQLite: no UID separation, human
/// presence, real payment or independent acceptance is proven here.
#[test]
#[ignore = "guarded full-root settlement contention scenario; run explicitly via its recipe"]
fn delivered_settlement_two_cli_instances_same_business_terms_single_effect() {
    let owned = owned();
    let request = owned.ledger_dir.path().join("settlement-request.json");
    fs::write(&request, &owned.document).unwrap();
    // Both live preparations exist simultaneously before either consumption:
    // process P2 spawns and reaches its prepared event while P1 still holds
    // its own unconsumed preparation, proving real concurrent preparations
    // against the same ledger. Each gets its own declaration path.
    let mut p1 = Prepared::prepare(
        &owned,
        owned.ledger_dir.path().join("operator-declaration-p1.json"),
    );
    let mut p2 = Prepared::prepare(
        &owned,
        owned.ledger_dir.path().join("operator-declaration-p2.json"),
    );
    assert_eq!(
        count(owned.ledger_dir.path()),
        0,
        "an effect exists before any consumption"
    );
    // Same exact business terms (same canonical action), independent live
    // preparation witnesses: the per-preparation root digests must differ,
    // otherwise the two processes would not hold distinct live preparations.
    assert_ne!(p1.permission.resource, p2.permission.resource);
    assert_eq!(
        p1.permission.resource.rsplit('/').next().unwrap(),
        p2.permission.resource.rsplit('/').next().unwrap(),
        "prepared canonical action digests differ: not the same business terms"
    );
    // Provision each preparation's own declaration before the barrier; no
    // Trust change happens inside the contention window. Each thread owns its
    // provisioned fixture: the v3 declaration references the fixture's own
    // layout directories, so the fixture must stay alive until that CLI
    // process consumed its frame and exited (see `provision`).
    let (fixture1, packet1) = provision(&owned, &p1);
    let (fixture2, packet2) = provision(&owned, &p2);
    let barrier = Arc::new(Barrier::new(2));
    let b1 = barrier.clone();
    let handle1 = std::thread::spawn(move || {
        let out = {
            let _keep = fixture1;
            b1.wait();
            p1.consume(&packet1)
        };
        out
    });
    let b2 = barrier.clone();
    let handle2 = std::thread::spawn(move || {
        let out = {
            let _keep = fixture2;
            b2.wait();
            p2.consume(&packet2)
        };
        out
    });
    let (status1, rest1, stderr1) = handle1.join().unwrap();
    let (status2, rest2, stderr2) = handle2.join().unwrap();
    // --- Verdict: exactly one business effect, with exact per-process receipts. ---
    let recorded = |label: &'static str,
                    status: &std::process::ExitStatus,
                    rest: &str,
                    stderr: &str|
     -> Option<Value> {
        println!("{label}-status {:?}", status.code());
        println!("{label}-stdout {rest}");
        println!("{label}-stderr {stderr}");
        if status.success() {
            let reply: Value = serde_json::from_str(rest.trim())
                .unwrap_or_else(|e| panic!("{label}: undecodable reply: {e}: {rest}"));
            assert_eq!(reply["event"], "paea-settlement-result", "{label}");
            assert_eq!(reply["outcome"], "recorded", "{label}");
            return Some(reply);
        }
        let error: Value = serde_json::from_str(stderr.trim())
            .unwrap_or_else(|e| panic!("{label}: undecodable refusal: {e}: {stderr}"));
        let code = error["failure"]["code"].as_str().unwrap().to_owned();
        println!("{label}-refusal {code}");
        None
    };
    let out1 = recorded("cli-1", &status1, &rest1, &stderr1);
    let out2 = recorded("cli-2", &status2, &rest2, &stderr2);
    // The durable ledger holds exactly one effect row — whatever the race
    // outcome was. Zero effects or two effects both fail this contract.
    assert_eq!(
        count(owned.ledger_dir.path()),
        1,
        "contention did not produce exactly one durable effect"
    );
    // The provisioned debt is settled once, its terms unchanged.
    assert_eq!(
        debt(owned.ledger_dir.path()),
        Some(("v7".into(), 420000, "EUR".into(), "account-17".into()))
    );
    let (row_intent, row_invoice, row_digest, receipt) = effect(owned.ledger_dir.path());
    assert_eq!(row_invoice, "invoice-17");
    assert_eq!(receipt["invoice_id"], "invoice-17");
    // Winner and loser stay compatible with the real contract. Which process
    // wins the race is not fixed: the winner is identified by its receipt
    // (`replayed=false`), never by spawn order.
    // - a successful loser replays the SAME durable intent (no second payment);
    // - a refused loser keeps its explicit refusal as terminal evidence; its
    //   cause is preserved verbatim above, never converted into a replay.
    match (&out1, &out2) {
        (Some(a), Some(b)) => {
            let (winner, loser) = match (
                a["replayed"].as_bool(),
                b["replayed"].as_bool(),
            ) {
                (Some(false), Some(true)) => (a, b),
                (Some(true), Some(false)) => (b, a),
                _ => panic!(
                    "exactly one process must claim the first effect; \
                     receipts: cli-1 replayed={:?}, cli-2 replayed={:?}",
                    a["replayed"], b["replayed"]
                ),
            };
            assert_eq!(winner["replayed"], false, "winner claimed a replay");
            assert_eq!(loser["replayed"], true, "loser replayed without replayed=true");
            assert_eq!(winner["intent_id"], loser["intent_id"], "two distinct durable intents");
            assert_eq!(winner["intent_id"], row_intent.as_str());
            assert_eq!(loser["action_digest"], winner["action_digest"]);
        }
        (Some(a), None) | (None, Some(a)) => {
            assert_eq!(a["replayed"], false);
            assert_eq!(a["intent_id"], row_intent.as_str());
        }
        (None, None) => panic!(
            "both consumers refused: no business effect despite the recorded row: \
             cli-1 stderr: {stderr1}; cli-2 stderr: {stderr2}"
        ),
    }
    if let Some(a) = &out1 {
        assert_eq!(a["action_digest"], row_digest.as_str());
    }
    if let Some(b) = &out2 {
        assert_eq!(b["action_digest"], row_digest.as_str());
    }
    // Public read-only reconciliation reports the consistent journal with
    // exactly one verified effect bound to the recorded intent.
    let mut command = delivered_binary();
    let report_raw = command
        .args([
            "paea",
            "reconcile-effects",
            owned.ledger_dir.path().to_str().unwrap(),
            "--json",
            "--root",
            owned.project_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        report_raw.status.success(),
        "{}",
        String::from_utf8_lossy(&report_raw.stderr)
    );
    assert!(report_raw.stderr.is_empty());
    let report: Value = serde_json::from_slice(&report_raw.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["settlement"]["state"], "consistent");
    assert_eq!(report["settlement"]["effectCount"], 1);
    assert_eq!(report["settlement"]["findings"][0]["intent_id"], row_intent.as_str());
    assert_eq!(
        report["settlement"]["findings"][0]["action_digest"],
        row_digest.as_str()
    );
    assert_eq!(
        report["settlement"]["findings"][0]["verified"],
        true,
        "unexpected refusals: {:?}",
        report["settlement"]["findings"][0]["refusals"]
    );
    owned.ledger.close().unwrap();
    owned.project.close().unwrap();
}
