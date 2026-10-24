//! Frozen role framing, not agent authentication, execution or authorization.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    schema_version: i64,
    producer: String,
    producer_sha256: String,
    roles: BTreeMap<String, RoleFrame>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleFrame {
    version: i64,
    methods: Vec<String>,
    paths: Vec<String>,
    prefix: String,
    suffix: String,
}
/// A current instruction observation. Workflow SHA follows Go's base/project
/// composition; prompt_sha256 covers the entire returned prompt. Neither digest
/// proves identity, authority, receipt by an agent or a future freshness lease.
#[derive(Clone, Serialize)]
pub struct AgentWorkflowObservation {
    pub workflow: crate::AgentWorkflow,
    pub prompt: String,
    pub body_bytes: usize,
    /// Effective supplied base-body cap; None is explicit unlimited.
    pub body_limit: Option<usize>,
    /// Set by Root under its settings reservation; pure assembly has no owner revision.
    pub settings_revision: Option<crate::SettingCount>,
    pub prompt_sha256: String,
}
/// Typed safe framing failures. IO/context/codec causes remain available without
/// rendering project documents, supplied role strings or other private payloads.
pub enum AgentWorkflowError {
    Refused(&'static str),
    Descriptor(serde_json::Error),
    Text(std::str::Utf8Error),
    OwnedText(std::string::FromUtf8Error),
    Encoding(crate::WorkError),
    Allocation(std::collections::TryReserveError),
    Context(crate::ProjectContextError),
}
impl AgentWorkflowError {
    /// Return a static public family; context retains its existing category.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Context(e) => e.code(),
            _ => "agent-workflow-invalid",
        }
    }
    /// Return a static actionable boundary without echoing private input.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(reason) => reason,
            Self::Descriptor(_) => "agent-workflow-descriptor",
            Self::Text(_) | Self::OwnedText(_) => "agent-workflow-utf8",
            Self::Encoding(_) => "agent-workflow-encoding",
            Self::Allocation(_) => "agent-workflow-allocation",
            Self::Context(_) => "agent-workflow-project-context",
        }
    }
}
impl fmt::Display for AgentWorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for AgentWorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for AgentWorkflowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Descriptor(e) => Some(e),
            Self::Text(e) => Some(e),
            Self::OwnedText(e) => Some(e),
            Self::Encoding(e) => Some(e),
            Self::Allocation(e) => Some(e),
            Self::Context(e) => Some(e),
            Self::Refused(_) => None,
        }
    }
}
fn hash(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
static SOURCES: &[(&str, &[u8])] = &[
    (
        ".claude/skills/apex/SKILL.md",
        include_bytes!("../../../../.claude/skills/apex/SKILL.md"),
    ),
    (
        ".claude/skills/audit-pdca/SKILL.md",
        include_bytes!("../../../../.claude/skills/audit-pdca/SKILL.md"),
    ),
    (
        ".claude/skills/code-reviewer/SKILL.md",
        include_bytes!("../../../../.claude/skills/code-reviewer/SKILL.md"),
    ),
    (
        ".claude/skills/debug/SKILL.md",
        include_bytes!("../../../../.claude/skills/debug/SKILL.md"),
    ),
    (
        ".claude/skills/product-delivery/SKILL.md",
        include_bytes!("../../../../.claude/skills/product-delivery/SKILL.md"),
    ),
    (
        ".claude/skills/product-planning/SKILL.md",
        include_bytes!("../../../../.claude/skills/product-planning/SKILL.md"),
    ),
    (
        ".claude/skills/product-review/SKILL.md",
        include_bytes!("../../../../.claude/skills/product-review/SKILL.md"),
    ),
    (
        ".claude/skills/replan/SKILL.md",
        include_bytes!("../../../../.claude/skills/replan/SKILL.md"),
    ),
    (
        ".claude/skills/spec-audit/SKILL.md",
        include_bytes!("../../../../.claude/skills/spec-audit/SKILL.md"),
    ),
    (
        ".claude/skills/spec-builder/SKILL.md",
        include_bytes!("../../../../.claude/skills/spec-builder/SKILL.md"),
    ),
    (
        ".claude/skills/verify-fix/SKILL.md",
        include_bytes!("../../../../.claude/skills/verify-fix/SKILL.md"),
    ),
    (
        "tools/agent-workflows/CONTRACT.md",
        include_bytes!("../../../../tools/agent-workflows/CONTRACT.md"),
    ),
    (
        "tools/agent-workflows/templates/HANDOFF.md",
        include_bytes!("../../../../tools/agent-workflows/templates/HANDOFF.md"),
    ),
    (
        "tools/agent-workflows/templates/QUALITY.md",
        include_bytes!("../../../../tools/agent-workflows/templates/QUALITY.md"),
    ),
    (
        "tools/agent-workflows/templates/TRACKING.md",
        include_bytes!("../../../../tools/agent-workflows/templates/TRACKING.md"),
    ),
];
/// Select immutable build inputs; unknown/missing/empty sources explicitly refuse.
/// This never reads the worker checkout, filesystem, network or credentials.
fn embedded(path: &str) -> Result<&'static str, AgentWorkflowError> {
    let bytes = SOURCES
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, bytes)| *bytes)
        .filter(|bytes| !bytes.is_empty())
        .ok_or(AgentWorkflowError::Refused(
            "agent-workflow-source-unavailable",
        ))?;
    std::str::from_utf8(bytes).map_err(AgentWorkflowError::Text)
}
/// Checked technical representation arithmetic, never an operator policy ceiling.
fn add(a: usize, b: usize) -> Result<usize, AgentWorkflowError> {
    a.checked_add(b).ok_or(AgentWorkflowError::Refused(
        "agent-workflow-size-representation",
    ))
}
/// Serialize the public workflow manifest followed by an exact supplied body.
/// Errors retain safe codec/text causes; performs no IO or authority decision.
fn manifest(workflow: &crate::AgentWorkflow, body: &str) -> Result<String, AgentWorkflowError> {
    let metadata =
        String::from_utf8(crate::encode_model(workflow).map_err(AgentWorkflowError::Encoding)?)
            .map_err(AgentWorkflowError::OwnedText)?;
    let mut prompt = String::new();
    let size = add(
        add("SWARM_AGENT_WORKFLOW ".len(), metadata.len())?,
        add(body.len(), 2)?,
    )?;
    prompt
        .try_reserve_exact(size)
        .map_err(AgentWorkflowError::Allocation)?;
    prompt.push_str("SWARM_AGENT_WORKFLOW ");
    prompt.push_str(&metadata);
    prompt.push('\n');
    prompt.push_str(body);
    prompt.push('\n');
    Ok(prompt)
}
/// Assemble exact frozen base guidance for one of four protocol roles under an
/// explicit byte cap supplied by the owner: zero refuses nonempty body, None is
/// unlimited. Cap excludes metadata/final framing newline and project context.
/// Data comes only from compiled canonical sources and the Go-derived descriptor.
/// Unknown role, malformed descriptor/source, UTF8, cap, allocation/representation
/// or encoding failure refuses without truncated output. No IO, actor, permission,
/// model capacity or received-context proof is established. Caller owns policy
/// freshness/adoption, transport and any later execution/identity checks.
pub fn assemble_agent_workflow(
    role: &str,
    cap: Option<usize>,
) -> Result<AgentWorkflowObservation, AgentWorkflowError> {
    use AgentWorkflowError as E;
    if !matches!(role, "planner" | "subplanner" | "worker" | "reviewer") {
        return Err(E::Refused("agent-workflow-role-unsupported"));
    }
    let descriptor: Descriptor =
        serde_json::from_slice(include_bytes!("../config/agent-workflow-descriptor.json"))
            .map_err(E::Descriptor)?;
    if descriptor.schema_version != 1
        || descriptor.producer != "internal/engine/agent_workflow.go"
        || descriptor.producer_sha256.len() != 64
        || !descriptor
            .producer_sha256
            .bytes()
            .all(|c| c.is_ascii_hexdigit())
        || descriptor.roles.len() != 4
    {
        return Err(E::Refused("agent-workflow-descriptor-invalid"));
    }
    let selected = descriptor
        .roles
        .get(role)
        .ok_or(E::Refused("agent-workflow-role-missing"))?;
    let mut expected = vec!["tools/agent-workflows/CONTRACT.md".to_owned()];
    for method in &selected.methods {
        expected.push(format!(".claude/skills/{method}/SKILL.md"));
    }
    if role == "worker" {
        expected.extend([
            "tools/agent-workflows/templates/HANDOFF.md".to_owned(),
            "tools/agent-workflows/templates/TRACKING.md".to_owned(),
            "tools/agent-workflows/templates/QUALITY.md".to_owned(),
        ]);
    }
    if selected.version != 1
        || selected.methods.is_empty()
        || selected.paths != expected
        || !selected
            .prefix
            .starts_with(&format!("Cadrage moteur version 1 · rôle {role}.\n"))
        || !selected.suffix.starts_with("\nFIN DES MÉTHODES")
    {
        return Err(E::Refused("agent-workflow-descriptor-role-invalid"));
    }
    let mut size = add(selected.prefix.len(), selected.suffix.len())?;
    for path in &selected.paths {
        size = add(
            size,
            add(
                add("\nSOURCE ".len() + "\n".len() * 2, path.len())?,
                embedded(path)?.len(),
            )?,
        )?;
    }
    if cap.is_some_and(|cap| size > cap) {
        return Err(E::Refused("agent-workflow-body-limit-exceeded"));
    }
    let mut body = String::new();
    body.try_reserve_exact(size).map_err(E::Allocation)?;
    body.push_str(&selected.prefix);
    for path in &selected.paths {
        body.push_str("\nSOURCE ");
        body.push_str(path);
        body.push('\n');
        body.push_str(embedded(path)?);
        body.push('\n');
    }
    body.push_str(&selected.suffix);
    if body.len() != size {
        return Err(E::Refused("agent-workflow-body-size-incoherent"));
    }
    let workflow = crate::AgentWorkflow {
        version: 1,
        role: role.into(),
        methods: Some(selected.methods.clone()),
        sha256: hash(body.as_bytes()),
        ..Default::default()
    };
    let prompt = manifest(&workflow, &body)?;
    let prompt_sha256 = hash(prompt.as_bytes());
    Ok(AgentWorkflowObservation {
        workflow,
        prompt,
        body_bytes: size,
        body_limit: cap,
        settings_revision: None,
        prompt_sha256,
    })
}
/// Compose frozen guidance with the actual ordinary project context under explicit
/// owner-supplied policies. Base cap stays separate from project/model capacity.
/// Project read errors propagate; there is no empty-success fallback or writable
/// skill override. Rewrites the leading manifest exactly like Go and binds the
/// base SHA plus project SHA; prompt_sha256 also covers the final full bytes.
/// Filesystem reads may occur, but no mutation, agent, acceptance, authorization
/// or guarantee of future freshness/received context is established.
pub fn read_agent_workflow(
    root: &Path,
    role: &str,
    body_cap: Option<usize>,
    limits: &crate::ContextPolicyLimits,
) -> Result<AgentWorkflowObservation, AgentWorkflowError> {
    let mut observation = assemble_agent_workflow(role, body_cap)?;
    if let Some(project) =
        crate::read_project_context(root, role, limits).map_err(AgentWorkflowError::Context)?
    {
        let combined = format!(
            "{}\n{}",
            observation.workflow.sha256, project.context.sha256
        );
        observation.workflow.sha256 = hash(combined.as_bytes());
        observation.workflow.project = Some(Box::new(project.context));
        let (_, body) = observation
            .prompt
            .split_once('\n')
            .ok_or(AgentWorkflowError::Refused("agent-workflow-header-missing"))?;
        let metadata = String::from_utf8(
            crate::encode_model(&observation.workflow).map_err(AgentWorkflowError::Encoding)?,
        )
        .map_err(AgentWorkflowError::OwnedText)?;
        let mut prompt = String::new();
        let size = add(
            add(
                add("SWARM_AGENT_WORKFLOW ".len() + 1, metadata.len())?,
                body.len(),
            )?,
            project.prompt.len(),
        )?;
        prompt
            .try_reserve_exact(size)
            .map_err(AgentWorkflowError::Allocation)?;
        prompt.push_str("SWARM_AGENT_WORKFLOW ");
        prompt.push_str(&metadata);
        prompt.push('\n');
        prompt.push_str(body);
        prompt.push_str(&project.prompt);
        observation.prompt_sha256 = hash(prompt.as_bytes());
        observation.prompt = prompt;
    }
    Ok(observation)
}
