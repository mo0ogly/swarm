//! Worker-side last-use transport gate on owned inert installations. No
//! provider is contacted; no emission path exists behind this operation.
//! Fixtures live under the repository fixture area (never /tmp) and every
//! refusal asserts the exact typed code.
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
use swarm_engine::{
    EngineSettingsChange, EngineSettingsValues, Preparation, RootStore, SettingCount,
    SupervisorIdentity, SystemEnvironment,
};

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
            "blp-worker-gate-{label}-{}-{}",
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
            serde_json::to_vec(&json!({"root":path,"owner":"blp-worker-gate-integration","fixture":true,"live":false}))
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
        reason: "Owned inert worker-gate fixture".into(),
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

/// The worker-side gate reuses the shared admission/destination decision for the
/// turn it owns. The legacy zero-admission path stays byte-identical (straight
/// Ok); an admitted but unattributed destination refuses; a tampered token
/// refuses custody; a matching attribution authorizes with nothing reserved.
#[test]
fn worker_gate_refuses_unauthorized_and_passes_bound_sends() {
    let f = Fixture::new("decision");
    // Trust bootstrap and admission FIRST: capture_in_tx refuses a second
    // capture while any active turn exists, so the admission capsule and its
    // input attributions must be recorded before either turn is sent. The
    // admission intent names event `intent`; request_hash is canonical over
    // the full send struct INCLUDING event_id, so only a turn sent with the
    // SAME event id matches the receipt.
    let admission = bootstrap(&f);
    // Zero-admission turn: event `legacy-event` has NO matching receipt. Send
    // and claim exactly like the real worker journey; the gate must return
    // Ok immediately.
    let unbound = serde_json::to_vec(&f.request("legacy-event")).unwrap();
    let legacy = f.root.preparation_send(&unbound, &mut SystemEnvironment).unwrap();
    let legacy_token = f.register(&legacy.id);
    f.root.preparation_worker_claim(&legacy.id, &legacy_token).unwrap();
    let before = read_reservations(&f.path);
    assert_eq!(
        f.root
            .preparation_worker_transport_gate(&legacy.id, &legacy_token, "https://example.invalid/v1", "fixed-model")
            .unwrap(),
        (),
        "zero matching admissions keeps the legacy path exactly Ok"
    );
    assert_eq!(
        read_reservations(&f.path),
        before,
        "the zero-admission fast path must not reserve anything"
    );
    // End the legacy turn without inventing usage; only one active turn per
    // preparation is allowed.
    f.root
        .preparation_worker_finish(&legacy.id, None, None, "fixture-no-contact", &mut SystemEnvironment)
        .unwrap();
    // Admitted turn: SAME event id as the admission (`intent`), so the receipt
    // matches (preparation_id, provider, request_hash) exactly.
    let bound = serde_json::to_vec(&f.request("intent")).unwrap();
    let turn = f.root.preparation_send(&bound, &mut SystemEnvironment).unwrap();
    let token = f.register(&turn.id);
    f.root.preparation_worker_claim(&turn.id, &token).unwrap();
    // Wrong custody token: registration identity, never a BLP code.
    let err = codes(
        f.root
            .preparation_worker_transport_gate(&turn.id, "0".repeat(64).as_str(), "https://example.invalid/v1", "fixed-model")
            .unwrap_err(),
    );
    assert!(err.contains("preparation-worker-registration-mismatch"), "{err}");
    // Admitted but destination not yet attributed: typed fail-closed refusal.
    let err = codes(
        f.root
            .preparation_worker_transport_gate(&turn.id, &token, "https://example.invalid/v1", "fixed-model")
            .unwrap_err(),
    );
    assert!(err.contains("blp-destination-not-attributed"), "{err}");
    // Admin attributes the exact observable destination; a wrong echo refuses.
    let bad_echo = json!({
        "schema_version":1,"event_id":"dest-bad-echo","expected_revision":1,"expected_trust_revision":f.trust_revision(),
        "connection":"api-local","classification":{"level":"secret","compartments":["a","b"]},
        "expected_base_url":"https://attacker.invalid/v1","expected_model":"fixed-model",
        "reason":"explicit destination decision"});
    let err = codes(call(&f.root, "blp-preparation-destination-attribute", bad_echo, "fixture-admin-token").unwrap_err());
    assert!(err.contains("blp-destination-echo-mismatch"), "{err}");
    attribute_destination(&f, "dest-bind");
    // Matching attribution: the gate authorizes the exact observable endpoint
    // the worker will dial. A different base_url/model than the dialled one
    // must refuse (the worker's actual endpoint is the gate's input).
    let err = codes(
        f.root
            .preparation_worker_transport_gate(&turn.id, &token, "https://attacker.invalid/v1", "fixed-model")
            .unwrap_err(),
    );
    assert!(err.contains("blp-destination-echo-mismatch"), "{err}");
    let err = codes(
        f.root
            .preparation_worker_transport_gate(&turn.id, &token, "https://example.invalid/v1", "other-model")
            .unwrap_err(),
    );
    assert!(err.contains("blp-destination-echo-mismatch"), "{err}");
    // Gate passes: the actual worker endpoint matches the attributed one.
    f.root
        .preparation_worker_transport_gate(&turn.id, &token, "https://example.invalid/v1", "fixed-model")
        .unwrap();
    // No reservation was created by any gate observation: exactly the rows
    // the two claims wrote, never a gate row.
    assert_eq!(
        read_reservations(&f.path),
        before + 1,
        "the gate never reserves"
    );
    let fixture_path = f.path.clone();
    f.close();
    std::fs::remove_dir_all(&fixture_path).ok();
}

/// Read the reservations count from the fixture database.
fn read_reservations(path: &PathBuf) -> i64 {
    let db = rusqlite::Connection::open_with_flags(
        path.join(".swarm/state.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    db.query_row("SELECT count(*) FROM preparation_reservations", [], |r| r.get(0))
        .unwrap()
}
