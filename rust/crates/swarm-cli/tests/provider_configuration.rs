#![cfg(target_os = "linux")]
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
};
use swarm_engine::{RootStore, SystemEnvironment};
/// Owned initialized root and explicit config request; unavailable executable is
/// intentional staging data, not a readiness witness or an executed provider.
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let file = root.path().join("request.json");
    fs::write(&file,br#"{"schema_version":1,"providers":{"fixture":{"command":"/fixture/program","args":["literal"]}}}"#).unwrap();
    (root, file)
}
/// Invoke only the actual compiled public CLI against the owned root.
fn call(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .args(["--json", "--root"])
        .arg(root)
        .output()
        .unwrap()
}
/// Require successful meaningful configuration-only JSON, never empty success.
fn value(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
/// Invalid transport/type/schema sources refuse before creating a runtime file.
/// Valid create/readback survives reopening; repeated init cannot replace bytes.
#[test]
fn public_provider_config_create_show_reject_invalid_and_preserve_existing_bytes() {
    let (root, input) = fixture();
    let valid = fs::read(&input).unwrap();
    let target = root.path().join(".swarm/providers.json");
    for invalid in [
        b"PRIVATE malformed".as_slice(),
        b"{\"schema_version\":2}",
        b"{\"schema_version\":1,\"PRIVATE_UNKNOWN\":true}",
    ] {
        // A distinct private control root preserves the same initialized contract.
        let (control, request) = fixture();
        let created = value(call(
            control.path(),
            &[
                "providers",
                "config",
                "init",
                "--input",
                request.to_str().unwrap(),
            ],
        ));
        assert_eq!(created["created"], true);
        fs::write(&input, invalid).unwrap();
        let refused = call(
            root.path(),
            &[
                "providers",
                "config",
                "init",
                "--input",
                input.to_str().unwrap(),
            ],
        );
        assert_eq!(refused.status.code(), Some(2));
        assert!(refused.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&refused.stderr).contains("PRIVATE"));
        assert!(!target.exists());
    }
    fs::write(&input, &valid).unwrap();
    let created = value(call(
        root.path(),
        &[
            "providers",
            "config",
            "init",
            "--input",
            input.to_str().unwrap(),
        ],
    ));
    assert_eq!(created["created"], true);
    assert_eq!(created["provider_execution_started"], false);
    assert_eq!(created["scope"], "cli_configuration_file_only_unverified");
    let original = fs::read(&target).unwrap();
    let shown = value(call(root.path(), &["providers", "config", "show"]));
    assert_eq!(shown["created"], false);
    assert_eq!(shown["digest"], created["digest"]);
    assert_eq!(
        shown["configuration"]["providers"]["fixture"]["command"],
        "/fixture/program"
    );
    fs::write(&input, br#"{"schema_version":1,"providers":{}}"#).unwrap();
    let refused = call(
        root.path(),
        &[
            "providers",
            "config",
            "init",
            "--input",
            input.to_str().unwrap(),
        ],
    );
    assert_eq!(refused.status.code(), Some(2));
    assert_eq!(fs::read(&target).unwrap(), original);
    fs::write(&target, b"PRIVATE corrupt source").unwrap();
    let refused = call(root.path(), &["providers", "config", "show"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&refused.stderr).contains("PRIVATE"));
    assert_eq!(fs::read(&target).unwrap(), b"PRIVATE corrupt source");
    fs::write(&target, &original).unwrap();
    value(call(root.path(), &["providers", "config", "show"]));
}
/// Controlled output fault; choosing write vs flush is a fixture, not policy.
struct Denied {
    flush: bool,
}
impl Write for Denied {
    /// Accept bytes only for the flush-fault witness; otherwise retain a write error.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if self.flush {
            Ok(raw.len())
        } else {
            Err(io::Error::other("PRIVATE sink refusal"))
        }
    }
    /// Inject only the selected flush failure, without mutating runtime data.
    fn flush(&mut self) -> io::Result<()> {
        if self.flush {
            Err(io::Error::other("PRIVATE sink refusal"))
        } else {
            Ok(())
        }
    }
}
/// Real create precedes serialization/output completion. Both write and flush
/// failures must return preserved-state errors and leave the exact file readable;
/// retry is refused, never an overwrite or a second provider execution.
#[test]
fn provider_config_output_and_flush_failure_keep_created_state_and_safe_cause() {
    for flush in [false, true] {
        let (root, input) = fixture();
        let args = vec![
            "providers".into(),
            "config".into(),
            "init".into(),
            "--root".into(),
            root.path().to_str().unwrap().into(),
            "--input".into(),
            input.to_str().unwrap().into(),
            "--json".into(),
        ];
        let mut stderr = vec![];
        let outcome = swarm_cli::run_detailed(&args, "en", &mut Denied { flush }, &mut stderr);
        assert_eq!(outcome.code, 2);
        assert!(matches!(
            outcome.error,
            Some(swarm_cli::CliError::ConfigurationStatePreserved { .. })
        ));
        assert!(!String::from_utf8_lossy(&stderr).contains("PRIVATE"));
        assert!(String::from_utf8_lossy(&stderr).contains("recorded"));
        let shown = value(call(root.path(), &["providers", "config", "show"]));
        assert_eq!(
            shown["configuration"]["providers"]["fixture"]["command"],
            "/fixture/program"
        );
        assert_eq!(
            call(
                root.path(),
                &[
                    "providers",
                    "config",
                    "init",
                    "--input",
                    input.to_str().unwrap()
                ]
            )
            .status
            .code(),
            Some(2)
        );
    }
}

/// Write only a request document, never a runtime provider file.
fn update_input(
    root: &Path,
    digest: &str,
    configuration: Option<serde_json::Value>,
    reference: Option<&str>,
    token: &str,
) -> PathBuf {
    let mut value =
        serde_json::json!({"schema_version":1,"expected_digest":digest,"preview_token":token});
    if let Some(config) = configuration {
        value["configuration"] = config;
    }
    if let Some(reference) = reference {
        value["restore_reference"] = reference.into();
    }
    let file = root.join("update.json");
    fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
    file
}
/// Actual public preview/apply/inspect/snapshot restoration, including malformed
/// source repair. Metadata validity is explicit; no empty/ready fallback or
/// private corruption marker reaches stdout/stderr diagnostics.
#[test]
fn public_provider_config_update_restore_and_inspect_repair_preserve_exact_history() {
    let (root, input) = fixture();
    value(call(
        root.path(),
        &[
            "providers",
            "config",
            "init",
            "--input",
            input.to_str().unwrap(),
        ],
    ));
    let target = root.path().join(".swarm/providers.json");
    let original = fs::read(&target).unwrap();
    let before = value(call(root.path(), &["providers", "config", "inspect"]));
    let next = serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":"/fixture/replacement","args":[]}}});
    let request = update_input(
        root.path(),
        before["digest"].as_str().unwrap(),
        Some(next),
        None,
        "",
    );
    let preview = value(call(
        root.path(),
        &[
            "providers",
            "config",
            "preview",
            "--input",
            request.to_str().unwrap(),
        ],
    ));
    assert_eq!(preview["preview"], true);
    assert_eq!(preview["applied"], false);
    assert_eq!(fs::read(&target).unwrap(), original);
    assert!(!root.path().join(".swarm/providers.lock").exists());
    let rejected = call(
        root.path(),
        &[
            "providers",
            "config",
            "apply",
            "--input",
            request.to_str().unwrap(),
        ],
    );
    assert_eq!(rejected.status.code(), Some(2));
    assert_eq!(fs::read(&target).unwrap(), original);
    assert!(!root.path().join(".swarm/providers.lock").exists());
    let mut wire: serde_json::Value = serde_json::from_slice(&fs::read(&request).unwrap()).unwrap();
    wire["preview_token"] = preview["preview_token"].clone();
    fs::write(&request, serde_json::to_vec(&wire).unwrap()).unwrap();
    let applied = value(call(
        root.path(),
        &[
            "providers",
            "config",
            "apply",
            "--input",
            request.to_str().unwrap(),
        ],
    ));
    let reference = applied["backup_reference"].as_str().unwrap();
    assert_eq!(
        fs::read(root.path().join(".swarm").join(reference)).unwrap(),
        original
    );
    assert_eq!(
        call(
            root.path(),
            &[
                "providers",
                "config",
                "apply",
                "--input",
                request.to_str().unwrap()
            ]
        )
        .status
        .code(),
        Some(2)
    );
    let references = value(call(root.path(), &["providers", "config", "backups"]));
    assert!(references["references"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == reference));
    let inspect = value(call(
        root.path(),
        &["providers", "config", "inspect", reference],
    ));
    assert_eq!(inspect["matches_reference"], true);
    assert_eq!(inspect["format_valid"], true);
    let restore = update_input(
        root.path(),
        applied["after_digest"].as_str().unwrap(),
        None,
        Some(reference),
        "",
    );
    let preview = value(call(
        root.path(),
        &[
            "providers",
            "config",
            "preview",
            "--input",
            restore.to_str().unwrap(),
        ],
    ));
    let mut wire: serde_json::Value = serde_json::from_slice(&fs::read(&restore).unwrap()).unwrap();
    wire["preview_token"] = preview["preview_token"].clone();
    fs::write(&restore, serde_json::to_vec(&wire).unwrap()).unwrap();
    value(call(
        root.path(),
        &[
            "providers",
            "config",
            "apply",
            "--input",
            restore.to_str().unwrap(),
        ],
    ));
    assert_eq!(fs::read(&target).unwrap(), original);
    fs::write(&target, b"PRIVATE corrupt current").unwrap();
    assert_eq!(
        call(root.path(), &["providers", "config", "show"])
            .status
            .code(),
        Some(2)
    );
    let inspected = call(root.path(), &["providers", "config", "inspect"]);
    assert!(!String::from_utf8_lossy(&inspected.stdout).contains("PRIVATE"));
    let inspected = value(inspected);
    assert_eq!(inspected["format_valid"], false);
    let config: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let repair = update_input(
        root.path(),
        inspected["digest"].as_str().unwrap(),
        Some(config),
        None,
        "",
    );
    let preview = value(call(
        root.path(),
        &[
            "providers",
            "config",
            "preview",
            "--input",
            repair.to_str().unwrap(),
        ],
    ));
    let mut wire: serde_json::Value = serde_json::from_slice(&fs::read(&repair).unwrap()).unwrap();
    wire["preview_token"] = preview["preview_token"].clone();
    fs::write(&repair, serde_json::to_vec(&wire).unwrap()).unwrap();
    let fixed = value(call(
        root.path(),
        &[
            "providers",
            "config",
            "apply",
            "--input",
            repair.to_str().unwrap(),
        ],
    ));
    assert_eq!(
        fs::read(
            root.path()
                .join(".swarm")
                .join(fixed["backup_reference"].as_str().unwrap())
        )
        .unwrap(),
        b"PRIVATE corrupt current"
    );
    value(call(root.path(), &["providers", "config", "show"]));
}
/// Both response write and flush failures occur after a real update. The exact
/// prior snapshot remains usable and current bytes match the preview; an old
/// digest cannot repeat/overwrite the change after a lost response.
#[test]
fn provider_config_update_output_failures_preserve_new_and_prior_content() {
    for flush in [false, true] {
        let (root, input) = fixture();
        value(call(
            root.path(),
            &[
                "providers",
                "config",
                "init",
                "--input",
                input.to_str().unwrap(),
            ],
        ));
        let original = fs::read(root.path().join(".swarm/providers.json")).unwrap();
        let meta = value(call(root.path(), &["providers", "config", "inspect"]));
        let next = serde_json::json!({"schema_version":1,"providers":{"fixture":{"command":"/fixture/replacement"}}});
        let request = update_input(
            root.path(),
            meta["digest"].as_str().unwrap(),
            Some(next),
            None,
            "",
        );
        let preview = value(call(
            root.path(),
            &[
                "providers",
                "config",
                "preview",
                "--input",
                request.to_str().unwrap(),
            ],
        ));
        let mut wire: serde_json::Value =
            serde_json::from_slice(&fs::read(&request).unwrap()).unwrap();
        wire["preview_token"] = preview["preview_token"].clone();
        fs::write(&request, serde_json::to_vec(&wire).unwrap()).unwrap();
        let args = vec![
            "providers".into(),
            "config".into(),
            "apply".into(),
            "--root".into(),
            root.path().to_str().unwrap().into(),
            "--input".into(),
            request.to_str().unwrap().into(),
            "--json".into(),
        ];
        let mut stderr = vec![];
        let result = swarm_cli::run_detailed(&args, "en", &mut Denied { flush }, &mut stderr);
        assert_eq!(result.code, 2);
        assert!(matches!(
            result.error,
            Some(swarm_cli::CliError::ConfigurationStatePreserved { .. })
        ));
        assert!(!String::from_utf8_lossy(&stderr).contains("PRIVATE"));
        let meta = value(call(root.path(), &["providers", "config", "inspect"]));
        assert_eq!(meta["digest"], preview["after_digest"]);
        let references = value(call(root.path(), &["providers", "config", "backups"]));
        assert_eq!(references["references"].as_array().unwrap().len(), 1);
        assert_eq!(
            fs::read(
                root.path()
                    .join(".swarm")
                    .join(references["references"][0].as_str().unwrap())
            )
            .unwrap(),
            original
        );
        assert_eq!(
            call(
                root.path(),
                &[
                    "providers",
                    "config",
                    "apply",
                    "--input",
                    request.to_str().unwrap()
                ]
            )
            .status
            .code(),
            Some(2)
        );
    }
}
