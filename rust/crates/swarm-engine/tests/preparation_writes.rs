//! Atomic Root writer faults on an owned isolated, already-open datastore.
//! Public CLI refuses extra schema objects before commands; this retained-Root
//! fixture injects only test SQL after a valid open, never a live mission DB.
use swarm_engine::{RootStore, SystemEnvironment};
use std::os::unix::fs::DirBuilderExt;

/// An available method is first selected successfully. A receipt-insert fault
/// must roll back both method/hash and revision, without touching document
/// history; dropping only the fixture trigger permits the identical intent.
#[test]
fn preparation_method_receipt_failure_rolls_back_and_recovers() {
    let path=std::env::temp_dir().join(format!("swarm-preparation-method-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    let root=RootStore::initialize(&path,&mut SystemEnvironment).unwrap();
    let p=root.preparation_command(br#"{"version":1,"action":"create","event_id":"method-create","expected_revision":0,"text":"retained"}"#,&mut SystemEnvironment).unwrap().value;
    let raw=serde_json::to_vec(&serde_json::json!({"version":1,"action":"method","preparation_id":p.id,"event_id":"method-control","expected_revision":1,"method":"debug"})).unwrap();
    let current=root.preparation_command(&raw,&mut SystemEnvironment).unwrap().value;
    assert_eq!(current.method,"debug");assert_eq!(current.revision,2);
    let db=rusqlite::Connection::open(path.join(".swarm/state.db")).unwrap();
    let request=serde_json::to_vec(&serde_json::json!({"version":1,"action":"method","preparation_id":p.id,"event_id":"method-fault","expected_revision":2,"method":"apex"})).unwrap();
    let count=|table:&str|db.query_row(&format!("SELECT count(*) FROM {table}"),[],|row|row.get::<_,i64>(0)).unwrap();
    let before=(count("preparation_documents"),count("preparation_commands"));
    db.execute_batch("CREATE TRIGGER preparation_method_fault BEFORE INSERT ON preparation_commands BEGIN SELECT RAISE(ABORT,'PRIVATE_METHOD_FAULT'); END").unwrap();
    let error=root.preparation_command(&request,&mut SystemEnvironment).err().expect("receipt fault must refuse");
    assert_eq!(error.code(),"agent-launch-storage");assert!(error.recorded_preparation().is_none());
    assert_eq!(root.preparation_show(&p.id).unwrap().value,current);
    assert_eq!((count("preparation_documents"),count("preparation_commands")),before);
    db.execute_batch("DROP TRIGGER preparation_method_fault").unwrap();
    let recovered=root.preparation_command(&request,&mut SystemEnvironment).unwrap().value;
    assert_eq!(recovered.method,"apex");assert_eq!(recovered.revision,3);
    assert_eq!(recovered.documents,current.documents);
    assert_eq!((count("preparation_documents"),count("preparation_commands")),(before.0,before.1+1));
    db.close().unwrap();root.close().unwrap();std::fs::remove_dir_all(path).unwrap();
}

/// Valid command, failing child-row insert, complete rollback and same-event
/// recovery. Both history and receipt failures must preserve the original draft,
/// document rows and event ledger. No provider, execution or approval is granted.
#[test]
fn preparation_child_insert_failure_rolls_back_and_same_event_recovers() {
    let path=std::env::temp_dir().join(format!("swarm-preparation-atomic-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    let root=RootStore::initialize(&path,&mut SystemEnvironment).unwrap();
    let mut current=root.preparation_command(br#"{"version":1,"action":"create","event_id":"atomic-create","expected_revision":0,"text":"initial"}"#,&mut SystemEnvironment).unwrap().value;
    let db=rusqlite::Connection::open(path.join(".swarm/state.db")).unwrap();
    for (index,table) in ["preparation_documents","preparation_commands"].iter().enumerate(){
        let request=serde_json::json!({"version":1,"action":"save","preparation_id":current.id,"event_id":format!("atomic-{index}"),"expected_revision":current.revision,"document":"plan","text":"new draft"});
        let raw=serde_json::to_vec(&request).unwrap();
        assert_eq!(root.preparation_show(&current.id).unwrap().value,current);
        let count=|table:&str|db.query_row(&format!("SELECT count(*) FROM {table}"),[],|row|row.get::<_,i64>(0)).unwrap();
        let before=(count("preparation_documents"),count("preparation_commands"));
        db.execute_batch(&format!("CREATE TRIGGER preparation_test_refusal BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'PRIVATE_SQL_FAULT'); END")).unwrap();
        let error=root.preparation_command(&raw,&mut SystemEnvironment).err().expect("injected insert must fail");
        assert_eq!(error.code(),"agent-launch-storage");
        assert!(!error.to_string().contains("PRIVATE_SQL_FAULT"));
        assert!(error.recorded_preparation().is_none(),"known rollback is not a committed receipt");
        assert_eq!(root.preparation_show(&current.id).unwrap().value,current);
        assert_eq!((count("preparation_documents"),count("preparation_commands")),before);
        db.execute_batch("DROP TRIGGER preparation_test_refusal").unwrap();
        let next=root.preparation_command(&raw,&mut SystemEnvironment).unwrap().value;
        assert_eq!(next.revision,current.revision+1);
        assert_eq!(next.documents.as_ref().unwrap()["plan"].text,"new draft");
        assert!(!next.plan_ready);
        assert_eq!((count("preparation_documents"),count("preparation_commands")),(before.0+1,before.1+1));
        assert_eq!(root.preparation_command(&raw,&mut SystemEnvironment).unwrap().value,next);
        current=next;
    }
    db.close().unwrap();root.close().unwrap();
    let reopened=RootStore::open(&path,false).unwrap();assert_eq!(reopened.preparation_show(&current.id).unwrap().value,current);reopened.close().unwrap();
    std::fs::remove_dir_all(path).unwrap();
}

/// A valid replay precedes receipt-only corruption at the current revision.
/// Reject altered content without another row, then recover the same event after
/// repairing only the fixture. Historical valid receipts remain replayable. This
/// is integrity detection, not protection against arbitrary database writers.
#[test]
fn preparation_current_receipt_content_must_match_stored_state() {
    let path=std::env::temp_dir().join(format!("swarm-preparation-receipt-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    let root=RootStore::initialize(&path,&mut SystemEnvironment).unwrap();
    let request=br#"{"version":1,"action":"create","event_id":"receipt-integrity","expected_revision":0,"text":"original"}"#;
    let initial=root.preparation_command(request,&mut SystemEnvironment).unwrap().value;
    assert_eq!(root.preparation_command(request,&mut SystemEnvironment).unwrap().value,initial);
    let db=rusqlite::Connection::open(path.join(".swarm/state.db")).unwrap();
    let original:Vec<u8>=db.query_row("SELECT CAST(response AS BLOB) FROM preparation_commands WHERE preparation_id=?1 AND event_id='receipt-integrity'",[&initial.id],|row|row.get(0)).unwrap();
    let count: i64=db.query_row("SELECT count(*) FROM preparation_commands",[],|row|row.get(0)).unwrap();
    for field in ["title","receipt_historical"] {
        let mut altered:serde_json::Value=serde_json::from_slice(&original).unwrap();
        altered[field]=if field=="title" {serde_json::json!("altered receipt")}else{serde_json::json!(true)};
        db.execute("UPDATE preparation_commands SET response=?1 WHERE preparation_id=?2 AND event_id='receipt-integrity'",rusqlite::params![serde_json::to_vec(&altered).unwrap(),initial.id]).unwrap();
        let result=root.preparation_command(request,&mut SystemEnvironment);
        assert!(result.is_err(),"altered current receipt accepted; fixture={}",path.display());
        assert_eq!(result.err().unwrap().code(),"preparation-receipt-identity-mismatch");
        assert_eq!(root.preparation_show(&initial.id).unwrap().value,initial);
        assert_eq!(db.query_row("SELECT count(*) FROM preparation_commands",[],|row|row.get::<_,i64>(0)).unwrap(),count);
        db.execute("UPDATE preparation_commands SET response=?1 WHERE preparation_id=?2 AND event_id='receipt-integrity'",rusqlite::params![original,initial.id]).unwrap();
        assert_eq!(root.preparation_command(request,&mut SystemEnvironment).unwrap().value,initial);
    }
    let save=serde_json::to_vec(&serde_json::json!({"version":1,"action":"save","preparation_id":initial.id,"event_id":"later","expected_revision":1,"document":"besoin","text":"later"})).unwrap();
    root.preparation_command(&save,&mut SystemEnvironment).unwrap();
    let historical=root.preparation_command(request,&mut SystemEnvironment).unwrap().value;
    assert!(historical.receipt_historical);assert_eq!(historical.revision,1);
    db.close().unwrap();root.close().unwrap();std::fs::remove_dir_all(path).unwrap();
}

/// Adopt-brief with an injected receipt-INSERT fault: the whole transaction
/// (revision bump, adopted copy, verdict clearing) must roll back, then dropping
/// only the owned fixture trigger lets the identical event recover. Verdict is
/// cleared, the brief is copied verbatim, document history is untouched and the
/// committed event replays identically. Fixture SQL touches only this isolated
/// test database, after a valid public adoption control; no provider, execution
/// or plan approval is established.
#[test]
fn preparation_adopt_brief_receipt_failure_rolls_back_and_same_event_recovers() {
    let path=std::env::temp_dir().join(format!("swarm-preparation-adopt-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    let root=RootStore::initialize(&path,&mut SystemEnvironment).unwrap();
    let created=root.preparation_command(br#"{"version":1,"action":"create","event_id":"adopt-create","expected_revision":0,"text":"need text"}"#,&mut SystemEnvironment).unwrap().value;
    assert_eq!(created.revision,1);
    let need_sha=created.documents.as_ref().unwrap()["besoin"].sha256.clone();
    let raw_brief=serde_json::to_vec(&serde_json::json!({"version":1,"action":"save","preparation_id":created.id,"event_id":"adopt-brief-save","expected_revision":1,"document":"brief","text":"brief text"})).unwrap();
    let with_brief=root.preparation_command(&raw_brief,&mut SystemEnvironment).unwrap().value;
    assert_eq!(with_brief.revision,2);
    let brief_doc=with_brief.documents.as_ref().unwrap()["brief"].clone();
    assert_eq!(brief_doc.need_hash,need_sha);
    assert!(with_brief.adopted_brief.is_none());
    let control=serde_json::to_vec(&serde_json::json!({"version":1,"action":"adopt-brief","preparation_id":created.id,"event_id":"adopt-valid-control","expected_revision":2,"sha256":brief_doc.sha256})).unwrap();
    let witnessed=root.preparation_command(&control,&mut SystemEnvironment).unwrap().value;
    assert_eq!(witnessed.adopted_brief.as_deref(),Some(&brief_doc));
    assert_eq!(witnessed.revision,3);
    let fresh=serde_json::to_vec(&serde_json::json!({"version":1,"action":"save","preparation_id":created.id,"event_id":"adopt-new-brief","expected_revision":3,"document":"brief","text":"revised brief"})).unwrap();
    let revised=root.preparation_command(&fresh,&mut SystemEnvironment).unwrap().value;
    assert_eq!(revised.revision,4);
    let brief_doc=revised.documents.as_ref().unwrap()["brief"].clone();
    let db=rusqlite::Connection::open(path.join(".swarm/state.db")).unwrap();
    // Seed a stale verdict so that clearing it by adoption is observable;
    // revision stays 4 so the writer CAS below is unaffected.
    let body:Vec<u8>=db.query_row("SELECT CAST(body AS BLOB) FROM preparations WHERE id=?1",[&created.id],|row|row.get(0)).unwrap();
    let mut seeded:serde_json::Value=serde_json::from_slice(&body).unwrap();
    seeded["verdict"]=serde_json::json!({"plan_hash":"stale-plan","brief_hash":"stale-brief","method_hash":"stale-method","at":"2020-01-01T00:00:00Z"});
    assert_eq!(db.execute("UPDATE preparations SET body=?1 WHERE id=?2",rusqlite::params![serde_json::to_vec(&seeded).unwrap(),created.id]).unwrap(),1);
    let current=root.preparation_show(&created.id).unwrap().value;
    assert!(current.verdict.is_some());assert_eq!(current.revision,4);
    let count=|table:&str|db.query_row(&format!("SELECT count(*) FROM {table}"),[],|row|row.get::<_,i64>(0)).unwrap();
    let before=(count("preparation_documents"),count("preparation_commands"));
    let request=serde_json::to_vec(&serde_json::json!({"version":1,"action":"adopt-brief","preparation_id":created.id,"event_id":"adopt-fault","expected_revision":4,"sha256":brief_doc.sha256})).unwrap();
    db.execute_batch("CREATE TRIGGER preparation_adopt_fault BEFORE INSERT ON preparation_commands BEGIN SELECT RAISE(ABORT,'PRIVATE_ADOPT_FAULT'); END").unwrap();
    let error=root.preparation_command(&request,&mut SystemEnvironment).err().expect("receipt fault must refuse");
    assert_eq!(error.code(),"agent-launch-storage");
    assert!(!error.to_string().contains("PRIVATE_ADOPT_FAULT"));
    assert!(error.recorded_preparation().is_none(),"known rollback is not a committed receipt");
    assert_eq!(root.preparation_show(&created.id).unwrap().value,current);
    assert_eq!((count("preparation_documents"),count("preparation_commands")),before);
    db.execute_batch("DROP TRIGGER preparation_adopt_fault").unwrap();
    let recovered=root.preparation_command(&request,&mut SystemEnvironment).unwrap().value;
    assert_eq!(recovered.revision,5);
    assert!(recovered.verdict.is_none());
    assert_eq!(recovered.adopted_brief.as_deref(),Some(&brief_doc));
    assert_eq!(recovered.documents,current.documents);
    assert!(!recovered.plan_ready);
    assert_eq!((count("preparation_documents"),count("preparation_commands")),(before.0,before.1+1));
    assert_eq!(root.preparation_command(&request,&mut SystemEnvironment).unwrap().value,recovered);
    db.close().unwrap();root.close().unwrap();
    let reopened=RootStore::open(&path,false).unwrap();
    assert_eq!(reopened.preparation_show(&created.id).unwrap().value,recovered);
    reopened.close().unwrap();
    std::fs::remove_dir_all(path).unwrap();
}
