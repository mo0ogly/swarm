#![cfg(unix)]
use std::{fs, path::PathBuf};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("swarm-context-guard-").unwrap());
        fs::create_dir(&root).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
    fn profile(&self) {
        fs::write(self.0.join("swarm.project.json"),br#"{"version":1,"name":"Project","instructions":[{"path":"guide.md","roles":["reviewer","worker"]}]}"#).unwrap();
        fs::write(self.0.join("guide.md"), b"Current project rules").unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
#[test]
fn root_context_guard_rereads_sources_profile_role_and_presence_without_mutation() {
    let f = Fixture::new();
    let store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    store.check_project_context(None, "reviewer").unwrap();
    f.profile();
    let previous = store.project_context("reviewer").unwrap().unwrap().context;
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    assert!(store
        .check_project_context(None, "reviewer")
        .unwrap_err()
        .to_string()
        .contains("project-context-stale"));
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    fs::write(f.0.join("guide.md"), b"Changed project rules").unwrap();
    assert!(store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap_err()
        .to_string()
        .contains("project-context-stale"));
    f.profile();
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    let profile = fs::read(f.0.join("swarm.project.json")).unwrap();
    fs::write(
        f.0.join("swarm.project.json"),
        [profile.as_slice(), b"\n"].concat(),
    )
    .unwrap();
    assert!(store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap_err()
        .to_string()
        .contains("project-context-stale"));
    f.profile();
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    assert!(store
        .check_project_context(Some(&previous), "worker")
        .unwrap_err()
        .to_string()
        .contains("project-context-stale"));
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    fs::remove_file(f.0.join("swarm.project.json")).unwrap();
    assert!(store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap_err()
        .to_string()
        .contains("project-context-stale"));
    store.check_project_context(None, "reviewer").unwrap();
    f.profile();
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    fs::write(f.0.join("swarm.project.json"), b"{invalid").unwrap();
    assert!(store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap_err()
        .to_string()
        .contains("profile-json"));
    assert_eq!(
        fs::read(f.0.join("swarm.project.json")).unwrap(),
        b"{invalid"
    );
    f.profile();
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        settings
    );
    assert!(store.list_works().unwrap().is_empty());
    store.close().unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    store.close().unwrap();
}
#[test]
fn root_context_guard_consumes_current_policy_and_preserves_refused_inputs() {
    let f = Fixture::new();
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    f.profile();
    let previous = store.project_context("reviewer").unwrap().unwrap().context;
    for (event, cap, passes) in [
        ("small", Some(SettingCount(1)), false),
        ("zero", Some(SettingCount(0)), false),
        ("unlimited", None, true),
    ] {
        change_cap(&mut store, &format!("{event}-control"), None);
        store
            .check_project_context(Some(&previous), "reviewer")
            .unwrap();
        change_cap(&mut store, event, cap);
        let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let source = fs::read(f.0.join("guide.md")).unwrap();
        let result = store.check_project_context(Some(&previous), "reviewer");
        if passes {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().to_string().contains("configured-cap"));
        }
        assert_eq!(
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        assert_eq!(fs::read(f.0.join("guide.md")).unwrap(), source);
    }
    store.close().unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    store
        .check_project_context(Some(&previous), "reviewer")
        .unwrap();
    store.close().unwrap();
}

fn change_cap(store: &mut RootStore, event: &str, cap: Option<SettingCount>) {
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.project_context_max_bytes = Some(SettingCap(cap));
    let mut change = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: event.into(),
        expected_revision: doc.revision,
        values,
        reason: "Observed context policy".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store
        .apply_settings(&change, &mut SystemEnvironment)
        .unwrap();
}
#[test]
fn root_context_guard_never_bypasses_missing_legacy_policy_even_when_profile_absent() {
    let f = Fixture::new();
    let store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    store.check_project_context(None, "reviewer").unwrap();
    store.close().unwrap();
    let file = f.0.join(".swarm/engine-settings.json");
    let original = fs::read(&file).unwrap();
    let legacy = include_bytes!("fixtures/legacy-v3-document.json");
    fs::write(&file, legacy).unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert!(store
        .check_project_context(None, "reviewer")
        .unwrap_err()
        .to_string()
        .contains("settings-context-policy-not-configured"));
    assert_eq!(fs::read(&file).unwrap(), legacy);
    store.close().unwrap();
    fs::write(&file, &original).unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    store.check_project_context(None, "reviewer").unwrap();
    store.close().unwrap();
}
