#![cfg(target_os = "linux")]
//! Operator package/handoff public routes on private synthetic /tmp roots only.
//! This fixture intentionally stores business/private material in its operator
//! vault; exported service data must exclude business seed and proof. No human,
//! real service account, authority or service installation is established.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};
use swarm_enrollment_authority::provisioning::{self as p, repository as repo, service_artifact};
use swarm_enrollment_config::store::{documents::Documents, Context};
use zeroize::Zeroizing;
struct Case {
    _root: tempfile::TempDir,
    operator: PathBuf,
    output: PathBuf,
    target: PathBuf,
    pin: String,
    service_pin: String,
    expected: Zeroizing<Vec<u8>>,
}
/// Set permissions on this fixture only; setup errors panic without data dumps.
fn mode(path: &std::path::Path, bits: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(bits)).unwrap()
}
impl Case {
    /// Canonical private package under actual controller UID. Service UID is only
    /// distinct synthetic data, not a mapped account. Pin is retained before write.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        mode(root.path(), 0o700);
        let operator = root.path().join("operator");
        let handoff = root.path().join("handoff");
        for d in [&operator, &handoff] {
            fs::create_dir(d).unwrap();
            mode(d, 0o700)
        }
        let target = root.path().join("future-service");
        let mut request: p::Request = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/paea_provisioning_request.json"
        ))
        .unwrap();
        let uid = rustix::process::geteuid().as_raw();
        request.controller_uid = uid;
        request.service_uid = if uid == 0 { 1 } else { !uid };
        request.root_directory = target.to_str().unwrap().into();
        let prepared = p::prepare(&request).unwrap();
        let artifact = service_artifact::from_prepared(&prepared).unwrap();
        let service_pin = artifact.pin().unwrap();
        let expected = artifact.encode_private().unwrap();
        drop(artifact);
        let seal = repo::seal(prepared).unwrap();
        let pin = seal.pin().as_str().to_owned();
        let store = repo::PackageStore::new(Documents::open(&operator, uid).unwrap());
        assert_eq!(
            store.create(&Context::new(), &seal).unwrap(),
            swarm_enrollment_config::store::documents::Publication::Durable
        );
        store.close().unwrap();
        Self {
            _root: root,
            operator,
            output: handoff.join("service-v1.bin"),
            target,
            pin,
            service_pin,
            expected,
        }
    }
    /// Actual public grammar, no Root/actor/key/head/clock override.
    fn args(&self, action: &str) -> Vec<String> {
        let mut a = vec!["identity".into(), action.into(), self.pin.clone()];
        if action != "operator-check" {
            a.push(self.output.to_str().unwrap().into());
        }
        a.extend(["--input", self.operator.to_str().unwrap(), "--json"].map(str::to_owned));
        a
    }
    /// Public observation/export must report meaningful facts and no service/human.
    fn run(&self, action: &str, lang: &str) -> serde_json::Value {
        let mut a = self.args(action);
        a.extend(["--lang".into(), lang.into()]);
        let mut out = vec![];
        let mut error = vec![];
        let code = swarm_cli::run(&a, lang, &mut out, &mut error);
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&error));
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["protocol"], "swarm-operator-package-cli-v1");
        assert_eq!(v["operator_package_pin"], self.pin);
        assert_eq!(v["service_artifact_pin"], self.service_pin);
        assert_eq!(v["controller_uid_matches_operator"], true);
        for field in [
            "human_authentication_verified",
            "current_authority_verified",
            "service_installed",
            "service_started",
            "separate_os_uid_verified",
            "confinement_verified",
        ] {
            assert_eq!(v[field], false, "{field}");
        }
        assert!(!self.target.exists());
        v
    }
    /// Package singleton is fixture data only, never use a live private package.
    fn package(&self) -> PathBuf {
        self.operator.join("initial-provisioning-v1.bin")
    }
}
/// Exact fixture bytes digest only, not an independently authenticated pin.
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Public data/UID observation in both languages does not export or start anything.
#[test]
fn operator_check_pinned_private_package_fr_en_without_export_or_journal() {
    let c = Case::new();
    let before = hash(&fs::read(c.package()).unwrap());
    for lang in ["fr", "en"] {
        let v = c.run("operator-check", lang);
        assert_eq!(v["publication"], "not_attempted");
        assert!(!c.output.exists());
        assert_eq!(hash(&fs::read(c.package()).unwrap()), before);
    }
}
/// Actual immutable export, repeated create refuses unchanged, explicit reconcile
/// observes exact expected service bytes without resetting or duplicating state.
#[test]
fn operator_service_export_exact_no_replace_and_explicit_reconcile() {
    let c = Case::new();
    c.run("operator-check", "en");
    let v = c.run("service-export", "en");
    assert_eq!(v["publication"], "durable");
    assert!(
        fs::read(&c.output).unwrap().as_slice() == c.expected.as_slice(),
        "service-only-bytes-mismatch"
    );
    let package = Zeroizing::new(fs::read(c.package()).unwrap());
    let header = b"swarm-initial-provisioning-v1\n".len();
    for secret in [
        &package[header..header + 32],
        &package[header + 64..header + 96],
    ] {
        assert!(
            !c.expected
                .windows(secret.len())
                .any(|window| window == secret),
            "operator-secret-in-service-artifact"
        );
    }
    let before = hash(&fs::read(&c.output).unwrap());
    let mut out = vec![];
    let mut err = vec![];
    assert_ne!(
        swarm_cli::run(&c.args("service-export"), "en", &mut out, &mut err),
        0
    );
    assert_eq!(hash(&fs::read(&c.output).unwrap()), before);
    let v = c.run("service-export-reconcile", "en");
    assert_eq!(v["output_durability"], "directory_acknowledged");
    assert_eq!(hash(&fs::read(&c.output).unwrap()), before);
}
/// Valid witness before pin and private file-mode refusal; restore and recover.
#[test]
fn operator_package_pin_and_permissions_refuse_without_output_then_recover() {
    let c = Case::new();
    c.run("operator-check", "en");
    let raw = hash(&fs::read(c.package()).unwrap());
    for variant in ["pin", "file", "parent"] {
        let mut a = c.args("operator-check");
        match variant {
            "pin" => a[2] = "00".repeat(32),
            "file" => mode(&c.package(), 0o644),
            _ => mode(&c.operator, 0o755),
        }
        let mut out = vec![];
        let mut err = vec![];
        assert_ne!(swarm_cli::run(&a, "en", &mut out, &mut err), 0);
        assert!(out.is_empty());
        assert!(!String::from_utf8(err)
            .unwrap()
            .contains(c.operator.to_str().unwrap()));
        mode(&c.package(), 0o600);
        mode(&c.operator, 0o700);
        assert_eq!(hash(&fs::read(c.package()).unwrap()), raw);
        c.run("operator-check", "en");
        assert!(!c.output.exists());
    }
}
/// A preexisting unrelated private output is preserved and never adopted.
#[test]
fn operator_export_preserves_existing_destination_and_refuses_reconciliation() {
    let c = Case::new();
    c.run("operator-check", "en");
    fs::write(&c.output, b"EXISTING").unwrap();
    mode(&c.output, 0o600);
    for action in ["service-export", "service-export-reconcile"] {
        let mut out = vec![];
        let mut err = vec![];
        assert_ne!(swarm_cli::run(&c.args(action), "en", &mut out, &mut err), 0);
        assert_eq!(fs::read(&c.output).unwrap(), b"EXISTING");
        assert!(!c.target.exists());
    }
}
struct Broken {
    flush: bool,
    bytes: Vec<u8>,
}
impl Write for Broken {
    /// Output-only controlled refusal after package publication; never log inputs.
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.flush {
            self.bytes.extend_from_slice(b);
            Ok(b.len())
        } else {
            Err(io::Error::other("PRIVATE_OPERATOR_WRITE"))
        }
    }
    /// Preserve actual all-bytes-written then flush-refused distinction.
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("PRIVATE_OPERATOR_FLUSH"))
    }
}
/// Output failure cannot erase known durable private artifact; reconcile recovers.
#[test]
fn operator_export_output_failure_keeps_publication_and_reconcile_recovers() {
    for flush in [false, true] {
        let c = Case::new();
        c.run("operator-check", "en");
        let mut out = Broken {
            flush,
            bytes: vec![],
        };
        let mut err = vec![];
        assert_ne!(
            swarm_cli::run(&c.args("service-export"), "en", &mut out, &mut err),
            0
        );
        let text = String::from_utf8(err).unwrap();
        assert!(
            text.contains("operator-output-publication:durable"),
            "{text}"
        );
        assert!(!text.contains("PRIVATE_") && !text.contains(c.operator.to_str().unwrap()));
        assert!(
            fs::read(&c.output).unwrap().as_slice() == c.expected.as_slice(),
            "service-bytes-mismatch"
        );
        c.run("service-export-reconcile", "en");
    }
}

/// Closed flags/duplicates refuse before source IO even with default values;
/// only this fixture's closed lock is removed to make the observation explicit.
#[test]
fn operator_closed_flags_refuse_before_source_io_then_recover() {
    let c = Case::new();
    for action in [
        "operator-check",
        "service-export",
        "service-export-reconcile",
    ] {
        for flags in [
            vec!["--root", "."],
            vec!["--phase", "delivery"],
            vec!["--server", "false"],
            vec!["--output", ""],
            vec!["--json"],
            vec!["--input", "duplicate"],
        ] {
            c.run("operator-check", "en");
            let lock = c.operator.join("private-documents.lock");
            fs::remove_file(&lock).unwrap();
            let before = hash(&fs::read(c.package()).unwrap());
            let mut args = c.args(action);
            args.extend(flags.into_iter().map(str::to_owned));
            let mut out = vec![];
            let mut err = vec![];
            assert_ne!(swarm_cli::run(&args, "en", &mut out, &mut err), 0);
            assert!(out.is_empty() && !lock.exists() && !c.output.exists() && !c.target.exists());
            assert_eq!(hash(&fs::read(c.package()).unwrap()), before);
            c.run("operator-check", "en");
        }
    }
}
