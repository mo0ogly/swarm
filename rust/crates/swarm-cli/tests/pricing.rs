#![cfg(unix)]
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output},
};
use swarm_engine::{RootStore, SystemEnvironment};
/// Fresh owned root, no Git/provider/server required by this catalogue contract.
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    root
}
/// Actual public binary, confined by argument to the private fixture only.
fn call(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swarm-rust"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}
/// Parse successful JSON while keeping unexpected exit/diagnostic output visible.
fn value(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
/// Synthetic complete price intent; the native engine must ignore forged metadata.
fn input(root: &Path, digest: &str, price: f64, cache: Option<f64>) -> std::path::PathBuf {
    let path = root.join("price.json");
    let raw = serde_json::json!({"schema_version":1,"expected_digest":digest,"rate":{"provider":"fixture","model":"example","currency":"USD","input_per_million":price,"output_per_million":5,"cache_read_per_million":cache,"source":"Illustrative fixture, no real tariff","reference_date":"2026-10-19","actor":"FORGED","updated":"FORGED","version":"FORGED"}});
    fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
    path
}
/// Save/list/estimate use immutable versions and explicit category volumes, with
/// no default tariff or silent zero. Stale writes refuse without changing bytes.
#[test]
fn pricing_cli_versions_unknown_cache_and_stale_write() {
    let root = fixture();
    let database_before = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let c = value(call(root.path(), &["pricing", "list", "--json"]));
    assert_eq!(c["rates"], serde_json::json!([]));
    assert!(!root.path().join(".swarm/model-rates.json").exists());
    let path = input(root.path(), c["digest"].as_str().unwrap(), 1e10, None);
    let first = value(call(
        root.path(),
        &["pricing", "save", "--input", path.to_str().unwrap()],
    ));
    assert_ne!(first["actor"], "FORGED");
    assert_ne!(first["version"], "FORGED");
    assert_ne!(first["updated"], "FORGED");
    let before = fs::read(root.path().join(".swarm/model-rates.json")).unwrap();
    let stale = call(
        root.path(),
        &["pricing", "save", "--input", path.to_str().unwrap()],
    );
    assert_eq!(stale.status.code(), Some(2));
    assert!(stale.stdout.is_empty());
    assert_eq!(
        fs::read(root.path().join(".swarm/model-rates.json")).unwrap(),
        before
    );
    let quote_file = root.path().join("quote.json");
    let quote = serde_json::json!({"rate_version":first["version"],"non_cached_input_tokens":1000000000001i64,"output_tokens":0,"cache_read_tokens":1});
    fs::write(&quote_file, serde_json::to_vec(&quote).unwrap()).unwrap();
    let unknown = value(call(
        root.path(),
        &[
            "pricing",
            "estimate",
            "--input",
            quote_file.to_str().unwrap(),
        ],
    ));
    assert_eq!(unknown["status"], "unknown");
    assert_eq!(unknown["estimated_amount"], serde_json::Value::Null);
    assert_eq!(unknown["missing_rates"], serde_json::json!(["cache_read"]));
    let c = value(call(root.path(), &["pricing", "list"]));
    let next = input(root.path(), c["digest"].as_str().unwrap(), 2.0, Some(0.0));
    let second = value(call(
        root.path(),
        &["pricing", "save", "--input", next.to_str().unwrap()],
    ));
    let mut q = quote.clone();
    q["rate_version"] = second["version"].clone();
    q["non_cached_input_tokens"] = 1000000.into();
    q["output_tokens"] = 1000000.into();
    fs::write(&quote_file, serde_json::to_vec(&q).unwrap()).unwrap();
    let known = value(call(
        root.path(),
        &[
            "pricing",
            "estimate",
            "--input",
            quote_file.to_str().unwrap(),
        ],
    ));
    assert_eq!(known["estimated_amount"], 7);
    assert_eq!(known["rate"]["cache_read_per_million"], 0);
    fs::write(&quote_file, serde_json::to_vec(&quote).unwrap()).unwrap();
    let prior = value(call(
        root.path(),
        &[
            "pricing",
            "estimate",
            "--input",
            quote_file.to_str().unwrap(),
        ],
    ));
    assert_eq!(prior["status"], "unknown");
    assert_eq!(prior["rate"]["input_per_million"].as_f64(), Some(1e10));
    assert_eq!(
        fs::read(root.path().join(".swarm/state.db")).unwrap(),
        database_before
    );
    assert_eq!(
        value(call(root.path(), &["pricing", "list"]))["rates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
/// A valid witness precedes malformed intent/corrupted retained data. Refusal
/// leaves exact catalogue bytes intact, then restoring private data recovers.
#[test]
fn pricing_cli_invalid_input_and_history_refuse_without_mutation_then_recover() {
    let root = fixture();
    let c = value(call(root.path(), &["pricing", "list"]));
    let file = input(root.path(), c["digest"].as_str().unwrap(), 2.0, None);
    value(call(
        root.path(),
        &["pricing", "save", "--input", file.to_str().unwrap()],
    ));
    let catalogue = root.path().join(".swarm/model-rates.json");
    let original = fs::read(&catalogue).unwrap();
    fs::write(&file, b"{\"PRIVATE_UNKNOWN\":1}").unwrap();
    let refused = call(
        root.path(),
        &["pricing", "save", "--input", file.to_str().unwrap()],
    );
    assert_eq!(refused.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&refused.stderr).contains("PRIVATE"));
    assert_eq!(fs::read(&catalogue).unwrap(), original);
    fs::write(&catalogue, b"PRIVATE corrupt catalogue").unwrap();
    let refused = call(root.path(), &["pricing", "list"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(refused.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&refused.stderr).contains("PRIVATE"));
    assert_eq!(fs::read(&catalogue).unwrap(), b"PRIVATE corrupt catalogue");
    fs::write(&catalogue, &original).unwrap();
    assert_eq!(
        value(call(root.path(), &["pricing", "list"]))["rates"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
struct Denied {
    flush: bool,
}
impl Write for Denied {
    /// Test-only write or flush fault after actual catalogue publication.
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.flush {
            Ok(b.len())
        } else {
            Err(io::Error::other("PRIVATE writer failure"))
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.flush {
            Err(io::Error::other("PRIVATE flush failure"))
        } else {
            Ok(())
        }
    }
}
/// Checked output/flush errors stay nonzero and explicitly retain the committed
/// catalogue; retrying the old digest is refused rather than appending twice.
#[test]
fn pricing_cli_output_failures_report_preserved_configuration() {
    for flush in [false, true] {
        let root = fixture();
        let c = value(call(root.path(), &["pricing", "list"]));
        let file = input(root.path(), c["digest"].as_str().unwrap(), 2.0, None);
        let args = vec![
            "pricing".into(),
            "save".into(),
            "--root".into(),
            root.path().to_str().unwrap().into(),
            "--input".into(),
            file.to_str().unwrap().into(),
        ];
        let mut stderr = vec![];
        let result = swarm_cli::run_detailed(&args, "en", &mut Denied { flush }, &mut stderr);
        assert_eq!(result.code, 2);
        assert!(matches!(
            result.error,
            Some(swarm_cli::CliError::ConfigurationStatePreserved { .. })
        ));
        assert!(!String::from_utf8_lossy(&stderr).contains("PRIVATE"));
        assert!(String::from_utf8_lossy(&stderr).contains("recorded"));
        let saved = value(call(root.path(), &["pricing", "list"]));
        assert_eq!(saved["rates"].as_array().unwrap().len(), 1);
        assert_eq!(
            call(
                root.path(),
                &["pricing", "save", "--input", file.to_str().unwrap()]
            )
            .status
            .code(),
            Some(2)
        );
    }
}
