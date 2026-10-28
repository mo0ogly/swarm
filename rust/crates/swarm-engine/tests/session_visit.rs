#![cfg(target_os = "linux")]
//! Actual Root inner-write regressions on owned inactive Roots. Triggers are
//! injected only after an ordinary valid Root open; public HTTP reopen rejects
//! altered schema first and is tested separately. No admission check is disabled.
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use swarm_engine::{
    GitObservation, RootStore, RootStoreError, RuntimeError, SystemEnvironment, VisitMarkInput,
    WorkEnvironment,
};
struct Fixture(PathBuf);
impl Fixture {
    /// Fresh private inactive Root, preserving failed fixtures for diagnosis.
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("visit-inner-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("visit fixture retained: {}", self.0.display())
        } else {
            fs::remove_dir_all(&self.0).unwrap()
        }
    }
}
/// Deterministic operator namespaces and valid clock are injected fixture inputs,
/// never identities supplied by HTTP/CLI or evidence of a human actor.
struct Environment {
    operator: &'static str,
}
impl WorkEnvironment for Environment {
    fn identifier(&mut self, p: &str) -> Result<String, RuntimeError> {
        SystemEnvironment.identifier(p)
    }
    fn timestamp(&mut self) -> Result<String, RuntimeError> {
        Ok("2026-10-27T12:00:00Z".into())
    }
    fn operator(&mut self) -> Result<String, RuntimeError> {
        Ok(self.operator.into())
    }
    fn git(&mut self, p: &Path) -> Result<GitObservation, RuntimeError> {
        SystemEnvironment.git(p)
    }
}
/// Create a real Work using the public event API; no acceptance/provider/SQL seed.
fn work(store: &mut RootStore) -> swarm_engine::Work {
    store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Visit inner test","objective":"Verify bookmark effects","scope":"Owned inactive fixture","criteria":["Exact saved or refused state"]}"#,&mut SystemEnvironment).unwrap().result.work
}
#[test]
fn existing_connection_noop_and_changed_readback_refuse_then_recover() {
    let f = Fixture::new();
    let mut root = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let w = work(&mut root);
    let before = serde_json::to_value(root.read_work(&w.id).unwrap()).unwrap();
    let intent = VisitMarkInput::historical();
    let saved = root
        .mark_session_visit(&w.id, &intent, &mut SystemEnvironment)
        .unwrap();
    assert!(saved.saved);
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    for (ddl,code) in [
        ("CREATE TRIGGER visit_test_fault BEFORE INSERT ON session_visits BEGIN SELECT RAISE(IGNORE); END","visit-write-not-recorded"),
        ("CREATE TRIGGER visit_test_fault AFTER UPDATE ON session_visits BEGIN UPDATE session_visits SET at='changed' WHERE work_id=NEW.work_id AND operator=NEW.operator; END","visit-write-readback-mismatch")
    ] {
        let previous=serde_json::to_value(root.session_visit(&w.id,&mut SystemEnvironment).unwrap()).unwrap();
        db.execute_batch(ddl).unwrap();
        let error=root.mark_session_visit(&w.id,&intent,&mut SystemEnvironment).err().expect("fixture must refuse");
        assert_eq!(error.code(),code);assert!(swarm_engine::auxiliary_effects(&error).is_empty());assert_eq!(serde_json::to_value(root.session_visit(&w.id,&mut SystemEnvironment).unwrap()).unwrap(),previous);
        db.execute_batch("DROP TRIGGER visit_test_fault").unwrap();
        assert!(root.mark_session_visit(&w.id,&intent,&mut SystemEnvironment).unwrap().saved);
        // Each fault has its own valid recovered control before the next one.
    }
    assert_eq!(
        serde_json::to_value(root.read_work(&w.id).unwrap()).unwrap(),
        before
    );
    db.execute(
        "UPDATE session_visits SET revision=-1 WHERE work_id=?1",
        [&w.id],
    )
    .unwrap();
    let error = root
        .session_visit(&w.id, &mut SystemEnvironment)
        .err()
        .expect("negative stored revision must refuse");
    assert_eq!(error.code(), "visit-recorded-revision-invalid");
    assert!(swarm_engine::auxiliary_effects(&error).is_empty());
    root.mark_session_visit(&w.id, &intent, &mut SystemEnvironment)
        .unwrap();
    assert_eq!(
        root.session_visit(&w.id, &mut SystemEnvironment)
            .unwrap()
            .visit
            .revision,
        w.revision
    );
    db.close().unwrap();
    root.close().unwrap();
}
#[test]
fn operators_are_independent_and_missing_bookmark_is_explicit() {
    let f = Fixture::new();
    let mut root = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let w = work(&mut root);
    let mut a = Environment {
        operator: "fixture-a",
    };
    let mut b = Environment {
        operator: "fixture-b",
    };
    assert!(!root.session_visit(&w.id, &mut a).unwrap().exists);
    root.mark_session_visit(&w.id, &VisitMarkInput::historical(), &mut a)
        .unwrap();
    assert!(root.session_visit(&w.id, &mut a).unwrap().exists);
    let absent = root.session_visit(&w.id, &mut b).unwrap();
    assert!(!absent.exists);
    assert_eq!(absent.exact_visit_revision, "0");
    assert!(absent.visit.at.is_empty());
    assert_eq!(absent.visit.operator, "fixture-b");
    let bad = swarm_engine::decode_visit_mark(
        format!(
            r#"{{"schema_version":1,"expected_revision":"{}"}}"#,
            w.revision + 1
        )
        .as_bytes(),
    )
    .unwrap();
    assert!(matches!(
        root.mark_session_visit(&w.id, &bad, &mut b),
        Err(RootStoreError::Refused("visit-revision-conflict"))
    ));
    assert!(!root.session_visit(&w.id, &mut b).unwrap().exists);
    root.close().unwrap();
    let reopened = RootStore::open(&f.0, false).unwrap();
    assert!(reopened.session_visit(&w.id, &mut a).unwrap().exists);
    assert!(!reopened.session_visit(&w.id, &mut b).unwrap().exists);
    reopened.close().unwrap();
}
