use std::error::Error;
use swarm_engine::{decode_planning_request, encode_planning_request};
/// Complete typed request data and custom dependencies round-trip without an
/// action. Missing/empty dependencies differ only for the revision protocol.
#[test]
fn planning_wire_preserves_complete_fields_and_custom_dependency_intent() {
    let raw=br#"{"schema_version":1,"event_id":"event","expected_revision":3,"scope":"root","scope_revision":2,"holder":"declared","generation":4,"lease_seconds":20,"reason":"Reason","agent_id":"declared-agent","missing_report":true,"checks":{"req-1":[{"id":"check","command":["cargo"],"criteria":[1],"justification":"Data","environment_exit_codes":[2],"review_output":true}]},"repository":{"path":"project","committed_only":false,"include_dirty":["src"]},"artifacts":[{"path":"proof.md","sha256":"hash"}],"operations":[{"kind":"revise-dependencies","id":"task","requirements":[],"depends":[]},{"kind":"other","id":"other","requirements":null,"depends":[]}]}"#;
    let request = decode_planning_request(raw).unwrap();
    assert_eq!(request.event_id, "event");
    assert_eq!(
        request.operations.as_ref().unwrap()[0].depends,
        Some(vec![])
    );
    let encoded = encode_planning_request(&request).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(value["repository"]["committed_only"], false);
    assert_eq!(value["operations"][0]["depends"], serde_json::json!([]));
    assert!(value["operations"][1].get("depends").is_none());
    assert_eq!(
        value["checks"]["req-1"][0]["environment_exit_codes"],
        serde_json::json!([2])
    );
    let copy = decode_planning_request(&encoded).unwrap();
    assert!(copy == request_without_ignored_empty(request));
}
/// Ordinary empty dependencies are omitted by Go's wire contract; expose that
/// expected canonicalization explicitly instead of pretending an identity clone.
fn request_without_ignored_empty(
    mut request: swarm_engine::PlanningRequest,
) -> swarm_engine::PlanningRequest {
    request.operations.as_mut().unwrap()[1].depends = None;
    request
}
/// A valid codec witness precedes strict type/unknown/trailing/UTF8 faults;
/// diagnostics retain a cause without echoing PRIVATE payload contents.
#[test]
fn planning_wire_rejects_invalid_types_unknown_fields_and_recovers() {
    let valid = br#"{"schema_version":1,"event_id":"event","expected_revision":2}"#;
    for bad in [
        br#"{"schema_version":"PRIVATE"}"#.as_slice(),
        br#"{"schema_version":1,"private_preparation":{}}"#,
        br#"{"repository":{"path":"PRIVATE","unknown":true}}"#,
        br#"{"schema_version":1} {}"#,
        b"PRIVATE invalid",
        &[255],
    ] {
        assert!(decode_planning_request(valid).is_ok());
        let error = decode_planning_request(bad).map(|_| ()).unwrap_err();
        assert!(error.source().is_some());
        assert!(!format!("{error:?}").contains("PRIVATE"));
        assert!(decode_planning_request(valid).is_ok());
    }
    let request=decode_planning_request(br#"{"SCOPE":"old","scope":null,"SCHEMA_VERSION":1,"expected_revision":9007199254740993,"scope":"root"}"#).unwrap();
    assert_eq!(request.scope, "root");
    assert_eq!(request.expected_revision, 9007199254740993);
}

/// An isolated data snapshot with two live lease declarations and one unique
/// requirement owner. It establishes neither real agents nor an enabled runtime.
fn controlled_work() -> swarm_engine::Work {
    let work = swarm_engine::decode_work(br#"{"schema_version":1,"id":"work","revision":2,"criteria":["Proof"],"planning":{"version":1,"paused":false,"failure":"Prior failure","max_tasks":10,"max_decisions":10,"max_activations":15,"scopes":[{"id":"root","objective":"Root","requirements":[],"revision":1,"state":"blocked","generation":2,"holder":"root-holder","lease_until":"2099-01-01T00:00:00Z"},{"id":"child","parent":"root","objective":"Child","requirements":["req-1"],"revision":1,"state":"blocked","generation":5,"holder":"child-holder","lease_until":"2099-01-01T00:00:00Z"}]}}"#).unwrap();
    assert!(work
        .planning
        .as_ref()
        .unwrap()
        .scopes
        .iter()
        .flatten()
        .all(|scope| !scope.holder.is_empty() && !scope.lease_until.is_empty()));
    work
}
fn control_request() -> swarm_engine::PlanningRequest {
    decode_planning_request(br#"{"schema_version":1,"event_id":"control","expected_revision":2}"#)
        .unwrap()
}
fn current_settings() -> swarm_engine::EngineSettingsDocument {
    swarm_engine::EngineSettingsDocument::initial(
        swarm_engine::EngineSettingsValues::installation_defaults().unwrap(),
        "fixture".into(),
        "2026-10-19T00:00:00Z".into(),
    )
    .unwrap()
}
/// All leases are fenced, counters/usage and original data stay intact. Passive
/// decision operations cannot run; reason selects a scope reset and inbox message.
#[test]
fn planning_controls_revoke_every_lease_and_preserve_budget_and_input() {
    use swarm_engine::{apply_planning_control as apply, PlanningControlAction as A};
    let work = controlled_work();
    swarm_engine::validate_planning_state(&work).unwrap();
    let settings = current_settings();
    let mut request = control_request();
    request.operations = Some(vec![swarm_engine::PlanningOperation {
        kind: "review-retry".into(),
        id: "not-a-task".into(),
        ..Default::default()
    }]);
    let before = swarm_engine::encode_model(&work).unwrap();
    let paused = apply(
        &work,
        A::Pause,
        &request,
        "2026-10-19T00:00:00.100000Z",
        &settings,
    )
    .unwrap();
    let planning = paused.planning.as_deref().unwrap();
    assert!(planning.paused);
    assert_eq!(planning.failure, "Prior failure");
    assert_eq!(planning.max_activations, 15);
    assert_eq!(planning.activations, 0);
    assert_eq!(planning.scopes.as_ref().unwrap()[0].generation, 3);
    assert_eq!(planning.scopes.as_ref().unwrap()[1].generation, 6);
    assert!(planning
        .scopes
        .iter()
        .flatten()
        .all(|s| s.holder.is_empty() && s.lease_until.is_empty()));
    request.reason = "Operator reason".into();
    request.scope = "child".into();
    let resumed = apply(
        &paused,
        A::Resume,
        &request,
        "2026-10-19T00:00:00.100000Z",
        &settings,
    )
    .unwrap();
    let p = resumed.planning.as_deref().unwrap();
    assert!(!p.paused);
    assert!(p.failure.is_empty());
    assert_eq!(p.scopes.as_ref().unwrap()[0].state, "blocked");
    assert_eq!(p.scopes.as_ref().unwrap()[1].state, "ready");
    let event = &p.inbox.as_ref().unwrap()[0];
    assert_eq!(event.kind, "operator_resume");
    assert_eq!(event.scope, "child");
    assert_eq!(event.message, request.reason);
    assert_eq!(event.at, "2026-10-19T00:00:00.1Z");
    assert_eq!(swarm_engine::encode_model(&work).unwrap(), before);
}
/// Each rejection follows a valid control witness and leaves input bytes intact;
/// numerical bounds here are representation, not a second operator ceiling.
#[test]
fn planning_controls_reject_scope_size_envelope_clock_and_generation_faults() {
    use swarm_engine::{apply_planning_control as apply, PlanningControlAction as A};
    let work = controlled_work();
    let settings = current_settings();
    for fault in [
        "schema",
        "scope-files",
        "review-budget",
        "instruction",
        "scope",
        "reason",
        "clock",
        "negative-generation",
        "overflow",
        "no-planning",
    ] {
        let witness = apply(
            &work,
            A::Pause,
            &control_request(),
            "2026-10-19T00:00:00Z",
            &settings,
        )
        .unwrap();
        assert!(witness.planning.unwrap().paused);
        let mut bad = work.clone();
        let mut request = control_request();
        let mut at = "2026-10-19T00:00:00Z";
        match fault {
            "schema" => request.schema_version = 0,
            "scope-files" => request.scope_files = Some(vec!["PRIVATE".into()]),
            "review-budget" => request.max_review_calls = 1,
            "instruction" => request.instruction_revision_id = "PRIVATE".into(),
            "scope" => {
                request.scope = "PRIVATE".into();
                request.reason = "Reason".into();
            }
            "reason" => request.reason = "é".repeat(2001),
            "clock" => at = "PRIVATE invalid clock",
            "negative-generation" => {
                bad.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].generation = -1
            }
            "overflow" => {
                bad.planning.as_mut().unwrap().scopes.as_mut().unwrap()[0].generation = i64::MAX
            }
            "no-planning" => bad.planning = None,
            _ => unreachable!(),
        }
        let before = swarm_engine::encode_model(&bad).unwrap();
        let error = apply(&bad, A::Resume, &request, at, &settings)
            .map(|_| ())
            .unwrap_err();
        assert!(!format!("{error:?}").contains("PRIVATE"));
        if fault == "clock" {
            assert!(error.source().is_some());
        }
        assert_eq!(swarm_engine::encode_model(&bad).unwrap(), before);
        assert!(apply(
            &work,
            A::Resume,
            &control_request(),
            "2026-10-19T00:00:00Z",
            &settings
        )
        .is_ok());
    }
    let mut boundary = control_request();
    boundary.reason = "é".repeat(2000);
    assert!(apply(
        &work,
        A::Resume,
        &boundary,
        "2026-10-19T00:00:00Z",
        &settings
    )
    .is_ok());
}
/// Real v10 history remains exact; controls not consuming reason policy stay
/// available. Explicit preview/apply adoption enables zero or unlimited, with no
/// fallback4000 on old journals and no changes made by candidate calculation.
#[test]
fn planning_resume_policy_requires_explicit_adoption_only_when_consumed() {
    use swarm_engine::{
        apply_planning_control as apply, EngineSettingsChange, EngineSettingsDocument,
        PlanningControlAction as A, SettingCap, SettingCount,
    };
    let mut doc =
        EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v10-document.json"))
            .unwrap();
    let historical = doc.history.clone();
    assert!(doc.replay(&doc.history.last().unwrap().change).unwrap());
    let work = controlled_work();
    let mut request = control_request();
    assert!(apply(&work, A::Pause, &request, "2026-10-19T00:00:00Z", &doc).is_ok());
    assert!(apply(&work, A::Resume, &request, "2026-10-19T00:00:00Z", &doc).is_ok());
    request.reason = "Nonblank".into();
    assert!(
        apply(&work, A::Resume, &request, "2026-10-19T00:00:00Z", &doc)
            .map(|_| ())
            .unwrap_err()
            .to_string()
            .contains("settings-planning-resume-policy-not-configured")
    );
    for (event, cap, reason_ok) in [
        ("zero", Some(SettingCount(0)), false),
        ("unlimited", None, true),
    ] {
        let before = swarm_engine::encode_model(&doc).unwrap();
        let mut values = doc.values.clone();
        values.planning_resume_max_reason_bytes = Some(SettingCap(cap));
        let mut change = EngineSettingsChange {
            schema_version: 11,
            event_id: event.into(),
            expected_revision: doc.revision,
            values,
            reason: "Explicit resume policy".into(),
            preview_token: String::new(),
        };
        change.preview_token = doc.preview(&change).unwrap().preview_token;
        assert_eq!(swarm_engine::encode_model(&doc).unwrap(), before);
        doc = doc
            .apply(&change, "fixture".into(), "2026-10-19T00:00:00Z".into())
            .unwrap()
            .0;
        assert_eq!(&doc.history[..historical.len()], historical.as_slice());
        assert_eq!(doc.schema_version, 11);
        request.reason = "x".repeat(5000);
        let before = swarm_engine::encode_model(&doc).unwrap();
        assert_eq!(
            apply(&work, A::Resume, &request, "2026-10-19T00:00:00Z", &doc).is_ok(),
            reason_ok
        );
        assert_eq!(swarm_engine::encode_model(&doc).unwrap(), before);
        request.reason = " \u{0085} ".into();
        assert!(
            apply(&work, A::Resume, &request, "2026-10-19T00:00:00Z", &doc)
                .unwrap()
                .planning
                .unwrap()
                .inbox
                .is_none()
        );
    }
}

/// Own a native fixture root; failure preserves it for diagnosis, never a mission.
#[cfg(unix)]
struct ControlRoot(std::path::PathBuf);
#[cfg(unix)]
impl ControlRoot {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("planning-control61-").unwrap());
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    /// Seed only this owned initialized fixture, with actual schema/UDFs enabled.
    /// No Store/mission row is edited outside the private test root.
    fn seed(&self, work: &swarm_engine::Work) {
        let db = rusqlite::Connection::open_with_flags(
            self.0.join(".swarm/state.db"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
        )
        .unwrap();
        db.busy_timeout(std::time::Duration::ZERO).unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        swarm_engine::register_identity_plan_projection(&db).unwrap();
        assert_eq!(
            db.execute(
                "INSERT INTO works(id,revision,body) VALUES(?1,?2,?3)",
                rusqlite::params![
                    work.id,
                    work.revision,
                    String::from_utf8(swarm_engine::encode_model(work).unwrap()).unwrap()
                ]
            )
            .unwrap(),
            1
        );
        db.close().unwrap();
    }
}
#[cfg(unix)]
impl Drop for ControlRoot {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Actual Root settings/reservation/transaction path fences both scopes once;
/// same event replays before new clock or policy edits, conflicts/revisions refuse
/// without event/state mutation. No fixture agent is run or authority fabricated.
#[cfg(unix)]
#[test]
fn root_planning_controls_commit_replay_and_refuse_atomically() {
    use swarm_engine::{RootStore, SystemEnvironment};
    let f = ControlRoot::new();
    RootStore::initialize(&f.0, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    let work = controlled_work();
    f.seed(&work);
    let mut store = RootStore::open(&f.0, true).unwrap();
    let request = control_request();
    let raw = encode_planning_request(&request).unwrap();
    let paused = store
        .execute_external(&work.id, "planning.pause", &raw, &mut SystemEnvironment)
        .unwrap();
    assert!(!paused.result.replayed);
    assert_eq!(paused.result.work.revision, work.revision + 1);
    let p = paused.result.work.planning.as_ref().unwrap();
    assert!(p.paused);
    assert_eq!(p.scopes.as_ref().unwrap()[0].generation, 3);
    assert!(p
        .scopes
        .iter()
        .flatten()
        .all(|s| s.holder.is_empty() && s.lease_until.is_empty()));
    assert_eq!(store.events(&work.id).unwrap().len(), 1);
    let before = std::fs::read(f.0.join(".swarm/state.db")).unwrap();
    let replay = store
        .execute_external(&work.id, "planning.pause", &raw, &mut SystemEnvironment)
        .unwrap();
    assert!(replay.result.replayed);
    assert_eq!(replay.result.work.revision, paused.result.work.revision);
    assert_eq!(std::fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    let mut conflict = request.clone();
    conflict.reason = "Different metadata".into();
    assert!(store
        .execute_external(
            &work.id,
            "planning.pause",
            &encode_planning_request(&conflict).unwrap(),
            &mut SystemEnvironment
        )
        .is_err());
    assert_eq!(std::fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    let mut stale = request.clone();
    stale.event_id = "stale".into();
    assert!(store
        .execute_external(
            &work.id,
            "planning.resume",
            &encode_planning_request(&stale).unwrap(),
            &mut SystemEnvironment
        )
        .is_err());
    assert_eq!(std::fs::read(f.0.join(".swarm/state.db")).unwrap(), before);
    let mut resumed = request;
    resumed.event_id = "resume".into();
    resumed.expected_revision = paused.result.work.revision;
    resumed.reason = "Operator resumed".into();
    let result = store
        .execute_external(
            &work.id,
            "planning.resume",
            &encode_planning_request(&resumed).unwrap(),
            &mut SystemEnvironment,
        )
        .unwrap();
    assert!(!result.result.work.planning.as_ref().unwrap().paused);
    assert_eq!(
        result
            .result
            .work
            .planning
            .as_ref()
            .unwrap()
            .scopes
            .as_ref()
            .unwrap()[0]
            .generation,
        4
    );
    assert_eq!(store.events(&work.id).unwrap().len(), 2);
    store.close().unwrap();
}
