#![cfg(unix)]
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use swarm_engine::{EngineSettingsChange, EngineSettingsDocument, SettingCap, SettingCount};
/// Call the actual public binary on an owned root; no provider/service is used.
fn call(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}
/// Initialize through CLI then declare one private hierarchical snapshot with
/// real projection triggers/UDFs. This is fixture data, never live mission editing.
fn fixture() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(call(root.path(), &["init", "--json"]).status.success());
    let raw=br#"{"schema_version":1,"id":"work","revision":1,"criteria":["Proof"],"planning":{"version":1,"paused":false,"failure":"Failure","max_tasks":10,"max_decisions":10,"max_activations":15,"scopes":[{"id":"root","objective":"Root","requirements":["req-1"],"revision":1,"generation":2,"holder":"fixture","lease_until":"2099-01-01T00:00:00Z","state":"blocked"}]}}"#;
    let work = swarm_engine::decode_work(raw).unwrap();
    swarm_engine::validate_planning_state(&work).unwrap();
    let db = rusqlite_connection(root.path());
    assert_eq!(
        db.execute(
            "INSERT INTO works(id,revision,body) VALUES('work',1,?1)",
            [std::str::from_utf8(raw).unwrap()]
        )
        .unwrap(),
        1
    );
    db.close().unwrap();
    root
}
/// Seed transport uses engine's existing supported projection registration via
/// a direct dev dependency; it opens only the supplied private fixture database.
fn rusqlite_connection(root: &Path) -> rusqlite::Connection {
    let db = rusqlite::Connection::open_with_flags(
        root.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .unwrap();
    db.busy_timeout(std::time::Duration::ZERO).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db
}
fn request(root: &Path, event: &str, revision: i64, reason: &str) -> std::path::PathBuf {
    let path = root.join(format!("{event}.json"));
    fs::write(&path,serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":revision,"reason":reason,"operations":[{"kind":"review-retry","id":"not-a-task","requirements":null}]})).unwrap()).unwrap();
    path
}
/// Real public show is read-only; controls and replays work without rendering a
/// derived view, even with a blocking view-directory collision. Decision metadata
/// remains passive, lease generation changes once and conflicting intent refuses.
#[test]
fn planning_cli_show_pause_resume_are_native_raw_json_and_replay_safe() {
    let root = fixture();
    let file = root.path().join(".swarm/state.db");
    let before = fs::read(&file).unwrap();
    let shown = call(
        root.path(),
        &[
            "planning",
            "show",
            "work",
            "--input",
            "missing-ignored-file",
            "--lang",
            "en",
        ],
    );
    assert!(shown.status.success());
    let work: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(work["id"], "work");
    assert_eq!(work["planning"]["scopes"][0]["generation"], 2);
    assert_eq!(fs::read(&file).unwrap(), before);
    let view = root.path().join(".swarm/views/work.md");
    fs::create_dir(&view).unwrap();
    fs::write(view.join("preserved"), b"view collision").unwrap();
    let pause = request(root.path(), "pause", 1, "");
    let args = [
        "planning",
        "pause",
        "work",
        "--input",
        pause.to_str().unwrap(),
        "--json",
    ];
    let result = call(root.path(), &args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let work: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(work["revision"], 2);
    assert_eq!(work["planning"]["paused"], true);
    assert_eq!(work["planning"]["scopes"][0]["generation"], 3);
    assert!(work["planning"]["scopes"][0].get("holder").is_none());
    assert!(work["planning"]["scopes"][0].get("lease_until").is_none());
    assert_eq!(fs::read(view.join("preserved")).unwrap(), b"view collision");
    let before = fs::read(&file).unwrap();
    let replay = call(root.path(), &args);
    assert!(replay.status.success());
    assert_eq!(replay.stdout, result.stdout);
    assert_eq!(fs::read(&file).unwrap(), before);
    let conflict = request(root.path(), "pause", 1, "Changed intent");
    let rejected = call(
        root.path(),
        &[
            "planning",
            "pause",
            "work",
            "--input",
            conflict.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(rejected.status.code(), Some(2));
    assert!(rejected.stdout.is_empty());
    assert_eq!(fs::read(&file).unwrap(), before);
    let resume = request(root.path(), "resume", 2, "Operator resumes");
    let result = call(
        root.path(),
        &[
            "planning",
            "resume",
            "work",
            "--input",
            resume.to_str().unwrap(),
            "--lang",
            "fr",
        ],
    );
    assert!(result.status.success());
    let work: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(work["revision"], 3);
    assert_eq!(work["planning"]["paused"], false);
    assert_eq!(work["planning"]["scopes"][0]["generation"], 4);
    assert_eq!(work["planning"]["inbox"][0]["kind"], "operator_resume");
    assert_eq!(work["planning"]["inbox"][0]["scope"], "root");
}
/// A saved journal is applied through the public CLI preview/CAS path, reopened
/// and consumed by the actual native command. Legacy history stays unchanged.
fn saved_cap(root: &Path, event: &str, cap: Option<i64>) {
    let shown = call(root, &["engine-settings", "show", "--json"]);
    assert!(shown.status.success());
    let doc = EngineSettingsDocument::decode(&shown.stdout).unwrap();
    let old = doc.history.clone();
    let mut values = doc.values;
    values.planning_resume_max_reason_bytes = Some(SettingCap(cap.map(SettingCount)));
    // Explicit operator adoption includes the complete v12 policy group.
    values.planning_request_max_bytes = Some(SettingCap(Some(SettingCount(65536))));
    values.planning_max_requirements = Some(SettingCap(Some(SettingCount(100))));
    let mut request = EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: event.into(),
        expected_revision: doc.revision,
        values,
        reason: "CLI saved resume reason policy".into(),
        preview_token: String::new(),
    };
    let preview = root.join(format!("{event}-preview.json"));
    fs::write(&preview, swarm_engine::encode_model(&request).unwrap()).unwrap();
    let result = call(
        root,
        &[
            "engine-settings",
            "preview",
            "--input",
            preview.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(result.status.success());
    request.preview_token = serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()
        ["preview_token"]
        .as_str()
        .unwrap()
        .into();
    let apply = root.join(format!("{event}-apply.json"));
    fs::write(&apply, swarm_engine::encode_model(&request).unwrap()).unwrap();
    assert!(call(
        root,
        &[
            "engine-settings",
            "apply",
            "--input",
            apply.to_str().unwrap(),
            "--json"
        ]
    )
    .status
    .success());
    let reopened = call(root, &["engine-settings", "show", "--json"]);
    let current = EngineSettingsDocument::decode(&reopened.stdout).unwrap();
    assert_eq!(&current.history[..old.len()], old.as_slice());
    assert_eq!(
        current.values.planning_resume_reason_limit().unwrap(),
        cap.map(|n| n as usize)
    );
}
/// Missing canonical request policy blocks new mutations until adoption. The
/// same previously refused event recovers after saved unlimited policy, without
/// reservations/consumption reset or duplicate event publication.
#[test]
fn planning_cli_saved_reason_policy_and_legacy_adoption_reach_real_consumer() {
    let root = fixture();
    fs::write(
        root.path().join(".swarm/engine-settings.json"),
        include_bytes!("../../swarm-engine/tests/fixtures/legacy-v10-document.json"),
    )
    .unwrap();
    let pause = request(root.path(), "pause", 1, "");
    let legacy_before = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let refused = call(
        root.path(),
        &[
            "planning",
            "pause",
            "work",
            "--input",
            pause.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr)
        .contains("settings-planning-request-policy-not-configured"));
    assert_eq!(
        fs::read(root.path().join(".swarm/state.db")).unwrap(),
        legacy_before
    );
    saved_cap(root.path(), "adopt-request-policy", Some(0));
    assert!(call(
        root.path(),
        &[
            "planning",
            "pause",
            "work",
            "--input",
            pause.to_str().unwrap(),
            "--json"
        ]
    )
    .status
    .success());
    let resume = request(root.path(), "resume", 2, "x");
    let file = root.path().join(".swarm/state.db");
    let before = fs::read(&file).unwrap();
    let refused = call(
        root.path(),
        &[
            "planning",
            "resume",
            "work",
            "--input",
            resume.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("planning-resume-reason-limit-exceeded")
    );
    assert_eq!(fs::read(&file).unwrap(), before);
    saved_cap(root.path(), "reason-zero", Some(0));
    let before = fs::read(&file).unwrap();
    let refused = call(
        root.path(),
        &[
            "planning",
            "resume",
            "work",
            "--input",
            resume.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(refused.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("planning-resume-reason-limit-exceeded")
    );
    assert_eq!(fs::read(&file).unwrap(), before);
    saved_cap(root.path(), "reason-unlimited", None);
    let resume = request(root.path(), "resume", 2, &"é".repeat(2500));
    let result = call(
        root.path(),
        &[
            "planning",
            "resume",
            "work",
            "--input",
            resume.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(result.status.success());
    let work: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(work["revision"], 3);
    assert_eq!(
        work["planning"]["inbox"][0]["message"]
            .as_str()
            .unwrap()
            .len(),
        5000
    );
}
/// Output or flush failure cannot hide a committed control. Preserve its safe
/// diagnostic and typed original IO cause; exact replay recovers the same result.
#[test]
fn planning_cli_preserves_committed_control_on_output_and_flush_failure() {
    for flush in [false, true] {
        let root = fixture();
        let input = request(root.path(), "pause", 1, "");
        let args = [
            "planning",
            "pause",
            "work",
            "--input",
            input.to_str().unwrap(),
            "--root",
            root.path().to_str().unwrap(),
            "--json",
        ]
        .map(String::from);
        struct Failing {
            flush: bool,
        }
        impl std::io::Write for Failing {
            fn write(&mut self, raw: &[u8]) -> std::io::Result<usize> {
                if self.flush {
                    Ok(raw.len())
                } else {
                    Err(std::io::Error::other("PRIVATE output cause"))
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                if self.flush {
                    Err(std::io::Error::other("PRIVATE flush cause"))
                } else {
                    Ok(())
                }
            }
        }
        let mut diagnostics = Vec::new();
        let outcome =
            swarm_cli::run_detailed(&args, "en", &mut Failing { flush }, &mut diagnostics);
        assert_eq!(outcome.code, 2);
        let Some(swarm_cli::CliError::PlanningStatePreserved {
            primary, revision, ..
        }) = outcome.error
        else {
            panic!("committed control must remain explicit")
        };
        assert_eq!(revision, 2);
        assert!(matches!(*primary, swarm_cli::CliError::Output(_)));
        assert!(!String::from_utf8_lossy(&diagnostics).contains("PRIVATE"));
        let recovery = call(
            root.path(),
            &[
                "planning",
                "pause",
                "work",
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(recovery.status.success());
        let work: serde_json::Value = serde_json::from_slice(&recovery.stdout).unwrap();
        assert_eq!(work["revision"], 2);
        assert_eq!(work["planning"]["scopes"][0]["generation"], 3);
    }
}
/// Public planning enable creates a new local hierarchy. Invalid budgets and
/// provider/repository paths refuse without mutation; replay does not enable twice.
#[test]
fn planning_cli_enables_new_work_and_refuses_unported_preparations() {
    let root = fixture();
    let mut store = swarm_engine::RootStore::open(root.path(), true).unwrap();
    let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"new-work","expected_revision":0,"title":"Enable","objective":"Observable","scope":"Fixture","criteria":["Proof"]}"#,&mut swarm_engine::SystemEnvironment).unwrap().result.work;
    store.close().unwrap();
    let request = serde_json::json!({"schema_version":1,"event_id":"enable","expected_revision":1,"max_tasks":5,"max_decisions":10,"max_activations":15});
    let input = root.path().join("enable.json");
    for (field, value) in [
        ("provider", serde_json::json!("unported")),
        (
            "repository",
            serde_json::json!({"path":"unported","committed_only":false}),
        ),
    ] {
        let mut bad = request.clone();
        bad[field] = value;
        fs::write(&input, serde_json::to_vec(&bad).unwrap()).unwrap();
        let before = fs::read(root.path().join(".swarm/state.db")).unwrap();
        let result = call(
            root.path(),
            &[
                "planning",
                "enable",
                &work.id,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["failure"]["code"], "command_not_ported");
        assert_eq!(
            fs::read(root.path().join(".swarm/state.db")).unwrap(),
            before
        );
    }
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let args = [
        "planning",
        "enable",
        &work.id,
        "--input",
        input.to_str().unwrap(),
        "--json",
    ];
    let result = call(root.path(), &args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let current: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(current["revision"], 2);
    assert_eq!(current["planning"]["scopes"][0]["id"], "root");
    assert_eq!(current["planning"]["judge"]["cycle"], 1);
    let before = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let replay = call(root.path(), &args);
    assert!(replay.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&replay.stdout).unwrap(),
        current
    );
    assert_eq!(
        fs::read(root.path().join(".swarm/state.db")).unwrap(),
        before
    );
}

/// Inject only a private fixture snapshot after a successful public control.
/// The public mutation must refuse invalid scope counters/limits without an event
/// or DB/settings change. Restore the pristine fixture, then retry the same intent;
/// this proves refusal/recovery, not a public repair of real consumed history.
#[test]
fn planning_controls_refuse_negative_scope_budget_without_commit_then_recover() {
    for field in ["activations", "max_tasks", "max_activations"] {
        let root = fixture();
        let valid = request(root.path(), "witness66", 1, "");
        assert!(call(
            root.path(),
            &[
                "planning",
                "pause",
                "work",
                "--input",
                valid.to_str().unwrap(),
                "--json"
            ]
        )
        .status
        .success());
        let db = rusqlite_connection(root.path());
        let original: Vec<u8> = db
            .query_row("SELECT body FROM works WHERE id='work'", [], |r| r.get(0))
            .unwrap();
        let mut corrupted: serde_json::Value = serde_json::from_slice(&original).unwrap();
        let revision = corrupted["revision"].as_i64().unwrap();
        corrupted["planning"]["scopes"][0][field] = (-1).into();
        db.execute(
            "UPDATE works SET body=?1 WHERE id='work'",
            [serde_json::to_string(&corrupted).unwrap()],
        )
        .unwrap();
        db.close().unwrap();
        let file = request(root.path(), "negative66", revision, "");
        let before = fs::read(root.path().join(".swarm/state.db")).unwrap();
        let settings = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
        let args = [
            "planning",
            "pause",
            "work",
            "--input",
            file.to_str().unwrap(),
            "--json",
        ];
        let refused = call(root.path(), &args);
        if refused.status.success() {
            let shown = call(root.path(), &["planning", "show", "work", "--json"]);
            assert!(shown.status.success());
            let state: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
            assert_eq!(
                state["revision"],
                revision + 1,
                "success did not commit as expected"
            );
            let preserved = root.keep();
            panic!(
                "negative field {field} committed revision {} on retained fixture {}",
                revision + 1,
                preserved.display()
            );
        }
        assert_eq!(refused.status.code(), Some(2));
        assert!(refused.stdout.is_empty());
        assert!(String::from_utf8_lossy(&refused.stderr).contains("planning-scope-budget-invalid"));
        assert_eq!(
            fs::read(root.path().join(".swarm/state.db")).unwrap(),
            before
        );
        assert_eq!(
            fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        let db = rusqlite_connection(root.path());
        db.execute("UPDATE works SET body=?1 WHERE id='work'", [original])
            .unwrap();
        db.close().unwrap();
        assert!(call(root.path(), &args).status.success());
    }
}
