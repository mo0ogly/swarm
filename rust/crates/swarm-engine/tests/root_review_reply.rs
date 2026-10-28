#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{
    encode_model, EngineSettingsChange, ReviewQuoteOriginal, ReviewReplyState, RootStore,
    SettingCap, SettingCount, SystemEnvironment,
};

struct Fixture(PathBuf);
impl Fixture {
    /// Allocate an owned private root; no live mission or provider participates.
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(swarm_engine::generate_identifier("reply-root-").unwrap());
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Apply a real preview/CAS policy change, preserving previous history and consumption.
fn policy(
    store: &mut RootStore,
    event: &str,
    change: impl FnOnce(&mut swarm_engine::EngineSettingsValues),
) {
    let current = store.settings().unwrap();
    let mut values = current.values.clone();
    change(&mut values);
    let mut request = EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: event.into(),
        expected_revision: current.revision,
        values,
        reason: "Observe actual reply policy".into(),
        preview_token: String::new(),
    };
    request.preview_token = store.preview_settings(&request).unwrap().preview_token;
    store
        .apply_settings(&request, &mut SystemEnvironment)
        .unwrap();
}
/// Create supported Work/task rows; observe supplied text under saved policies without acceptance or mutation.
#[test]
fn root_reply_parser_uses_actual_task_and_saved_policy_after_reopen_without_mutation() {
    let fixture = Fixture::new();
    let mut store = RootStore::initialize(&fixture.0, &mut SystemEnvironment).unwrap();
    let work=store.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Reply fixture","objective":"Parsing only","scope":"Fixture","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work;
    let task = serde_json::json!({"schema_version":1,"event_id":"task","expected_revision":work.revision,"id":"task","title":"Task","deliverable":"report.md","criteria":["Proof"]});
    let work = store
        .execute_external(
            &work.id,
            "task.add",
            &serde_json::to_vec(&task).unwrap(),
            &mut SystemEnvironment,
        )
        .unwrap()
        .result
        .work;
    let raw=br#"{"reason":"Valid rationale","criteria":[{"index":1,"verdict":"pass","evidence":"Original exact evidence"}]}"#;
    let originals = [ReviewQuoteOriginal {
        name: "proof.md",
        kind: "source",
        content: "Original `exact` evidence",
    }];
    let before = encode_model(&store.read_work(&work.id).unwrap()).unwrap();
    let events = store.events(&work.id).unwrap().len();
    for setting in ["minimum", "maximum", "normalized"] {
        assert_eq!(
            store
                .inspect_review_reply(&work.id, "task", raw, &originals)
                .unwrap()
                .parsed
                .state,
            ReviewReplyState::Passed
        );
        policy(
            &mut store,
            &format!("change-{setting}"),
            |v| match setting {
                "minimum" => v.review_reply_min_text_bytes = Some(SettingCount(16)),
                "maximum" => {
                    v.review_reply_max_text_bytes = Some(SettingCap(Some(SettingCount(20))))
                }
                _ => v.review_quote_min_normalized_bytes = Some(SettingCount(24)),
            },
        );
        store.close().unwrap();
        store = RootStore::open(&fixture.0, true).unwrap();
        let settings = fs::read(fixture.0.join(".swarm/engine-settings.json")).unwrap();
        assert_eq!(
            store
                .inspect_review_reply(&work.id, "task", raw, &originals)
                .err()
                .unwrap()
                .code(),
            "review_fragment_protocol"
        );
        assert_eq!(
            fs::read(fixture.0.join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
        policy(&mut store, &format!("restore-{setting}"), |v| {
            v.review_reply_min_text_bytes = Some(SettingCount(8));
            v.review_reply_max_text_bytes = Some(SettingCap(None));
            v.review_quote_min_normalized_bytes = Some(SettingCount(8));
        });
        store
            .inspect_review_reply(&work.id, "task", raw, &originals)
            .unwrap();
        assert_eq!(
            encode_model(&store.read_work(&work.id).unwrap()).unwrap(),
            before
        );
        assert_eq!(store.events(&work.id).unwrap().len(), events);
    }
    assert_eq!(
        store
            .inspect_review_reply(&work.id, "absent", raw, &originals)
            .err()
            .unwrap()
            .code(),
        "review-reply-task-absent"
    );
    store
        .inspect_review_reply(&work.id, "task", raw, &originals)
        .unwrap();
    store.close().unwrap();
}
