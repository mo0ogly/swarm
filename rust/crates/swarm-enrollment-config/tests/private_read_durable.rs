#![cfg(target_os = "linux")]
//! Explicit local-FS acknowledgements in isolated private roots, no live package.
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use swarm_enrollment_config::store::{
    documents::{Documents, ReadDurability, ReadValidation},
    Context, FixtureFault,
};
/// Actual private root/store with synthetic data; no policy default or live key.
fn fixture() -> (tempfile::TempDir, Documents, String) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let s = Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    let ctx = Context::new();
    let r = s.reserve(&ctx).unwrap();
    r.create("private", b"PRIVATE_DURABLE_BYTES").unwrap();
    r.finish().unwrap();
    (
        d,
        s,
        format!("{:x}", Sha256::digest(b"PRIVATE_DURABLE_BYTES")),
    )
}
/// Real successful witness before each durability fault injection.
fn valid() {
    let (_d, s, pin) = fixture();
    let ctx = Context::new();
    let r = s.reserve(&ctx).unwrap();
    assert!(
        r.read_private_durable("private", &pin).unwrap().as_bytes() == b"PRIVATE_DURABLE_BYTES"
    );
    r.finish().unwrap();
    s.close().unwrap();
}

/// Explicit operation acknowledges both real fsyncs, with exact private bytes.
#[test]
fn durable_private_read_actual_file_directory_ack_exact_and_reopen() {
    valid();
    let (d, s, pin) = fixture();
    let ctx = Context::new();
    let r = s.reserve(&ctx).unwrap();
    assert!(
        r.read_private_durable("private", &pin).unwrap().as_bytes() == b"PRIVATE_DURABLE_BYTES"
    );
    r.finish().unwrap();
    s.close().unwrap();
    let s = Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    let r = s.reserve(&ctx).unwrap();
    assert!(
        r.read_private_durable("private", &pin).unwrap().as_bytes() == b"PRIVATE_DURABLE_BYTES"
    );
    r.finish().unwrap();
    s.close().unwrap();
}

/// Wrong or malformed pin refuses before syncing; old observer never adopts sync.
#[test]
fn durable_private_read_wrong_pin_precedes_sync_and_plain_read_is_unchanged() {
    valid();
    let (_d, s, pin) = fixture();
    let ctx = Context::new();
    let r = s.reserve(&ctx).unwrap();
    s.fixture_faults(&[FixtureFault::FileSync]);
    assert!(r.read_private("private").unwrap().as_bytes() == b"PRIVATE_DURABLE_BYTES");
    let e = r
        .read_private_durable("private", &"00".repeat(32))
        .unwrap_err();
    assert_eq!(e.validation, ReadValidation::BytesRead);
    assert_eq!(e.durability, ReadDurability::NotAcknowledged);
    assert_eq!(e.cause.code(), "pins");
    let e = r.read_private_durable("private", "invalid").unwrap_err();
    assert_eq!(e.validation, ReadValidation::NotAttempted);
    assert_eq!(e.durability, ReadDurability::NotAcknowledged);
    s.fixture_faults(&[]);
    assert!(r.read_private_durable("private", &pin).is_ok());
    r.finish().unwrap();
    s.close().unwrap();
}

/// File/directory/post-close faults retain exactly the previous acknowledgements.
#[test]
fn durable_private_read_file_directory_close_failures_keep_ack_and_recover() {
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
        let (_d, s, pin) = fixture();
        let ctx = Context::new();
        let r = s.reserve(&ctx).unwrap();
        s.fixture_faults(&[fault]);
        let e = r.read_private_durable("private", &pin).unwrap_err();
        assert_eq!(e.validation, ReadValidation::PinMatched);
        assert_eq!(e.durability, phase);
        assert!(!format!("{e:?} {e}").contains("PRIVATE_DURABLE_BYTES"));
        s.fixture_faults(&[]);
        assert!(r.read_private_durable("private", &pin).is_ok());
        r.finish().unwrap();
        s.close().unwrap();
    }
}

/// Missing/cancel outcomes never become empty successful durable material.
#[test]
fn durable_private_read_missing_cancel_and_explicit_release_preserve_original() {
    valid();
    let (d, s, pin) = fixture();
    let ctx = Context::new();
    let r = s.reserve(&ctx).unwrap();
    let e = r.read_private_durable("absent", &pin).unwrap_err();
    assert_eq!(e.validation, ReadValidation::ReadUnknown);
    assert_eq!(e.durability, ReadDurability::NotAcknowledged);
    assert_eq!(e.cause.code(), "not_configured");
    ctx.cancel();
    let e = r.read_private_durable("private", &pin).unwrap_err();
    assert_eq!(e.durability, ReadDurability::NotAcknowledged);
    assert_eq!(e.cause.code(), "canceled");
    r.finish().unwrap();
    s.close().unwrap();
    assert_eq!(
        fs::read(d.path().join("private")).unwrap(),
        b"PRIVATE_DURABLE_BYTES"
    );
}
