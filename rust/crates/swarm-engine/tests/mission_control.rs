#![cfg(target_os = "linux")]
//! Global departure controls use real isolated Root transactions, never consent grants.
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{
    EngineSettingsChange, MissionSupervisionError, RootStore, RootStoreError, SettingCap,
    SettingCount, SystemEnvironment, WorkEngineError,
};

/// Owned test Root. Successful fixtures alone are removed; panics keep evidence.
struct Fixture {
    path: PathBuf,
    store: Option<RootStore>,
    work: String,
}
impl Fixture {
    /// Initialize and create through supported Root methods without SQL seeds,
    /// provider metadata, external calls or a running mission database.
    fn new() -> Self {
        let path = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("mission-control180-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut store = RootStore::initialize(&path, &mut SystemEnvironment).unwrap();
        let raw = serde_json::json!({"schema_version":1,"event_id":"create","expected_revision":0,"title":"Global pause","objective":"Preserve ownership","scope":"Owned fixture","criteria":["Pause is not launch consent"]});
        let work = store
            .execute_external(
                "",
                "work.create",
                &serde_json::to_vec(&raw).unwrap(),
                &mut SystemEnvironment,
            )
            .unwrap()
            .result
            .work
            .id;
        Self {
            path,
            store: Some(store),
            work,
        }
    }
    /// Supported policy preview/apply/reopen; actual slots are not DDL defaults.
    fn policy(&self, event: &str, initial: i64, max: i64) {
        let store = self.store.as_ref().unwrap();
        let doc = store.settings().unwrap();
        let mut values = doc.values;
        values.mission_dispatch_default_slots = Some(SettingCount(initial));
        values.mission_dispatch_max_slots = Some(SettingCap(Some(SettingCount(max))));
        let mut r = EngineSettingsChange {
            schema_version: doc.schema_version,
            event_id: event.into(),
            expected_revision: doc.revision,
            values,
            reason: "Explicit test admission policy".into(),
            preview_token: String::new(),
        };
        r.preview_token = store.preview_settings(&r).unwrap().preview_token;
        store.apply_settings(&r, &mut SystemEnvironment).unwrap();
        assert_eq!(
            store
                .settings()
                .unwrap()
                .values
                .mission_dispatch_default_slots,
            Some(SettingCount(initial))
        );
    }
    /// Exact read-only Work, priorities and canonical/control event/consent state. sqlite
    /// scalar Values preserve null/type distinctions without recreating state.
    fn snapshot(&self) -> (swarm_engine::Work, Vec<Vec<rusqlite::types::Value>>) {
        let work = self.store.as_ref().unwrap().read_work(&self.work).unwrap();
        let db = rusqlite::Connection::open_with_flags(
            self.path.join(".swarm/state.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut result = Vec::new();
        for sql in [
            "SELECT * FROM cockpit_controls ORDER BY work_id",
            "SELECT * FROM cockpit_tasks ORDER BY work_id,task_id",
            "SELECT * FROM cockpit_events ORDER BY seq",
            "SELECT * FROM events ORDER BY id",
        ] {
            let mut statement = db.prepare(sql).unwrap();
            let n = statement.column_count();
            let mut rows = statement.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                result.push((0..n).map(|i| row.get(i).unwrap()).collect());
            }
            drop(rows);
            statement.finalize().unwrap();
        }
        db.close().unwrap();
        (work, result)
    }
    /// Apply one new event with current actual revision through common Root.
    fn apply(&mut self, action: &str, event: &str) -> swarm_engine::Work {
        let before = self.store.as_ref().unwrap().read_work(&self.work).unwrap();
        let raw=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":before.revision,"reason":"Explicit operator departure control"})).unwrap();
        self.store
            .as_mut()
            .unwrap()
            .execute_external(&self.work, action, &raw, &mut SystemEnvironment)
            .unwrap()
            .result
            .work
    }
}
impl Drop for Fixture {
    /// Close owned Root explicitly; never delete failed fixtures or mask a panic.
    fn drop(&mut self) {
        if let Some(store) = self.store.take() {
            let result = store.close();
            if !std::thread::panicking() {
                result.unwrap();
                fs::remove_dir_all(&self.path).unwrap();
            }
        }
    }
}
/// Read absence, valid non-DDL settings and all public controls before replay,
/// policy tightening, stale revision and foreign-field refusals. No process runs.
#[test]
fn actual_controls_preserve_absence_consent_policy_and_canonical_replay() {
    let mut f = Fixture::new();
    f.policy("initial-slots", 17, 33);
    let initial = f
        .store
        .as_ref()
        .unwrap()
        .mission_control_view(&f.work)
        .unwrap();
    assert!(
        initial.paused.is_none()
            && initial.autonomy.is_none()
            && initial.slots.is_none()
            && initial.consent_enabled.is_none()
    );
    let before = f.snapshot();
    let unpause=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"unpause-without-controls","expected_revision":1,"reason":"No implicit initialization"})).unwrap();
    assert_eq!(
        f.store
            .as_mut()
            .unwrap()
            .execute_external(&f.work, "mission.unpause", &unpause, &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "mission-controls-required"
    );
    assert_eq!(f.snapshot(), before);
    let paused = f.apply("mission.pause", "pause");
    let view = f
        .store
        .as_ref()
        .unwrap()
        .mission_control_view(&f.work)
        .unwrap();
    assert_eq!(view.paused, Some(true));
    assert_eq!(view.autonomy, Some("manuel"));
    assert_eq!(view.slots, Some(17));
    assert!(view.consent_enabled.is_none());
    let unpaused = f.apply("mission.unpause", "unpause");
    assert_eq!(unpaused.revision, paused.revision + 1);
    let view = f
        .store
        .as_ref()
        .unwrap()
        .mission_control_view(&f.work)
        .unwrap();
    assert_eq!(view.paused, Some(false));
    assert_eq!(view.autonomy, Some("manuel"));
    assert!(view.consent_enabled.is_none());
    let stopped = f.apply("mission.stop", "stop");
    let view = f
        .store
        .as_ref()
        .unwrap()
        .mission_control_view(&f.work)
        .unwrap();
    assert_eq!(view.paused, Some(true));
    assert_eq!(view.consent_enabled, Some(false));
    let before = f.snapshot();
    let old=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"pause","expected_revision":1,"reason":"Explicit operator departure control"})).unwrap();
    let replay = f
        .store
        .as_mut()
        .unwrap()
        .execute_external(&f.work, "mission.pause", &old, &mut SystemEnvironment)
        .unwrap();
    assert!(replay.result.replayed);
    assert_eq!(replay.result.work, stopped);
    assert_eq!(f.snapshot(), before);
    f.policy("tighter-slots", 3, 4);
    let still_paused = f.apply("mission.pause", "pause-after-policy");
    assert_eq!(
        f.store
            .as_ref()
            .unwrap()
            .mission_control_view(&f.work)
            .unwrap()
            .slots,
        Some(17)
    );
    let before = f.snapshot();
    let stale=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"stale","expected_revision":1,"reason":"Preserve current state"})).unwrap();
    assert_eq!(
        f.store
            .as_mut()
            .unwrap()
            .execute_external(&f.work, "mission.stop", &stale, &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "revision_conflict"
    );
    assert_eq!(f.snapshot(), before);
    let foreign=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"foreign","expected_revision":still_paused.revision,"reason":"Cannot declare consent","enabled":true})).unwrap();
    assert_eq!(
        f.store
            .as_mut()
            .unwrap()
            .execute_external(&f.work, "mission.stop", &foreign, &mut SystemEnvironment)
            .err()
            .unwrap()
            .code(),
        "mission-control-request-invalid"
    );
    assert_eq!(f.snapshot(), before);
}
/// Negative SQL trigger fails after the staged controls write, proving rollback
/// of controls/audit/Work, then recovery of the exact same unrecorded request.
/// The trigger is an owned fault only; no positive receipt/permission is seeded.
#[test]
fn audit_failure_rolls_back_staged_controls_then_same_request_recovers() {
    let mut f = Fixture::new();
    let before = f.snapshot();
    let db = rusqlite::Connection::open(f.path.join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER owned_pause_failure BEFORE INSERT ON cockpit_events WHEN NEW.kind='pause' BEGIN SELECT RAISE(ABORT,'owned audit refusal'); END").unwrap();
    let raw=serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"pause-recovery","expected_revision":1,"reason":"Exercise transaction rollback"})).unwrap();
    let error = f
        .store
        .as_mut()
        .unwrap()
        .execute_external(&f.work, "mission.pause", &raw, &mut SystemEnvironment)
        .err()
        .unwrap();
    assert!(
        matches!(error,RootStoreError::Mutation(WorkEngineError::Mission(ref e)) if matches!(e.as_ref(),MissionSupervisionError::Sql(_)))
    );
    assert_eq!(f.snapshot(), before);
    db.execute_batch("DROP TRIGGER owned_pause_failure")
        .unwrap();
    db.close().unwrap();
    let actual = f
        .store
        .as_mut()
        .unwrap()
        .execute_external(&f.work, "mission.pause", &raw, &mut SystemEnvironment)
        .unwrap();
    assert!(!actual.result.replayed);
    assert_eq!(actual.result.work.revision, 2);
    assert_eq!(
        f.store
            .as_ref()
            .unwrap()
            .mission_control_view(&f.work)
            .unwrap()
            .paused,
        Some(true)
    );
}

/// A genuine successful ordinal write precedes audit-trigger injection on an
/// already-open Root. Require typed SQL failure, unchanged Work/value/events,
/// then exact-request recovery/replay. No schema check or authority is bypassed:
/// the extra trigger is fixture-only and removed before normal reopen.
#[test]
fn priority_audit_refusal_rolls_back_saved_value_and_recovers() {
    let mut f=Fixture::new();
    let add=serde_json::json!({"schema_version":1,"event_id":"priority-task","expected_revision":1,"id":"task","title":"Ordinal fixture","deliverable":"Atomic metadata","criteria":["No execution permission"]});
    f.store.as_mut().unwrap().execute_external(&f.work,"task.add",&serde_json::to_vec(&add).unwrap(),&mut SystemEnvironment).unwrap();
    let request=|event:&str,revision:i64,value:&str|serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":event,"expected_revision":revision,"task_id":"task","priority":value,"reason":"Owned atomic ordinal fixture"})).unwrap();
    f.store.as_mut().unwrap().execute_external(&f.work,"mission.priority",&request("priority-valid",2,"1"),&mut SystemEnvironment).unwrap();
    assert_eq!(f.store.as_ref().unwrap().mission_task_order(&f.work).unwrap().tasks[0].priority,"1");
    let db=rusqlite::Connection::open(f.path.join(".swarm/state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER owned_priority_failure BEFORE INSERT ON cockpit_events WHEN NEW.kind='priority' BEGIN SELECT RAISE(ABORT,'owned priority audit refusal'); END").unwrap();
    let before=f.snapshot();let raw=request("priority-recovery",3,"-9223372036854775808");
    let error=f.store.as_mut().unwrap().execute_external(&f.work,"mission.priority",&raw,&mut SystemEnvironment).err().unwrap();
    assert!(matches!(error,RootStoreError::Mutation(WorkEngineError::Mission(ref e)) if matches!(e.as_ref(),MissionSupervisionError::Sql(_))));
    assert_eq!(f.snapshot(),before);
    assert_eq!(f.store.as_ref().unwrap().mission_task_order(&f.work).unwrap().tasks[0].priority,"1");
    db.execute_batch("DROP TRIGGER owned_priority_failure").unwrap();db.close().unwrap();
    let actual=f.store.as_mut().unwrap().execute_external(&f.work,"mission.priority",&raw,&mut SystemEnvironment).unwrap();assert!(!actual.result.replayed);assert_eq!(actual.result.work.revision,4);
    assert_eq!(f.store.as_ref().unwrap().mission_task_order(&f.work).unwrap().tasks[0].priority,i64::MIN.to_string());
    let recorded=f.snapshot();let again=f.store.as_mut().unwrap().execute_external(&f.work,"mission.priority",&raw,&mut SystemEnvironment).unwrap();assert!(again.result.replayed);assert_eq!(f.snapshot(),recorded);
}
