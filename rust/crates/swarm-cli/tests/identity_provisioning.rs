#![cfg(target_os = "linux")]
//! Public provisioning-request preflight on isolated private roots. No entropy,
//! package, grant, human, service account or actual UID separation is created.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};
use swarm_enrollment_authority::provisioning as p;

struct Case {
    _root: tempfile::TempDir,
    path: PathBuf,
    target: PathBuf,
    raw: Vec<u8>,
    pin: String,
}
impl Case {
    /// Canonical explicit request data under a private /tmp parent; cheap KDF and
    /// grants are fixture inputs, not production defaults or current authority.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = root.path().join("request.json");
        let target = root.path().join("future-service");
        let mut request: p::Request = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/paea_provisioning_request.json"
        ))
        .unwrap();
        request.root_directory = target.to_str().unwrap().into();
        let uid = rustix::process::geteuid().as_raw();
        request.service_uid = uid;
        request.controller_uid = if uid == 0 { 1 } else { !uid };
        let raw = p::encode_request(&request).unwrap();
        fs::write(&path, &raw).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let pin = hash(&raw);
        Self {
            _root: root,
            path,
            target,
            raw,
            pin,
        }
    }
    /// Actual public CLI grammar; no source/head/clock/identity override.
    fn args(&self, pin: &str, lang: &str) -> Vec<String> {
        [
            "identity",
            "provisioning-check",
            pin,
            "--input",
            self.path.to_str().unwrap(),
            "--json",
            "--lang",
            lang,
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }
    /// Run the public Rust route and require meaningful closed observations.
    fn valid(&self, lang: &str) {
        let mut out = vec![];
        let mut error = vec![];
        let code = swarm_cli::run(&self.args(&self.pin, lang), lang, &mut out, &mut error);
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&error));
        let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(value["protocol"], "swarm-provisioning-request-cli-v1");
        assert_eq!(value["request_digest"], self.pin);
        assert_eq!(value["request_validated"], true);
        assert_eq!(value["input_durability"], "directory_acknowledged");
        for f in [
            "provisioning_started",
            "human_authentication_verified",
            "current_authority_verified",
            "service_started",
            "confinement_verified",
        ] {
            assert_eq!(value[f], false, "{f}");
        }
        assert_eq!(fs::read(&self.path).unwrap(), self.raw);
        assert!(!self.target.exists());
    }
    /// Invalid input never returns success/JSON/secret diagnostics or target data.
    fn refused(&self, pin: &str) {
        let mut out = vec![];
        let mut error = vec![];
        assert_ne!(
            swarm_cli::run(&self.args(pin, "en"), "en", &mut out, &mut error),
            0
        );
        assert!(out.is_empty());
        let text = String::from_utf8(error).unwrap();
        assert!(!text.contains(self.path.to_str().unwrap()) && !text.contains(&self.pin));
        assert!(!self.target.exists());
    }
}
/// Exact bytes digest for this synthetic fixture, not independently trusted data.
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Both public languages report validation without material or installation.
#[test]
fn provisioning_check_private_canonical_request_fr_en_without_material_or_target() {
    let c = Case::new();
    c.valid("fr");
    c.valid("en");
}
/// A valid witness precedes each corrupted pin/document/role, then exact recovery.
#[test]
fn provisioning_check_pin_canonical_reserved_roles_and_recovery() {
    let c = Case::new();
    for variant in ["pin", "frame", "service_uid", "controller_uid", "unknown"] {
        c.valid("en");
        let mut bad = c.raw.clone();
        match variant {
            "frame" => bad.push(b'\n'),
            "service_uid" | "controller_uid" => {
                let mut r: p::Request = serde_json::from_slice(&bad).unwrap();
                assert_eq!(serde_json::to_vec(&r).unwrap(), bad);
                if variant == "service_uid" {
                    r.service_uid = u32::MAX;
                } else {
                    r.controller_uid = u32::MAX;
                }
                bad = serde_json::to_vec(&r).unwrap();
            }
            "unknown" => {
                let mut v: serde_json::Value = serde_json::from_slice(&bad).unwrap();
                v["unexpected"] = true.into();
                bad = serde_json::to_vec(&v).unwrap();
            }
            _ => {}
        }
        fs::write(&c.path, &bad).unwrap();
        c.refused(&if variant == "pin" {
            "00".repeat(32)
        } else {
            hash(&bad)
        });
        fs::write(&c.path, &c.raw).unwrap();
        c.valid("en");
    }
}
/// Protected file/parent checks must refuse before exposing data, then recover.
#[test]
fn provisioning_check_unsafe_file_links_parent_and_recovery() {
    for variant in ["mode", "parent", "hardlink", "symlink"] {
        let c = Case::new();
        c.valid("en");
        let held = c.path.with_extension("held");
        match variant {
            "mode" => fs::set_permissions(&c.path, fs::Permissions::from_mode(0o644)).unwrap(),
            "parent" => {
                fs::set_permissions(c.path.parent().unwrap(), fs::Permissions::from_mode(0o755))
                    .unwrap()
            }
            "hardlink" => fs::hard_link(&c.path, &held).unwrap(),
            _ => {
                fs::rename(&c.path, &held).unwrap();
                std::os::unix::fs::symlink(&held, &c.path).unwrap();
            }
        }
        c.refused(&c.pin);
        match variant {
            "mode" => fs::set_permissions(&c.path, fs::Permissions::from_mode(0o600)).unwrap(),
            "parent" => {
                fs::set_permissions(c.path.parent().unwrap(), fs::Permissions::from_mode(0o700))
                    .unwrap()
            }
            "hardlink" => fs::remove_file(&held).unwrap(),
            _ => {
                fs::remove_file(&c.path).unwrap();
                fs::rename(&held, &c.path).unwrap();
            }
        }
        c.valid("en");
    }
}
/// Default-valued forbidden flags and duplicates refuse before a private lock.
#[test]
fn provisioning_check_closed_flags_refuse_before_input_io() {
    for flags in [
        vec!["--root", "."],
        vec!["--phase", "delivery"],
        vec!["--server", "false"],
        vec!["--output", ""],
        vec!["--json"],
        vec!["--input", "duplicate"],
        vec!["--lang", "en"],
    ] {
        let c = Case::new();
        let mut args = c.args(&c.pin, "en");
        args.extend(flags.into_iter().map(str::to_owned));
        let mut out = vec![];
        let mut error = vec![];
        assert_ne!(swarm_cli::run(&args, "en", &mut out, &mut error), 0);
        assert!(out.is_empty());
        assert!(!c
            .path
            .parent()
            .unwrap()
            .join("private-documents.lock")
            .exists());
        assert_eq!(fs::read(&c.path).unwrap(), c.raw);
        assert!(!c.target.exists());
    }
}
struct Broken {
    flush: bool,
    bytes: Vec<u8>,
}
impl Write for Broken {
    /// Controlled output-only fault after successful validation; do not touch inputs.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if self.flush {
            self.bytes.extend_from_slice(raw);
            Ok(raw.len())
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "PRIVATE_PROVISIONING_OUTPUT",
            ))
        }
    }
    /// Keep late all-bytes-written/flush refusal distinct from write uncertainty.
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "PRIVATE_PROVISIONING_FLUSH",
        ))
    }
}
/// Known successful validation survives either output phase, with safe diagnostics.
#[test]
fn provisioning_check_output_and_flush_keep_validation_without_material() {
    let c = Case::new();
    for flush in [false, true] {
        c.valid("en");
        let mut out = Broken {
            flush,
            bytes: vec![],
        };
        let mut error = vec![];
        assert_ne!(
            swarm_cli::run(&c.args(&c.pin, "en"), "en", &mut out, &mut error),
            0
        );
        let text = String::from_utf8(error).unwrap();
        assert!(
            text.contains("provisioning-request-validated:true"),
            "{text}"
        );
        assert!(!text.contains("PRIVATE_") && !text.contains(c.path.to_str().unwrap()));
        if flush {
            let v: serde_json::Value = serde_json::from_slice(&out.bytes).unwrap();
            assert_eq!(v["request_validated"], true);
        }
        assert_eq!(fs::read(&c.path).unwrap(), c.raw);
        assert!(!c.target.exists());
        c.valid("en");
    }
}
