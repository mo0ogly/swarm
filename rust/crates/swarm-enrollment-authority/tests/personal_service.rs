//! Guarded request data and real isolated service core, not peer/OS acceptance.
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use swarm_enrollment_authority::{
    authenticated_control::Context,
    control,
    personal_service::{OutcomeData, Service},
    provisioning::{self as p, service_artifact, service_installation as install},
    service_request::{self as wire, Operation},
};
use swarm_enrollment_config::{Change, Head, Values};
use swarm_identity_contract::Permission;
use zeroize::Zeroizing;
const PIN: &str = "abababababababababababababababababababababababababababababababab";
const PASSWORD: &[u8] = b"synthetic\0exact-password\n";
/// Existing canonical public configuration fixture, no secret in JSON.
fn change() -> Change {
    Change {
        schema_version: 1,
        event_id: "service-core-fixture".into(),
        expected_head: Head::default(),
        values: Values {
            mode: "local".into(),
            socket_path: "/run/fixture/paea.sock".into(),
        },
        reason: "explicit service fixture".into(),
    }
}
/// Encode/decode same valid witness; no IO, admission or service constructed.
fn codec_valid() {
    let raw = wire::encode(
        PIN,
        &[13; 32],
        &Operation::Enroll {
            proof: &[42; 32],
            password: PASSWORD,
        },
    )
    .unwrap();
    let r = wire::decode(&raw).unwrap();
    assert_eq!(r.target_pin(), PIN);
    assert_eq!(r.challenge(), &[13; 32]);
    match r.operation() {
        Operation::Enroll { proof, password } => {
            assert_eq!(*proof, &[42; 32]);
            assert_eq!(*password, PASSWORD);
        }
        _ => panic!("wrong op"),
    }
}
/// Exact binary secrets stay borrowed from one guarded buffer, never Debug text.
#[test]
fn personal_service_packet_exact_binary_and_borrowed_secret_redaction() {
    codec_valid();
    let raw = wire::encode(
        PIN,
        &[13; 32],
        &Operation::Enroll {
            proof: &[42; 32],
            password: PASSWORD,
        },
    )
    .unwrap();
    let r = wire::decode(&raw).unwrap();
    let base = raw.as_ptr() as usize;
    match r.operation() {
        Operation::Enroll { proof, password } => {
            assert_eq!(
                proof.as_ptr() as usize,
                base + wire::HEADER.len() + 64 + 32 + 1 + 8
            );
            assert_eq!(
                password.as_ptr() as usize,
                base + wire::HEADER.len() + 64 + 32 + 1 + 8 + 32
            );
        }
        _ => panic!("wrong op"),
    }
    assert_eq!(format!("{r:?}"), "PersonalServiceRequest { <private> }");
    assert_eq!(
        format!("{:?}", r.operation()),
        "PersonalServiceOperation { <private> }"
    );
}
/// Valid witness before each truncation/opcode/length/pin/tail refusal, then recover.
#[test]
fn personal_service_packet_invalid_frames_refuse_without_secret_diagnostics() {
    let good = wire::encode(PIN, &[13; 32], &Operation::Initialize).unwrap();
    let opcode = wire::HEADER.len() + 64 + 32;
    let mut variants: Vec<Zeroizing<Vec<u8>>> = (0..good.len())
        .map(|i| Zeroizing::new(good[..i].to_vec()))
        .collect();
    let mut bad = good.clone();
    bad[0] ^= 1;
    variants.push(bad);
    let mut bad = good.clone();
    bad[wire::HEADER.len()] = b'G';
    variants.push(bad);
    let mut bad = good.clone();
    bad[opcode] = 0;
    variants.push(bad);
    let mut bad = good.clone();
    bad[opcode] = 8;
    variants.push(bad);
    let mut bad = good.clone();
    bad[opcode + 1..opcode + 9].copy_from_slice(&u64::MAX.to_be_bytes());
    variants.push(bad);
    let mut bad = good.clone();
    bad.push(42);
    variants.push(bad);
    let mut bad = good.clone();
    bad[opcode] = 2;
    bad.extend([42; 32]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[opcode] = 4;
    variants.push(bad);
    for bad in variants {
        codec_valid();
        let e = wire::decode(&bad).unwrap_err();
        assert_eq!(e.code(), "service-request-format");
        assert!(!format!("{e:?}").contains("password"));
        codec_valid();
    }
}
/// All existing control operations preserve canonical Change and exact passwords.
#[test]
fn personal_service_packet_all_operations_and_public_change_canonical_refusals() {
    for request in [
        control::Request::Read,
        control::Request::Preview(change()),
        control::Request::Create(change()),
        control::Request::Apply(change()),
    ] {
        let expected = request.operation();
        let raw = wire::encode(
            PIN,
            &[7; 32],
            &Operation::Control {
                password: PASSWORD,
                request: request.clone(),
            },
        )
        .unwrap();
        match wire::decode(&raw).unwrap().operation() {
            Operation::Control { password, request } => {
                assert_eq!(*password, PASSWORD);
                assert_eq!(request.operation(), expected)
            }
            _ => panic!("wrong control"),
        }
    }
    let body = swarm_enrollment_config::encode_change(&change()).unwrap();
    let mut invalid = vec![b"{}".to_vec(), [b" ".as_slice(), body.as_slice()].concat()];
    let text = String::from_utf8(body.clone()).unwrap();
    invalid.push(text.replacen('{', "{\"actor\":\"forged\",", 1).into_bytes());
    invalid.push(text.replacen('{', "{\"schema_version\":1,", 1).into_bytes());
    for body in invalid {
        codec_valid();
        let mut raw = wire::encode(PIN, &[0; 32], &Operation::Initialize).unwrap();
        let op = wire::HEADER.len() + 64 + 32;
        raw[op] = 6;
        raw[op + 1..op + 9].copy_from_slice(&(body.len() as u64).to_be_bytes());
        raw.extend(body);
        raw.extend(PASSWORD);
        assert!(wire::decode(&raw).is_err());
    }
    let mut raw = wire::encode(
        PIN,
        &[0; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    )
    .unwrap();
    raw[wire::HEADER.len() + 64 + 32] = 4;
    assert!(wire::decode(&raw).is_err());
}
/// Required secret/target checks happen before encoding; no implicit credential.
#[test]
fn personal_service_packet_encode_empty_secret_and_invalid_change_refuse() {
    codec_valid();
    for op in [
        Operation::Enroll {
            proof: &[42; 32],
            password: &[],
        },
        Operation::Control {
            password: &[],
            request: control::Request::Read,
        },
    ] {
        assert!(wire::encode(PIN, &[0; 32], &op).is_err());
    }
    assert!(wire::encode("BAD", &[0; 32], &Operation::Recover).is_err());
    let mut c = change();
    c.schema_version = 2;
    assert!(wire::encode(
        PIN,
        &[0; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Apply(c)
        }
    )
    .is_err());
    let raw = wire::encode(PIN, &[0; 32], &Operation::Recover).unwrap();
    assert!(matches!(
        wire::decode(&raw).unwrap().operation(),
        Operation::Recover
    ));
}
/// New real protected service root, current euid only as a disposable fixture label.
fn fixture() -> (tempfile::TempDir, p::Prepared, install::Installed) {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut request: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    request.root_directory = d.path().join("service").to_str().unwrap().into();
    request.service_uid = rustix::process::geteuid().as_raw();
    request.controller_uid = if request.service_uid == 0 {
        1
    } else {
        !request.service_uid
    };
    request.grant.permissions = ["read", "preview", "create", "apply"]
        .into_iter()
        .map(|operation| Permission {
            operation: "effect.execute".into(),
            resource: format!(
                "enrollment-config/{}/{operation}",
                request.installation_digest
            ),
        })
        .collect();
    let prepared = p::prepare(&request).unwrap();
    let artifact = service_artifact::from_prepared(&prepared).unwrap();
    let installed = install::install(
        &install::Context::new(),
        &artifact,
        &artifact.pin().unwrap(),
        install::Mode::Create,
    )
    .unwrap();
    (d, prepared, installed)
}
/// Independent installer success is retained before opening; no peer admitted.
fn open(i: &install::Installed) -> Service {
    Service::open(
        &swarm_enrollment_state::Context::new(),
        &i.bootstrap_path,
        &i.bootstrap_pin,
        Path::new(&i.personal_seed_path),
    )
    .unwrap()
}
/// One guarded packet; core Result success does not mean its inner operation passed.
fn call(service: &mut Service, ctx: &Context, pin: &str, operation: Operation<'_>) -> OutcomeData {
    reply_call(service, ctx, pin, operation).outcome
}
/// Preserve complete Reply for response/output checks; request binding excludes
/// secrets and is compared before the guarded packet is dropped.
fn reply_call(
    service: &mut Service,
    ctx: &Context,
    pin: &str,
    operation: Operation<'_>,
) -> swarm_enrollment_authority::personal_service::Reply {
    let raw = wire::encode(pin, &[19; 32], &operation).unwrap();
    let digest = wire::decode(&raw).unwrap().public_digest();
    let reply = service.dispatch(ctx, wire::decode(&raw).unwrap()).unwrap();
    assert_eq!(reply.challenge, [19; 32]);
    assert_eq!(reply.request_digest(), digest);
    assert_eq!(format!("{reply:?}"), "PersonalServiceReply { <private> }");
    reply
}
/// Only public target/correlation/action/Change influence this binding; never
/// publish a password/proof hash usable as a fast offline knowledge oracle.
#[test]
fn personal_service_public_binding_excludes_secrets_and_includes_exact_action() {
    let first = wire::encode(
        PIN,
        &[11; 32],
        &Operation::Enroll {
            proof: &[42; 32],
            password: PASSWORD,
        },
    )
    .unwrap();
    let same_public = wire::encode(
        PIN,
        &[11; 32],
        &Operation::Enroll {
            proof: &[7; 32],
            password: b"different\0secret",
        },
    )
    .unwrap();
    let digest = wire::decode(&first).unwrap().public_digest();
    assert_eq!(digest, wire::decode(&same_public).unwrap().public_digest());
    for raw in [
        wire::encode(
            &"00".repeat(32),
            &[11; 32],
            &Operation::Enroll {
                proof: &[42; 32],
                password: PASSWORD,
            },
        )
        .unwrap(),
        wire::encode(
            PIN,
            &[12; 32],
            &Operation::Enroll {
                proof: &[42; 32],
                password: PASSWORD,
            },
        )
        .unwrap(),
        wire::encode(PIN, &[11; 32], &Operation::Recover).unwrap(),
    ] {
        assert_ne!(digest, wire::decode(&raw).unwrap().public_digest());
    }
    let create = wire::encode(
        PIN,
        &[11; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    )
    .unwrap();
    let mut different = change();
    different.reason = "different explicit action".into();
    let changed = wire::encode(
        PIN,
        &[11; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(different),
        },
    )
    .unwrap();
    assert_ne!(
        wire::decode(&create).unwrap().public_digest(),
        wire::decode(&changed).unwrap().public_digest()
    );
}
/// Real initialized/enrolled/recovered results map only their actual public
/// phases; canonical response has no credential digest, proof or password field.
#[test]
fn personal_service_response_actual_lifecycle_phases_and_canonical_send() {
    use swarm_enrollment_authority::service_response as response;
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    let init = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Initialize,
    );
    let mut sink = vec![];
    let sent = response::send(&mut sink, init).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&sink).unwrap();
    assert_eq!(value["operation"], 1);
    assert_eq!(value["checkpoint"], "durable");
    assert_eq!(value["journal"], "not_attempted");
    assert_eq!(swarm_wire_json::encode(&sent.observation).unwrap(), sink);
    assert!(sent.observation.completed());
    let mut proof = Zeroizing::new(Vec::new());
    p.write_installation_proof(&mut *proof).unwrap();
    let reply = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Enroll {
            proof: proof.as_slice().try_into().unwrap(),
            password: PASSWORD,
        },
    );
    let mut sink = vec![];
    let sent = response::send(&mut sink, reply).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&sink).unwrap();
    assert_eq!(value["operation"], 2);
    assert_eq!(value["intent"], "durable");
    assert_eq!(value["journal"], "acknowledged");
    assert_eq!(value["sequence"], 3);
    assert!(matches!(sent.reply.outcome, OutcomeData::Account(Ok(_))));
    let reply = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Recover,
    );
    let mut sink = vec![];
    let sent = response::send(&mut sink, reply).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&sink).unwrap();
    assert_eq!(value["operation"], 3);
    assert_eq!(value["journal"], "current_verified");
    assert!(value["intent"].is_null());
    assert_eq!(value["sequence"], 3);
    let keys = value.as_object().unwrap();
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schemas/paea/personal-service-response-v1.schema.json"
    ))
    .unwrap();
    let required: std::collections::BTreeSet<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        keys.keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        required
    );
    for forbidden in [
        "password",
        "proof",
        "seed",
        "credential_digest",
        "head",
        "path",
    ] {
        assert!(!keys.contains_key(forbidden));
    }
    assert_eq!(
        format!("{:?}", sent.observation),
        "PersonalServiceObservation { <private> }"
    );
    service.close().unwrap();
}
/// Real password/duplicate-init failures remain failures after successful JSON
/// delivery; complete original causes survive in Sent rather than being dropped.
#[test]
fn personal_service_response_actual_refusals_keep_original_failure_after_send() {
    use swarm_enrollment_authority::service_response as response;
    core_valid();
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    let reply = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Initialize,
    );
    let sent = response::send(&mut vec![], reply).unwrap();
    assert!(!sent.observation.completed());
    assert_eq!(
        sent.observation.failure_layers(),
        &[response::FailureLayer::Personal]
    );
    assert!(matches!(
        sent.reply.outcome,
        OutcomeData::Initialized(Err(_))
    ));
    let reply = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Control {
            password: b"wrong-fixture-password",
            request: control::Request::Create(change()),
        },
    );
    let sent = response::send(&mut vec![], reply).unwrap();
    assert!(!sent.observation.completed());
    assert!(sent.observation.view().is_none());
    assert!(matches!(sent.reply.outcome, OutcomeData::Control(Err(_))));
    service.close().unwrap();
}
/// Disposable writer: immediate/partial failure or flush refusal. No business
/// operation is retried; safe diagnostics must not leak the custom IO text.
struct BrokenOutput {
    mode: u8,
    wrote: bool,
}
impl std::io::Write for BrokenOutput {
    /// Record a synthetic partial/full write; preserve the next original failure.
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.mode == 3 {
            return Ok(bytes.len());
        }
        if self.mode == 2 && !self.wrote {
            self.wrote = true;
            return Ok(1);
        }
        Err(std::io::Error::other("private-output-fixture-never-log"))
    }
    /// Real write_all success remains known if this separate flush refuses.
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("private-flush-fixture-never-log"))
    }
}
/// Actual successful mutation is retained under explicitly synthetic nested
/// after-use/after-storage faults, then each IO failure retains every branch.
#[test]
fn personal_service_response_nested_known_result_and_output_failure_branches() {
    use std::error::Error;
    use swarm_enrollment_authority::{
        personal::coordinator::UseError, service_response as response,
    };
    for mode in [1, 2, 3] {
        core_valid();
        let (_d, p, i) = fixture();
        let mut service = open(&i);
        enroll(&mut service, &p, &i);
        let mut reply = reply_call(
            &mut service,
            &Context::new(),
            &i.bootstrap_pin,
            Operation::Control {
                password: PASSWORD,
                request: control::Request::Create(change()),
            },
        );
        let original = std::mem::replace(&mut reply.outcome, OutcomeData::Initialized(Ok(())));
        let view = match original {
            OutcomeData::Control(Ok(Ok(control::Response::View(v)))) => v,
            _ => panic!("valid witness failed"),
        };
        reply.outcome = OutcomeData::Control(Err(UseError::AfterUse {
            cause: swarm_enrollment_state::Error::State,
            outcome: Err(control::Error::AfterStorage {
                cause: swarm_enrollment_state::Error::Closed,
                storage: Box::new(Ok(control::Response::View(view))),
                effect_unknown: true,
            }),
        }));
        let failure = response::send(&mut BrokenOutput { mode, wrote: false }, reply).unwrap_err();
        assert_eq!(
            failure.phase,
            if mode == 3 {
                response::OutputPhase::WrittenFlushFailed
            } else {
                response::OutputPhase::WriteUnknown
            }
        );
        assert!(!failure.observation.completed());
        assert!(failure.observation.configuration_effect_unknown());
        assert_eq!(
            failure.observation.failure_layers(),
            &[
                response::FailureLayer::PersonalAfterUse,
                response::FailureLayer::BusinessAfterStorage
            ]
        );
        assert_eq!(
            failure.observation.view().unwrap().snapshot.history.len(),
            1
        );
        assert!(failure.source().is_some());
        assert!(!format!("{failure:?}").contains("never-log"));
        match failure.reply.outcome {
            OutcomeData::Control(Err(UseError::AfterUse {
                outcome: Err(control::Error::AfterStorage { storage, .. }),
                ..
            })) => assert!(storage.is_ok()),
            _ => panic!("lost independent cause"),
        }
        service.close().unwrap();
    }
}
/// Explicitly corrupt only a synthetic copy of an actual returned view with an
/// unrepresentable RFC3339 year. Encoding refuses before writer IO and retains
/// the original typed payload; this is not an observed production view defect.
#[test]
fn personal_service_response_encoding_refusal_keeps_reply_before_any_output() {
    use swarm_enrollment_authority::service_response as response;
    core_valid();
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    let mut reply = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    );
    match &mut reply.outcome {
        OutcomeData::Control(Ok(Ok(control::Response::View(v)))) => {
            v.snapshot.history[0].at = time::Date::MIN.midnight().assume_utc();
        }
        _ => panic!("actual valid witness failed"),
    }
    let mut writer = vec![];
    let failure = response::send(&mut writer, reply).unwrap_err();
    assert_eq!(failure.phase, response::OutputPhase::Encode);
    assert!(writer.is_empty());
    assert!(matches!(failure.cause, response::OutputCause::Encode(_)));
    assert!(matches!(
        failure.reply.outcome,
        OutcomeData::Control(Ok(Ok(control::Response::View(_))))
    ));
    assert_eq!(
        failure.observation.view().unwrap().snapshot.history.len(),
        1
    );
    service.close().unwrap();
}
/// A sole personal after-use error retains an inner successful result. Dropping
/// that known View is a malformed response, even though completed remains false.
/// Fault wrapper is synthetic; the preceding actual mutation is a valid witness.
#[test]
fn personal_service_consumer_refuses_dropped_known_after_use_result() {
    use swarm_enrollment_authority::{
        personal::coordinator::UseError,
        service_response::{self as response, consumer},
    };
    core_valid();
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    let packet = wire::encode(
        &i.bootstrap_pin,
        &[19; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    )
    .unwrap();
    let request = wire::decode(&packet).unwrap();
    let mut reply = service
        .dispatch(&Context::new(), wire::decode(&packet).unwrap())
        .unwrap();
    let original = std::mem::replace(&mut reply.outcome, OutcomeData::Initialized(Ok(())));
    let result = match original {
        OutcomeData::Control(Ok(Ok(v))) => v,
        _ => panic!("actual witness failed"),
    };
    reply.outcome = OutcomeData::Control(Err(UseError::AfterUse {
        cause: swarm_enrollment_state::Error::State,
        outcome: Ok(result),
    }));
    let mut raw = vec![];
    response::send(&mut raw, reply).unwrap();
    let pins = p.declaration().unwrap().control.installation.pins;
    assert!(consumer::decode(&raw, &request, &pins).is_ok());
    let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    value["view"] = serde_json::Value::Null;
    let malformed: response::Observation = serde_json::from_value(value).unwrap();
    let missing = swarm_wire_json::encode(&malformed).unwrap();
    assert!(
        consumer::decode(&missing, &request, &pins).is_err(),
        "sole AfterUse must not lose its known successful inner result"
    );
    assert!(consumer::decode(&raw, &request, &pins).is_ok());
    service.close().unwrap();
}
/// Canonical semantic mutation preserves actual field ordering. This ensures
/// early canonical rejection cannot mask the later shape/history check.
fn mutate_response(raw: &[u8], f: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(raw).unwrap();
    f(&mut value);
    let typed: swarm_enrollment_authority::service_response::Observation =
        serde_json::from_value(value).unwrap();
    swarm_wire_json::encode(&typed).unwrap()
}
/// Actual roundtrip keeps expected request alive and independent pins separate
/// from returned data. This checks data only, never admits a real peer.
fn checked_exchange(
    service: &mut Service,
    pin: &str,
    pins: &swarm_enrollment_config::Pins,
    operation: Operation<'_>,
) -> (
    Vec<u8>,
    swarm_enrollment_authority::service_response::consumer::BoundResponse,
) {
    use swarm_enrollment_authority::service_response::{self as response, consumer};
    let packet = wire::encode(pin, &[19; 32], &operation).unwrap();
    let expected = wire::decode(&packet).unwrap();
    let reply = service
        .dispatch(&Context::new(), wire::decode(&packet).unwrap())
        .unwrap();
    let mut raw = vec![];
    response::send(&mut raw, reply).unwrap();
    let bound = consumer::decode(&raw, &expected, pins).unwrap();
    (raw, bound)
}
/// All seven real operations, preview versus saved state and exact replay are
/// accepted with producer validators; no empty success or inferred apply.
#[test]
fn personal_service_consumer_actual_all_operations_preview_and_replay() {
    let (_d, p, i) = fixture();
    let pins = p.declaration().unwrap().control.installation.pins;
    let mut service = open(&i);
    let (_, b) = checked_exchange(&mut service, &i.bootstrap_pin, &pins, Operation::Initialize);
    assert!(b.observation().completed());
    let mut proof = Zeroizing::new(Vec::new());
    p.write_installation_proof(&mut *proof).unwrap();
    let (_, b) = checked_exchange(
        &mut service,
        &i.bootstrap_pin,
        &pins,
        Operation::Enroll {
            proof: proof.as_slice().try_into().unwrap(),
            password: PASSWORD,
        },
    );
    assert!(b.observation().completed());
    let (_, b) = checked_exchange(&mut service, &i.bootstrap_pin, &pins, Operation::Recover);
    assert!(b.observation().completed());
    let (_, b) = checked_exchange(
        &mut service,
        &i.bootstrap_pin,
        &pins,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    );
    let head = b.observation().view().unwrap().head.clone();
    let (_, b) = checked_exchange(
        &mut service,
        &i.bootstrap_pin,
        &pins,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Preview(change()),
        },
    );
    assert!(b.observation().projection().unwrap().current_event_replay);
    let mut next = change();
    next.event_id = "service-core-next-event".into();
    next.expected_head = head;
    next.reason = "next explicit configuration".into();
    let (_, b) = checked_exchange(
        &mut service,
        &i.bootstrap_pin,
        &pins,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Preview(next.clone()),
        },
    );
    assert_eq!(
        b.observation()
            .projection()
            .unwrap()
            .proposed
            .snapshot
            .history
            .len(),
        2
    );
    assert!(!b.observation().projection().unwrap().current_event_replay);
    let (_, b) = checked_exchange(
        &mut service,
        &i.bootstrap_pin,
        &pins,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Read,
        },
    );
    assert_eq!(b.observation().view().unwrap().snapshot.history.len(), 1);
    for _ in 0..2 {
        let (_, b) = checked_exchange(
            &mut service,
            &i.bootstrap_pin,
            &pins,
            Operation::Control {
                password: PASSWORD,
                request: control::Request::Apply(next.clone()),
            },
        );
        assert_eq!(b.observation().view().unwrap().snapshot.history.len(), 2);
    }
    service.close().unwrap();
}
/// Wrong request binding/canonical frame, incomplete success, inconsistent
/// phases/layers and data mixtures refuse after a valid same-response witness.
#[test]
fn personal_service_consumer_binding_and_semantic_shape_refusals_then_recovery() {
    use swarm_enrollment_authority::service_response::consumer;
    core_valid();
    let (_d, p, i) = fixture();
    let pins = p.declaration().unwrap().control.installation.pins;
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    let packet = wire::encode(
        &i.bootstrap_pin,
        &[19; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    )
    .unwrap();
    let expected = wire::decode(&packet).unwrap();
    let (raw, _) = checked_exchange(
        &mut service,
        &i.bootstrap_pin,
        &pins,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    );
    let variants = vec![
        mutate_response(&raw, |v| {
            v["request_digest"] = serde_json::json!("00".repeat(32))
        }),
        mutate_response(&raw, |v| v["challenge"] = serde_json::json!("A".repeat(43))),
        mutate_response(&raw, |v| v["operation"] = serde_json::json!(4)),
        mutate_response(&raw, |v| v["view"] = serde_json::Value::Null),
        mutate_response(&raw, |v| v["completed"] = serde_json::json!(false)),
        mutate_response(&raw, |v| {
            v["failure_layers"] = serde_json::json!(["business"])
        }),
        mutate_response(&raw, |v| v["checkpoint"] = serde_json::json!("durable")),
        mutate_response(&raw, |v| v["sequence"] = serde_json::json!(3)),
        [b" ".as_slice(), raw.as_slice()].concat(),
        raw[..raw.len() - 1].to_vec(),
        String::from_utf8(raw.clone())
            .unwrap()
            .replacen('{', "{\"password\":\"forged\",", 1)
            .into_bytes(),
        String::from_utf8(raw.clone())
            .unwrap()
            .replacen('{', "{\"completed\":true,", 1)
            .into_bytes(),
    ];
    for bad in variants {
        assert!(consumer::decode(&raw, &expected, &pins).is_ok());
        assert!(consumer::decode(&bad, &expected, &pins).is_err());
        assert!(consumer::decode(&raw, &expected, &pins).is_ok());
    }
    let other = wire::encode(
        &i.bootstrap_pin,
        &[20; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    )
    .unwrap();
    assert!(consumer::decode(&raw, &wire::decode(&other).unwrap(), &pins).is_err());
    service.close().unwrap();
}
/// Correctly shaped empty/altered history, wrong pins/head and a different yet
/// valid adjacent Change cannot pass by preserving a top-level request digest.
#[test]
fn personal_service_consumer_history_pins_and_neighbor_action_refuse() {
    use swarm_enrollment_authority::service_response::{self as response, consumer};
    core_valid();
    let (_d, p, i) = fixture();
    let pins = p.declaration().unwrap().control.installation.pins;
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    let packet = wire::encode(
        &i.bootstrap_pin,
        &[19; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    )
    .unwrap();
    let expected = wire::decode(&packet).unwrap();
    let (raw, b) = checked_exchange(
        &mut service,
        &i.bootstrap_pin,
        &pins,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    );
    let variants = [
        mutate_response(&raw, |v| {
            v["view"]["snapshot"]["history"] = serde_json::json!([])
        }),
        mutate_response(&raw, |v| {
            v["view"]["head"]["digest"] = serde_json::json!("00".repeat(32))
        }),
        mutate_response(&raw, |v| {
            v["view"]["snapshot"]["history"][0]["change"]["reason"] =
                serde_json::json!("changed without rehash")
        }),
        mutate_response(&raw, |v| {
            v["view"]["snapshot"]["pins"]["installation_digest"] =
                serde_json::json!("00".repeat(32))
        }),
    ];
    for bad in variants {
        assert!(consumer::decode(&raw, &expected, &pins).is_ok());
        assert!(consumer::decode(&bad, &expected, &pins).is_err());
    }
    let mut other_pins = pins.clone();
    other_pins.installation_digest = "00".repeat(32);
    assert!(consumer::decode(&raw, &expected, &other_pins).is_err());
    let mut adjacent = b.observation().view().unwrap().snapshot.clone();
    adjacent.history[0].change.reason = "another valid explicit action".into();
    let adjacent = swarm_enrollment_config::snapshot_view(&adjacent).unwrap();
    let value = serde_json::to_value(adjacent).unwrap();
    let bad = mutate_response(&raw, |v| v["view"] = value);
    let typed: response::Observation = serde_json::from_slice(&bad).unwrap();
    swarm_enrollment_config::validate_view(typed.view().unwrap()).unwrap();
    assert!(consumer::decode(&bad, &expected, &pins).is_err());
    assert!(consumer::decode(&raw, &expected, &pins).is_ok());
    service.close().unwrap();
}
/// Personal success phases must match init/enroll/recover, and failed acknowledged
/// progress remains acceptable without being promoted to success. Canonical
/// semantic negatives have a same-response valid witness before each refusal.
#[test]
fn personal_service_consumer_personal_phase_verdict_and_sequence_coherence() {
    use swarm_enrollment_authority::service_response::consumer;
    let (_d, p, i) = fixture();
    let pins = p.declaration().unwrap().control.installation.pins;
    let mut service = open(&i);
    let mut proof = Zeroizing::new(Vec::new());
    p.write_installation_proof(&mut *proof).unwrap();
    for operation in [
        Operation::Initialize,
        Operation::Enroll {
            proof: proof.as_slice().try_into().unwrap(),
            password: PASSWORD,
        },
        Operation::Recover,
    ] {
        let packet = wire::encode(&i.bootstrap_pin, &[19; 32], &operation).unwrap();
        let expected = wire::decode(&packet).unwrap();
        let (raw, _) = checked_exchange(&mut service, &i.bootstrap_pin, &pins, operation);
        let variants = vec![
            mutate_response(&raw, |v| v["checkpoint"] = serde_json::json!("visible")),
            mutate_response(&raw, |v| v["journal"] = serde_json::json!("unknown")),
            mutate_response(&raw, |v| v["sequence"] = serde_json::json!(0)),
            mutate_response(&raw, |v| {
                v["configuration_effect_unknown"] = serde_json::json!(true)
            }),
            mutate_response(&raw, |v| {
                v["failure_layers"] = serde_json::json!(["business"])
            }),
            mutate_response(&raw, |v| v["intent"] = serde_json::json!("unknown")),
        ];
        for bad in variants {
            assert!(consumer::decode(&raw, &expected, &pins).is_ok());
            assert!(consumer::decode(&bad, &expected, &pins).is_err());
        }
        if matches!(
            expected.operation(),
            Operation::Initialize | Operation::Enroll { .. }
        ) {
            let late_failure = mutate_response(&raw, |v| {
                v["completed"] = serde_json::json!(false);
                v["failure_layers"] = serde_json::json!(["personal"])
            });
            let b = consumer::decode(&late_failure, &expected, &pins).unwrap();
            assert!(!b.observation().completed());
        }
    }
    service.close().unwrap();
}
/// Explicit actual init/enroll and typed account check, never a human-proof claim.
fn enroll(service: &mut Service, p: &p::Prepared, i: &install::Installed) {
    assert!(matches!(
        call(
            service,
            &Context::new(),
            &i.bootstrap_pin,
            Operation::Initialize
        ),
        OutcomeData::Initialized(Ok(()))
    ));
    let mut proof = Zeroizing::new(Vec::new());
    p.write_installation_proof(&mut *proof).unwrap();
    match call(
        service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Enroll {
            proof: proof.as_slice().try_into().unwrap(),
            password: PASSWORD,
        },
    ) {
        OutcomeData::Account(Ok(a)) => assert_eq!(a.head().sequence, 3),
        _ => panic!("not enrolled"),
    }
}
/// Valid full core operation under the same real fixture preconditions before
/// each fault; a common startup refusal cannot mask the intended rejection.
fn core_valid() {
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    assert!(matches!(
        call(
            &mut service,
            &Context::new(),
            &i.bootstrap_pin,
            Operation::Control {
                password: PASSWORD,
                request: control::Request::Create(change())
            }
        ),
        OutcomeData::Control(Ok(Ok(control::Response::View(_))))
    ));
    service.close().unwrap();
}
/// Full same-owner core path then protected reopen, real storage and no duplicate.
#[test]
fn personal_service_core_real_cold_enrollment_control_and_reopen() {
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    for _ in 0..2 {
        match call(
            &mut service,
            &Context::new(),
            &i.bootstrap_pin,
            Operation::Control {
                password: PASSWORD,
                request: control::Request::Create(change()),
            },
        ) {
            OutcomeData::Control(Ok(Ok(control::Response::View(v)))) => {
                assert_eq!(v.snapshot.history.len(), 1)
            }
            x => panic!("unexpected {x:?}"),
        }
    }
    service.close().unwrap();
    let mut service = open(&i);
    match call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Recover,
    ) {
        OutcomeData::Account(Ok(a)) => assert_eq!(a.head().sequence, 3),
        _ => panic!("not recovered"),
    }
    match call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Read,
        },
    ) {
        OutcomeData::Control(Ok(Ok(control::Response::View(v)))) => {
            assert_eq!(v.snapshot.history.len(), 1)
        }
        x => panic!("unexpected {x:?}"),
    }
    assert_eq!(format!("{service:?}"), "PersonalService { <private> }");
    service.close().unwrap();
}
/// Wrong target refuses before init; canceled operation retains a typed failure.
#[test]
fn personal_service_core_wrong_target_and_cancellation_before_init_then_recover() {
    core_valid();
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    let raw = wire::encode(&"00".repeat(32), &[1; 32], &Operation::Initialize).unwrap();
    let e = service
        .dispatch(&Context::new(), wire::decode(&raw).unwrap())
        .unwrap_err();
    assert_eq!(e.code(), "service-target");
    assert!(!Path::new(&i.bootstrap_path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("personal-witness/personal-checkpoint.json")
        .exists());
    let canceled = Context::new();
    canceled.cancel();
    assert!(matches!(
        call(
            &mut service,
            &canceled,
            &i.bootstrap_pin,
            Operation::Initialize
        ),
        OutcomeData::Initialized(Err(_))
    ));
    enroll(&mut service, &p, &i);
    assert!(Path::new(&i.bootstrap_path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("personal-witness/personal-checkpoint.json")
        .is_file());
    service.close().unwrap();
}
/// Wrong password cannot publish configuration, and errors remain nested explicitly.
#[test]
fn personal_service_core_password_refusal_and_duplicate_init_keep_state_then_valid() {
    core_valid();
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    assert!(matches!(
        call(
            &mut service,
            &Context::new(),
            &i.bootstrap_pin,
            Operation::Initialize
        ),
        OutcomeData::Initialized(Err(_))
    ));
    assert!(matches!(
        call(
            &mut service,
            &Context::new(),
            &i.bootstrap_pin,
            Operation::Control {
                password: b"wrong-fixture-password",
                request: control::Request::Create(change())
            }
        ),
        OutcomeData::Control(Err(_))
    ));
    let config = Path::new(&i.bootstrap_path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("configuration/config.json");
    assert!(!config.exists());
    match call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    ) {
        OutcomeData::Control(Ok(Ok(control::Response::View(v)))) => {
            assert_eq!(v.snapshot.history.len(), 1)
        }
        x => panic!("unexpected {x:?}"),
    }
    match call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Recover,
    ) {
        OutcomeData::Account(Ok(a)) => assert_eq!(a.head().sequence, 3),
        _ => panic!("not recovered"),
    };
    service.close().unwrap();
}
/// Initial pinned startup refusal keeps the actual typed bootstrap cause and
/// does not initialize a checkpoint; the same installed sources still reopen.
#[test]
fn personal_service_core_startup_refusal_preserves_cause_then_valid_open() {
    use std::error::Error;
    core_valid();
    let (_d, p, i) = fixture();
    let e = Service::open(
        &swarm_enrollment_state::Context::new(),
        &i.bootstrap_path,
        &"00".repeat(32),
        Path::new(&i.personal_seed_path),
    )
    .unwrap_err();
    assert!(e.source().is_some());
    assert_eq!(format!("{e:?}"), "personal service: service-startup");
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    service.close().unwrap();
}
/// Prepare actual pending intent/journal/checkpoint effects through the existing
/// fixture-only lifecycle hook, then recover via the normal core with no secret,
/// new event or reset. Hooks do not exist on this service's production API.
#[test]
#[cfg(feature = "test-fixtures")]
fn personal_service_core_exact_pending_recovery_after_actual_durable_phases() {
    use swarm_enrollment_authority::{
        authenticated_startup,
        personal::coordinator::{Error, FixturePhase},
    };
    for phase in [
        FixturePhase::IntentDurable,
        FixturePhase::JournalApplied,
        FixturePhase::CheckpointDurable,
    ] {
        core_valid();
        let (_d, p, i) = fixture();
        let app = authenticated_startup::open_lifecycle(
            &swarm_enrollment_state::Context::new(),
            &i.bootstrap_path,
            &i.bootstrap_pin,
            Path::new(&i.personal_seed_path),
        )
        .unwrap();
        let ctx = Context::new();
        app.initialize(&ctx).unwrap();
        app.fixture_hook(Some(std::sync::Arc::new(move |actual| {
            if phase == actual {
                Err(Error::Internal)
            } else {
                Ok(())
            }
        })));
        let mut proof = Zeroizing::new(Vec::new());
        p.write_installation_proof(&mut *proof).unwrap();
        let failure = app
            .enroll(&ctx, proof.as_slice().try_into().unwrap(), PASSWORD)
            .unwrap_err();
        assert_eq!(
            failure.progress.intent,
            swarm_enrollment_config::store::documents::Publication::Durable
        );
        if phase != FixturePhase::IntentDurable {
            assert!(matches!(
                failure.progress.journal,
                swarm_enrollment_authority::personal::coordinator::JournalOutcome::Acknowledged(_)
            ));
        }
        app.fixture_hook(None);
        app.close().unwrap();
        let mut service = open(&i);
        for _ in 0..2 {
            match call(
                &mut service,
                &Context::new(),
                &i.bootstrap_pin,
                Operation::Recover,
            ) {
                OutcomeData::Account(Ok(a)) => assert_eq!(a.head().sequence, 3),
                x => panic!("unexpected {x:?}"),
            }
        }
        assert!(matches!(
            call(
                &mut service,
                &Context::new(),
                &i.bootstrap_pin,
                Operation::Enroll {
                    proof: proof.as_slice().try_into().unwrap(),
                    password: b"new-refused-password"
                }
            ),
            OutcomeData::Account(Err(_))
        ));
        service.close().unwrap();
    }
}

/// Controller role comes only from the pinned bootstrap, survives a target refusal,
/// a real initialize and explicit reopen. The synthetic different UID is data,
/// not a real OS account or evidence that a peer has been admitted.
#[test]
fn personal_service_retains_pinned_controller_role_across_refusal_and_reopen() {
    let (_directory, prepared, installed) = fixture();
    let expected = prepared
        .declaration()
        .unwrap()
        .control
        .installation
        .controller_uid;
    assert_ne!(expected, rustix::process::geteuid().as_raw());
    let mut service = open(&installed);
    assert_eq!(service.controller_uid(), expected);
    let bad = wire::encode(PIN, &[23; 32], &Operation::Recover).unwrap();
    assert_eq!(
        service
            .dispatch(&Context::new(), wire::decode(&bad).unwrap())
            .err()
            .unwrap()
            .code(),
        "service-target"
    );
    assert_eq!(service.controller_uid(), expected);
    match call(
        &mut service,
        &Context::new(),
        &installed.bootstrap_pin,
        Operation::Initialize,
    ) {
        OutcomeData::Initialized(result) => result.unwrap(),
        _ => panic!("expected actual initialize"),
    }
    assert_eq!(service.controller_uid(), expected);
    service.close().unwrap();
    let reopened = open(&installed);
    assert_eq!(reopened.controller_uid(), expected);
    reopened.close().unwrap();
}

/// Actual kernel packet lends its exact guarded bytes to the existing decoder,
/// without copying/reallocating secrets or including unused receive capacity.
/// Both an empty-tail operation and exact binary credential operation are covered.
#[test]
fn personal_service_request_decodes_exact_guarded_kernel_packet_without_copy() {
    use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
    use std::os::fd::AsRawFd;
    let (sender, receiver) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut channel = swarm_linux_identity_channel::configure(&receiver).unwrap();
    for operation in [
        Operation::Initialize,
        Operation::Enroll {
            proof: &[42; 32],
            password: PASSWORD,
        },
    ] {
        let raw = wire::encode(PIN, &[19; 32], &operation).unwrap();
        let storage = Zeroizing::new(vec![0xa5; raw.len() + 8]);
        let allocation = storage.as_ptr();
        let capacity = storage.capacity();
        send(sender.as_raw_fd(), &raw, MsgFlags::empty()).unwrap();
        let packet = channel.receive(storage).unwrap();
        assert_eq!(packet.guarded_data().len(), raw.len());
        assert_eq!(packet.guarded_data().capacity(), capacity);
        assert_eq!(packet.guarded_data().as_ptr(), allocation);
        assert_eq!(packet.data(), raw.as_slice());
        let request = wire::decode(packet.guarded_data()).unwrap();
        assert_eq!(request.target_pin(), PIN);
        match request.operation() {
            Operation::Initialize => {}
            Operation::Enroll { proof, password } => {
                assert_eq!(*proof, &[42; 32]);
                assert_eq!(*password, PASSWORD);
                let begin = packet.data().as_ptr() as usize;
                let end = begin + packet.data().len();
                let actual = password.as_ptr() as usize;
                assert!(actual >= begin && actual + password.len() == end);
            }
            _ => panic!("unexpected fixture operation"),
        }
        drop(request);
        packet.close().unwrap();
    }
}

/// Valid kernel namespaces are a witness, not admission: the actual current UID
/// still differs from the pinned controller role. Reject before business use for
/// both valid/malformed payloads and preserve an initialized checkpoint exactly.
#[test]
fn personal_service_kernel_subject_wrong_controller_refuses_without_business_change() {
    use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
    use std::{collections::BTreeMap, ffi::OsString, os::fd::AsRawFd};
    /// Read owned fixture checkpoint files only; no live database/secret output.
    fn snapshot(path: &Path) -> BTreeMap<OsString, Vec<u8>> {
        fs::read_dir(path)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                assert!(entry.file_type().unwrap().is_file());
                (entry.file_name(), fs::read(entry.path()).unwrap())
            })
            .collect()
    }
    let (_directory, prepared, installed) = fixture();
    let mut service = open(&installed);
    match call(
        &mut service,
        &Context::new(),
        &installed.bootstrap_pin,
        Operation::Initialize,
    ) {
        OutcomeData::Initialized(result) => result.unwrap(),
        _ => panic!("initialize witness"),
    }
    let checkpoint = prepared
        .declaration()
        .unwrap()
        .personal
        .checkpoint_directory;
    let before = snapshot(Path::new(&checkpoint));
    assert!(!before.is_empty());
    let (sender, receiver) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut channel = swarm_linux_identity_channel::configure(&receiver).unwrap();
    let valid = wire::encode(&installed.bootstrap_pin, &[27; 32], &Operation::Initialize).unwrap();
    for bytes in [valid.as_slice(), b"malformed-controller-payload".as_slice()] {
        send(sender.as_raw_fd(), bytes, MsgFlags::empty()).unwrap();
        let packet = channel
            .receive(Zeroizing::new(vec![0; bytes.len() + 8]))
            .unwrap();
        packet.namespaces().unwrap().close().unwrap();
        assert_ne!(packet.credentials().1, service.controller_uid());
        let error = service
            .controller_namespaces(&packet)
            .err()
            .expect("wrong pinned controller must refuse");
        assert_eq!(error.code(), "service-controller-uid");
        assert!(error.channel_error().is_none());
        assert!(snapshot(Path::new(&checkpoint)) == before);
        packet.close().unwrap();
    }
    service.close().unwrap();
}

/// Actual public packet-to-dispatch path refuses current service UID as the
/// protected controller before malformed decode; real initialized state survives.
/// Its synthetic distinct controller UID does not prove a second OS account.
#[test]
fn personal_service_packet_dispatch_wrong_uid_preserves_real_state() {
    use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
    use std::os::fd::AsRawFd;
    let (_directory, prepared, installed) = fixture();
    let mut service = open(&installed);
    match call(
        &mut service,
        &Context::new(),
        &installed.bootstrap_pin,
        Operation::Initialize,
    ) {
        OutcomeData::Initialized(r) => r.unwrap(),
        _ => panic!("valid init witness"),
    }
    let checkpoint = prepared
        .declaration()
        .unwrap()
        .personal
        .checkpoint_directory;
    /// Snapshot only this isolated fixture witness; no secret content is printed.
    fn snapshot(p: &str) -> std::collections::BTreeMap<std::ffi::OsString, Vec<u8>> {
        fs::read_dir(p)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), fs::read(entry.path()).unwrap())
            })
            .collect()
    }
    let before = snapshot(&checkpoint);
    assert!(!before.is_empty());
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = swarm_linux_identity_channel::configure(&rx).unwrap();
    for body in [
        wire::encode(&installed.bootstrap_pin, &[58; 32], &Operation::Recover).unwrap(),
        Zeroizing::new(b"bad-frame".to_vec()),
    ] {
        send(tx.as_raw_fd(), &body, MsgFlags::empty()).unwrap();
        let p = c.receive(Zeroizing::new(vec![0; body.len() + 8])).unwrap();
        let expected = p.namespaces().unwrap();
        let e = service
            .dispatch_controller_packet(&Context::new(), &p, &expected)
            .err()
            .unwrap();
        assert_eq!(e.code(), "service-controller-uid");
        assert!(e.reply.is_none() && e.cleanup.is_none());
        assert!(snapshot(&checkpoint) == before);
        expected.close().unwrap();
        p.close().unwrap();
    }
    assert!(snapshot(&checkpoint) == before);
    service.close().unwrap();
    let reopened = open(&installed);
    reopened.close().unwrap();
}

/// Actual service rejects its own UID before consulting an expired process
/// anchor or malformed payload. Initialized fixture bytes and reopen survive.
/// Controller complement is synthetic data; no separate OS account is claimed.
#[test]
fn personal_service_spawned_dispatch_wrong_uid_before_expired_anchor() {
    use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
    use std::{
        io::Write,
        os::fd::AsRawFd,
        process::{Command, Stdio},
    };
    let (_directory, prepared, installed) = fixture();
    let mut service = open(&installed);
    match call(
        &mut service,
        &Context::new(),
        &installed.bootstrap_pin,
        Operation::Initialize,
    ) {
        OutcomeData::Initialized(r) => r.unwrap(),
        _ => panic!("valid init witness"),
    }
    let checkpoint = prepared
        .declaration()
        .unwrap()
        .personal
        .checkpoint_directory;
    /// Observe only disposable witness bytes; never print their contents.
    fn snapshot(p: &str) -> std::collections::BTreeMap<std::ffi::OsString, Vec<u8>> {
        fs::read_dir(p)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (e.file_name(), fs::read(e.path()).unwrap())
            })
            .collect()
    }
    let before = snapshot(&checkpoint);
    assert!(!before.is_empty());
    let mut child = Command::new("/usr/bin/python3")
        .args(["-c", "import sys; sys.stdin.buffer.read(1)"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let anchor = swarm_linux_identity_channel::ProcessAnchor::capture_child(&mut child);
    child.stdin.as_mut().unwrap().write_all(b"X").unwrap();
    assert!(child.wait().unwrap().success());
    let anchor = anchor.unwrap();
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut c = swarm_linux_identity_channel::configure(&rx).unwrap();
    send(tx.as_raw_fd(), b"malformed", MsgFlags::empty()).unwrap();
    let packet = c.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let expected = packet.namespaces().unwrap();
    assert_eq!(
        anchor.matches(&packet).err().unwrap().code(),
        "identity-channel-process-exited"
    );
    let error = service
        .dispatch_spawned_controller_packet(&Context::new(), &packet, &anchor, &expected)
        .err()
        .unwrap();
    assert_eq!(error.code(), "service-controller-uid");
    assert!(error.reply.is_none() && error.cleanup.is_none());
    assert!(snapshot(&checkpoint) == before);
    expected.close().unwrap();
    packet.close().unwrap();
    anchor.close().unwrap();
    service.close().unwrap();
    let reopened = open(&installed);
    reopened.close().unwrap();
}

/// Atomic-created type cannot bypass pinned service UID, even after child exit.
/// Real fixture initialization/reopen survive; no distinct OS UID success claim.
#[cfg(target_env = "gnu")]
#[test]
fn personal_service_atomic_dispatch_uid_before_finished_controller() {
    use nix::sys::{
        signal::SigSet,
        socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType},
    };
    use std::{
        ffi::CString,
        os::fd::{AsFd, AsRawFd},
    };
    use swarm_linux_identity_channel::atomic_spawn::{self, Spec};
    let (_directory, _prepared, installed) = fixture();
    let mut service = open(&installed);
    match call(
        &mut service,
        &Context::new(),
        &installed.bootstrap_pin,
        Operation::Initialize,
    ) {
        OutcomeData::Initialized(r) => r.unwrap(),
        _ => panic!("valid init control"),
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/usr/bin/true").unwrap();
    let directory = CString::new("/").unwrap();
    let argv = [CString::new("true").unwrap()];
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let spec = Spec {
        program: &program,
        arguments: &argv,
        environment: &[],
        directory: &directory,
        stdin: file.as_fd(),
        stdout: file.as_fd(),
        stderr: file.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let mut child = atomic_spawn::launch(&spec).unwrap();
    assert!(child.wait().unwrap().success());
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut channel = swarm_linux_identity_channel::configure(&rx).unwrap();
    send(tx.as_raw_fd(), b"malformed", MsgFlags::empty()).unwrap();
    let packet = channel.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let expected = packet.namespaces().unwrap();
    let error = service
        .dispatch_atomic_controller_packet(&Context::new(), &packet, &child, &expected)
        .err()
        .unwrap();
    assert_eq!(error.code(), "service-controller-uid");
    assert!(error.reply.is_none() && error.cleanup.is_none());
    expected.close().unwrap();
    packet.close().unwrap();
    child.close().unwrap();
    service.close().unwrap();
    open(&installed).close().unwrap();
}

/// Actual isolated control result crosses exactly one kernel packet and the
/// request-bound consumer; equal UID here is transport evidence only, not admission.
#[test]
fn personal_service_packet_response_exact_bound_result_and_recovery() {
    use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
    use swarm_enrollment_authority::service_response::{self as response, consumer};
    use swarm_linux_identity_channel::configure;
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    let raw = wire::encode(
        &i.bootstrap_pin,
        &[61; 32],
        &Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    )
    .unwrap();
    let request = wire::decode(&raw).unwrap();
    let reply = service
        .dispatch(&Context::new(), wire::decode(&raw).unwrap())
        .unwrap();
    let (a, b) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut out = configure(&a).unwrap();
    let mut input = configure(&b).unwrap();
    let sent = response::send_packet(&mut out, reply).unwrap();
    let packet = input.receive(Zeroizing::new(vec![0; 16384])).unwrap();
    assert_eq!(
        packet.data(),
        swarm_wire_json::encode(&sent.observation).unwrap()
    );
    let pins = p.declaration().unwrap().control.installation.pins;
    let bound = consumer::decode(packet.data(), &request, &pins).unwrap();
    assert!(bound.observation().completed());
    assert_eq!(
        bound.observation().view().unwrap().snapshot.history.len(),
        1
    );
    packet.close().unwrap();
    service.close().unwrap();
}
/// Valid packet before closed-peer injection; committed View survives EPIPE and
/// read-only reopen proves persisted result, without retrying the failed mutation.
#[test]
fn personal_service_packet_response_closed_peer_retains_known_view_and_reopen() {
    use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
    use swarm_enrollment_authority::service_response as response;
    use swarm_linux_identity_channel::configure;
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    let (a, b) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut out = configure(&a).unwrap();
    let mut input = configure(&b).unwrap();
    let witness = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Recover,
    );
    response::send_packet(&mut out, witness).unwrap();
    let packet = input.receive(Zeroizing::new(vec![0; 16384])).unwrap();
    assert!(!packet.data().is_empty());
    packet.close().unwrap();
    drop(input);
    drop(b);
    let reply = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    );
    let failure = response::send_packet(&mut out, reply).unwrap_err();
    match &failure.cause {
        response::PacketOutputCause::Send(e) => {
            assert!(e.attempted);
            assert_eq!(
                e.cause.causes()[0].1.raw_os_error(),
                Some(nix::errno::Errno::EPIPE as i32)
            );
        }
        _ => panic!("wrong output cause"),
    };
    assert_eq!(
        failure.observation.view().unwrap().snapshot.history.len(),
        1
    );
    assert!(matches!(
        failure.reply.outcome,
        OutcomeData::Control(Ok(Ok(control::Response::View(_))))
    ));
    service.close().unwrap();
    let mut service = open(&i);
    let read = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Read,
        },
    );
    match &read.outcome {
        OutcomeData::Control(Ok(Ok(control::Response::View(v)))) => {
            assert_eq!(v.snapshot.history.len(), 1)
        }
        _ => panic!("persisted view missing"),
    };
    let (c, d) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut out = configure(&c).unwrap();
    let mut input = configure(&d).unwrap();
    response::send_packet(&mut out, read).unwrap();
    input
        .receive(Zeroizing::new(vec![0; 16384]))
        .unwrap()
        .close()
        .unwrap();
    service.close().unwrap();
}
/// Synthetic unrepresentable timestamp follows a real valid View witness.
/// Encode refuses before any packet; original typed result remains for diagnosis.
#[test]
fn personal_service_packet_response_encode_refusal_before_queue() {
    use nix::sys::socket::{recv, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
    use std::os::fd::AsRawFd;
    use swarm_enrollment_authority::service_response as response;
    use swarm_linux_identity_channel::configure;
    let (_d, p, i) = fixture();
    let mut service = open(&i);
    enroll(&mut service, &p, &i);
    let reply = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Create(change()),
        },
    );
    let (a, b) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut out = configure(&a).unwrap();
    let mut input = configure(&b).unwrap();
    response::send_packet(&mut out, reply).unwrap();
    input
        .receive(Zeroizing::new(vec![0; 16384]))
        .unwrap()
        .close()
        .unwrap();
    let mut reply = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Control {
            password: PASSWORD,
            request: control::Request::Read,
        },
    );
    match &mut reply.outcome {
        OutcomeData::Control(Ok(Ok(control::Response::View(v)))) => {
            v.snapshot.history[0].at = time::Date::MIN.midnight().assume_utc()
        }
        _ => panic!("valid view missing"),
    };
    let failure = response::send_packet(&mut out, reply).unwrap_err();
    assert!(matches!(
        failure.cause,
        response::PacketOutputCause::Encode(_)
    ));
    assert_eq!(
        failure.observation.view().unwrap().snapshot.history.len(),
        1
    );
    let mut buf = [0; 128];
    assert_eq!(
        recv(b.as_raw_fd(), &mut buf, MsgFlags::MSG_DONTWAIT).unwrap_err(),
        nix::errno::Errno::EAGAIN
    );
    service.close().unwrap();
}

/// Actual immutable bootstrap UID refuses before an exited controller or malformed
/// frame; no response or state change, then supported reopen preserves state.
#[cfg(target_env = "gnu")]
#[test]
fn personal_service_atomic_exchange_wrong_uid_no_reply_and_reopen() {
    use nix::sys::{
        signal::SigSet,
        socket::{recv, send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType},
    };
    use std::{
        ffi::CString,
        fs::OpenOptions,
        os::fd::{AsFd, AsRawFd},
    };
    use swarm_linux_identity_channel::{
        self as channel,
        atomic_spawn::{self, Spec},
    };
    let (_d, prepared, i) = fixture();
    /// Capture owned fixture checkpoint bytes only; never inspect live state.
    fn snapshot(path: &Path) -> std::collections::BTreeMap<std::ffi::OsString, Vec<u8>> {
        fs::read_dir(path).unwrap().map(|entry| { let entry=entry.unwrap(); assert!(entry.file_type().unwrap().is_file()); (entry.file_name(),fs::read(entry.path()).unwrap()) }).collect()
    }
    let mut service = open(&i);
    let init = reply_call(
        &mut service,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Initialize,
    );
    assert!(matches!(init.outcome, OutcomeData::Initialized(Ok(()))));
    let checkpoint=prepared.declaration().unwrap().personal.checkpoint_directory;
    let before=snapshot(Path::new(&checkpoint));assert!(!before.is_empty());
    let (a, b) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut source = channel::configure(&b).unwrap();
    send(
        a.as_raw_fd(),
        b"independent-parent-domain",
        MsgFlags::MSG_NOSIGNAL,
    )
    .unwrap();
    let observed = source.receive(Zeroizing::new(vec![0; 128])).unwrap();
    let expected = observed.namespaces().unwrap();
    observed.close().unwrap();
    let null = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/usr/bin/true").unwrap();
    let args = [CString::new("true").unwrap()];
    let directory = CString::new("/").unwrap();
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let mut child = atomic_spawn::launch(&Spec {
        program: &program,
        arguments: &args,
        environment: &[],
        directory: &directory,
        stdin: null.as_fd(),
        stdout: null.as_fd(),
        stderr: null.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    })
    .unwrap();
    assert!(child.wait().unwrap().success());
    send(
        a.as_raw_fd(),
        b"malformed-private-fixture",
        MsgFlags::MSG_NOSIGNAL,
    )
    .unwrap();
    let error = service
        .exchange_atomic_controller_packet(
            &Context::new(),
            &mut source,
            Zeroizing::new(vec![0; 128]),
            &child,
            &expected,
        )
        .unwrap_err();
    assert_eq!(error.code(), "service-controller-uid");
    assert!(error.known_reply().is_none());
    assert!(error.packet_close.is_none());
    let mut buf = [0; 128];
    assert_eq!(
        recv(a.as_raw_fd(), &mut buf, MsgFlags::MSG_DONTWAIT).unwrap_err(),
        nix::errno::Errno::EAGAIN
    );
    assert!(!format!("{error:?}").contains("malformed-private-fixture"));
    expected.close().unwrap();
    child.close().unwrap();
    service.close().unwrap();
    let mut reopened = open(&i);
    let again = reply_call(
        &mut reopened,
        &Context::new(),
        &i.bootstrap_pin,
        Operation::Initialize,
    );
    assert!(matches!(again.outcome, OutcomeData::Initialized(Err(_))));
    assert_eq!(snapshot(Path::new(&checkpoint)),before);
    reopened.close().unwrap();
}
