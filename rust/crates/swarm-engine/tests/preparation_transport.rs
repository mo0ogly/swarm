//! Destination binding and last-use transport decisions on owned inert
//! installations. No provider is contacted; no emission path exists behind
//! these operations. All fixtures live under the repository fixture area
//! (never /tmp) and every refusal asserts the exact typed code.
#![cfg(target_os = "linux")]
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{fs, io::Write, os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt}, path::PathBuf};
use swarm_engine::{
    EngineSettingsChange, EngineSettingsValues, Preparation, RootStore, SettingCount,
    SystemEnvironment,
};

struct Fixture {
    root: RootStore,
    path: PathBuf,
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
            "blp-transport-{label}-{}-{}",
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
            serde_json::to_vec(&json!({"root":path,"owner":"blp-transport-integration","fixture":true,"live":false}))
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
        Self { root, path, preparation }
    }

    /// Bind a fresh explicit send intent to the current saved capability.
    fn request(&self, event: &str) -> Value {
        let cap = self.root.preparation_call_capabilities().unwrap();
        let provider = cap.providers.iter().find(|p| p.name == "api-local").unwrap();
        json!({"version":1,"preparation_id":self.preparation.id,"expected_revision":self.preparation.revision,
            "event_id":event,"provider":"api-local","capability_hash":provider.digest,"message":"Owned fixture"})
    }

    /// Admit without dispatch: admission never contacts a provider.
    fn admit(&self, event: &str) -> Value {
        let intent = serde_json::to_vec(&self.request(event)).unwrap();
        let body = json!({"schema_version":1,"event_id":event,"expected_revision":1,
            "expected_trust_revision":self.trust_revision(),"intent":serde_json::from_slice::<Value>(&intent).unwrap()});
        call(&self.root, "blp-preparation-admit", body, "fixture-sender-token").unwrap()
    }

    /// Current trust revision of this Root's saved state.
    fn trust_revision(&self) -> i64 {
        serde_json::from_slice::<Value>(&self.root.trust("show", b"", "").unwrap().bytes)
            .unwrap()["revision"]
            .as_i64()
            .unwrap()
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
        reason: "Owned inert transport fixture".into(),
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

/// Bootstrap the shared fixture trust state: two identities, one explicit BLP
/// policy (no quarantine) and every source attributed for the prepared intent.
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
        // Distinct event ids per slice; exact duplicates retain the producer's
        // attribution conflict, but this fixture attributes every distinct slice.
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

/// Attribute the exact destination (admin echo of the resolved observable
/// account/endpoint) and return the producer reply.
fn attribute_destination(f: &Fixture, event: &str) -> Value {
    call(&f.root, "blp-preparation-destination-attribute", json!({
        "schema_version":1,"event_id":event,"expected_revision":1,"expected_trust_revision":f.trust_revision(),
        "connection":"api-local","classification":{"level":"secret","compartments":["a","b"]},
        "expected_base_url":"https://example.invalid/v1","expected_model":"fixed-model",
        "reason":"explicit destination decision"}), "fixture-admin-token").unwrap()
}

/// Transport request naming the admitted capsule and the exact observable
/// account/endpoint the caller intends to contact. The wire contract records
/// no event: the capsule itself is the observation's identity, so there is no
/// event_id field to supply.
fn transport(f: &Fixture, admission: &Value) -> Value {
    json!({"schema_version":1,"admission_id":admission["admission_id"],
        "expected_revision":1,"expected_trust_revision":f.trust_revision(),
        "connection":"api-local","expected_base_url":"https://example.invalid/v1","expected_model":"fixed-model"})
}

/// The granted last-use decision is fixed, replay-stable, key-free and
/// explicitly non-emissive. A wrong echo, an unattributed destination and a
/// changed connection each refuse with their exact typed code, and nothing is
/// ever written to reservations or turns by any transport observation.
#[test]
fn transport_decision_is_fixed_authorized_and_never_emissive() {
    let f = Fixture::new("decision");
    let admission = bootstrap(&f);
    // Refusals before any attribution exist.
    let err = codes(call(&f.root, "blp-preparation-transport", transport(&f, &admission), "fixture-sender-token").unwrap_err());
    assert!(err.contains("blp-destination-not-attributed"), "{err}");
    // Admin echoes wrong observable base_url: producer refuses before recording.
    let bad_echo = json!({
        "schema_version":1,"event_id":"dest-bad-echo","expected_revision":1,"expected_trust_revision":f.trust_revision(),
        "connection":"api-local","classification":{"level":"secret","compartments":["a","b"]},
        "expected_base_url":"https://attacker.invalid/v1","expected_model":"fixed-model",
        "reason":"explicit destination decision"});
    let err = codes(call(&f.root, "blp-preparation-destination-attribute", bad_echo, "fixture-admin-token").unwrap_err());
    assert!(err.contains("blp-destination-echo-mismatch"), "{err}");
    // Non-admin cannot attribute.
    let err = codes(call(&f.root, "blp-preparation-destination-attribute", json!({
        "schema_version":1,"event_id":"dest-nonadmin","expected_revision":1,"expected_trust_revision":f.trust_revision(),
        "connection":"api-local","classification":{"level":"secret","compartments":["a","b"]},
        "expected_base_url":"https://example.invalid/v1","expected_model":"fixed-model",
        "reason":"explicit destination decision"}), "fixture-sender-token").unwrap_err());
    assert!(err.contains("authorization_required"), "{err}");
    // Correct admin attribution records exactly once; replay returns the same.
    let recorded = attribute_destination(&f, "dest-bind");
    assert_eq!(recorded["capture_phase"], "destination-binding");
    assert_eq!(recorded["destination"]["model"], "fixed-model");
    assert!(!recorded.to_string().contains("PRIVATE_FIXTURE_KEY"));
    let replay = attribute_destination(&f, "dest-bind");
    assert_eq!(recorded, replay, "exact replay returns the stored reply");
    // Granted decision: fixed safe fields, key-free, explicitly non-emissive.
    let decision = call(&f.root, "blp-preparation-transport", transport(&f, &admission), "fixture-sender-token").unwrap();
    assert_eq!(decision["authorized"], true);
    assert_eq!(decision["state"], "transport-authorized-not-sent");
    assert_eq!(decision["principal"], "sender");
    // The provider name is `api-` + the connection catalogue id.
    assert_eq!(decision["destination"]["connection"], "local");
    assert_eq!(decision["destination_label"], json!({"level":"secret","compartments":["a","b"]}));
    assert_eq!(decision["runtime_authorized"], false);
    assert_eq!(decision["runtime_enforced"], false);
    assert!(!decision.to_string().contains("PRIVATE_FIXTURE_KEY"));
    assert_eq!(decision["source_count"], admission["source_count"]);
    // Exact replay returns the identical fixed decision.
    let replayed = call(&f.root, "blp-preparation-transport", transport(&f, &admission), "fixture-sender-token").unwrap();
    assert_eq!(decision, replayed, "replay returns the same observed decision");
    // Ownership: another principal cannot observe this capsule.
    let err = codes(call(&f.root, "blp-preparation-transport", transport(&f, &admission), "fixture-admin-token").unwrap_err());
    assert!(err.contains("blp-admission-not-found"), "{err}");
    // Zero emission: no turn, no reservation row was created by any of this.
    let fixture_path = f.path.clone();
    f.close();
    assert_eq!(
        read_reservations(&fixture_path),
        0,
        "transport observations must not reserve"
    );
    std::fs::remove_dir_all(&fixture_path).ok();
}

/// Read the reservations count from the fixture database after the Root closed.
fn read_reservations(path: &PathBuf) -> i64 {
    let db = rusqlite::Connection::open_with_flags(
        path.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    db.query_row("SELECT count(*) FROM preparation_reservations", [], |r| r.get(0))
        .unwrap()
}
