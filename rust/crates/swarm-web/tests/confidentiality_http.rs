//! BLP history regression through the real protected HTTP service on a fresh
//! inert Root. No accepted mission, retained source fixture, provider or human
//! declassification is used or claimed. Producer calls use the public engine
//! API: object attribution is not exposed as an HTTP operation yet.
#![cfg(target_os="linux")]
use http::{header::{COOKIE, HOST, ORIGIN}, Request};
use serde_json::{json,Value};
use std::{io::Write,net::TcpListener,path::Path};
use swarm_engine::{RootStore,SystemEnvironment};
use swarm_web::{LocalWebAuthority,NativeHttpService};

const KEY:&str="session-0123456789abcdef01234567";

/// Make a session/Host/Origin/CSRF-protected POST with an independent fixture
/// bearer. Root and operation are fixed by the caller, never response content.
fn post(service:&NativeHttpService,listener:&TcpListener,op:&str,body:Value)->swarm_web::HttpReply {
    let authority=LocalWebAuthority::from_listener(listener,KEY).unwrap();
    service.handle(Request::builder().method("POST")
        .uri(format!("/api/v1/trust-actions?action={op}"))
        .header(HOST,authority.host()).header(COOKIE,format!("{}={KEY}",authority.cookie_name()))
        .header(ORIGIN,format!("http://{}",authority.host())).header("x-swarm-csrf",KEY)
        .header("authorization","Bearer fixture-admin-token")
        .body(serde_json::to_vec(&body).unwrap()).unwrap()).unwrap()
}

/// Publish bytes atomically only within this test's inactive trust journal,
/// with original bytes retained by the caller. No live Root, fake successful
/// admission or identity proof follows from this negative fixture injection.
fn inject(root:&Path,bytes:&[u8]) {
    let directory=root.join(".swarm");
    let mut file=tempfile::NamedTempFile::new_in(&directory).unwrap();
    file.write_all(bytes).unwrap();file.as_file().sync_all().unwrap();
    assert_eq!(std::fs::read(file.path()).unwrap(),bytes);
    file.persist(directory.join("trust-actions.json")).unwrap();
    std::fs::File::open(directory).unwrap().sync_all().unwrap();
}

/// Classify via the existing engine producer, reopen at each operation and
/// preserve close failures. Returns the real result; no HTTP admission is
/// fabricated and no object data is delivered by this helper.
fn attribute(root:&Path,revision:u64,id:&str,level:&str)->Result<(),swarm_engine::RootStoreError> {
    let store=RootStore::open(root,true).unwrap();
    let body=json!({"schema_version":1,"event_id":id,"expected_revision":revision,
        "expected_trust_revision":1,"canonical_digest":"d".repeat(64),"project_path":root.to_str().unwrap(),
        "classification":{"level":level,"compartments":["client-a"]},"reason":"PRIVATE_HTTP_BLP_FIXTURE"});
    let result=store.trust("blp-object-attribute",&serde_json::to_vec(&body).unwrap(),"fixture-admin-token");
    store.close().unwrap();result.map(|_|())
}

/// Demonstrate HTTP bootstrap/policy control, public producer refusal without
/// mutation, equal renewal, poisoned-history HTTP refusal without material,
/// and exact fixture recovery. Session authority is not human declassification.
#[test]
fn policy_renewal_and_poisoned_history_refuse_through_http() {
    use std::os::unix::fs::PermissionsExt;
    let directory=tempfile::tempdir().unwrap();
    std::fs::set_permissions(directory.path(),std::fs::Permissions::from_mode(0o700)).unwrap();
    let root=directory.path();RootStore::initialize(root,&mut SystemEnvironment).unwrap().close().unwrap();
    let listener=TcpListener::bind("127.0.0.1:0").unwrap();
    let service=NativeHttpService::open(root,LocalWebAuthority::from_listener(&listener,KEY).unwrap()).unwrap();
    let config=json!({"schema_version":1,"event_id":"http-trust-init","expected_revision":0,"reason":"Owned inert BLP HTTP fixture",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
        "identities":[{"subject":"admin","integrity":3,"roles":["admin"],"token":"fixture-admin-token"},
            {"subject":"worker","integrity":1,"roles":["requester"],"token":"fixture-worker-token"},
            {"subject":"approver","integrity":2,"roles":["approver"],"token":"fixture-approver-token"}]});
    let reply=post(&service,&listener,"config-apply",config);
    assert!(reply.response.status().is_success(),"bootstrap: {}",String::from_utf8_lossy(reply.response.body()));
    let policy=json!({"schema_version":1,"levels":[{"id":"public","name_fr":"Public","name_en":"Public"},
        {"id":"confidential","name_fr":"Confidentiel","name_en":"Confidential"}],"compartments":["client-a"],
        "clearances":{"admin":{"level":"confidential","compartments":["client-a"]}}});
    for expected in [0,1] {
        let reply=post(&service,&listener,"blp-policy-apply",json!({"schema_version":1,
            "event_id":format!("http-policy-{expected}"),"expected_revision":expected,
            "expected_trust_revision":1,"reason":"HTTP fixture policy renewal","policy":policy}));
        assert!(reply.response.status().is_success(),"policy: {}",String::from_utf8_lossy(reply.response.body()));
        if expected==0 {attribute(root,1,"first","confidential").unwrap();}
    }
    let state=root.join(".swarm/trust-actions.json");let before=std::fs::read(&state).unwrap();
    assert_eq!(attribute(root,2,"lower","public").unwrap_err().code(),"blp-declassification-required");
    assert_eq!(std::fs::read(&state).unwrap(),before);
    attribute(root,2,"equal","confidential").unwrap();
    let valid=std::fs::read(&state).unwrap();
    let control=post(&service,&listener,"blp-show",json!({}));
    assert!(control.response.status().is_success());
    assert!(String::from_utf8_lossy(control.response.body()).contains("PRIVATE_HTTP_BLP_FIXTURE"));
    assert_eq!(std::fs::read(&state).unwrap(),valid);
    let mut poisoned:Value=serde_json::from_slice(&valid).unwrap();
    poisoned["confidentiality"]["history"][1]["attributions"][0]["level"]="public".into();
    let poisoned=serde_json::to_vec(&poisoned).unwrap();inject(root,&poisoned);
    let refused=post(&service,&listener,"blp-show",json!({}));
    assert!(!refused.response.status().is_success());assert!(refused.failure.is_some());
    let body:Value=serde_json::from_slice(refused.response.body()).unwrap();
    assert_eq!(body["failure"]["code"],"trust-proof-integrity");
    let raw=String::from_utf8(refused.response.body().to_vec()).unwrap();
    for material in ["PRIVATE_HTTP_BLP_FIXTURE".to_string(),"d".repeat(64)] {
        assert!(!raw.contains(&material));
        assert!(refused.response.headers().values().all(|v|!v.to_str().unwrap().contains(&material)));
    }
    assert_eq!(std::fs::read(&state).unwrap(),poisoned);
    inject(root,&valid);
    let recovered=post(&service,&listener,"blp-show",json!({}));
    assert!(recovered.response.status().is_success());assert_eq!(std::fs::read(&state).unwrap(),valid);
    service.close().unwrap();
}
