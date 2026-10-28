#![cfg(target_os = "linux")]
use std::{fs, os::unix::fs::PermissionsExt};
use swarm_engine::{
    EngineSettingsChange, Provider, Providers, RootStore, SettingCap, SettingCount,
    SystemEnvironment,
};
/// Owned private test root, retained on a panic for diagnosis.
struct OwnedRoot(std::path::PathBuf);
impl OwnedRoot {
    /// Borrow only this fixture's root; this is not a live mission path.
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for OwnedRoot {
    /// Clean only a successful fixture; preserve a failed case for diagnosis.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Fresh private Root with real settings/storage and existing engine dependencies.
/// No provider/server/account or unlisted tempfile crate is required.
fn root() -> OwnedRoot {
    let root = OwnedRoot(
        std::env::temp_dir().join(swarm_engine::generate_identifier("provider-config-").unwrap()),
    );
    fs::create_dir(root.path()).unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    RootStore::initialize(root.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    root
}
/// Explicit staging declaration; no executable/default model is inferred.
fn config() -> Providers {
    Providers {
        schema_version: 1,
        providers: Some(
            [(
                "fixture".into(),
                Provider {
                    command: "/fixture/program".into(),
                    args: Some(vec!["explicit".into()]),
                    ..Default::default()
                },
            )]
            .into(),
        ),
    }
}
/// Apply the actual persisted cap with preview, preserving settings history.
fn cap(store: &RootStore, value: Option<i64>) {
    let doc = store.settings().unwrap();
    let mut values = doc.values.clone();
    values.agent_provider_config_max_bytes = Some(SettingCap(value.map(SettingCount)));
    let mut request = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: format!("cap-{}", doc.revision.0),
        expected_revision: doc.revision,
        values,
        reason: "Owned configuration cap fixture".into(),
        preview_token: String::new(),
    };
    request.preview_token = store.preview_settings(&request).unwrap().preview_token;
    store
        .apply_settings(&request, &mut SystemEnvironment)
        .unwrap();
}
/// Real publication/readback survives reopening, preserves SQL and refuses any
/// subsequent init. Nested config is retained data, never model/executable proof.
#[test]
fn provider_file_initialize_reopen_and_no_overwrite_preserve_state() {
    let root = root();
    let db = root.path().join(".swarm/state.db");
    let before = fs::read(&db).unwrap();
    let path = root.path().join(".swarm/providers.json");
    let store = RootStore::open(root.path(), true).unwrap();
    let result = store.initialize_provider_configuration(&config()).unwrap();
    assert!(result.created && !result.provider_execution_started);
    assert_eq!(result.configuration, config());
    let raw = fs::read(&path).unwrap();
    let digest = result.digest;
    let error = store
        .initialize_provider_configuration(&config())
        .err()
        .unwrap();
    assert_eq!(error.code(), "provider-configuration-already-exists");
    assert_eq!(fs::read(&path).unwrap(), raw);
    store.close().unwrap();
    let store = RootStore::open(root.path(), false).unwrap();
    let result = store.provider_configuration().unwrap();
    assert_eq!(result.digest, digest);
    assert!(!result.created && !result.provider_execution_started);
    assert_eq!(
        store
            .initialize_provider_configuration(&config())
            .err()
            .unwrap()
            .code(),
        "root-read-only"
    );
    store.close().unwrap();
    assert_eq!(fs::read(db).unwrap(), before);
}
/// Zero cap denies new bytes; explicit unlimited recovers. Invalid outer schema
/// and malformed retained source stay errors, preserve originals and never invent
/// an empty configuration. Only fixture data is restored for the valid witness.
#[test]
fn provider_file_policy_schema_and_read_failures_refuse_then_recover() {
    let root = root();
    let path = root.path().join(".swarm/providers.json");
    let store = RootStore::open(root.path(), true).unwrap();
    store.initialize_provider_configuration(&config()).unwrap();
    let original = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    cap(&store, Some(0));
    assert_eq!(
        store
            .initialize_provider_configuration(&config())
            .err()
            .unwrap()
            .code(),
        "provider-configuration-outside-saved-policy"
    );
    assert!(!path.exists());
    cap(&store, None);
    let mut invalid = config();
    invalid.schema_version = 2;
    assert_eq!(
        store
            .initialize_provider_configuration(&invalid)
            .err()
            .unwrap()
            .code(),
        "provider-configuration-schema-invalid"
    );
    assert!(!path.exists());
    store.initialize_provider_configuration(&config()).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::write(&path, b"PRIVATE malformed").unwrap();
    let error = store.provider_configuration().err().unwrap();
    assert_eq!(error.code(), "provider-configuration-invalid");
    assert!(!error.to_string().contains("PRIVATE"));
    assert_eq!(fs::read(&path).unwrap(), b"PRIVATE malformed");
    fs::write(&path, &original).unwrap();
    store.provider_configuration().unwrap();
    store.close().unwrap();
}
/// Compile-gated failures reach the actual no-replace publisher; each has a valid
/// create witness. Before rename no target appears, then removing the fault
/// recovers. A visible-but-unsynced directory is reported preserved/uncertain,
/// never rollback or a reason to overwrite the target on replay.
#[cfg(feature = "test-fixtures")]
#[test]
fn provider_file_create_faults_and_visible_uncertainty_preserve_causes_and_recover() {
    use swarm_engine::{AtomicStorageFault as F, PublicationState, RootStoreError};
    for fault in [
        F::PartialWriteNoSpace,
        F::SyncFile,
        F::CloseFile,
        F::Rename,
        F::SyncAndCleanup,
        F::SyncDirectory,
    ] {
        let root = root();
        let path = root.path().join(".swarm/providers.json");
        let store = RootStore::open(root.path(), true).unwrap();
        store.initialize_provider_configuration(&config()).unwrap();
        let expected = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        let error = store
            .fixture_initialize_provider_configuration(&config(), fault)
            .err()
            .unwrap();
        if fault == F::SyncDirectory {
            assert_eq!(error.code(), "root-state-preserved");
            match &error {
                RootStoreError::Published { primary } => match primary.as_ref() {
                    RootStoreError::Storage(e) => {
                        assert_eq!(e.publication, PublicationState::RenamedDurabilityUnknown)
                    }
                    _ => panic!("storage cause required"),
                },
                _ => panic!("preserved state required"),
            }
            assert_eq!(fs::read(&path).unwrap(), expected);
            store.provider_configuration().unwrap();
            assert_eq!(
                store
                    .initialize_provider_configuration(&config())
                    .err()
                    .unwrap()
                    .code(),
                "provider-configuration-already-exists"
            );
        } else {
            match &error {
                RootStoreError::Storage(e) => {
                    assert_eq!(e.publication, PublicationState::NotPublished);
                    if fault == F::SyncAndCleanup {
                        assert!(e.cleanup.is_some());
                    }
                }
                _ => panic!("storage cause required"),
            }
            assert!(!path.exists());
            store.initialize_provider_configuration(&config()).unwrap();
            assert_eq!(fs::read(&path).unwrap(), expected);
        }
        println!("PASS provider-file-fault {fault:?}");
        store.close().unwrap();
    }
}

/// Build the public strict proposal; expected digest is an actual observation.
fn edit(store: &RootStore, configuration: &Providers) -> swarm_engine::ProviderConfigurationChange {
    let digest = store.inspect_provider_configuration(None).unwrap().digest;
    swarm_engine::decode_provider_configuration_change(&serde_json::to_vec(&serde_json::json!({"schema_version":1,"expected_digest":digest,"configuration":configuration})).unwrap()).unwrap()
}
/// Attach only the token returned by the current readonly preview.
fn confirmed(
    store: &RootStore,
    configuration: &Providers,
) -> swarm_engine::ProviderConfigurationChange {
    let request = edit(store, configuration);
    let token = store
        .preview_provider_configuration(&request)
        .unwrap()
        .preview_token;
    let digest = store.inspect_provider_configuration(None).unwrap().digest;
    swarm_engine::decode_provider_configuration_change(&serde_json::to_vec(&serde_json::json!({"schema_version":1,"expected_digest":digest,"configuration":configuration,"preview_token":token})).unwrap()).unwrap()
}
/// Restore through the same current-source/preview boundary, never by rewriting
/// runtime fixtures. A reference is validated data, not a grant or arbitrary path.
fn restore(store: &RootStore, reference: &str) -> swarm_engine::ProviderConfigurationChange {
    let digest = store.inspect_provider_configuration(None).unwrap().digest;
    let mut wire = serde_json::json!({"schema_version":1,"expected_digest":digest,"restore_reference":reference});
    let r = swarm_engine::decode_provider_configuration_change(&serde_json::to_vec(&wire).unwrap())
        .unwrap();
    wire["preview_token"] = store
        .preview_provider_configuration(&r)
        .unwrap()
        .preview_token
        .into();
    swarm_engine::decode_provider_configuration_change(&serde_json::to_vec(&wire).unwrap()).unwrap()
}
/// Actual directory names bind readonly preview assertions, including absence of
/// a newly created provider lock or snapshot; no source content is hidden.
fn names(root: &std::path::Path) -> Vec<std::ffi::OsString> {
    let mut v: Vec<_> = fs::read_dir(root.join(".swarm"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    v.sort();
    v
}
/// Preview has no filesystem mutation. CAS/token protect apply; content snapshots
/// survive repeated changes and exact restore. No-op sync creates no snapshot,
/// and no operation changes the mission database or claims provider execution.
#[test]
fn provider_file_preview_apply_restore_and_noop_preserve_snapshots_and_sql() {
    let root = root();
    let path = root.path().join(".swarm/providers.json");
    let store = RootStore::open(root.path(), true).unwrap();
    store.initialize_provider_configuration(&config()).unwrap();
    let original = fs::read(&path).unwrap();
    let database = fs::read(root.path().join(".swarm/state.db")).unwrap();
    let mut next = config();
    next.providers
        .as_mut()
        .unwrap()
        .get_mut("fixture")
        .unwrap()
        .command = "/fixture/replacement".into();
    let request = edit(&store, &next);
    let before = names(root.path());
    let preview = store.preview_provider_configuration(&request).unwrap();
    assert!(preview.preview && !preview.applied && preview.changed);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(names(root.path()), before);
    assert_eq!(
        store
            .apply_provider_configuration(&request)
            .err()
            .unwrap()
            .code(),
        "provider-configuration-preview-required"
    );
    assert_eq!(names(root.path()), before);
    let request = confirmed(&store, &next);
    let applied = store.apply_provider_configuration(&request).unwrap();
    assert!(applied.applied && applied.changed && !applied.provider_execution_started);
    let prior = applied.backup_reference.unwrap();
    assert_eq!(
        fs::read(root.path().join(".swarm").join(&prior)).unwrap(),
        original
    );
    assert!(store
        .inspect_provider_configuration(Some(&prior))
        .unwrap()
        .matches_reference
        .unwrap());
    let changed = fs::read(&path).unwrap();
    let refs = store.provider_configuration_backups().unwrap().references;
    assert!(refs.contains(&prior));
    assert_eq!(
        store
            .apply_provider_configuration(&request)
            .err()
            .unwrap()
            .code(),
        "revision_conflict"
    );
    assert_eq!(fs::read(&path).unwrap(), changed);
    assert_eq!(
        store.provider_configuration_backups().unwrap().references,
        refs
    );
    let noop = store
        .apply_provider_configuration(&confirmed(&store, &next))
        .unwrap();
    assert!(noop.applied && !noop.changed && noop.backup_reference.is_none());
    assert_eq!(
        store.provider_configuration_backups().unwrap().references,
        refs
    );
    let restored = store
        .apply_provider_configuration(&restore(&store, &prior))
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(
        fs::read(
            root.path()
                .join(".swarm")
                .join(restored.backup_reference.unwrap())
        )
        .unwrap(),
        changed
    );
    assert_eq!(
        fs::read(root.path().join(".swarm").join(&prior)).unwrap(),
        original
    );
    store.close().unwrap();
    let store = RootStore::open(root.path(), false).unwrap();
    assert_eq!(
        store.provider_configuration().unwrap().configuration,
        config()
    );
    store.close().unwrap();
    assert_eq!(
        fs::read(root.path().join(".swarm/state.db")).unwrap(),
        database
    );
}
/// Settings revisions invalidate preview tokens. Go's provider lock cannot be
/// bypassed; release is a changed precondition for recovery. Corrupt/symlink
/// snapshots refuse without altering current bytes; another valid snapshot works.
#[test]
fn provider_file_stale_policy_lock_and_unsafe_restore_refuse_then_recover() {
    use std::os::unix::fs::{symlink, OpenOptionsExt};
    let root = root();
    let path = root.path().join(".swarm/providers.json");
    let store = RootStore::open(root.path(), true).unwrap();
    store.initialize_provider_configuration(&config()).unwrap();
    let original = fs::read(&path).unwrap();
    let mut next = config();
    next.providers
        .as_mut()
        .unwrap()
        .get_mut("fixture")
        .unwrap()
        .command = "/fixture/replacement".into();
    let request = confirmed(&store, &next);
    cap(&store, None);
    assert_eq!(
        store
            .apply_provider_configuration(&request)
            .err()
            .unwrap()
            .code(),
        "provider-configuration-preview-required"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(store
        .provider_configuration_backups()
        .unwrap()
        .references
        .is_empty());
    assert!(!root.path().join(".swarm/providers.lock").exists());
    let request = confirmed(&store, &next);
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(root.path().join(".swarm/providers.lock"))
        .unwrap();
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive).unwrap();
    let error = store.apply_provider_configuration(&request).err().unwrap();
    match error {
        swarm_engine::RootStoreError::Storage(e) => {
            assert_eq!(e.operation, swarm_engine::LocalStorageOperation::Lock)
        }
        _ => panic!("lock cause required"),
    };
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(store
        .provider_configuration_backups()
        .unwrap()
        .references
        .is_empty());
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::Unlock).unwrap();
    nix::unistd::close(lock).unwrap();
    let applied = store.apply_provider_configuration(&request).unwrap();
    let reference = applied.backup_reference.unwrap();
    let changed = fs::read(&path).unwrap();
    fs::write(
        root.path().join(".swarm").join(&reference),
        b"PRIVATE corrupt backup",
    )
    .unwrap();
    let inspected = store
        .inspect_provider_configuration(Some(&reference))
        .unwrap();
    assert_eq!(inspected.matches_reference, Some(false));
    assert!(!inspected.format_valid);
    let wire = serde_json::json!({"schema_version":1,"expected_digest":store.inspect_provider_configuration(None).unwrap().digest,"restore_reference":reference});
    let bad =
        swarm_engine::decode_provider_configuration_change(&serde_json::to_vec(&wire).unwrap())
            .unwrap();
    assert_eq!(
        store
            .preview_provider_configuration(&bad)
            .err()
            .unwrap()
            .code(),
        "provider-configuration-backup-digest-mismatch"
    );
    assert_eq!(fs::read(&path).unwrap(), changed);
    let link = format!(
        "providers-before-{}-snapshot-abcdef.json",
        store.inspect_provider_configuration(None).unwrap().digest
    );
    symlink("providers.json", root.path().join(".swarm").join(&link)).unwrap();
    let wire = serde_json::json!({"schema_version":1,"expected_digest":store.inspect_provider_configuration(None).unwrap().digest,"restore_reference":link});
    let bad =
        swarm_engine::decode_provider_configuration_change(&serde_json::to_vec(&wire).unwrap())
            .unwrap();
    assert!(store.preview_provider_configuration(&bad).is_err());
    assert_eq!(fs::read(&path).unwrap(), changed);
    let mut third = next.clone();
    third
        .providers
        .as_mut()
        .unwrap()
        .get_mut("fixture")
        .unwrap()
        .command = "/fixture/third".into();
    let valid = store
        .apply_provider_configuration(&confirmed(&store, &third))
        .unwrap()
        .backup_reference
        .unwrap();
    store
        .apply_provider_configuration(&restore(&store, &valid))
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), changed);
    store.close().unwrap();
}
/// Explicit metadata inspection enables repair without treating malformed data as
/// valid or dropping its original bytes. Invalid old content is kept for forensics
/// and cannot be silently restored as a working configuration.
#[test]
fn provider_file_corrupt_source_is_inspected_and_repaired_with_original_retained() {
    let root = root();
    let path = root.path().join(".swarm/providers.json");
    let store = RootStore::open(root.path(), true).unwrap();
    store.initialize_provider_configuration(&config()).unwrap();
    store.provider_configuration().unwrap();
    fs::write(&path, b"PRIVATE malformed source").unwrap();
    assert!(store.provider_configuration().is_err());
    let meta = store.inspect_provider_configuration(None).unwrap();
    assert!(!meta.format_valid);
    assert_eq!(
        meta.format_error_code,
        Some("provider-configuration-invalid")
    );
    assert_eq!(meta.scope, "source_metadata_only_no_admission");
    let repaired = store
        .apply_provider_configuration(&confirmed(&store, &config()))
        .unwrap();
    let reference = repaired.backup_reference.unwrap();
    assert_eq!(
        fs::read(root.path().join(".swarm").join(&reference)).unwrap(),
        b"PRIVATE malformed source"
    );
    store.provider_configuration().unwrap();
    let wire = serde_json::json!({"schema_version":1,"expected_digest":store.inspect_provider_configuration(None).unwrap().digest,"restore_reference":reference});
    let invalid =
        swarm_engine::decode_provider_configuration_change(&serde_json::to_vec(&wire).unwrap())
            .unwrap();
    assert_eq!(
        store
            .preview_provider_configuration(&invalid)
            .err()
            .unwrap()
            .code(),
        "provider-configuration-invalid"
    );
    store.provider_configuration().unwrap();
    store.close().unwrap();
}
/// Backup and replacement faults each follow a successful update/restore witness.
/// Before main rename bytes stay exact; after visible uncertainty an explicit
/// same-content apply performs real fsync confirmation without a new snapshot.
#[cfg(feature = "test-fixtures")]
#[test]
fn provider_file_update_phase_failures_preserve_content_history_and_confirm_recovery() {
    use swarm_engine::AtomicStorageFault as F;
    for backup in [true, false] {
        for fault in [
            F::PartialWriteNoSpace,
            F::SyncFile,
            F::CloseFile,
            F::Rename,
            F::SyncAndCleanup,
            F::SyncDirectory,
        ] {
            let root = root();
            let path = root.path().join(".swarm/providers.json");
            let store = RootStore::open(root.path(), true).unwrap();
            store.initialize_provider_configuration(&config()).unwrap();
            let original = fs::read(&path).unwrap();
            let mut next = config();
            next.providers
                .as_mut()
                .unwrap()
                .get_mut("fixture")
                .unwrap()
                .command = "/fixture/replacement".into();
            let valid = store
                .apply_provider_configuration(&confirmed(&store, &next))
                .unwrap()
                .backup_reference
                .unwrap();
            store
                .apply_provider_configuration(&restore(&store, &valid))
                .unwrap();
            assert_eq!(fs::read(&path).unwrap(), original);
            let earlier: Vec<_> = store
                .provider_configuration_backups()
                .unwrap()
                .references
                .into_iter()
                .map(|n| {
                    let b = fs::read(root.path().join(".swarm").join(&n)).unwrap();
                    (n, b)
                })
                .collect();
            let request = confirmed(&store, &next);
            let error = store
                .fixture_apply_provider_configuration(
                    &request,
                    if backup { fault } else { F::None },
                    if backup { F::None } else { fault },
                )
                .err()
                .unwrap();
            if !backup && fault == F::SyncDirectory {
                assert_eq!(error.code(), "root-state-preserved");
                assert_ne!(fs::read(&path).unwrap(), original);
                assert_eq!(
                    store
                        .apply_provider_configuration(&request)
                        .err()
                        .unwrap()
                        .code(),
                    "revision_conflict"
                );
                let refs = store.provider_configuration_backups().unwrap().references;
                let confirmed = store
                    .apply_provider_configuration(&confirmed(&store, &next))
                    .unwrap();
                assert!(confirmed.applied && !confirmed.changed);
                assert_eq!(
                    store.provider_configuration_backups().unwrap().references,
                    refs
                );
            } else {
                assert_eq!(fs::read(&path).unwrap(), original);
                store.apply_provider_configuration(&request).unwrap();
                assert_ne!(fs::read(&path).unwrap(), original);
            }
            for (name, bytes) in earlier {
                assert_eq!(
                    fs::read(root.path().join(".swarm").join(name)).unwrap(),
                    bytes
                );
            }
            println!("PASS provider-update-fault backup={backup} fault={fault:?}");
            store.close().unwrap();
        }
    }
}

/// Identical file content and local revision in different Roots cannot reuse a
/// preview. This binds the actual target, not an authenticated user or secret;
/// the second Root's own fresh preview still authorizes its local file operation.
#[test]
fn provider_file_preview_token_is_bound_to_actual_root_and_policy_values() {
    let a = root();
    let b = root();
    let sa = RootStore::open(a.path(), true).unwrap();
    let sb = RootStore::open(b.path(), true).unwrap();
    sa.initialize_provider_configuration(&config()).unwrap();
    sb.initialize_provider_configuration(&config()).unwrap();
    let mut next = config();
    next.providers
        .as_mut()
        .unwrap()
        .get_mut("fixture")
        .unwrap()
        .command = "/fixture/replacement".into();
    let request = edit(&sa, &next);
    let pa = sa.preview_provider_configuration(&request).unwrap();
    let pb = sb.preview_provider_configuration(&request).unwrap();
    assert_eq!(pa.before_digest, pb.before_digest);
    assert_ne!(pa.preview_token, pb.preview_token);
    let wire = serde_json::json!({"schema_version":1,"expected_digest":pa.before_digest,"configuration":next,"preview_token":pa.preview_token});
    let foreign =
        swarm_engine::decode_provider_configuration_change(&serde_json::to_vec(&wire).unwrap())
            .unwrap();
    let before = fs::read(b.path().join(".swarm/providers.json")).unwrap();
    let files = names(b.path());
    assert_eq!(
        sb.apply_provider_configuration(&foreign)
            .err()
            .unwrap()
            .code(),
        "provider-configuration-preview-required"
    );
    assert_eq!(
        fs::read(b.path().join(".swarm/providers.json")).unwrap(),
        before
    );
    assert_eq!(names(b.path()), files);
    sb.apply_provider_configuration(&confirmed(&sb, &next))
        .unwrap();
    sa.apply_provider_configuration(&confirmed(&sa, &next))
        .unwrap();
    sa.close().unwrap();
    sb.close().unwrap();
}
