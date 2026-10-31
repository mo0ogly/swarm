//! Persistent BLP policy/public engine regressions on owned fixture roots.
//! No test invokes a provider or claims runtime information-flow enforcement.
#![cfg(target_os="linux")]
use serde_json::{json, Value};
use swarm_engine::{RootStore, SystemEnvironment};

struct Fixture(std::path::PathBuf);
impl Fixture {
    /// Allocate below the configured test TMPDIR. The recipe supplies the project
    /// fixture area; no live database, service or operator credentials are used.
    fn new() -> Self {
        let area=std::env::var_os("TMPDIR").expect("project-local TMPDIR required");
        let area=std::path::PathBuf::from(area).canonicalize().unwrap();
        assert!(!area.starts_with("/tmp"));
        let path=area.join(swarm_engine::generate_identifier("blp-").unwrap());
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let store=RootStore::initialize(&path,&mut SystemEnvironment).unwrap(); store.close().unwrap();
        let fixture=Self(path);
        fixture.call("config-apply",config(0,"trust-init"),"").unwrap(); fixture
    }
    /// Exercise the public Root consumer and close it on both success/refusal.
    fn call(&self, op:&str, input:Value, token:&str)->Result<Value,String> {
        let store=RootStore::open(&self.0,true).unwrap();
        let result=store.trust_as_principal(op,&serde_json::to_vec(&input).unwrap(),token);
        store.close().unwrap();
        result.map(|r|serde_json::from_slice(&r.bytes).unwrap()).map_err(|e|e.to_string())
    }
    fn state(&self)->Vec<u8>{std::fs::read(self.0.join(".swarm/trust-actions.json")).unwrap()}
    fn apply(&self)->Value{self.call("blp-policy-apply",change(0,"blp-init"),"fixture-admin-token").unwrap()}
}
impl Drop for Fixture {
    fn drop(&mut self){if !std::thread::panicking(){std::fs::remove_dir_all(&self.0).unwrap();}}
}
fn config(revision:u64,event:&str)->Value {
    json!({"schema_version":1,"event_id":event,"expected_revision":revision,"reason":"Owned isolated BLP fixture",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
        "identities":[{"subject":"admin","integrity":3,"roles":["admin"],"token":"fixture-admin-token"},
        {"subject":"worker","integrity":1,"roles":["requester"],"token":"fixture-worker-token"}]})
}
fn change(revision:u64,event:&str)->Value {
    json!({"schema_version":1,"event_id":event,"expected_revision":revision,"expected_trust_revision":1,"reason":"Explicit confidentiality policy fixture",
        "policy":{"schema_version":1,"levels":[{"id":"public","name_fr":"Public","name_en":"Public"},{"id":"confidential","name_fr":"Confidentiel","name_en":"Confidential"}],
        "compartments":["client-a","client-b"],"clearances":{"worker":{"level":"confidential","compartments":["client-a"]}}}})
}
fn check(destination:Value)->Value {
    json!({"schema_version":1,"expected_revision":1,"sources":[{"level":"confidential","compartments":["client-a"]}],"destination":destination})
}

#[test]
fn policy_preview_apply_reopen_and_replay_preserve_history() {
    let f=Fixture::new(); let before=f.state();
    let preview=f.call("blp-policy-preview",change(0,"blp-init"),"fixture-admin-token").unwrap();
    assert_eq!(preview["applied"],false); assert_eq!(f.state(),before);
    let applied=f.apply(); assert_eq!(applied["proposed_revision"],"1"); assert_eq!(applied["runtime_enforced"],false);
    let after=f.state(); assert_eq!(f.apply(),applied); assert_eq!(f.state(),after);
    let view=f.call("blp-show",json!({}),"fixture-admin-token").unwrap();
    assert_eq!(view["record"]["history"].as_array().unwrap().len(),1);
    assert_eq!(view["record"]["history"][0]["actor"],"admin");
    assert_eq!(view["record"]["history"][0]["policy"],change(0,"blp-init")["policy"]);
    assert_eq!(view["capability"]["runtime_enforced"],false);
}

#[test]
fn session_local_operator_or_forged_level_cannot_configure_blp() {
    let f=Fixture::new(); f.apply(); let before=f.state();
    for token in ["","fixture-worker-token","unknown-private-token"] {
        for op in ["blp-show","blp-policy-preview","blp-policy-apply"] {
            let input=if op=="blp-show"{json!({})}else{change(1,"unauthorized")};
            assert!(f.call(op,input,token).unwrap_err().contains("authorization_required"));
        }
    }
    let mut forged=change(1,"forged"); forged["actor"]="admin".into(); forged["integrity"]=3.into();
    assert!(f.call("blp-policy-apply",forged,"fixture-admin-token").is_err());
    assert_eq!(f.state(),before);
}

#[test]
fn supplied_label_diagnostics_are_readonly_and_never_runtime_authority() {
    let f=Fixture::new(); f.apply(); let before=f.state();
    let allowed=f.call("blp-check",check(json!({"level":"confidential","compartments":["client-a"]})),"fixture-worker-token").unwrap();
    assert_eq!(allowed["allowed_by_supplied_labels"],true); assert_eq!(allowed["runtime_authorized"],false);
    for target in [json!({"level":"public","compartments":[]}),json!({"level":"confidential","compartments":["client-b"]}),Value::Null] {
        let denied=f.call("blp-check",check(target),"fixture-worker-token").unwrap();
        assert_eq!(denied["allowed_by_supplied_labels"],false); assert_eq!(denied["runtime_authorized"],false);
        assert!(denied.get("output_classification").is_none());
    }
    let mut unknown=check(json!({"level":"public","compartments":[]})); unknown["sources"]=json!([null]);
    assert_eq!(f.call("blp-check",unknown,"fixture-worker-token").unwrap()["reason"],"blp-classification-required");
    assert_eq!(f.state(),before);
}

#[test]
fn conflicts_catalog_changes_and_unrecognized_grants_do_not_mutate() {
    let f=Fixture::new(); f.apply(); let before=f.state();
    let mut cases=vec![change(0,"stale"),change(1,"blp-init")];
    let mut changed=change(1,"bad-order");changed["policy"]["levels"].as_array_mut().unwrap().reverse();cases.push(changed);
    changed=change(1,"unknown-grant");changed["policy"]["clearances"]["intruder"]=json!({"level":"public","compartments":[]});cases.push(changed);
    changed=change(1,"bad-schema");changed["policy"]["schema_version"]=2.into();cases.push(changed);
    for proposal in cases {assert!(f.call("blp-policy-apply",proposal,"fixture-admin-token").is_err());assert_eq!(f.state(),before);}
}

#[test]
fn trust_revision_policy_revocation_and_old_replays_are_not_reactivated() {
    let f=Fixture::new(); f.apply();
    let mut revoked=change(1,"revoke-clearance");revoked["policy"]["clearances"]=json!({});
    f.call("blp-policy-apply",revoked,"fixture-admin-token").unwrap();
    let before=f.state(); assert!(f.call("blp-policy-apply",change(0,"blp-init"),"fixture-admin-token").unwrap_err().contains("blp-replay-stale"));
    assert_eq!(f.state(),before);
    f.call("config-apply",config(1,"trust-change"),"fixture-admin-token").unwrap();
    let view=f.call("blp-show",json!({}),"fixture-admin-token").unwrap(); assert_eq!(view["capability"]["authority_current"],false);
    let before=f.state(); let mut input=check(json!({"level":"confidential","compartments":["client-a"]}));input["expected_revision"]=2.into();
    assert!(f.call("blp-check",input,"fixture-worker-token").unwrap_err().contains("blp-policy-stale")); assert_eq!(f.state(),before);
    let mut recovered=change(2,"explicit-renewal");recovered["expected_trust_revision"]=2.into();
    assert_eq!(f.call("blp-policy-apply",recovered,"fixture-admin-token").unwrap()["proposed_revision"],"3");
}
