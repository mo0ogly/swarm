use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment};
/// An owned fresh root and ordinary Work, created solely through public APIs.
/// Failed fixtures remain for diagnosis; no provider, agent or live mission runs.
struct Fixture {
    root: std::path::PathBuf,
    id: String,
}
impl Fixture {
    fn new(requirements: usize) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("planning-enable62-").unwrap());
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut store = RootStore::initialize(&root, &mut SystemEnvironment).unwrap();
        let raw=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Enable fixture","objective":"Observable planning","scope":"Private","criteria":(0..requirements).map(|i|format!("Requirement {i}")).collect::<Vec<_>>()})).unwrap();
        let work = store
            .execute_external("", "work.create", &raw, &mut SystemEnvironment)
            .unwrap()
            .result
            .work;
        store.close().unwrap();
        Self { root, id: work.id }
    }
    /// Read original DB bytes after every public call has released its connection.
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.root.join(".swarm/state.db")).unwrap()
    }
    /// Execute one real Root transaction, closing it before observing effects.
    fn apply(
        &self,
        kind: &str,
        value: &serde_json::Value,
    ) -> Result<swarm_engine::RootMutationOutcome, swarm_engine::RootStoreError> {
        let mut store = RootStore::open(&self.root, true).unwrap();
        let result = store.execute_external(
            &self.id,
            kind,
            &serde_json::to_vec(value).unwrap(),
            &mut SystemEnvironment,
        );
        store.close().unwrap();
        result
    }
    /// Save and reopen both v12 policy choices via the real preview/CAS API.
    fn policy(&self, bytes: Option<i64>, requirements: Option<i64>) {
        let store = RootStore::open(&self.root, true).unwrap();
        let doc = store.settings().unwrap();
        let old = doc.history.clone();
        let mut values = doc.values;
        values.planning_request_max_bytes = Some(SettingCap(bytes.map(SettingCount)));
        values.planning_max_requirements = Some(SettingCap(requirements.map(SettingCount)));
        let mut change = EngineSettingsChange {
            schema_version: values.schema_version(),
            event_id: swarm_engine::generate_identifier("policy-").unwrap(),
            expected_revision: doc.revision,
            values,
            reason: "Observe effective planning policy".into(),
            preview_token: String::new(),
        };
        change.preview_token = store.preview_settings(&change).unwrap().preview_token;
        store
            .apply_settings(&change, &mut SystemEnvironment)
            .unwrap();
        store.close().unwrap();
        let reopened = RootStore::open(&self.root, false).unwrap();
        let doc = reopened.settings().unwrap();
        assert_eq!(&doc.history[..old.len()], old.as_slice());
        assert_eq!(
            doc.values.planning_request_limit().unwrap(),
            bytes.map(|v| v as usize)
        );
        reopened.close().unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
fn enable() -> serde_json::Value {
    serde_json::json!({"schema_version":1,"event_id":"enable","expected_revision":1,"max_tasks":10,"max_decisions":12,"max_activations":15,"max_judge_cycles":0})
}
/// Root publishes exactly one hierarchy/event with normalized operator checks;
/// stored replay bypasses changed policy, while conflicting intent stays refused.
#[test]
fn native_enable_commits_complete_hierarchy_and_replays_after_policy_change() {
    let f = Fixture::new(2);
    let mut request = enable();
    request["checks"] = serde_json::json!({"req-2":[{"id":"check","command":["cargo","check"],"criteria":[99],"justification":"Operator authorized local check"}]});
    let result = f.apply("planning.enable", &request).unwrap();
    let p = result.result.work.planning.as_ref().unwrap();
    assert_eq!(p.version, 1);
    assert_eq!(p.max_decisions, 12);
    assert_eq!(p.activations, 0);
    assert_eq!(
        p.scopes.as_ref().unwrap()[0].requirements,
        Some(vec!["req-1".into(), "req-2".into()])
    );
    assert_eq!(p.inbox.as_ref().unwrap()[0].kind, "brief");
    assert_eq!(p.judge.as_ref().unwrap().cycle, 1);
    assert!(!p.reviewer_required);
    assert!(p.repository.is_none());
    let c = &p.checks.as_ref().unwrap()["req-2"].as_ref().unwrap()[0];
    assert_eq!(c.criteria, Some(vec![1]));
    assert_eq!(c.timeout_seconds, 60);
    f.policy(Some(0), Some(0));
    let before = f.bytes();
    let replay = f.apply("planning.enable", &request).unwrap();
    assert!(replay.result.replayed);
    assert_eq!(replay.result.work.revision, 2);
    assert_eq!(f.bytes(), before);
    request["reason"] = serde_json::json!("different intent");
    assert!(f.apply("planning.enable", &request).is_err());
    assert_eq!(f.bytes(), before);
}
/// Valid witnesses precede invalid budgets, existing tasks and unported provider/
/// repository paths; refusals preserve DB bytes and the same request can recover.
#[test]
fn native_enable_refuses_invalid_and_unported_inputs_without_mutation() {
    for (field, value, code) in [
        (
            "max_tasks",
            serde_json::json!(0),
            "planning-control-invalid",
        ),
        (
            "max_decisions",
            serde_json::json!(0),
            "planning-control-invalid",
        ),
        (
            "max_activations",
            serde_json::json!(0),
            "planning-control-invalid",
        ),
        (
            "max_judge_cycles",
            serde_json::json!(-1),
            "planning-control-invalid",
        ),
        (
            "provider",
            serde_json::json!("unported"),
            "command_not_ported",
        ),
        (
            "repository",
            serde_json::json!({"path":"private","committed_only":false}),
            "command_not_ported",
        ),
        (
            "checks",
            serde_json::json!({"req-9":[]}),
            "planning-control-invalid",
        ),
        (
            "checks",
            serde_json::json!({"req-1":[{"id":"c","command":["unknown"],"justification":"Operator choice"}]}),
            "format",
        ),
    ] {
        let witness = Fixture::new(1);
        witness.apply("planning.enable", &enable()).unwrap();
        let f = Fixture::new(1);
        let mut bad = enable();
        bad[field] = value;
        let before = f.bytes();
        let error = f.apply("planning.enable", &bad).err().expect("must refuse");
        assert_eq!(error.code(), code);
        assert_eq!(f.bytes(), before);
        f.apply("planning.enable", &enable()).unwrap();
    }
}
/// Canonical bytes count escaping rather than input whitespace; zero/exact/null,
/// historical adoption and recovery are observable at the actual Root boundary.
#[test]
fn native_planning_canonical_cap_is_saved_exact_and_recoverable() {
    let f = Fixture::new(2);
    f.policy(None, Some(1));
    let before = f.bytes();
    assert!(f.apply("planning.enable", &enable()).is_err());
    assert_eq!(f.bytes(), before);
    f.policy(None, Some(2));
    f.apply("planning.enable", &enable()).unwrap();
    let request = serde_json::json!({"schema_version":1,"event_id":"pause","expected_revision":2,"reason":"<é>"});
    let typed =
        swarm_engine::decode_planning_request(&serde_json::to_vec(&request).unwrap()).unwrap();
    let canonical = swarm_engine::encode_planning_request(&typed).unwrap();
    assert!(canonical.windows(6).any(|v| v == b"\\u003c"));
    f.policy(Some(canonical.len() as i64 - 1), None);
    let before = f.bytes();
    let error = f.apply("planning.pause", &request).err().unwrap();
    assert_eq!(error.code(), "planning-control-invalid");
    assert_eq!(f.bytes(), before);
    f.policy(Some(canonical.len() as i64), None);
    let paused = f.apply("planning.pause", &request).unwrap();
    assert_eq!(paused.result.work.revision, 3);
    f.policy(Some(0), None);
    let before = f.bytes();
    assert!(f.apply("planning.pause", &request).unwrap().result.replayed);
    assert_eq!(f.bytes(), before);
    let next = serde_json::json!({"schema_version":1,"event_id":"second","expected_revision":3,"reason":"x".repeat(65537)});
    assert!(f.apply("planning.pause", &next).is_err());
    assert_eq!(f.bytes(), before);
    f.policy(None, None);
    assert_eq!(
        f.apply("planning.pause", &next)
            .unwrap()
            .result
            .work
            .revision,
        4
    );
}
/// Historical v11 is readable, but new transactions require visible v12 adoption.
/// No failed event is consumed; the same intended enable recovers after save.
#[test]
fn native_enable_requires_explicit_legacy_request_policy_adoption() {
    let f = Fixture::new(1);
    std::fs::write(
        f.root.join(".swarm/engine-settings.json"),
        include_bytes!("fixtures/legacy-v11-document.json"),
    )
    .unwrap();
    let before = f.bytes();
    let error = f.apply("planning.enable", &enable()).err().unwrap();
    assert_eq!(error.code(), "engine-settings");
    assert!(error
        .to_string()
        .contains("settings-planning-request-policy-not-configured"));
    assert_eq!(f.bytes(), before);
    f.policy(Some(65536), Some(100));
    assert_eq!(
        f.apply("planning.enable", &enable())
            .unwrap()
            .result
            .work
            .revision,
        2
    );
}
/// A fresh Work with a real manual task cannot be silently adopted as a new
/// hierarchy. A second enable with a new event also refuses without a DB change.
#[test]
fn native_enable_refuses_existing_tasks_and_second_enable() {
    let f = Fixture::new(1);
    f.apply("task.add",&serde_json::json!({"schema_version":1,"event_id":"task","expected_revision":1,"id":"task","title":"Task","deliverable":"report","criteria":["Proof"]})).unwrap();
    let mut request = enable();
    request["expected_revision"] = serde_json::json!(2);
    let before = f.bytes();
    assert!(f.apply("planning.enable", &request).is_err());
    assert_eq!(f.bytes(), before);
    let fresh = Fixture::new(1);
    fresh.apply("planning.enable", &enable()).unwrap();
    request["event_id"] = serde_json::json!("second-enable");
    let before = fresh.bytes();
    assert!(fresh.apply("planning.enable", &request).is_err());
    assert_eq!(fresh.bytes(), before);
}
