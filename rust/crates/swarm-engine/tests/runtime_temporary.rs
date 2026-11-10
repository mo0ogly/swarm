#![cfg(unix)]
//! v51 runtime temporary directory consumer: applying the saved setting
//! creates the directory (0700), exports TMPDIR for this process and its
//! children; a Root without settings is left untouched; an invalid saved
//! value refuses. Fixture Roots live in the configured fixture area (never /tmp).
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};
use swarm_engine::{EngineSettingsChange, RootStore, SystemEnvironment};

fn root(label: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures").canonicalize().unwrap();
    let r = base.join(swarm_engine::generate_identifier(&format!("runtime-tmp-{label}-")).unwrap());
    fs::create_dir(&r).unwrap();
    fs::set_permissions(&r, fs::Permissions::from_mode(0o700)).unwrap();
    r
}

/// Single test: TMPDIR is process-global, so the cases run in order.
#[test]
fn apply_exports_tmpdir_to_children_and_refuses_invalid() {
    let empty = root("empty");
    assert_eq!(RootStore::apply_runtime_temporary_directory(&empty).unwrap(), None);

    let r = root("default");
    let store = RootStore::initialize(&r, &mut SystemEnvironment).unwrap();
    store.close().unwrap();
    let applied = RootStore::apply_runtime_temporary_directory(&r).unwrap().unwrap();
    assert_eq!(applied, r.join(".swarm-fixtures"));
    assert_eq!(fs::metadata(&applied).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(std::env::temp_dir(), applied);
    let child = Command::new("sh").args(["-c", "printf %s \"$TMPDIR\""]).output().unwrap();
    assert_eq!(String::from_utf8(child.stdout).unwrap(), applied.to_str().unwrap());

    // Saved custom absolute value through the public preview/apply path.
    let custom = r.join("custom-area");
    let store = RootStore::open(&r, true).unwrap();
    let d = store.settings().unwrap();
    let mut values = d.values;
    values.runtime_temporary_directory = Some(custom.to_str().unwrap().into());
    let mut change = EngineSettingsChange {
        schema_version: d.schema_version,
        event_id: "runtime-tmp-custom".into(),
        expected_revision: d.revision,
        values: values.clone(),
        reason: "Fixture custom temporary directory".into(),
        preview_token: String::new(),
    };
    change.preview_token = store.preview_settings(&change).unwrap().preview_token;
    store.apply_settings(&change, &mut SystemEnvironment).unwrap();
    // /tmp is refused at preview, so it can never be saved.
    let d = store.settings().unwrap();
    let mut bad = values;
    bad.runtime_temporary_directory = Some("/tmp/swarm".into());
    let refused = EngineSettingsChange {
        schema_version: d.schema_version,
        event_id: "runtime-tmp-bad".into(),
        expected_revision: d.revision,
        values: bad,
        reason: "Fixture /tmp refusal".into(),
        preview_token: String::new(),
    };
    assert!(store.preview_settings(&refused).is_err());
    store.close().unwrap();
    assert_eq!(RootStore::apply_runtime_temporary_directory(&r).unwrap().unwrap(), custom);
    assert_eq!(std::env::var("TMPDIR").unwrap(), custom.to_str().unwrap());

    // An existing non-directory at the configured path refuses.
    fs::remove_dir(&custom).unwrap();
    fs::write(&custom, b"not a dir").unwrap();
    assert_eq!(
        RootStore::apply_runtime_temporary_directory(&r).unwrap_err().code(),
        "runtime-temporary-directory"
    );
    fs::remove_dir_all(&empty).unwrap();
    fs::remove_dir_all(&r).unwrap();
}
