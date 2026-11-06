//! BF-16a reproduction: the public preparation dialogue returns an admitted
//! turn's classified answer with no reader control at all. BF-16b extends the
//! same reader verdict to the show and history presentation frontiers. No
//! provider is contacted; fixtures live under the repository fixture area
//! (never /tmp).
#![cfg(target_os = "linux")]
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::PathBuf,
};
use swarm_engine::{EngineSettingsChange, EngineSettingsValues, Preparation, RootStore, SettingCount, SupervisorIdentity, SystemEnvironment};

struct Fixture {
    root: RootStore,
    path: PathBuf,
    db: Connection,
    preparation: Preparation,
}

impl Fixture {
    /// Initialize a real Root in the repository fixture area, enable the
    /// preparation call, publish an owned inert provider pair and create one
    /// preparation with an explicit finite budget envelope.
    fn new(label: &str) -> Self {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.swarm-fixtures");
        std::fs::create_dir_all(&base).unwrap();
        let path = base.join(format!(
            "blp-dialogue-reader-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let mut marker = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path.join("fixture-birth.json"))
            .unwrap();
        marker.write_all(
            serde_json::to_vec(&json!({"root":path,"owner":"blp-dialogue-reader-integration","fixture":true,"live":false}))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        marker.sync_all().unwrap();
        drop(marker);
        let root = RootStore::initialize(&path, &mut SystemEnvironment).unwrap();
        settings(&root, |s| {
            s.preparation_write_enabled = Some(SettingCount(1));
            s.preparation_call_enabled = Some(SettingCount(1));
        });
        fs::write(
            path.join(".swarm/providers.json"),
            br#"{"schema_version":1,"providers":{}}"#,
        )
        .unwrap();
        fs::write(
            path.join(".swarm/ai-connections.json"),
            br#"{"local":{"id":"local","label":"Owned fixture","base_url":"https://example.invalid/v1","model":"fixed-model","key":"PRIVATE_FIXTURE_KEY","disabled":false}}"#,
        )
        .unwrap();
        for file in ["providers.json", "ai-connections.json"] {
            std::fs::set_permissions(
                path.join(".swarm").join(file),
                std::fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let p = root
            .preparation_command(
                br#"{"version":1,"action":"create","event_id":"create","expected_revision":0,"text":"Fixture need"}"#,
                &mut SystemEnvironment,
            )
            .unwrap()
            .value;
        let preparation = root
            .preparation_command(
                &serde_json::to_vec(&json!({
                    "version":1,"action":"budget","event_id":"budget","preparation_id":p.id,
                    "expected_revision":p.revision,
                    "budget":{"limit_usd":10,"reserve_per_launch_usd":0.5,"estimate_source":"Owned fixture","reference_date":"2026-10-27"}
                }))
                .unwrap(),
                &mut SystemEnvironment,
            )
            .unwrap()
            .value;
        let db = Connection::open(path.join(".swarm/state.db")).unwrap();
        Self { root, path, db, preparation }
    }

    /// Bind a fresh explicit send intent to the current saved capability.
    fn request(&self, event: &str) -> Value {
        let cap = self.root.preparation_call_capabilities().unwrap();
        let provider = cap.providers.iter().find(|p| p.name == "api-local").unwrap();
        json!({"version":1,"preparation_id":self.preparation.id,"expected_revision":self.preparation.revision,
            "event_id":event,"provider":"api-local","capability_hash":provider.digest,"message":"Owned fixture"})
    }

    /// Register this test's actual kernel identity in synthetic private data.
    /// This is an explicit fixture injection, not production enrollment evidence.
    fn register(&self, id: &str) -> String {
        let raw: Vec<u8> = self
            .db
            .query_row(
                "SELECT CAST(body AS BLOB) FROM preparation_turns WHERE id=?",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        let mut v: Value = serde_json::from_slice(&raw).unwrap();
        let own = SupervisorIdentity::current().unwrap();
        v["supervisor_pid"] = own.pid().into();
        v["supervisor_stamp"] = own.process_stamp().into();
        v["host"] = own.host().into();
        let token = format!(
            "{:x}",
            Sha256::digest(v["_native"]["nonce"].as_str().unwrap().as_bytes())
        );
        self.db
            .execute(
                "UPDATE preparation_turns SET body=? WHERE id=?",
                params![serde_json::to_vec(&v).unwrap(), id],
            )
            .unwrap();
        token
    }

    /// Close the Root while retaining the attributed fixture for later proof.
    fn close(self) {
        self.root.close().unwrap();
    }
}

/// Apply a complete explicit settings change through preview and CAS.
fn settings(root: &RootStore, change: impl FnOnce(&mut EngineSettingsValues)) {
    let doc = root.settings().unwrap();
    let mut values = doc.values;
    change(&mut values);
    let mut r = EngineSettingsChange {
        schema_version: values.schema_version(),
        event_id: format!("settings-{}", doc.revision.0),
        expected_revision: doc.revision,
        values,
        reason: "Owned inert dialogue-reader fixture".into(),
        preview_token: String::new(),
    };
    r.preview_token = root.preview_settings(&r).unwrap().preview_token;
    root.apply_settings(&r, &mut SystemEnvironment).unwrap();
}

fn call(root: &RootStore, op: &str, body: Value, token: &str) -> Result<Value, swarm_engine::RootStoreError> {
    let raw = serde_json::to_vec(&body).unwrap();
    match root.trust_as_principal(op, &raw, token) {
        Ok(r) => Ok(serde_json::from_slice(&r.bytes).unwrap()),
        Err(e) => { eprintln!("CALL {op} REFUSED: {e}"); Err(e) }
    }
}

/// Flatten an error chain to a searchable string of typed codes.
fn codes(error: swarm_engine::RootStoreError) -> String {
    let mut out = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(e) = source {
        out.push_str(&format!(":{e}"));
        source = e.source();
    }
    out
}

/// Extract the typed error from a read observation without requiring Debug on
/// the success payload; a success here is a contract failure, not a panic.
fn refused(read: Result<swarm_engine::PreparationReadObservation<Vec<swarm_engine::PreparationTurn>>, swarm_engine::RootStoreError>) -> swarm_engine::RootStoreError {
    match read {
        Err(e) => e,
        Ok(_) => panic!("classified dialogue was served where the BF-16a contract requires a refusal"),
    }
}

/// Bootstrap the shared fixture trust state: three identities (the reader
/// deliberately holds NO clearance), one explicit BLP policy (no quarantine)
/// and every source attributed for the prepared intent, then admission.
fn bootstrap(f: &Fixture) -> Value {
    call(&f.root, "config-apply", json!({"schema_version":1,"event_id":"init","expected_revision":0,"reason":"owned fixture",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
        "identities":[{"subject":"admin","integrity":3,"roles":["admin"],"token":"fixture-admin-token"},
        {"subject":"sender","integrity":1,"roles":["requester"],"token":"fixture-sender-token"},
        {"subject":"reader","integrity":1,"roles":["requester"],"token":"fixture-reader-token"}]}), "").unwrap();
    call(&f.root, "blp-policy-apply", json!({"schema_version":1,"event_id":"policy","expected_revision":0,
        "expected_trust_revision":1,"reason":"explicit fixture policy",
        "policy":{"schema_version":1,"levels":[{"id":"public","name_fr":"Public","name_en":"Public"},{"id":"secret","name_fr":"Secret","name_en":"Secret"}],
        "compartments":["a","b"],"clearances":{"admin":{"level":"secret","compartments":["a","b"]},"sender":{"level":"secret","compartments":["a","b"]}}}}),
        "fixture-admin-token").unwrap();
    let intent = f.request("intent");
    let admission = json!({"schema_version":1,"event_id":"admission","expected_revision":1,
        "expected_trust_revision":1,"intent":intent});
    let attribute = |index: usize, event: &str| json!({"schema_version":1,"event_id":event,
        "expected_revision":1,"expected_trust_revision":1,"intent":intent,"source_index":index,
        "classification":{"level":"secret","compartments":["a","b"]},"reason":"explicit source decision"});
    let first = call(&f.root, "blp-preparation-input-attribute", attribute(0, "input-0"), "fixture-admin-token").unwrap();
    let count = first["source_count"].as_u64().unwrap() as usize;
    for index in 1..count {
        if let Err(e) = call(&f.root, "blp-preparation-input-attribute", attribute(index, &format!("input-{index}")), "fixture-admin-token") {
            let text = codes(e);
            assert!(text.contains("blp-attribution-conflict") || text.contains("blp-preparation-attribute-invalid"),
                "unexpected attribution failure: {text}");
        }
    }
    let result = call(&f.root, "blp-preparation-admit", admission, "fixture-sender-token").unwrap();
    assert_eq!(result["principal"], "sender");
    result
}

/// BF-16a controlled contract. History: this test file started as the
/// reproduction of the hole (an admitted turn's classified answer was served
/// by the public dialogue read with no reader identity or clearance check —
/// that old assertion passed before the fix and is retained as the negative
/// matrix below). Now the read refuses.
#[test]
fn dialogue_admitted_answer_readable_without_any_reader_control() {
    let f = Fixture::new("hole");
    let _admission = bootstrap(&f);
    // Admitted turn: same event id as the admission, so the receipt matches.
    let bound = serde_json::to_vec(&f.request("intent")).unwrap();
    let turn = f.root.preparation_send(&bound, &mut SystemEnvironment).unwrap();
    let token = f.register(&turn.id);
    f.root.preparation_worker_claim(&turn.id, &token).unwrap();
    f.root
        .preparation_worker_finish(
            &turn.id,
            Some(r#"{"message":"Réponse classifiée de fixture"}"#),
            None,
            "fixture-no-contact",
            &mut SystemEnvironment,
        )
        .unwrap();
    // BF-16a contract: a bare session is never authorization once admitted
    // classified output is involved. The read refuses with the stable code
    // and serves no turn byte.
    let bare = codes(refused(f.root.preparation_dialogue(&f.preparation.id, None)));
    assert!(
        bare.contains("blp-dialogue-authentication-required"),
        "expected bare-session refusal, got: {bare}"
    );
    assert!(
        !bare.contains("Réponse classifiée"),
        "refusal must carry no classified payload"
    );
    // An unknown token is a typed authorization refusal (HTTP 403 family).
    let unknown = codes(refused(f.root.preparation_dialogue(&f.preparation.id, Some("no-such-token"))));
    assert!(
        unknown.contains("Authorization") || unknown.contains("authorization_required"),
        "expected typed authorization refusal for unknown token, got: {unknown}"
    );
    // The reader identity holds NO clearance by fixture construction: the
    // known credential authenticates but the clearance check refuses.
    let under = codes(refused(f.root.preparation_dialogue(&f.preparation.id, Some("fixture-reader-token"))));
    assert!(
        under.contains("blp-dialogue-access-denied"),
        "expected under-cleared reader refusal, got: {under}"
    );
    // The admitted principal's own credential reads the dialogue: a cleared
    // reader with the right subject keeps the read available (recovery side).
    let cleared = f.root.preparation_dialogue(&f.preparation.id, Some("fixture-sender-token")).unwrap();
    let answered = cleared
        .value
        .iter()
        .find(|t| t.id == turn.id)
        .expect("admitted turn present in dialogue");
    let answer = answered.answer.as_ref().expect("answer present");
    assert!(
        answer.message.contains("Réponse classifiée"),
        "a cleared reader keeps reading the admitted dialogue"
    );
    let fixture_path = f.path.clone();
    f.close();
    std::fs::remove_dir_all(&fixture_path).ok();
}

/// Legacy continuity (BF-16a): with NO admission receipt matching the
/// preparation, a bare session still reads the dialogue exactly as before —
/// the control keys on matched admissions, never on policy presence, so a
/// configured Root with unclassified preparations keeps serving sessions.
#[test]
fn dialogue_without_any_matched_admission_keeps_legacy_bare_read() {
    let f = Fixture::new("legacy");
    // Trust configured (identities + policy) but no admission, attribution
    // or classified send happens: no receipt can match this preparation.
    // Three identities are the technical bootstrap minimum.
    call(&f.root, "config-apply", json!({"schema_version":1,"event_id":"init","expected_revision":0,"reason":"owned fixture",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
        "identities":[{"subject":"admin","integrity":3,"roles":["admin"],"token":"fixture-admin-token"},
        {"subject":"sender","integrity":1,"roles":["requester"],"token":"fixture-sender-token"},
        {"subject":"reader","integrity":1,"roles":["requester"],"token":"fixture-reader-token"}]}), "").unwrap();
    call(&f.root, "blp-policy-apply", json!({"schema_version":1,"event_id":"policy","expected_revision":0,
        "expected_trust_revision":1,"reason":"explicit fixture policy",
        "policy":{"schema_version":1,"levels":[{"id":"public","name_fr":"Public","name_en":"Public"}],
        "compartments":[],"clearances":{"admin":{"level":"public","compartments":[]}}}}),
        "fixture-admin-token").unwrap();
    // One ordinary legacy send/finish cycle with no admission at all.
    let unbound = serde_json::to_vec(&f.request("legacy-event")).unwrap();
    let turn = f.root.preparation_send(&unbound, &mut SystemEnvironment).unwrap();
    let token = f.register(&turn.id);
    f.root.preparation_worker_claim(&turn.id, &token).unwrap();
    f.root
        .preparation_worker_finish(
            &turn.id,
            Some(r#"{"message":"réponse legacy sans admission"}"#),
            None,
            "fixture-no-contact",
            &mut SystemEnvironment,
        )
        .unwrap();
    // Bare session reads it: the zero-receipt legacy path stays intact.
    let observed = f.root.preparation_dialogue(&f.preparation.id, None).unwrap();
    let answered = observed
        .value
        .iter()
        .find(|t| t.id == turn.id)
        .expect("legacy turn present in dialogue");
    let answer = answered.answer.as_ref().expect("answer present");
    assert!(
        answer.message.contains("réponse legacy"),
        "legacy dialogue remains readable to a bare session without admission"
    );
    let fixture_path = f.path.clone();
    f.close();
    std::fs::remove_dir_all(&fixture_path).ok();
}

/// BF-16b: show and history are presentation frontiers of the same admitted
/// classified output. The adopted documents they expose inherit the source
/// turn's classified domain, so once one admission receipt matches the
/// preparation's recorded turns, a bare session must be refused on BOTH
/// frontiers with the shared dialogue code and an under-cleared reader with
/// access-denied, before any preparation or document byte is served — while
/// the cleared sender keeps its reads (recovery side). Zero matched receipts
/// keeps the legacy bare read on both frontiers, proven here by the second
/// fixture whose preparation carries no admission at all.
#[test]
fn show_and_history_inherit_the_dialogue_reader_verdict() {
    // Classified fixture: admitted turn, so show/history become frontiers.
    let f = Fixture::new("show-frontier");
    let _admission = bootstrap(&f);
    let bound = serde_json::to_vec(&f.request("intent")).unwrap();
    let turn = f.root.preparation_send(&bound, &mut SystemEnvironment).unwrap();
    let token = f.register(&turn.id);
    f.root.preparation_worker_claim(&turn.id, &token).unwrap();
    f.root
        .preparation_worker_finish(
            &turn.id,
            Some(r#"{"message":"Réponse classifiée de fixture"}"#),
            None,
            "fixture-no-contact",
            &mut SystemEnvironment,
        )
        .unwrap();
    // BF-16b contract on show: a bare session is never authorization once
    // admitted classified output exists; the shared dialogue code refuses
    // before any preparation byte.
    let bare_show = codes(f.root.preparation_show(&f.preparation.id, None).map(|_| ()).map_err(|e| e).err().expect("classified show must refuse a bare session"));
    assert!(
        bare_show.contains("blp-dialogue-authentication-required"),
        "expected bare-session show refusal, got: {bare_show}"
    );
    // The same verdict on history for every document kind.
    for kind in ["besoin", "brief", "plan"] {
        let bare_history = codes(f.root
            .preparation_history(&f.preparation.id, kind, None, None)
            .map(|_| ())
            .map_err(|e| e)
            .err()
            .expect("classified history must refuse a bare session"));
        assert!(
            bare_history.contains("blp-dialogue-authentication-required"),
            "expected bare-session history refusal for {kind}, got: {bare_history}"
        );
    }
    // An under-cleared known reader authenticates but holds no clearance.
    let under_show = codes(f.root
        .preparation_show(&f.preparation.id, Some("fixture-reader-token"))
        .map(|_| ())
        .map_err(|e| e)
        .err()
        .expect("classified show must refuse an under-cleared reader"));
    assert!(
        under_show.contains("blp-dialogue-access-denied"),
        "expected under-cleared show refusal, got: {under_show}"
    );
    // Recovery side: the cleared sender keeps both frontiers readable. Show
    // serves the preparation (the dialogue is a separate frontier, asserted
    // above); history serves the saved revisions (none yet for every kind).
    let shown = f.root
        .preparation_show(&f.preparation.id, Some("fixture-sender-token"))
        .unwrap();
    assert_eq!(
        shown.value.id, f.preparation.id,
        "a cleared reader keeps reading the admitted show frontier"
    );
    for (kind, expected) in [("besoin", 1), ("brief", 0), ("plan", 0)] {
        let history = f.root
            .preparation_history(&f.preparation.id, kind, None, Some("fixture-sender-token"))
            .unwrap();
        assert_eq!(
            history.value.len(),
            expected,
            "history for {kind} must still be served to a cleared reader"
        );
    }
    let fixture_path = f.path.clone();
    f.close();
    std::fs::remove_dir_all(&fixture_path).ok();
}

/// BF-16b legacy continuity: with NO admission receipt matching the
/// preparation, show and history keep serving a bare session exactly as
/// before — the control keys on matched admissions, never on policy presence.
#[test]
fn show_and_history_without_any_matched_admission_keep_legacy_bare_read() {
    let f = Fixture::new("show-legacy");
    // Trust configured but no admission: no receipt can match this
    // preparation, so both frontiers stay on the legacy path.
    call(&f.root, "config-apply", json!({"schema_version":1,"event_id":"init","expected_revision":0,"reason":"owned fixture",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
        "identities":[{"subject":"admin","integrity":3,"roles":["admin"],"token":"fixture-admin-token"},
        {"subject":"sender","integrity":1,"roles":["requester"],"token":"fixture-sender-token"},
        {"subject":"reader","integrity":1,"roles":["requester"],"token":"fixture-reader-token"}]}), "").unwrap();
    call(&f.root, "blp-policy-apply", json!({"schema_version":1,"event_id":"policy","expected_revision":0,
        "expected_trust_revision":1,"reason":"explicit fixture policy",
        "policy":{"schema_version":1,"levels":[{"id":"public","name_fr":"Public","name_en":"Public"}],
        "compartments":[],"clearances":{"admin":{"level":"public","compartments":[]}}}}),
        "fixture-admin-token").unwrap();
    // One ordinary legacy send/finish cycle with no admission at all.
    let unbound = serde_json::to_vec(&f.request("legacy-event")).unwrap();
    let turn = f.root.preparation_send(&unbound, &mut SystemEnvironment).unwrap();
    let token = f.register(&turn.id);
    f.root.preparation_worker_claim(&turn.id, &token).unwrap();
    f.root
        .preparation_worker_finish(
            &turn.id,
            Some(r#"{"message":"réponse legacy sans admission"}"#),
            None,
            "fixture-no-contact",
            &mut SystemEnvironment,
        )
        .unwrap();
    // Bare session reads both frontiers: the zero-receipt legacy path stays
    // intact — show serves the preparation, history serves saved revisions.
    let shown = f.root.preparation_show(&f.preparation.id, None).unwrap();
    assert_eq!(
        shown.value.id, f.preparation.id,
        "legacy show remains readable to a bare session without admission"
    );
    for (kind, expected) in [("besoin", 1), ("brief", 0), ("plan", 0)] {
        let history = f.root
            .preparation_history(&f.preparation.id, kind, None, None)
            .unwrap();
        assert_eq!(
            history.value.len(),
            expected,
            "legacy history for {kind} must stay readable to a bare session"
        );
    }
    let fixture_path = f.path.clone();
    f.close();
    std::fs::remove_dir_all(&fixture_path).ok();
}
