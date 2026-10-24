#![cfg(target_os = "linux")]
use std::io;
#[test]
fn native_bootstrap_cli_rejects_incomplete_identity_without_creating_a_project() {
    let mut out = Vec::new();
    let mut error = Vec::new();
    let code = swarm_cli::run(&["identity".into()], "fr", &mut out, &mut error);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(!error.is_empty());
}
#[test]
fn output_failure_returns_nonzero_and_reports_the_cause() {
    struct Denied;
    impl io::Write for Denied {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(28))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut error = Vec::new();
    assert_eq!(
        swarm_cli::run(&["--help".into()], "fr", &mut Denied, &mut error),
        2
    );
    assert!(!error.is_empty());
}
#[test]
fn option_values_are_not_reinterpreted_as_global_flags_and_english_errors_match() {
    let mut out = Vec::new();
    let mut error = Vec::new();
    let args = [
        "identity",
        "bootstrap-check",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "--input",
        "--json",
    ]
    .map(String::from);
    assert_eq!(swarm_cli::run(&args, "fr", &mut out, &mut error), 2);
    assert!(error.starts_with(b"Erreur :"));
    error.clear();
    assert_eq!(
        swarm_cli::run(
            &["identity".into(), "--lang".into(), "en".into()],
            "fr",
            &mut out,
            &mut error
        ),
        2
    );
    assert!(error.starts_with(b"Error:"));
}
#[test]
fn diagnostic_write_failure_preserves_both_causes_without_echoing_private_io_data() {
    struct Denied;
    impl io::Write for Denied {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("private-writer-value"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let result = swarm_cli::run_detailed(&["--help".into()], "fr", &mut Denied, &mut Denied);
    assert_eq!(result.code, 2);
    assert!(result.error.is_some());
    assert!(result.diagnostic_error.is_some());
    assert!(!format!("{result:?}").contains("private-writer-value"));
}
fn fixture() -> (tempfile::TempDir, String, String) {
    use sha2::{Digest, Sha256};
    use std::{fs, os::unix::fs::PermissionsExt};
    let root = tempfile::TempDir::new().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["bootstrap", "configuration", "witness"] {
        fs::create_dir(root.path().join(name)).unwrap();
        fs::set_permissions(root.path().join(name), fs::Permissions::from_mode(0o700)).unwrap();
    }
    let document = swarm_enrollment_bootstrap::Document {
        schema_version: 1,
        pins: swarm_enrollment_config::Pins {
            installation_digest: "a".repeat(64),
            public_key: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc".into(),
            principal: swarm_identity_contract::Principal {
                id: "operator".into(),
                kind: "human".into(),
                issuer: "https://issuer.example".into(),
                subject: "fixture <human>\u{2028}".into(),
            },
        },
        service_uid: rustix::process::geteuid().as_raw(),
        controller_uid: !rustix::process::geteuid().as_raw(),
        configuration_directory: root.path().join("configuration").to_str().unwrap().into(),
        witness_directory: root.path().join("witness").to_str().unwrap().into(),
    };
    // The subject ends with a non-space marker so Text's edge-space rule remains valid.
    let mut document = document;
    document.pins.principal.subject.push('x');
    let raw = swarm_enrollment_bootstrap::encode(&document).unwrap();
    let path = root.path().join("bootstrap/bootstrap.json");
    fs::write(&path, &raw).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    (
        root,
        path.to_str().unwrap().into(),
        format!("{:x}", Sha256::digest(raw)),
    )
}
#[test]
fn real_native_binary_observes_storage_without_fabricating_human_or_service_readiness() {
    let (root, path, digest) = fixture();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args([
            "identity",
            "bootstrap-check",
            &digest,
            "--input",
            &path,
            "--json",
        ])
        .env("SWARM_LANG", "en")
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["bootstrap_digest"], digest);
    assert_eq!(value["storage_verified"], true);
    for key in [
        "human_authentication_verified",
        "confinement_verified",
        "service_started",
    ] {
        assert_eq!(value[key], false);
    }
    assert_eq!(
        value["bootstrap"]["pins"]["bootstrap_principal"]["subject"],
        "fixture <human>\u{2028}x"
    );
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("<human>\\u2028x"));
    assert!(!root.path().join(".swarm").exists());
    for name in ["configuration", "witness"] {
        assert_eq!(
            std::fs::read_dir(root.path().join(name)).unwrap().count(),
            0
        );
    }
}
#[test]
fn successful_command_with_output_failure_preserves_os_cause_and_storage_classification() {
    struct Full;
    impl io::Write for Full {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(28))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let (_root, path, digest) = fixture();
    let mut error = Vec::new();
    let args = vec![
        "identity".into(),
        "bootstrap-check".into(),
        digest,
        "--input".into(),
        path,
        "--json".into(),
    ];
    let result = swarm_cli::run_detailed(&args, "en", &mut Full, &mut error);
    assert_eq!(result.code, 2);
    assert!(result.diagnostic_error.is_none());
    let original = result.error.unwrap();
    assert_eq!(original.code(), "storage_unavailable");
    assert!(std::error::Error::source(&original).is_some());
    let value: serde_json::Value = serde_json::from_slice(&error).unwrap();
    assert_eq!(value["failure"]["code"], "storage_unavailable");
    assert_eq!(value["failure"]["retryable"], false);
}
#[test]
fn language_option_order_and_missing_values_preserve_the_public_parser_contract() {
    for (args, language, code, pattern) in [
        (vec!["--help"], "fr", 0, "migration partielle"),
        (vec!["--lang=en", "--help"], "fr", 0, "partial migration"),
        (
            vec!["--lang=en", "--lang", "fr", "--help"],
            "xx",
            0,
            "migration partielle",
        ),
        (vec!["--lang", "en", "--input"], "fr", 2, "missing value:"),
        (vec!["--json", "--input"], "fr", 2, "valeur manquante :"),
        (vec!["--lang"], "fr", 2, "--lang requires fr or en"),
        (
            vec!["--lang", "xx", "--help"],
            "fr",
            2,
            "unsupported language",
        ),
    ] {
        let args = args.into_iter().map(String::from).collect::<Vec<_>>();
        let mut out = Vec::new();
        let mut error = Vec::new();
        assert_eq!(swarm_cli::run(&args, language, &mut out, &mut error), code);
        let bytes = if code == 0 { out } else { error };
        assert!(String::from_utf8(bytes).unwrap().contains(pattern));
    }
}
#[test]
fn invalid_os_argument_is_refused_and_failed_diagnostic_is_retained() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let args = vec![OsString::from_vec(vec![0xff])];
    let mut out = Vec::new();
    let mut error = Vec::new();
    let result = swarm_cli::run_os_detailed(&args, "fr", &mut out, &mut error);
    assert_eq!(result.code, 2);
    assert!(matches!(
        result.error,
        Some(swarm_cli::CliError::ArgumentEncoding)
    ));
    assert!(out.is_empty());
    assert!(error.starts_with(b"arguments must be valid UTF-8"));
    struct Denied;
    impl io::Write for Denied {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(5))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let result = swarm_cli::run_os_detailed(&args, "fr", &mut out, &mut Denied);
    assert_eq!(result.code, 2);
    assert!(result.error.is_some());
    assert!(result.diagnostic_error.is_some());
}

#[test]
fn native_root_settings_cli_persists_reopens_previews_and_replays_without_go() {
    use std::fs;
    let root = tempfile::TempDir::new().unwrap();
    fn invoke(args: Vec<String>) -> (u8, serde_json::Value) {
        let mut out = Vec::new();
        let mut error = Vec::new();
        let code = swarm_cli::run(&args, "fr", &mut out, &mut error);
        let raw = if code == 0 { out } else { error };
        (code, serde_json::from_slice(&raw).unwrap())
    }
    let args = |parts: &[&str]| {
        let mut a = parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        a.extend([
            "--root".into(),
            root.path().to_str().unwrap().into(),
            "--json".into(),
        ]);
        a
    };
    assert_eq!(invoke(args(&["init"])).0, 0);
    let (code, doc) = invoke(args(&["engine-settings", "show"]));
    assert_eq!(code, 0);
    assert_eq!(doc["revision"], "1");
    let path = root.path().join("request.json");
    let mut values = doc["values"].clone();
    values["task_max_attempts"] = "9007199254740993".into();
    let mut change = serde_json::json!({"schema_version":doc["schema_version"],"event_id":"configure","expected_revision":"1","values":values,"reason":"Operator selects exact value"});
    fs::write(&path, serde_json::to_vec(&change).unwrap()).unwrap();
    let before = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    let (code, preview) = invoke(args(&[
        "engine-settings",
        "preview",
        "--input",
        path.to_str().unwrap(),
    ]));
    assert_eq!(code, 0);
    assert_eq!(preview["values"]["task_max_attempts"], "9007199254740993");
    assert_eq!(
        fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    // Apply without preview token refuses with the original document intact.
    assert_eq!(
        invoke(args(&[
            "engine-settings",
            "apply",
            "--input",
            path.to_str().unwrap()
        ]))
        .0,
        2
    );
    assert_eq!(
        fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    change["preview_token"] = preview["preview_token"].clone();
    fs::write(&path, serde_json::to_vec(&change).unwrap()).unwrap();
    let (code, applied) = invoke(args(&[
        "engine-settings",
        "apply",
        "--input",
        path.to_str().unwrap(),
    ]));
    assert_eq!(code, 0);
    assert_eq!(applied["replayed"], false);
    assert_eq!(applied["document"]["revision"], "2");
    let (code, reopened) = invoke(args(&["engine-settings", "show"]));
    assert_eq!(code, 0);
    assert_eq!(reopened["values"]["task_max_attempts"], "9007199254740993");
    let (code, replayed) = invoke(args(&[
        "engine-settings",
        "apply",
        "--input",
        path.to_str().unwrap(),
    ]));
    assert_eq!(code, 0);
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["document"]["history"].as_array().unwrap().len(), 2);
}
#[test]
fn settings_publication_survives_lost_cli_output_and_replays_once() {
    use std::fs;
    let root = tempfile::TempDir::new().unwrap();
    let mut env = swarm_engine::SystemEnvironment;
    let store = swarm_engine::RootStore::initialize(root.path(), &mut env).unwrap();
    let doc = store.settings().unwrap();
    let mut r = swarm_engine::EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "response-loss".into(),
        expected_revision: doc.revision,
        values: doc.values,
        reason: "Operator changes retry delay".into(),
        preview_token: String::new(),
    };
    r.values.storage_busy_retry_delay_ms = swarm_engine::SettingCount(33);
    r.preview_token = store.preview_settings(&r).unwrap().preview_token;
    store.close().unwrap();
    let input = root.path().join("request.json");
    fs::write(&input, serde_json::to_vec(&r).unwrap()).unwrap();
    let args = [
        "engine-settings",
        "apply",
        "--root",
        root.path().to_str().unwrap(),
        "--input",
        input.to_str().unwrap(),
        "--json",
    ]
    .map(String::from);
    struct Full;
    impl io::Write for Full {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(28))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut errors = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut Full, &mut errors), 2);
    let mut out = Vec::new();
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut errors), 0);
    let result: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(result["replayed"], true);
    assert_eq!(result["document"]["history"].as_array().unwrap().len(), 2);
    assert_eq!(
        result["document"]["values"]["storage_busy_retry_delay_ms"],
        "33"
    );
}

#[test]
fn settings_catalogue_needs_no_project_and_configuration_errors_keep_safe_reason_codes() {
    let root = tempfile::TempDir::new().unwrap();
    let mut out = Vec::new();
    let mut error = Vec::new();
    let args = [
        "engine-settings",
        "describe",
        "--root",
        root.path().to_str().unwrap(),
        "--json",
    ]
    .map(String::from);
    assert_eq!(swarm_cli::run(&args, "en", &mut out, &mut error), 0);
    let catalog: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        catalog["fields"].as_array().unwrap().len(),
        catalog["defaults"].as_object().unwrap().len()
    );
    assert!(!root.path().join(".swarm").exists());
    let store =
        swarm_engine::RootStore::initialize(root.path(), &mut swarm_engine::SystemEnvironment)
            .unwrap();
    let doc = store.settings().unwrap();
    store.close().unwrap();
    let mut values = doc.values;
    values.task_max_attempts = Some(swarm_engine::SettingCount(0));
    let change = swarm_engine::EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "bad-zero".into(),
        expected_revision: doc.revision,
        values,
        reason: "PRIVATE fixture reason".into(),
        preview_token: String::new(),
    };
    let input = root.path().join("request.json");
    std::fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    let before = std::fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    out.clear();
    error.clear();
    let args = [
        "engine-settings",
        "preview",
        "--root",
        root.path().to_str().unwrap(),
        "--input",
        input.to_str().unwrap(),
        "--json",
    ]
    .map(String::from);
    assert_eq!(swarm_cli::run(&args, "fr", &mut out, &mut error), 2);
    assert!(out.is_empty());
    let failure: serde_json::Value = serde_json::from_slice(&error).unwrap();
    assert_eq!(
        failure["failure"]["code"],
        "settings-task-cap-positive-or-unlimited"
    );
    assert!(!String::from_utf8(error).unwrap().contains("PRIVATE"));
    assert_eq!(
        std::fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
        before
    );
}

#[test]
fn settings_file_framing_fifo_device_and_exact_boundary_refuse_or_recover_without_mutation() {
    use std::fs;
    let root = tempfile::TempDir::new().unwrap();
    let store =
        swarm_engine::RootStore::initialize(root.path(), &mut swarm_engine::SystemEnvironment)
            .unwrap();
    let doc = store.settings().unwrap();
    store.close().unwrap();
    let request = serde_json::json!({"schema_version":doc.schema_version,"event_id":"input-control","expected_revision":doc.revision,"values":doc.values,"reason":"Input framing proof"});
    let raw = serde_json::to_vec(&request).unwrap();
    let file = root.path().join("request.json");
    fs::write(&file, &raw).unwrap();
    let invoke = |path: &std::path::Path| {
        let args = [
            "engine-settings",
            "preview",
            "--root",
            root.path().to_str().unwrap(),
            "--input",
            path.to_str().unwrap(),
            "--json",
        ]
        .map(String::from);
        let mut out = Vec::new();
        let mut error = Vec::new();
        let code = swarm_cli::run(&args, "en", &mut out, &mut error);
        (
            code,
            serde_json::from_slice::<serde_json::Value>(if code == 0 { &out } else { &error })
                .unwrap(),
        )
    };
    let (code, control) = invoke(&file);
    assert_eq!(code, 0);
    let original = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    let mut boundary = raw.clone();
    boundary.resize(swarm_engine::COMMAND_INPUT_MAX_BYTES, b' ');
    fs::write(&file, &boundary).unwrap();
    let (code, preview) = invoke(&file);
    assert_eq!(code, 0);
    assert_eq!(preview["preview_token"], control["preview_token"]);
    boundary.push(b' ');
    fs::write(&file, &boundary).unwrap();
    let (code, error) = invoke(&file);
    assert_eq!(code, 2);
    assert_eq!(error["failure"]["code"], "input-document-too-large");
    fs::write(&file, &raw).unwrap();
    assert_eq!(invoke(&file).0, 0);
    let directory = fs::File::open(root.path()).unwrap();
    rustix::fs::mkfifoat(
        &directory,
        "request.fifo",
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    drop(directory);
    for path in [
        root.path().join("request.fifo"),
        std::path::PathBuf::from("/dev/zero"),
    ] {
        assert_eq!(invoke(&file).0, 0);
        let (code, error) = invoke(&path);
        assert_eq!(code, 2);
        assert_eq!(error["failure"]["code"], "input-regular-file-required");
        assert_eq!(invoke(&file).0, 0);
        assert_eq!(
            fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
            original
        );
    }
}
#[test]
fn native_binary_supports_explicit_finite_stdin_for_settings_without_publishing_preview() {
    use std::{
        fs,
        io::Write,
        process::{Command, Stdio},
    };
    let root = tempfile::TempDir::new().unwrap();
    let store =
        swarm_engine::RootStore::initialize(root.path(), &mut swarm_engine::SystemEnvironment)
            .unwrap();
    let doc = store.settings().unwrap();
    store.close().unwrap();
    let request = serde_json::json!({"schema_version":doc.schema_version,"event_id":"stdin-control","expected_revision":doc.revision,"values":doc.values,"reason":"Finite explicit input stream"});
    let before = fs::read(root.path().join(".swarm/engine-settings.json")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args([
            "engine-settings",
            "preview",
            "--root",
            root.path().to_str().unwrap(),
            "--input",
            "-",
            "--json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let preview: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(preview["current_revision"], "1");
    assert!(preview["preview_token"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert_eq!(
        fs::read(root.path().join(".swarm/engine-settings.json")).unwrap(),
        before
    );
}

#[test]
fn quality_profile_cli_is_readonly_and_legacy_configuration_has_no_implicit_bar() {
    let root = tempfile::TempDir::new().unwrap();
    let call = |parts: &[&str]| {
        let mut args = parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        args.extend([
            "--root".into(),
            root.path().to_str().unwrap().into(),
            "--json".into(),
        ]);
        let mut out = Vec::new();
        let mut error = Vec::new();
        let code = swarm_cli::run(&args, "fr", &mut out, &mut error);
        (code, if code == 0 { out } else { error })
    };
    assert_ne!(call(&["engine-settings", "quality-profile"]).0, 0);
    assert!(!root.path().join(".swarm").exists());
    assert_eq!(call(&["init"]).0, 0);
    let file = root.path().join(".swarm/engine-settings.json");
    let before = std::fs::read(&file).unwrap();
    let (code, bytes) = call(&["engine-settings", "quality-profile"]);
    assert_eq!(code, 0);
    let profile: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(profile["schema_version"], 1);
    assert_eq!(profile["generation"], "0");
    assert_eq!(profile["legacy_method2_thresholds_compatible"], true);
    assert_eq!(std::fs::read(&file).unwrap(), before);
    std::fs::write(
        &file,
        include_bytes!("../../swarm-engine/tests/fixtures/legacy-v2-document.json"),
    )
    .unwrap();
    let old = std::fs::read(&file).unwrap();
    let (code, bytes) = call(&["engine-settings", "quality-profile"]);
    assert_eq!(code, 2);
    let failure: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        failure["failure"]["code"],
        "settings-quality-policy-not-configured"
    );
    assert_eq!(std::fs::read(&file).unwrap(), old);
}

#[test]
fn native_evaluate_legacy_is_readonly_and_business_refusal_keeps_json_without_error() {
    use sha2::{Digest, Sha256};
    let root = tempfile::TempDir::new().unwrap();
    std::fs::write(root.path().join("proof.txt"), b"proof").unwrap();
    let mut doc = serde_json::json!({"method_version":"2","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":[{"id":"score","domain":"quality","mandatory":true,"gate":"delivery","severity":"critical","penalty":20,"max_penalty":100}],"results":[{"id":"score","status":"PASS","count":0,"evidence":["proof.txt"]}]});
    let file = root.path().join("gate.json");
    let args = vec![
        "evaluate".into(),
        "--root".into(),
        root.path().to_str().unwrap().into(),
        "--input".into(),
        file.to_str().unwrap().into(),
        "--json".into(),
    ];
    for (status, count, expected) in [
        ("PASS", 0, 0),
        ("FAIL", 1, 1),
        ("FAIL|PARTIEL", 1, 2),
        ("PASS", 0, 0),
    ] {
        doc["results"][0]["status"] = status.into();
        doc["results"][0]["count"] = count.into();
        std::fs::write(&file, serde_json::to_vec(&doc).unwrap()).unwrap();
        let before = std::fs::read(&file).unwrap();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = swarm_cli::run_detailed(&args, "fr", &mut out, &mut err);
        assert_eq!(result.code, expected);
        if expected == 2 {
            assert!(result.error.is_some());
            assert!(out.is_empty());
            assert!(!err.is_empty())
        } else {
            assert!(result.error.is_none());
            assert!(err.is_empty());
            let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
            assert_eq!(value["ship_allowed"], expected == 0)
        }
        assert_eq!(std::fs::read(&file).unwrap(), before);
        assert!(!root.path().join(".swarm").exists());
    }
}

#[test]
fn native_evaluate_uses_saved_quality_policy_and_reports_output_failure() {
    use sha2::{Digest, Sha256};
    use std::io;
    let root = tempfile::TempDir::new().unwrap();
    let call = |parts: &[&str]| {
        let mut args = parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        args.extend([
            "--root".into(),
            root.path().to_str().unwrap().into(),
            "--json".into(),
        ]);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = swarm_cli::run_detailed(&args, "fr", &mut out, &mut err);
        (result, out, err)
    };
    assert_eq!(call(&["init"]).0.code, 0);
    std::fs::write(root.path().join("proof.txt"), b"proof").unwrap();
    let gate = root.path().join("gate.json");
    std::fs::write(&gate,serde_json::to_vec(&serde_json::json!({"method_version":"3","scope_id":"task","artifacts":{"proof.txt":format!("{:x}",Sha256::digest(b"proof"))},"domains":{"quality":1},"checks":[{"id":"score","domain":"quality","mandatory":false,"gate":"delivery","severity":"minor","penalty":20,"max_penalty":100}],"results":[{"id":"score","status":"FAIL","count":1,"evidence":["proof.txt"]}]})).unwrap()).unwrap();
    let evaluate = ["evaluate", "--input", gate.to_str().unwrap()];
    let (before, out, err) = call(&evaluate);
    assert_eq!(before.code, 0);
    assert!(err.is_empty());
    let first: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(first["evaluation"]["quality"], 80.0);
    let (_, doc, _) = call(&["engine-settings", "show"]);
    let doc: serde_json::Value = serde_json::from_slice(&doc).unwrap();
    let mut change = serde_json::json!({"schema_version":doc["schema_version"],"event_id":"raise-bar-cli","expected_revision":doc["revision"],"values":doc["values"],"reason":"Saved operator quality bar"});
    change["values"]["quality_delivery_min_percent"] = "90".into();
    let input = root.path().join("settings-change.json");
    std::fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    let (preview, out, _) = call(&[
        "engine-settings",
        "preview",
        "--input",
        input.to_str().unwrap(),
    ]);
    assert_eq!(preview.code, 0);
    change["preview_token"] =
        serde_json::from_slice::<serde_json::Value>(&out).unwrap()["preview_token"].clone();
    std::fs::write(&input, serde_json::to_vec(&change).unwrap()).unwrap();
    assert_eq!(
        call(&[
            "engine-settings",
            "apply",
            "--input",
            input.to_str().unwrap()
        ])
        .0
        .code,
        0
    );
    let settings = root.path().join(".swarm/engine-settings.json");
    let saved = std::fs::read(&settings).unwrap();
    let (refused, out, err) = call(&evaluate);
    assert_eq!(refused.code, 1);
    assert!(refused.error.is_none());
    assert!(err.is_empty());
    let second: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(second["policy"]["generation"], "1");
    assert_eq!(second["evaluation"]["ship_allowed"], false);
    assert_ne!(
        first["evaluation"]["config_digest"],
        second["evaluation"]["config_digest"]
    );
    struct Broken;
    impl io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture output refused",
            ))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let args = [
        "evaluate",
        "--input",
        gate.to_str().unwrap(),
        "--root",
        root.path().to_str().unwrap(),
        "--json",
    ]
    .map(str::to_string);
    let mut err = Vec::new();
    let failed = swarm_cli::run_detailed(&args, "fr", &mut Broken, &mut err);
    assert_eq!(failed.code, 2);
    assert!(
        matches!(failed.error,Some(swarm_cli::CliError::Output(ref e)) if e.kind()==io::ErrorKind::BrokenPipe)
    );
    assert_eq!(call(&evaluate).0.code, 1);
    assert_eq!(std::fs::read(settings).unwrap(), saved);
}

#[test]
fn project_context_cli_uses_saved_limits_without_writing_or_starting_provider() {
    let root = tempfile::TempDir::new().unwrap();
    let store =
        swarm_engine::RootStore::initialize(root.path(), &mut swarm_engine::SystemEnvironment)
            .unwrap();
    store.close().unwrap();
    std::fs::write(root.path().join("guide.md"), b"Project rules").unwrap();
    std::fs::write(root.path().join("swarm.project.json"),br#"{"version":1,"name":"Project","instructions":[{"path":"guide.md","roles":["reviewer"]}]}"#).unwrap();
    let call = || {
        let args = [
            "project-context",
            "reviewer",
            "--root",
            root.path().to_str().unwrap(),
            "--json",
        ]
        .map(str::to_string);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = swarm_cli::run(&args, "fr", &mut out, &mut err);
        (code, out, err)
    };
    let settings = root.path().join(".swarm/engine-settings.json");
    let before = std::fs::read(&settings).unwrap();
    let (code, out, err) = call();
    assert_eq!(code, 0);
    assert!(err.is_empty());
    let observed: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(observed["context"]["role"], "reviewer");
    assert_eq!(std::fs::read(&settings).unwrap(), before);
    let store = swarm_engine::RootStore::open(root.path(), true).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values;
    values.project_context_max_bytes = Some(swarm_engine::SettingCap(Some(
        swarm_engine::SettingCount(1),
    )));
    let mut change = swarm_engine::EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "context-cap".into(),
        expected_revision: doc.revision,
        values,
        reason: "Actual context reader policy".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut swarm_engine::SystemEnvironment)
        .unwrap();
    store.close().unwrap();
    let saved = std::fs::read(&settings).unwrap();
    let (code, out, err) = call();
    assert_eq!(code, 2);
    assert!(out.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&err).unwrap();
    assert_eq!(error["failure"]["code"], "project-context-invalid");
    assert_eq!(std::fs::read(settings).unwrap(), saved);
}
