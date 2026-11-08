//! Data provenance portage (T4 « provenance ») on owned isolated roots; no
//! provider. Mirrors the historical Go contract: public projection without
//! mutation, admission refusing before any capture, exact classified path with
//! recovery, strict typed validation, approve/commit re-admission of
//! pre-policy actions, persistence across reopen and resource classification.

use serde_json::{json, Value};
use swarm_engine::{RootStore, SystemEnvironment};
struct Fixture(std::path::PathBuf);
impl Fixture {
    /// Initialize one owned private root inside the repository fixture area
    /// (never /tmp, which the operator forbids for any project artifact);
    /// retain it on a failed assertion.
    fn new() -> Self {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.swarm-fixtures/trust-data-policy");
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
    /// Raw access for typed engine errors (Invalid/Authorization classification).
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
    /// Byte image of the retained trust state, for no-mutation assertions.
    fn state_bytes(&self) -> Vec<u8> {
        std::fs::read(self.0.join(".swarm/trust-actions.json")).unwrap()
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
/// carry identical bounds, so every stored mandate stays far from expiry.
fn stamp(offset_minutes: i64) -> String {
    use time::format_description::well_known::Rfc3339;
    static BASE: std::sync::OnceLock<time::OffsetDateTime> = std::sync::OnceLock::new();
    let t = *BASE.get_or_init(time::OffsetDateTime::now_utc)
        + time::Duration::minutes(offset_minutes);
    t.format(&Rfc3339)
        .unwrap()
        .replace("+00:00", "Z")
}
fn config(id: &str, revision: i64) -> Value {
    json!({"schema_version":1,"event_id":id,"expected_revision":revision,"reason":"Isolated data policy regression","policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},"identities":[{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},{"subject":"executor","token":"fixture-executor-token","integrity":2,"roles":["effect"]},{"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}]})
}
fn cmd(id: &str, v: Value) -> Value {
    let mut v = v;
    v["schema_version"] = 1.into();
    v["event_id"] = id.into();
    v
}
/// Go `activeDataPolicy`: one classified resource, ledger-only destination set.
fn active_policy() -> Value {
    json!({"resources":[{"resource":"invoice/acme","classification":"internal"}],"enabled":true,"classifications":["public","internal","confidential","restricted"],"destinations":["local-ledger"],"providers":["swarm"],"tools":["simulated-settlement"]})
}
/// Go `allowedDataContext`: the single admitted path for that policy.
fn allowed_context() -> Value {
    json!({"classification":"internal","destination":"local-ledger","provider":"swarm","tool":"simulated-settlement"})
}
/// Configuration change carrying `policy` as the data_policy extension.
fn policy_change(id: &str, revision: i64, policy: Value) -> Value {
    let mut c = config(id, revision);
    c["data_policy"] = policy;
    c
}
fn enable_data_policy(f: &Fixture, revision: i64) {
    let v = f.call(
        "config-apply",
        &policy_change("policy-enable", revision, active_policy()),
        "",
    );
    // The apply reply is the projected state (no applied flag): the stored
    // effective policy must be visible and active in it.
    assert_eq!(v["data_policy"]["enabled"], true, "policy not applied");
}
/// Positive-control root mandate: standard invoice terms for the classified
/// resource, one hour lifetime.
fn data_mandate(f: &Fixture, event: &str) -> Value {
    let m = cmd(
        event,
        json!({"mandate_id":"data-mandate","subject":"requester","actions":["settle"],"resource":"invoice/acme","max_amount_minor":50000,"currency":"EUR","beneficiary":"ACME","starts_at":stamp(-1),"expires_at":stamp(60)}),
    );
    let v = f.call("mandate-create", &m, "fixture-admin-token");
    assert_eq!(v["state"], "admitted", "{event}");
    v
}
/// Go `dataRequest`: classified request under the active policy.
fn data_request(event: &str) -> Value {
    cmd(
        event,
        json!({"mandate_id":"data-mandate","resource":"invoice/acme","amount_minor":100,"currency":"EUR","beneficiary":"ACME","idempotency_key":event,"data_context":allowed_context()}),
    )
}
/// Go `approveAction`: exact stored-action terms, approver token.
fn approve_action(f: &Fixture, event: &str, a: &Value) -> Value {
    let v = f.call(
        "approve",
        &cmd(
            event,
            json!({"action_id":a["id"],"mandate_id":a["mandate_id"],"resource":a["resource"],"content_digest":a["content_digest"],"amount_minor":a["amount_minor"],"currency":a["currency"],"beneficiary":a["beneficiary"]}),
        ),
        "fixture-approver-token",
    );
    assert_eq!(v["state"], "approved", "{event}");
    v
}

/// Public contract: a valid inactive policy previews with the effective
/// data_policy block and never mutates retained state.
#[test]
fn data_policy_public_contract() {
    let f = Fixture::new();
    f.call("config-apply", &config("config", 0), "");
    let before = f.state_bytes();
    let preview = f.call(
        "config-preview",
        &policy_change("policy-preview", 1, json!({"resources":[],"enabled":false,"classifications":[],"destinations":[],"providers":[],"tools":[]})),
        "",
    );
    assert!(
        preview.get("data_policy").is_some(),
        "preview omits effective data policy"
    );
    assert_eq!(preview["applied"], false);
    assert_eq!(f.state_bytes(), before, "preview mutated state");
}

/// Admission refuses every non-classified path before any capture: the
/// beneficiary sentinel and the supplied context never persist, and state
/// stays byte-identical.
#[test]
fn data_policy_admission_rejects_without_capture() {
    for (name, context) in [
        ("absent", None),
        (
            "url",
            Some(json!({"classification":"internal","destination":"https://example.invalid/leak","provider":"swarm","tool":"simulated-settlement"})),
        ),
        (
            "path",
            Some(json!({"classification":"internal","destination":"../../private","provider":"swarm","tool":"simulated-settlement"})),
        ),
        (
            "symlink",
            Some(json!({"classification":"internal","destination":"linked-secret","provider":"swarm","tool":"simulated-settlement"})),
        ),
        (
            "binary",
            Some(json!({"classification":"internal","destination":"local-ledger","provider":"swarm","tool":"/bin/cat"})),
        ),
        (
            "provider",
            Some(json!({"classification":"internal","destination":"local-ledger","provider":"external","tool":"simulated-settlement"})),
        ),
        (
            "confidential",
            Some(json!({"classification":"confidential","destination":"local-ledger","provider":"swarm","tool":"simulated-settlement"})),
        ),
        (
            "restricted",
            Some(json!({"classification":"restricted","destination":"local-ledger","provider":"swarm","tool":"simulated-settlement"})),
        ),
    ] {
        let f = Fixture::new();
        f.call("config-apply", &config("config", 0), "");
        enable_data_policy(&f, 1);
        data_mandate(&f, "data-mandate");
        let mut c = data_request("data-denied");
        c["beneficiary"] = json!("SENSITIVE-SENTINEL");
        match &context {
            None => {
                c.as_object_mut().unwrap().remove("data_context");
            }
            Some(v) => c["data_context"] = v.clone(),
        }
        let before = f.state_bytes();
        let err = f
            .try_call("request", &c, "fixture-requester-token")
            .err()
            .unwrap();
        assert_eq!(err.code(), "authorization_required", "{name}: path not refused");
        let after = f.state_bytes();
        assert_eq!(after, before, "{name}: denied data persisted");
        assert!(
            !std::str::from_utf8(&after).unwrap().contains("SENSITIVE-SENTINEL"),
            "{name}: sentinel captured"
        );
    }
}

/// The exact classified path admits; a caller downgrading the classification
/// refuses; approve and commit re-admit the stored path; the stored context
/// survives; narrowing the policy afterwards still lets the committed effect
/// reconcile (recovery), and the historical commit replay does not duplicate.
#[test]
fn data_policy_exact_action_and_recovery() {
    let f = Fixture::new();
    f.call("config-apply", &config("config", 0), "");
    enable_data_policy(&f, 1);
    data_mandate(&f, "data-mandate");
    let a = f.call("request", &data_request("data-request"), "fixture-requester-token");
    let mut other = data_request("data-other-content");
    other["data_context"]["classification"] = json!("public");
    let err = f
        .try_call("request", &other, "fixture-requester-token")
        .err()
        .unwrap();
    assert_eq!(err.code(), "authorization_required", "caller downgraded resource classification");
    let a = approve_action(&f, "data-approve", &a);
    let commit = cmd("data-commit", json!({"action_id":a["id"],"lose_response":true}));
    let v = f.call("commit", &commit, "fixture-executor-token");
    assert_eq!(v["state"], "unknown", "commit lost-response state");
    let st: Value =
        serde_json::from_slice(&f.state_bytes()).unwrap();
    assert_eq!(
        st["actions"][a["id"].as_str().unwrap()]["data_context"]["classification"],
        "internal",
        "context not persisted"
    );
    // Narrow the policy to empty classifications; the committed effect must
    // still reconcile (Go deliberately skips re-admission on reconcile).
    let mut narrowed = active_policy();
    narrowed["classifications"] = json!([]);
    f.call(
        "config-apply",
        &policy_change("policy-narrow", 2, narrowed),
        "",
    );
    let v = f.call(
        "reconcile",
        &cmd("data-reconcile", json!({"action_id":a["id"]})),
        "fixture-executor-token",
    );
    assert_eq!(v["state"], "reconciled", "committed effect lost");
    // Historical replay of the same commit stays idempotent: no second effect.
    let v = f.call("commit", &commit, "fixture-executor-token");
    assert_eq!(v["state"], "unknown", "historical replay lost");
    let view = f.call("show", &json!({}), "");
    assert_eq!(view["ledger"].as_array().unwrap().len(), 1, "effect duplicated");
}

/// Strict typed validation: every policy key and every context field is
/// explicit and non-null; invalid typed edits refuse at configuration
/// without mutation.
#[test]
fn data_policy_strict_validation() {
    let full = active_policy();
    for key in ["enabled", "classifications", "destinations", "providers", "tools", "resources"] {
        for mode in ["missing", "null"] {
            let f = Fixture::new();
            f.call("config-apply", &config("config", 0), "");
            let mut p = full.clone();
            match mode {
                "missing" => {
                    p.as_object_mut().unwrap().remove(key);
                }
                _ => {
                    p[key] = Value::Null;
                }
            }
            let raw = serde_json::to_string(&p).unwrap();
            let mut c = config("policy-strict", 1);
            c.as_object_mut().unwrap().insert(
                "data_policy".into(),
                serde_json::from_str::<Value>(&raw).unwrap(),
            );
            let err = f
                .try_call("config-apply", &c, "")
                .err()
                .unwrap();
            assert_eq!(err.code(), "format", "{key}-{mode}: incomplete policy accepted");
        }
    }
    for key in ["classification", "destination", "provider", "tool"] {
        let f = Fixture::new();
        let mut ctx = allowed_context();
        ctx[key] = Value::Null;
        // A null context field is only reachable as request input; the strict
        // decode must refuse before any state change.
        let c = cmd(
            "strict-context",
            json!({"mandate_id":"data-mandate","resource":"invoice/acme","amount_minor":100,"currency":"EUR","beneficiary":"ACME","idempotency_key":"strict-context","data_context":ctx}),
        );
        let err = f.try_call("request", &c, "fixture-requester-token").err().unwrap();
        assert_eq!(err.code(), "format", "context-{key}: null context field accepted");
    }
    for (name, edit) in [
        ("null-tools", Box::new(|p: &mut Value| {
            p.as_object_mut().unwrap().remove("tools");
        }) as Box<dyn Fn(&mut Value)>),
        ("duplicate-classifications", Box::new(|p: &mut Value| {
            p["classifications"] = json!(["public", "public"]);
        })),
        ("unknown-destination", Box::new(|p: &mut Value| {
            p["destinations"] = json!(["https://example.invalid"]);
        })),
        ("unknown-provider", Box::new(|p: &mut Value| {
            p["providers"] = json!(["external"]);
        })),
        ("unknown-tool", Box::new(|p: &mut Value| {
            p["tools"] = json!(["/bin/cat"]);
        })),
    ] {
        let f = Fixture::new();
        f.call("config-apply", &config("config", 0), "");
        let mut p = active_policy();
        p["enabled"] = json!(false);
        edit(&mut p);
        let before = f.state_bytes();
        let err = f
            .try_call("config-apply", &policy_change("policy-invalid", 1, p), "")
            .err()
            .unwrap();
        assert_eq!(err.code(), "format", "{name}: invalid typed policy accepted");
        assert_eq!(f.state_bytes(), before, "{name}: invalid policy mutated");
    }
}

/// Approve and commit re-admit the recorded context of a pre-policy action:
/// an unclassified stored action can never cross an effect boundary after the
/// policy activates, and each denial stays without mutation.
#[test]
fn data_policy_checks_approval_and_first_effect() {
    for operation in ["approve", "commit"] {
        let f = Fixture::new();
        f.call("config-apply", &config("config", 0), "");
        data_mandate(&f, "data-mandate");
        let a = f.call(
            "request",
            &cmd("old-request", json!({"mandate_id":"data-mandate","resource":"invoice/acme","amount_minor":100,"currency":"EUR","beneficiary":"ACME","idempotency_key":"old-key"})),
            "fixture-requester-token",
        );
        let a = if operation == "commit" {
            approve_action(&f, "old-approve", &a)
        } else {
            a
        };
        enable_data_policy(&f, 1);
        let mut c = json!({"action_id":a["id"],"content_digest":a["content_digest"],"mandate_id":a["mandate_id"],"amount_minor":a["amount_minor"],"currency":a["currency"],"beneficiary":a["beneficiary"]});
        c["schema_version"] = 1.into();
        c["event_id"] = json!("new-step");
        let token = if operation == "commit" {
            "fixture-executor-token"
        } else {
            "fixture-approver-token"
        };
        let before = f.state_bytes();
        let err = f.try_call(operation, &c, token).err().unwrap();
        assert_eq!(err.code(), "authorization_required", "{operation}: unclassified action proceeded");
        assert_eq!(f.state_bytes(), before, "{operation}: denied step mutated");
    }
}

/// A disabled policy refuses labelled input without mutation; the enabled
/// policy persists across reopen; and an approval command can never replace
/// the stored context with a supplied one.
#[test]
fn data_policy_persistence_and_disabled() {
    let f = Fixture::new();
    f.call("config-apply", &config("config", 0), "");
    let c = data_request("disabled-data");
    let before = f.state_bytes();
    let err = f
        .try_call("request", &c, "fixture-requester-token")
        .err()
        .unwrap();
    assert_eq!(err.code(), "authorization_required", "disabled policy accepted labelled input");
    assert_eq!(f.state_bytes(), before, "disabled admission mutated");
    enable_data_policy(&f, 1);
    // Plain configuration without data_policy keeps the stored policy.
    f.call("config-apply", &config("config-plain", 2), "");
    let view = f.call("show", &json!({}), "");
    assert_eq!(
        view["data_policy"]["enabled"], true,
        "policy not preserved"
    );
    // Approval with a context and an absent action refuses: context on a
    // non-request operation is refused before the target lookup.
    let mut c2 = cmd(
        "approval-substitution",
        json!({"action_id":"not-present","mandate_id":"data-mandate","resource":"invoice/acme","amount_minor":100,"currency":"EUR","beneficiary":"ACME","data_context":allowed_context()}),
    );
    c2["content_digest"] = json!("ignored");
    let before = f.state_bytes();
    let err = f
        .try_call("approve", &c2, "fixture-approver-token")
        .err()
        .unwrap();
    assert_eq!(err.code(), "authorization_required", "approval replaced context");
    assert_eq!(f.state_bytes(), before, "substitution mutated");
}

/// Resource classification is enforced at request time for every narrowed
/// policy shape, and invalid resource rules refuse at configuration even
/// while disabled.
#[test]
fn data_policy_resource_classification() {
    for (mode, policy) in [
        (
            "uncatalogued",
            json!({"resources":[{"resource":"other","classification":"internal"}],"enabled":true,"classifications":["public","internal","confidential","restricted"],"destinations":["local-ledger"],"providers":["swarm"],"tools":["simulated-settlement"]}),
        ),
        (
            "downgraded",
            json!({"resources":[{"resource":"invoice/acme","classification":"restricted"}],"enabled":true,"classifications":["public","internal","confidential","restricted"],"destinations":["local-ledger"],"providers":["swarm"],"tools":["simulated-settlement"]}),
        ),
        (
            "empty-rules",
            json!({"resources":[],"enabled":true,"classifications":["public","internal","confidential","restricted"],"destinations":["local-ledger"],"providers":["swarm"],"tools":["simulated-settlement"]}),
        ),
    ] {
        let f = Fixture::new();
        f.call("config-apply", &config("config", 0), "");
        f.call("config-apply", &policy_change("policy-narrow", 1, policy), "");
        data_mandate(&f, "data-mandate");
        let c = data_request("classification-denied");
        let before = f.state_bytes();
        let err = f
            .try_call("request", &c, "fixture-requester-token")
            .err()
            .unwrap();
        assert_eq!(err.code(), "authorization_required", "{mode}: resource classification not enforced");
        assert_eq!(f.state_bytes(), before, "{mode}: classification denial mutated state");
    }
    for (name, rules) in [
        ("null-rules", Value::Null),
        (
            "empty-resource",
            json!([{"resource":"","classification":"internal"}]),
        ),
        (
            "empty-classification",
            json!([{"resource":"invoice/acme","classification":""}]),
        ),
        (
            "unknown-classification",
            json!([{"resource":"invoice/acme","classification":"unknown"}]),
        ),
        (
            "duplicate-resource",
            json!([{"resource":"invoice/acme","classification":"internal"},{"resource":"invoice/acme","classification":"public"}]),
        ),
    ] {
        let f = Fixture::new();
        f.call("config-apply", &config("config", 0), "");
        let mut p = active_policy();
        p["enabled"] = json!(false);
        p["resources"] = rules;
        // Rules carry through a separate typed edit; the supplied block keeps
        // the active-shaped core with the tested resource list.
        let before = f.state_bytes();
        let err = f
            .try_call("config-apply", &policy_change("policy-rules", 1, p), "")
            .err()
            .unwrap();
        assert_eq!(err.code(), "format", "{name}: invalid resource policy accepted");
        assert_eq!(f.state_bytes(), before, "{name}: invalid resource policy mutated state");
    }
}
