use swarm_enrollment_authority::credentials::{self as c, Parameters};
use swarm_identity_contract::Principal;
fn principal() -> Principal {
    Principal {
        id: "human-local-1".into(),
        kind: "human".into(),
        issuer: "urn:swarm:personal:test-installation".into(),
        subject: "stable-subject-1".into(),
    }
}
// Small explicit algorithm-valid fixture costs, never production configuration.
fn costs() -> Parameters {
    Parameters {
        memory_kib: 32,
        passes: 1,
        parallelism: 1,
    }
}
#[test]
fn credential_real_argon2_roundtrip_proves_only_exact_password_record() {
    let p = principal();
    let installation = "a".repeat(64);
    let record = c::create(&installation, &p, 1, costs(), b"PERSONAL_TEST_PASSWORD").unwrap();
    let raw = c::encode(&record).unwrap();
    let parsed = c::decode(&raw).unwrap();
    assert_eq!(c::encode(&parsed).unwrap(), raw);
    let pin = c::fingerprint(&parsed).unwrap();
    let proof = c::verify(
        &installation,
        &p,
        1,
        &pin,
        &parsed,
        b"PERSONAL_TEST_PASSWORD",
    )
    .unwrap();
    assert_eq!(proof.principal(), &p);
    assert_eq!(proof.generation(), 1);
    assert_eq!(proof.record_digest(), pin);
    assert_eq!(
        c::verify(&installation, &p, 1, &pin, &parsed, b"wrong")
            .unwrap_err()
            .code(),
        "password"
    );
    assert!(c::verify(
        &installation,
        &p,
        1,
        &pin,
        &parsed,
        b"PERSONAL_TEST_PASSWORD"
    )
    .is_ok());
    let debug = format!("{record:?} {proof:?}");
    for secret in [
        "PERSONAL_TEST_PASSWORD",
        record.salt.as_str(),
        record.derived.as_str(),
    ] {
        assert!(!debug.contains(secret));
    }
}
#[test]
fn credential_independent_pin_context_principal_generation_refuse_before_hashing() {
    let p = principal();
    let id = "b".repeat(64);
    let r = c::create(&id, &p, 1, costs(), b"secret").unwrap();
    let pin = c::fingerprint(&r).unwrap();
    for variant in 0..5 {
        let mut q = p.clone();
        let mut got = r.clone();
        let (mut expected_id, mut generation, mut expected_pin) = (id.clone(), 1, pin.clone());
        match variant {
            0 => expected_id = "c".repeat(64),
            1 => q.subject = "another-subject".into(),
            2 => generation = 2,
            3 => expected_pin = "f".repeat(64),
            _ => got.derived = "AA".into(),
        };
        let error =
            c::verify(&expected_id, &q, generation, &expected_pin, &got, b"wrong").unwrap_err();
        assert_ne!(error.code(), "password", "variant {variant}");
    }
    assert!(c::verify(&id, &p, 1, &pin, &r, b"secret").is_ok());
}
#[test]
fn credential_strict_wire_invalid_crypto_and_empty_secret_refuse_without_fallback() {
    let p = principal();
    let r = c::create(&"d".repeat(64), &p, 1, costs(), b"secret").unwrap();
    let raw = String::from_utf8(c::encode(&r).unwrap()).unwrap();
    for got in [
        format!(" {raw}"),
        format!("{raw}\n"),
        raw.replacen("{", "{\"schema_version\":1,", 1),
        raw.replacen("{", "{\"administrator\":true,", 1),
        raw.replacen("argon2id-v19", "argon2i-v19", 1),
        raw.replace("\"memory_kib\":32", "\"memory_kib\":null"),
    ] {
        assert!(c::decode(got.as_bytes()).is_err());
    }
    for parameters in [
        Parameters {
            memory_kib: 0,
            ..costs()
        },
        Parameters {
            passes: 0,
            ..costs()
        },
        Parameters {
            parallelism: 0,
            ..costs()
        },
        Parameters {
            memory_kib: 8,
            passes: 1,
            parallelism: 2,
        },
    ] {
        assert!(parameters.validate().is_err());
        assert!(c::create(&"d".repeat(64), &p, 1, parameters, b"secret").is_err());
    }
    assert!(c::create(&"d".repeat(64), &p, 1, costs(), b"").is_err());
    let mut not_human = p.clone();
    not_human.kind = "service".into();
    assert!(c::create(&"d".repeat(64), &not_human, 1, costs(), b"secret").is_err());
    for variant in 0..7 {
        let mut got = r.clone();
        match variant {
            0 => got.schema_version = 2,
            1 => got.generation = 0,
            2 => got.installation_digest = "D".repeat(64),
            3 => got.salt = "a".into(),
            4 => got.derived = "a".into(),
            5 => got.salt.push('='),
            _ => got.algorithm = "argon2i-v19".into(),
        };
        assert!(c::encode(&got).is_err());
    }
}
#[test]
fn credential_os_salts_separate_identical_secrets_and_digest_domain_is_closed() {
    let p = principal();
    let a = c::create(&"e".repeat(64), &p, 1, costs(), b"secret").unwrap();
    let b = c::create(&"e".repeat(64), &p, 1, costs(), b"secret").unwrap();
    assert_ne!(a.salt, b.salt);
    assert_ne!(a.derived, b.derived);
    assert_ne!(c::fingerprint(&a).unwrap(), c::fingerprint(&b).unwrap());
    for r in [&a, &b] {
        let pin = c::fingerprint(r).unwrap();
        assert!(c::verify(&"e".repeat(64), &p, 1, &pin, r, b"secret").is_ok());
    }
}

#[test]
fn credential_explicit_documented_costs_roundtrip_and_preserve_typed_causes() {
    // RustCrypto Book example costs: a test input, not a Swarm default/policy.
    let parameters = Parameters {
        memory_kib: 19456,
        passes: 2,
        parallelism: 1,
    };
    let p = principal();
    let record = c::create(&"f".repeat(64), &p, 2, parameters, b"TEST_ONLY_PASSWORD").unwrap();
    let parsed = c::decode(&c::encode(&record).unwrap()).unwrap();
    assert_eq!(parsed.parameters, parameters);
    let pin = c::fingerprint(&parsed).unwrap();
    assert!(c::verify(&"f".repeat(64), &p, 2, &pin, &parsed, b"TEST_ONLY_PASSWORD").is_ok());
    let error = Parameters {
        memory_kib: 0,
        ..parameters
    }
    .validate()
    .unwrap_err();
    assert!(std::error::Error::source(&error)
        .unwrap()
        .is::<argon2::Error>());
    let error = c::decode(b"{\"PRIVATE_SECRET\":").unwrap_err();
    assert!(std::error::Error::source(&error)
        .unwrap()
        .is::<serde_json::Error>());
    assert!(!format!("{error} {error:?}").contains("PRIVATE_SECRET"));
}

#[test]
fn credential_malformed_base64_retains_original_typed_cause_with_redacted_diagnostics() {
    let mut record = c::create(
        &"a".repeat(64),
        &principal(),
        1,
        costs(),
        b"PERSONAL_PASSWORD",
    )
    .unwrap();
    record.salt = "PRIVATE_BAD_???".into();
    let error = c::encode(&record).unwrap_err();
    assert_eq!(error.code(), "credential-encoding");
    assert!(std::error::Error::source(&error)
        .unwrap()
        .is::<base64::DecodeError>());
    assert!(!format!("{error:?} {error}").contains("PRIVATE_BAD"));
}
