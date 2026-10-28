//! Actual isolated storage failures and pure typed wrapper regressions.
use super::*;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_enrollment_authority::provisioning::{self as material, repository as package};
use swarm_enrollment_config::{
    self as config,
    store::{
        self,
        documents::{Documents, Publication},
    },
};
struct Case {
    _root: tempfile::TempDir,
    operator: PathBuf,
    pin: package::Pin,
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
            "../../../../../tests/fixtures/paea_provisioning_request.json"
        ))
        .unwrap();
        request.controller_uid = if wrong_role { other } else { uid };
        request.service_uid = if wrong_role { uid } else { other };
        request.root_directory = root.path().join("absent-service").to_str().unwrap().into();
        let prepared = material::prepare(&request).unwrap();
        let seal = package::seal(prepared).unwrap();
        let pin = seal.pin().clone();
        let store = package::PackageStore::new(Documents::open(&operator, uid).unwrap());
        assert_eq!(
            store.create(&store::Context::new(), &seal).unwrap(),
            Publication::Durable
        );
        store.close().unwrap();
        Self {
            _root: root,
            operator,
            pin,
        }
    }
}
/// The common IO-custom matrix must retain both package cleanup and earlier
/// target failure, without private data in Display or safe diagnostic labels.
#[test]
fn operator_custom_io_output_wrapper_retains_package_and_target_branches() {
    let nested = package::Error::Joined {
        cause: Box::new(package::Error::Store(config::Error::State)),
        cleanup: config::Error::Pins,
    };
    let c = Case::new(false);
    let report = core::run(Action::Check, &c.operator, c.pin.as_str(), None).unwrap();
    let earlier = core::Error::Target {
        report: report.clone(),
        cause: core::TargetCause::Joined {
            cause: Box::new(core::TargetCause::Storage(config::Error::Pending)),
            cleanup: config::Error::Closed,
        },
    };
    let e = Error::Output {
        report,
        phase: OutputPhase::Flush,
        cause: io::Error::other(nested),
        failure: Some(Box::new(earlier)),
    };
    let cli = CliError::OperatorPackage {
        cause: Box::new(e),
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
    assert!(causes.iter().any(|e| matches!(
        e.downcast_ref::<config::Error>(),
        Some(config::Error::Pending)
    )));
    assert!(causes.iter().any(|e| matches!(
        e.downcast_ref::<config::Error>(),
        Some(config::Error::Closed)
    )));
    let labels = crate::safe_causes(&cli);
    assert!(labels
        .iter()
        .any(|s| s == "operator-output-publication:not_attempted"));
    assert!(labels
        .iter()
        .any(|s| s == "operator-transport-phase:written-flush-failed"));
    assert!(!format!("{cli}").contains(&"ab".repeat(32)));
}
