use sha2::{Digest, Sha256};
use swarm_engine::{
    assemble_agent_workflow, EngineSettingsChange, RootStore, SettingCap, SettingCount,
    SystemEnvironment,
};
/// Owned initialized root; preserve failures, remove only this test's own files.
struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("workflow63-").unwrap());
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        RootStore::initialize(&p, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        Self(p)
    }
    /// Persist/reopen body policy using the real preview/CAS API, retaining history.
    fn policy(&self, cap: Option<i64>) {
        let store = RootStore::open(&self.0, true).unwrap();
        let doc = store.settings().unwrap();
        let history = doc.history.clone();
        let mut values = doc.values;
        values.agent_workflow_max_body_bytes = Some(SettingCap(cap.map(SettingCount)));
        let mut r = EngineSettingsChange {
            schema_version: values.schema_version(),
            event_id: swarm_engine::generate_identifier("workflow-policy-").unwrap(),
            expected_revision: doc.revision,
            values,
            reason: "Observe actual body policy".into(),
            preview_token: String::new(),
        };
        r.preview_token = store.preview_settings(&r).unwrap().preview_token;
        store.apply_settings(&r, &mut SystemEnvironment).unwrap();
        store.close().unwrap();
        let root = RootStore::open(&self.0, false).unwrap();
        let doc = root.settings().unwrap();
        assert_eq!(&doc.history[..history.len()], history.as_slice());
        assert_eq!(
            doc.values.agent_workflow_body_limit().unwrap(),
            cap.map(|v| v as usize)
        );
        root.close().unwrap();
    }
    /// Read through Root and close before checking settings/DB bytes.
    fn read(
        &self,
        role: &str,
    ) -> Result<swarm_engine::AgentWorkflowObservation, swarm_engine::RootStoreError> {
        let store = RootStore::open(&self.0, false).unwrap();
        let result = store.agent_workflow(role);
        store.close().unwrap();
        result
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
/// Validate one engine-built role: complete body/hash/manifest, worker-only
/// templates, diagnosis scope and reviewer separation. No provider or authority.
fn assert_frozen_role(role: &str) {
    let observed = assemble_agent_workflow(role, Some(32000)).unwrap();
    let (header, body) = observed.prompt.split_once('\n').unwrap();
    let body = body.strip_suffix('\n').unwrap();
    let metadata: serde_json::Value =
        serde_json::from_str(header.strip_prefix("SWARM_AGENT_WORKFLOW ").unwrap()).unwrap();
    assert_eq!(metadata["role"], role);
    assert_eq!(body.len(), observed.body_bytes);
    assert_eq!(
        format!("{:x}", Sha256::digest(body.as_bytes())),
        observed.workflow.sha256
    );
    assert!(body.contains("SOURCE tools/agent-workflows/CONTRACT.md"));
    assert_eq!(
        body.contains("SOURCE tools/agent-workflows/templates/HANDOFF.md"),
        role == "worker"
    );
    assert_eq!(
        body.contains("SOURCE tools/agent-workflows/templates/QUALITY.md"),
        role == "worker"
    );
    if role == "worker" {
        for clause in [
            "inputs/preconditions",
            "trace Admin/API/CLI to real use",
            "Refusal must not become empty success",
            "separate production and fixtures",
        ] {
            assert!(body.contains(clause), "missing quality clause {clause}");
        }
    }
    assert_eq!(
        observed
            .workflow
            .methods
            .as_ref()
            .unwrap()
            .iter()
            .any(|s| s == "debug"),
        role != "reviewer"
    );
    let methods = observed.workflow.methods.as_ref().unwrap();
    assert!(methods.iter().any(|m| m == "audit-pdca"));
    if role == "reviewer" {
        assert!(methods.iter().any(|m| m == "code-reviewer"));
        assert!(!methods.iter().any(|m| m == "apex"));
    }
}
/// Named actual native role witness; shared assertions inspect complete bytes,
/// not a declared role list, independent review or model compliance.
#[test]
fn frozen_role_planner_contract() {
    assert_frozen_role("planner");
}
/// Named actual native role witness; shared assertions inspect complete bytes,
/// not a declared role list, independent review or model compliance.
#[test]
fn frozen_role_subplanner_contract() {
    assert_frozen_role("subplanner");
}
/// Named actual native role witness; shared assertions inspect complete bytes,
/// not a declared role list, independent review or model compliance.
#[test]
fn frozen_role_worker_contract() {
    assert_frozen_role("worker");
}
/// Named actual native role witness; shared assertions inspect complete bytes,
/// not a declared role list, independent review or model compliance.
#[test]
fn frozen_role_reviewer_contract() {
    assert_frozen_role("reviewer");
}
/// Valid worker witness precedes unknown-role and local-override refusal;
/// saved Root context composition cannot replace engine-owned method bytes.
#[test]
fn frozen_workflows_bind_exact_body_and_role_without_local_override() {
    assert_frozen_role("worker");
    assert!(assemble_agent_workflow("worker", None).is_ok());
    let error = assemble_agent_workflow("PRIVATE_UNKNOWN_ROLE", None)
        .err()
        .unwrap();
    assert_eq!(error.reason(), "agent-workflow-role-unsupported");
    assert!(!format!("{error:?}").contains("PRIVATE"));
    let root = Fixture::new();
    std::fs::create_dir_all(root.0.join(".claude/skills/apex")).unwrap();
    std::fs::write(
        root.0.join(".claude/skills/apex/SKILL.md"),
        b"UNTRUSTED_WORKER_OVERRIDE",
    )
    .unwrap();
    let observed = root.read("worker").unwrap();
    assert!(!observed.prompt.contains("UNTRUSTED_WORKER_OVERRIDE"));
}
/// Saved body0/exact/under/null affect the real Root reader without truncation or
/// DB/settings mutation during observation. Prompt bytes and body bytes differ.
#[test]
fn workflow_body_policy_is_exact_saved_and_recoverable() {
    let f = Fixture::new();
    let witness = f.read("worker").unwrap();
    assert!(witness.prompt.len() > witness.body_bytes);
    for cap in [
        Some(witness.body_bytes as i64),
        Some(witness.body_bytes as i64 - 1),
        Some(0),
        None,
    ] {
        f.policy(cap);
        let settings = std::fs::read(f.0.join(".swarm/engine-settings.json")).unwrap();
        let db = std::fs::read(f.0.join(".swarm/state.db")).unwrap();
        let actual = f.read("worker");
        if cap.is_some_and(|cap| cap < witness.body_bytes as i64) {
            assert_eq!(
                actual.err().unwrap().to_string(),
                "agent-workflow-invalid: agent-workflow-body-limit-exceeded"
            );
        } else {
            assert_eq!(actual.unwrap().prompt, witness.prompt);
        }
        assert_eq!(std::fs::read(f.0.join(".swarm/state.db")).unwrap(), db);
        assert_eq!(
            std::fs::read(f.0.join(".swarm/engine-settings.json")).unwrap(),
            settings
        );
    }
}
/// Compose current ordinary project sources with a new manifest/hash; a missing
/// source refuses instead of dropping project instructions, and recovery restores.
#[test]
fn workflow_project_composition_preserves_primary_and_full_prompt_bindings() {
    let f = Fixture::new();
    let base = f.read("worker").unwrap();
    std::fs::write(f.0.join("guide.md"), b"Project instructions").unwrap();
    std::fs::write(f.0.join("swarm.project.json"),br#"{"version":1,"name":"Fixture","instructions":[{"path":"guide.md","roles":["worker"]}]}"#).unwrap();
    let combined = f.read("worker").unwrap();
    assert!(combined.prompt.contains("Project instructions"));
    assert_eq!(combined.body_bytes, base.body_bytes);
    let project = combined.workflow.project.as_ref().unwrap();
    assert_eq!(
        combined.workflow.sha256,
        format!(
            "{:x}",
            Sha256::digest(format!("{}\n{}", base.workflow.sha256, project.sha256).as_bytes())
        )
    );
    assert_eq!(
        combined.prompt_sha256,
        format!("{:x}", Sha256::digest(combined.prompt.as_bytes()))
    );
    std::fs::remove_file(f.0.join("guide.md")).unwrap();
    assert!(f.read("worker").is_err());
    std::fs::write(f.0.join("guide.md"), b"Project instructions").unwrap();
    assert_eq!(f.read("worker").unwrap().prompt, combined.prompt);
}
/// Old v12 settings remain readable, but the new body reader requires explicit
/// adoption; applying the same old journal is not implicit context authority.
#[test]
fn workflow_legacy_absence_requires_explicit_body_policy_adoption() {
    let f = Fixture::new();
    std::fs::write(
        f.0.join(".swarm/engine-settings.json"),
        include_bytes!("fixtures/legacy-v12-document.json"),
    )
    .unwrap();
    let error = f.read("worker").err().unwrap();
    assert_eq!(
        error.code(),
        "settings-agent-workflow-policy-not-configured"
    );
    f.policy(Some(32000));
    assert!(f.read("worker").is_ok());
}
/// Full-prompt freshness refuses forged text even with the old declared metadata
/// hash, and rechecks current saved caps. This is data freshness, not identity.
#[test]
fn workflow_freshness_checks_full_prompt_and_current_policy() {
    let f = Fixture::new();
    let previous = f.read("worker").unwrap();
    let check = |prior: &swarm_engine::AgentWorkflowObservation| {
        let root = RootStore::open(&f.0, false).unwrap();
        let result = root.check_agent_workflow(prior, "worker");
        root.close().unwrap();
        result
    };
    check(&previous).unwrap();
    for keep_hash in [true, false] {
        let mut forged = previous.clone();
        forged.prompt.push_str("PRIVATE_TAMPERED");
        if !keep_hash {
            forged.prompt_sha256 = format!("{:x}", Sha256::digest(forged.prompt.as_bytes()));
        }
        assert_eq!(forged.workflow.sha256, previous.workflow.sha256);
        let error = check(&forged).err().unwrap();
        assert!(error.to_string().contains("agent-workflow-stale"));
        assert!(!error.to_string().contains("PRIVATE_TAMPERED"));
    }
    let mut forged = previous.clone();
    forged.body_bytes += 1;
    assert!(check(&forged).is_err());
    check(&previous).unwrap();
    f.policy(None);
    check(&previous).unwrap();
    f.policy(Some(0));
    assert!(check(&previous)
        .err()
        .unwrap()
        .to_string()
        .contains("body-limit-exceeded"));
    f.policy(None);
    check(&previous).unwrap();
    std::fs::write(f.0.join("guide.md"), b"New project context").unwrap();
    std::fs::write(f.0.join("swarm.project.json"),br#"{"version":1,"name":"Fixture","instructions":[{"path":"guide.md","roles":["worker"]}]}"#).unwrap();
    assert!(check(&previous).is_err());
    check(&f.read("worker").unwrap()).unwrap();
}
