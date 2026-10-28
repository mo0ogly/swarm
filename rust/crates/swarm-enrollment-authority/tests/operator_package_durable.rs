//! Explicit operator-package recovery, no installed service or human enrollment.
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use swarm_enrollment_authority::provisioning::{self as p, repository as v};
use swarm_enrollment_config::store::{
    documents::{Documents, ReadDurability},
    Context, FixtureFault,
};
/// Private operator fixture, fault clone only for current test-fixtures feature.
fn fixture() -> (tempfile::TempDir, Documents, v::PackageStore, v::Seal) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let docs = Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    let request: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    let seal = v::seal(p::prepare(&request).unwrap()).unwrap();
    let store = v::PackageStore::new(docs.clone());
    (d, docs, store, seal)
}
/// Match actual recovered encoding to the independently known candidate digest.
fn matches(seal: &v::Seal, prepared: p::Prepared) {
    let bytes = prepared.encode_private().unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&*bytes)),
        seal.pin().as_str()
    );
}
/// Establish same actual two-fsync witness before each injected refusal.
fn valid() {
    let (_d, _docs, s, seal) = fixture();
    s.create(&Context::new(), &seal).unwrap();
    matches(&seal, s.load_durable(&Context::new(), seal.pin()).unwrap());
    s.close().unwrap();
}

/// Recovery from a visible package explicitly acknowledges its file and directory.
#[test]
fn operator_package_durable_recovers_visible_exact_material_without_new_entropy() {
    valid();
    let (_d, docs, s, seal) = fixture();
    docs.fixture_faults(&[FixtureFault::DirectorySync]);
    let e = s.create(&Context::new(), &seal).unwrap_err();
    assert_eq!(
        e.publication,
        swarm_enrollment_config::store::documents::Publication::Visible
    );
    docs.fixture_faults(&[]);
    matches(&seal, s.load_durable(&Context::new(), seal.pin()).unwrap());
    matches(&seal, s.load_durable(&Context::new(), seal.pin()).unwrap());
    s.close().unwrap();
}

/// Every late failure preserves actual acknowledgements and withholds keys.
#[test]
fn operator_package_durable_sync_close_unlock_failures_retain_known_ack() {
    for (fault, phase, observation) in [
        (
            FixtureFault::FileSync,
            ReadDurability::NotAcknowledged,
            v::ReadObservation::PinMatched,
        ),
        (
            FixtureFault::DirectorySync,
            ReadDurability::FileAcknowledged,
            v::ReadObservation::PinMatched,
        ),
        (
            FixtureFault::ReadClose,
            ReadDurability::DirectoryAcknowledged,
            v::ReadObservation::PinMatched,
        ),
        (
            FixtureFault::Unlock,
            ReadDurability::DirectoryAcknowledged,
            v::ReadObservation::Decoded,
        ),
    ] {
        valid();
        let (_d, docs, s, seal) = fixture();
        s.create(&Context::new(), &seal).unwrap();
        docs.fixture_faults(&[fault]);
        let e = s.load_durable(&Context::new(), seal.pin()).unwrap_err();
        assert_eq!(e.durability, phase);
        assert_eq!(e.observation, observation);
        docs.fixture_faults(&[]);
        matches(&seal, s.load_durable(&Context::new(), seal.pin()).unwrap());
        s.close().unwrap();
    }
}

/// Pin mismatch precedes fsync faults; historical load remains observation only.
#[test]
fn operator_package_durable_wrong_pin_before_sync_plain_load_stays_plain() {
    valid();
    let (_d, docs, s, seal) = fixture();
    s.create(&Context::new(), &seal).unwrap();
    docs.fixture_faults(&[FixtureFault::FileSync]);
    matches(&seal, s.load(&Context::new(), seal.pin()).unwrap());
    let wrong = v::Pin::parse(&"00".repeat(32)).unwrap();
    let e = s.load_durable(&Context::new(), &wrong).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::BytesRead);
    assert_eq!(e.durability, ReadDurability::NotAcknowledged);
    assert_eq!(e.cause.code(), "operator-package-pin");
    let plain = s.load(&Context::new(), &wrong).unwrap_err();
    assert_eq!(plain.observation, v::ReadObservation::BytesRead);
    assert_eq!(plain.durability, ReadDurability::NotAcknowledged);
    docs.fixture_faults(&[FixtureFault::ReadClose]);
    let joined = s.load_durable(&Context::new(), &wrong).unwrap_err();
    assert_eq!(joined.observation, v::ReadObservation::BytesRead);
    assert_eq!(joined.durability, ReadDurability::NotAcknowledged);
    assert_eq!(joined.cause.code(), "operator-package-pin");
    let lower = std::error::Error::source(&joined.cause)
        .unwrap()
        .downcast_ref::<swarm_enrollment_config::store::documents::DurableReadError>()
        .unwrap();
    assert!(lower.cause.cleanup_error().is_some());
    docs.fixture_faults(&[]);
    matches(&seal, s.load_durable(&Context::new(), seal.pin()).unwrap());
    s.close().unwrap();
}

/// Matching pinned bytes can be durable yet invalid material; keep that known
/// filesystem effect in the decode error and never return an empty/key result.
#[test]
fn operator_package_durable_invalid_matching_frame_keeps_ack_in_error() {
    valid();
    let (_d, docs, s, _seal) = fixture();
    let ctx = Context::new();
    let raw = b"PRIVATE_INVALID_MATERIAL";
    let pin = v::Pin::parse(&format!("{:x}", Sha256::digest(raw))).unwrap();
    let r = docs.reserve(&ctx).unwrap();
    r.create("initial-provisioning-v1.bin", raw).unwrap();
    r.finish().unwrap();
    let e = s.load_durable(&ctx, &pin).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::PinMatched);
    assert_eq!(e.durability, ReadDurability::DirectoryAcknowledged);
    assert_eq!(e.cause.code(), "provisioning-request");
    assert!(!format!("{e:?} {e}").contains("PRIVATE_INVALID_MATERIAL"));
    s.close().unwrap();
}
