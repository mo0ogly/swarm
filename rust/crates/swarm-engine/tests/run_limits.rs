#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{
    resolve_run_limits, EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues,
    RootStore, RunLimits, RuntimeLimitPreview, SettingCap, SettingCount, SystemEnvironment,
};
struct Fixture(PathBuf);
impl Fixture {
    /// Private real native root; no mission/provider or live data.
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("runtime-limits-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        RootStore::initialize(&p, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        Self(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Explicit schema1 preview; absent provider/mission fields inherit saved values.
fn input() -> RuntimeLimitPreview {
    RuntimeLimitPreview {
        schema_version: 1,
        ..Default::default()
    }
}
/// Saved installation values are the only normalization fallback. Null ceilings
/// remove definition bounds, while incomplete/legacy policy refuses explicitly.
#[test]
fn runtime_policy_defaults_nullable_ceilings_and_legacy_adoption() {
    let values = EngineSettingsValues::installation_defaults().unwrap();
    let p = values.runtime_limit_policy().unwrap();
    let frozen = p.normalize(&RunLimits::default()).unwrap();
    assert_eq!(
        (
            frozen.silence_seconds,
            frozen.tool_seconds,
            frozen.max_tool_calls,
            frozen.max_repeated_calls,
            frozen.max_consecutive_errors
        ),
        (180, 300, 100, 4, 3)
    );
    let mut changed = values.clone();
    changed.runtime_max_tool_calls_ceiling = Some(SettingCap(None));
    changed.runtime_max_tool_calls_default = Some(SettingCount(10001));
    changed.validate().unwrap();
    assert_eq!(
        changed
            .runtime_limit_policy()
            .unwrap()
            .normalize(&RunLimits::default())
            .unwrap()
            .max_tool_calls,
        10001
    );
    changed.runtime_max_tool_calls_ceiling = Some(SettingCap(Some(SettingCount(10000))));
    assert!(changed.validate().is_err());
    let mut raw = serde_json::to_value(&values).unwrap();
    let catalog = swarm_engine::EngineSettingsCatalog::installation().unwrap();
    let keys: Vec<_> = catalog
        .fields
        .iter()
        .filter(|f| f.introduced_schema_version > 14)
        .map(|f| f.name.clone())
        .collect();
    for key in &keys {
        raw.as_object_mut().unwrap().remove(key);
    }
    let legacy = EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(legacy.schema_version(), 14);
    assert!(legacy.runtime_limit_policy().is_err());
    let old =
        EngineSettingsDocument::initial(legacy, "fixture".into(), "2026-10-19T00:00:00Z".into())
            .unwrap();
    let mut request = EngineSettingsChange {
        schema_version: 15,
        event_id: "adopt-runtime".into(),
        expected_revision: old.revision,
        values: {
            let mut adopted = raw.clone();
            let current = serde_json::to_value(&values).unwrap();
            for field in &catalog.fields {
                if field.introduced_schema_version == 15 {
                    adopted[&field.name] = current[&field.name].clone();
                }
            }
            EngineSettingsValues::decode(&serde_json::to_vec(&adopted).unwrap()).unwrap()
        },
        reason: "Explicit runtime policy adoption".into(),
        preview_token: String::new(),
    };
    request.preview_token = old.preview(&request).unwrap().preview_token;
    let (new, replayed) = old
        .apply(&request, "fixture".into(), "2026-10-19T01:00:00Z".into())
        .unwrap();
    assert!(!replayed);
    assert_eq!(new.schema_version, 15);
    assert_eq!(&new.history[..old.history.len()], &old.history);
    assert!(new.values.runtime_limit_policy().is_ok());
    raw[&keys[0]] = serde_json::json!("1");
    assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
}
/// Mission requests cannot raise budgets or enable observation; current Admin
/// durations may replace old observations but previous consumed/call ceilings stay.
#[test]
fn runtime_resolution_tightens_and_preserves_previous_attempt_contract() {
    let policy = EngineSettingsValues::installation_defaults()
        .unwrap()
        .runtime_limit_policy()
        .unwrap();
    let empty = RunLimits::default();
    let mut mission = RunLimits {
        max_tool_calls: 50,
        ..Default::default()
    };
    let prior = RunLimits {
        max_tool_calls: 20,
        tool_seconds: 40,
        ..Default::default()
    };
    let admin = RunLimits {
        tool_seconds: 700,
        observation_mode: 1,
        ..Default::default()
    };
    let value = resolve_run_limits(&empty, &mission, &admin, Some(&prior), &policy).unwrap();
    assert_eq!(value.max_tool_calls, 20);
    assert_eq!(value.tool_seconds, 700);
    assert_eq!(value.observation_mode, 1);
    mission.tool_seconds = 60;
    assert_eq!(
        resolve_run_limits(&empty, &mission, &admin, Some(&prior), &policy)
            .unwrap()
            .tool_seconds,
        60
    );
    mission.max_tool_calls = 101;
    assert_eq!(
        resolve_run_limits(&empty, &mission, &empty, None, &policy)
            .unwrap_err()
            .code(),
        "runtime-mission-cannot-relax-provider"
    );
    for bad in [
        RunLimits {
            observation_mode: 1,
            ..Default::default()
        },
        RunLimits {
            max_tool_calls: -1,
            ..Default::default()
        },
        RunLimits {
            max_tool_calls: 10001,
            ..Default::default()
        },
    ] {
        assert!(resolve_run_limits(&empty, &bad, &empty, None, &policy).is_err());
    }
    assert!(resolve_run_limits(
        &empty,
        &empty,
        &empty,
        Some(&RunLimits {
            max_tool_calls: -1,
            ..Default::default()
        }),
        &policy
    )
    .is_err());
}
/// Save through public preview/apply, reopen, and observe changed normalization
/// under real persisted scope overlays. Reads preserve source/config/history.
#[test]
fn runtime_root_saved_policy_and_scope_overlays_reach_actual_consumer() {
    let f = Fixture::new();
    let store = RootStore::open(&f.0, true).unwrap();
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.runtime_max_tool_calls_default = Some(SettingCount(73));
    let mut r = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: "runtime-save".into(),
        expected_revision: doc.revision,
        values,
        reason: "Configure future launches".into(),
        preview_token: String::new(),
    };
    let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    r.preview_token = store.preview_settings(&r).unwrap().preview_token;
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    store.apply_settings(&r, &mut SystemEnvironment).unwrap();
    store.close().unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    for (scope, work, key, body) in [
        ("project", "", "", r#"{"max_tool_calls":60}"#),
        (
            "mission",
            "work",
            "",
            r#"{"max_tool_calls":40,"observation_mode":1}"#,
        ),
        ("role", "work", "worker", r#"{"max_tool_calls":30}"#),
        ("task", "work", "task", r#"{"max_tool_calls":10}"#),
    ] {
        db.execute("INSERT INTO run_limits_config VALUES(?,?,?,1,?,'fixture','Explicit setting','2026-10-19T00:00:00Z')",rusqlite::params![scope,work,key,body.as_bytes()]).unwrap();
    }
    let mut readonly = RootStore::open(&f.0, false).unwrap();
    let saved = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let value = readonly
        .preview_agent_limits("work", "worker", "task", &input())
        .unwrap();
    assert_eq!(value.max_tool_calls, 10);
    assert_eq!(value.observation_mode, 1);
    assert_eq!(
        readonly
            .preview_agent_limits("other", "worker", "task", &input())
            .unwrap()
            .max_tool_calls,
        60
    );
    db.execute("DELETE FROM run_limits_config", []).unwrap();
    assert_eq!(
        readonly
            .preview_agent_limits("work", "worker", "task", &input())
            .unwrap()
            .max_tool_calls,
        73
    );
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        saved
    );
    readonly.close().unwrap();
    db.close().unwrap();
}
/// Every corrupt stored override has the same valid witness, refuses without
/// becoming inherited success, then recovers after original valid data returns.
#[test]
fn runtime_root_corrupt_override_refuses_without_fallback_then_recovers() {
    for (revision, scope, body, reason) in [
        (0, "mission", r#"{"max_tool_calls":10}"#, "reason"),
        (1, "mission", "{PRIVATE", "reason"),
        (1, "mission", r#"{"max_tool_calls":-1}"#, "reason"),
        (1, "role", r#"{"observation_mode":1}"#, "reason"),
        (1, "mission", r#"{"observation_mode":1}"#, ""),
    ] {
        let f = Fixture::new();
        let mut store = RootStore::open(&f.0, false).unwrap();
        let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
        db.execute("INSERT INTO run_limits_config VALUES('mission','work','',1,?,'fixture','reason','2026-10-19T00:00:00Z')",[br#"{"max_tool_calls":10}"#.as_slice()]).unwrap();
        assert_eq!(
            store
                .preview_agent_limits("work", "worker", "task", &input())
                .unwrap()
                .max_tool_calls,
            10
        );
        db.execute(
            "UPDATE run_limits_config SET revision=?,scope=?,scope_key=?,body=?,reason=?",
            rusqlite::params![
                revision,
                scope,
                if scope == "role" { "worker" } else { "" },
                body.as_bytes(),
                reason
            ],
        )
        .unwrap();
        let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let e = store
            .preview_agent_limits("work", "worker", "task", &input())
            .unwrap_err();
        assert!(!e.to_string().contains("PRIVATE"));
        assert_eq!(
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            before
        );
        db.execute("DELETE FROM run_limits_config", []).unwrap();
        assert_eq!(
            store
                .preview_agent_limits("work", "worker", "task", &input())
                .unwrap()
                .max_tool_calls,
            100
        );
        store.close().unwrap();
        db.close().unwrap();
    }
}
