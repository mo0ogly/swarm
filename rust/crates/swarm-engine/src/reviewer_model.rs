//! Versioned routing for a mission's independent reviewer. Configuration never
//! authorizes a call or changes the provenance of an existing opinion.
use crate::{AgentLaunchError as E, EngineSettingsDocument, Launch, ReviewerConfig, RoleModel, Work, WorkEnvironment};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Existing role-model envelope. This native operation supports the explicit
/// independent reviewer selection and refuses other roles and inheritance.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewerModelRequest {
    pub schema_version: i64,
    pub event_id: String,
    pub expected_revision: i64,
    pub scope: String,
    pub reviewer: bool,
    pub provider: String,
    pub level: String,
    pub model_policy_hash: String,
    pub provider_digest: String,
    pub inherit: bool,
}

/// Recognize the bounded reviewer configuration request. Missing authority,
/// unknown keys and invalid provider policy pins fail without any IO.
pub fn decode_reviewer_model_request(raw: &[u8]) -> Result<ReviewerModelRequest, E> {
    use crate::model_wire::Kind;
    let request: ReviewerModelRequest = crate::work::decode_mode(
        raw,
        &[
            ("schema_version", Kind::Int64),
            ("event_id", Kind::String),
            ("expected_revision", Kind::Int64),
            ("scope", Kind::String),
            ("reviewer", Kind::Bool),
            ("provider", Kind::String),
            ("level", Kind::String),
            ("model_policy_hash", Kind::String),
            ("provider_digest", Kind::String),
            ("inherit", Kind::Bool),
        ],
        true,
    ).map_err(|e| E::caused("reviewer-model-request-invalid", e))?;
    if request.schema_version != 1
        || request.expected_revision < 1
        || !request.reviewer
        || !request.scope.is_empty()
        || request.inherit
        || ![&request.event_id, &request.provider].into_iter()
            .all(|v| swarm_identity_contract::name(v.as_bytes()))
        || !matches!(request.level.as_str(), "auto" | "simple" | "standard" | "exigeant")
        || !crate::provider_configuration::configuration_digest_valid(&request.model_policy_hash)
        || (!request.provider_digest.is_empty()
            && !crate::provider_configuration::configuration_digest_valid(&request.provider_digest))
    {
        return Err(E::Refused("reviewer-model-request-invalid"));
    }
    Ok(request)
}

/// Apply requires the exact route observed during preview. Revision and model
/// policy pinning are separate checks enforced by the caller and resolver.
pub fn require_reviewer_model_preview(request: &ReviewerModelRequest) -> Result<(), E> {
    if request.provider_digest.is_empty() {
        return Err(E::Refused("reviewer-model-preview-required"));
    }
    Ok(())
}

/// Reduced public configuration receipt. No task instructions, evidence,
/// credentials or provider launch authority are returned.
#[derive(Serialize)]
pub struct ReviewerModelView {
    pub revision: i64,
    pub planning: ReviewerModelPlanning,
}

#[derive(Serialize)]
pub struct ReviewerModelPlanning {
    pub reviewer: ReviewerConfig,
}

impl ReviewerModelView {
    /// Extract only the recorded reviewer configuration from this Work revision.
    pub fn from_work(work: &Work) -> Result<Self, E> {
        let reviewer = work.planning.as_deref()
            .and_then(|p| p.reviewer.as_deref())
            .ok_or(E::Refused("reviewer-model-reviewer-required"))?;
        Ok(Self { revision: work.revision, planning: ReviewerModelPlanning { reviewer: reviewer.clone() } })
    }
}

/// Reject a configuration write when a planner owns a scope or a review is in
/// flight. The normalized owner ledger is checked in the caller's transaction.
#[cfg(target_os = "linux")]
fn require_idle(db: &rusqlite::Connection, work: &Work) -> Result<(), E> {
    let planning = work.planning.as_deref().ok_or(E::Refused("reviewer-model-reviewer-required"))?;
    if !planning.paused {
        return Err(E::Refused("reviewer-model-planning-pause-required"));
    }
    if planning.scopes.iter().flatten().any(|scope| !scope.holder.is_empty())
        || work.tasks.iter().flatten().any(|task| task.independent_review.as_deref()
            .is_some_and(|review| matches!(review.state.as_str(), "queued" | "running")))
    {
        return Err(E::Refused("reviewer-model-active-owner"));
    }
    crate::RootStore::require_idle_mission_reviews(db, &work.id)
}

/// Resolve an actual tool-free route under the catalogue reservation. Caller
/// owns the saved settings reservation that also fences catalogue and model
/// policy updates. Original and cleanup errors are joined.
#[cfg(target_os = "linux")]
fn resolve(root: &Path, request: &ReviewerModelRequest, settings: &EngineSettingsDocument) -> Result<RoleModel, E> {
    use crate::agent_launch_error::finish;
    let policy = settings.values.agent_launch_policy().map_err(|e| E::caused(e.reason(), e))?;
    let directory = crate::local_storage::LocalDirectory::open(&root.join(".swarm"))
        .map_err(|e| E::caused("reviewer-model-root-open", e))?;
    let result = (|| {
        let guard = directory.reserve_provider_configuration()
            .map_err(|e| E::caused("reviewer-model-provider-reserve", e))?;
        let launch = Launch {
            provider: request.provider.clone(),
            level: request.level.clone(),
            model_policy_hash: request.model_policy_hash.clone(),
            ..Default::default()
        };
        let observed = crate::reviewer_bootstrap::observe_provider(&directory, &launch, settings, &policy)
            .and_then(|(provider_digest, route)| {
                if !request.provider_digest.is_empty() && provider_digest != request.provider_digest {
                    return Err(E::Refused("reviewer-model-provider-changed"));
                }
                if route.is_none() {
                    return Err(E::Refused("reviewer-model-route-required"));
                }
                Ok(RoleModel { provider: request.provider.clone(), provider_digest, route, ..Default::default() })
            });
        finish(observed, guard.close().map_err(|e| E::caused("reviewer-model-provider-close", e)))
    })();
    finish(result, directory.close().map_err(|e| E::caused("reviewer-model-root-close", e)))
}

/// Stage a resolved reviewer choice on the caller's candidate. No quota, task,
/// attempt or existing opinion is rewritten. The caller owns durable replay.
#[cfg(target_os = "linux")]
pub(crate) fn apply(
    db: &rusqlite::Connection,
    root: &Path,
    work: &mut Work,
    request: &ReviewerModelRequest,
    settings: &EngineSettingsDocument,
    env: &mut impl WorkEnvironment,
) -> Result<(), E> {
    require_idle(db, work)?;
    let reviewer = work.planning.as_deref_mut()
        .and_then(|p| p.reviewer.as_deref_mut())
        .ok_or(E::Refused("reviewer-model-reviewer-required"))?;
    let mut selection = resolve(root, request, settings)?;
    selection.actor = env.operator().map_err(|e| E::caused("reviewer-model-operator", e))?;
    selection.at = env.timestamp().map_err(|e| E::caused("reviewer-model-clock", e))?;
    reviewer.provider.clone_from(&selection.provider);
    reviewer.provider_digest.clone_from(&selection.provider_digest);
    reviewer.model_route.clone_from(&selection.route);
    reviewer.model_selection = Some(Box::new(selection));
    Ok(())
}
