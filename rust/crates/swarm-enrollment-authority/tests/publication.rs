mod common;
use common::document;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use swarm_enrollment_authority::publication::{
    decode, prepare, Enrollment, Publication, PUBLICATION_PURPOSE,
};
use swarm_enrollment_authority::{encode, sign_enrollment};
use swarm_enrollment_proof::Verifier;
use swarm_enrollment_state::{Context, Head, Journal};
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn publication() -> Publication {
    let d = document();
    let snapshot = b"{\"agents\":[]}".to_vec();
    Publication {
        schema_version: 1,
        purpose: PUBLICATION_PURPOSE.into(),
        project_digest: d.project_digest.clone(),
        principal: d.principal.clone(),
        session_digest: "e".repeat(64),
        session_verified_at: d.not_before,
        session_expires_at: d.expires_at,
        scope: d.scope.clone(),
        trust_revision: d.trust_revision,
        enrollment: Enrollment {
            id: d.enrollment_id,
            generation: 1,
            head: Head {
                sequence: 1,
                digest: "c".repeat(64),
            },
            document_digest: hash(&encode(&document()).unwrap()),
        },
        event_id: "event-1".into(),
        snapshot_revision: 1,
        snapshot_digest: hash(&snapshot),
        snapshot,
        root_grants: vec!["root-2".into(), "root-1".into()],
        previous_registry_head_digest: String::new(),
    }
}
#[test]
fn complete_descriptor_digest_and_owned_bytes_preserve_root_order() {
    let mut p = publication();
    let q = prepare(&p).unwrap();
    let bytes = q.bytes();
    let mut preimage = b"swarm-enrollment-publication-request-v1\0request\0".to_vec();
    preimage.extend(&bytes);
    assert_eq!(q.digest().to_vec(), Sha256::digest(preimage).to_vec());
    assert_eq!(q.publication().unwrap(), p);
    assert_eq!(
        q.expected().unwrap().request_digest,
        hash(
            &[
                b"swarm-enrollment-publication-request-v1\0request\0".as_slice(),
                &bytes
            ]
            .concat()
        )
    );
    p.snapshot[0] = b'!';
    let mut owned = q.publication().unwrap();
    owned.root_grants.reverse();
    owned.snapshot[0] = b'!';
    assert_eq!(q.bytes(), bytes);
    assert_eq!(
        q.publication().unwrap().root_grants,
        vec!["root-2", "root-1"]
    );
    assert_eq!(decode(&bytes).unwrap().digest(), q.digest());
}
#[test]
fn invalid_descriptor_dimensions_and_noop_snapshot_are_rejected() {
    for kind in [
        "schema",
        "purpose",
        "project",
        "principal",
        "session",
        "zero-start",
        "inverted",
        "scope",
        "trust",
        "enrollment",
        "generation",
        "head",
        "document",
        "event",
        "revision",
        "digest",
        "digest-mismatch",
        "empty",
        "array",
        "trailing",
        "no-roots",
        "duplicate-root",
        "invalid-root",
        "previous",
    ] {
        let mut p = publication();
        prepare(&p).unwrap();
        match kind {
            "schema" => p.schema_version = 2,
            "purpose" => p.purpose = "other".into(),
            "project" => p.project_digest = "bad".into(),
            "principal" => p.principal.kind = "service".into(),
            "session" => p.session_digest = "bad".into(),
            "zero-start" => {
                p.session_verified_at =
                    time::OffsetDateTime::from_unix_timestamp(-62135596800).unwrap()
            }
            "inverted" => p.session_expires_at = p.session_verified_at,
            "scope" => p.scope.plan_version = 0,
            "trust" => p.trust_revision = 0,
            "enrollment" => p.enrollment.id.clear(),
            "generation" => p.enrollment.generation = 2,
            "head" => p.enrollment.head = Head::default(),
            "document" => p.enrollment.document_digest.clear(),
            "event" => p.event_id = "../x".into(),
            "revision" => p.snapshot_revision = 0,
            "digest" => p.snapshot_digest = "bad".into(),
            "digest-mismatch" => p.snapshot.push(b' '),
            "empty" => {
                p.snapshot.clear();
                p.snapshot_digest = hash(&p.snapshot)
            }
            "array" => {
                p.snapshot = b"[]".to_vec();
                p.snapshot_digest = hash(&p.snapshot)
            }
            "trailing" => {
                p.snapshot = b"{}{}".to_vec();
                p.snapshot_digest = hash(&p.snapshot)
            }
            "no-roots" => p.root_grants.clear(),
            "duplicate-root" => p.root_grants.push(p.root_grants[0].clone()),
            "invalid-root" => p.root_grants[0] = "../root".into(),
            "previous" => p.previous_registry_head_digest = "bad".into(),
            _ => unreachable!(),
        };
        assert_eq!(prepare(&p).unwrap_err().code(), "request", "{kind}");
    }
}
#[test]
fn opaque_snapshot_retains_go_json_syntax_and_all_original_bytes() {
    for snapshot in [
        b"{\"n\":1e1000000}".to_vec(),
        vec![b'{', b'"', b'x', b'"', b':', b'"', 255, b'"', b'}'],
        "\u{a0}{\"x\":1}\u{a0}".as_bytes().to_vec(),
        b"{\"x\":1,\"x\":2}".to_vec(),
        b"{\"x\":\"\\ud800\"}".to_vec(),
    ] {
        let mut p = publication();
        p.snapshot = snapshot.clone();
        p.snapshot_digest = hash(&snapshot);
        let q = prepare(&p).unwrap();
        assert_eq!(q.publication().unwrap().snapshot, snapshot);
        assert_eq!(decode(&q.bytes()).unwrap().bytes(), q.bytes());
    }
}
#[test]
fn snapshot_depth_is_go_json_representation_not_an_operator_policy() {
    let mut p = publication();
    p.snapshot = [
        b"{".as_slice(),
        b"\"x\":",
        &b"[".repeat(9999),
        b"0",
        &b"]".repeat(9999),
        b"}",
    ]
    .concat();
    p.snapshot_digest = hash(&p.snapshot);
    prepare(&p).unwrap();
    p.snapshot = [
        b"{".as_slice(),
        b"\"x\":",
        &b"[".repeat(10000),
        b"0",
        &b"]".repeat(10000),
        b"}",
    ]
    .concat();
    p.snapshot_digest = hash(&p.snapshot);
    assert_eq!(prepare(&p).unwrap_err().code(), "request");
}
#[test]
fn canonical_frame_and_every_descriptor_byte_affect_the_binding() {
    let p = publication();
    let q = prepare(&p).unwrap();
    let raw = String::from_utf8(q.bytes()).unwrap();
    for bad in [
        format!(" {raw}"),
        format!("{raw}{{}}"),
        raw.replacen('{', "{\"extra\":true,", 1),
        raw.replace("\"principal_id\"", "\"id\""),
        raw.replace("\"root_grants\"", "\"Root_Grants\""),
    ] {
        decode(&q.bytes()).unwrap();
        assert_eq!(decode(bad.as_bytes()).unwrap_err().code(), "request");
    }
    let mut changed = p.clone();
    changed.root_grants.reverse();
    assert_ne!(prepare(&changed).unwrap().digest(), q.digest());
    changed = p.clone();
    changed.snapshot = b"{ \"agents\":[]}".to_vec();
    changed.snapshot_digest = hash(&changed.snapshot);
    assert_ne!(prepare(&changed).unwrap().digest(), q.digest());
}
#[test]
fn challenge_binding_is_data_only_and_keeps_cryptography_separate() {
    let p = publication();
    let q = prepare(&p).unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    let v = Verifier::new(&key.verifying_key().to_bytes(), [11; 32]).unwrap();
    let ch = q.new_challenge(&v).unwrap();
    let mut request = ch.request();
    q.bind(&request).unwrap();
    request.protocol = "malformed".into();
    q.bind(&request).unwrap();
    assert!(swarm_enrollment_proof::sign_response(&request, b"opaque", &key).is_err());
    request = ch.request();
    request.request_digest = "a".repeat(64);
    assert_eq!(q.bind(&request).unwrap_err().code(), "binding");
    let other = Verifier::new(&key.verifying_key().to_bytes(), [12; 32]).unwrap();
    assert_eq!(q.new_challenge(&other).err().unwrap().code(), "binding");
}
#[test]
fn authenticated_consumer_requires_exact_document_and_explicit_session() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    let doc = document();
    let j = Journal::open(
        &Context::new(),
        dir.path().join("journal.sqlite"),
        &key.verifying_key().to_bytes(),
        [11; 32],
    )
    .unwrap();
    let raw = sign_enrollment(&key, [11; 32], &Head::default(), &doc).unwrap();
    let head = j.apply(&Context::new(), &Head::default(), &raw).unwrap();
    let mut p = publication();
    p.enrollment.head = head;
    let q = prepare(&p).unwrap();
    let v = Verifier::new(&key.verifying_key().to_bytes(), [11; 32]).unwrap();
    let ch = q.new_challenge(&v).unwrap();
    let raw = j
        .sign_active(
            &Context::new(),
            &p.enrollment.head,
            &doc.enrollment_id,
            ch.request(),
            &key,
        )
        .unwrap();
    let s = v.verify(&ch, &raw).unwrap();
    assert_eq!(q.check_active(&s, doc.not_before).unwrap(), doc);
    assert_eq!(
        q.check_active(&s, p.session_expires_at).unwrap_err().code(),
        "session"
    );
    let mut wrong = p.clone();
    wrong.enrollment.document_digest = "a".repeat(64);
    let wrong = prepare(&wrong).unwrap();
    let ch = wrong.new_challenge(&v).unwrap();
    let raw = j
        .sign_active(
            &Context::new(),
            &p.enrollment.head,
            &doc.enrollment_id,
            ch.request(),
            &key,
        )
        .unwrap();
    let s = v.verify(&ch, &raw).unwrap();
    assert_eq!(
        wrong.check_active(&s, doc.not_before).unwrap_err().code(),
        "binding"
    );
}

#[test]
fn prepared_clones_are_immutable_across_concurrent_challenges_and_debug_is_safe() {
    let q = prepare(&publication()).unwrap();
    let expected = q.expected().unwrap();
    let debug = format!("{q:?}");
    assert!(debug.contains(&expected.request_digest));
    assert!(!debug.contains("agents"));
    let key = SigningKey::from_bytes(&[7; 32]);
    let v = Verifier::new(&key.verifying_key().to_bytes(), [11; 32]).unwrap();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let q = q.clone();
            let v = v.clone();
            std::thread::spawn(move || {
                let ch = q.new_challenge(&v).unwrap();
                q.bind(&ch.request()).unwrap();
                q.bytes()
            })
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().unwrap(), q.bytes());
    }
}
