#![cfg(target_os = "linux")]
use std::{fs, path::Path, process::Command};
use swarm_engine::{RootStore, SystemEnvironment};
/// Run the public binary against an owned fixture; no live root or provider.
fn call(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}
/// Persist through the supported native mutation path before checking CLI reads.
fn fixture() -> (tempfile::TempDir, swarm_engine::Work) {
    let root = tempfile::tempdir().unwrap();
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .arg(root.path())
        .status()
        .unwrap()
        .success());
    let mut store = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    let raw = r#"{"schema_version":1,"event_id":"view-create","expected_revision":0,"title":"Vue <&> café","objective":"Objectif","scope":"Fixture","criteria":["preuve"]}"#;
    let work = store
        .execute_external("", "work.create", raw.as_bytes(), &mut SystemEnvironment)
        .unwrap()
        .result
        .work;
    store.close().unwrap();
    (root, work)
}
/// Show reads real stored state and all envelope fields without writing a view;
/// resume publishes exactly the derived Markdown without changing the Work row.
#[test]
fn public_show_is_read_only_and_resume_publishes_real_fr_en_view() {
    let (root, work) = fixture();
    let db = root.path().join(".swarm/state.db");
    let before = fs::read(&db).unwrap();
    let settings = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    let file = root.path().join(format!(".swarm/views/{}.md", work.id));
    for language in ["fr", "en"] {
        let result = call(
            root.path(),
            &["work", "show", &work.id, "--json", "--lang", language],
        );
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stderr.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        for field in [
            "work",
            "validation",
            "events",
            "current_gates",
            "current_git",
            "resume_markdown",
        ] {
            assert!(value.get(field).is_some(), "{field}");
        }
        assert_eq!(value["work"]["id"], work.id);
        assert_eq!(value["work"]["revision"], work.revision);
        let markdown = value["resume_markdown"].as_str().unwrap();
        assert!(markdown.contains("Vue <&> café"));
        assert!(markdown.contains("## Objectif"));
        assert_eq!(value["validation"]["label"], "OUVERT");
        let previous = fs::read(&file).ok();
        let text = call(root.path(), &["work", "show", &work.id, "--lang", language]);
        assert!(text.status.success());
        if language == "fr" {
            assert_eq!(text.stdout, format!("{markdown}\n").as_bytes());
        } else {
            assert!(String::from_utf8_lossy(&text.stdout).contains("## Objective"));
        }
        assert_eq!(fs::read(&file).ok(), previous);
        let resume = call(
            root.path(),
            &["resume", &work.id, "--json", "--lang", language],
        );
        assert!(
            resume.status.success(),
            "{}",
            String::from_utf8_lossy(&resume.stderr)
        );
        assert_eq!(resume.stdout, result.stdout);
        assert_eq!(fs::read_to_string(&file).unwrap(), markdown);
        assert_eq!(fs::read(&db).unwrap(), before);
        assert_eq!(
            fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
    }
}
/// Publishing replaces a symlink entry without following its target. A directory
/// collision then refuses with all contents intact; removing it recovers the view.
#[test]
fn public_resume_replaces_link_safely_refuses_directory_and_recovers() {
    let (root, work) = fixture();
    let valid = call(root.path(), &["work", "show", &work.id, "--json"]);
    assert!(valid.status.success());
    let db = root.path().join(".swarm/state.db");
    let before = fs::read(&db).unwrap();
    let file = root.path().join(format!(".swarm/views/{}.md", work.id));
    let target = root.path().join("retained.txt");
    fs::write(&target, b"original private fixture").unwrap();
    std::os::unix::fs::symlink(&target, &file).unwrap();
    let replaced = call(root.path(), &["resume", &work.id, "--json"]);
    assert!(replaced.status.success());
    assert!(!fs::symlink_metadata(&file)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read(&target).unwrap(), b"original private fixture");
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    fs::write(file.join("preserved"), b"original").unwrap();
    let failure = call(root.path(), &["resume", &work.id, "--json"]);
    assert_eq!(failure.status.code(), Some(2));
    assert!(failure.stdout.is_empty());
    assert_eq!(fs::read(file.join("preserved")).unwrap(), b"original");
    assert_eq!(fs::read(&db).unwrap(), before);
    fs::remove_file(file.join("preserved")).unwrap();
    fs::remove_dir(&file).unwrap();
    let recovery = call(root.path(), &["resume", &work.id, "--json"]);
    assert!(recovery.status.success());
    assert_eq!(
        recovery.stdout,
        call(root.path(), &["work", "show", &work.id, "--json"]).stdout
    );
}

/// An expected-revision guard checks before replacing an existing view. The
/// read-only store rejects writes, while normal Root rendering still recovers.
#[test]
fn root_view_revision_guard_preserves_existing_view_and_read_only_contract() {
    let (root, work) = fixture();
    let file = root.path().join(format!(".swarm/views/{}.md", work.id));
    let store = RootStore::open(root.path(), true).unwrap();
    store
        .render_work_view(
            &work.id,
            Some(work.revision),
            swarm_engine::WorkViewLanguage::French,
        )
        .unwrap();
    let before = fs::read(&file).unwrap();
    let failure = store
        .render_work_view(
            &work.id,
            Some(work.revision + 1),
            swarm_engine::WorkViewLanguage::French,
        )
        .unwrap_err();
    assert_eq!(failure.code(), "view-work-revision-conflict");
    assert_eq!(fs::read(&file).unwrap(), before);
    store.close().unwrap();
    let store = RootStore::open(root.path(), false).unwrap();
    assert_eq!(
        store
            .render_work_view(&work.id, None, swarm_engine::WorkViewLanguage::French)
            .unwrap_err()
            .code(),
        "root-read-only"
    );
    store
        .inspect_work_view(&work.id, swarm_engine::WorkViewLanguage::English)
        .unwrap();
    store.close().unwrap();
}

/// Full public mutation envelopes carry actual stored revisions and raw events.
/// Invalid updates preserve state; replay recovers after a post-commit view fault.
#[test]
fn public_mutations_replay_recover_after_committed_view_failure() {
    let (root, mut work) = fixture();
    let input = root.path().join("request.json");
    let selected_id = work.id.clone();
    let operations = [
        (
            vec!["work", "update"],
            serde_json::json!({"title":"Updated <&>"}),
        ),
        (
            vec!["task", "add"],
            serde_json::json!({"id":"task-a","title":"Task","owner":"human","deliverable":"Result","criteria":["Proof"]}),
        ),
        (
            vec!["task", "update"],
            serde_json::json!({"id":"task-a","status":"blocked","blocker":"Waiting","next":"Resolve"}),
        ),
        (
            vec!["ooda"],
            serde_json::json!({"observation":"Fact","orientation":"Context","decision":"Proceed","owner":"human","next":"Implement","result":"Decision saved"}),
        ),
        (
            vec!["checkpoint"],
            serde_json::json!({"summary":"Checkpoint","next":"Continue"}),
        ),
    ];
    for (index, (args, mut request)) in operations.into_iter().enumerate() {
        request["schema_version"] = 1.into();
        request["event_id"] = format!("public-{index}").into();
        request["expected_revision"] = work.revision.into();
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        let mut command = args.clone();
        command.extend([&selected_id, "--input", input.to_str().unwrap(), "--json"]);
        let result = call(root.path(), &command);
        assert!(
            result.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(value["work"]["revision"], work.revision + 1);
        work = serde_json::from_value(value["work"].clone()).unwrap();
        assert_eq!(value["events"].as_array().unwrap().len(), index + 2);
        if index == 3 {
            assert!(value["resume_markdown"]
                .as_str()
                .unwrap()
                .contains("observation=Fact"));
        }
        let replay = call(root.path(), &command);
        assert!(replay.status.success());
        assert_eq!(replay.stdout, result.stdout);
    }
    let file = root.path().join(format!(".swarm/views/{}.md", work.id));
    let valid = call(root.path(), &["work", "show", &work.id, "--json"]);
    assert!(valid.status.success());
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    let request = serde_json::json!({"schema_version":1,"event_id":"committed-view-fault","expected_revision":work.revision,"title":"Committed despite view fault"});
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let command = [
        "work",
        "update",
        &work.id,
        "--input",
        input.to_str().unwrap(),
        "--json",
    ];
    let failed = call(root.path(), &command);
    assert_eq!(failed.status.code(), Some(2));
    assert!(failed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("enregistré"));
    let store = RootStore::open(root.path(), false).unwrap();
    let committed = store.read_work(&work.id).unwrap();
    assert_eq!(committed.revision, work.revision + 1);
    assert_eq!(committed.title, "Committed despite view fault");
    store.close().unwrap();
    fs::remove_dir(&file).unwrap();
    let recovery = call(root.path(), &command);
    assert!(recovery.status.success());
    let recovered: serde_json::Value = serde_json::from_slice(&recovery.stdout).unwrap();
    assert_eq!(recovered["work"]["revision"], committed.revision);
    assert!(file.is_file());
    let before = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let invalid = serde_json::json!({"schema_version":1,"event_id":"invalid-update","expected_revision":committed.revision,"title":""});
    fs::write(&input, serde_json::to_vec(&invalid).unwrap()).unwrap();
    // An unknown field is rejected before publication, independently of optional title semantics.
    let mut invalid = invalid;
    invalid["unexpected"] = true.into();
    fs::write(&input, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert_eq!(call(root.path(), &command).status.code(), Some(2));
    assert_eq!(
        fs::read(root.path().join(".swarm/state.db")).unwrap(),
        before
    );
    let list = call(root.path(), &["work", "list", "--lang", "en"]);
    assert!(list.status.success());
    let text = String::from_utf8(list.stdout.clone()).unwrap();
    assert!(text.contains("Committed despite view fault"));
    assert!(text.contains("BLOCKED"));
    assert_eq!(
        list.stdout,
        call(root.path(), &["resume", "--lang", "en"]).stdout
    );
}

/// A writer failure after commit stays a failed command with the original cause
/// and the committed revision. The same event recovers without duplicating state.
#[test]
fn failed_output_after_mutation_retains_committed_revision_and_io_cause() {
    let (root, work) = fixture();
    let input = root.path().join("request.json");
    fs::write(&input,serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"output-fault","expected_revision":work.revision,"title":"Committed output failure"})).unwrap()).unwrap();
    let args = [
        "work",
        "update",
        &work.id,
        "--input",
        input.to_str().unwrap(),
        "--root",
        root.path().to_str().unwrap(),
        "--json",
    ]
    .map(String::from);
    struct Denied;
    impl std::io::Write for Denied {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("private-output-cause"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut diagnostics = Vec::new();
    let outcome = swarm_cli::run_detailed(&args, "en", &mut Denied, &mut diagnostics);
    assert_eq!(outcome.code, 2);
    let Some(swarm_cli::CliError::StatePreserved {
        primary, revision, ..
    }) = outcome.error
    else {
        panic!("committed revision required")
    };
    assert_eq!(revision, work.revision + 1);
    assert!(matches!(*primary, swarm_cli::CliError::Output(_)));
    assert!(!String::from_utf8_lossy(&diagnostics).contains("private-output-cause"));
    let recovery = call(
        root.path(),
        &[
            "work",
            "update",
            &work.id,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(recovery.status.success());
    let value: serde_json::Value = serde_json::from_slice(&recovery.stdout).unwrap();
    assert_eq!(value["work"]["revision"], revision);
}

/// Buffered output may accept every byte and still fail its flush. Preserve the
/// original transport cause and committed state; an exact replay must recover.
#[test]
fn flush_failure_after_commit_is_not_reported_as_success() {
    let (root, work) = fixture();
    let input = root.path().join("request.json");
    fs::write(&input,serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"flush-fault","expected_revision":work.revision,"title":"Committed flush failure"})).unwrap()).unwrap();
    let args = [
        "work",
        "update",
        &work.id,
        "--input",
        input.to_str().unwrap(),
        "--root",
        root.path().to_str().unwrap(),
        "--json",
    ]
    .map(String::from);
    struct FlushDenied(Vec<u8>);
    impl std::io::Write for FlushDenied {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("private-flush-cause"))
        }
    }
    let mut writer = FlushDenied(Vec::new());
    let mut diagnostics = Vec::new();
    let outcome = swarm_cli::run_detailed(&args, "en", &mut writer, &mut diagnostics);
    assert_eq!(outcome.code, 2);
    assert!(!writer.0.is_empty());
    let Some(swarm_cli::CliError::StatePreserved {
        primary, revision, ..
    }) = outcome.error
    else {
        panic!("committed revision required")
    };
    assert!(matches!(*primary, swarm_cli::CliError::Output(_)));
    assert_eq!(revision, work.revision + 1);
    assert!(!String::from_utf8_lossy(&diagnostics).contains("private-flush-cause"));
    let recovery = call(
        root.path(),
        &[
            "work",
            "update",
            &work.id,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(recovery.status.success());
    let value: serde_json::Value = serde_json::from_slice(&recovery.stdout).unwrap();
    assert_eq!(value["work"]["revision"], revision);
}

/// Real saved limits produce a safe actionable refusal, not an opaque format
/// success. Raising policy recovers the same unused event with no lost history.
#[test]
fn saved_criterion_limit_refusal_keeps_safe_reason_and_recovers_after_policy_change() {
    let (root, work) = fixture();
    let change = |event: &str, value: i64| {
        let store = RootStore::open(root.path(), true).unwrap();
        let doc = store.settings().unwrap();
        let mut values = doc.values;
        values.task_max_criteria = Some(swarm_engine::SettingCount(value));
        let mut request = swarm_engine::EngineSettingsChange {
            schema_version: values.schema_version(),
            event_id: event.into(),
            expected_revision: doc.revision,
            values,
            reason: "Saved criterion limit at actual CLI".into(),
            preview_token: String::new(),
        };
        request.preview_token = store.preview_settings(&request).unwrap().preview_token;
        store
            .apply_settings(&request, &mut SystemEnvironment)
            .unwrap();
        store.close().unwrap();
    };
    change("limit-two", 2);
    let input = root.path().join("request.json");
    let control = serde_json::json!({"schema_version":1,"event_id":"count-control","expected_revision":work.revision,"id":"control","title":"Task","deliverable":"Report","criteria":["A","B"]});
    fs::write(&input, serde_json::to_vec(&control).unwrap()).unwrap();
    let args = [
        "task",
        "add",
        &work.id,
        "--input",
        input.to_str().unwrap(),
        "--json",
    ];
    let valid = call(root.path(), &args);
    assert!(valid.status.success());
    let valid: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    let revision = valid["work"]["revision"].as_i64().unwrap();
    let bad = serde_json::json!({"schema_version":1,"event_id":"count-recovery","expected_revision":revision,"id":"three","title":"Task","deliverable":"Report","criteria":["A","B","private-critical-criterion"]});
    fs::write(&input, serde_json::to_vec(&bad).unwrap()).unwrap();
    let db = root.path().join(".swarm/state.db");
    let before = fs::read(&db).unwrap();
    let denied = call(root.path(), &args);
    assert_eq!(denied.status.code(), Some(2));
    assert!(denied.stdout.is_empty());
    let failure: serde_json::Value = serde_json::from_slice(&denied.stderr).unwrap();
    assert!(failure["failure"]["message"]
        .as_str()
        .unwrap()
        .contains("task-criteria-invalid"));
    assert!(!String::from_utf8_lossy(&denied.stderr).contains("private-critical-criterion"));
    assert_eq!(fs::read(&db).unwrap(), before);
    change("limit-three", 3);
    let recovery = call(root.path(), &args);
    assert!(recovery.status.success());
    let value: serde_json::Value = serde_json::from_slice(&recovery.stdout).unwrap();
    assert_eq!(value["work"]["revision"], revision + 1);
}

/// Real settings publication changes only future manual phase defaults. Existing
/// task requirements survive, criteria stay bound even for an explicit empty
/// phase set, and malformed choices never mutate private fixture state/history.
/// Uses actual CLI/Root operations without a provider or acceptance claim.
#[test]
fn saved_manual_gate_defaults_apply_to_new_tasks_without_weakening_existing_contracts() {
    let (root, work) = fixture();
    let input = root.path().join("phase-request.json");
    let add = |id: &str, revision: i64| -> serde_json::Value {
        let request = serde_json::json!({"schema_version":1,"event_id":format!("phase-{id}"),"expected_revision":revision,"id":id,"title":"Task","deliverable":"Report","criteria":["Proof"]});
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        let output = call(
            root.path(),
            &[
                "task",
                "add",
                &work.id,
                "--input",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };
    let configure = |event: &str, phases: serde_json::Value| {
        let store = RootStore::open(root.path(), true).unwrap();
        let document = store.settings().unwrap();
        let mut values = serde_json::to_value(&document.values).unwrap();
        values["task_manual_required_gate_phases"] = phases;
        let values =
            swarm_engine::EngineSettingsValues::decode(&serde_json::to_vec(&values).unwrap())
                .expect("manual phase defaults must be an authoritative persisted setting");
        let mut request = swarm_engine::EngineSettingsChange {
            schema_version: values.schema_version(),
            event_id: event.into(),
            expected_revision: document.revision,
            values,
            reason: "Explicit defaults for future manual tasks".into(),
            preview_token: String::new(),
        };
        request.preview_token = store.preview_settings(&request).unwrap().preview_token;
        store
            .apply_settings(&request, &mut SystemEnvironment)
            .unwrap();
        store.close().unwrap();
    };
    let first = add("before", work.revision);
    let original = first["work"]["tasks"][0]["plan_checks"].clone();
    assert_eq!(original.as_object().unwrap().len(), 4);
    configure("only-validation", serde_json::json!(["validation"]));
    let second = add("after", first["work"]["revision"].as_i64().unwrap());
    assert_eq!(second["work"]["tasks"][0]["plan_checks"], original);
    let checks = second["work"]["tasks"][1]["plan_checks"]
        .as_object()
        .unwrap();
    assert_eq!(checks.len(), 2);
    assert_eq!(checks["plan-validation"], "validation");
    assert_eq!(checks["plan-criterion-1"], "validation");
    configure("no-phase-defaults", serde_json::json!([]));
    let third = add("empty", second["work"]["revision"].as_i64().unwrap());
    assert_eq!(third["work"]["tasks"][0]["plan_checks"], original);
    assert_eq!(
        third["work"]["tasks"][1]["plan_checks"],
        second["work"]["tasks"][1]["plan_checks"]
    );
    assert_eq!(
        third["work"]["tasks"][2]["plan_checks"],
        serde_json::json!({"plan-criterion-1":"validation"})
    );
    configure(
        "restore-all-phase-defaults",
        serde_json::json!(["entry", "validation", "delivery"]),
    );
    let rename = serde_json::json!({"schema_version":1,"event_id":"phase-rename","expected_revision":third["work"]["revision"],"id":"empty","title":"Renamed"});
    fs::write(&input, serde_json::to_vec(&rename).unwrap()).unwrap();
    let renamed = call(
        root.path(),
        &[
            "task",
            "update",
            &work.id,
            "--input",
            input.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(renamed.status.success());
    let renamed: serde_json::Value = serde_json::from_slice(&renamed.stdout).unwrap();
    assert_eq!(
        renamed["work"]["tasks"][2]["plan_checks"],
        third["work"]["tasks"][2]["plan_checks"]
    );
    let settings = root.path().join(".swarm/engine-settings.json");
    let database = root.path().join(".swarm/state.db");
    let before = (fs::read(&settings).unwrap(), fs::read(&database).unwrap());
    for phases in [
        serde_json::json!(["unknown"]),
        serde_json::json!(["Validation"]),
        serde_json::json!([""]),
        serde_json::json!(null),
        serde_json::json!(true),
    ] {
        let store = RootStore::open(root.path(), true).unwrap();
        let mut values = serde_json::to_value(&store.settings().unwrap().values).unwrap();
        values["task_manual_required_gate_phases"] = phases;
        assert!(
            swarm_engine::EngineSettingsValues::decode(&serde_json::to_vec(&values).unwrap())
                .is_err()
        );
        store.close().unwrap();
        assert_eq!(
            (fs::read(&settings).unwrap(), fs::read(&database).unwrap()),
            before
        );
    }
}

/// A real archived v8 settings document stays readable with unchanged history.
/// Actual CLI creation refuses absent phase policy without writes, then resumes
/// the same event after explicit v9 adoption. The fixture is isolated; historical
/// producer bytes establish no actor authority or acceptance of existing work.
#[test]
fn legacy_manual_phase_policy_requires_explicit_adoption_before_new_creation() {
    let (root, work) = fixture();
    let settings = root.path().join(".swarm/engine-settings.json");
    let legacy = include_bytes!("../../swarm-engine/tests/fixtures/legacy-v8-document.json");
    fs::write(&settings, legacy).unwrap();
    let database = root.path().join(".swarm/state.db");
    let before = (fs::read(&database).unwrap(), fs::read(&settings).unwrap());
    let shown = call(root.path(), &["work", "show", &work.id, "--json"]);
    assert!(shown.status.success());
    let input = root.path().join("legacy-phase-input.json");
    let request = serde_json::json!({"schema_version":1,"event_id":"legacy-phase-adopt","expected_revision":work.revision,"id":"new-task","title":"Task","deliverable":"Report","criteria":["Proof"]});
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let args = [
        "task",
        "add",
        &work.id,
        "--input",
        input.to_str().unwrap(),
        "--json",
    ];
    let denied = call(root.path(), &args);
    assert_eq!(denied.status.code(), Some(2));
    assert!(denied.stdout.is_empty());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("task-gate-defaults-not-configured"));
    assert_eq!(
        (fs::read(&database).unwrap(), fs::read(&settings).unwrap()),
        before
    );
    let store = RootStore::open(root.path(), true).unwrap();
    let previous = store.settings().unwrap();
    assert_eq!(previous.schema_version, 8);
    // Use actual captured v9 values; future installation defaults cannot upgrade this case.
    let values = swarm_engine::EngineSettingsDocument::decode(include_bytes!(
        "../../swarm-engine/tests/fixtures/legacy-v9-document.json"
    ))
    .unwrap()
    .values;
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: "adopt-v9-manual-phases".into(),
        expected_revision: previous.revision,
        values,
        reason: "Explicitly adopt phase defaults without rewriting history".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
    let adopted = store.settings().unwrap();
    assert_eq!(adopted.schema_version, 9);
    assert_eq!(adopted.history.len(), previous.history.len() + 1);
    assert_eq!(
        &adopted.history[..previous.history.len()],
        previous.history.as_slice()
    );
    store.close().unwrap();
    let resumed = call(root.path(), &args);
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(value["work"]["revision"], work.revision + 1);
    assert_eq!(
        value["work"]["tasks"][0]["plan_checks"]
            .as_object()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(call(root.path(), &args).stdout, resumed.stdout);
}
