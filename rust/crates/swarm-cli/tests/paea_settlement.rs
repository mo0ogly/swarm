//! Two-phase real CLI fixture for the guarded PAEA fictitious settlement call:
//! one actual project Root with a real plan and Trust configuration, one
//! physically separate ledger Root initialized for the fixture's declared
//! project, then the CLI's own preparation event answered by an out-of-band
//! provisioned protected declaration and a private stdin frame. Fictitious
//! SQLite effects only; no protected daemon, real payment, UID separation,
//! human presence or accepted service is claimed.
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
};
use swarm_engine::{RootStore, SystemEnvironment};
use swarm_identity_contract::Scope;
use swarm_protection_contract::Proposal;

const PIN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Canonical interchange fixture remains declared data, never admission
/// evidence; the same document feeds the ledger initialization and the CLI's
/// `--input` request file.
fn document() -> (Proposal, swarm_protection_contract::AdmissionContext, Vec<u8>) {
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

/// Actual public work.create and persisted plan stamp, no fabricated generation.
fn project() -> (tempfile::TempDir, RootStore, Scope) {
    let (dir, mut store) = root();
    let work = store
        .execute_external(
            "",
            "work.create",
            br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Settlement plan","objective":"Exact fictitious effect","scope":"Fixture","criteria":["No duplicate invoice"]}"#,
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work
        .id;
    let config = json!({
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

/// Full owned fixture set for one CLI settlement run: the actual project Root
/// (writable, plan scope read, Trust applied), the physically separate ledger
/// Root initialized for the document's declared project, and the exact request
/// document bytes. All owners must be explicitly closed by the caller.
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

/// Full two-phase CLI invocation: the prepared event names the exact
/// permission, the fixture mints it into a declaration, then the frame answers
/// with the fixture pin and personal password. `bad_frame` replaces the frame
/// with a wrong header so the refusal happens before any consumption.
fn invoke(owned: &Owned, operation: &str, bad_frame: bool) -> (std::process::ExitStatus, String, String) {
    let project_dir = owned.project_dir.path();
    let ledger_dir = owned.ledger_dir.path();
    let input = ledger_dir.join("settlement-request.json");
    fs::write(&input, &owned.document).unwrap();
    let declaration_dir = ledger_dir.join("operator-declaration");
    fs::create_dir_all(&declaration_dir).unwrap();
    fs::set_permissions(&declaration_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let declaration = declaration_dir.join("declaration.json");
    let mut child = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args([
            "--json",
            "paea",
            "settlement-call",
            operation,
            project_dir.to_str().unwrap(),
            ledger_dir.to_str().unwrap(),
            declaration.to_str().unwrap(),
            "--input",
            input.to_str().unwrap(),
        ])
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
    let f = fixture::Fixture::build_for_trust(permission, owned.scope.clone(), owned.trust_revision);
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

/// Real authority consumes one exact commitment through the CLI: the durable
/// ledger effect row is recorded once and no frame secret ever escapes.
#[test]
#[ignore = "requires the retained canonical payment document fixture"]
fn settlement_call_cli_commits_once_and_records_effect() {
    let owned = owned();
    let (status, rest, stderr) = invoke(&owned, "commit", false);
    assert!(status.success(), "stderr: {stderr}");
    let reply: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(reply["event"], "paea-settlement-result");
    assert_eq!(reply["outcome"], "recorded");
    assert_eq!(reply["replayed"], false);
    assert_eq!(reply["invoice_id"], "invoice-17");
    assert!(reply["intent_id"].as_str().unwrap().len() == 64);
    assert!(reply["action_digest"].as_str().unwrap().len() == 64);
    assert_eq!(count(owned.ledger_dir.path()), 1);
    owned.ledger.close().unwrap();
    owned.project.close().unwrap();
}

/// Reconciliation through the CLI before any commitment reports the explicit
/// negative outcome and publishes no durable effect row.
#[test]
#[ignore = "requires the retained canonical payment document fixture"]
fn settlement_call_cli_reconciles_before_commit_reports_not_committed() {
    let owned = owned();
    let (status, rest, stderr) = invoke(&owned, "reconcile", false);
    assert!(status.success(), "stderr: {stderr}");
    let reply: Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(reply["event"], "paea-settlement-result");
    assert_eq!(reply["outcome"], "reconciled_not_committed");
    assert_eq!(reply["invoice_id"], "invoice-17");
    assert!(reply["intent_id"].as_str().unwrap().len() == 64);
    assert_eq!(count(owned.ledger_dir.path()), 0);
    owned.ledger.close().unwrap();
    owned.project.close().unwrap();
}

/// A wrong frame refuses without any preparation consumption and without any
/// business mutation; the project Trust state is unchanged and the ledger has
/// no effect row.
#[test]
#[ignore = "requires the retained canonical payment document fixture"]
fn settlement_call_cli_refuses_bad_frame_without_consumption() {
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
    owned.ledger.close().unwrap();
    owned.project.close().unwrap();
}
