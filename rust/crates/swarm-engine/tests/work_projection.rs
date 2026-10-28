#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use swarm_engine::{
    EngineSettingsChange, EngineSettingsDocument, EngineSettingsValues, RootStore, SettingCap,
    SettingCount, SystemEnvironment, Work, WorkViewLanguage,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("work-projection-").unwrap());
        fs::create_dir(&p).unwrap();
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
/// Only supported native operations create the Work, task and owned attempt.
fn started(f: &Fixture) -> Work {
    let mut s = RootStore::initialize(&f.0, &mut SystemEnvironment).unwrap();
    let w=s.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Work","objective":"Data","scope":"Fixture","criteria":["Proof"]}"#,&mut SystemEnvironment).unwrap().result.work;
    let w=s.execute_external(&w.id,"task.add",&serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"task","expected_revision":w.revision,"id":"task","title":"Task","deliverable":"report.md","criteria":["Proof"]})).unwrap(),&mut SystemEnvironment).unwrap().result.work;
    let w=s.execute_external(&w.id,"task.update",&serde_json::to_vec(&serde_json::json!({"schema_version":1,"event_id":"start","expected_revision":w.revision,"id":"task","status":"running"})).unwrap(),&mut SystemEnvironment).unwrap().result.work;
    s.close().unwrap();
    w
}
/// Declare receipt data on a closed private fixture, with real FK/UDF projection.
/// This is not an engine execution, actor authentication or acceptance proof.
fn publish(f: &Fixture, w: &mut Work, receipt: &[u8], event: &str) {
    w.revision += 1;
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    swarm_engine::register_identity_plan_projection(&db).unwrap();
    db.execute(
        "UPDATE works SET revision=?1,body=?2 WHERE id=?3",
        rusqlite::params![
            w.revision,
            String::from_utf8(swarm_engine::encode_model(w).unwrap()).unwrap(),
            w.id
        ],
    )
    .unwrap();
    db.execute(
        "INSERT INTO events VALUES(?1,?2,?3,'task.auto-validation','date',?4,?4)",
        rusqlite::params![event, w.id, w.revision, receipt],
    )
    .unwrap();
    db.close().unwrap();
}
/// Apply settings through preview/CAS; no direct mutation of the journal.
fn change(s: &RootStore, event: &str, edit: impl FnOnce(&mut EngineSettingsValues)) {
    let d = s.settings().unwrap();
    let mut values = d.values;
    edit(&mut values);
    let mut r = EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: event.into(),
        expected_revision: d.revision,
        values,
        reason: "Actual projection setting".into(),
        preview_token: String::new(),
    };
    r.preview_token = s.preview_settings(&r).unwrap().preview_token;
    s.apply_settings(&r, &mut SystemEnvironment).unwrap();
}

/// Real v7 producer history/replay survives v8 adoption; partial groups and width
/// zero refuse, null and zero counts remain distinct and preview changes no state.
#[test]
fn legacy_v7_history_replay_and_explicit_v8_presentation_adoption_are_preserved() {
    let old =
        EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v7-document.json")).unwrap();
    let replay: EngineSettingsChange =
        serde_json::from_slice(include_bytes!("fixtures/legacy-v7-request.json")).unwrap();
    // The captured request was preview-only, so it is not a replay event.
    assert!(!old.replay(&replay).unwrap());
    let historical = &old.history.last().unwrap().change;
    assert!(old.replay(historical).unwrap());
    assert!(old.values.work_view_limits().is_err());
    let initial = swarm_engine::encode_model(&old).unwrap();
    let defaults = EngineSettingsValues::installation_defaults().unwrap();
    let mut raw = serde_json::to_value(&defaults).unwrap();
    for field in [
        "evidence_longest_measured_count",
        "work_view_recent_decisions",
        "work_view_wrap_characters",
    ] {
        let saved = raw.as_object_mut().unwrap().remove(field).unwrap();
        assert!(EngineSettingsValues::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
        raw[field] = saved;
    }
    // Captured values prevent future defaults from silently changing this v8 case.
    let defaults =
        EngineSettingsDocument::decode(include_bytes!("fixtures/legacy-v8-document.json"))
            .unwrap()
            .values;
    let mut r = EngineSettingsChange {
        schema_version: 8,
        event_id: "adopt-view".into(),
        expected_revision: old.revision,
        values: defaults,
        reason: "Explicit presentation adoption".into(),
        preview_token: String::new(),
    };
    r.preview_token = old.preview(&r).unwrap().preview_token;
    assert_eq!(swarm_engine::encode_model(&old).unwrap(), initial);
    let upgraded = old
        .apply(&r, "operator".into(), "2026-10-18T00:00:00Z".into())
        .unwrap()
        .0;
    assert_eq!(&upgraded.history[..old.history.len()], &old.history);
    assert!(upgraded.replay(&r).unwrap());
    assert!(upgraded.replay(historical).unwrap());
    let mut downgrade = replay.clone();
    downgrade.expected_revision = upgraded.revision;
    assert_eq!(
        upgraded.preview(&downgrade).unwrap_err().reason(),
        "settings-schema-downgrade"
    );
    let mut zero = upgraded.values.clone();
    zero.work_view_wrap_characters = Some(SettingCap(Some(SettingCount(0))));
    assert!(zero.validate().is_err());
    zero.work_view_wrap_characters = Some(SettingCap(None));
    zero.evidence_longest_measured_count = Some(SettingCap(Some(SettingCount(0))));
    zero.work_view_recent_decisions = Some(SettingCap(None));
    zero.validate().unwrap();
    let limits = zero.work_view_limits().unwrap();
    assert_eq!(limits.wrap_characters, None);
    assert_eq!(limits.longest_measured_count, Some(0));
    assert_eq!(limits.recent_decisions, None);
}

/// Unicode scalar wrapping preserves Go boundaries, controls/newlines and empty
/// lines; selection never prunes stored events or includes a non-OODA event.
#[test]
fn text_wrapping_and_recent_decisions_obey_explicit_zero_null_and_unicode_contract() {
    use swarm_engine::{recent_work_decisions, wrap_work_text, RecordedEvent};
    for (input, width, expected) in [
        ("abc defghij", Some(5), "abc\ndefgh\nij"),
        ("ééé", Some(2), "éé\né"),
        ("a\tb", Some(4), "a   \nb"),
        ("a\r\u{85}\tb\n", None, "a    b\n"),
        ("\n\n", Some(1), "\n\n"),
    ] {
        assert_eq!(wrap_work_text(input, width).unwrap(), expected);
    }
    assert!(wrap_work_text("content", Some(0)).is_err());
    assert_eq!(wrap_work_text("content", None).unwrap(), "content");
    let events: Vec<_> = (0..4)
        .map(|n| RecordedEvent {
            id: n.to_string().into_bytes(),
            work_id: b"work".to_vec(),
            revision: n,
            kind: if n == 1 {
                b"other".to_vec()
            } else {
                b"ooda".to_vec()
            },
            at: vec![],
            payload: None,
        })
        .collect();
    assert_eq!(recent_work_decisions(&events, None).len(), 3);
    assert_eq!(recent_work_decisions(&events, Some(0)).len(), 0);
    assert_eq!(recent_work_decisions(&events, Some(1))[0].revision, 3);
    assert_eq!(events.len(), 4);
}

/// Current/retained measurements keep stable ordering; saved caps affect the
/// actual Root projection after reopening, preserving Work/history/settings on read.
#[test]
fn stored_work_projection_aggregates_failures_and_uses_saved_longest_limit_after_reopen() {
    let f = Fixture::new();
    let mut w = started(&f);
    let task = &mut w.tasks.as_mut().unwrap()[0];
    let attempt = task.attempts.as_ref().unwrap().last().unwrap().id.clone();
    let receipt = serde_json::json!({"attempt_id":attempt,"receipt":"current","revision":w.revision,"controls":[
        {"id":"passed","executed":true,"passed":true,"wall_duration_ms":10},
        {"id":"unknown","wall_duration_ms":50},{"id":"failed","executed":true,"wall_duration_ms":50}]});
    task.automatic_validation = Some(Box::new(serde_json::from_value(receipt.clone()).unwrap()));
    let old=serde_json::to_vec(&serde_json::json!({"attempt_id":attempt,"receipt":"old","controls":[{"id":"history","executed":true,"passed":true,"wall_duration_ms":100}]})).unwrap();
    publish(&f, &mut w, &old, "history");
    publish(
        &f,
        &mut w,
        &serde_json::to_vec(&receipt).unwrap(),
        "current",
    );
    let s = RootStore::open(&f.0, true).unwrap();
    let before = swarm_engine::encode_model(&s.read_work(&w.id).unwrap()).unwrap();
    let count = s.events(&w.id).unwrap().len();
    let v = s
        .inspect_work_validation(&w.id, WorkViewLanguage::French)
        .unwrap()
        .validation;
    assert_eq!(v.state, "open");
    assert_eq!(v.label, "OUVERT");
    assert_eq!(v.tasks["task"].evidence.controls.state, "failed");
    assert_eq!(v.tasks["task"].evidence.acceptance.state, "not_accepted");
    assert!(!v.tasks["task"].fresh);
    let e = &v.tasks["task"].evidence;
    assert_eq!(e.controls.items[1].exit_code, None);
    assert_eq!(e.controls.history.len(), 1);
    assert_eq!(
        e.controls
            .longest_measured
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["history", "unknown", "failed", "passed"]
    );
    change(&s, "one", |values| {
        values.evidence_longest_measured_count = Some(SettingCap(Some(SettingCount(1))))
    });
    s.close().unwrap();
    let s = RootStore::open(&f.0, true).unwrap();
    let saved = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    let v = s
        .inspect_work_validation(&w.id, WorkViewLanguage::English)
        .unwrap()
        .validation;
    assert_eq!(v.label, "OPEN");
    assert_eq!(v.tasks["task"].evidence.controls.longest_measured.len(), 1);
    assert_eq!(s.events(&w.id).unwrap().len(), count);
    assert_eq!(
        swarm_engine::encode_model(&s.read_work(&w.id).unwrap()).unwrap(),
        before
    );
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        saved
    );
    change(&s, "zero", |v| {
        v.evidence_longest_measured_count = Some(SettingCap(Some(SettingCount(0))))
    });
    s.close().unwrap();
    let s = RootStore::open(&f.0, true).unwrap();
    assert!(s
        .inspect_work_validation(&w.id, WorkViewLanguage::French)
        .unwrap()
        .validation
        .tasks["task"]
        .evidence
        .controls
        .longest_measured
        .is_empty());
    change(&s, "all", |v| {
        v.evidence_longest_measured_count = Some(SettingCap(None))
    });
    s.close().unwrap();
    let s = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        s.inspect_work_validation(&w.id, WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["task"]
            .evidence
            .controls
            .longest_measured
            .len(),
        4
    );
    s.close().unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute(
        "UPDATE events SET payload=? WHERE id='history'",
        [b"{private-invalid".as_slice()],
    )
    .unwrap();
    db.close().unwrap();
    let s = RootStore::open(&f.0, false).unwrap();
    let error = s
        .inspect_work_validation(&w.id, WorkViewLanguage::French)
        .err()
        .unwrap();
    assert_eq!(error.code(), "control-history-format");
    assert!(!format!("{error:?}").contains("private-invalid"));
    assert_eq!(
        swarm_engine::encode_model(&s.read_work(&w.id).unwrap()).unwrap(),
        before
    );
    s.close().unwrap();
    let db = rusqlite::Connection::open(f.0.join(".swarm/state.db")).unwrap();
    db.execute("UPDATE events SET payload=? WHERE id='history'", [old])
        .unwrap();
    db.close().unwrap();
    let s = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        s.inspect_work_validation(&w.id, WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["task"]
            .evidence
            .controls
            .history
            .len(),
        1
    );
    s.close().unwrap();
}

/// No receipt/gate means unknown, including empty current results; a stored
/// accepted status without a gate is historical/stale, never validated.
#[test]
fn missing_execution_and_gate_never_become_pass_or_current_acceptance() {
    let f = Fixture::new();
    let mut w = started(&f);
    let s = RootStore::open(&f.0, false).unwrap();
    let v = s
        .inspect_work_validation(&w.id, WorkViewLanguage::French)
        .unwrap()
        .validation;
    let e = &v.tasks["task"].evidence;
    assert_eq!(e.controls.state, "unknown");
    assert_eq!(e.controls.items[0].command, Some(vec![]));
    s.close().unwrap();
    let task = &mut w.tasks.as_mut().unwrap()[0];
    task.status = "accepted".into();
    task.automatic_validation = Some(Default::default());
    let attempt = task.attempts.as_ref().unwrap().last().unwrap().id.clone();
    publish(
        &f,
        &mut w,
        &serde_json::to_vec(
            &serde_json::json!({"attempt_id":attempt,"receipt":"empty","controls":[]}),
        )
        .unwrap(),
        "empty",
    );
    let s = RootStore::open(&f.0, false).unwrap();
    let v = s
        .inspect_work_validation(&w.id, WorkViewLanguage::French)
        .unwrap()
        .validation;
    assert_eq!(v.state, "blocked");
    assert_eq!(v.historical, 1);
    assert_eq!(v.stale, 1);
    assert_eq!(v.validated, 0);
    assert_eq!(v.tasks["task"].evidence.controls.state, "unknown");
    assert_eq!(v.tasks["task"].evidence.acceptance.state, "stale");
    assert!(v.tasks["task"]
        .blockers
        .iter()
        .any(|b| b == "Gate manquante"));
    s.close().unwrap();
}

/// The current receipt guard feeds real TaskEvidence. A saved output-policy
/// mismatch becomes explicit stale evidence, while original claims never grant
/// Work acceptance. Restoring the policy rechecks actual bytes after reopening.
#[test]
fn stored_human_control_freshness_rechecks_saved_policy_and_real_artifacts() {
    use sha2::{Digest, Sha256};
    let f = Fixture::new();
    let mut w = started(&f);
    fs::write(f.0.join("receipt.json"), b"receipt data").unwrap();
    let t = &mut w.tasks.as_mut().unwrap()[0];
    t.validation_policy = Some(Box::new(swarm_engine::ValidationPolicy {
        mode: "human".into(),
        controls: Some(vec![swarm_engine::ValidationControl {
            id: "control".into(),
            command: Some(vec!["cargo".into(), "test".into()]),
            review_output: true,
            ..Default::default()
        }]),
        ..Default::default()
    }));
    let attempt = t.attempts.as_ref().unwrap().last().unwrap().id.clone();
    let raw = serde_json::json!({"attempt_id":attempt,"controller":"controller://swarm-validation","policy_digest":swarm_engine::validation_policy_digest(t.validation_policy.as_deref().unwrap()).unwrap(),"receipt":"receipt.json","state":"pending_human","artifacts":{"receipt.json":format!("{:x}",Sha256::digest(b"receipt data"))},"controls":[{"id":"control","command":["cargo","test"],"executed":true,"passed":true,"exit_code":0,"started_at":"start","finished_at":"finish","review_output":"proof","output_bytes":5}]});
    t.automatic_validation = Some(Box::new(serde_json::from_value(raw.clone()).unwrap()));
    publish(
        &f,
        &mut w,
        &serde_json::to_vec(&raw).unwrap(),
        "receipt-data",
    );
    let s = RootStore::open(&f.0, true).unwrap();
    let v = s
        .inspect_work_validation(&w.id, WorkViewLanguage::French)
        .unwrap()
        .validation;
    assert_eq!(v.tasks["task"].evidence.freshness, "fresh");
    assert_eq!(
        v.tasks["task"].evidence.controls.items[0].freshness,
        "fresh"
    );
    assert!(!v.tasks["task"].fresh);
    assert_eq!(v.validated, 0);
    change(&s, "cap-four", |values| {
        values.validation_max_output_bytes = Some(SettingCap(Some(SettingCount(4))))
    });
    s.close().unwrap();
    let s = RootStore::open(&f.0, true).unwrap();
    let before = swarm_engine::encode_model(&s.read_work(&w.id).unwrap()).unwrap();
    let v = s
        .inspect_work_validation(&w.id, WorkViewLanguage::French)
        .unwrap()
        .validation;
    let e = &v.tasks["task"].evidence;
    assert_eq!(e.freshness, "stale");
    assert_eq!(e.controls.items[0].freshness, "stale");
    assert!(e
        .limits
        .iter()
        .any(|l| l.contains("control-output-incoherent")));
    assert_eq!(e.acceptance.state, "not_accepted");
    assert_eq!(
        swarm_engine::encode_model(&s.read_work(&w.id).unwrap()).unwrap(),
        before
    );
    change(&s, "unlimited-output", |values| {
        values.validation_max_output_bytes = Some(SettingCap(None))
    });
    s.close().unwrap();
    let s = RootStore::open(&f.0, false).unwrap();
    assert_eq!(
        s.inspect_work_validation(&w.id, WorkViewLanguage::French)
            .unwrap()
            .validation
            .tasks["task"]
            .evidence
            .freshness,
        "fresh"
    );
    s.close().unwrap();
}
