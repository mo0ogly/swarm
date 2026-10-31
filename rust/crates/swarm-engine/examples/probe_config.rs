// Temporary probe: reproduce the fixture config-apply refusal with its reason.
use swarm_engine::{RootStore, SystemEnvironment};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn main() {
    let proof = PathBuf::from(
        std::env::var("SWARM_PAEA_SOURCE_FIXTURE").expect("fixture env"),
    );
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(proof.join("result.json")).unwrap()).unwrap();
    let path = PathBuf::from(value["root"].as_str().unwrap());
    let root = RootStore::open(&path, true).unwrap();
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join(".swarm/trust-actions.json")).unwrap()).unwrap();
    let revision = state["revision"].as_i64().unwrap();
    let event = format!("git-call-http-agreement-config-{revision}");
    let mut identities = vec![
        json!({"subject":"enrolled-effect","integrity":3,"roles":["admin","effect","approver"],"enrollment_principal":{"principal_id":"operator","principal_kind":"human","issuer":"https://fixture.invalid","subject":"stable-person"}}),
        json!({"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]}),
        json!({"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]}),
        json!({"subject":"agreement-admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}),
    ];
    let _ = &mut identities;
    let config = serde_json::to_vec(&json!({
        "schema_version":1,
        "event_id":event,
        "expected_revision":revision,
        "reason":"Explicit fixture effect role for Git call HTTP",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":false},
        "identities":identities,
    })).unwrap();
    match root.trust("config-apply", &config, "") {
        Ok(_) => println!("APPLIED at {revision}"),
        Err(e) => println!("REFUSED: {}", e),
    }
    let _ = root.close();
    let _ = SystemEnvironment;
}
