//! Opt-in rollback SDK on an explicitly owned, genuine retained native fixture.
use super::*;
/// Inject TEMP failures only after a valid current native witness. The caller
/// supplies an isolated fixture and strict envelope; never a mission database.
/// Compare actual Work, event count and complete normalized worker rows after
/// each refusal; restore trigger and recheck the public preview before return.
#[test]
#[ignore = "requires explicit retained inert native fixture and envelope"]
fn first_publication_sql_rollback_preserves_all_state() {
    let path = std::path::PathBuf::from(std::env::var_os("SWARM_FIRST_PUBLICATION_FIXTURE").expect("owned fixture root"));
    assert!(path.is_absolute() && path.file_name().unwrap().as_encoded_bytes().starts_with(b"swarm-model83-"));
    assert!(std::fs::symlink_metadata(path.join(".rust-migration-fixture")).unwrap().file_type().is_file());
    let input = std::fs::read(std::env::var_os("SWARM_FIRST_PUBLICATION_REQUEST").expect("strict envelope")).unwrap();
    let work_id = std::env::var("SWARM_FIRST_PUBLICATION_WORK").expect("owned work");
    let root = RootStore::open(&path,true).unwrap();
    let before = root.read_work(&work_id).unwrap();
    let rows = || crate::agent_launch::query(&root.db,
        "SELECT agent_id,work_id,task_id,base_commit,path,state,result_commit,detail FROM managed_attempts ORDER BY agent_id",[],|rows| {
        let mut all=Vec::new();
        while let Some(row)=rows.next()? { all.push((0..8).map(|i|row.get::<_,String>(i)).collect::<Result<Vec<_>,_>>()?); }
        Ok(all)
    }).unwrap();
    let original_rows = rows();
    let events = || root.db.query_row("SELECT count(*) FROM events",[],|r|r.get::<_,i64>(0)).unwrap();
    let original_events=events();
    assert!(root.preview_managed_publication(&work_id,&input).unwrap().current_candidate_verified);
    for (name,sql) in [
        ("row-abort","CREATE TEMP TRIGGER publication_fault BEFORE UPDATE ON main.managed_attempts BEGIN SELECT RAISE(ABORT,'owned row abort'); END"),
        ("event-abort","CREATE TEMP TRIGGER publication_fault BEFORE INSERT ON main.events WHEN NEW.kind='managed.publication' BEGIN SELECT RAISE(ABORT,'owned event abort'); END"),
        ("silent-row","CREATE TEMP TRIGGER publication_fault AFTER UPDATE ON main.managed_attempts WHEN NEW.state='integrated' BEGIN UPDATE managed_attempts SET state='ready',result_commit='',detail='' WHERE agent_id=NEW.agent_id; END"),
        ("transaction-rollback","CREATE TEMP TRIGGER publication_fault BEFORE UPDATE ON main.managed_attempts BEGIN SELECT RAISE(ROLLBACK,'owned transaction rollback'); END"),
    ] {
        root.db.execute_batch(sql).unwrap();
        let error = root.publish_managed_candidate(&work_id,&input).err().expect(name);
        assert!(!error.to_string().is_empty());
        root.db.execute_batch("DROP TRIGGER temp.publication_fault").unwrap();
        assert!(root.db.is_autocommit(),"{name}");
        assert_eq!(root.read_work(&work_id).unwrap(),before,"{name}");
        assert_eq!(rows(),original_rows,"{name}");assert_eq!(events(),original_events,"{name}");
        assert!(root.preview_managed_publication(&work_id,&input).unwrap().current_candidate_verified,"{name}");
        println!("MANAGED_PUBLICATION_ROLLBACK_VERIFIED {name}");
    }
    root.close().unwrap();
}
