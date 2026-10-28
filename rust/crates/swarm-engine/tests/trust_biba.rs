#![cfg(target_os = "linux")]
use serde_json::{json, Value};
use swarm_engine::{RootStore, SystemEnvironment};
struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(swarm_engine::generate_identifier("biba-").unwrap());
        std::fs::create_dir(&p).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        let s = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
        s.close().unwrap();
        Self(p)
    }
    fn call(&self, op: &str, v: Value, token: &str) -> Value {
        let s = RootStore::open(&self.0, true).unwrap();
        let r = s
            .trust(op, &serde_json::to_vec(&v).unwrap(), token)
            .unwrap();
        s.close().unwrap();
        serde_json::from_slice(&r.bytes).unwrap()
    }
    fn config(&self) {
        self.call("config-apply",json!({"schema_version":1,"event_id":"config","expected_revision":0,"reason":"Private isolated fixture","policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},"identities":[{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},{"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin","effect"]}]}),"");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn cmd(id: &str, v: Value) -> Value {
    let mut v = v;
    v["schema_version"] = 1.into();
    v["event_id"] = id.into();
    v
}

/// An exact admitted action is the valid control before testing response labels.
fn response_fixture() -> (Fixture, Value) {
    let f = Fixture::new(); f.config();
    f.call("mandate-create",cmd("response-mandate",json!({"mandate_id":"response","subject":"requester","actions":["settle"],"resource":"audit/response","max_amount_minor":1000,"currency":"EUR","beneficiary":"supplier"})),"fixture-admin-token");
    let a=f.call("request",cmd("response-request",json!({"mandate_id":"response","resource":"audit/response","amount_minor":450,"currency":"EUR","beneficiary":"supplier","idempotency_key":"response"})),"fixture-requester-token");
    f.call("approve",cmd("response-approve",json!({"action_id":a["id"],"mandate_id":"response","content_digest":a["content_digest"],"resource":"audit/response","amount_minor":450,"currency":"EUR","beneficiary":"supplier"})),"fixture-approver-token");
    (f,a)
}

/// I3 invocation may engage one effect, but cannot read its lower I2 object in
/// either a new response or a replay. The status acknowledgement is trusted.
#[test]
fn biba_effect_responses_and_replays_do_not_read_down() {
    let (f,a)=response_fixture();
    let v=cmd("response-commit",json!({"action_id":a["id"],"lose_response":true}));
    for input in [v.clone(),v,cmd("response-known",json!({"action_id":a["id"]}))] {
        let r=f.call("commit",input,"fixture-admin-token");
        assert_eq!(r["kind"],"trust-status-ack-v1");
        assert_eq!(r["state"],"unknown");
        assert_eq!(r["object_id"],a["id"]);
        for key in ["amount_minor","currency","beneficiary","resource","original_digest","content_digest"] { assert!(r.get(key).is_none(),"{key}"); }
    }
    // A real historical reply may contain the full object. Replace only that
    // raw reply in this private fixture, preserving all signed nested bytes.
    let path=f.0.join(".swarm/trust-actions.json");
    type RawMap=std::collections::BTreeMap<String,Box<serde_json::value::RawValue>>;
    let mut outer:RawMap=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let actions:RawMap=serde_json::from_str(outer["actions"].get()).unwrap();
    let mut replies:RawMap=serde_json::from_str(outer["event_replies"].get()).unwrap();
    replies.insert("response-commit".into(),serde_json::value::RawValue::from_string(actions[a["id"].as_str().unwrap()].get().to_owned()).unwrap());
    outer.insert("event_replies".into(),serde_json::value::RawValue::from_string(serde_json::to_string(&replies).unwrap()).unwrap());
    std::fs::write(&path,serde_json::to_vec(&outer).unwrap()).unwrap();
    let replay=f.call("commit",cmd("response-commit",json!({"action_id":a["id"],"lose_response":true})),"fixture-admin-token");
    assert_eq!(replay["kind"],"trust-status-ack-v1");assert!(replay.get("amount_minor").is_none());
    let r=f.call("reconcile",cmd("response-reconcile",json!({"action_id":a["id"]})),"fixture-admin-token");
    assert_eq!(r["kind"],"trust-status-ack-v1");assert_eq!(r["state"],"reconciled");
    assert_eq!(f.call("show",json!({}),"")["ledger"].as_array().unwrap().len(),1);
    let s=RootStore::open(&f.0,false).unwrap();
    let visible=s.trust("read-object",&serde_json::to_vec(&cmd("response-read-i2",json!({"action_id":a["id"]}))).unwrap(),"fixture-approver-token").unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&visible.bytes).unwrap()["amount_minor"],450);
    assert_eq!(s.trust("read-object",&serde_json::to_vec(&cmd("response-read",json!({"action_id":a["id"]}))).unwrap(),"fixture-admin-token").err().unwrap().code(),"authorization_required");s.close().unwrap();
}

/// Native principal configuration requires current I3 even on a retained replay;
/// the separate local operator path remains available for explicit recovery.
#[test]
fn biba_principal_admin_cannot_promote_itself_or_replay_after_withdrawal() {
    let f=Fixture::new();f.config();
    let raw=std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
    let state:Value=serde_json::from_slice(&raw).unwrap();
    let mut c=json!({"schema_version":1,"event_id":"principal-config","expected_revision":1,"reason":"Explicit principal change","policy":state["policy"],"identities":[{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},{"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin","effect"]}]});
    let s=RootStore::open(&f.0,true).unwrap();
    for op in ["config-preview","config-apply"] {for token in ["","fixture-requester-token","fixture-approver-token"] {
        assert_eq!(s.trust_as_principal(op,&serde_json::to_vec(&c).unwrap(),token).err().unwrap().code(),"authorization_required");
        assert_eq!(std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap(),raw);
    }}
    assert!(s.trust_as_principal("config-apply",&serde_json::to_vec(&c).unwrap(),"fixture-admin-token").is_ok());
    c["event_id"]="withdraw-admin".into();c["expected_revision"]=2.into();c["identities"][2]["disabled"]=true.into();
    s.trust("config-apply",&serde_json::to_vec(&c).unwrap(),"").unwrap();
    c["event_id"]="principal-config".into();c["expected_revision"]=1.into();c["identities"][2].as_object_mut().unwrap().remove("disabled");
    assert_eq!(s.trust_as_principal("config-apply",&serde_json::to_vec(&c).unwrap(),"fixture-admin-token").err().unwrap().code(),"authorization_required");s.close().unwrap();
    let empty=Fixture::new();let s=RootStore::open(&empty.0,true).unwrap();c["event_id"]="native-bootstrap".into();c["expected_revision"]=0.into();
    assert!(s.trust_as_principal("config-apply",&serde_json::to_vec(&c).unwrap(),"").is_ok());
    assert_eq!(s.trust_as_principal("config-apply",&serde_json::to_vec(&c).unwrap(),"").err().unwrap().code(),"authorization_required");s.close().unwrap();
}

/// Known active, malformed, misspelled and duplicate policy input must refuse
/// before publication; unported blocks are retained, never silently configured.
#[test]
fn biba_extension_fields_and_unported_edits_refuse_without_mutation() {
    let f=Fixture::new();f.config();let path=f.0.join(".swarm/trust-actions.json");let before=std::fs::read(&path).unwrap();
    let st:Value=serde_json::from_slice(&before).unwrap();
    let base=json!({"schema_version":1,"event_id":"bad-extension","expected_revision":1,"reason":"Explicit invalid extension","policy":st["policy"],"identities":[{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},{"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin","effect"]}]});
    let s=RootStore::open(&f.0,true).unwrap();
    for (name,value,code) in [
        ("oidc",json!({"Enabled":true}),"format"),
        ("oidc",json!({"enabled":false,"clients":"not-an-array"}),"format"),
        ("workloads",json!({"enabled":false,"clients":"not-an-array"}),"format"),
        ("data_policy",json!({"enabled":false,"unexpected_authority":true}),"format"),
        ("oidc",json!({"enabled":true}),"command_not_ported"),
        ("data_policy",json!({"enabled":true}),"command_not_ported"),
    ] {
        let mut c=base.clone();c[name]=value;
        for op in ["config-preview","config-apply"] {
            assert_eq!(s.trust(op,&serde_json::to_vec(&c).unwrap(),"").err().unwrap().code(),code);
            assert_eq!(std::fs::read(&path).unwrap(),before);
        }
    }
    assert_eq!(serde_json::from_slice::<Value>(&s.trust("config-preview",&serde_json::to_vec(&base).unwrap(),"").unwrap().bytes).unwrap()["applied"],false);
    let encoded=serde_json::to_string(&base).unwrap();
    let duplicate=format!("{{\"oidc\":{{\"enabled\":true}},\"oidc\":null,{}",&encoded[1..]);
    for op in ["config-preview","config-apply"] {
        assert_eq!(s.trust(op,duplicate.as_bytes(),"").err().unwrap().code(),"format");
        assert_eq!(std::fs::read(&path).unwrap(),before);
    }
    let raw=serde_json::to_string(&base).unwrap();let raw=format!("{{\"oidc\":{{\"enabled\":true,\"enabled\":false}},{}",&raw[1..]);
    assert_eq!(s.trust("config-apply",raw.as_bytes(),"").err().unwrap().code(),"format");s.close().unwrap();
}

/// Canonical legacy nil lists carry no federated authority. They must not turn
/// an inactive retained configuration into a broken local identity installation.
#[test]
fn biba_inactive_federation_nil_lists_preserve_local_admission() {
    let f=Fixture::new();f.config();let path=f.0.join(".swarm/trust-actions.json");
    let mut st:Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    st["oidc"]=json!({"enabled":false,"mappings":null});st["workloads"]=json!({"enabled":false,"clients":null});
    std::fs::write(&path,serde_json::to_vec(&st).unwrap()).unwrap();
    assert_eq!(f.call("direct-write",cmd("inactive-nil",json!({"resource":"official"})),"fixture-requester-token")["state"],"denied");
}
/// Positive authorized path precedes bypass/refusal witnesses; real Root IO and
/// private key persistence, no network/provider or OS confinement is claimed.
#[test]
fn biba_exact_approval_unique_effect_lost_response_reconciliation_and_proof() {
    let f = Fixture::new();
    f.config();
    let m=f.call("mandate-create",cmd("mandate",json!({"mandate_id":"mandate","subject":"requester","actions":["settle"],"resource":"invoice/one","max_amount_minor":1000,"currency":"EUR","beneficiary":"supplier"})),"fixture-admin-token");
    assert_eq!(m["kind"], "trust-status-ack-v1");
    assert_eq!(f.call("show",json!({}),"")["mandates"]["mandate"]["integrity"],2);
    let request = cmd(
        "request",
        json!({"mandate_id":"mandate","resource":"invoice/one","amount_minor":450,"currency":"EUR","beneficiary":"supplier","idempotency_key":"single"}),
    );
    let a = f.call("request", request.clone(), "fixture-requester-token");
    assert_eq!(a["state"], "pending_approval");
    let mut approval = cmd(
        "approve",
        json!({"action_id":a["id"],"mandate_id":"mandate","content_digest":a["content_digest"],"resource":"invoice/one","amount_minor":451,"currency":"EUR","beneficiary":"supplier"}),
    );
    assert_eq!(
        f.call("approve", approval.clone(), "fixture-approver-token")["state"],
        "denied"
    );
    approval["amount_minor"] = 450.into();
    approval["event_id"] = "approve-exact".into();
    assert_eq!(
        f.call("approve", approval, "fixture-approver-token")["state"],
        "approved"
    );
    let effect = cmd("commit", json!({"action_id":a["id"],"lose_response":true}));
    assert_eq!(
        f.call("commit", effect.clone(), "fixture-admin-token")["state"],
        "unknown"
    );
    assert_eq!(
        f.call("commit", effect, "fixture-admin-token")["state"],
        "unknown"
    );
    let r = f.call(
        "reconcile",
        cmd("reconcile", json!({"action_id":a["id"]})),
        "fixture-admin-token",
    );
    assert_eq!(r["state"], "reconciled");
    assert_eq!(
        f.call("show", json!({}), "")["ledger"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let store = RootStore::open(&f.0, false).unwrap();
    let export = store.trust("export", b"", "").unwrap().bytes;
    let anchor = store.trust("anchor", b"", "").unwrap().bytes;
    store.close().unwrap();
    let key = std::fs::read_to_string(f.0.join(".swarm/trust-actions-proof.key")).unwrap();
    let proof = format!(
        "{{\"export\":{},\"anchor\":{},\"verification_key\":{}}}",
        String::from_utf8(export).unwrap(),
        String::from_utf8(anchor).unwrap(),
        serde_json::to_string(&key).unwrap()
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&swarm_engine::verify_trust(proof.as_bytes()).unwrap())
            .unwrap()["verified"],
        true
    );
}
/// A valid core precedes an active extension. The extension must reach its
/// explicit unsupported guard rather than fail inside serde's flatten buffer.
#[test]
fn biba_extension_reaches_authoritative_guard_without_mutation() {
    let f = Fixture::new();
    f.config();
    let path = f.0.join(".swarm/trust-actions.json");
    let before = std::fs::read(&path).unwrap();
    let store = RootStore::open(&f.0, true).unwrap();
    let proposal = json!({"schema_version":1,"event_id":"extension","expected_revision":1,"reason":"Explicit extension fixture","policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},"identities":[{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},{"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}],"data_policy":{"enabled":true}});
    let error = store
        .trust("config-apply", &serde_json::to_vec(&proposal).unwrap(), "")
        .err()
        .unwrap();
    assert_eq!(error.code(), "command_not_ported");
    store.close().unwrap();
    assert_eq!(std::fs::read(path).unwrap(), before);
}
/// Independent processes share the historical flock: one action key produces
/// one ledger entry even when distinct commit events race. No provider executes.
#[test]
fn biba_parallel_commit_has_one_native_effect() {
    let f = Fixture::new();
    f.config();
    f.call("mandate-create",cmd("m",json!({"mandate_id":"m","subject":"requester","actions":["settle"],"resource":"invoice/one","max_amount_minor":1000,"currency":"EUR","beneficiary":"supplier"})),"fixture-admin-token");
    let a=f.call("request",cmd("r",json!({"mandate_id":"m","resource":"invoice/one","amount_minor":9,"currency":"EUR","beneficiary":"supplier","idempotency_key":"once"})),"fixture-requester-token");
    f.call("approve",cmd("a",json!({"action_id":a["id"],"mandate_id":"m","resource":"invoice/one","content_digest":a["content_digest"],"amount_minor":9,"currency":"EUR","beneficiary":"supplier"})),"fixture-approver-token");
    let joins: Vec<_> = (0..8)
        .map(|i| {
            let path = f.0.clone();
            let id = a["id"].clone();
            std::thread::spawn(move || {
                let s = RootStore::open(&path, true).unwrap();
                let r = s
                    .trust(
                        "commit",
                        &serde_json::to_vec(&cmd(
                            &format!("parallel-{i}"),
                            json!({"action_id":id}),
                        ))
                        .unwrap(),
                        "fixture-admin-token",
                    )
                    .unwrap();
                s.close().unwrap();
                serde_json::from_slice::<Value>(&r.bytes).unwrap()
            })
        })
        .collect();
    for j in joins {
        assert_eq!(j.join().unwrap()["state"], "committed");
    }
    assert_eq!(
        f.call("show", json!({}), "")["ledger"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
/// Public read access is strict Biba; supplied integrity cannot authorize read
/// down, write up or invocation up, and business denials preserve their reason.
#[test]
fn biba_untrusted_claims_and_role_separation_are_enforced() {
    let f = Fixture::new();
    f.config();
    let v = cmd(
        "forge",
        json!({"actor":"admin","integrity":3,"resource":"official"}),
    );
    assert_eq!(
        f.call("direct-write", v.clone(), "fixture-requester-token")["state"],
        "denied"
    );
    assert_eq!(
        f.call(
            "elevate",
            cmd("elevate", json!({"integrity":3})),
            "fixture-requester-token"
        )["state"],
        "denied"
    );
    assert_eq!(
        f.call(
            "mandate-create",
            cmd("not-admin", json!({"mandate_id":"forged"})),
            "fixture-requester-token"
        )["state"],
        "denied"
    );
    let state = f.call("show", json!({}), "");
    assert!(state["mandates"].as_object().unwrap().is_empty());
    assert_eq!(state["evidence_count"], 3);
}
/// Retained inactive extensions reopen byte-faithfully; losing key material
/// refuses the next transaction without replacing the private key or state.
#[test]
fn biba_raw_extension_reopens_and_key_loss_fails_closed() {
    let f = Fixture::new();
    f.config();
    let path = f.0.join(".swarm/trust-actions.json");
    let mut state: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    state["data_policy"] = json!({"enabled":false,"resources":[]});
    std::fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert_eq!(
        f.call("show", json!({}), "")["data_policy"]["enabled"],
        false
    );
    let key = f.0.join(".swarm/trust-actions-proof.key");
    let saved = std::fs::read(&key).unwrap();
    std::fs::remove_file(&key).unwrap();
    let before = std::fs::read(&path).unwrap();
    let store = RootStore::open(&f.0, true).unwrap();
    assert!(store
        .trust(
            "direct-write",
            &serde_json::to_vec(&cmd("missing-key", json!({"resource":"official"}))).unwrap(),
            "fixture-requester-token"
        )
        .is_err());
    store.close().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!key.exists());
    std::fs::write(&key, saved).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        f.call(
            "direct-write",
            cmd("missing-key", json!({"resource":"official"})),
            "fixture-requester-token"
        )["state"],
        "denied"
    );
}
/// The access connector has a distinct native effect, exact subject/role binding
/// and readback. Tampering in this private fixture must refuse reconciliation.
#[test]
fn biba_access_effect_is_exact_and_cannot_reconcile_substitution(){
 let f=Fixture::new();f.config();let connector=json!({"id":"local-access-simulation","version":1,"access":{"subject":"reader","role":"auditor"}});
 f.call("mandate-create",cmd("access-m",json!({"mandate_id":"access-m","subject":"requester","actions":["grant-access"],"resource":"access/read","connector":connector})),"fixture-admin-token");
 let a=f.call("request",cmd("access-r",json!({"mandate_id":"access-m","resource":"access/read","connector":connector,"idempotency_key":"access-once"})),"fixture-requester-token");
 let mut approval=cmd("access-a",json!({"mandate_id":"access-m","action_id":a["id"],"resource":"access/read","connector":connector,"content_digest":a["content_digest"]}));approval["connector"]["access"]["role"]="wrong".into();assert_eq!(f.call("approve",approval.clone(),"fixture-approver-token")["state"],"denied");approval["event_id"]="access-exact".into();approval["connector"]=connector;assert_eq!(f.call("approve",approval,"fixture-approver-token")["state"],"approved");
 assert_eq!(f.call("commit",cmd("access-c",json!({"action_id":a["id"],"lose_response":true})),"fixture-admin-token")["state"],"unknown");let view=f.call("show",json!({}),"");assert!(view["ledger"].as_array().unwrap().is_empty());assert_eq!(view["access_ledger"][0]["access"]["role"],"auditor");
 let path=f.0.join(".swarm/trust-actions.json");let original=std::fs::read_to_string(&path).unwrap();let index=original.find("\"access_ledger\"").unwrap();let changed=format!("{}{}",&original[..index],original[index..].replacen("auditor","hackerx",1));std::fs::write(&path,changed).unwrap();assert_eq!(f.call("reconcile",cmd("access-altered",json!({"action_id":a["id"]})),"fixture-admin-token")["state"],"denied");std::fs::write(&path,original).unwrap();assert_eq!(f.call("reconcile",cmd("access-recovered",json!({"action_id":a["id"]})),"fixture-admin-token")["state"],"reconciled");
}
/// A mapped principal remains federated even when its provider is disabled.
/// Its old local hash cannot become a static bearer fallback.
#[test]
fn biba_disabled_federation_mapping_has_no_static_fallback(){
 let f=Fixture::new();f.config();let path=f.0.join(".swarm/trust-actions.json");let mut state:Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();state["oidc"]=json!({"enabled":false,"mappings":[{"subject":"requester"}]});let before=serde_json::to_vec(&state).unwrap();std::fs::write(&path,&before).unwrap();let store=RootStore::open(&f.0,true).unwrap();let error=store.trust("direct-write",&serde_json::to_vec(&cmd("no-fallback",json!({}))).unwrap(),"fixture-requester-token").err().unwrap();assert_eq!(error.code(),"command_not_ported");store.close().unwrap();assert_eq!(std::fs::read(path).unwrap(),before);
}
/// Audit metadata must describe the actual retained originals, including a
/// damaged private fixture. A count of zero must never be a hardcoded success.
#[test]
fn biba_audit_reports_missing_originals(){
 let f=Fixture::new();f.config();f.call("direct-write",cmd("original",json!({"resource":"official"})),"fixture-requester-token");assert_eq!(f.call("show",json!({}),"")["originals_missing"],0);let path=f.0.join(".swarm/trust-actions.json");let mut state:Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();state["originals"]=json!({});std::fs::write(&path,serde_json::to_vec(&state).unwrap()).unwrap();assert_eq!(f.call("show",json!({}),"")["originals_missing"],1);
}

/// Null retained federation means absent, without enabling any authority.
#[test]
fn biba_null_federation_blocks_remain_absent() {
    let f=Fixture::new(); f.config();
    let path=f.0.join(".swarm/trust-actions.json");
    let mut state:Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    state["oidc"]=Value::Null; state["workloads"]=Value::Null;
    std::fs::write(&path,serde_json::to_vec(&state).unwrap()).unwrap();
    assert_eq!(f.call("direct-write",cmd("null-block",json!({"resource":"official"})),"fixture-requester-token")["state"],"denied");
}
/// Business commands cannot silently carry ignored configuration fields.
#[test]
fn biba_business_policy_field_refuses_without_mutation() {
    let f=Fixture::new(); f.config();
    assert_eq!(f.call("direct-write",cmd("zero-policy",json!({"resource":"official","policy":{"mandate_ttl_seconds":0,"unknown_reconcile_seconds":0,"evidence_retention_days":0,"require_separation":false}})),"fixture-requester-token")["state"],"denied");
    let path=f.0.join(".swarm/trust-actions.json");
    let before=std::fs::read(&path).unwrap();
    let s=RootStore::open(&f.0,true).unwrap();
    let raw=serde_json::to_vec(&cmd("business-policy",json!({"resource":"official","policy":{"mandate_ttl_seconds":-1,"unknown_reconcile_seconds":0,"evidence_retention_days":0,"require_separation":false}}))).unwrap();
    assert!(s.trust("direct-write",&raw,"fixture-requester-token").is_err());
    s.close().unwrap(); assert_eq!(std::fs::read(&path).unwrap(),before);
}
