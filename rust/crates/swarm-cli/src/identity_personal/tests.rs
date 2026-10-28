//! Pure command/observation/diagnostic witnesses. No real enrollment, syscall,
//! RootStore or human identity is created by these unit fixtures.
use super::*;
use std::io;
/// Same closed transport as the native command; nonexistent paths are not opened.
fn arguments() -> Arguments {
    crate::parse(
        &[
            "identity",
            "personal-enroll",
            "pin",
            "/private-fixture/key.seed",
            "--input",
            "/private-fixture/bootstrap.json",
            "--json",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>(),
        "en",
    )
    .unwrap()
}
#[test]
fn personal_cli_shape_validation_is_pure_and_rejects_malformed_transport() {
    assert!(matches!(
        validated_action(&arguments()).unwrap(),
        Action::Enroll
    ));
    let mutations: Vec<fn(&mut Arguments)> = vec![
        |a| {
            a.pos.pop();
        },
        |a| a.input.clear(),
        |a| a.input = "-".into(),
        |a| a.root = "/other".into(),
        |a| a.server = true,
        |a| a.output = "x".into(),
        |a| a.task = "x".into(),
        |a| a.phase = "entry".into(),
        |a| a.pos[1] = "unrecognized".into(),
    ];
    for change in mutations {
        let mut a = arguments();
        change(&mut a);
        assert!(matches!(validated_action(&a), Err(CliError::Usage)));
    }
}
struct Broken {
    flush: bool,
}
impl Write for Broken {
    /// Simulate either write or flush failure without an account or side effect.
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.flush {
            Ok(b.len())
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "PRIVATE_WRITE_CAUSE",
            ))
        }
    }
    /// Preserve an independent late transport refusal after a successful write.
    fn flush(&mut self) -> io::Result<()> {
        if self.flush {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "PRIVATE_FLUSH_CAUSE",
            ))
        } else {
            Ok(())
        }
    }
}
#[test]
fn personal_cli_output_failure_retains_supplied_known_report_and_safe_phase() {
    for flush in [false, true] {
        let mut report = Report::completed(Action::Enroll, Some(3));
        report.close = "closed";
        let e = emit(report, None, &mut Broken { flush }).unwrap_err();
        let r = serde_json::to_value(e.report().unwrap()).unwrap();
        assert_eq!(r["operation_result"], "completed");
        assert_eq!(r["sequence"], 3);
        assert_eq!(r["checkpoint_publication"], "durable");
        assert_eq!(r["human_presence_verified"], false);
        let root = CliError::PersonalLifecycle {
            cause: Box::new(e),
            english: true,
        };
        let labels = crate::safe_causes(&root);
        assert!(labels.iter().any(|v| v == "personal-reported-sequence:3"));
        assert!(labels.iter().any(|v| v
            == if flush {
                "personal-output-phase:flush"
            } else {
                "personal-output-phase:write"
            }));
        assert!(!format!("{root:?} {root}").contains("PRIVATE_"));
    }
}
#[test]
fn personal_cli_error_links_keep_primary_secondary_and_custom_io_redacted() {
    use protected_signer::{Error as KeyError, Stage};
    let key = KeyError::Joined {
        cause: Box::new(KeyError::Io {
            stage: Stage::Read,
            cause: io::Error::new(io::ErrorKind::PermissionDenied, "PRIVATE_KEY_IO"),
        }),
        cleanup: Box::new(KeyError::Io {
            stage: Stage::DirectoryClose,
            cause: io::Error::from_raw_os_error(5),
        }),
    };
    let startup = startup::Error::Cleanup {
        cause: Box::new(startup::Error::Signing(key)),
        source: Some(personal::Error::Secret),
        coordinator: Some(coordinator::Error::Joined {
            cause: Box::new(coordinator::Error::Checkpoint),
            cleanup: Box::new(coordinator::Error::Pending),
        }),
    };
    let wrapped = CliError::Output(io::Error::other(CliError::PersonalLifecycle {
        cause: Box::new(Error::Startup(startup)),
        english: true,
    }));
    let labels = crate::safe_causes(&wrapped);
    for label in [
        "protected-signer:read",
        "protected-signer:close",
        "personal-source:installation-proof",
        "personal-coordinator:personal-checkpoint",
        "personal-coordinator:personal-pending",
    ] {
        assert!(
            labels.iter().any(|v| v == label),
            "missing {label}: {labels:?}"
        );
    }
    assert!(!format!("{wrapped:?} {wrapped}").contains("PRIVATE_KEY_IO"));
    let mut report = Report::completed(Action::Recover, Some(3));
    report.close = "failed";
    let root = CliError::PersonalLifecycle {
        cause: Box::new(Error::Close {
            cause: CloseError {
                personal: Some(coordinator::Error::Internal),
                control: Some(swarm_enrollment_authority::control::Error::Busy),
            },
            report,
        }),
        english: false,
    };
    let labels = crate::safe_causes(&root);
    assert!(labels
        .iter()
        .any(|v| v == "personal-coordinator:personal-coordinator-state"));
    assert!(labels.iter().any(|v| v == "enrollment-control:busy"));
    assert!(labels
        .iter()
        .any(|v| v == "personal-reported-journal:current_verified"));
}
