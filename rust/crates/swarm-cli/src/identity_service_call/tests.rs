//! Native adapter unit contracts on isolated real roots; no OS/peer acceptance.
use super::*;
use std::{fs, io::Cursor, os::unix::fs::PermissionsExt};
use swarm_enrollment_authority::provisioning::{
    self as p, service_artifact, service_installation as install,
};
use zeroize::Zeroizing;
const PASSWORD: &[u8] = b"synthetic\0call-password\n";
/// Same parser/closed option producer, no IO or fabricated authenticated context.
fn args(pin: &str) -> Arguments {
    crate::parse(
        &[
            "identity",
            "service-call",
            pin,
            "/nonexistent-fixture/seed",
            "--input",
            "/nonexistent-fixture/bootstrap",
            "--json",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>(),
        "en",
    )
    .unwrap()
}
/// Actual private publisher, no live state. Grants are explicit fixture inputs.
fn fixture() -> (tempfile::TempDir, p::Prepared, Arguments) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut r: p::Request = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    r.root_directory = d.path().join("service").to_str().unwrap().into();
    r.service_uid = rustix::process::geteuid().as_raw();
    r.controller_uid = if r.service_uid == 0 {
        1
    } else {
        !r.service_uid
    };
    r.grant.permissions = ["read", "preview", "create", "apply"]
        .into_iter()
        .map(|op| swarm_identity_contract::Permission {
            operation: "effect.execute".into(),
            resource: format!("enrollment-config/{}/{op}", r.installation_digest),
        })
        .collect();
    let p = p::prepare(&r).unwrap();
    let artifact = service_artifact::from_prepared(&p).unwrap();
    let i = install::install(
        &install::Context::new(),
        &artifact,
        &artifact.pin().unwrap(),
        install::Mode::Create,
    )
    .unwrap();
    let mut a = args(&i.bootstrap_pin);
    a.input = i.bootstrap_path;
    a.pos[3] = i.personal_seed_path;
    (d, p, a)
}
/// Guarded exact packet for one operation. Only synthetic credentials are used.
fn packet(a: &Arguments, op: request::Operation<'_>) -> Zeroizing<Vec<u8>> {
    request::encode(&a.pos[2], &[19; 32], &op).unwrap()
}
/// Real init/enroll through this adapter, returning no credential digests.
fn enroll(a: &Arguments, p: &p::Prepared) {
    run(
        a,
        &mut Cursor::new(packet(a, request::Operation::Initialize)),
        &mut vec![],
    )
    .unwrap();
    let mut proof = Zeroizing::new(Vec::new());
    p.write_installation_proof(&mut *proof).unwrap();
    run(
        a,
        &mut Cursor::new(packet(
            a,
            request::Operation::Enroll {
                proof: proof.as_slice().try_into().unwrap(),
                password: PASSWORD,
            },
        )),
        &mut vec![],
    )
    .unwrap();
}
/// Existing canonical configuration fixture, independent of controller identity.
fn change() -> swarm_enrollment_config::Change {
    swarm_enrollment_config::Change {
        schema_version: 1,
        event_id: "service-call-unit".into(),
        expected_head: Default::default(),
        values: swarm_enrollment_config::Values {
            mode: "local".into(),
            socket_path: "/run/fixture/call.sock".into(),
        },
        reason: "explicit call fixture".into(),
    }
}
/// Pure shape before every malformed form; shared parser refuses forbidden
/// default-valued flags and duplicates rather than letting defaults hide them.
#[test]
fn service_call_closed_shape_and_options_refuse_before_io() {
    let a = args(&"ab".repeat(32));
    validate(&a).unwrap();
    for change in [
        |a: &mut Arguments| a.pos[2] = "bad".into(),
        |a: &mut Arguments| a.input = "-".into(),
        |a: &mut Arguments| a.input = "relative".into(),
        |a: &mut Arguments| a.pos[3] = "relative".into(),
        |a: &mut Arguments| a.pos[1] = "other".into(),
    ] {
        let mut a = args(&"ab".repeat(32));
        change(&mut a);
        assert!(validate(&a).is_err());
    }
    for flags in [
        vec!["--root", "."],
        vec!["--phase", "delivery"],
        vec!["--json", "--json"],
    ] {
        let mut words = vec![
            "identity",
            "service-call",
            "pin",
            "/seed",
            "--input",
            "/bootstrap",
        ];
        words.extend(flags);
        assert!(crate::parse(
            &words.into_iter().map(str::to_owned).collect::<Vec<_>>(),
            "en"
        )
        .is_err());
    }
}
/// Valid frame targeting a different independent pin refuses before nonexistent
/// startup is touched; empty input is a request refusal, never empty success.
#[test]
fn service_call_wrong_target_and_empty_packet_refuse_before_startup() {
    let a = args(&"ab".repeat(32));
    validate(&a).unwrap();
    let raw = request::encode(&"00".repeat(32), &[1; 32], &request::Operation::Initialize).unwrap();
    assert!(matches!(
        run(&a, &mut Cursor::new(raw), &mut vec![]),
        Err(Error::Target)
    ));
    assert!(matches!(
        run(&a, &mut Cursor::new(vec![]), &mut vec![]),
        Err(Error::Request(_))
    ));
}
/// Real adapter lifecycle then configuration effect/replay/recover; stdout
/// completed is operation-only and failures still have nonzero typed Result.
#[test]
fn service_call_real_lifecycle_control_and_recovery_preserve_exact_state() {
    let (_d, p, a) = fixture();
    enroll(&a, &p);
    for _ in 0..2 {
        let mut out = vec![];
        run(
            &a,
            &mut Cursor::new(packet(
                &a,
                request::Operation::Control {
                    password: PASSWORD,
                    request: control::Request::Create(change()),
                },
            )),
            &mut out,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(
            v["view"]["snapshot"]["history"].as_array().unwrap().len(),
            1
        );
        assert_eq!(v["completed"], true);
    }
    let mut out = vec![];
    run(
        &a,
        &mut Cursor::new(packet(&a, request::Operation::Recover)),
        &mut out,
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["sequence"], 3);
    assert_eq!(v["journal"], "current_verified");
    let error = run(
        &a,
        &mut Cursor::new(packet(
            &a,
            request::Operation::Control {
                password: b"wrong",
                request: control::Request::Read,
            },
        )),
        &mut vec![],
    )
    .unwrap_err();
    assert_eq!(error.code(), "service-call-operation");
    assert!(!error.known().unwrap().completed());
}
/// Synthetic output refusal only; it never repeats a storage operation.
struct Broken {
    flush: bool,
}
impl Write for Broken {
    /// Allow full write for the flush case, otherwise retain the original cause.
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if self.flush {
            Ok(raw.len())
        } else {
            Err(io::Error::other("PRIVATE_CALL_WRITE"))
        }
    }
    /// Independent refusal after write_all success, without private text leakage.
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("PRIVATE_CALL_FLUSH"))
    }
}
/// Explicit synthetic close branches supplied after real close; this verifies
/// adapter consumption, not an actual kernel-close injection.
fn close_failure() -> CloseError {
    CloseError {
        personal: Some(
            swarm_enrollment_authority::personal::coordinator::Error::Storage(
                swarm_enrollment_config::Error::Path,
            ),
        ),
        control: Some(control::Error::Close {
            authority: None,
            storage: Some(swarm_enrollment_config::Error::Busy),
        }),
    }
}
/// Actual Reply owner remains inspectable after known success plus supplied close
/// and output errors. No safe summary may print a password, target or IOtext.
#[test]
fn service_call_output_failure_keeps_known_reply_and_independent_close() {
    for flush in [false, true] {
        let (_d, p, a) = fixture();
        enroll(&a, &p);
        let raw = packet(&a, request::Operation::Recover);
        let mut service = core::Service::open(
            &swarm_enrollment_state::Context::new(),
            &a.input,
            &a.pos[2],
            Path::new(&a.pos[3]),
        )
        .unwrap();
        let reply = service
            .dispatch(&Context::new(), request::decode(&raw).unwrap())
            .unwrap();
        service.close().unwrap();
        let error = finish(reply, Some(close_failure()), &mut Broken { flush }).unwrap_err();
        assert!(error.known().unwrap().completed());
        assert!(error.observation().contains("sequence=Some(3)"));
        assert!(error.observation().contains("close=failed"));
        let labels = crate::safe_causes(&error);
        for label in ["enrollment-config:path", "enrollment-config:busy"] {
            assert!(labels.iter().any(|s| s == label), "missing {label}")
        }
        assert!(!format!("{error:?}").contains("PRIVATE_CALL"));
    }
}
/// Synthetic nested failure over real sources exercises the existing customIO
/// wrapper traversal. Output, outer authority, inner storage and both closes
/// must all reach safe diagnostic labels, never only source().
#[test]
fn service_call_custom_io_keeps_after_use_storage_and_both_close_branches() {
    let (_d, p, a) = fixture();
    enroll(&a, &p);
    let raw = packet(&a, request::Operation::Recover);
    let mut service = core::Service::open(
        &swarm_enrollment_state::Context::new(),
        &a.input,
        &a.pos[2],
        Path::new(&a.pos[3]),
    )
    .unwrap();
    let mut reply = service
        .dispatch(&Context::new(), request::decode(&raw).unwrap())
        .unwrap();
    service.close().unwrap();
    reply.outcome = OutcomeData::Control(Err(UseError::AfterUse {
        cause: swarm_enrollment_state::Error::Canceled,
        outcome: Err(control::Error::AfterStorage {
            cause: swarm_enrollment_state::Error::Closed,
            storage: Box::new(Err(swarm_enrollment_config::Error::Joined {
                cause: Box::new(swarm_enrollment_config::Error::State),
                cleanup: Box::new(swarm_enrollment_config::Error::Pending),
            })),
            effect_unknown: true,
        }),
    }));
    let error = finish(reply, Some(close_failure()), &mut Broken { flush: true }).unwrap_err();
    let cli = CliError::Output(io::Error::other(CliError::ServiceCall {
        cause: Box::new(error),
        english: true,
    }));
    let labels = crate::safe_causes(&cli);
    for label in [
        "enrollment-state:canceled",
        "enrollment-state:closed",
        "enrollment-config:state",
        "enrollment-config:pending",
        "enrollment-config:path",
        "enrollment-config:busy",
    ] {
        assert!(
            labels.iter().any(|s| s == label),
            "missing {label}: {labels:?}"
        );
    }
    let mut stderr = vec![];
    let result = crate::report(cli, true, true, &mut stderr);
    assert_ne!(result.code, 0);
    assert!(!String::from_utf8(stderr).unwrap().contains("PRIVATE_CALL"));
}
/// A supplied close refusal is nonzero even when JSON write/flush and the actual
/// operation succeeded; stdout retains that known operation without rollback.
#[test]
fn service_call_close_failure_is_distinct_from_completed_operation_and_output() {
    let (_d, p, a) = fixture();
    enroll(&a, &p);
    let raw = packet(&a, request::Operation::Recover);
    let mut service = core::Service::open(
        &swarm_enrollment_state::Context::new(),
        &a.input,
        &a.pos[2],
        Path::new(&a.pos[3]),
    )
    .unwrap();
    let reply = service
        .dispatch(&Context::new(), request::decode(&raw).unwrap())
        .unwrap();
    service.close().unwrap();
    let mut out = vec![];
    let error = finish(reply, Some(close_failure()), &mut out).unwrap_err();
    assert_eq!(error.code(), "service-call-close");
    assert!(error.known().unwrap().completed());
    assert!(error.observation().contains("close=failed"));
    let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(value["completed"], true);
    assert_eq!(value["sequence"], 3);
}
