use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::Path};
use swarm_engine::{
    decode_planning_report_event, read_planning_report, EngineSettingsChange, ExchangeArtifact,
    PlanningEvent, RootStore, SettingCap, SettingCount, SystemEnvironment,
};
/// Owned initialized fixture; no live state, provider or model is used.
struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(swarm_engine::generate_identifier("report65-").unwrap());
        fs::create_dir(&root).unwrap();
        RootStore::initialize(&root, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        Self(root)
    }
    /// Write complete fixture bytes and their declared event; labels grant no authority.
    fn event(&self, raw: &[u8]) -> PlanningEvent {
        fs::write(self.0.join("report.md"), raw).unwrap();
        PlanningEvent {
            id: "event".into(),
            scope: "root".into(),
            task: "task".into(),
            attempt: "attempt".into(),
            kind: "handoff".into(),
            handoff: Some(Box::new(ExchangeArtifact {
                path: "report.md".into(),
                sha256: format!("{:x}", Sha256::digest(raw)),
            })),
            ..Default::default()
        }
    }
    /// Save/reopen the actual operator policy through preview/CAS, preserving history.
    fn policy(&self, cap: Option<i64>) {
        let root = RootStore::open(&self.0, true).unwrap();
        let doc = root.settings().unwrap();
        let history = doc.history.clone();
        let mut values = doc.values;
        values.planning_report_max_bytes = Some(SettingCap(cap.map(SettingCount)));
        let mut change = EngineSettingsChange {
            schema_version: values.schema_version(),
            event_id: swarm_engine::generate_identifier("report-policy-").unwrap(),
            expected_revision: doc.revision,
            values,
            reason: "Verify complete reports".into(),
            preview_token: String::new(),
        };
        change.preview_token = root.preview_settings(&change).unwrap().preview_token;
        root.apply_settings(&change, &mut SystemEnvironment)
            .unwrap();
        root.close().unwrap();
        let root = RootStore::open(&self.0, false).unwrap();
        let current = root.settings().unwrap();
        assert_eq!(&current.history[..history.len()], history.as_slice());
        assert_eq!(
            current.values.planning_report_limit().unwrap(),
            cap.map(|v| v as usize)
        );
        root.close().unwrap();
    }
    /// Observe through Root and close before examining persisted bytes.
    fn read(
        &self,
        event: &PlanningEvent,
    ) -> Result<Option<swarm_engine::PlanningReportContent>, swarm_engine::RootStoreError> {
        let root = RootStore::open(&self.0, false).unwrap();
        let result = root.planning_report(event);
        root.close().unwrap();
        result
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Explicit source wins; historical fallback requires exactly one lowercase .md
/// on handoff only. No selection is None, never a fabricated empty report.
#[test]
fn report_selection_matches_go_and_preserves_exact_complete_bytes() {
    let f = Fixture::new();
    let mut event = f.event("Report café <&>\n".as_bytes());
    let value = f.read(&event).unwrap().unwrap();
    assert_eq!(value.event_id, event.id);
    assert_eq!(value.text, "Report café <&>\n");
    assert_eq!(value.attempt, event.attempt);
    let reference = *event.handoff.take().unwrap();
    event.artifacts = Some(vec![reference.clone()]);
    assert_eq!(f.read(&event).unwrap().unwrap(), value);
    for kind in ["operator_resume", "HANDOFF"] {
        event.kind = kind.into();
        assert!(f.read(&event).unwrap().is_none());
    }
    event.kind = "handoff".into();
    event.artifacts.as_mut().unwrap().push(reference.clone());
    assert!(f.read(&event).unwrap().is_none());
    event.artifacts = Some(vec![ExchangeArtifact {
        path: "report.MD".into(),
        ..reference.clone()
    }]);
    assert!(f.read(&event).unwrap().is_none());
    event.handoff = Some(Box::new(reference));
    assert_eq!(f.read(&event).unwrap().unwrap(), value);
}
/// Exact persisted boundaries, zero/unlimited, legacy absence/adoption and recovery
/// are observed without changing database/settings during each report read.
#[test]
fn report_policy_is_saved_exact_zero_unlimited_and_legacy_requires_adoption() {
    let f = Fixture::new();
    let e = f.event(&vec![b'x'; 64000]);
    let db = fs::read(f.0.join(".swarm/state.db")).unwrap();
    let settings = fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
    assert_eq!(f.read(&e).unwrap().unwrap().text.len(), 64000);
    assert_eq!(fs::read(f.0.join(".swarm/state.db")).unwrap(), db);
    assert_eq!(
        fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
        settings
    );
    let e = f.event(&vec![b'x'; 64001]);
    assert!(f.read(&e).is_err());
    f.policy(Some(64001));
    assert_eq!(f.read(&e).unwrap().unwrap().text.len(), 64001);
    f.policy(Some(0));
    assert!(f.read(&e).is_err());
    f.policy(None);
    assert!(f.read(&e).unwrap().is_some());
    fs::write(
        f.0.join(".swarm/engine-settings.json"),
        include_bytes!("fixtures/legacy-v13-document.json"),
    )
    .unwrap();
    assert!(f
        .read(&e)
        .unwrap_err()
        .to_string()
        .contains("settings-planning-report-policy-not-configured"));
    f.policy(Some(64001));
    assert!(f.read(&e).unwrap().is_some());
}
/// Every injected refusal follows a valid report witness. Missing/unsafe/malformed
/// bytes never become None; restoration is a fresh valid observation.
#[test]
fn report_refuses_invalid_bytes_paths_and_changed_digest_with_recovery() {
    let f = Fixture::new();
    for raw in [b"".as_slice(), b"nul\0byte", b"\xff"] {
        let witness = f.event(b"valid");
        assert!(f.read(&witness).unwrap().is_some());
        let e = f.event(raw);
        let error = f.read(&e).unwrap_err();
        assert_eq!(error.code(), "planning-report-invalid");
        assert!(!format!("{error:?}").contains("report.md"));
    }
    let e = f.event(b"valid");
    assert!(f.read(&e).unwrap().is_some());
    fs::write(f.0.join("report.md"), b"changed").unwrap();
    assert!(f
        .read(&e)
        .unwrap_err()
        .to_string()
        .contains("report-digest-changed"));
    fs::write(f.0.join("report.md"), b"valid").unwrap();
    assert!(f.read(&e).unwrap().is_some());
    fs::remove_file(f.0.join("report.md")).unwrap();
    let error = f.read(&e).unwrap_err();
    assert!(error.source().unwrap().source().is_some());
    assert!(!error.to_string().contains("report.md"));
    for name in ["../PRIVATE_REPORT", "/PRIVATE_REPORT", ".swarm/state.db"] {
        let mut e = f.event(b"valid");
        assert!(f.read(&e).unwrap().is_some());
        e.handoff.as_mut().unwrap().path = name.into();
        let error = f.read(&e).unwrap_err();
        assert!(!error.to_string().contains("PRIVATE_REPORT"));
    }
    let e = f.event(b"valid");
    assert!(f.read(&e).unwrap().is_some());
}
/// Preserve the shared Go-compatible contained-alias contract, then reject an
/// outside target and directory after fresh valid witnesses; no OS lease is claimed.
#[cfg(unix)]
#[test]
fn report_resolves_contained_symlink_and_refuses_escape_nonregular_files() {
    let f = Fixture::new();
    let mut e = f.event(b"valid");
    assert!(f.read(&e).unwrap().is_some());
    std::os::unix::fs::symlink(f.0.join("report.md"), f.0.join("link.md")).unwrap();
    e.handoff.as_mut().unwrap().path = "link.md".into();
    assert_eq!(f.read(&e).unwrap().unwrap().text, "valid");
    let outside = Fixture::new();
    outside.event(b"valid");
    std::os::unix::fs::symlink(outside.0.join("report.md"), f.0.join("escape.md")).unwrap();
    e.handoff.as_mut().unwrap().path = "escape.md".into();
    assert!(f.read(&e).is_err());
    e.handoff.as_mut().unwrap().path = "link.md".into();
    assert!(f.read(&e).unwrap().is_some());
    e.handoff.as_mut().unwrap().path = "directory.md".into();
    fs::create_dir(f.0.join("directory.md")).unwrap();
    assert!(f.read(&e).is_err());
}
/// Real Go-compatible decoding is strict about unknown fields/types, preserves
/// omitted reference as data and does not authenticate supplied identity labels.
#[test]
fn report_event_codec_rejects_invalid_inputs_without_private_diagnostics() {
    let f = Fixture::new();
    let e = f.event(b"valid");
    let raw = serde_json::to_vec(&e).unwrap();
    let decoded = decode_planning_report_event(&raw).unwrap();
    assert!(read_planning_report(Path::new(&f.0), &decoded, None)
        .unwrap()
        .is_some());
    for raw in [
        b"{\"PRIVATE_FIELD\":true}".as_slice(),
        b"{\"id\":3}",
        b"{} trailing",
        b"\xff",
    ] {
        let err = decode_planning_report_event(raw).unwrap_err();
        assert!(err.source().is_some());
        assert!(!format!("{err:?}").contains("PRIVATE_FIELD"));
    }
    assert!(
        read_planning_report(&f.0, &decode_planning_report_event(b"{}").unwrap(), None)
            .unwrap()
            .is_none()
    );
}
