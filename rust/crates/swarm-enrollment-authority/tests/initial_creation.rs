#![cfg(target_os = "linux")]
//! Initial private material only; no service Root/account/current authority.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use swarm_enrollment_authority::provisioning::{self as p, initial_creation as initial};
use swarm_enrollment_config::store::{documents::Publication, Context};
use zeroize::Zeroizing;
struct Case {
    root: tempfile::TempDir,
    operator: PathBuf,
    anchor: PathBuf,
    request: p::Request,
    raw: Vec<u8>,
    pin: String,
}
/// Fixture-only atomic private writes, preserving old bytes until synced replace.
fn write(path: &Path, raw: &[u8]) {
    let mut f = tempfile::NamedTempFile::new_in(path.parent().unwrap()).unwrap();
    f.as_file_mut().write_all(raw).unwrap();
    f.as_file().sync_all().unwrap();
    f.persist(path).unwrap();
    let dir = fs::File::open(path.parent().unwrap()).unwrap();
    dir.sync_all().unwrap();
}
/// Canonical digest of actual bytes, independent of later package observations.
fn pin(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
impl Case {
    /// Current-owned test directories and explicit UID/crypto data only. Service
    /// UID is synthetic other data; no account or namespace separation is claimed.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let operator = root.path().join("operator");
        let anchor_dir = root.path().join("anchor");
        for dir in [&operator, &anchor_dir] {
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
        let digest = pin(&raw);
        Self {
            root,
            operator,
            anchor: anchor_dir.join("pin.json"),
            request,
            raw,
            pin: digest,
        }
    }
    /// One public creation, without fixture hooks/provider/Root overrides.
    fn create(&self) -> Result<initial::Receipt, initial::Failure> {
        initial::create(
            &self.raw,
            &self.pin,
            &self.operator,
            &self.anchor,
            &Context::new(),
        )
    }
    /// Public reconciliation must derive no fresh material or implicit new attempt.
    fn reconcile(&self) -> Result<initial::Receipt, initial::Failure> {
        initial::reconcile(
            &self.raw,
            &self.pin,
            &self.operator,
            &self.anchor,
            &Context::new(),
        )
    }
    /// Package path is existing repository protocol, never a discovered arbitrary head.
    fn package(&self) -> PathBuf {
        self.operator.join("initial-provisioning-v1.bin")
    }
    /// Initial intent persists beside selected protected anchor, never inside service.
    fn intent(&self) -> PathBuf {
        self.anchor.with_file_name("pin.json.intent")
    }
    /// Assertions include meaningful state and no unexpected installation artifacts.
    fn created(&self) -> initial::Receipt {
        let receipt = self.create().unwrap();
        assert_eq!(receipt.progress.intent_publication, Publication::Durable);
        assert_eq!(receipt.progress.anchor_publication, Publication::Durable);
        assert_eq!(receipt.progress.package_publication, Publication::Durable);
        assert!(
            receipt.progress.material_generation_attempted
                && receipt.progress.material_generation_completed
        );
        assert!(self.intent().is_file() && self.anchor.is_file() && self.package().is_file());
        assert!(!self.root.path().join("absent-service").exists());
        receipt
    }
}
/// Valid full setup then exact explicit recovery, with immutable intent and pin.
#[test]
fn initial_create_then_reconcile_exact_private_material() {
    let c = Case::new();
    let receipt = c.created();
    let package = Zeroizing::new(fs::read(c.package()).unwrap());
    assert_eq!(receipt.checkpoint.request_pin(), c.pin);
    assert_eq!(receipt.checkpoint.package_pin(), pin(&package));
    let recovered = c.reconcile().unwrap();
    assert!(
        !recovered.progress.material_generation_attempted
            && !recovered.progress.material_generation_completed
    );
    assert_eq!(
        recovered.progress.package_publication,
        Publication::NotAttempted
    );
    assert!(recovered.progress.package_request_matched);
    assert!(
        fs::read(c.package()).unwrap().as_slice() == package.as_slice(),
        "package-rewritten"
    );
}
/// Any existing intent/anchor/package refuses before material generation, even
/// equal or malformed. No overwrite/adoption/automatic repair is a create path.
#[test]
fn initial_existing_slot_refuses_before_generation() {
    let witness = Case::new();
    witness.created();
    for which in ["intent", "anchor", "package"] {
        let c = Case::new();
        let path = match which {
            "intent" => c.intent(),
            "anchor" => c.anchor.clone(),
            _ => c.package(),
        };
        write(&path, b"EXISTING");
        let e = c.create().unwrap_err();
        assert!(!e.progress.material_generation_attempted);
        assert!(fs::read(&path).unwrap() == b"EXISTING");
        assert_eq!(e.progress.package_publication, Publication::NotAttempted);
    }
}
/// Raw digest/role preconditions fail before storage or generation; original input
/// restored succeeds. No request-selected controller UID is used for ownership.
#[test]
fn initial_wrong_request_pin_and_controller_refuse_before_files() {
    let witness = Case::new();
    witness.created();
    let c = Case::new();
    let e = initial::create(
        &c.raw,
        &"00".repeat(32),
        &c.operator,
        &c.anchor,
        &Context::new(),
    )
    .unwrap_err();
    assert!(!e.progress.material_generation_attempted);
    assert!(!c.intent().exists() && !c.anchor.exists() && !c.package().exists());
    let mut wrong = c.request.clone();
    std::mem::swap(&mut wrong.controller_uid, &mut wrong.service_uid);
    let raw = p::encode_request(&wrong).unwrap();
    let e = initial::create(&raw, &pin(&raw), &c.operator, &c.anchor, &Context::new()).unwrap_err();
    assert!(!e.progress.material_generation_attempted);
    assert!(!c.intent().exists() && !c.package().exists());
    c.created();
}
/// Existing Request codec requires exact canonical bytes. A pretty encoding
/// with its correct raw hash still refuses before generation; original recovers.
#[test]
fn initial_noncanonical_request_refuses_before_generation() {
    let c = Case::new();
    let raw = serde_json::to_vec_pretty(&c.request).unwrap();
    assert_ne!(pin(&raw), c.pin);
    let e = initial::create(&raw, &pin(&raw), &c.operator, &c.anchor, &Context::new()).unwrap_err();
    assert!(!e.progress.material_generation_attempted);
    assert!(!c.intent().exists() && !c.anchor.exists() && !c.package().exists());
    c.created();
    assert!(c.reconcile().unwrap().progress.package_request_matched);
}
/// Existing nonblocking locks reject physical alias without waiting or new policy.
#[test]
fn initial_same_directory_alias_refuses_without_material() {
    let witness = Case::new();
    witness.created();
    let c = Case::new();
    let alias = c.operator.join("pin.json");
    let e = initial::create(&c.raw, &c.pin, &c.operator, &alias, &Context::new()).unwrap_err();
    assert!(!e.progress.material_generation_attempted);
    assert!(!alias.exists() && !c.package().exists());
}
/// No regenerated material or replacement when independently anchored inputs go
/// missing/change; restoring the same original bytes explicitly recovers.
#[test]
fn initial_reconcile_missing_or_changed_files_refuses_then_recovers() {
    for which in ["anchor", "package"] {
        let c = Case::new();
        c.created();
        let path = if which == "anchor" {
            c.anchor.clone()
        } else {
            c.package()
        };
        let held = path.with_extension("held");
        fs::rename(&path, &held).unwrap();
        let e = c.reconcile().unwrap_err();
        assert!(!e.progress.material_generation_attempted);
        fs::rename(&held, &path).unwrap();
        let original = Zeroizing::new(fs::read(&path).unwrap());
        write(&path, b"INVALID");
        let e = c.reconcile().unwrap_err();
        assert!(!e.progress.material_generation_attempted);
        assert!(fs::read(&path).unwrap() == b"INVALID");
        write(&path, &original);
        assert!(c.reconcile().unwrap().progress.package_request_matched);
    }
}
/// Intent without an anchor is retained and explicitly refuses regeneration;
/// reconciliation does not turn incomplete provisioning into empty success.
#[test]
fn initial_orphan_intent_never_regenerates_or_disappears() {
    let witness = Case::new();
    witness.created();
    let c = Case::new();
    let raw=format!("{{\"schema_version\":1,\"protocol\":\"swarm-initial-provisioning-intent-v1\",\"request_pin\":\"{}\"}}",c.pin);
    write(&c.intent(), raw.as_bytes());
    let e = c.create().unwrap_err();
    assert!(!e.progress.material_generation_attempted);
    let e = c.reconcile().unwrap_err();
    assert!(!e.progress.material_generation_attempted);
    assert_eq!(fs::read(c.intent()).unwrap(), raw.as_bytes());
    assert!(!c.anchor.exists() && !c.package().exists());
}
/// Another valid request cannot borrow the old anchor/package as its own setup.
#[test]
fn initial_reconcile_different_request_refuses_current_bytes() {
    let c = Case::new();
    c.created();
    let before = pin(&fs::read(c.package()).unwrap());
    let mut changed = c.request.clone();
    changed.personal_generation += 1;
    let raw = p::encode_request(&changed).unwrap();
    let e =
        initial::reconcile(&raw, &pin(&raw), &c.operator, &c.anchor, &Context::new()).unwrap_err();
    assert!(
        e.progress.intent_decoded,
        "canonical-intent-decode-lost-after-request-mismatch"
    );
    assert!(!e.progress.material_generation_attempted);
    assert_eq!(pin(&fs::read(c.package()).unwrap()), before);
    assert!(c.reconcile().is_ok());
}
