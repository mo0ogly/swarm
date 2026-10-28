//! Provider/reviewer bootstrap observations. No process or caller identity grant.
use crate::{ModelRoute, ReviewerConfig};
/// Internal configuration observation from actual sources, never JSON input or
/// a launch capability. Future execution must recheck digest/model/permissions.
pub(crate) struct Prepared {
    pub provider: String,
    pub provider_digest: String,
    pub route: Option<Box<ModelRoute>>,
    pub reviewer: Box<ReviewerConfig>,
}
/// Read the selected CLI catalogue and saved defaults under its real provider
/// reservation (creating its private providers.lock if absent). Strict
/// name/source/model/readonly adapter checks precede output;
/// close/IO/refusal causes are joined. No provider/model is executed. This is a
/// point-in-time configuration snapshot, not a lease preventing future changes.
/// Provider cooldown is neither consumed nor cleared: configuration executes no
/// provider; actual admission must still enforce the existing cooldown guard.
/// Root owns saved settings, transport authority, Work transaction and replay.
#[cfg(target_os = "linux")]
pub(crate) fn prepare(
    root: &std::path::Path,
    request: &crate::PlanningRequest,
    at: &str,
    settings: &crate::EngineSettingsDocument,
) -> Result<Prepared, crate::AgentLaunchError> {
    use crate::{agent_launch_error::finish, local_storage::LocalDirectory, AgentLaunchError as E};
    if !swarm_identity_contract::name(request.provider.as_bytes()) {
        return Err(E::Refused("planning-provider-name-invalid"));
    }
    let defaults = settings
        .values
        .reviewer_creation_defaults()
        .map_err(|e| E::caused(e.reason(), e))?;
    let policy = settings
        .values
        .agent_launch_policy()
        .map_err(|e| E::caused(e.reason(), e))?;
    let directory = LocalDirectory::open(&root.join(".swarm"))
        .map_err(|e| E::caused("planning-provider-directory", e))?;
    let result = (|| {
        let reservation = directory
            .reserve_provider_configuration()
            .map_err(|e| E::caused("planning-provider-reservation", e))?;
        let prepared = (|| {
            let launch = crate::Launch {
                provider: request.provider.clone(),
                level: request.level.clone(),
                model_policy_hash: request.model_policy_hash.clone(),
                ..Default::default()
            };
            let (digest, route) = observe_provider(&directory, &launch, settings, &policy)?;
            let instant = swarm_wire_json::go_rfc3339::parse(at)
                .map_err(|e| E::caused("planning-provider-clock", e))?;
            let authorized = instant
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|e| E::caused("planning-provider-clock", e))?;
            let max_calls = if request.max_review_calls == 0 {
                request.max_activations
            } else {
                request.max_review_calls
            };
            if max_calls < 1 {
                return Err(E::Refused("planning-review-budget-positive-required"));
            }
            let reviewer = ReviewerConfig {
                provider: request.provider.clone(),
                provider_digest: digest.clone(),
                model_route: route.clone(),
                max_calls,
                authorized,
                max_files: Some(Box::new(defaults.delivery_files.0)),
                max_context_files: Some(Box::new(defaults.context_files.0)),
                max_source_bytes: Some(Box::new(defaults.source_bytes.0)),
                max_context_bytes: Some(Box::new(defaults.context_bytes.0)),
                ..Default::default()
            };
            Ok(Prepared {
                provider: request.provider.clone(),
                provider_digest: digest,
                route,
                reviewer: Box::new(reviewer),
            })
        })();
        finish(
            prepared,
            reservation
                .close()
                .map_err(|e| E::caused("planning-provider-reservation-close", e)),
        )
    })();
    finish(
        result,
        directory
            .close()
            .map_err(|e| E::caused("planning-provider-directory-close", e)),
    )
}
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::{
        fs,
        io::Write,
        os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    };
    /// Requested readonly role flags survive later model/effort compilation.
    /// Empty fixture executable is only inspected, never run; this proves no
    /// external CLI support, effective sandbox or actual model execution.
    #[test]
    fn readonly_adapter_before_model_arguments_preserves_effort_and_sandbox() {
        let root = std::env::temp_dir().join(crate::generate_identifier("reviewer-argv-").unwrap());
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let path = root.join("codex");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .open(&path)
            .unwrap();
        file.write_all(b"fixture inspected only").unwrap();
        file.sync_all().unwrap();
        drop(file);
        let provider = crate::Provider {
            command: path.to_str().unwrap().into(),
            args: Some(vec![
                "--dangerously-bypass-approvals-and-sandbox".into(),
                "--model".into(),
                "old-model".into(),
            ]),
            ..Default::default()
        };
        let readonly = crate::prepare_assistant_provider(&provider).unwrap();
        let choice = crate::ModelChoice {
            model: "fixture-model".into(),
            effort: "fixture-effort".into(),
        };
        let args = crate::compile_provider_model_arguments(
            crate::ProviderModelSyntax::Codex,
            readonly.args.as_deref().unwrap(),
            &choice,
        )
        .unwrap();
        assert!(args.windows(2).any(|w| w == ["--sandbox", "read-only"]));
        assert!(args.windows(2).any(|w| w == ["--model", "fixture-model"]));
        assert!(args
            .iter()
            .any(|v| v.contains("model_reasoning_effort") && v.contains("fixture-effort")));
        assert!(!args.iter().any(|v| v.contains("dangerously")));
        fs::remove_dir_all(root).unwrap();
    }
}

/// Observe an actual CLI catalogue/model source under the caller-owned provider
/// reservation. Preserves readonly-before-model order and typed safe failures.
/// No IO handle escapes, provider runs, cooldown is consumed, identity authenticated
/// or permission granted. The caller must compare any previously frozen digest
/// and close its reservation, preserving original and cleanup causes.
#[cfg(target_os = "linux")]
pub(crate) fn observe_provider(
    directory: &crate::local_storage::LocalDirectory,
    launch: &crate::Launch,
    settings: &crate::EngineSettingsDocument,
    policy: &crate::AgentLaunchPolicy,
) -> Result<(String, Option<Box<crate::ModelRoute>>), crate::AgentLaunchError> {
    use crate::AgentLaunchError as E;
    let (provider, _) = crate::agent_launch::provider(directory, launch, policy)?;
    let digest = crate::provider_configuration::configuration_digest(
        &crate::encode_model(&provider).map_err(|e| E::caused("planning-provider-encoding", e))?,
    );
    // Reconstruct the readonly role first. Resolving afterwards keeps
    // requested effort/model without discarding those choices again.
    let readonly = crate::prepare_assistant_provider(&provider)
        .map_err(|e| E::caused("planning-provider-assistant-unavailable", e))?;
    let resolved = crate::provider_model_route::resolve_configuration_metadata(
        &readonly,
        launch,
        &settings.values,
        policy,
    )?;
    let mut route = resolved.route().cloned().map(Box::new);
    if let Some(route) = &mut route {
        route.reason =
            "Politique fournisseur enregistrée · planning · demande non authentifiée".into();
    }
    Ok((digest, route))
}
