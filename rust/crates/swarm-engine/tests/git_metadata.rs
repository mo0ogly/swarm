#![cfg(target_os = "linux")]
use std::{fs, os::unix::ffi::OsStrExt, path::PathBuf, process::Command};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment};
struct Fixture(PathBuf);
impl Fixture {
    /// Initialize a real isolated Git repository and native Root with private data.
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("git-metadata-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        let f = Self(p);
        RootStore::initialize(&f.0, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        f.git(&["init"]);
        fs::write(f.0.join("probe.txt"), b"original\n").unwrap();
        fs::write(f.0.join(".gitattributes"), b"*.txt filter=fixture\n").unwrap();
        f.git(&["add", "probe.txt", ".gitattributes"]);
        f.git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@localhost",
            "commit",
            "-m",
            "Fixture",
        ]);
        f
    }
    /// Fixed fixture setup commands only; metadata under test uses its real shared runner.
    fn git(&self, args: &[&str]) {
        let o = Command::new("git")
            .arg("-C")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Shell-quote only this owned fixture path, for Git's configured test callback.
fn shell_path(path: &std::path::Path) -> String {
    let bytes = path.as_os_str().as_bytes();
    let text = std::str::from_utf8(bytes).unwrap();
    format!("'{}'", text.replace('\'', "'\\''"))
}
fn create(
    s: &mut RootStore,
) -> Result<swarm_engine::RootMutationOutcome, swarm_engine::RootStoreError> {
    s.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Work","objective":"Data","scope":"Fixture","criteria":["Proof"]}"#,&mut SystemEnvironment)
}
fn timeout(s: &RootStore, event: &str, value: Option<i64>) {
    let d = s.settings().unwrap();
    let mut values = d.values;
    values.git_command_timeout_seconds = Some(SettingCap(value.map(SettingCount)));
    let mut r = EngineSettingsChange {
        schema_version: d.schema_version,
        event_id: event.into(),
        expected_revision: d.revision,
        values,
        reason: "Actual Git metadata deadline".into(),
        preview_token: String::new(),
    };
    r.preview_token = s.preview_settings(&r).unwrap().preview_token;
    s.apply_settings(&r, &mut SystemEnvironment).unwrap();
}

/// All native metadata consumers neutralize repository callbacks and optional
/// index writes. Valid control precedes configured filter injection; ordinary
/// display facts never become execution/acceptance evidence.
#[test]
fn root_legacy_and_mutation_git_reads_disable_external_filters_and_preserve_index() {
    let f = Fixture::new();
    let mut s = RootStore::open(&f.0, true).unwrap();
    let control = s.inspect_git_metadata().unwrap();
    assert!(control.issues.is_empty());
    assert!(!control.state.head.is_empty());
    let marker = f.0.join(".swarm/filter-called");
    assert!(!marker.exists());
    let filter = format!("printf invoked > {}; cat", shell_path(&marker));
    f.git(&["config", "filter.fixture.clean", &filter]);
    f.git(&["config", "filter.fixture.required", "true"]);
    fs::write(f.0.join("probe.txt"), b"modified\n").unwrap();
    let index = fs::read(f.0.join(".git/index")).unwrap();
    let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let current = s.inspect_git_metadata().unwrap();
    assert!(current.external_filters_disabled);
    assert!(current.issues.is_empty());
    assert!(current.state.changes.contains("probe.txt"));
    assert!(!marker.exists());
    let legacy = swarm_engine::observe_git(&f.0).unwrap();
    assert_eq!(legacy.state, current.state);
    assert!(!marker.exists());
    let created = create(&mut s).unwrap();
    assert_eq!(created.result.work.git, current.state);
    assert!(!marker.exists());
    assert_eq!(fs::read(f.0.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        settings
    );
    f.git(&["config", "--unset", "filter.fixture.clean"]);
    f.git(&["config", "--unset", "filter.fixture.required"]);
    let recovered = s.inspect_git_metadata().unwrap();
    assert!(recovered.issues.is_empty());
    assert_eq!(
        recovered.external_filters_disabled,
        control.external_filters_disabled
    );
    assert!(!marker.exists());
    s.close().unwrap();
}

/// Missing repository and unborn HEAD are explicit field diagnostics; no empty
/// value is reported as a verified clean checkpoint, and private output stays redacted.
#[test]
fn unavailable_fields_keep_original_failures_and_recover_after_real_git_initialization() {
    let p =
        std::env::temp_dir().join(swarm_engine::generate_identifier("git-unavailable-").unwrap());
    fs::create_dir(&p).unwrap();
    let s = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
    let missing = s.inspect_git_metadata().unwrap();
    assert_eq!(missing.issues.len(), 3);
    assert_eq!(missing.diagnostics.len(), 3);
    assert!(missing.state.changes.is_empty());
    for issue in &missing.issues {
        assert!(matches!(
            issue.cause,
            swarm_engine::GitReportError::Command { .. }
        ));
        assert!(!format!("{issue:?}").contains(p.to_str().unwrap()));
    }
    s.close().unwrap();
    let f = Fixture(p);
    f.git(&["init"]);
    let s = RootStore::open(&f.0, false).unwrap();
    let unborn = s.inspect_git_metadata().unwrap();
    assert_eq!(unborn.issues.len(), 1);
    assert_eq!(unborn.issues[0].field, swarm_engine::GitMetadataField::Head);
    s.close().unwrap();
    fs::write(f.0.join("proof"), b"proof").unwrap();
    f.git(&["add", "proof"]);
    f.git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@localhost",
        "commit",
        "-m",
        "Fixture",
    ]);
    let s = RootStore::open(&f.0, false).unwrap();
    assert!(s.inspect_git_metadata().unwrap().issues.is_empty());
    s.close().unwrap();
}

/// The real journal deadline reaches metadata and mutation, refuses without
/// publishing a Work, and a saved explicit unlimited policy recovers after reopen.
#[test]
fn saved_git_deadline_refuses_before_work_publication_and_recovers_after_reopen() {
    let f = Fixture::new();
    let s = RootStore::open(&f.0, true).unwrap();
    assert!(s.inspect_git_metadata().unwrap().issues.is_empty());
    timeout(&s, "immediate", Some(0));
    s.close().unwrap();
    let mut s = RootStore::open(&f.0, true).unwrap();
    let before = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let e = s.inspect_git_metadata().unwrap_err();
    assert_eq!(e.code(), "git-metadata");
    assert!(std::error::Error::source(&e).is_some());
    assert!(create(&mut s).is_err());
    assert!(s.list_works().unwrap().is_empty());
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        before
    );
    timeout(&s, "unlimited", None);
    s.close().unwrap();
    let mut s = RootStore::open(&f.0, true).unwrap();
    assert!(s.inspect_git_metadata().unwrap().issues.is_empty());
    let work = create(&mut s).unwrap().result.work;
    assert_eq!(s.list_works().unwrap().len(), 1);
    assert_eq!(s.events(&work.id).unwrap().len(), 1);
    s.close().unwrap();
}

/// Legacy journals authorize no Git metadata execution by implicit fallback.
/// Optional mutation metadata is visibly unavailable; the strict reader refuses.
#[test]
fn legacy_policy_absence_is_explicit_without_invented_deadline_and_adoption_recovers() {
    let f = Fixture::new();
    fs::write(
        f.0.join(".swarm/engine-settings.json"),
        include_bytes!("fixtures/legacy-v1-document.json"),
    )
    .unwrap();
    let mut s = RootStore::open(&f.0, true).unwrap();
    assert!(s.inspect_git_metadata().is_err());
    let outcome = create(&mut s).unwrap();
    assert_eq!(outcome.result.git_issues.len(), 3);
    assert!(outcome.result.git_issues.iter().all(|i| matches!(
        i.cause,
        swarm_engine::GitIssueCause::Policy("settings-git-policy-not-configured")
    )));
    let old = s.settings().unwrap();
    // Captured v8 values keep this historical adoption independent of future defaults.
    let values = swarm_engine::EngineSettingsDocument::decode(include_bytes!(
        "fixtures/legacy-v8-document.json"
    ))
    .unwrap()
    .values;
    let mut adoption = EngineSettingsChange {
        schema_version: 8,
        event_id: "adopt-git".into(),
        expected_revision: old.revision,
        values,
        reason: "Explicit Git policy adoption".into(),
        preview_token: String::new(),
    };
    adoption.preview_token = s.preview_settings(&adoption).unwrap().preview_token;
    s.apply_settings(&adoption, &mut SystemEnvironment).unwrap();
    assert_eq!(
        &s.settings().unwrap().history[..old.history.len()],
        &old.history
    );
    s.close().unwrap();
    let s = RootStore::open(&f.0, false).unwrap();
    assert!(s.inspect_git_metadata().unwrap().issues.is_empty());
    s.close().unwrap();
}
