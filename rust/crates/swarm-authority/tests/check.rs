#![cfg(target_os = "linux")]
//! B2.1 readiness check on private fixtures (never /tmp). Valid witness
//! first, then each refusal keeps a typed code and leaves the private roots
//! empty.
//!
//! Fixture area: the protected-storage loader refuses any group/other-writable
//! ancestor, and the checkout (`ChatGPT/`, `swarm/`) is group-writable, so
//! `<repo>/.swarm-fixtures` cannot host these roots. They live under
//! `$SWARM_PROTECTED_FIXTURE_AREA`, default `~/.local/state/swarm-protected-fixtures`
//! (persistent, owner-only), and are removed after success.
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};
use swarm_enrollment_bootstrap::{encode, Document};
use swarm_enrollment_config::Pins;
use swarm_identity_contract::Principal;

fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}

/// Private root with bootstrap/configuration/witness dirs (0700) and a
/// declaration naming the current euid as service UID.
fn fixture(label: &str) -> (PathBuf, Document, PathBuf) {
    let base = std::env::var_os("SWARM_PROTECTED_FIXTURE_AREA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/state/swarm-protected-fixtures"));
    fs::create_dir_all(&base).unwrap();
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).unwrap();
    // The declaration requires clean absolute paths (no `..`).
    let base = base.canonicalize().unwrap();
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root = base.join(format!("authority-{label}-{}-{nanos}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["bootstrap", "configuration", "witness"] {
        fs::create_dir(root.join(name)).unwrap();
        fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
    }
    let euid = rustix::process::geteuid().as_raw();
    let d = Document {
        schema_version: 1,
        pins: Pins {
            installation_digest: "a".repeat(64),
            public_key: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc".into(),
            principal: Principal {
                id: "operator".into(),
                kind: "human".into(),
                issuer: "https://issuer.example".into(),
                subject: "fixture-human".into(),
            },
        },
        service_uid: euid,
        controller_uid: if euid == 0 { 1 } else { !euid },
        configuration_directory: root.join("configuration").to_str().unwrap().into(),
        witness_directory: root.join("witness").to_str().unwrap().into(),
    };
    let path = root.join("bootstrap/declaration.json");
    (root, d, path)
}

fn write(path: &PathBuf, raw: &[u8]) {
    fs::write(path, raw).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn check_accepts_exact_declaration_and_refuses_pin_uid_and_mode() {
    let (root, d, path) = fixture("check");
    let raw = encode(&d).unwrap();
    write(&path, &raw);
    let ready = swarm_authority::check(path.to_str().unwrap(), &hash(&raw)).unwrap();
    assert_eq!((ready.service_uid, ready.controller_uid), (d.service_uid, d.controller_uid));
    assert_eq!(swarm_authority::check(path.to_str().unwrap(), &"b".repeat(64)).unwrap_err().code(), "pin");
    let mut foreign = d.clone();
    foreign.service_uid = d.controller_uid;
    foreign.controller_uid = d.service_uid;
    let raw2 = encode(&foreign).unwrap();
    write(&path, &raw2);
    assert!(swarm_authority::check(path.to_str().unwrap(), &hash(&raw2)).is_err(), "service_uid must equal euid");
    write(&path, &raw);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(swarm_authority::check(path.to_str().unwrap(), &hash(&raw)).is_err(), "non-private file refuses");
    for dir in ["configuration", "witness"] {
        assert_eq!(fs::read_dir(root.join(dir)).unwrap().count(), 0);
    }
    fs::remove_dir_all(root).unwrap();
}

/// The shipped binary: JSON readiness on stdout, typed failure on stderr,
/// usage errors exit 64.
#[test]
fn binary_check_prints_readiness_or_typed_failure() {
    let (root, d, path) = fixture("bin");
    let raw = encode(&d).unwrap();
    write(&path, &raw);
    let bin = env!("CARGO_BIN_EXE_swarm-authority");
    let ok = Command::new(bin).args(["check", "--declaration", path.to_str().unwrap(), "--pin", &hash(&raw), "--json"]).output().unwrap();
    assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stderr));
    let v: serde_json::Value = serde_json::from_slice(&ok.stdout).unwrap();
    assert_eq!(v["service_uid"], d.service_uid);
    let bad = Command::new(bin).args(["check", "--declaration", path.to_str().unwrap(), "--pin", &"c".repeat(64), "--json"]).output().unwrap();
    assert_eq!(bad.status.code(), Some(2));
    let e: serde_json::Value = serde_json::from_slice(&bad.stderr).unwrap();
    assert_eq!(e["failure"]["code"], "pin");
    let usage = Command::new(bin).args(["check", "--json"]).output().unwrap();
    assert_eq!(usage.status.code(), Some(64));
    fs::remove_dir_all(root).unwrap();
}
