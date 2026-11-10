#![cfg(target_os = "linux")]
use ed25519_dalek::SigningKey;
use swarm_enrollment_authority::{
    credentials::Parameters,
    personal::{self as p, Bootstrap, Expected, Source, UseError, ACCOUNT_ID, BOOTSTRAP_ID},
};
use swarm_enrollment_state::{sign_change, Context};
mod personal_common;
use personal_common::Fixture;
#[test]
fn personal_bootstrap_secret_is_consumed_atomically_exact_retry_and_password_use_after_reopen() {
    let f = Fixture::new();
    let prepared = f
        .source
        .prepare(
            &Context::new(),
            &f.head,
            &f.key,
            &f.proof,
            b"PERSONAL_FIXTURE_SECRET",
        )
        .unwrap();
    assert_eq!(
        f.source.enroll(&Context::new(), &prepared).unwrap(),
        *prepared.next_head()
    );
    assert_eq!(
        f.source.enroll(&Context::new(), &prepared).unwrap(),
        *prepared.next_head()
    );
    assert!(f
        .source
        .prepare(
            &Context::new(),
            prepared.next_head(),
            &f.key,
            &f.proof,
            b"other"
        )
        .is_err());
    let j = f.bound();
    assert_eq!(
        j.with_active(&Context::new(), prepared.next_head(), BOOTSTRAP_ID, |_| Ok(
            ()
        ))
        .unwrap_err()
        .code(),
        "revoked"
    );
    j.close().unwrap();
    let principal = f
        .source
        .with_authenticated(
            &Context::new(),
            prepared.next_head(),
            prepared.credential_digest(),
            b"PERSONAL_FIXTURE_SECRET",
            |proof| Ok::<_, ()>(proof.principal().clone()),
        )
        .unwrap()
        .unwrap();
    assert_eq!(principal, f.expected.principal);
    f.source.close().unwrap();
    let source = Source::open(
        &Context::new(),
        &f.path,
        &f.key.verifying_key().to_bytes(),
        [43; 32],
        f.expected.clone(),
    )
    .unwrap();
    assert!(source
        .with_authenticated(
            &Context::new(),
            prepared.next_head(),
            prepared.credential_digest(),
            b"PERSONAL_FIXTURE_SECRET",
            |_| Ok::<_, ()>(())
        )
        .is_ok());
    source.close().unwrap();
}
#[test]
fn personal_bad_proof_context_key_password_pin_and_stale_head_refuse_without_callback_then_recover()
{
    let f = Fixture::new();
    assert!(f
        .source
        .prepare(&Context::new(), &f.head, &f.key, &[0; 32], b"secret")
        .is_err());
    assert!(f
        .source
        .prepare(
            &Context::new(),
            &f.head,
            &SigningKey::from_bytes(&[44; 32]),
            &f.proof,
            b"secret"
        )
        .is_err());
    let prepared = f
        .source
        .prepare(&Context::new(), &f.head, &f.key, &f.proof, b"secret")
        .unwrap();
    let conflicting = f
        .source
        .prepare(
            &Context::new(),
            &f.head,
            &f.key,
            &f.proof,
            b"another-secret",
        )
        .unwrap();
    f.source.enroll(&Context::new(), &prepared).unwrap();
    assert!(f.source.enroll(&Context::new(), &conflicting).is_err());
    for (head, pin, password) in [
        (&f.head, prepared.credential_digest(), b"secret".as_slice()),
        (prepared.next_head(), "f", b"secret".as_slice()),
        (
            prepared.next_head(),
            prepared.credential_digest(),
            b"wrong".as_slice(),
        ),
    ] {
        let mut called = false;
        assert!(f
            .source
            .with_authenticated(&Context::new(), head, pin, password, |_| {
                called = true;
                Ok::<_, ()>(())
            })
            .is_err());
        assert!(!called);
    }
    let mut expected = f.expected.clone();
    expected.principal.subject = "substituted".into();
    let other = Source::open(
        &Context::new(),
        &f.path,
        &f.key.verifying_key().to_bytes(),
        [43; 32],
        expected,
    )
    .unwrap();
    assert!(other
        .with_authenticated(
            &Context::new(),
            prepared.next_head(),
            prepared.credential_digest(),
            b"secret",
            |_| Ok::<_, ()>(())
        )
        .is_err());
    other.close().unwrap();
    assert!(f
        .source
        .with_authenticated(
            &Context::new(),
            prepared.next_head(),
            prepared.credential_digest(),
            b"secret",
            |_| Ok::<_, ()>(())
        )
        .is_ok());
    f.source.close().unwrap();
}
#[test]
fn personal_account_reservation_holds_through_usage_then_explicit_revocation_prevents_later_callback(
) {
    let f = Fixture::new();
    let prepared = f
        .source
        .prepare(&Context::new(), &f.head, &f.key, &f.proof, b"secret")
        .unwrap();
    f.source.enroll(&Context::new(), &prepared).unwrap();
    let other = f.bound();
    let revoke = sign_change(
        &f.key,
        [43; 32],
        prepared.next_head(),
        "revoke",
        ACCOUNT_ID,
        b"",
    )
    .unwrap();
    let result = f
        .source
        .with_authenticated(
            &Context::new(),
            prepared.next_head(),
            prepared.credential_digest(),
            b"secret",
            |_| {
                assert!(other
                    .apply_batch(&Context::new(), prepared.next_head(), &[&revoke])
                    .unwrap_err()
                    .is_busy());
                Ok::<_, ()>("observed effect")
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(result, "observed effect");
    let revoked = other
        .apply_batch(&Context::new(), prepared.next_head(), &[&revoke])
        .unwrap();
    let mut called = false;
    assert!(f
        .source
        .with_authenticated(
            &Context::new(),
            &revoked,
            prepared.credential_digest(),
            b"secret",
            |_| {
                called = true;
                Ok::<_, ()>(())
            }
        )
        .is_err());
    assert!(!called);
    assert!(f
        .source
        .prepare(&Context::new(), &revoked, &f.key, &f.proof, b"new")
        .is_err());
    other.close().unwrap();
    f.source.close().unwrap();
}
#[test]
fn personal_post_usage_cancellation_retains_success_or_failure_outcome_without_relabeling_effect() {
    for failed in [false, true] {
        let f = Fixture::new();
        let prepared = f
            .source
            .prepare(&Context::new(), &f.head, &f.key, &f.proof, b"secret")
            .unwrap();
        f.source.enroll(&Context::new(), &prepared).unwrap();
        let ctx = Context::new();
        let marker = f._dir.path().join("external-effect");
        let result = f
            .source
            .with_authenticated(
                &ctx,
                prepared.next_head(),
                prepared.credential_digest(),
                b"secret",
                |_| {
                    std::fs::write(&marker, b"DONE").unwrap();
                    ctx.cancel();
                    if failed {
                        Err("PRIVATE_CALLBACK_ERROR")
                    } else {
                        Ok("PRIVATE_CALLBACK_SUCCESS")
                    }
                },
            )
            .unwrap_err();
        assert_eq!(std::fs::read(marker).unwrap(), b"DONE");
        assert!(!format!("{result:?} {result}").contains("PRIVATE_CALLBACK"));
        match result {
            UseError::AfterUse { cause, outcome } => {
                assert_eq!(cause.code(), "canceled");
                assert_eq!(outcome.is_err(), failed);
            }
            _ => panic!("usage outcome lost"),
        };
        f.source.close().unwrap();
    }
}

#[test]
fn personal_bootstrap_wire_and_installation_binding_refuse_without_consuming_pending_source() {
    let f = Fixture::new();
    let b = Bootstrap {
        schema_version: 1,
        installation_digest: f.expected.installation_digest.clone(),
        principal: f.expected.principal.clone(),
        generation: 1,
        parameters: Parameters {
            memory_kib: 32,
            passes: 1,
            parallelism: 1,
        },
        proof_commitment: p::commitment(&f.expected.installation_digest, &f.proof).unwrap(),
    };
    let raw = String::from_utf8(p::encode_bootstrap(&b).unwrap()).unwrap();
    for got in [
        format!(" {raw}"),
        format!("{raw}\n"),
        raw.replacen("{", "{\"schema_version\":1,", 1),
        raw.replacen("{", "{\"human_verified\":true,", 1),
        raw.replace("\"schema_version\":1", "\"schema_version\":null"),
    ] {
        assert!(p::decode_bootstrap(got.as_bytes()).is_err());
    }
    let mut expected = f.expected.clone();
    expected.installation_digest = "b".repeat(64);
    let other = Source::open(
        &Context::new(),
        &f.path,
        &f.key.verifying_key().to_bytes(),
        [43; 32],
        expected,
    )
    .unwrap();
    assert!(other
        .prepare(&Context::new(), &f.head, &f.key, &f.proof, b"secret")
        .is_err());
    other.close().unwrap();
    let mut bad = b.clone();
    bad.parameters.passes = 0;
    assert!(p::encode_bootstrap(&bad).is_err());
    assert!(p::commitment("not-a-digest", &f.proof).is_err());
    let prepared = f
        .source
        .prepare(&Context::new(), &f.head, &f.key, &f.proof, b"secret")
        .unwrap();
    assert_eq!(
        f.source.enroll(&Context::new(), &prepared).unwrap(),
        *prepared.next_head()
    );
    f.source.close().unwrap();
}
