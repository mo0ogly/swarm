//! Service-only handoff data, never an installed service or human authority.
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use swarm_enrollment_authority::provisioning::{self as p, service_artifact as s};
use swarm_enrollment_bootstrap as b;
use zeroize::Zeroizing;
/// Explicit synthetic request; same producer as prior operator-package cases.
fn material() -> p::Prepared {
    let r: p::Request = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/paea_provisioning_request.json"
    ))
    .unwrap();
    p::prepare(&r).unwrap()
}
/// Data digest only; never authenticate a pin learned from the tested bytes.
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
/// Independent fixture encoder of the public binary format, no live secret.
fn frame(
    seed: &[u8],
    doc: &b::AuthenticatedDocument,
    business: &[u8],
    personal: &[u8],
) -> Zeroizing<Vec<u8>> {
    let document = b::encode_authenticated(doc).unwrap();
    let capacity = b"swarm-service-installation-v1\n".len()
        + 32
        + 24
        + document.len()
        + business.len()
        + personal.len();
    let mut raw = Zeroizing::new(Vec::with_capacity(capacity));
    raw.extend_from_slice(b"swarm-service-installation-v1\n");
    raw.extend_from_slice(seed);
    for part in [document.as_slice(), business, personal] {
        raw.extend_from_slice(&(part.len() as u64).to_be_bytes());
        raw.extend_from_slice(part)
    }
    raw
}
/// Guard only the synthetic personal seed delivered by the service artifact.
fn seed(artifact: &s::ServiceArtifact) -> Zeroizing<Vec<u8>> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(32));
    artifact.write_personal_seed(&mut *bytes).unwrap();
    bytes
}
/// Exact valid codec witness before each framing/binding fault.
fn valid() {
    let a = s::from_prepared(&material()).unwrap();
    let raw = a.encode_private().unwrap();
    let read = s::decode(&raw, &a.pin().unwrap()).unwrap();
    assert!(read.encode_private().unwrap().as_slice() == raw.as_slice());
}

/// Only personal seed is encoded; operator business seed and proof fields absent.
#[test]
fn service_artifact_exact_roundtrip_excludes_operator_secrets_and_redacts() {
    let prepared = material();
    let operator = prepared.encode_private().unwrap();
    let offset = b"swarm-initial-provisioning-v1\n".len();
    let business_seed = &operator[offset..offset + 32];
    let proof = &operator[offset + 64..offset + 96];
    let artifact = s::from_prepared(&prepared).unwrap();
    let raw = artifact.encode_private().unwrap();
    assert!(raw
        .windows(32)
        .all(|part| part != business_seed && part != proof));
    let decoded = s::decode(&raw, &artifact.pin().unwrap()).unwrap();
    assert!(decoded.encode_private().unwrap().as_slice() == raw.as_slice());
    assert_eq!(artifact.document(), decoded.document());
    assert_eq!(artifact.root_directory(), "/synthetic-initial-service");
    assert_eq!(
        format!("{artifact:?}"),
        "ServiceInstallationArtifact { <private> }"
    );
    let bytes = seed(&decoded);
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let key = ed25519_dalek::SigningKey::from_bytes(bytes.as_slice().try_into().unwrap());
    assert_eq!(
        URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes()),
        artifact.document().personal.authority_public_key
    );
}

/// Wrong pin and malformed lengths/header/EOF never fall back to partial data.
#[test]
fn service_artifact_pin_header_lengths_truncation_and_trailing_refuse() {
    valid();
    let a = s::from_prepared(&material()).unwrap();
    let raw = a.encode_private().unwrap();
    assert!(s::decode(&raw, &"00".repeat(32)).is_err());
    assert!(s::decode(&raw, "invalid").is_err());
    for length in [0, 10, raw.len() - 1] {
        valid();
        let part = &raw[..length];
        assert!(s::decode(part, &hash(part)).is_err());
    }
    for variant in 0..3 {
        valid();
        let mut bad = Zeroizing::new(raw.to_vec());
        match variant {
            0 => bad[0] ^= 1,
            1 => {
                let start = b"swarm-service-installation-v1\n".len() + 32;
                bad[start..start + 8].copy_from_slice(&u64::MAX.to_be_bytes())
            }
            _ => bad.push(0),
        }
        assert!(s::decode(&bad, &hash(&bad)).is_err());
    }
}

/// Independently pinned but inconsistent seed/layout/head refuses as data binding.
#[test]
fn service_artifact_seed_layout_generation_and_head_bindings_refuse() {
    let a = s::from_prepared(&material()).unwrap();
    let original_seed = seed(&a);
    for variant in 0..4 {
        valid();
        let mut key = Zeroizing::new(original_seed.to_vec());
        let mut doc = a.document().clone();
        match variant {
            0 => key[0] ^= 1,
            1 => {
                doc.control.installation.configuration_directory =
                    "/synthetic-initial-service/elsewhere".into()
            }
            2 => doc.control.generation = 2,
            _ => doc.personal.bootstrap_head.digest = "00".repeat(32),
        };
        if variant == 2 {
            doc.control.authority_head.sequence = 2
        }
        let raw = frame(&key, &doc, a.business_initial(), a.personal_initial());
        assert!(s::decode(&raw, &hash(&raw)).is_err());
    }
}

/// Actual signature checks belong to the journal writer. After a refused event,
/// applying the supplied valid event at base zero proves same-journal recovery.
fn apply_real(artifact: &s::ServiceArtifact, recovery: Option<&[u8]>) {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use swarm_enrollment_state::{Context, Head, Journal};
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let key = URL_SAFE_NO_PAD
        .decode(&artifact.document().control.installation.pins.public_key)
        .unwrap();
    let journal = Journal::open(
        &Context::new(),
        d.path().join("business.sqlite"),
        &key,
        [0x12; 32],
    )
    .unwrap();
    let result = journal.apply(
        &Context::new(),
        &Head::default(),
        artifact.business_initial(),
    );
    if let Some(valid) = recovery {
        assert!(result.is_err());
        let recovered = journal
            .apply(&Context::new(), &Head::default(), valid)
            .unwrap();
        assert_eq!(recovered.sequence, 1);
        assert_eq!(recovered.digest, hash(valid));
    } else {
        assert_eq!(result.unwrap(), artifact.document().control.authority_head)
    }
    journal.close().unwrap();
    if recovery.is_none() {
        let key = URL_SAFE_NO_PAD
            .decode(&artifact.document().personal.authority_public_key)
            .unwrap();
        let journal = Journal::open(
            &Context::new(),
            d.path().join("personal.sqlite"),
            &key,
            [0xcd; 32],
        )
        .unwrap();
        assert_eq!(
            journal
                .apply(
                    &Context::new(),
                    &Head::default(),
                    artifact.personal_initial()
                )
                .unwrap(),
            artifact.document().personal.bootstrap_head
        );
        journal.close().unwrap();
    }
}
/// Valid handoff events produce their exact expected heads in real isolated DBs.
#[test]
fn service_artifact_generated_initial_events_apply_to_real_journals() {
    let a = s::from_prepared(&material()).unwrap();
    apply_real(&a, None);
}

/// A digest binds opaque bytes, not their signature: an intentionally bad event
/// can decode as data but the authoritative writer refuses before its commit.
#[test]
fn service_artifact_codec_does_not_claim_signature_authority_real_journal_refuses() {
    let a = s::from_prepared(&material()).unwrap();
    apply_real(&a, None);
    let mut business = a.business_initial().to_vec();
    let needle = b"\"signature\":\"";
    let at = business
        .windows(needle.len())
        .position(|v| v == needle)
        .unwrap()
        + needle.len();
    business[at] = if business[at] == b'A' { b'B' } else { b'A' };
    let mut doc = a.document().clone();
    doc.control.authority_head.digest = hash(&business);
    let raw = frame(&seed(&a), &doc, &business, a.personal_initial());
    let data = s::decode(&raw, &hash(&raw)).unwrap();
    apply_real(&data, Some(a.business_initial()));
}

/// Personal seed delivery preserves original write/flush phase without raw text.
#[test]
fn service_artifact_private_seed_sink_error_retains_typed_phase() {
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "PRIVATE_SEED_SINK",
            ))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    valid();
    let a = s::from_prepared(&material()).unwrap();
    let e = a.write_personal_seed(&mut Broken).unwrap_err();
    assert!(!format!("{e:?} {e}").contains("PRIVATE_SEED_SINK"));
    let source = std::error::Error::source(&e)
        .unwrap()
        .downcast_ref::<p::Error>()
        .unwrap();
    assert_eq!(source.output_phase(), Some(p::OutputPhase::WriteUnknown));
}
