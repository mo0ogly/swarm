#![cfg(target_os = "linux")]
mod personal_common;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use personal_common::Fixture;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use swarm_enrollment_authority::{
    credentials,
    personal::{Source, ACCOUNT_ID},
};
use swarm_enrollment_state::{sign_change, Context, Head};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    schema_version: i64,
    base: Head,
    changes: [String; 2],
}
/// Actual signed journal schema, preserving its producer's canonical field order.
#[derive(Serialize, Deserialize)]
struct SignedEvent {
    protocol: String,
    authority_key_id: String,
    project_digest: String,
    sequence: i64,
    previous: String,
    operation: String,
    id: String,
    body: String,
    signature: String,
}
/// Keep exact codec field order for faults, so canonicalization cannot mask the semantic case.
fn encode(saved: &Saved) -> Vec<u8> {
    swarm_wire_json::encode(saved).unwrap()
}
/// Decode the already signed fixture envelope solely to inject a valid second event.
fn second_body(saved: &Saved) -> Vec<u8> {
    let raw = URL_SAFE_NO_PAD.decode(&saved.changes[1]).unwrap();
    let event: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    URL_SAFE_NO_PAD
        .decode(event["body"].as_str().unwrap())
        .unwrap()
}
/// Sign a structurally valid replacement second event with unchanged first-event chain.
fn replace_second(f: &Fixture, saved: &mut Saved, id: &str, body: &[u8]) {
    let first = URL_SAFE_NO_PAD.decode(&saved.changes[0]).unwrap();
    let middle = Head {
        sequence: saved.base.sequence + 1,
        digest: format!("{:x}", Sha256::digest(first)),
    };
    saved.changes[1] =
        URL_SAFE_NO_PAD.encode(sign_change(&f.key, [43; 32], &middle, "enroll", id, body).unwrap());
}
#[test]
fn exact_personal_proposal_restores_after_reopen_without_storage_access_then_replays_once() {
    let f = Fixture::new();
    let proposal = f
        .source
        .prepare(
            &Context::new(),
            &f.head,
            &f.key,
            &f.proof,
            b"PERSONAL_SECRET",
        )
        .unwrap();
    let raw = f
        .source
        .encode_prepared(&Context::new(), &proposal)
        .unwrap();
    let pin = proposal.credential_digest().to_owned();
    let next = proposal.next_head().clone();
    f.source.close().unwrap();
    let source = Source::open(
        &Context::new(),
        &f.path,
        &f.key.verifying_key().to_bytes(),
        [43; 32],
        f.expected.clone(),
    )
    .unwrap();
    let writer = rusqlite::Connection::open(&f.path).unwrap();
    let bound = f.bound();
    writer.execute_batch("BEGIN IMMEDIATE").unwrap();
    let saved: Saved = serde_json::from_slice(&raw).unwrap();
    let signed = [
        URL_SAFE_NO_PAD.decode(&saved.changes[0]).unwrap(),
        URL_SAFE_NO_PAD.decode(&saved.changes[1]).unwrap(),
    ];
    assert_eq!(
        bound
            .preview_batch(&Context::new(), &saved.base, &[&signed[0], &signed[1]])
            .unwrap()
            .predicted_head(),
        &next
    );
    let restored = source.restore_prepared(&Context::new(), &raw).unwrap();
    assert_eq!(restored.credential_digest(), pin);
    assert_eq!(
        source.encode_prepared(&Context::new(), &restored).unwrap(),
        raw
    );
    writer.execute_batch("ROLLBACK").unwrap();
    bound.close().unwrap();
    assert_eq!(
        bound
            .preview_batch(&Context::new(), &saved.base, &[&signed[0], &signed[1]])
            .unwrap()
            .predicted_head(),
        &next
    );
    assert_eq!(source.enroll(&Context::new(), &restored).unwrap(), next);
    assert_eq!(
        source
            .enroll(
                &Context::new(),
                &source.restore_prepared(&Context::new(), &raw).unwrap()
            )
            .unwrap(),
        next
    );
    assert!(source
        .with_authenticated(&Context::new(), &next, &pin, b"PERSONAL_SECRET", |_| Ok::<
            _,
            (),
        >(
            ()
        ))
        .is_ok());
    assert!(!String::from_utf8(raw).unwrap().contains("PERSONAL_SECRET"));
    source.close().unwrap();
}
#[test]
fn personal_proposal_valid_witness_under_writer_then_bad_wire_signature_context_and_semantics_refuse(
) {
    let f = Fixture::new();
    let proposal = f
        .source
        .prepare(&Context::new(), &f.head, &f.key, &f.proof, b"secret")
        .unwrap();
    let raw = f
        .source
        .encode_prepared(&Context::new(), &proposal)
        .unwrap();
    let initial: Saved = serde_json::from_slice(&raw).unwrap();
    assert_eq!(encode(&initial), raw);
    let writer = rusqlite::Connection::open(&f.path).unwrap();
    writer.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(f.source.restore_prepared(&Context::new(), &raw).is_ok());
    let raw_text = String::from_utf8(raw.clone()).unwrap();
    for bad in [
        format!(" {raw_text}"),
        format!("{raw_text}\n"),
        raw_text.replacen("{", "{\"schema_version\":1,", 1),
        raw_text.replacen("{", "{\"authorized\":true,", 1),
    ] {
        assert!(f
            .source
            .restore_prepared(&Context::new(), bad.as_bytes())
            .is_err());
    }
    for variant in 0..9 {
        let mut saved: Saved = serde_json::from_slice(&raw).unwrap();
        match variant {
            0 => saved.schema_version = 2,
            1 => saved.base.digest = "b".repeat(64),
            2 => saved.changes.swap(0, 1),
            3 => saved.changes[1] = "???".into(),
            4 => {
                let body = second_body(&saved);
                replace_second(&f, &mut saved, "wrong-account", &body);
            }
            5 => {
                let mut record = credentials::decode(&second_body(&saved)).unwrap();
                record.principal.subject = "substituted".into();
                replace_second(
                    &f,
                    &mut saved,
                    ACCOUNT_ID,
                    &credentials::encode(&record).unwrap(),
                );
            }
            6 => {
                let mut signed: SignedEvent =
                    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&saved.changes[1]).unwrap())
                        .unwrap();
                signed.signature = "A".repeat(86);
                saved.changes[1] =
                    URL_SAFE_NO_PAD.encode(swarm_wire_json::encode(&signed).unwrap());
            }
            7 => {
                let body = second_body(&saved);
                saved.changes[0] = URL_SAFE_NO_PAD.encode(
                    sign_change(
                        &f.key,
                        [43; 32],
                        &saved.base,
                        "enroll",
                        "other-resource",
                        b"opaque",
                    )
                    .unwrap(),
                );
                replace_second(&f, &mut saved, ACCOUNT_ID, &body);
            }
            _ => {
                let body = second_body(&saved);
                let first = URL_SAFE_NO_PAD.decode(&saved.changes[0]).unwrap();
                let middle = Head {
                    sequence: saved.base.sequence + 1,
                    digest: format!("{:x}", Sha256::digest(first)),
                };
                saved.changes[1] = URL_SAFE_NO_PAD.encode(
                    sign_change(&f.key, [99; 32], &middle, "enroll", ACCOUNT_ID, &body).unwrap(),
                );
            }
        };
        let error = f
            .source
            .restore_prepared(&Context::new(), &encode(&saved))
            .unwrap_err();
        let expected = [
            "personal-proposal",
            "head",
            "head",
            "personal-encoding",
            "personal-proposal",
            "personal-binding",
            "change",
            "personal-proposal",
            "context",
        ][variant];
        assert_eq!(error.code(), expected, "variant {variant}");
    }
    assert!(f.source.restore_prepared(&Context::new(), &raw).is_ok());
    writer.execute_batch("ROLLBACK").unwrap();
    assert!(f.source.enroll(&Context::new(), &proposal).is_ok());
    f.source.close().unwrap();
}
#[test]
fn personal_proposal_another_installation_and_precanceled_preview_refuse_then_recover() {
    let f = Fixture::new();
    let p = f
        .source
        .prepare(&Context::new(), &f.head, &f.key, &f.proof, b"secret")
        .unwrap();
    let raw = f.source.encode_prepared(&Context::new(), &p).unwrap();
    for change_installation in [false, true] {
        let mut expected = f.expected.clone();
        if change_installation {
            expected.installation_digest = "e".repeat(64);
        } else {
            expected.generation += 1;
        }
        let other = Source::open(
            &Context::new(),
            &f.path,
            &f.key.verifying_key().to_bytes(),
            [43; 32],
            expected,
        )
        .unwrap();
        assert_eq!(
            other
                .restore_prepared(&Context::new(), &raw)
                .unwrap_err()
                .code(),
            "personal-binding"
        );
        assert_eq!(
            other
                .encode_prepared(&Context::new(), &p)
                .unwrap_err()
                .code(),
            "personal-binding"
        );
        other.close().unwrap();
    }
    let canceled = Context::new();
    canceled.cancel();
    assert!(f.source.restore_prepared(&canceled, &raw).is_err());
    assert!(f.source.restore_prepared(&Context::new(), &raw).is_ok());
    f.source.close().unwrap();
}
