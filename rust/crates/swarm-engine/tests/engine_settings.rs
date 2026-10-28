use swarm_engine::{
    decode_request, decode_work, update_task_definition, DefinitionContext, EngineSettingsChange,
    EngineSettingsDocument, EngineSettingsValues, SettingCount,
};
fn doc() -> EngineSettingsDocument {
    EngineSettingsDocument::initial(
        EngineSettingsValues::installation_defaults().unwrap(),
        "fixture/uid:1".into(),
        "2026-10-18T00:00:00Z".into(),
    )
    .unwrap()
}
fn change(d: &EngineSettingsDocument) -> EngineSettingsChange {
    let mut values = d.values.clone();
    values.task_max_attempts = Some(SettingCount(5));
    EngineSettingsChange {
        schema_version: d.schema_version,
        event_id: "settings-change".into(),
        expected_revision: d.revision,
        values,
        reason: "Operator requested wider definition cap".into(),
        preview_token: String::new(),
    }
}
#[test]
fn decimal_counts_remain_exact_and_optional_caps_require_explicit_presence() {
    let largest: SettingCount = serde_json::from_str("\"9223372036854775807\"").unwrap();
    assert_eq!(largest.0, i64::MAX);
    for bad in [
        "3",
        "null",
        "\"-1\"",
        "\"01\"",
        "\"+1\"",
        "\" 1\"",
        "\"9223372036854775808\"",
    ] {
        assert!(
            serde_json::from_str::<SettingCount>(bad).is_err(),
            "accepted {bad}"
        );
    }
    let values = EngineSettingsValues::installation_defaults().unwrap();
    let mut raw = serde_json::to_value(&values).unwrap();
    raw.as_object_mut().unwrap().remove("task_max_attempts");
    assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
    raw["task_max_attempts"] = serde_json::Value::Null;
    assert!(
        EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap())
            .unwrap()
            .task_max_attempts
            .is_none()
    );
}
#[test]
fn preview_is_pure_apply_requires_exact_token_and_replay_does_not_add_history() {
    let d = doc();
    let before = serde_json::to_vec(&d).unwrap();
    let mut r = change(&d);
    let preview = d.preview(&r).unwrap();
    assert_eq!(serde_json::to_vec(&d).unwrap(), before);
    assert!(d
        .apply(&r, "operator".into(), "2026-10-18T00:00:01Z".into())
        .is_err());
    r.preview_token = preview.preview_token;
    let (next, replayed) = d
        .apply(&r, "operator".into(), "2026-10-18T00:00:01Z".into())
        .unwrap();
    assert!(!replayed);
    assert_eq!(next.revision, SettingCount(2));
    assert_eq!(next.history.len(), 2);
    next.validate().unwrap();
    let (again, replayed) = next
        .apply(
            &r,
            "different-observation".into(),
            "2026-10-18T00:00:02Z".into(),
        )
        .unwrap();
    assert!(replayed);
    assert_eq!(again, next);
    let mut different = r.clone();
    different.reason = "Different operator intent".into();
    assert!(next
        .apply(&different, "operator".into(), "2026-10-18T00:00:03Z".into())
        .is_err());
    let mut stale = r.clone();
    stale.event_id = "new-change".into();
    assert!(next.preview(&stale).is_err());
}
#[test]
fn history_tampering_and_invalid_defaults_fail_without_silent_fallback() {
    let d = doc();
    for mutation in 0..4 {
        let mut bad = d.clone();
        match mutation {
            0 => bad.values.task_max_attempts = Some(SettingCount(8)),
            1 => bad.history[0].actor = "changed".into(),
            2 => bad.history[0].previous_digest = "changed".into(),
            _ => bad.history.clear(),
        };
        assert!(bad.validate().is_err());
    }
    let mut bad = d.values.clone();
    bad.storage_busy_timeout_ms = SettingCount(i64::from(i32::MAX) + 1);
    assert!(bad.validate().is_err());
    bad = d.values.clone();
    bad.validation_default_timeout_seconds = SettingCount(301);
    assert!(bad.validate().is_err());
    bad = d.values.clone();
    bad.task_max_attempts = Some(SettingCount(0));
    assert!(bad.validate().is_err());
    bad = d.values.clone();
    bad.validation_allowed_programs.insert("../outside".into());
    assert!(bad.validate().is_err());
}
#[test]
fn effective_values_change_the_actual_definition_consumer_and_unlimited_is_explicit() {
    let initial = doc();
    let work=decode_work(br#"{"schema_version":1,"id":"work","revision":1,"tasks":[{"id":"task","status":"todo","criteria":["proof"]}]}"#).unwrap();
    let r = decode_request(br#"{"schema_version":1,"id":"task","max_attempts":4}"#).unwrap();
    let limits = initial.values.mutation_limits().unwrap();
    let context = DefinitionContext {
        authorized_at: "",
        actor: "",
        task_limits: limits.task,
        policy_limits: limits.policy,
    };
    assert!(update_task_definition(&work, &work.tasks.as_ref().unwrap()[0], &r, &context).is_err());
    let mut request = change(&initial);
    request.preview_token = initial.preview(&request).unwrap().preview_token;
    let (next, _) = initial
        .apply(&request, "operator".into(), "2026-10-18T00:00:01Z".into())
        .unwrap();
    let limits = next.values.mutation_limits().unwrap();
    let context = DefinitionContext {
        authorized_at: "",
        actor: "",
        task_limits: limits.task,
        policy_limits: limits.policy,
    };
    assert_eq!(
        update_task_definition(&work, &work.tasks.as_ref().unwrap()[0], &r, &context)
            .unwrap()
            .plan_max_attempts,
        4
    );
    let mut unlimited = next.values.clone();
    unlimited.task_max_attempts = None;
    let limits = unlimited.mutation_limits().unwrap();
    assert_eq!(limits.task.max_attempts, i64::MAX);
    assert_eq!(
        serde_json::to_value(&unlimited).unwrap()["task_max_attempts"],
        serde_json::Value::Null
    );
}

/// Explicit v39 status windows preserve legacy absence, null/unlimited and zero.
/// Invalid wire and partial groups refuse without altering the valid control.
#[test]
fn status_view_windows_validate_absence_null_zero_and_recovery() {
    use swarm_engine::SettingCap;
    let values=EngineSettingsValues::installation_defaults().unwrap();
    assert_eq!(values.schema_version(),39);
    assert_eq!(values.mission_status_view_limits().unwrap(),(Some(200),Some(200)));
    let original=swarm_wire_json::encode(&values).unwrap();
    let mut legacy=values.clone();
    legacy.work_view_status_recent_agents=None;
    assert!(legacy.validate().is_err());
    legacy.work_view_status_change_events=None;
    legacy.validate().unwrap();
    assert_eq!(legacy.schema_version(),38);
    assert_eq!(legacy.mission_status_view_limits().unwrap_err().reason(),"settings-status-view-not-configured");
    let mut explicit=values.clone();
    explicit.work_view_status_recent_agents=Some(SettingCap(None));
    explicit.work_view_status_change_events=Some(SettingCap(Some(SettingCount(0))));
    explicit.validate().unwrap();
    assert_eq!(explicit.mission_status_view_limits().unwrap(),(None,Some(0)));
    let reopened=EngineSettingsValues::decode(&swarm_wire_json::encode(&explicit).unwrap()).unwrap();
    assert_eq!(reopened,explicit);
    for invalid in [serde_json::json!("-1"),serde_json::json!("01"),serde_json::json!(1),serde_json::json!("2.5")] {
        let mut raw=serde_json::to_value(&values).unwrap();raw["work_view_status_recent_agents"]=invalid;
        assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
        assert_eq!(swarm_wire_json::encode(&values).unwrap(),original);
    }
    explicit.work_view_status_recent_agents=Some(SettingCap(Some(SettingCount(-1))));
    assert!(explicit.validate().is_err());
    explicit.work_view_status_recent_agents=Some(SettingCap(None));
    explicit.validate().unwrap();
}

/// Pure preview and explicit v38→v39 adoption preserve old journal entries;
/// reopen/replay and stale publication refusal retain the operator's choice.
#[test]
fn status_view_windows_upgrade_save_reopen_and_stale_refusal() {
    let current=EngineSettingsValues::installation_defaults().unwrap();
    let mut legacy=current.clone();legacy.work_view_status_recent_agents=None;legacy.work_view_status_change_events=None;
    let old=EngineSettingsDocument::initial(legacy,"fixture/uid:1".into(),"2026-10-23T00:00:00Z".into()).unwrap();
    let bytes=swarm_wire_json::encode(&old).unwrap();
    let mut proposal=EngineSettingsChange {schema_version:39,event_id:"status-window-adoption".into(),expected_revision:old.revision,values:current,reason:"Explicit presentation windows".into(),preview_token:String::new()};
    let preview=old.preview(&proposal).unwrap();
    assert_eq!(swarm_wire_json::encode(&old).unwrap(),bytes);
    proposal.preview_token=preview.preview_token;
    let (saved,replay)=old.apply(&proposal,"fixture/uid:1".into(),"2026-10-23T00:00:01Z".into()).unwrap();
    assert!(!replay);assert_eq!(&saved.history[..old.history.len()],&old.history);
    let reopened=EngineSettingsDocument::decode(&swarm_wire_json::encode(&saved).unwrap()).unwrap();
    assert_eq!(reopened,saved);
    assert_eq!(reopened.values.mission_status_view_limits().unwrap(),(Some(200),Some(200)));
    assert!(reopened.replay(&proposal).unwrap());
    proposal.event_id="stale-status-window".into();
    assert!(reopened.preview(&proposal).is_err());
    assert_eq!(reopened,saved);
}
