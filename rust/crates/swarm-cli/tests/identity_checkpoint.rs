#![cfg(target_os = "linux")]
//! Public read-only anchor API on private synthetic fixtures; no Root/package.
use serde_json::Value;
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
};
struct Case {
    root: tempfile::TempDir,
    input: std::path::PathBuf,
}
impl Case {
    /// Actual current-owned fixture anchor, containing synthetic canonical pins only.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let input = root.path().join("anchor.json");
        let raw=format!("{{\"schema_version\":1,\"protocol\":\"swarm-operator-anchor-v1\",\"request_pin\":\"{}\",\"package_pin\":\"{}\"}}","ab".repeat(32),"cd".repeat(32));
        fs::write(&input, raw).unwrap();
        fs::set_permissions(&input, fs::Permissions::from_mode(0o600)).unwrap();
        Self { root, input }
    }
    /// Exact public argv, without overriding actual storage UID or selecting Root.
    fn args(&self) -> Vec<String> {
        vec![
            "identity".into(),
            "checkpoint-check".into(),
            "ab".repeat(32),
            "--input".into(),
            self.input.to_str().unwrap().into(),
            "--json".into(),
        ]
    }
    /// One public run with separated diagnostic/output; private data stay in fixture.
    fn run(&self, args: &[String], lang: &str) -> (u8, Vec<u8>, String) {
        let mut out = vec![];
        let mut err = vec![];
        let code = swarm_cli::run(args, lang, &mut out, &mut err);
        (code, out, String::from_utf8(err).unwrap())
    }
    /// Assert exact private report and all unproven authority claims stay false.
    fn valid(&self, lang: &str) {
        let (code, out, err) = self.run(&self.args(), lang);
        assert_eq!(code, 0, "{err}");
        let v: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["protocol"], "swarm-operator-anchor-cli-v1");
        assert_eq!(v["request_pin"], "ab".repeat(32));
        assert_eq!(v["package_pin"], "cd".repeat(32));
        assert_eq!(v["read_validation"], "bytes_read");
        assert_eq!(v["read_durability"], "directory_acknowledged");
        for key in [
            "locator_trust_verified",
            "current_authority_verified",
            "human_authentication_verified",
            "separate_os_uid_verified",
            "operator_package_read",
            "material_generated",
            "service_installed",
        ] {
            assert_eq!(v[key], false, "{key}");
        }
        assert!(!self.root.path().join(".swarm").exists());
        assert!(!self
            .root
            .path()
            .join("initial-provisioning-v1.bin")
            .exists());
    }
}
/// Read both languages without generating a package, Root or material.
#[test]
fn checkpoint_read_exact_report_fr_en_without_package() {
    let c = Case::new();
    c.valid("fr");
    c.valid("en");
}
/// Wrong request binding or malformed pin refuses with no private stdout; restore.
#[test]
fn checkpoint_pin_binding_refuses_and_original_recovers() {
    let c = Case::new();
    c.valid("en");
    for pin in ["ef".repeat(32), "bad".into()] {
        let mut a = c.args();
        a[2] = pin;
        let (code, out, err) = c.run(&a, "en");
        assert_ne!(code, 0);
        assert!(out.is_empty());
        assert!(!err.contains(c.input.to_str().unwrap()));
    }
    c.valid("fr");
}
/// Unsafe file metadata is not waived by trusting a selected locator.
#[test]
fn checkpoint_private_metadata_refuses_then_restores() {
    let c = Case::new();
    c.valid("en");
    fs::set_permissions(&c.input, fs::Permissions::from_mode(0o644)).unwrap();
    let (code, out, _) = c.run(&c.args(), "en");
    assert_ne!(code, 0);
    assert!(out.is_empty());
    fs::set_permissions(&c.input, fs::Permissions::from_mode(0o600)).unwrap();
    c.valid("en");
}
/// Noncanonical data never become a trusted record or a discovered package pin.
#[test]
fn checkpoint_noncanonical_data_refuses_without_mutation() {
    let c = Case::new();
    c.valid("en");
    let original = fs::read(&c.input).unwrap();
    let mut bad = original.clone();
    bad.push(b' ');
    fs::write(&c.input, &bad).unwrap();
    let (code, out, _) = c.run(&c.args(), "fr");
    assert_ne!(code, 0);
    assert!(out.is_empty());
    assert_eq!(fs::read(&c.input).unwrap(), bad);
    fs::write(&c.input, original).unwrap();
    c.valid("fr");
}
/// Default aliases/extra flags refuse before reading/creating an anchor lock.
#[test]
fn checkpoint_closed_flags_refuse_before_io() {
    for extra in [
        vec!["--root", "."],
        vec!["--phase", "delivery"],
        vec!["--json"],
        vec!["--server", "false"],
    ] {
        let c = Case::new();
        let mut a = c.args();
        a.extend(extra.iter().map(|s| s.to_string()));
        let (code, out, _) = c.run(&a, "en");
        assert_ne!(code, 0);
        assert!(out.is_empty());
        assert!(!c.root.path().join("private-documents.lock").exists());
        c.valid("en");
    }
}
struct Broken {
    flush: bool,
    bytes: Vec<u8>,
}
impl Write for Broken {
    /// Controlled writer refusal preserves completed read facts, no private log data.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if self.flush {
            self.bytes.extend_from_slice(raw);
            Ok(raw.len())
        } else {
            Err(io::Error::other("PRIVATE_ANCHOR_WRITE"))
        }
    }
    /// Distinguish all-bytes-written from successfully flushed output.
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("PRIVATE_ANCHOR_FLUSH"))
    }
}
/// Late output failure remains failure; known decoded/ack facts survive safely.
#[test]
fn checkpoint_output_failure_keeps_known_read_and_recovers() {
    for flush in [false, true] {
        let c = Case::new();
        c.valid("en");
        let mut out = Broken {
            flush,
            bytes: vec![],
        };
        let mut err = vec![];
        assert_ne!(swarm_cli::run(&c.args(), "en", &mut out, &mut err), 0);
        let text = String::from_utf8(err).unwrap();
        assert!(text.contains("anchor-decoded:true"));
        assert!(text.contains("anchor-read-durability:directory_acknowledged"));
        assert!(!text.contains("PRIVATE_ANCHOR") && !text.contains(c.input.to_str().unwrap()));
        c.valid("en");
    }
}
