#![cfg(target_os = "linux")]
//! Real native CLI dispatcher/Root with explicit failing output doubles; no OS fault or live project.
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCount, SystemEnvironment};
/// Own and initialize a real private Root, retained when a test fails.
fn fixture() -> PathBuf {
    let p =
        std::env::temp_dir().join(swarm_engine::generate_identifier("settings-output92-").unwrap());
    fs::create_dir(&p).unwrap();
    RootStore::initialize(&p, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    p
}
/// Fresh current-revision preview is obtained through the public Root API and saved as test input.
fn proposal(root: &Path, event: &str, value: i64) -> PathBuf {
    let s = RootStore::open(root, true).unwrap();
    let doc = s.settings().unwrap();
    let mut values = doc.values.clone();
    values.task_max_attempts = Some(SettingCount(value));
    let mut request = EngineSettingsChange {
        schema_version: doc.schema_version,
        event_id: event.into(),
        expected_revision: doc.revision,
        values,
        reason: "Output fault witness".into(),
        preview_token: String::new(),
    };
    request.preview_token = s.preview_settings(&request).unwrap().preview_token;
    s.close().unwrap();
    let p = root.join(format!("{event}.json"));
    fs::write(&p, serde_json::to_vec(&request).unwrap()).unwrap();
    p
}
/// Actual public CLI argv, not a reimplementation of apply or output handling.
fn args(root: &Path, input: &Path, lang: &str, json: bool) -> Vec<String> {
    let mut a = vec![
        "engine-settings".into(),
        "apply".into(),
        "--root".into(),
        root.to_str().unwrap().into(),
        "--input".into(),
        input.to_str().unwrap().into(),
        "--lang".into(),
        lang.into(),
    ];
    if json {
        a.push("--json".into());
    }
    a
}
/// Observe saved revision/history/values through a separately reopened public store.
fn document(root: &Path) -> swarm_engine::EngineSettingsDocument {
    let s = RootStore::open(root, false).unwrap();
    let d = s.settings().unwrap();
    assert!(s.list_works().unwrap().is_empty());
    s.close().unwrap();
    d
}
struct FailingOutput {
    flush: bool,
    bytes: Vec<u8>,
}
impl Write for FailingOutput {
    /// Inject only one output boundary; flush variant accepts complete bytes first.
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.flush {
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fixture output write",
            ))
        }
    }
    /// Explicit flush failure, independent of successful settings publication/write.
    fn flush(&mut self) -> io::Result<()> {
        if self.flush {
            Err(io::Error::other("fixture output flush"))
        } else {
            Ok(())
        }
    }
}
/// Successful witness precedes every write/flush fault. A persisted apply must
/// warn in FR/EN and JSON/text, preserve its original cause and replay once only.
#[test]
fn settings_apply_output_failure_retains_recorded_state_and_replay_in_all_diagnostics() {
    for lang in ["fr", "en"] {
        for json in [false, true] {
            for flush in [false, true] {
                let root = fixture();
                let first = proposal(&root, "witness", 76);
                let mut out = Vec::new();
                let mut err = Vec::new();
                let witness = swarm_cli::run_detailed(
                    &args(&root, &first, lang, json),
                    "fr",
                    &mut out,
                    &mut err,
                );
                assert_eq!(witness.code, 0);
                assert!(err.is_empty());
                assert!(!out.is_empty());
                let before = document(&root);
                let input = proposal(&root, "failure", 77);
                let mut sink = FailingOutput {
                    flush,
                    bytes: vec![],
                };
                let result = swarm_cli::run_detailed(
                    &args(&root, &input, lang, json),
                    "fr",
                    &mut sink,
                    &mut err,
                );
                assert_eq!(result.code, 2);
                assert!(result.error.is_some());
                assert!(result.diagnostic_error.is_none());
                let after = document(&root);
                assert_eq!(after.revision.0, before.revision.0 + 1);
                assert_eq!(after.values.task_max_attempts, Some(SettingCount(77)));
                let persisted = fs::read(root.join(".swarm/engine-settings.json")).unwrap();
                let raw = String::from_utf8(err.clone()).unwrap();
                let message = if json {
                    let v: serde_json::Value = serde_json::from_slice(&err).unwrap();
                    assert_eq!(v["failure"]["retryable"], false);
                    assert!(v["failure"]["causes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|x| x.as_str().is_some_and(|s| s.starts_with("io-kind:"))));
                    v["failure"]["message"].as_str().unwrap().to_owned()
                } else {
                    raw
                };
                assert!(
                    message.contains(if lang == "en" {
                        "Configuration is recorded"
                    } else {
                        "Configuration enregistrée"
                    }),
                    "missing state marker: lang={lang} json={json} flush={flush} message={message}"
                );
                assert_eq!(sink.bytes.is_empty(), !flush);
                out.clear();
                err.clear();
                let replay = swarm_cli::run_detailed(
                    &args(&root, &input, lang, json),
                    "fr",
                    &mut out,
                    &mut err,
                );
                assert_eq!(replay.code, 0);
                assert!(err.is_empty());
                assert_eq!(
                    fs::read(root.join(".swarm/engine-settings.json")).unwrap(),
                    persisted
                );
                assert_eq!(document(&root).revision, after.revision);
                let mut replay_sink = FailingOutput {
                    flush,
                    bytes: vec![],
                };
                err.clear();
                let replay_failure = swarm_cli::run_detailed(
                    &args(&root, &input, lang, json),
                    "fr",
                    &mut replay_sink,
                    &mut err,
                );
                assert_eq!(replay_failure.code, 2);
                assert!(String::from_utf8(err).unwrap().contains(if lang == "en" {
                    "Configuration is recorded"
                } else {
                    "Configuration enregistrée"
                }));
                assert_eq!(
                    fs::read(root.join(".swarm/engine-settings.json")).unwrap(),
                    persisted
                );
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
}

/// Public initialization argv; keeps the same diagnostic modes as later applies.
fn init_args(root: &Path, lang: &str, json: bool) -> Vec<String> {
    let mut a = vec![
        "init".into(),
        "--root".into(),
        root.to_str().unwrap().into(),
        "--lang".into(),
        lang.into(),
    ];
    if json {
        a.push("--json".into());
    }
    a
}
/// Successful initialization must also retain its known created configuration
/// when stdout subsequently fails; it does not claim rollback or a running server.
#[test]
fn initialized_configuration_survives_output_write_and_flush_failure() {
    for lang in ["fr", "en"] {
        for json in [false, true] {
            for flush in [false, true] {
                let root = std::env::temp_dir()
                    .join(swarm_engine::generate_identifier("init-output92-").unwrap());
                fs::create_dir(&root).unwrap();
                let witness = root.join("witness");
                fs::create_dir(&witness).unwrap();
                let mut out = vec![];
                let mut err = vec![];
                assert_eq!(
                    swarm_cli::run_detailed(
                        &init_args(&witness, lang, json),
                        "fr",
                        &mut out,
                        &mut err
                    )
                    .code,
                    0
                );
                assert!(err.is_empty());
                assert!(!out.is_empty());
                let target = root.join("target");
                fs::create_dir(&target).unwrap();
                let mut sink = FailingOutput {
                    flush,
                    bytes: vec![],
                };
                let result = swarm_cli::run_detailed(
                    &init_args(&target, lang, json),
                    "fr",
                    &mut sink,
                    &mut err,
                );
                assert_eq!(result.code, 2);
                assert_eq!(document(&target).revision, SettingCount(1));
                let message = String::from_utf8(err.clone()).unwrap();
                assert!(
                    message.contains(if lang == "en" {
                        "Configuration is recorded"
                    } else {
                        "Configuration enregistrée"
                    }),
                    "missing initialization state marker: {message}"
                );
                let before = fs::read(target.join(".swarm/engine-settings.json")).unwrap();
                out.clear();
                err.clear();
                assert_eq!(
                    swarm_cli::run_detailed(
                        &init_args(&target, lang, json),
                        "fr",
                        &mut out,
                        &mut err
                    )
                    .code,
                    0
                );
                assert_eq!(
                    fs::read(target.join(".swarm/engine-settings.json")).unwrap(),
                    before
                );
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
}

/// Read-only output failures and invalid applies do not claim a new saved state.
/// Every injection has a successful same-operation witness and a recovery check.
#[test]
fn readonly_and_refused_settings_operations_do_not_claim_publication() {
    for lang in ["fr", "en"] {
        for json in [false, true] {
            for flush in [false, true] {
                let root = fixture();
                for operation in ["show", "preview", "apply"] {
                    let input = proposal(&root, &format!("negative-{operation}"), 79);
                    let mut a = args(&root, &input, lang, json);
                    a[1] = operation.into();
                    if operation == "show" {
                        let index = a.iter().position(|x| x == "--input").unwrap();
                        a.drain(index..index + 2);
                    }
                    let mut out = vec![];
                    let mut err = vec![];
                    if operation == "apply" {
                        let mut preview = a.clone();
                        preview[1] = "preview".into();
                        assert_eq!(
                            swarm_cli::run_detailed(&preview, "fr", &mut out, &mut err).code,
                            0
                        );
                    } else {
                        assert_eq!(
                            swarm_cli::run_detailed(&a, "fr", &mut out, &mut err).code,
                            0
                        );
                    }
                    assert!(err.is_empty());
                    assert!(!out.is_empty());
                    let before = fs::read(root.join(".swarm/engine-settings.json")).unwrap();
                    let original = fs::read(&input).unwrap();
                    if operation == "apply" {
                        let mut value: serde_json::Value =
                            serde_json::from_slice(&original).unwrap();
                        value["preview_token"] = "".into();
                        fs::write(&input, serde_json::to_vec(&value).unwrap()).unwrap();
                    }
                    let mut sink = FailingOutput {
                        flush,
                        bytes: vec![],
                    };
                    let r = swarm_cli::run_detailed(&a, "fr", &mut sink, &mut err);
                    assert_eq!(r.code, 2);
                    assert!(r.diagnostic_error.is_none());
                    let message = String::from_utf8(err.clone()).unwrap();
                    assert!(
                        !message.contains("Configuration is recorded")
                            && !message.contains("Configuration enregistrée"),
                        "false publication: {operation} {message}"
                    );
                    assert_eq!(
                        fs::read(root.join(".swarm/engine-settings.json")).unwrap(),
                        before
                    );
                    fs::write(&input, &original).unwrap();
                    out.clear();
                    err.clear();
                    assert_eq!(
                        swarm_cli::run_detailed(&a, "fr", &mut out, &mut err).code,
                        0
                    );
                    assert!(err.is_empty());
                    if operation != "apply" {
                        assert_eq!(
                            fs::read(root.join(".swarm/engine-settings.json")).unwrap(),
                            before
                        );
                    }
                    assert!(document(&root).revision.0 >= 1);
                }
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
}
