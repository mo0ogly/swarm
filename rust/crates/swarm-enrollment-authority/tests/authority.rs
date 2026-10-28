use ed25519_dalek::SigningKey;
use swarm_enrollment_authority::{
    decode, decode_active, encode, sign_enrollment, Document, Expected,
};
use swarm_enrollment_proof::Verifier;
use swarm_enrollment_state::{sign_change, Context, Head, Journal};
use swarm_identity_contract::Permission;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
fn at(raw: &str) -> OffsetDateTime {
    OffsetDateTime::parse(raw, &Rfc3339).unwrap()
}
mod common;
use common::document;

struct Fixture {
    _dir: tempfile::TempDir,
    journal: Journal,
    key: SigningKey,
    doc: Document,
    expected: Expected,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let doc = document();
        let journal = Journal::open(
            &Context::new(),
            dir.path().join("journal.sqlite"),
            &key.verifying_key().to_bytes(),
            [11; 32],
        )
        .unwrap();
        let raw = sign_enrollment(&key, [11; 32], &Head::default(), &doc).unwrap();
        let head = journal
            .apply(&Context::new(), &Head::default(), &raw)
            .unwrap();
        let expected = Expected {
            head,
            enrollment_id: doc.enrollment_id.clone(),
            generation: 1,
            project_digest: doc.project_digest.clone(),
            principal: doc.principal.clone(),
            scope: doc.scope.clone(),
            trust_revision: 2,
            request_digest: "0d".repeat(32),
        };
        Self {
            _dir: dir,
            journal,
            key,
            doc,
            expected,
        }
    }
    fn statement(&self) -> swarm_enrollment_proof::Statement {
        let v = Verifier::new(&self.key.verifying_key().to_bytes(), [11; 32]).unwrap();
        let ch = v.new_challenge([13; 32]).unwrap();
        let raw = self
            .journal
            .sign_active(
                &Context::new(),
                &self.expected.head,
                &self.doc.enrollment_id,
                ch.request(),
                &self.key,
            )
            .unwrap();
        v.verify(&ch, &raw).unwrap()
    }
}
#[test]
fn canonical_roundtrip_owned_rights_and_explicit_empty_delegation() {
    let mut d = document();
    let raw = encode(&d).unwrap();
    let mut got = decode(&raw).unwrap();
    assert_eq!(got, d);
    got.permissions[0].resource = "changed".into();
    assert_eq!(decode(&raw).unwrap(), d);
    d.delegable.clear();
    let raw = encode(&d).unwrap();
    assert!(String::from_utf8(raw.clone())
        .unwrap()
        .contains("\"delegable\":[]"));
    assert_eq!(decode(&raw).unwrap(), d);
    let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(value["principal"]["principal_id"], "alice");
    assert_eq!(value["principal"]["principal_kind"], "human");
    assert!(value["principal"].get("id").is_none());
}
#[test]
fn schema_rights_and_dimensions_are_never_defaulted_or_expanded() {
    for kind in [
        "schema",
        "id",
        "generation",
        "project",
        "local",
        "service",
        "scope",
        "trust",
        "no-rights",
        "duplicate",
        "expanded",
        "duplicate-delegable",
        "wildcard",
        "zero-time",
        "inverted",
    ] {
        let mut d = document();
        encode(&d).unwrap();
        match kind {
            "schema" => d.schema_version = 2,
            "id" => d.enrollment_id = " ".into(),
            "generation" => d.generation = 0,
            "project" => d.project_digest = "unverified".into(),
            "local" => {
                d.principal.kind = "local".into();
                d.principal.issuer = "urn:swarm:local".into();
                d.principal.subject = "alice".into()
            }
            "service" => d.principal.kind = "service".into(),
            "scope" => d.scope.plan_version = 0,
            "trust" => d.trust_revision = 0,
            "no-rights" => d.permissions.clear(),
            "duplicate" => d.permissions.push(d.permissions[0].clone()),
            "expanded" => d.delegable.push(Permission {
                operation: "tool.invoke".into(),
                resource: "other".into(),
            }),
            "duplicate-delegable" => d.delegable.push(d.delegable[0].clone()),
            "wildcard" => d.permissions[0].resource = "*".into(),
            "zero-time" => d.not_before = at("0001-01-01T00:00:00Z"),
            "inverted" => d.expires_at = d.not_before,
            _ => unreachable!(),
        };
        assert_eq!(encode(&d).unwrap_err().code(), "document", "{kind}");
    }
}
#[test]
fn exact_json_rejects_noncanonical_nested_or_omitted_values() {
    let raw = encode(&document()).unwrap();
    let s = String::from_utf8(raw.clone()).unwrap();
    for bad in [
        format!(" {s}"),
        format!("{s}{{}}"),
        s.replacen('{', "{\"extra\":true,", 1),
        s.replacen('{', "{\"schema_version\":1,", 1),
        s.replace("\"principal_id\"", "\"Principal_ID\""),
        s.replace("\"principal_kind\":\"human\"", "\"principal_kind\":null"),
        s.replace("\"trust_configuration_revision\":2,", ""),
        s.replace("\"plan_version\":3", "\"plan_version\":3.0"),
    ] {
        decode(&raw).unwrap();
        assert_eq!(decode(bad.as_bytes()).unwrap_err().code(), "document");
    }
}
#[test]
fn utc_nanoseconds_and_calendar_boundaries_have_no_duration_default() {
    let mut d = document();
    let base = d.clone();
    d.not_before = d
        .not_before
        .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
    d.expires_at = d
        .expires_at
        .to_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
    assert_eq!(encode(&d).unwrap(), encode(&base).unwrap());
    let noncanon = String::from_utf8(encode(&base).unwrap())
        .unwrap()
        .replace("2026-10-17T08:00:00Z", "2026-10-17T10:00:00+02:00");
    assert_eq!(decode(noncanon.as_bytes()).unwrap_err().code(), "document");
    for (start, end) in [
        ("0000-01-01T00:00:00Z", "0001-01-01T00:00:00Z"),
        (
            "2026-10-17T08:00:00.000000001Z",
            "2026-10-17T08:00:00.000000002Z",
        ),
        (
            "9999-12-31T23:59:59.999999998Z",
            "9999-12-31T23:59:59.999999999Z",
        ),
    ] {
        let mut d = document();
        d.not_before = at(start);
        d.expires_at = at(end);
        let raw = encode(&d).unwrap();
        assert_eq!(decode(&raw).unwrap(), d);
        assert!(String::from_utf8(raw).unwrap().contains(start));
    }
}
#[test]
fn proposal_signing_requires_next_generation_and_same_project() {
    let f = Fixture::new();
    for kind in ["generation", "project", "overflow", "head"] {
        let (mut d, mut h) = (f.doc.clone(), Head::default());
        match kind {
            "generation" => d.generation = 2,
            "project" => d.project_digest = "a".repeat(64),
            "overflow" => h.sequence = i64::MAX,
            "head" => h.sequence = -1,
            _ => unreachable!(),
        };
        assert_eq!(
            sign_enrollment(&f.key, [11; 32], &h, &d)
                .unwrap_err()
                .code(),
            "binding"
        );
    }
}
#[test]
fn authenticated_consumer_binds_each_independent_dimension_and_window() {
    let f = Fixture::new();
    let s = f.statement();
    assert_eq!(
        decode_active(&s, &f.expected, f.doc.not_before).unwrap(),
        f.doc
    );
    for kind in [
        "head",
        "id",
        "generation",
        "project",
        "principal",
        "scope",
        "trust",
        "request",
        "invalid-head",
        "invalid-scope",
    ] {
        let mut e = f.expected.clone();
        match kind {
            "head" => e.head.digest = "a".repeat(64),
            "id" => e.enrollment_id = "other".into(),
            "generation" => e.generation = 2,
            "project" => e.project_digest = "a".repeat(64),
            "principal" => e.principal.subject = "other".into(),
            "scope" => e.scope.plan_digest = "a".repeat(64),
            "trust" => e.trust_revision = 3,
            "request" => e.request_digest = "a".repeat(64),
            "invalid-head" => e.head = Head::default(),
            "invalid-scope" => e.scope.plan_version = 0,
            _ => unreachable!(),
        };
        assert_eq!(
            decode_active(&s, &e, f.doc.not_before).unwrap_err().code(),
            "binding",
            "{kind}"
        );
    }
    for t in [
        at("0001-01-01T00:00:00Z"),
        f.doc.not_before - time::Duration::nanoseconds(1),
        f.doc.expires_at,
        f.doc.expires_at + time::Duration::nanoseconds(1),
    ] {
        assert_eq!(
            decode_active(&s, &f.expected, t).unwrap_err().code(),
            "inactive"
        );
    }
}
#[test]
fn historical_signature_does_not_reread_a_new_revocation() {
    let f = Fixture::new();
    let s = f.statement();
    let raw = sign_change(
        &f.key,
        [11; 32],
        &f.expected.head,
        "revoke",
        &f.doc.enrollment_id,
        b"",
    )
    .unwrap();
    let head = f
        .journal
        .apply(&Context::new(), &f.expected.head, &raw)
        .unwrap();
    assert_eq!(
        decode_active(&s, &f.expected, f.doc.not_before).unwrap(),
        f.doc
    );
    let mut current = f.expected.clone();
    current.head = head;
    assert_eq!(
        decode_active(&s, &current, f.doc.not_before)
            .unwrap_err()
            .code(),
        "binding"
    );
}

#[test]
fn authenticated_bad_frame_preserves_its_version_cause() {
    use std::error::Error as _;
    let f = Fixture::new();
    let v = Verifier::new(&f.key.verifying_key().to_bytes(), [11; 32]).unwrap();
    let ch = v.new_challenge([13; 32]).unwrap();
    let payload=serde_json::to_vec(&serde_json::json!({"schema_version":2,"head":f.expected.head,"record":{"id":f.doc.enrollment_id,"body":"","enrolled_sequence":1}})).unwrap();
    let raw = swarm_enrollment_proof::sign_response(&ch.request(), &payload, &f.key).unwrap();
    let s = v.verify(&ch, &raw).unwrap();
    let err = decode_active(&s, &f.expected, f.doc.not_before).unwrap_err();
    assert_eq!(err.code(), "document");
    assert!(err.source().unwrap().to_string().contains("reply-version"));
}
#[test]
fn later_event_does_not_change_creation_generation_and_body_cannot_override_it() {
    let mut f = Fixture::new();
    let raw = sign_change(
        &f.key,
        [11; 32],
        &f.expected.head,
        "enroll",
        "other",
        b"opaque",
    )
    .unwrap();
    f.expected.head = f
        .journal
        .apply(&Context::new(), &f.expected.head, &raw)
        .unwrap();
    assert_eq!(
        decode_active(&f.statement(), &f.expected, f.doc.not_before)
            .unwrap()
            .generation,
        1
    );
    let mut forged = f.doc.clone();
    forged.generation = 2;
    let body = encode(&forged).unwrap();
    let payload = swarm_wire_json::encode(&swarm_enrollment_state::ActiveReply {
        schema_version: 1,
        head: f.expected.head.clone(),
        record: swarm_enrollment_state::Record {
            id: f.doc.enrollment_id.clone(),
            body,
            enrolled_sequence: 1,
        },
    })
    .unwrap();
    let v = Verifier::new(&f.key.verifying_key().to_bytes(), [11; 32]).unwrap();
    let ch = v.new_challenge([13; 32]).unwrap();
    let response = swarm_enrollment_proof::sign_response(&ch.request(), &payload, &f.key).unwrap();
    let s = v.verify(&ch, &response).unwrap();
    assert_eq!(
        decode_active(&s, &f.expected, f.doc.not_before)
            .unwrap_err()
            .code(),
        "binding"
    );
}
#[test]
fn many_distinct_rights_have_no_new_policy_cap_and_duplicates_still_fail() {
    let mut d = document();
    d.permissions = (0..1000)
        .map(|i| Permission {
            operation: "agent.create".into(),
            resource: format!("task/{i}"),
        })
        .collect();
    d.delegable = d.permissions.iter().step_by(2).cloned().collect();
    encode(&d).unwrap();
    d.permissions.push(d.permissions[0].clone());
    assert_eq!(encode(&d).unwrap_err().code(), "document");
    d.permissions.pop();
    encode(&d).unwrap();
}
