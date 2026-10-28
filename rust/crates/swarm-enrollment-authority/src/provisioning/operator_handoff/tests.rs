//! Actual isolated storage failures and pure typed wrapper regressions.
use super::*;
/// Test-only wide-identity traversal of this kernel’s declared wrapper matrix.
/// It retains primary/source, custom IO and independent cleanup without logging.
fn causes<'a>(error: &'a (dyn StdError + 'static)) -> Vec<&'a (dyn StdError + 'static)> {
    let mut pending = vec![error];
    let mut seen: Vec<*const dyn StdError> = vec![];
    let mut result = vec![];
    while let Some(e) = pending.pop() {
        let p = e as *const dyn StdError;
        if seen.iter().any(|old| std::ptr::eq(*old, p)) {
            continue;
        }
        seen.push(p);
        result.push(e);
        pending.extend(error_links(e).1);
        if let Some(cause) = e.source() {
            pending.push(cause);
        }
    }
    result
}

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use zeroize::Zeroizing;
struct Case {
    root: tempfile::TempDir,
    operator: PathBuf,
    pin: package::Pin,
    raw: Zeroizing<Vec<u8>>,
    artifact_pin: String,
}
/// Private fixture permissions only; never modifies live state.
fn mode(path: &Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap()
}
impl Case {
    /// Store an explicit synthetic package with or without matching controller UID.
    /// Fixture draws entropy and writes keys only in its private operator vault.
    fn new(wrong_role: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        mode(root.path());
        let operator = root.path().join("operator");
        fs::create_dir(&operator).unwrap();
        mode(&operator);
        let uid = rustix::process::geteuid().as_raw();
        let other = if uid == 0 { 1 } else { !uid };
        let mut request: material::Request = serde_json::from_str(include_str!(
            "../../../../../../tests/fixtures/paea_provisioning_request.json"
        ))
        .unwrap();
        request.controller_uid = if wrong_role { other } else { uid };
        request.service_uid = if wrong_role { uid } else { other };
        request.root_directory = root.path().join("absent-service").to_str().unwrap().into();
        let prepared = material::prepare(&request).unwrap();
        let artifact = service_artifact::from_prepared(&prepared).unwrap();
        let raw = artifact.encode_private().unwrap();
        let artifact_pin = artifact.pin().unwrap();
        drop(artifact);
        let seal = package::seal(prepared).unwrap();
        let pin = seal.pin().clone();
        let store = package::PackageStore::new(Documents::open(&operator, uid).unwrap());
        assert_eq!(
            store.create(&store::Context::new(), &seal).unwrap(),
            Publication::Durable
        );
        store.close().unwrap();
        Self {
            root,
            operator,
            pin,
            raw,
            artifact_pin,
        }
    }
    /// New private handoff directory belongs to this fixture only.
    fn handoff(&self) -> PathBuf {
        let d = self.root.path().join("handoff");
        fs::create_dir(&d).unwrap();
        mode(&d);
        d
    }
}
/// Valid read before real unlock/store-close failures; keys withheld, progress and
/// independent branches retained, explicit clean reopen works without target.
#[test]
#[cfg(feature = "test-fixtures")]
fn operator_input_late_release_close_keep_decode_and_withhold_keys_then_recover() {
    let c = Case::new(false);
    let uid = rustix::process::geteuid().as_raw();
    assert!(load(&c.operator, c.pin.as_str(), uid).is_ok());
    let docs = Documents::open(&c.operator, uid).unwrap();
    docs.fixture_faults(&[store::FixtureFault::Unlock, store::FixtureFault::StoreClose]);
    let e = load_owned(package::PackageStore::new(docs), &c.pin, uid).unwrap_err();
    assert_eq!(e.progress.read, package::ReadObservation::Decoded);
    assert_eq!(e.progress.durability, ReadDurability::DirectoryAcknowledged);
    let error = Error::Input(e);
    assert!(
        causes(&error)
            .iter()
            .filter(|e| e.downcast_ref::<config::Error>().is_some())
            .count()
            >= 2
    );
    assert!(load(&c.operator, c.pin.as_str(), uid).is_ok());
    assert!(!c.root.path().join("absent-service").exists());
}
/// A wrong declared controller remains primary even if source close also fails;
/// a valid current-UID package control precedes this separate invalid fixture.
#[test]
#[cfg(feature = "test-fixtures")]
fn operator_wrong_role_and_close_keep_primary_refusal_without_handoff() {
    let valid = Case::new(false);
    let uid = rustix::process::geteuid().as_raw();
    assert!(load(&valid.operator, valid.pin.as_str(), uid).is_ok());
    let c = Case::new(true);
    assert!(load(&c.operator, c.pin.as_str(), uid).is_err());
    let docs = Documents::open(&c.operator, uid).unwrap();
    docs.fixture_faults(&[store::FixtureFault::StoreClose]);
    let e = load_owned(package::PackageStore::new(docs), &c.pin, uid).unwrap_err();
    assert_eq!(e.cause.code(), "operator-controller-uid");
    assert_eq!(e.progress.read, package::ReadObservation::Decoded);
    let error = Error::Input(e);
    assert!(causes(&error)
        .iter()
        .any(|e| e.downcast_ref::<config::Error>().is_some()));
    assert!(!c.root.path().join("absent-service").exists());
    assert!(load(&valid.operator, valid.pin.as_str(), uid).is_ok());
}
/// Actual rename then directory-sync/unlock/close refusal keeps visible output;
/// expected bytes remain, explicit reconciliation acknowledges without rewriting.
#[test]
#[cfg(feature = "test-fixtures")]
fn operator_export_visible_then_cleanup_failure_reconciles_exact_bytes() {
    let c = Case::new(false);
    let uid = rustix::process::geteuid().as_raw();
    let d = c.handoff();
    assert!(target(
        &d.join("control.bin"),
        Action::Export,
        &c.raw,
        &c.artifact_pin,
        uid
    )
    .is_ok());
    let docs = Documents::open(&d, uid).unwrap();
    docs.fixture_faults(&[
        store::FixtureFault::DirectorySync,
        store::FixtureFault::Unlock,
        store::FixtureFault::StoreClose,
    ]);
    let (progress, cause) =
        target_owned(docs, "service.bin", Action::Export, &c.raw, &c.artifact_pin).unwrap_err();
    assert_eq!(progress.publication, Publication::Visible);
    assert!(
        fs::read(d.join("service.bin")).unwrap().as_slice() == c.raw.as_slice(),
        "service-byte-mismatch"
    );
    let mut report = Report::new(Action::Export, c.pin.as_str(), &c.artifact_pin);
    report.target(progress, false);
    let error = Error::Target { report, cause };
    assert!(
        causes(&error)
            .iter()
            .filter(|e| e.downcast_ref::<config::Error>().is_some())
            .count()
            >= 3
    );
    let observed = target(
        &d.join("service.bin"),
        Action::Reconcile,
        &c.raw,
        &c.artifact_pin,
        uid,
    )
    .unwrap();
    assert_eq!(observed.durability, ReadDurability::DirectoryAcknowledged);
}
/// Read fsync failure preserves matched pin and file acknowledgement, never
/// claims prior publication; exact clean read recovery leaves bytes unchanged.
#[test]
#[cfg(feature = "test-fixtures")]
fn operator_reconcile_read_ack_failure_keeps_pin_and_all_cleanup() {
    let c = Case::new(false);
    let uid = rustix::process::geteuid().as_raw();
    let d = c.handoff();
    let path = d.join("service.bin");
    assert!(target(&path, Action::Export, &c.raw, &c.artifact_pin, uid).is_ok());
    assert!(target(&path, Action::Reconcile, &c.raw, &c.artifact_pin, uid).is_ok());
    let docs = Documents::open(&d, uid).unwrap();
    docs.fixture_faults(&[
        store::FixtureFault::DirectorySync,
        store::FixtureFault::Unlock,
        store::FixtureFault::StoreClose,
    ]);
    let (progress, cause) = target_owned(
        docs,
        "service.bin",
        Action::Reconcile,
        &c.raw,
        &c.artifact_pin,
    )
    .unwrap_err();
    assert_eq!(progress.publication, Publication::NotAttempted);
    assert_eq!(progress.validation, ReadValidation::PinMatched);
    assert_eq!(progress.durability, ReadDurability::FileAcknowledged);
    let mut report = Report::new(Action::Reconcile, c.pin.as_str(), &c.artifact_pin);
    report.target(progress, false);
    let error = Error::Target { report, cause };
    assert!(
        causes(&error)
            .iter()
            .filter(|e| e.downcast_ref::<config::Error>().is_some())
            .count()
            >= 3
    );
    assert!(target(&path, Action::Reconcile, &c.raw, &c.artifact_pin, uid).is_ok());
    assert!(
        fs::read(path).unwrap().as_slice() == c.raw.as_slice(),
        "service-byte-mismatch"
    );
}
/// Temporary-name entropy failure precedes output publication; a valid control
/// comes first and explicit retry writes the same already-derived artifact.
#[test]
#[cfg(feature = "test-fixtures")]
fn operator_temporary_nonce_failure_refuses_then_retries_same_material() {
    let c = Case::new(false);
    let uid = rustix::process::geteuid().as_raw();
    let d = c.handoff();
    assert!(target(
        &d.join("control.bin"),
        Action::Export,
        &c.raw,
        &c.artifact_pin,
        uid
    )
    .is_ok());
    let docs = Documents::open(&d, uid).unwrap();
    docs.fixture_faults(&[store::FixtureFault::Entropy]);
    let (progress, _cause) =
        target_owned(docs, "service.bin", Action::Export, &c.raw, &c.artifact_pin).unwrap_err();
    assert_eq!(progress.publication, Publication::NotAttempted);
    assert!(!d.join("service.bin").exists());
    let result = target(
        &d.join("service.bin"),
        Action::Export,
        &c.raw,
        &c.artifact_pin,
        uid,
    )
    .unwrap();
    assert_eq!(result.publication, Publication::Durable);
    assert!(
        fs::read(d.join("service.bin")).unwrap().as_slice() == c.raw.as_slice(),
        "retry-changed-service-material"
    );
    assert!(load(&c.operator, c.pin.as_str(), uid).is_ok());
}
