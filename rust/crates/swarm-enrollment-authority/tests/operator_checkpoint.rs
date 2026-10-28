#![cfg(target_os = "linux")]
//! Actual private synthetic pin records; no keys/package/Root or current authority.
use std::{error::Error as StdError, fs, os::unix::fs::PermissionsExt};
use swarm_enrollment_authority::provisioning::{
    checkpoint::{self as c, Checkpoint},
    repository,
};
use swarm_enrollment_config::{
    self as config,
    store::{
        documents::{Documents, Publication, ReadDurability, ReadValidation},
        Context, FixtureFault,
    },
};
/// Canonical synthetic data only, not an approved request or issued package.
fn record() -> Checkpoint {
    Checkpoint::new(
        &"ab".repeat(32),
        &repository::Pin::parse(&"cd".repeat(32)).unwrap(),
    )
    .unwrap()
}
/// Real current-owned private fixture; every operation explicitly closes itself.
fn fixture() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    d
}
/// Open current-owned fixture only, never a service/mission store.
fn open(d: &tempfile::TempDir) -> Documents {
    Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap()
}
/// Valid exact create/load witness before injected failures.
fn valid() {
    let d = fixture();
    assert_eq!(
        c::create(open(&d), "anchor.json", &record(), &Context::new()).unwrap(),
        Publication::Durable
    );
    let got = c::load(open(&d), "anchor.json", &"ab".repeat(32), &Context::new()).unwrap();
    assert_eq!(got.encode().unwrap(), record().encode().unwrap());
}
/// Closed canonical metadata has no key material and never self-authenticates.
#[test]
fn checkpoint_canonical_closed_record_refuses_ambiguous_bytes() {
    let value = record();
    let raw = value.encode().unwrap();
    let decoded = Checkpoint::decode(&raw).unwrap();
    assert_eq!(decoded.request_pin(), &"ab".repeat(32));
    assert_eq!(decoded.package_pin(), &"cd".repeat(32));
    assert_eq!(format!("{decoded:?}"), "OperatorCheckpoint { <private> }");
    let mut extra = raw.clone();
    extra.push(b' ');
    assert!(Checkpoint::decode(&extra).is_err());
    let text = String::from_utf8(raw).unwrap();
    for bad in [
        text.replace("schema_version\":1", "schema_version\":2"),
        text.replace("{", "{\"extra\":true,"),
        text.replace("{", "{\"request_pin\":\"bad\","),
        text.replace(&"cd".repeat(32), &"CD".repeat(32)),
    ] {
        assert!(Checkpoint::decode(bad.as_bytes()).is_err());
    }
    assert!(Checkpoint::new("bad", &repository::Pin::parse(&"cd".repeat(32)).unwrap()).is_err());
}
/// Protected-location reread is independent of package bytes; no package exists.
#[test]
fn checkpoint_private_create_reopen_binding_refusal_then_recovery() {
    valid();
    let d = fixture();
    c::create(open(&d), "anchor.json", &record(), &Context::new()).unwrap();
    let before = fs::read(d.path().join("anchor.json")).unwrap();
    let e = c::load(open(&d), "anchor.json", &"ef".repeat(32), &Context::new()).unwrap_err();
    assert!(matches!(e.cause, c::Error::RequestBinding));
    assert!(e.decoded);
    assert_eq!(e.validation, ReadValidation::BytesRead);
    assert_eq!(e.durability, ReadDurability::DirectoryAcknowledged);
    assert!(c::load(open(&d), "anchor.json", &"ab".repeat(32), &Context::new()).is_ok());
    assert_eq!(fs::read(d.path().join("anchor.json")).unwrap(), before);
    assert!(!d.path().join("initial-provisioning-v1.bin").exists());
}
/// Repeated create refuses even equal bytes, never overwrite/adopt/reset an anchor.
#[test]
fn checkpoint_no_replace_keeps_existing_anchor_exact() {
    valid();
    let d = fixture();
    c::create(open(&d), "anchor.json", &record(), &Context::new()).unwrap();
    let before = fs::read(d.path().join("anchor.json")).unwrap();
    let e = c::create(open(&d), "anchor.json", &record(), &Context::new()).unwrap_err();
    assert_eq!(e.publication, Publication::NotAttempted);
    assert_eq!(fs::read(d.path().join("anchor.json")).unwrap(), before);
    assert!(c::load(open(&d), "anchor.json", &"ab".repeat(32), &Context::new()).is_ok());
}
/// Visible publication and independent unlock/close failures survive; pinned data
/// are not regenerated, explicit trusted-locator recovery acknowledges same bytes.
#[test]
fn checkpoint_visible_and_cleanup_failure_retains_effect_then_reopens() {
    valid();
    let d = fixture();
    let docs = open(&d);
    docs.fixture_faults(&[
        FixtureFault::DirectorySync,
        FixtureFault::Unlock,
        FixtureFault::StoreClose,
    ]);
    let e = c::create(docs, "anchor.json", &record(), &Context::new()).unwrap_err();
    assert_eq!(e.publication, Publication::Visible);
    assert_eq!(
        fs::read(d.path().join("anchor.json")).unwrap(),
        record().encode().unwrap()
    );
    assert!(c::load(open(&d), "anchor.json", &"ab".repeat(32), &Context::new()).is_ok());
}
/// Read failures retain every known ack without PinMatched; no record returned.
#[test]
fn checkpoint_read_ack_failures_withhold_record_then_recover() {
    for (fault, ack) in [
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
        let d = fixture();
        c::create(open(&d), "anchor.json", &record(), &Context::new()).unwrap();
        let docs = open(&d);
        docs.fixture_faults(&[fault]);
        let e = c::load(docs, "anchor.json", &"ab".repeat(32), &Context::new()).unwrap_err();
        assert_eq!(e.validation, ReadValidation::BytesRead);
        assert_eq!(e.durability, ack);
        assert!(!e.decoded);
        assert!(c::load(open(&d), "anchor.json", &"ab".repeat(32), &Context::new()).is_ok());
    }
}
/// Late release/close after decode withholds the value and retains both causes.
#[test]
fn checkpoint_late_close_retains_decode_and_independent_causes() {
    valid();
    let d = fixture();
    c::create(open(&d), "anchor.json", &record(), &Context::new()).unwrap();
    let docs = open(&d);
    docs.fixture_faults(&[FixtureFault::Unlock, FixtureFault::StoreClose]);
    let e = c::load(docs, "anchor.json", &"ab".repeat(32), &Context::new()).unwrap_err();
    assert!(e.decoded);
    assert_eq!(e.durability, ReadDurability::DirectoryAcknowledged);
    assert!(matches!(e.cause, c::Error::Joined { .. }));
    assert!(c::load(open(&d), "anchor.json", &"ab".repeat(32), &Context::new()).is_ok());
}
/// Trusted locator never means accepted schema: invalid canonical data stay intact.
#[test]
fn checkpoint_invalid_record_refuses_after_ack_without_adoption() {
    valid();
    let d = fixture();
    let docs = open(&d);
    let ctx = Context::new();
    let r = docs.reserve(&ctx).unwrap();
    r.create("anchor.json", b"{}").unwrap();
    r.finish().unwrap();
    docs.close().unwrap();
    let e = c::load(open(&d), "anchor.json", &"ab".repeat(32), &ctx).unwrap_err();
    assert!(!e.decoded);
    assert_eq!(e.durability, ReadDurability::DirectoryAcknowledged);
    assert_eq!(fs::read(d.path().join("anchor.json")).unwrap(), b"{}");
    assert!(c::create(open(&d), "anchor.json", &record(), &ctx).is_err());
}
/// Wide-identity traversal retains custom IO and independent cleanup, without
/// formatting private pins or raw underlying IO text. No IO is performed here.
#[test]
fn checkpoint_custom_io_and_cleanup_matrix_keeps_typed_causes() {
    let cause = c::Error::Joined {
        cause: Box::new(c::Error::Store(config::Error::Io {
            class: config::IoClass::Storage,
            cause: std::io::Error::other(config::Error::Pins),
        })),
        cleanup: config::Error::Closed,
    };
    let mut pending: Vec<&(dyn StdError + 'static)> = vec![&cause];
    let mut seen: Vec<*const dyn StdError> = vec![];
    let mut pins = false;
    let mut closed = false;
    while let Some(e) = pending.pop() {
        let id = e as *const dyn StdError;
        if seen.iter().any(|old| std::ptr::eq(*old, id)) {
            continue;
        }
        seen.push(id);
        pins |= matches!(e.downcast_ref::<config::Error>(), Some(config::Error::Pins));
        closed |= matches!(
            e.downcast_ref::<config::Error>(),
            Some(config::Error::Closed)
        );
        pending.extend(c::error_links(e).1);
        if let Some(s) = e.source() {
            pending.push(s);
        }
    }
    assert!(pins && closed);
    assert!(!format!("{cause}").contains(&"ab".repeat(32)));
}

/// Mandatory pure shape checks precede reservation/read/write, yet real owner
/// close still happens and must not erase the original refusal or create a file.
#[test]
fn checkpoint_invalid_binding_and_record_keep_independent_close() {
    valid();
    let d = fixture();
    let docs = open(&d);
    docs.fixture_faults(&[FixtureFault::StoreClose]);
    let e = c::load(docs, "anchor.json", "bad", &Context::new()).unwrap_err();
    assert_eq!(e.validation, ReadValidation::NotAttempted);
    assert_eq!(e.durability, ReadDurability::NotAcknowledged);
    assert!(!e.decoded);
    assert!(matches!(e.cause, c::Error::Joined { .. }));
    assert!(!d.path().join("anchor.json").exists());
    let raw = String::from_utf8(record().encode().unwrap())
        .unwrap()
        .replace("schema_version\":1", "schema_version\":2");
    let invalid: Checkpoint = serde_json::from_str(&raw).unwrap();
    let docs = open(&d);
    docs.fixture_faults(&[FixtureFault::StoreClose]);
    let e = c::create(docs, "anchor.json", &invalid, &Context::new()).unwrap_err();
    assert_eq!(e.publication, Publication::NotAttempted);
    assert!(matches!(e.cause, c::Error::Joined { .. }));
    assert!(!d.path().join("anchor.json").exists());
    c::create(open(&d), "anchor.json", &record(), &Context::new()).unwrap();
    assert!(c::load(open(&d), "anchor.json", &"ab".repeat(32), &Context::new()).is_ok());
}
