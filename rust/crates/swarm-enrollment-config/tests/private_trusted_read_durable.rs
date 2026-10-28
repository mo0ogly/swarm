#![cfg(target_os = "linux")]
//! Explicit trusted-locator FS acknowledgement; no independently pinned claim.
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
};
use swarm_enrollment_config::{
    store::{
        documents::{Documents, ReadDurability, ReadValidation},
        Context, FixtureFault,
    },
    Error,
};
/// Synthetic anchor data and actual private directories only; no live identities.
fn fixture() -> (tempfile::TempDir, Documents) {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let docs = Documents::open(root.path(), rustix::process::geteuid().as_raw()).unwrap();
    let ctx = Context::new();
    let r = docs.reserve(&ctx).unwrap();
    r.create("anchor.json", b"SYNTHETIC_ANCHOR").unwrap();
    r.finish().unwrap();
    (root, docs)
}
/// A real successful witness precedes every injected refusal, with explicit closes.
fn valid() {
    let (_root, docs) = fixture();
    let ctx = Context::new();
    let r = docs.reserve(&ctx).unwrap();
    assert!(
        r.read_trusted_private_durable("anchor.json")
            .unwrap()
            .as_bytes()
            == b"SYNTHETIC_ANCHOR"
    );
    r.finish().unwrap();
    docs.close().unwrap();
}
/// Both syncs acknowledge exact unchanged bytes; reopen does not learn a package.
#[test]
fn trusted_private_read_exact_ack_and_reopen() {
    valid();
    let (root, docs) = fixture();
    let ctx = Context::new();
    let r = docs.reserve(&ctx).unwrap();
    assert!(
        r.read_trusted_private_durable("anchor.json")
            .unwrap()
            .as_bytes()
            == b"SYNTHETIC_ANCHOR"
    );
    r.finish().unwrap();
    docs.close().unwrap();
    let docs = Documents::open(root.path(), rustix::process::geteuid().as_raw()).unwrap();
    let r = docs.reserve(&ctx).unwrap();
    assert!(r.read_trusted_private_durable("anchor.json").is_ok());
    r.finish().unwrap();
    docs.close().unwrap();
}
/// Every ack remains a byte-read fact, never PinMatched; retry uses same bytes.
#[test]
fn trusted_private_read_faults_keep_ack_without_pin_and_recover() {
    for (fault, phase) in [
        (FixtureFault::FileSync, ReadDurability::NotAcknowledged),
        (
            FixtureFault::DirectorySync,
            ReadDurability::FileAcknowledged,
        ),
        (
            FixtureFault::ReadClose,
            ReadDurability::DirectoryAcknowledged,
        ),
    ] {
        valid();
        let (root, docs) = fixture();
        let ctx = Context::new();
        let r = docs.reserve(&ctx).unwrap();
        docs.fixture_faults(&[fault]);
        let e = r.read_trusted_private_durable("anchor.json").unwrap_err();
        assert_eq!(e.validation, ReadValidation::BytesRead);
        assert_eq!(e.durability, phase);
        assert!(std::error::Error::source(&e).is_some());
        docs.fixture_faults(&[]);
        assert!(r.read_trusted_private_durable("anchor.json").is_ok());
        assert!(fs::read(root.path().join("anchor.json")).unwrap() == b"SYNTHETIC_ANCHOR");
        r.finish().unwrap();
        docs.close().unwrap();
    }
}
/// Added entry cannot weaken the existing mandatory-pin verification-before-sync.
#[test]
fn trusted_private_read_preserves_independent_pin_refusal() {
    valid();
    let (_root, docs) = fixture();
    let ctx = Context::new();
    let r = docs.reserve(&ctx).unwrap();
    assert!(r.read_trusted_private_durable("anchor.json").is_ok());
    docs.fixture_faults(&[FixtureFault::FileSync]);
    let e = r
        .read_private_durable("anchor.json", &"00".repeat(32))
        .unwrap_err();
    assert_eq!(e.validation, ReadValidation::BytesRead);
    assert_eq!(e.durability, ReadDurability::NotAcknowledged);
    assert!(matches!(e.cause, Error::Pins));
    let e = r.read_private_durable("anchor.json", "bad").unwrap_err();
    assert_eq!(e.validation, ReadValidation::NotAttempted);
    assert!(matches!(e.cause, Error::Pins));
    docs.fixture_faults(&[]);
    assert!(r.read_trusted_private_durable("anchor.json").is_ok());
    r.finish().unwrap();
    docs.close().unwrap();
}
/// A trusted locator does not waive private metadata, links, missing or name checks.
#[test]
fn trusted_private_read_unsafe_files_and_names_refuse_then_restore() {
    for variant in ["mode", "hardlink", "symlink", "missing", "name", "reserved"] {
        valid();
        let (root, docs) = fixture();
        let path = root.path().join("anchor.json");
        let held = root.path().join("held.json");
        let ctx = Context::new();
        let r = docs.reserve(&ctx).unwrap();
        let selected = match variant {
            "mode" => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
                "anchor.json"
            }
            "hardlink" => {
                fs::hard_link(&path, &held).unwrap();
                "anchor.json"
            }
            "symlink" => {
                fs::rename(&path, &held).unwrap();
                symlink(&held, &path).unwrap();
                "anchor.json"
            }
            "missing" => {
                fs::rename(&path, &held).unwrap();
                "anchor.json"
            }
            "name" => "../anchor.json",
            _ => "private-documents.lock",
        };
        let e = r.read_trusted_private_durable(selected).unwrap_err();
        assert_ne!(e.validation, ReadValidation::PinMatched);
        assert_eq!(e.durability, ReadDurability::NotAcknowledged);
        match variant {
            "mode" => fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap(),
            "hardlink" => fs::remove_file(&held).unwrap(),
            "symlink" => {
                fs::remove_file(&path).unwrap();
                fs::rename(&held, &path).unwrap();
            }
            "missing" => fs::rename(&held, &path).unwrap(),
            _ => {}
        }
        assert!(r.read_trusted_private_durable("anchor.json").is_ok());
        r.finish().unwrap();
        docs.close().unwrap();
    }
}
/// Cancellation refuses before IO, with no private buffer or phantom ack returned.
#[test]
fn trusted_private_read_cancelled_then_fresh_context_recovers() {
    valid();
    let (_root, docs) = fixture();
    let ctx = Context::new();
    let r = docs.reserve(&ctx).unwrap();
    ctx.cancel();
    let e = r.read_trusted_private_durable("anchor.json").unwrap_err();
    assert_eq!(e.validation, ReadValidation::NotAttempted);
    assert_eq!(e.durability, ReadDurability::NotAcknowledged);
    assert!(matches!(e.cause, Error::Canceled));
    r.finish().unwrap();
    let fresh = Context::new();
    let r = docs.reserve(&fresh).unwrap();
    assert!(r.read_trusted_private_durable("anchor.json").is_ok());
    r.finish().unwrap();
    docs.close().unwrap();
}
