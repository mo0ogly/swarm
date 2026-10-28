#![cfg(target_os = "linux")]
//! Public dormant-material setup only; private /tmp fixtures, no live Root/service.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use swarm_enrollment_authority::provisioning as p;
/// Exact private fixture digest, never learn expected pin from package bytes.
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Atomic private fixture replacement with synced bytes; no live paths used.
fn write(path: &Path, raw: &[u8]) {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap()).unwrap();
    file.as_file_mut().write_all(raw).unwrap();
    file.as_file().sync_all().unwrap();
    file.persist(path).unwrap();
    fs::File::open(path.parent().unwrap())
        .unwrap()
        .sync_all()
        .unwrap();
}
struct Case {
    root: tempfile::TempDir,
    input: PathBuf,
    op: PathBuf,
    anchor: PathBuf,
    request: p::Request,
    raw: Vec<u8>,
    pin: String,
}
impl Case {
    /// Current-owned private request and two private targets, no default UID/grant.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let op = root.path().join("operator");
        let a = root.path().join("anchor");
        for dir in [&op, &a] {
            fs::create_dir(dir).unwrap();
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut request: p::Request = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/paea_provisioning_request.json"
        ))
        .unwrap();
        let uid = rustix::process::geteuid().as_raw();
        request.controller_uid = uid;
        request.service_uid = if uid == 0 { 1 } else { !uid };
        request.root_directory = root.path().join("absent-service").to_str().unwrap().into();
        let raw = p::encode_request(&request).unwrap();
        let pin = hash(&raw);
        let input = root.path().join("request.json");
        write(&input, &raw);
        Self {
            root,
            input,
            op,
            anchor: a.join("pin.json"),
            request,
            raw,
            pin,
        }
    }
    /// Closed explicit paths/independent pin; no Root, UID, key or policy override.
    fn args(&self, action: &str) -> Vec<String> {
        vec![
            "identity".into(),
            action.into(),
            self.pin.clone(),
            self.op.to_str().unwrap().into(),
            self.anchor.to_str().unwrap().into(),
            "--input".into(),
            self.input.to_str().unwrap().into(),
            "--json".into(),
        ]
    }
    /// One public call with separate output and safe diagnostic capture.
    fn run(&self, args: &[String], lang: &str) -> (u8, Vec<u8>, String) {
        let mut out = vec![];
        let mut err = vec![];
        let code = swarm_cli::run(args, lang, &mut out, &mut err);
        (code, out, String::from_utf8(err).unwrap())
    }
    /// Expected singleton operator bytes, never a service handoff directory.
    fn package(&self) -> PathBuf {
        self.op.join("initial-provisioning-v1.bin")
    }
    /// Assert actual package hash/state, closed metadata and no service effects.
    fn checked(&self, action: &str, lang: &str) -> Value {
        let (code, out, err) = self.run(&self.args(action), lang);
        assert_eq!(code, 0, "{err}");
        assert!(err.is_empty());
        let v: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["protocol"], "swarm-initial-creation-cli-v1");
        assert_eq!(v["core_result"], "completed");
        assert_eq!(v["operation"], action);
        assert_eq!(v["request_pin"], self.pin);
        assert_eq!(v["package_pin"], hash(&fs::read(self.package()).unwrap()));
        assert!(v["package_request_matched"].as_bool().unwrap());
        for key in [
            "locator_trust_verified",
            "current_authority_verified",
            "human_authentication_verified",
            "separate_os_uid_verified",
            "confinement_verified",
            "service_installed",
            "service_started",
        ] {
            assert_eq!(v[key], false, "{key}");
        }
        assert!(!self.root.path().join("absent-service").exists());
        v
    }
}
/// Both languages create once; all subsequent reconciliation is read-only.
#[test]
fn initial_cli_create_reconcile_exact_fr_en() {
    for lang in ["fr", "en"] {
        let c = Case::new();
        let v = c.checked("initial-create", lang);
        assert_eq!(v["intent_publication"], "durable");
        assert_eq!(v["anchor_publication"], "durable");
        assert_eq!(v["package_publication"], "durable");
        assert_eq!(v["material_generation_attempted"], true);
        let v = c.checked("initial-reconcile", lang);
        assert_eq!(v["package_publication"], "not_attempted");
        assert_eq!(v["material_generation_attempted"], false);
        assert_eq!(v["package_read"], "decoded");
    }
}
/// Duplicate create never overwrites/regenerates; original pinned read recovers.
#[test]
fn initial_cli_existing_slot_refuses_without_overwrite() {
    let c = Case::new();
    c.checked("initial-create", "en");
    let before = hash(&fs::read(c.package()).unwrap());
    let (code, out, err) = c.run(&c.args("initial-create"), "en");
    assert_ne!(code, 0);
    assert!(out.is_empty());
    assert!(!err.contains(c.root.path().to_str().unwrap()));
    assert_eq!(hash(&fs::read(c.package()).unwrap()), before);
    c.checked("initial-reconcile", "en");
}
/// Valid peer fixture first; malformed byte pin/canonical data/controller are
/// refused before creating any target, then original canonical bytes recover.
#[test]
fn initial_cli_request_pin_canonical_and_uid_refuse_before_targets() {
    for variant in ["pin", "canonical", "uid"] {
        let control = Case::new();
        control.checked("initial-create", "en");
        let mut c = Case::new();
        let mut a = c.args("initial-create");
        match variant {
            "pin" => a[2] = "00".repeat(32),
            "canonical" => {
                let raw = serde_json::to_vec_pretty(&c.request).unwrap();
                write(&c.input, &raw);
                a[2] = hash(&raw);
            }
            _ => {
                std::mem::swap(&mut c.request.controller_uid, &mut c.request.service_uid);
                let raw = p::encode_request(&c.request).unwrap();
                write(&c.input, &raw);
                a[2] = hash(&raw);
            }
        }
        let (code, out, err) = c.run(&a, "fr");
        assert_ne!(code, 0);
        assert!(out.is_empty());
        assert!(!c.package().exists() && !c.anchor.exists());
        assert!(!err.contains(c.root.path().to_str().unwrap()));
        write(&c.input, &c.raw);
        c.checked("initial-create", "fr");
    }
}

/// Unsafe source/target metadata or physical alias refuses without material;
/// restore original preconditions and a fresh explicit creation can succeed.
#[test]
fn initial_cli_private_metadata_and_alias_refuse_then_recover() {
    for variant in ["source", "anchor", "alias"] {
        let witness = Case::new();
        witness.checked("initial-create", "en");
        let c = Case::new();
        let mut a = c.args("initial-create");
        match variant {
            "source" => fs::set_permissions(&c.input, fs::Permissions::from_mode(0o644)).unwrap(),
            "anchor" => fs::set_permissions(
                c.anchor.parent().unwrap(),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap(),
            _ => a[4] = c.op.join("pin.json").to_str().unwrap().into(),
        };
        let (code, out, err) = c.run(&a, "en");
        assert_ne!(code, 0);
        assert!(out.is_empty());
        assert!(!c.package().exists() && !c.anchor.exists());
        assert!(!err.contains(c.root.path().to_str().unwrap()));
        if variant == "source" {
            fs::set_permissions(&c.input, fs::Permissions::from_mode(0o600)).unwrap();
        } else if variant == "anchor" {
            fs::set_permissions(
                c.anchor.parent().unwrap(),
                fs::Permissions::from_mode(0o700),
            )
            .unwrap();
        }
        c.checked("initial-create", "en");
    }
}
/// Default-valued forbidden flags/duplicates refuse before even source lock IO.
#[test]
fn initial_cli_closed_flags_refuse_before_source_or_target_io() {
    for action in ["initial-create", "initial-reconcile"] {
        for flags in [
            vec!["--root", "."],
            vec!["--phase", "delivery"],
            vec!["--output", ""],
            vec!["--json"],
            vec!["--server", "false"],
        ] {
            let c = Case::new();
            let mut a = c.args(action);
            a.extend(flags.iter().map(|s| s.to_string()));
            let (code, out, _) = c.run(&a, "en");
            assert_ne!(code, 0);
            assert!(out.is_empty());
            assert!(!c.root.path().join("private-documents.lock").exists());
            assert!(!c.package().exists() && !c.anchor.exists());
            c.checked("initial-create", "en");
        }
    }
}
struct Broken {
    flush: bool,
    bytes: Vec<u8>,
}
impl Write for Broken {
    /// Controlled output refusal after all private publication owners are closed.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if self.flush {
            self.bytes.extend_from_slice(raw);
            Ok(raw.len())
        } else {
            Err(io::Error::other("PRIVATE_INITIAL_WRITE"))
        }
    }
    /// Separate complete write from acknowledged flush; no receiver proof implied.
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("PRIVATE_INITIAL_FLUSH"))
    }
}
/// Late output cannot erase durable effect or prompt automatic regeneration.
#[test]
fn initial_cli_output_failure_keeps_package_and_reconcile_recovers() {
    for flush in [false, true] {
        let witness = Case::new();
        witness.checked("initial-create", "en");
        let c = Case::new();
        let mut output = Broken {
            flush,
            bytes: vec![],
        };
        let mut err = vec![];
        assert_ne!(
            swarm_cli::run(&c.args("initial-create"), "en", &mut output, &mut err),
            0
        );
        let err = String::from_utf8(err).unwrap();
        assert!(err.contains("initial-package-publication:durable"));
        assert!(!err.contains("PRIVATE_INITIAL") && !err.contains(c.root.path().to_str().unwrap()));
        let before = hash(&fs::read(c.package()).unwrap());
        let v = c.checked("initial-reconcile", "en");
        assert_eq!(v["material_generation_attempted"], false);
        assert_eq!(hash(&fs::read(c.package()).unwrap()), before);
        let (code, _, _) = c.run(&c.args("initial-create"), "en");
        assert_ne!(code, 0);
        assert_eq!(hash(&fs::read(c.package()).unwrap()), before);
    }
}
