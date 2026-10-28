//! Closed grammar, private input and late-output witnesses; isolated roots only.
use super::*;
use std::{
    fs,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};
/// Actual closed parser input; all paths are synthetic and unopened in shape tests.
fn arguments() -> Arguments {
    crate::parse(
        &[
            "identity",
            "service-install",
            "00",
            "--input",
            "/private-fixture/service-v1.bin",
            "--json",
        ]
        .map(str::to_owned),
        "en",
    )
    .unwrap()
}
/// Supply known metadata only, no account/FS mutation and no inferred authority.
fn completed() -> Report {
    let mut p = core::Progress::default();
    p.stage = Publication::Durable;
    p.intent = Publication::Durable;
    p.business = core::JournalProgress::Acknowledged;
    p.personal = core::JournalProgress::Acknowledged;
    p.published = Publication::Durable;
    Report::completed(
        Action::Install,
        core::Installed {
            bootstrap_path: "/PRIVATE_LOCATION/bootstrap.json".into(),
            bootstrap_pin: "ab".repeat(32),
            personal_seed_path: "/PRIVATE_LOCATION/seed".into(),
            progress: p,
        },
    )
}
/// Deterministic transport refusal; no arbitrary IO error text may enter stderr.
struct Broken {
    flush: bool,
}
impl Write for Broken {
    /// Accept the supplied byte count only for a flush-failure fixture; otherwise
    /// return a synthetic write error without performing IO or publishing bytes.
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.flush {
            Ok(b.len())
        } else {
            Err(io::Error::other("PRIVATE_OUTPUT_CAUSE"))
        }
    }
    /// Fail after a successful synthetic write when requested; no real sink or
    /// secret is used. The adapter must preserve its earlier known report.
    fn flush(&mut self) -> io::Result<()> {
        if self.flush {
            Err(io::Error::other("PRIVATE_FLUSH_CAUSE"))
        } else {
            Ok(())
        }
    }
}
/// Actual random service-only material file; no root/journal/account created.
fn input() -> (tempfile::TempDir, std::path::PathBuf, String) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut request: swarm_enrollment_authority::provisioning::Request = serde_json::from_str(
        include_str!("../../../../../tests/fixtures/paea_provisioning_request.json"),
    )
    .unwrap();
    request.root_directory = d.path().join("service").to_str().unwrap().into();
    request.service_uid = rustix::process::geteuid().as_raw();
    request.controller_uid = if request.service_uid == 0 {
        1
    } else {
        !request.service_uid
    };
    let prepared = swarm_enrollment_authority::provisioning::prepare(&request).unwrap();
    let artifact = service_artifact::from_prepared(&prepared).unwrap();
    let raw = artifact.encode_private().unwrap();
    let path = d.path().join("service-v1.bin");
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    f.write_all(&raw).unwrap();
    f.sync_all().unwrap();
    nix::unistd::close(f).unwrap();
    (d, path, artifact.pin().unwrap())
}
#[test]
/// Pure command shape checks reject every unsupported field before file access.
fn service_cli_closed_shape_rejects_before_input_access() {
    assert!(matches!(action(&arguments()).unwrap(), Action::Install));
    let changes: Vec<fn(&mut Arguments)> = vec![
        |a| {
            a.pos.pop();
        },
        |a| a.pos.push("extra".into()),
        |a| a.pos[1] = "unknown".into(),
        |a| a.input.clear(),
        |a| a.input = "-".into(),
        |a| a.input = "relative.bin".into(),
        |a| a.root = "other".into(),
        |a| a.server = true,
        |a| a.output = "x".into(),
        |a| a.phase = "entry".into(),
        |a| a.task = "x".into(),
    ];
    for change in changes {
        let mut a = arguments();
        change(&mut a);
        assert!(matches!(action(&a), Err(CliError::Usage)));
    }
}
#[test]
/// Late transport failures preserve supplied durable metadata and safe diagnostics.
fn service_cli_output_write_flush_retains_known_durable_report() {
    for flush in [false, true] {
        let r = completed();
        assert!(!format!("{r:?}").contains("PRIVATE_LOCATION"));
        let e = emit(r, None, &mut Broken { flush }).unwrap_err();
        assert_eq!(e.report().unwrap().publication, "durable");
        assert!(!format!("{e:?} {e}").contains("PRIVATE_"));
        let (labels, _) = error_links(&e);
        assert!(labels
            .iter()
            .any(|s| s == "service-reported-publication:durable"));
        assert!(labels.iter().any(|s| s
            == if flush {
                "service-output-phase:flush"
            } else {
                "service-output-phase:write"
            }));
    }
}
#[test]
/// A second output error keeps both original refusal and independent cleanup cause.
fn service_cli_output_failure_retains_original_operation_and_cleanup_branches() {
    let failure = core::Failure {
        progress: core::Progress::default(),
        cause: core::Error::Joined {
            cause: Box::new(core::Error::State(swarm_enrollment_state::Error::Head)),
            cleanup: Box::new(core::Error::Io {
                phase: core::IoPhase::Close,
                cause: io::Error::other("PRIVATE_CLOSE_CAUSE"),
            }),
        },
    };
    let report = Report::failed(Action::Install, &failure.progress);
    let e = emit(
        report.clone(),
        Some(Error::Operation {
            cause: Box::new(failure),
            report,
        }),
        &mut Broken { flush: false },
    )
    .unwrap_err();
    let cli = CliError::ServiceInstallation {
        cause: Box::new(e),
        english: true,
    };
    let typed = cli.typed_causes();
    assert!(typed
        .iter()
        .any(|e| e.downcast_ref::<swarm_enrollment_state::Error>().is_some()));
    assert!(typed.iter().any(|e| matches!(
        e.downcast_ref::<core::Error>(),
        Some(core::Error::Io {
            phase: core::IoPhase::Close,
            ..
        })
    )));
    assert!(!format!("{cli}").contains("PRIVATE_"));
}
#[test]
/// Real private input rejects wrong pin/mode without target effects, then recovers.
fn service_cli_private_input_pin_mode_and_exact_recovery() {
    let (d, path, pin) = input();
    let before = fs::read(&path).unwrap();
    let e = load(&path, &"00".repeat(32)).unwrap_err();
    assert!(!e.progress.decoded);
    assert_eq!(e.progress.durability, ReadDurability::NotAcknowledged);
    assert!(!d.path().join("service").exists());
    assert_eq!(before, fs::read(&path).unwrap());
    assert!(load(&path, &pin).is_ok());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(load(&path, &pin).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(load(&path, &pin).is_ok());
}
#[test]
#[cfg(feature = "test-fixtures")]
/// Decoded keys remain withheld after actual release/close faults; explicit reopen recovers.
fn service_cli_input_release_and_close_keep_progress_and_discard_decoded_key() {
    let (d, path, pin) = input();
    let docs = Documents::open(d.path(), rustix::process::geteuid().as_raw()).unwrap();
    docs.fixture_faults(&[
        config::store::FixtureFault::Unlock,
        config::store::FixtureFault::StoreClose,
    ]);
    let e = load_owned(docs, path.file_name().unwrap().to_str().unwrap(), &pin).unwrap_err();
    assert!(e.progress.decoded);
    assert_eq!(e.progress.validation, ReadValidation::PinMatched);
    assert_eq!(e.progress.durability, ReadDurability::DirectoryAcknowledged);
    assert!(matches!(e.cause, InputCause::Joined { .. }));
    assert!(load(&path, &pin).is_ok());
}
#[test]
/// Custom IO wrappers retain the entire typed primary/cleanup tree without private output.
fn service_cli_custom_io_wrapper_retains_root_cleanup_without_private_text() {
    let inner = core::Error::Joined {
        cause: Box::new(core::Error::State(swarm_enrollment_state::Error::Head)),
        cleanup: Box::new(core::Error::Storage(config::Error::Pins)),
    };
    let e = Error::Output {
        cause: io::Error::other(inner),
        phase: OutputPhase::Flush,
        failure: None,
        report: completed(),
    };
    let cli = CliError::ServiceInstallation {
        cause: Box::new(e),
        english: false,
    };
    let causes = cli.typed_causes();
    assert!(causes
        .iter()
        .any(|e| matches!(e.downcast_ref::<config::Error>(), Some(config::Error::Pins))));
    assert!(causes.iter().any(|e| matches!(
        e.downcast_ref::<swarm_enrollment_state::Error>(),
        Some(swarm_enrollment_state::Error::Head)
    )));
    assert!(!format!("{cli}").contains("PRIVATE_LOCATION"));
}
