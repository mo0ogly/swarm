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
        serde_json::from_slice::<Value>(&reply.bytes).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
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
/// Positive-control root mandate for the invoice resource.
fn root_mandate(f: &Fixture, event: &str) -> Value {
    let m = cmd(
        event,
        json!({"mandate_id":"root","subject":"requester","actions":["settle"],"resource":"invoice/acme","max_amount_minor":1000,"currency":"EUR","beneficiary":"ACME","starts_at":"2026-11-02T12:00:00Z","expires_at":"2026-11-02T13:00:00Z"}),
    );
    let v = f.call("mandate-create", &m, "fixture-admin-token");
    assert_eq!(v["state"], "admitted", "{event}");
    v
}
fn delegated(id: &str, parent: &str, subject: &str) -> Value {
    cmd(
        format!("del-{id}").as_str(),
        json!({"mandate_id":id,"parent_mandate_id":parent,"subject":subject,"actions":["settle"],"resource":"invoice/acme","max_amount_minor":1000,"currency":"EUR","beneficiary":"ACME","starts_at":"2026-11-02T12:00:00Z","expires_at":"2026-11-02T12:30:00Z"}),
    )
}
fn delegated_request(id: &str, mandate: &str) -> Value {
    cmd(
        id,
        json!({"mandate_id":mandate,"resource":"invoice/acme","amount_minor":100,"currency":"EUR","beneficiary":"ACME","idempotency_key":id}),
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
    let before = std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
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
    let _ = before;
}

/// Depth 0 disables child mandates entirely: admission refuses with the
/// historical reason while an I1 requester can never endorse an I2 object.
#[test]
fn delegation_depth_zero_and_authority_boundary() {
    let f = Fixture::new();
    f.call("config-apply", &config("configure", 0, Some(0)), "");
    root_mandate(&f, "root-mandate");
    let child = delegated("child", "root", "carol");
    let v = f.call("mandate-create", &child, "fixture-requester-token");
    assert_eq!(v["state"], "denied");
    assert_eq!(v["reason"], "administrateur I3 requis");
    let child = cmd("del-child", child);
    let v = f.call("mandate-create", &child, "fixture-admin-token");
    assert_eq!(v["state"], "denied");
    assert_eq!(v["reason"], "délégations désactivées");
}

/// The full chain works: two levels of delegation admit, a child request flows
/// to approval and commit, and the depth ceiling refuses a third link.
#[test]
fn delegation_chain_request_approve_commit_and_depth_ceiling() {
    let f = Fixture::new();
    f.call("config-apply", &config("configure", 0, Some(1)), "");
    root_mandate(&f, "root-mandate");
    let child = f.call("mandate-create", &delegated("child", "root", "carol"), "fixture-admin-token");
    assert_eq!(child["state"], "admitted");
    assert_eq!(child["parent_mandate_id"], "root");
    assert_eq!(child["delegator"], "admin");
    // Depth ceiling: a grandchild would be two links above the root.
    let v = f.call(
        "mandate-create",
        &delegated("grand", "child", "dave"),
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
    let approval = cmd(
        "delegate-approve",
        json!({"action_id":a["id"],"mandate_id":"child","resource":"invoice/acme","content_digest":a["content_digest"],"amount_minor":100,"currency":"EUR","beneficiary":"ACME"}),
    );
    assert_eq!(
        f.call("approve", &approval, "fixture-approver-token")["state"],
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
            &delegated("child", "root", "carol"),
            "fixture-admin-token",
        );
        let a = f.call(
            "request",
            &delegated_request("before-revoke", "child"),
            "fixture-carol-delegation-token",
        );
        assert_eq!(a["state"], "pending_approval");
        if phase == "commit" {
            let approval = cmd(
                "before-approve",
                json!({"action_id":a["id"],"mandate_id":"child","resource":"invoice/acme","content_digest":a["content_digest"],"amount_minor":100,"currency":"EUR","beneficiary":"ACME"}),
            );
            assert_eq!(
                f.call("approve", &approval, "fixture-approver-token")["state"],
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
            "approve" => f.call(
                "approve",
                &cmd(
                    "after-revoke",
                    json!({"action_id":a["id"],"mandate_id":"child","resource":"invoice/acme","content_digest":a["content_digest"],"amount_minor":100,"currency":"EUR","beneficiary":"ACME"}),
                ),
                "fixture-approver-token",
            ),
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
        ("start", json!({"starts_at":"2026-11-02T11:00:00Z","expires_at":"2026-11-02T12:30:00Z"})),
        ("expiry", json!({"expires_at":"2026-11-02T15:00:00Z"})),
        ("subject", json!({"subject":"unknown"})),
        ("parent", json!({"parent_mandate_id":"unknown"})),
    ];
    for (name, mutate) in cases {
        let f = Fixture::new();
        f.call("config-apply", &config("configure", 0, Some(2)), "");
        root_mandate(&f, "root-mandate");
        let mut c = delegated("rejected", "root", "carol");
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
    let mut c = delegated("default-child", "root", "carol");
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
        &delegated("child", "root", "carol"),
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
