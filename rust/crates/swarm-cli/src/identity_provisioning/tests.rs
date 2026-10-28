//! Private read/cleanup and typed diagnostic regressions; disposable roots only.
use super::*;
use std::{fs, os::unix::fs::PermissionsExt};
/// Canonical request fixture without any entropy, service or journal creation.
fn input() -> (tempfile::TempDir, std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut r: material::Request = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    r.root_directory = d.path().join("absent-target").to_str().unwrap().into();
    let uid = rustix::process::geteuid().as_raw();
    r.service_uid = uid;
    r.controller_uid = if uid == 0 { 1 } else { !uid };
    let raw = material::encode_request(&r).unwrap();
    let p = d.path().join("request.json");
    fs::write(&p, &raw).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
    (d, p, format!("{:x}", Sha256::digest(raw)))
}
/// Valid data before actual release+close faults; known decode and independent
/// causes survive, no Request returned, reopening can recover.
#[test]
#[cfg(feature = "test-fixtures")]
fn provisioning_input_release_and_close_withhold_request_keep_all_causes_then_recover() {
    let (d, path, pin) = input();
    assert!(load(&path, &pin).is_ok());
    let before = fs::read(&path).unwrap();
    let docs = Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    docs.fixture_faults(&[
        config::store::FixtureFault::Unlock,
        config::store::FixtureFault::StoreClose,
    ]);
    let e = load_owned(docs, "request.json", &pin).unwrap_err();
    assert!(e.progress.decoded);
    assert_eq!(e.progress.validation, ReadValidation::PinMatched);
    assert_eq!(e.progress.durability, ReadDurability::DirectoryAcknowledged);
    assert!(matches!(e.cause, InputCause::Joined { .. }));
    let cli = CliError::ProvisioningCheck {
        cause: Box::new(Error::Input(e)),
        english: true,
    };
    let labels = crate::safe_causes(&cli);
    assert!(labels
        .iter()
        .any(|s| s == "provisioning-request-validated:true"));
    assert!(
        cli.typed_causes()
            .iter()
            .filter(|e| e.downcast_ref::<config::Error>().is_some())
            .count()
            >= 2
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(load(&path, &pin).is_ok());
    assert!(!d.path().join("absent-target").exists());
}
/// Wrong pin read+release+close failure retains original refusal and cleanup;
/// no validation/grant success is inferred from any acknowledged storage phase.
#[test]
#[cfg(feature = "test-fixtures")]
fn provisioning_pin_refusal_keeps_independent_release_close_causes() {
    let (d, path, pin) = input();
    assert!(load(&path, &pin).is_ok());
    let docs = Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    docs.fixture_faults(&[
        config::store::FixtureFault::Unlock,
        config::store::FixtureFault::StoreClose,
    ]);
    let e = load_owned(docs, "request.json", &"00".repeat(32)).unwrap_err();
    assert!(!e.progress.decoded);
    let cli = CliError::ProvisioningCheck {
        cause: Box::new(Error::Input(e)),
        english: false,
    };
    assert!(cli
        .typed_causes()
        .iter()
        .any(|e| matches!(e.downcast_ref::<config::Error>(), Some(config::Error::Pins))));
    assert!(
        cli.typed_causes()
            .iter()
            .filter(|e| e.downcast_ref::<config::Error>().is_some())
            .count()
            >= 3
    );
    assert!(load(&path, &pin).is_ok());
}
/// IO custom wrapper must not hide independent config cleanup; safe labels omit
/// private error text and retain the actual successful validation/output phase.
#[test]
fn provisioning_custom_io_wrapper_keeps_complete_cause_matrix_without_private_text() {
    let (_d, path, pin) = input();
    let request = load(&path, &pin).unwrap();
    let cause = InputCause::Joined {
        cause: Box::new(InputCause::Storage(config::Error::State)),
        cleanup: config::Error::Pins,
    };
    let error = Error::Output {
        report: Report::checked(&request, &pin),
        phase: OutputPhase::Flush,
        cause: io::Error::other(cause),
    };
    let cli = CliError::ProvisioningCheck {
        cause: Box::new(error),
        english: true,
    };
    let causes = cli.typed_causes();
    assert!(causes.iter().any(|e| matches!(
        e.downcast_ref::<config::Error>(),
        Some(config::Error::State)
    )));
    assert!(causes
        .iter()
        .any(|e| matches!(e.downcast_ref::<config::Error>(), Some(config::Error::Pins))));
    let labels = crate::safe_causes(&cli);
    assert!(labels
        .iter()
        .any(|s| s == "provisioning-output-phase:written-flush-failed"));
    assert!(!format!("{cli}").contains(path.to_str().unwrap()));
}
