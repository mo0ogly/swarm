//! Reviewer-specific tool-free inputs. Candidate integration is in progress;
//! preparing these values grants no reservation, process or execution permission.
use crate::{
    agent_launch_error::finish,
    agent_launch_preflight::HostPreparation,
    provider_model_route::ModelRoutePreparation,
    root_store::{task_validation, RootStore, RootStoreError as E},
    AgentLaunchError as A, AgentLaunchPolicy, AgentWorkflow, EngineSettingsDocument, Launch,
    ReviewerConfig, TaskReportRequest, TaskReviewInputs, Work,
};
use rusqlite::Transaction;
use std::{collections::BTreeMap, path::Path, time::Duration};

/// Frozen current observations, deliberately without Debug. These private values
/// do not authenticate a model or grant execution; registry/peer/child checks and
/// current original source guards must precede any future provider exec.
pub(super) struct Material {
    pub work: Work,
    pub config: ReviewerConfig,
    pub submitted: TaskReportRequest,
    pub workflow: AgentWorkflow,
    pub artifacts: BTreeMap<String, String>,
    pub inputs: TaskReviewInputs,
    pub launch: Launch,
    pub model: ModelRoutePreparation,
    pub host: HostPreparation,
    pub policy: AgentLaunchPolicy,
    pub settings: EngineSettingsDocument,
    pub duration: Option<Duration>,
    /// Verified original recovery, not a caller-provided allowance or grant.
    pub recovery: Option<crate::root_store::review_recovery::Recovery>,
}
/// Retain typed failure behind its safe public Root family, without payload text.
fn launch(error: A) -> E {
    E::Launch(Box::new(error))
}
/// Prepare actual task originals plus a tool-free provider route/host candidate
/// under caller-owned SQL/settings snapshots. Required reply/event caps disabling
/// observation refuse before any reservation or launch. Legacy provider-wait
/// state requires adoption rather than bypass. All provider metadata is rechecked;
/// missing/stale/unsupported/IO/codec/release causes survive without empty success.
/// No Work, ledger, budget or provider process is changed here. Host preparation
/// creates/syncs/removes a workspace probe; failures include its cleanup causes.
#[cfg(test)]
pub(super) fn prepare(
    store: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    work: &str,
    request: &task_validation::Request,
) -> Result<Material, E> {
    prepare_for_job(store, tx, settings, work, request, None)
}
/// Rebuild observations for the exact current review record selected by the
/// checked job guard. No allowance is accepted from public JSON. Caller owns
/// snapshot/lock and must compare original hashes/TTL before any permission.
/// Like prepare, this performs a temporary workspace write/sync/remove probe;
/// its cleanup failure refuses rather than promising a side-effect-free read.
#[cfg(test)]
pub(super) fn prepare_for_job(
    store: &RootStore,
    tx: &Transaction<'_>,
    settings: EngineSettingsDocument,
    work: &str,
    request: &task_validation::Request,
    review: Option<&crate::IndependentReview>,
) -> Result<Material, E> {
    prepare_for_job_using(store,tx,settings,work,request,review,None)
}
/// Borrow externally held candidate custody inside this snapshot. All ordinary
/// provider/host/budget/settings guards remain; no descriptor ownership transfers.
/// Missing managed custody retains the existing refusal, never an ordinary fallback.
pub(super) fn prepare_for_job_using(
    store:&RootStore, tx:&Transaction<'_>,settings:EngineSettingsDocument,
    work:&str,request:&task_validation::Request,review:Option<&crate::IndependentReview>,
    candidate:Option<&task_validation::ReviewCandidate>,
)->Result<Material,E>{
    require_native_wait(store)?;
    let prepared = store.prepare_review_snapshot_using(tx,settings,work,request,review,candidate)?;
    let config = prepared
        .work
        .planning
        .as_deref()
        .and_then(|p| p.reviewer.as_deref())
        .cloned()
        .ok_or(E::Refused("reviewer_required"))?;
    let duration =
        crate::quotas::review_timeout_duration(config.timeout_seconds).map_err(launch)?;
    let current = prepared.work.clone();
    let settings = prepared.settings.clone();
    let workflow = prepared.workflow.workflow.clone();
    let mut artifacts = prepared.controls.artifacts.clone();
    let submitted =
        task_validation::submission(tx, &current, &prepared.preview.task_id).map_err(launch)?;
    let recovery = prepared.recovery.clone();
    let inputs = store.assemble_review_inputs_using(tx,prepared,candidate,true)?;
    if inputs.response_limit_bytes == Some(0) {
        return Err(E::Refused("review-call-response-disabled"));
    }
    let execution = settings
        .values
        .agent_execution_policy()
        .map_err(E::Settings)?;
    if execution.event_max_bytes.0.is_some_and(|n| n.0 == 0) {
        return Err(E::Refused("review-call-events-disabled"));
    }
    for source in inputs
        .materials
        .iter()
        .filter(|s| matches!(s.kind.as_str(), "report" | "document"))
    {
        if artifacts
            .get(&source.name)
            .is_some_and(|hash| hash != &source.sha256)
        {
            return Err(E::Refused("review-input-artifact-changed-since-controls"));
        }
        artifacts.insert(source.name.clone(), source.sha256.clone());
    }
    let policy = settings.values.agent_launch_policy().map_err(E::Settings)?;
    let launch_request = Launch {
        provider: config.provider.clone(),
        level: config
            .model_route
            .as_deref()
            .map_or_else(String::new, |r| r.level.clone()),
        model_policy_hash: config
            .model_route
            .as_deref()
            .map_or_else(String::new, |r| r.policy_hash.clone()),
        ..Default::default()
    };
    let reservation = store
        .directory
        .reserve_provider_configuration()
        .map_err(E::Storage)?;
    let observed: Result<_, A> = (|| {
        let (provider, _) =
            crate::agent_launch::provider(&store.directory, &launch_request, &policy)?;
        let hash = crate::provider_configuration::configuration_digest(
            &crate::encode_model(&provider)
                .map_err(|e| A::caused("review-provider-encoding", e))?,
        );
        if hash != config.provider_digest {
            return Err(A::Refused("review-provider-changed"));
        }
        let readonly = crate::prepare_assistant_provider(&provider)
            .map_err(|e| A::caused("review-provider-assistant-unavailable", e))?;
        let model = crate::provider_model_route::resolve(
            &readonly,
            &launch_request,
            &settings.values,
            &policy,
        )?;
        if !crate::provider_model_route::same_authority_route(
            config.model_route.as_deref(),
            model.route(),
        ) {
            return Err(A::Refused("review-model-changed"));
        }
        let host = crate::agent_launch_preflight::observe(
            &store.root,
            &launch_request.provider,
            &model,
            &launch_request,
            &policy,
        )?;
        model.recheck(&launch_request, &settings.values, &policy)?;
        crate::agent_launch_preflight::recheck(
            &store.root,
            &model,
            &launch_request,
            &policy,
            &host,
        )?;
        Ok((model, host))
    })();
    let (model, host) = finish(
        observed,
        reservation
            .close()
            .map_err(|e| A::caused("review-provider-reservation-close", e)),
    )
    .map_err(launch)?;
    Ok(Material {
        work: current,
        config,
        submitted,
        workflow,
        artifacts,
        inputs,
        launch: launch_request,
        model,
        host,
        policy,
        settings,
        duration,
        recovery,
    })
}
impl Material {
    /// Reobserve immutable provider/host inputs without extending the original
    /// registry TTL. Caller must separately enforce permission, current source
    /// and actual peer/child identity. No launch, debit, grant or mutation occurs.
    pub(super) fn fresh(&self, root: &Path) -> Result<(), A> {
        self.model
            .recheck(&self.launch, &self.settings.values, &self.policy)?;
        crate::agent_launch_preflight::recheck(
            root,
            &self.model,
            &self.launch,
            &self.policy,
            &self.host,
        )
    }
}

/// Require explicit adoption of historical provider-wait state, even during
/// ongoing source observations. Presence refuses; no historical file is parsed,
/// overwritten or silently mapped to planner policy. This creates no process.
pub(super) fn require_native_wait(store: &RootStore) -> Result<(), E> {
    if store
        .directory
        .names()
        .map_err(E::Storage)?
        .iter()
        .any(|name| name.as_slice() == b"provider-wait.json")
    {
        return Err(E::Refused("review-legacy-wait-adoption-required"));
    }
    Ok(())
}
