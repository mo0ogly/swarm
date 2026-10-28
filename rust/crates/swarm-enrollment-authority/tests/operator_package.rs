//! Isolated operator package fixtures, no live key, grant, journal or service.
use std::{fs, os::unix::fs::PermissionsExt};
use swarm_enrollment_authority::provisioning::{self as p, repository as v};
use swarm_enrollment_config::store::{
    documents::{Documents, Publication},
    Context, FixtureFault,
};
/// Real private temporary directory and a descriptor-bound store. The Documents
/// clone is retained solely for isolated public test-fixture fault injection.
fn fixture() -> (tempfile::TempDir, Documents, v::PackageStore) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let docs = Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    let store = v::PackageStore::new(docs.clone());
    (d, docs, store)
}
/// Explicit synthetic operator request, with no production policy default.
fn material() -> p::Prepared {
    let r: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    p::prepare(&r).unwrap()
}
/// Same meaningful valid create/load before each injection.
fn valid() {
    let (_d, _docs, s) = fixture();
    let seal = v::seal(material()).unwrap();
    s.create(&Context::new(), &seal).unwrap();
    let read = s.load(&Context::new(), seal.pin()).unwrap();
    material_bytes(&seal, &read);
    s.close().unwrap();
}
/// Compare recovered encoding via its pin, avoiding Debug-printing private data.
fn material_bytes(seal: &v::Seal, read: &p::Prepared) -> zeroize::Zeroizing<Vec<u8>> {
    let raw = read.encode_private().unwrap();
    use sha2::{Digest, Sha256};
    assert_eq!(format!("{:x}", Sha256::digest(&*raw)), seal.pin().as_str());
    raw
}

/// Exact pin is available before IO; create never adopts/replaces an existing file.
#[test]
fn operator_package_unique_create_exact_load_reopen_and_redaction() {
    let (d, _docs, s) = fixture();
    let seal = v::seal(material()).unwrap();
    let pin = seal.pin().clone();
    assert_eq!(pin.as_str().len(), 64);
    assert_eq!(format!("{seal:?}"), "OperatorPackageSeal { <private> }");
    assert_eq!(
        s.create(&Context::new(), &seal).unwrap(),
        Publication::Durable
    );
    let read = s.load(&Context::new(), &pin).unwrap();
    material_bytes(&seal, &read);
    let e = s.create(&Context::new(), &seal).unwrap_err();
    assert_eq!(e.publication, Publication::NotAttempted);
    assert_eq!(e.cause.code(), "exists");
    s.close().unwrap();
    let docs = Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    let s = v::PackageStore::new(docs);
    material_bytes(&seal, &s.load(&Context::new(), &pin).unwrap());
    s.close().unwrap();
}

/// Wrong pin and actual file tampering refuse without returning or rotating keys.
#[test]
fn operator_package_wrong_pin_and_tamper_refuse_then_original_recovers() {
    valid();
    let (d, _docs, s) = fixture();
    let prepared = material();
    let raw = prepared.encode_private().unwrap();
    let seal = v::seal(prepared).unwrap();
    s.create(&Context::new(), &seal).unwrap();
    let wrong = v::Pin::parse(&"00".repeat(32)).unwrap();
    let e = s.load(&Context::new(), &wrong).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::BytesRead);
    assert_eq!(e.cause.code(), "operator-package-pin");
    let path = d.path().join("initial-provisioning-v1.bin");
    fs::write(&path, b"private-corrupt").unwrap();
    assert!(s.load(&Context::new(), seal.pin()).is_err());
    fs::write(&path, &*raw).unwrap();
    material_bytes(&seal, &s.load(&Context::new(), seal.pin()).unwrap());
    s.close().unwrap();
}

/// Actual pre-rename faults leave no final package; same sealed material retries.
#[test]
fn operator_package_prepublication_faults_preserve_same_candidate_and_recover() {
    for fault in [
        FixtureFault::Write,
        FixtureFault::PartialWrite,
        FixtureFault::FileSync,
        FixtureFault::FileClose,
    ] {
        valid();
        let (d, docs, s) = fixture();
        let seal = v::seal(material()).unwrap();
        let pin = seal.pin().clone();
        docs.fixture_faults(&[fault]);
        let e = s.create(&Context::new(), &seal).unwrap_err();
        assert_eq!(e.publication, Publication::NotAttempted);
        assert!(!d.path().join("initial-provisioning-v1.bin").exists());
        docs.fixture_faults(&[]);
        assert_eq!(
            s.create(&Context::new(), &seal).unwrap(),
            Publication::Durable
        );
        assert_eq!(seal.pin().as_str(), pin.as_str());
        material_bytes(&seal, &s.load(&Context::new(), &pin).unwrap());
        s.close().unwrap();
    }
}

/// Known publication survives directorysync/unlock errors; read is observation,
/// not an implicit new durability acknowledgement or permission to overwrite.
#[test]
fn operator_package_late_errors_preserve_visible_durable_and_exact_recovery() {
    for (fault, phase) in [
        (FixtureFault::DirectorySync, Publication::Visible),
        (FixtureFault::Unlock, Publication::Durable),
    ] {
        valid();
        let (_d, docs, s) = fixture();
        let seal = v::seal(material()).unwrap();
        docs.fixture_faults(&[fault]);
        let e = s.create(&Context::new(), &seal).unwrap_err();
        assert_eq!(e.publication, phase);
        docs.fixture_faults(&[]);
        material_bytes(&seal, &s.load(&Context::new(), seal.pin()).unwrap());
        let again = s.create(&Context::new(), &seal).unwrap_err();
        assert_eq!(again.publication, Publication::NotAttempted);
        assert_eq!(again.cause.code(), "exists");
        s.close().unwrap();
    }
}

/// Read observations and independent release errors survive, with material withheld.
#[test]
fn operator_package_read_and_release_causes_cancel_and_close_remain_explicit() {
    valid();
    let (_d, docs, s) = fixture();
    let seal = v::seal(material()).unwrap();
    s.create(&Context::new(), &seal).unwrap();
    docs.fixture_faults(&[FixtureFault::Unlock]);
    let e = s.load(&Context::new(), seal.pin()).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::Decoded);
    let wrong = v::Pin::parse(&"00".repeat(32)).unwrap();
    let e = s.load(&Context::new(), &wrong).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::BytesRead);
    assert_eq!(e.cause.code(), "operator-package-pin");
    assert!(e.cause.cleanup_error().is_some());
    docs.fixture_faults(&[]);
    let ctx = Context::new();
    ctx.cancel();
    let e = s.load(&ctx, seal.pin()).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::NotAttempted);
    material_bytes(&seal, &s.load(&Context::new(), seal.pin()).unwrap());
    s.close().unwrap();
}

/// Matching an external byte digest is not permission to accept a malformed
/// material frame; refusal leaves those exact bytes intact and issues no key.
#[test]
fn operator_package_matching_pin_malformed_payload_refuses_without_adoption() {
    valid();
    let (_d, docs, s) = fixture();
    let ctx = Context::new();
    let r = docs.reserve(&ctx).unwrap();
    let raw = b"PRIVATE_INVALID_MATERIAL";
    r.create("initial-provisioning-v1.bin", raw).unwrap();
    r.finish().unwrap();
    use sha2::{Digest, Sha256};
    let pin = v::Pin::parse(&format!("{:x}", Sha256::digest(raw))).unwrap();
    let e = s.load(&ctx, &pin).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::PinMatched);
    assert_eq!(e.cause.code(), "provisioning-request");
    assert!(!format!("{e:?} {e}").contains("PRIVATE_INVALID_MATERIAL"));
    let r = docs.reserve(&ctx).unwrap();
    assert!(
        r.read_private("initial-provisioning-v1.bin")
            .unwrap()
            .as_bytes()
            == raw
    );
    r.finish().unwrap();
    s.close().unwrap();
}

/// Missing/read-close/owner-close outcomes stay typed, with recovery from the
/// original independently known pin and no fallback or replacement package.
#[test]
fn operator_package_pin_shape_missing_read_close_and_owner_close_recover() {
    valid();
    for raw in [
        String::new(),
        "AB".repeat(32),
        "x".repeat(64),
        "00".repeat(31),
    ] {
        assert!(v::Pin::parse(&raw).is_err());
    }
    let (d, docs, s) = fixture();
    let seal = v::seal(material()).unwrap();
    let pin = seal.pin().clone();
    let e = s.load(&Context::new(), &pin).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::ReadUnknown);
    assert_eq!(e.cause.code(), "not_configured");
    s.create(&Context::new(), &seal).unwrap();
    material_bytes(&seal, &s.load(&Context::new(), &pin).unwrap());
    docs.fixture_faults(&[FixtureFault::ReadClose]);
    let e = s.load(&Context::new(), &pin).unwrap_err();
    assert_eq!(e.observation, v::ReadObservation::ReadUnknown);
    assert_eq!(e.cause.code(), "storage");
    docs.fixture_faults(&[]);
    material_bytes(&seal, &s.load(&Context::new(), &pin).unwrap());
    docs.fixture_faults(&[FixtureFault::StoreClose]);
    let e = s.close().unwrap_err();
    assert_eq!(e.code(), "storage");
    docs.fixture_faults(&[]);
    docs.close().unwrap();
    let fresh = v::PackageStore::new(
        Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap(),
    );
    material_bytes(&seal, &fresh.load(&Context::new(), &pin).unwrap());
    fresh.close().unwrap();
}
