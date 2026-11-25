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
/// observation refuse before any reservation or launch. A legacy provider-wait
/// file must agree with the native settings. All provider metadata is rechecked;
/// missing/stale/unsupported/IO/codec/release causes survive without empty success.
/// The initial claim creates an engine-labelled I0 output leaf; rechecks only
/// open the leaf named by the recorded event and bind its host scope. Output
/// remains untrusted. No Work, ledger, budget or provider process is changed here. Host preparation
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
    require_native_wait(store, &settings)?;
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
    let workspace = crate::native_provider_exec::output_workspace(
        &store.directory, crate::native_provider_exec::OutputRole::Reviewer,
        &request.event_id, review.is_none(),
    ).map_err(launch)?;
    let launch_request = Launch {
        workspace,
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
        let model = crate::provider_model_route::resolve_reviewer(
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

/// Check the same legacy coexistence contract as planner admission.
/// Review-specific limits still come from the saved reviewer configuration.
pub(super) fn require_native_wait(store: &RootStore, settings: &EngineSettingsDocument) -> Result<(), E> {
    crate::root_store::verify_native_wait(&store.directory, settings, "review-legacy-wait-adoption-required")
        .map_err(launch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    use crate::{
        planning_call_preview::PlanningCallPreviewRequest,
        provider_wait::{ProviderWaitConfig, ProviderWaitValues},
    };

    /// Exercise the wait-file gate through both production preparation entry
    /// points on an isolated Root. Observe a downstream refusal after passing
    /// the gate, without launching a provider or changing either input file.
    #[test]
    fn native_roles_read_matching_legacy_wait_without_changing_settings() {
        let area = tempfile::tempdir_in(crate::fixture_area()).unwrap();
        let store = RootStore::initialize(area.path(), &mut crate::SystemEnvironment).unwrap();
        let settings = store.settings().unwrap();
        let wait = settings.values.planning_wait_policy().unwrap();
        let mut legacy = ProviderWaitConfig {
            schema_version: 1,
            revision: 0,
            history: Some(vec![]),
            values: ProviderWaitValues {
                silence_seconds: wait.silence_seconds.0,
                max_duration_seconds: wait.max_duration_seconds.0,
                planning_lease_seconds: wait.default_lease_seconds.0,
            },
        };
        let planner = PlanningCallPreviewRequest {
            schema_version: 1,
            expected_revision: 1,
            claim_id: "missing-claim".into(),
            scope: "root".into(),
            scope_revision: 1,
            holder: "fixture".into(),
            generation: 1,
        };
        let reviewer = task_validation::Request {
            schema_version: 1,
            expected_revision: 1,
            event_id: "legacy-wait-fixture".into(),
            task_id: "missing-task".into(),
        };
        let observe = || {
            let tx = store.db.unchecked_transaction().unwrap();
            let plan = crate::planning_run_material::prepare(
                &tx, &store.root, &store.directory, &settings, "missing-work", &planner, false,
            ).err().unwrap();
            let review = prepare_for_job_using(
                &store, &tx, settings.clone(), "missing-work", &reviewer, None, None,
            ).err().unwrap();
            tx.rollback().unwrap();
            let review_code = match &review {
                E::Launch(error) => error.code(),
                _ => review.code(),
            };
            (plan.code(), review_code)
        };
        let settings_path = area.path().join(".swarm/engine-settings.json");
        let settings_bytes = std::fs::read(&settings_path).unwrap();
        let path = area.path().join(".swarm/provider-wait.json");
        let absent = observe();
        let check = |bytes: &[u8]| {
            std::fs::OpenOptions::new()
                .create(true).truncate(true).write(true).mode(0o600)
                .open(&path).unwrap().write_all(bytes).unwrap();
            let observed = observe();
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert_eq!(std::fs::read(&settings_path).unwrap(), settings_bytes);
            observed
        };
        assert_eq!(check(&serde_json::to_vec(&legacy).unwrap()), absent);
        legacy.values.planning_lease_seconds += 1;
        assert_eq!(check(&serde_json::to_vec(&legacy).unwrap()), (
            "planning-legacy-wait-adoption-required", "review-legacy-wait-adoption-required",
        ));
        assert_eq!(check(b"{"), ("provider-wait-source-invalid", "provider-wait-source-invalid"));
        store.close().unwrap();
    }
}
