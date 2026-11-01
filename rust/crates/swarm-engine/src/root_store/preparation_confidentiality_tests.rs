//! Source-derived BLP diagnostic on an owned native-row fixture. Native-row
//! insertion exercises the consumer, not admission/dispatch or a real provider.
use super::*;
use serde_json::{json, Value};

/// Run public Trust operations and retain the actual structured result/error.
fn call(root: &RootStore, op: &str, request: Value, token: &str) -> Result<Value, E> {
    root.trust_as_principal(op, &serde_json::to_vec(&request).unwrap(), token)
        .map(|r| serde_json::from_slice(&r.bytes).unwrap())
}
/// Explicit fixture policy: worker can read only compartment a, admin a+b.
fn policy(revision: u64) -> Value {
    json!({"schema_version":1,"event_id":format!("policy-{revision}"),"expected_revision":revision,
        "expected_trust_revision":1,"reason":"owned fixture",
        "policy":{"schema_version":1,"levels":[{"id":"public","name_fr":"Public","name_en":"Public"},
            {"id":"secret","name_fr":"Secret","name_en":"Secret"}],"compartments":["a","b"],
            "clearances":{"admin":{"level":"secret","compartments":["a","b"]},
                "worker":{"level":"secret","compartments":["a"]}}}})
}
/// Extract static safe causes for assertions; never render fixture prompt text.
fn codes(error: E) -> String {
    let mut result=error.to_string();
    let mut cause=std::error::Error::source(&error);
    while let Some(error)=cause { result.push_str(&format!(":{error}")); cause=error.source(); }
    result
}

#[test]
fn recorded_sources_authentication_join_refusal_and_recovery() {
    let area=std::env::var_os("TMPDIR").expect("recipe fixture area required");
    assert!(!std::path::Path::new(&area).canonicalize().unwrap().starts_with("/tmp"));
    let dir=tempfile::tempdir_in(area).unwrap();
    let root=RootStore::initialize(dir.path(), &mut crate::SystemEnvironment).unwrap();
    call(&root,"config-apply",json!({"schema_version":1,"event_id":"init","expected_revision":0,
        "reason":"owned fixture","policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,
            "evidence_retention_days":365,"require_separation":true},
        "identities":[{"subject":"admin","integrity":3,"roles":["admin"],"token":"fixture-admin-token"},
            {"subject":"worker","integrity":1,"roles":["requester"],"token":"fixture-worker-token"},
            {"subject":"approver","integrity":2,"roles":["approver"],"token":"fixture-approver-token"}]}),"").unwrap();
    call(&root,"blp-policy-apply",policy(0),"fixture-admin-token").unwrap();
    let (turn,mut native)=super::level_tests::provenance_row();
    native.root=root.instance().unwrap();
    let original=encoded(&turn,&native).unwrap();
    root.db.execute("INSERT INTO preparations(id,revision,body) VALUES('preparation',1,'{}')",[]).unwrap();
    root.db.execute("INSERT INTO preparation_turns(id,preparation_id,status,body) VALUES('turn','preparation','pending',?1)",[&original]).unwrap();
    let request=json!({"schema_version":1,"expected_revision":1,"preparation_id":"preparation","turn_id":"turn"});
    let state_path=dir.path().join(".swarm/trust-actions.json");
    let before=std::fs::read(&state_path).unwrap();
    for token in ["","unknown-fixture-token"] {
        assert!(codes(call(&root,"blp-preparation-check",request.clone(),token).unwrap_err()).contains("authorization_required"));
    }
    let mut missing=request.clone();missing["turn_id"]="missing".into();
    assert!(codes(call(&root,"blp-preparation-check",missing,"fixture-approver-token").unwrap_err()).contains("blp-access-denied"));
    assert!(codes(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap_err()).contains("blp-classification-required"));
    assert_eq!(before,std::fs::read(&state_path).unwrap());
    let manifest=serde_json::to_value(native.prompt_provenance.as_ref().unwrap()).unwrap();
    let mut digests=std::collections::BTreeSet::new();
    let mut last=None;
    for part in manifest["parts"].as_array().unwrap() {
        let digest=part["sha256"].as_str().unwrap();
        if !digests.insert(digest) { continue; }
        let secret=part["origin"]["field"]=="user_message";
        let compartments=if secret { json!(["a","b"]) } else { json!(["a"]) };
        let attr=json!({"schema_version":1,"event_id":format!("source-{}",digests.len()),"expected_revision":1,
            "expected_trust_revision":1,"canonical_digest":digest,"project_path":dir.path().to_str().unwrap(),
            "classification":{"level":if secret {"secret"} else {"public"},"compartments":compartments},"reason":"exact source fixture"});
        // Keep one actual source absent to prove there is no partial-success fallback.
        if secret { last=Some(attr); } else { call(&root,"blp-object-attribute",attr,"fixture-admin-token").unwrap(); }
    }
    assert!(codes(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap_err()).contains("blp-classification-required"));
    call(&root,"blp-object-attribute",last.unwrap(),"fixture-admin-token").unwrap();
    let witness=call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap();
    assert_eq!(witness["output_classification"],json!({"level":"secret","compartments":["a","b"]}));
    assert_eq!(witness["source_count"],manifest["parts"].as_array().unwrap().len());
    assert_eq!(witness["runtime_authorized"],false); assert_eq!(witness["diagnostic_only"],true);
    assert_eq!(witness["actor"],"admin");
    assert!(!witness.to_string().contains("fixture-user-message"));
    let before=std::fs::read(&state_path).unwrap();
    assert!(codes(call(&root,"blp-preparation-check",request.clone(),"fixture-worker-token").unwrap_err()).contains("blp-access-denied"));
    let mut bad=request.clone();bad["expected_revision"]=0.into();
    assert!(codes(call(&root,"blp-preparation-check",bad,"fixture-admin-token").unwrap_err()).contains("blp-policy-stale"));
    let mut bad=request.clone();bad["classification"]=json!({"level":"public","compartments":[]});
    assert!(call(&root,"blp-preparation-check",bad,"fixture-admin-token").is_err());
    let mut bad=request.clone();bad["preparation_id"]="different".into();
    assert!(codes(call(&root,"blp-preparation-check",bad,"fixture-admin-token").unwrap_err()).contains("blp-preparation-parent-mismatch"));
    let mut changed=turn;changed.prompt.push('x');
    root.db.execute("UPDATE preparation_turns SET body=?1",[encoded(&changed,&native).unwrap()]).unwrap();
    assert!(codes(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap_err()).contains("preparation-prompt-provenance-invalid"));
    root.db.execute("UPDATE preparation_turns SET body=?1",[&original]).unwrap();
    assert_eq!(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap(),witness);
    let mut historical=sample_legacy(&native);
    root.db.execute("UPDATE preparation_turns SET body=?1",[encoded(&changed,&historical).unwrap()]).unwrap();
    assert!(codes(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap_err()).contains("blp-preparation-provenance-required"));
    root.db.execute("UPDATE preparation_turns SET body=?1",[&original]).unwrap();
    assert_eq!(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap(),witness);
    historical=serde_json::from_slice(&serde_json::to_vec(&native).unwrap()).unwrap();
    historical.root.directory.0+=1;
    root.db.execute("UPDATE preparation_turns SET body=?1",[encoded(&changed,&historical).unwrap()]).unwrap();
    // Restore the actual prompt first so provenance cannot mask Root rejection.
    changed.prompt.pop();
    root.db.execute("UPDATE preparation_turns SET body=?1",[encoded(&changed,&historical).unwrap()]).unwrap();
    assert!(codes(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap_err()).contains("preparation-root-instance-changed"));
    root.db.execute("UPDATE preparation_turns SET body=?1",[&original]).unwrap();
    assert_eq!(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap(),witness);
    assert_eq!(before,std::fs::read(&state_path).unwrap());
    root.close().unwrap();
    let root=RootStore::open(dir.path(),false).unwrap();
    assert_eq!(call(&root,"blp-preparation-check",request.clone(),"fixture-admin-token").unwrap(),witness);
    root.close().unwrap();
    let root=RootStore::open(dir.path(),true).unwrap();
    call(&root,"blp-policy-apply",policy(1),"fixture-admin-token").unwrap();
    let mut renewed=request;renewed["expected_revision"]=2.into();
    assert!(codes(call(&root,"blp-preparation-check",renewed,"fixture-admin-token").unwrap_err()).contains("blp-classification-required"));
    root.close().unwrap();
}

/// Historical native shape on the same fixture Root; no provenance is invented.
fn sample_legacy(native: &Native) -> Native {
    let mut legacy: Native=serde_json::from_slice(&serde_json::to_vec(native).unwrap()).unwrap();
    legacy.version=1;legacy.prompt_provenance=None;legacy
}
