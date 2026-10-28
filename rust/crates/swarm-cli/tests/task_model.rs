#![cfg(target_os = "linux")]
use std::io::{self, Write};
use swarm_engine::{RootStore, SystemEnvironment};
/// Fresh isolated initialized Work/task; no provider is needed to clear choice.
fn fixture() -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    let mut s = RootStore::initialize(root.path(), &mut SystemEnvironment).unwrap();
    let work=s.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Title","objective":"Goal","scope":"Fixture","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
    s.execute_external(&work,"task.add",br#"{"schema_version":1,"event_id":"add","expected_revision":1,"id":"task","title":"Task","deliverable":"Report","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap();
    s.close().unwrap();
    (root, work)
}
/// Complete inherited request used only against the owned fixture.
fn request(event: &str, revision: i64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":revision,"task_id":"task","inherit":true})).unwrap()
}
/// Output-only fault preserves private text in typed causes, never public stderr.
struct Denied(bool);
impl Write for Denied {
    /// A write refusal or accepted buffer distinguishes the two output stages.
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.0 {
            Ok(b.len())
        } else {
            Err(io::Error::other("PRIVATE sink"))
        }
    }
    /// Return the explicit flush failure only for the corresponding stage.
    fn flush(&mut self) -> io::Result<()> {
        if self.0 {
            Err(io::Error::other("PRIVATE sink"))
        } else {
            Ok(())
        }
    }
}
/// Real versioned apply commits before response write/flush. Readback/replay
/// preserve the event/revision, never double-apply or imply a provider run.
#[test]
fn task_model_output_failures_preserve_committed_choice_and_replay() {
    for flush in [false, true] {
        let (root, work) = fixture();
        let input = root.path().join("choice.json");
        std::fs::write(&input, request("clear", 2)).unwrap();
        // Valid read-only preview is the witness before output fault injection.
        let s = RootStore::open(root.path(), false).unwrap();
        assert_eq!(
            s.preview_task_model(&work, &request("clear", 2), &mut SystemEnvironment)
                .unwrap()
                .revision,
            2
        );
        s.close().unwrap();
        let args = vec![
            "task-model".into(),
            "apply".into(),
            work.clone(),
            "--root".into(),
            root.path().to_str().unwrap().into(),
            "--input".into(),
            input.to_str().unwrap().into(),
            "--json".into(),
        ];
        let mut stderr = vec![];
        let outcome = swarm_cli::run_detailed(&args, "en", &mut Denied(flush), &mut stderr);
        assert_eq!(outcome.code, 2);
        assert!(matches!(
            outcome.error,
            Some(swarm_cli::CliError::PlanningStatePreserved { .. })
        ));
        assert!(!String::from_utf8_lossy(&stderr).contains("PRIVATE"));
        let mut s = RootStore::open(root.path(), true).unwrap();
        assert_eq!(s.read_work(&work).unwrap().revision, 3);
        let events = s.events(&work).unwrap();
        let replay = s
            .execute_external(
                &work,
                "task.model",
                &request("clear", 2),
                &mut SystemEnvironment,
            )
            .unwrap();
        assert!(replay.result.replayed);
        assert_eq!(replay.result.work.revision, 3);
        assert_eq!(s.events(&work).unwrap().len(), events.len());
        s.close().unwrap();
    }
}
/// A foreign persistent trigger is refused by Root schema validation. Read-only
/// observation preserves Work/events; dropping only our fixture trigger permits
/// the same request. Transaction fault injection is separately tested in WorkEngine.
#[test]
fn task_model_schema_refusal_preserves_work_then_same_request_recovers() {
    let (root, work) = fixture();
    let s = RootStore::open(root.path(), false).unwrap();
    let before = swarm_engine::encode_model(&s.read_work(&work).unwrap()).unwrap();
    let events = s.events(&work).unwrap().len();
    s.close().unwrap();
    let path = root.path().join(".swarm/state.db");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER task_model_fixture_refusal BEFORE INSERT ON events WHEN NEW.kind='task.model' BEGIN SELECT RAISE(ABORT,'PRIVATE event fault'); END;").unwrap();
    let e = RootStore::open(root.path(), true).err().unwrap();
    assert_eq!(e.code(), "root-schema-mismatch");
    assert!(!e.to_string().contains("PRIVATE"));
    assert_eq!(
        swarm_engine::encode_model(&swarm_engine::read_recorded_work(&db, &work).unwrap()).unwrap(),
        before
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM events WHERE work_id=?",
            [&work],
            |r| r.get::<_, i64>(0)
        )
        .unwrap() as usize,
        events
    );
    db.execute_batch("DROP TRIGGER task_model_fixture_refusal;")
        .unwrap();
    db.close().unwrap();
    let mut s = RootStore::open(root.path(), true).unwrap();
    assert_eq!(
        s.execute_external(
            &work,
            "task.model",
            &request("recover", 2),
            &mut SystemEnvironment
        )
        .unwrap()
        .result
        .work
        .revision,
        3
    );
    assert_eq!(s.events(&work).unwrap().len(), events + 1);
    s.close().unwrap();
}
