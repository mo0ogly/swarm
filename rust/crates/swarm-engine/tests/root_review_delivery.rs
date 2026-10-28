#![cfg(unix)]
use std::{fs, path::PathBuf};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("review-document-").unwrap());
        fs::create_dir(&p).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
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
#[test]
fn root_document_reader_consumes_saved_cap_without_mutation_and_recovers() {
    let f = Fixture::new();
    let mut store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    fs::create_dir(f.0.join("docs")).unwrap();
    let file = f.0.join("docs/delivery.md");
    fs::write(&file, b"Review delivery").unwrap();
    for (event, cap) in [
        ("small", Some(SettingCount(1))),
        ("zero", Some(SettingCount(0))),
        ("default", Some(SettingCount(48000))),
        ("unlimited", None),
    ] {
        change(&mut store, &format!("{event}-control"), None);
        assert_eq!(
            store.review_delivery("docs/delivery.md").unwrap(),
            "Review delivery"
        );
        change(&mut store, event, cap);
        let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let result = store.review_delivery("docs/delivery.md");
        if cap.is_some_and(|v| v.0 < 15) {
            assert!(result.unwrap_err().to_string().contains("configured-cap"));
        } else {
            assert_eq!(result.unwrap(), "Review delivery");
        }
        assert_eq!(fs::read(&file).unwrap(), b"Review delivery");
        assert_eq!(
            fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        assert!(store.list_works().unwrap().is_empty());
    }
    store.close().unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        store.review_delivery("docs/delivery.md").unwrap(),
        "Review delivery"
    );
    store.close().unwrap();
}
#[test]
fn root_document_reader_preserves_missing_invalid_and_unconfigured_errors() {
    let f = Fixture::new();
    let store = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    fs::write(f.0.join("ordinary.md"), b"Text").unwrap();
    assert_eq!(store.review_delivery("ordinary.md").unwrap(), "Text");
    for name in ["missing.md", "../outside.md"] {
        assert!(store.review_delivery(name).is_err());
        assert_eq!(store.review_delivery("ordinary.md").unwrap(), "Text");
    }
    fs::write(f.0.join("ordinary.md"), b"NUL\0text").unwrap();
    assert!(store
        .review_delivery("ordinary.md")
        .unwrap_err()
        .to_string()
        .contains("source-text-required"));
    fs::write(f.0.join("ordinary.md"), b"Text").unwrap();
    assert_eq!(store.review_delivery("ordinary.md").unwrap(), "Text");
    store.close().unwrap();
    let settings = f.0.join(".swarm/engine-settings.json");
    let current = fs::read(&settings).unwrap();
    let legacy = include_bytes!("fixtures/legacy-v3-document.json");
    fs::write(&settings, legacy).unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        store.review_delivery("ordinary.md").unwrap_err().code(),
        "settings-context-policy-not-configured"
    );
    assert_eq!(fs::read(&settings).unwrap(), legacy);
    store.close().unwrap();
    fs::write(&settings, current).unwrap();
    let store = RootStore::open(&f.0, false).unwrap();
    assert_eq!(store.review_delivery("ordinary.md").unwrap(), "Text");
    store.close().unwrap();
}
fn change(store: &mut RootStore, event: &str, cap: Option<SettingCount>) {
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.review_delivery_max_bytes = Some(SettingCap(cap));
    let mut request = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: event.into(),
        expected_revision: doc.revision,
        values,
        reason: "Actual document cap".into(),
        preview_token: String::new(),
    };
    request.preview_token = store.preview_settings(&request).unwrap().preview_token;
    store
        .apply_settings(&request, &mut SystemEnvironment)
        .unwrap();
}
