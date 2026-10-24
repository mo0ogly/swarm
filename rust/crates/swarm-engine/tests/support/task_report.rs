//! Deterministic storage fixture, not proof of a real provider execution.
use std::{fs, path::PathBuf};
use swarm_engine::{RootStore, SystemEnvironment};
/// One private storage double; keeps its directory after a panicking assertion.
pub struct Fixture {
    pub root: PathBuf,
    pub work: String,
}
impl Fixture {
    /// Initialize a private native schema/public Work, then seed completed records
    /// as an explicit storage double. Public Python recipe covers the real child.
    /// No live database, provider account, PID or authenticated actor is used.
    pub fn new() -> Self {
        let root = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("report-fixture-").unwrap());
        fs::create_dir(&root).unwrap();
        let mut s = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let work=s.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Report","objective":"Storage contract","scope":"Fixture","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
        s.execute_external(&work,"task.add",br#"{"schema_version":1,"event_id":"add","expected_revision":1,"id":"task","title":"Report","deliverable":"Report","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap();
        let settings = s.settings().unwrap();
        s.close().unwrap();
        let db = rusqlite::Connection::open(root.join(".swarm/state.db")).unwrap();
        swarm_engine::register_identity_plan_projection(&db).unwrap();
        let mut w = swarm_engine::read_recorded_work(&db, &work).unwrap();
        let task = &mut w.tasks.as_mut().unwrap()[0];
        task.status = "blocked".into();
        task.attempts = Some(vec![swarm_engine::Attempt {
            id: "attempt".into(),
            status: "completed".into(),
            started: "2026-10-19T00:00:00Z".into(),
            ended: "2026-10-19T00:00:01Z".into(),
        }]);
        db.execute(
            "UPDATE works SET body=? WHERE id=?",
            rusqlite::params![swarm_engine::encode_model(&w).unwrap(), work],
        )
        .unwrap();
        let prompt = "Controlled storage fixture";
        let hash = sha(prompt.as_bytes());
        let agent=swarm_engine::decode_agent(&serde_json::to_vec(&serde_json::json!({"id":"session","work_id":work,"task_id":"task","attempt_id":"attempt","workspace":root,"status":"completed","role":"worker","exit_code":0,"ended":"2026-10-19T00:00:01Z","prompt":prompt,"native_execution":{"schema_version":1,"settings_revision":settings.revision,"work_revision":2,"context_sha256":hash,"phase":"permission_recorded","policy":settings.values.agent_execution_policy().unwrap()},"native_execution_result":{"fixture_only":true}})).unwrap()).unwrap();
        db.execute("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES('session',?,'task',?,'completed',?,?)",rusqlite::params![work,root.to_str().unwrap(),swarm_engine::encode_model(&agent).unwrap(),b"{}".as_slice()]).unwrap();
        db.execute(
            "INSERT INTO reservations VALUES('session',?,1.0,'estimated')",
            [&work],
        )
        .unwrap();
        db.close().unwrap();
        fs::write(root.join("handoff.md"), b"Controlled report\n").unwrap();
        Self { root, work }
    }
    /// Canonical fresh intent; no caller actor or synthetic execution claim.
    pub fn request(&self, event: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":2,"id":"task","agent_id":"session","report_path":"handoff.md","report_sha256":sha(b"Controlled report\n")})).unwrap()
    }
}
/// Hash actual fixture bytes, independent of any identity/quality assertion.
fn sha(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(b))
}
impl Drop for Fixture {
    /// Retain failed fixtures for diagnosis; remove only this owned root on success.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
