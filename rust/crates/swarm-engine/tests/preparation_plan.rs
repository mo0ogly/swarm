//! Plan-verdict receipt rollback on an owned, already-open Root only.
use swarm_engine::{RootStore, SystemEnvironment};
use std::os::unix::fs::DirBuilderExt;

/// A real valid verdict precedes the injected receipt failure. The transaction
/// must retain the original verdict/revision/documents and all row counts, then
/// the identical event succeeds once after dropping only the fixture trigger.
/// No mission, provider, acceptance identity or live datastore is involved.
#[test]
fn plan_verdict_receipt_failure_rolls_back_and_replays() {
    let path=std::env::temp_dir().join(format!("swarm-plan-atomic-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    let root=RootStore::initialize(&path,&mut SystemEnvironment).unwrap();
    let command=|value:serde_json::Value|root.preparation_command(&serde_json::to_vec(&value).unwrap(),&mut SystemEnvironment).unwrap().value;
    let mut p=command(serde_json::json!({"version":1,"action":"create","event_id":"plan-create","expected_revision":0,"text":"need"}));
    p=command(serde_json::json!({"version":1,"action":"save","preparation_id":p.id,"event_id":"brief-save","expected_revision":p.revision,"document":"brief","text":"brief"}));
    p=command(serde_json::json!({"version":1,"action":"adopt-brief","preparation_id":p.id,"event_id":"brief-adopt","expected_revision":p.revision,"sha256":p.documents.as_ref().unwrap()["brief"].sha256}));
    let plan=serde_json::json!({"version":1,"objective":"Fixture plan","assumptions":[],"questions":[],"tasks":[{"id":"t1","title":"Task","role":"worker","scope":"scope","deliverable":"delivery","depends":[],"criteria":["proof"],"proof":"evidence","entry":"entry","validation":"validation","delivery":"delivery","stop":"stop","max_attempts":1,"max_tool_calls":1}]});
    p=command(serde_json::json!({"version":1,"action":"save","preparation_id":p.id,"event_id":"plan-save","expected_revision":p.revision,"document":"plan","text":plan.to_string()}));
    p=command(serde_json::json!({"version":1,"action":"validate-plan","preparation_id":p.id,"event_id":"valid-control","expected_revision":p.revision,"sha256":p.documents.as_ref().unwrap()["plan"].sha256}));
    assert!(p.plan_ready);assert!(p.verdict.is_some());
    let db=rusqlite::Connection::open(path.join(".swarm/state.db")).unwrap();
    let count=|table:&str|db.query_row(&format!("SELECT count(*) FROM {table}"),[],|row|row.get::<_,i64>(0)).unwrap();
    let before=(count("preparation_documents"),count("preparation_commands"));
    let request=serde_json::to_vec(&serde_json::json!({"version":1,"action":"validate-plan","preparation_id":p.id,"event_id":"plan-fault","expected_revision":p.revision,"sha256":p.documents.as_ref().unwrap()["plan"].sha256})).unwrap();
    db.execute_batch("CREATE TRIGGER plan_receipt_fault BEFORE INSERT ON preparation_commands BEGIN SELECT RAISE(ABORT,'PRIVATE_PLAN_FAULT'); END").unwrap();
    let error=root.preparation_command(&request,&mut SystemEnvironment).err().expect("receipt insert must fail");
    assert_eq!(error.code(),"agent-launch-storage");assert!(!error.to_string().contains("PRIVATE_PLAN_FAULT"));assert!(error.recorded_preparation().is_none());
    assert_eq!(root.preparation_show(&p.id).unwrap().value,p);
    assert_eq!((count("preparation_documents"),count("preparation_commands")),before);
    db.execute_batch("DROP TRIGGER plan_receipt_fault").unwrap();
    let recovered=root.preparation_command(&request,&mut SystemEnvironment).unwrap().value;
    assert!(recovered.plan_ready);assert_eq!(recovered.revision,p.revision+1);assert_eq!(recovered.documents,p.documents);
    assert_eq!((count("preparation_documents"),count("preparation_commands")),(before.0,before.1+1));
    assert_eq!(root.preparation_command(&request,&mut SystemEnvironment).unwrap().value,recovered);
    db.close().unwrap();root.close().unwrap();
    let reopened=RootStore::open(&path,false).unwrap();assert_eq!(reopened.preparation_show(&p.id).unwrap().value,recovered);reopened.close().unwrap();std::fs::remove_dir_all(path).unwrap();
}
