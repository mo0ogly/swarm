//! Mandate delegation portage on owned isolated roots; no provider. Mirrors the
//! historical Go contract: configuration admission, strict child narrowing,
//! depth ceiling, ancestor revocation at approval/commit boundaries and
//! default lifetime capped by the parent.

use serde_json::{json, Value};
use swarm_engine::{RootStore, SystemEnvironment};
struct Fixture(std::path::PathBuf);
impl Fixture {
    /// Initialize one owned private root inside the repository fixture area
    /// (never /tmp, which the operator forbids for any project artifact);
    /// retain it on a failed assertion.
    fn new() -> Self {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.swarm-fixtures/trust-delegation");
        std::fs::create_dir_all(&base).unwrap();
        let p = base.join(swarm_engine::generate_identifier("").unwrap());
        std::fs::create_dir(&p).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        RootStore::initialize(&p, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        Self(p)
    }
    /// Invoke the shared public contract and close the opened root before decode.
    fn call(&self, op: &str, v: &Value, token: &str) -> Value {
        let s = RootStore::open(&self.0, true).unwrap();
        let r = s.trust(op, &serde_json::to_vec(v).unwrap(), token);
        s.close().unwrap();
        use std::error::Error;
        let reply = r.unwrap_or_else(|e| {
            panic!(
                "{op}: {e}; {}",
                e.source().map(|s| s.to_string()).unwrap_or_default()
            )
        });
        serde_json::from_slice(&reply.bytes).unwrap()
    }
    /// Raw access for typed engine errors (Invalid/Unsupported classification).
    fn try_call(&self, op: &str, v: &Value, token: &str) -> Result<Value, swarm_engine::TrustError> {
        let s = RootStore::open(&self.0, true).unwrap();
        let r = s.trust(op, &serde_json::to_vec(v).unwrap(), token);
        s.close().unwrap();
        let reply = match r {
            Ok(reply) => reply,
            Err(swarm_engine::RootStoreError::Trust(e)) => return Err(*e),
            Err(e) => panic!("{op}: unexpected store error {e}"),
        };
        let value: Value =
            serde_json::from_slice(&reply.bytes).unwrap_or_else(|e| panic!("{op}: {e}"));
        Ok(value)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// RFC3339 instant at `offset_minutes` from one process-wide base instant,
/// mirroring the Go contract's fixed fixture clock: commands of one scenario
/// carry identical bounds, so narrowing compares equal instants rather than
/// drifted ones while every stored mandate stays far from expiry.
fn stamp(offset_minutes: i64) -> String {
    use time::format_description::well_known::Rfc3339;
    static BASE: std::sync::OnceLock<time::OffsetDateTime> = std::sync::OnceLock::new();
    let t = *BASE.get_or_init(time::OffsetDateTime::now_utc)
        + time::Duration::minutes(offset_minutes);
    t.format(&Rfc3339)
        .unwrap()
        .replace("+00:00", "Z")
}
fn config(id: &str, revision: i64, depth: Option<i64>) -> Value {
    let mut c = json!({"schema_version":1,"event_id":id,"expected_revision":revision,"reason":"Isolated delegation regression","policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},"identities":[{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},{"subject":"executor","token":"fixture-executor-token","integrity":2,"roles":["effect"]},{"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]},{"subject":"carol","token":"fixture-carol-delegation-token","integrity":1,"roles":["requester"]},{"subject":"dave","token":"fixture-dave-delegation-token","integrity":1,"roles":["requester"]}]});
    if let Some(d) = depth {
        c["delegation"] = json!({"max_depth": d});
    }
    c
}
fn cmd(id: &str, v: Value) -> Value {
    let mut v = v;
    v["schema_version"] = 1.into();
    v["event_id"] = id.into();
    v
}
/// Positive-control root mandate: one hour from now, standard invoice terms.
fn root_mandate(f: &Fixture, event: &str) -> Value {
    let m = cmd(
        event,
        json!({"mandate_id":"root","subject":"requester","actions":["settle"],"resource":"invoice/acme","max_amount_minor":1000,"currency":"EUR","beneficiary":"ACME","starts_at":stamp(0),"expires_at":stamp(60)}),
    );
    let v = f.call("mandate-create", &m, "fixture-admin-token");
    assert_eq!(v["state"], "admitted", "{event}");
    v
}
/// One-link child under `parent`: same terms, thirty-minute lifetime.
fn delegated(id: &str, event: &str, parent: &str, subject: &str) -> Value {
    cmd(
        event,
        json!({"mandate_id":id,"parent_mandate_id":parent,"subject":subject,"actions":["settle"],"resource":"invoice/acme","max_amount_minor":1000,"currency":"EUR","beneficiary":"ACME","starts_at":stamp(0),"expires_at":stamp(30)}),
    )
}
fn delegated_request(id: &str, mandate: &str) -> Value {
    cmd(
        id,
        json!({"mandate_id":mandate,"resource":"invoice/acme","amount_minor":100,"currency":"EUR","beneficiary":"ACME","idempotency_key":id}),
    )
}
fn approval_for(id: &str, a: &Value) -> Value {
    cmd(
        id,
        json!({"action_id":a["id"],"mandate_id":a["mandate_id"],"resource":"invoice/acme","content_digest":a["content_digest"],"amount_minor":100,"currency":"EUR","beneficiary":"ACME"}),
    )
}

/// Configuration admission: an explicit non-negative depth persists through a
/// fresh open and the projection reports it; a non-integer or negative shape
/// refuses without mutating state.
#[test]
fn delegation_config_admission_persists_and_rejects_bad_shapes() {
    let f = Fixture::new();
    assert!(f.try_call("config-apply", &config("neg", 0, Some(-1)), "").is_err());
    for raw in [r#"{"max_depth":null}"#, r#"{"max_depth":"2"}"#, r#"{"max_depth":1.5}"#, r#"{"max_depth":true}"#] {
        let mut c = config("shape", 0, None);
        c["delegation"] = serde_json::from_str::<Value>(raw).unwrap();
        assert!(f.try_call("config-apply", &c, "").is_err(), "{raw}");
    }
    assert_eq!(
        f.call("config-apply", &config("admit", 0, Some(2)), "")["delegation"],
        json!({"max_depth": 2})
    );
    assert_eq!(
        f.call("show", &json!({}), "")["delegation"],
        json!({"max_depth": 2})
    );
    let mut c = config("edit", 1, Some(1));
    c["expected_revision"] = 1.into();
    assert_eq!(
        f.call("config-apply", &c, "")["delegation"],
        json!({"max_depth": 1})
    );
}

/// Depth 0 disables child mandates entirely: admission refuses with the
/// historical reason while an I1 requester can never endorse an I2 object.
#[test]
fn delegation_depth_zero_and_authority_boundary() {
    let f = Fixture::new();
    f.call("config-apply", &config("configure", 0, Some(0)), "");
    root_mandate(&f, "root-mandate");
    let v = f.call(
        "mandate-create",
        &delegated("child", "del-child-requester", "root", "carol"),
        "fixture-requester-token",
    );
    assert_eq!(v["state"], "denied");
    assert_eq!(v["reason"], "administrateur I3 requis");
    let v = f.call(
        "mandate-create",
        &delegated("child", "del-child-admin", "root", "carol"),
        "fixture-admin-token",
    );
    assert_eq!(v["state"], "denied");
    assert_eq!(v["reason"], "délégations désactivées");
}

/// The full chain works: a one-link child admits, serves its subject through
/// request, approval and commit, and the depth ceiling refuses a second link.
#[test]
fn delegation_chain_request_approve_commit_and_depth_ceiling() {
    let f = Fixture::new();
    f.call("config-apply", &config("configure", 0, Some(1)), "");
    root_mandate(&f, "root-mandate");
    let child = f.call(
        "mandate-create",
        &delegated("child", "del-child", "root", "carol"),
        "fixture-admin-token",
    );
    // The I2 object is redacted to a status acknowledgement for its I3
    // endorser; the recorded ancestry is read from the stored state.
    assert_eq!(child["state"], "admitted");
    let stored = f.call("show", &json!({}), "");
    let child_m = &stored["mandates"]["child"];
    assert_eq!(child_m["parent_mandate_id"], "root");
    assert_eq!(child_m["delegator"], "admin");
    assert_eq!(child_m["parent_content_digest"], stored["mandates"]["root"]["content_digest"]);
    // Depth ceiling: a grandchild would be two links above the root.
    let v = f.call(
        "mandate-create",
        &delegated("grand", "del-grand", "child", "dave"),
        "fixture-admin-token",
    );
    assert_eq!(v["state"], "denied");
    assert_eq!(v["reason"], "profondeur de délégation dépassée");
    // A one-link child mandate serves its own subject normally.
    let a = f.call(
        "request",
        &delegated_request("delegate-request", "child"),
        "fixture-carol-delegation-token",
    );
    assert_eq!(a["state"], "pending_approval");
    assert_eq!(
        f.call("approve", &approval_for("delegate-approve", &a), "fixture-approver-token")["state"],
        "approved"
    );
    let commit = cmd("delegate-commit", json!({"action_id":a["id"]}));
    assert_eq!(
        f.call("commit", &commit, "fixture-executor-token")["state"],
        "committed"
    );
}

/// An ancestor revocation must block approval and commit at the boundary with
/// the historical reason, without mutating protected business state.
#[test]
fn delegation_ancestor_revocation_blocks_boundaries() {
    for phase in ["approve", "commit"] {
        let f = Fixture::new();
        f.call("config-apply", &config("configure", 0, Some(1)), "");
        root_mandate(&f, "root-mandate");
        f.call(
            "mandate-create",
            &delegated("child", "del-child", "root", "carol"),
            "fixture-admin-token",
        );
        let a = f.call(
            "request",
            &delegated_request("before-revoke", "child"),
            "fixture-carol-delegation-token",
        );
        assert_eq!(a["state"], "pending_approval");
        if phase == "commit" {
            assert_eq!(
                f.call("approve", &approval_for("before-approve", &a), "fixture-approver-token")["state"],
                "approved"
            );
        }
        assert_eq!(
            f.call(
                "mandate-revoke",
                &cmd("ancestor-revoke", json!({"mandate_id":"root"})),
                "fixture-admin-token"
            )["state"],
            "revoked"
        );
        let before = f.call("show", &json!({}), "");
        let v = match phase {
            "approve" => f.call("approve", &approval_for("after-revoke", &a), "fixture-approver-token"),
            _ => f.call(
                "commit",
                &cmd("after-revoke", json!({"action_id":a["id"]})),
                "fixture-executor-token",
            ),
        };
        assert_eq!(v["state"], "denied", "{phase}");
        assert_eq!(v["reason"], "mandat parent révoqué", "{phase}");
        let after = f.call("show", &json!({}), "");
        assert_eq!(before["actions"], after["actions"], "{phase}");
        assert_eq!(after["ledger"], json!([]), "{phase}");
    }
}

/// A child may never widen its parent: amount, resource, currency, beneficiary,
/// actions, start, expiry, subject and parent must each stay strictly inside.
#[test]
fn delegation_rejects_amplification() {
    let cases: Vec<(&str, Value)> = vec![
        ("amount", json!({"max_amount_minor":1001})),
        ("resource", json!({"resource":"invoice/other"})),
        ("currency", json!({"currency":"USD"})),
        ("beneficiary", json!({"beneficiary":"ATTACKER"})),
        ("actions", json!({"actions":["settle","other"]})),
        ("start", json!({"starts_at":stamp(-30)})),
        ("expiry", json!({"expires_at":stamp(90)})),
        ("subject", json!({"subject":"unknown"})),
        ("parent", json!({"parent_mandate_id":"unknown"})),
    ];
    for (name, mutate) in cases {
        let f = Fixture::new();
        f.call("config-apply", &config("configure", 0, Some(2)), "");
        root_mandate(&f, "root-mandate");
        let mut c = delegated("rejected", "del-rejected", "root", "carol");
        for (k, v) in mutate.as_object().unwrap() {
            c[k] = v.clone();
        }
        let before = f.call("show", &json!({}), "");
        let v = f.call("mandate-create", &c, "fixture-admin-token");
        assert_eq!(v["state"], "denied", "{name}");
        let after = f.call("show", &json!({}), "");
        assert_eq!(
            after["mandates"].as_object().unwrap().len(),
            before["mandates"].as_object().unwrap().len(),
            "{name}"
        );
        assert_eq!(after["actions"], before["actions"], "{name}");
        assert_eq!(after["ledger"], before["ledger"], "{name}");
        assert_eq!(
            after["evidence_count"].as_u64().unwrap(),
            before["evidence_count"].as_u64().unwrap() + 1,
            "{name}"
        );
    }
}

/// A child without an explicit expiry inherits at most its parent's lifetime.
#[test]
fn delegation_default_expiry_bounded_by_parent() {
    let f = Fixture::new();
    f.call("config-apply", &config("configure", 0, Some(1)), "");
    let parent = root_mandate(&f, "root-mandate");
    let mut c = delegated("default-child", "del-default-child", "root", "carol");
    c.as_object_mut().unwrap().remove("expires_at");
    let child = f.call("mandate-create", &c, "fixture-admin-token");
    assert_eq!(child["state"], "admitted");
    assert_eq!(child["expires_at"], parent["expires_at"]);
}

/// The public projection reports each mandate's effective chain status.
#[test]
fn delegation_projection_reports_status() {
    let f = Fixture::new();
    f.call("config-apply", &config("configure", 0, Some(1)), "");
    root_mandate(&f, "root-mandate");
    f.call(
        "mandate-create",
        &delegated("child", "del-child", "root", "carol"),
        "fixture-admin-token",
    );
    let view = f.call("show", &json!({}), "");
    assert_eq!(view["mandate_status"]["root"], "Mandat disponible");
    assert_eq!(view["mandate_status"]["child"], "Mandat disponible");
    f.call(
        "mandate-revoke",
        &cmd("revoke-root", json!({"mandate_id":"root"})),
        "fixture-admin-token",
    );
    let view = f.call("show", &json!({}), "");
    assert_eq!(view["mandate_status"]["root"], "mandat révoqué");
    assert_eq!(view["mandate_status"]["child"], "mandat parent révoqué");
}

/// A technical stored-state failure must refuse the operator projection with
/// its typed cause instead of an availability label: an unparseable stored
/// `starts_at` makes `show` fail typed, without mutating stored state, and
/// exact restoration recovers the valid projection. Mirrors the reproduced
/// supervisor fault on candidate 325, on an isolated owned root only.
#[test]
fn mandate_status_projection_propagates_stored_timestamp_error() {
    let f = Fixture::new();
    f.call("config-apply", &config("configure", 0, Some(1)), "");
    root_mandate(&f, "root-mandate");
    let view = f.call("show", &json!({}), "");
    assert_eq!(view["mandate_status"]["root"], "Mandat disponible");
    // Owned stored-state fault only: replace the root mandate's recorded start
    // with a value outside the RFC3339 contract, preserving every other byte.
    // The stored value is formatted by the engine (offset form, zeroed
    // nanoseconds), so the fault locates the pretty-printed key itself: compact
    // event replies never carry a space after the colon, so exactly one match.
    let path = f.0.join(".swarm/trust-actions.json");
    let original = std::fs::read(&path).unwrap();
    let text = std::str::from_utf8(&original).unwrap();
    let key = "\"starts_at\": \"";
    assert_eq!(text.matches(key).count(), 1, "one stored mandate start");
    let start = text.find(key).unwrap() + key.len();
    let end = start + text[start..].find('"').unwrap();
    assert!(text[start..end].contains('T'), "RFC3339 stored start");
    let mut mutated = original.clone();
    mutated.splice(start..end, b"STORED_INVALID_TIMESTAMP".iter().copied());
    std::fs::write(&path, &mutated).unwrap();
    // The projection refuses typed instead of reporting availability, and the
    // failed read-only show leaves the stored bytes untouched.
    let err = f.try_call("show", &json!({}), "").err().expect("show refuses");
    assert!(
        matches!(&err, swarm_engine::TrustError::Invalid("date RFC3339 requise")),
        "{err}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), mutated, "no mutation");
    // Exact restoration recovers the valid projection, byte-identical.
    std::fs::write(&path, &original).unwrap();
    let view = f.call("show", &json!({}), "");
    assert_eq!(view["mandate_status"]["root"], "Mandat disponible");
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

/// A stored delegation depth whose shape configuration admission refuses must
/// refuse the operator projection with its typed cause instead of silently
/// reading as 0 and displaying the business refusal "délégations
/// désactivées": a string stored `max_depth` makes `show` fail typed, without
/// mutating stored state, and exact restoration recovers both mandates.
/// Mirrors the reproduced supervisor fault on image330 (root isolated via the
/// public CLI, stored `delegation.max_depth` replaced by
/// `INVALID_STORED_DEPTH`, child flipped to "délégations désactivées" with
/// `fault_exit` 0), on an owned isolated root only. The explicit zero stays
/// the legitimate business refusal; the stored block absent entirely keeps
/// the historical compatibility fallback to the registered default.
#[test]
fn mandate_status_projection_propagates_stored_depth_error() {
    let f = Fixture::new();
    f.call("config-apply", &config("configure", 0, Some(1)), "");
    root_mandate(&f, "root-mandate");
    f.call(
        "mandate-create",
        &delegated("child", "del-child", "root", "carol"),
        "fixture-admin-token",
    );
    let view = f.call("show", &json!({}), "");
    assert_eq!(view["mandate_status"]["root"], "Mandat disponible");
    assert_eq!(view["mandate_status"]["child"], "Mandat disponible");
    // Owned stored-state fault only: replace the stored integer depth with a
    // string, preserving every other byte. Extensions are retained as compact
    // JSON (CompactJson strips whitespace) and serialize verbatim inside the
    // pretty state, but retained raw command copies inside the state also
    // contain "max_depth", so the fault anchors structurally on the one
    // top-level extension line — newline plus two-space pretty indent plus
    // the compact delegation object — whose literal byte sequence cannot
    // occur inside an escaped string copy; the object brace terminates the
    // value. No global replace: the splice touches only this field.
    let path = f.0.join(".swarm/trust-actions.json");
    let original = std::fs::read(&path).unwrap();
    let text = std::str::from_utf8(&original).unwrap();
    let anchor = "\n  \"delegation\": {\"max_depth\":";
    assert_eq!(
        text.matches(anchor).count(),
        1,
        "one top-level delegation extension"
    );
    let start = text.find(anchor).unwrap() + anchor.len();
    let end = start + text[start..].find('}').unwrap();
    assert_eq!(&text[start..end], "1", "stored integer depth");
    let mut mutated = original.clone();
    mutated.splice(start..end, b"\"INVALID_STORED_DEPTH\"".iter().copied());
    std::fs::write(&path, &mutated).unwrap();
    // The projection refuses typed — the same shape code configuration
    // admission already uses — instead of an availability label, and the
    // failed read-only show leaves the stored bytes untouched.
    let err = f.try_call("show", &json!({}), "").err().expect("show refuses");
    assert!(
        matches!(
            &err,
            swarm_engine::TrustError::Integrity("trust-delegation-shape")
        ),
        "{err}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), mutated, "no mutation");
    // Exact restoration recovers both mandates, byte-identical.
    std::fs::write(&path, &original).unwrap();
    let view = f.call("show", &json!({}), "");
    assert_eq!(view["mandate_status"]["root"], "Mandat disponible");
    assert_eq!(view["mandate_status"]["child"], "Mandat disponible");
    assert_eq!(std::fs::read(&path).unwrap(), original);
    // The other stored shape configuration admission refuses — a negative
    // depth (read-only show never runs require_supported, so the reader is
    // the only barrier) — refuses with the same typed code, still without
    // mutating the stored bytes, and exact restoration recovers again.
    let mut negative = original.clone();
    negative.splice(start..end, b"-1".iter().copied());
    std::fs::write(&path, &negative).unwrap();
    let err = f.try_call("show", &json!({}), "").err().expect("show refuses");
    assert!(
        matches!(
            &err,
            swarm_engine::TrustError::Integrity("trust-delegation-shape")
        ),
        "{err}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), negative, "no mutation");
    std::fs::write(&path, &original).unwrap();
    let view = f.call("show", &json!({}), "");
    assert_eq!(view["mandate_status"]["root"], "Mandat disponible");
    assert_eq!(view["mandate_status"]["child"], "Mandat disponible");
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
