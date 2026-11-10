//! Read-only CLI provider inventory; model defaults are never invented.
use super::{finished, RootStore, RootStoreError as E};
use crate::{ModelPolicy, ProviderModelOption, ProviderPolicyExport};
use serde::Serialize;
use std::{collections::BTreeMap, os::unix::fs::PermissionsExt, path::Path};

/// Go Admin row plus explicit metadata state. Command path is intentionally
/// visible to the authenticated local operator; arguments/env/API keys are not.
/// Health observes mode bits only, not ACL, execution, account or model access.
#[derive(Serialize)]
pub struct ProviderAdministrationEntry {
    pub adapter: String,
    pub command: String,
    pub policy: Option<ModelPolicy>,
    pub models: Vec<ProviderModelOption>,
    pub saved: bool,
    pub health: &'static str,
    pub health_issue: Option<&'static str>,
    pub assistant_unavailable: String,
    pub model_options_state: &'static str,
}
/// One checked CLI source snapshot, with portable saved policies only. Empty
/// export can occur when no policy is saved and is not approval of an empty
/// import. No implicit vendor policy or model/effort list is supplied.
#[derive(Serialize)]
pub struct ProviderAdministration {
    pub version: i64,
    pub digest: String,
    pub providers: BTreeMap<String, ProviderAdministrationEntry>,
    pub export: ProviderPolicyExport,
    pub scope: &'static str,
    pub provider_execution_started: bool,
}
/// Metadata value and original per-row unavailable-command/helper causes.
/// Consumers must retain causes on later serialization/close/output errors.
/// These observations grant no account, execution or effective isolation proof.
pub struct ProviderAdministrationObservation {
    pub value: ProviderAdministration,
    pub causes: Vec<E>,
}
impl RootStore {
    /// Read a source-bound CLI inventory under the shared settings reservation.
    /// Saved policy shape and local catalogue errors refuse with typed causes.
    /// Missing/nonexecutable/relative command or unavailable tool-free adapter
    /// is an explicit row observation with original causes retained, not a ready
    /// claim or successful empty fallback. This does not merge API connections;
    /// effective selection still uses resolve_provider_model's namespace guard.
    /// No process, Agent/Work write, cost reservation, key or argument export.
    pub fn provider_administration(&self) -> Result<ProviderAdministrationObservation, E> {
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let mut causes = Vec::new();
        let result = (|| {
            let source = self.provider_configuration()?;
            let settings = self.read_settings()?;
            let policy = settings.values.agent_launch_policy().map_err(E::Settings)?;
            let mut providers = BTreeMap::new(); let mut saved = BTreeMap::new();
            for (name, provider) in source.configuration.providers.unwrap_or_default() {
                if let Some(policy) = provider.model_policy.as_deref() {
                    crate::provider_model_route::policy_levels(policy).map_err(|e| E::Launch(Box::new(e)))?;
                    saved.insert(name.clone(), policy.clone());
                }
                let models = crate::provider_model_route::model_options(&provider, &policy)
                    .map_err(|e| E::Launch(Box::new(e)))?;
                let (health, health_issue) = command_health(&provider.command, &mut causes);
                let assistant_unavailable = match crate::prepare_assistant_provider(&provider) {
                    Ok(_) => String::new(),
                    Err(error) => {
                        let code = error.code(); causes.push(E::Launch(Box::new(crate::AgentLaunchError::caused(code, error))));
                        code.into()
                    }
                };
                let state = if models.is_empty() { "no_local_model_declaration" } else { "declared_unverified" };
                providers.insert(name, ProviderAdministrationEntry {
                    adapter: crate::provider_model_route::adapter(&provider).into(),
                    command: provider.command.clone(), policy: provider.model_policy.as_deref().cloned(),
                    models, saved: provider.model_policy.is_some(), health, health_issue,
                    assistant_unavailable, model_options_state: state,
                });
            }
            Ok(ProviderAdministration { version: 1, digest: source.digest, providers,
                export: ProviderPolicyExport { version: 1, policies: saved },
                scope: "cli_operator_metadata_only_no_admission", provider_execution_started: false })
        })();
        match finished(result, reservation.close().map_err(E::Storage)) {
            Ok(value) => Ok(ProviderAdministrationObservation { value, causes }),
            Err(primary) => causes.into_iter().fold(Err(primary), |result, cause| finished(result, Err(cause))),
        }
    }
}
/// Observe only an absolute configured command's file kind/mode bits. Expected
/// unavailability is a row state with retained typed cause; no executable runs
/// and no path is formatted into diagnostics. Observations are not a lease/ACL.
fn command_health(command: &str, causes: &mut Vec<E>) -> (&'static str, Option<&'static str>) {
    let result = if !Path::new(command).is_absolute() {
        Err(E::Refused("provider-admin-command-absolute-required"))
    } else {
        std::fs::metadata(command)
            .map_err(|error| E::Launch(Box::new(crate::AgentLaunchError::caused("provider-admin-command-metadata", error))))
            .and_then(|metadata| if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 { Ok(()) }
                else { Err(E::Refused("provider-admin-command-not-executable-file")) })
    };
    match result {
        Ok(()) => ("Executable metadata observed; authentication and model unverified", None),
        Err(cause) => { let code = cause.code(); causes.push(cause); ("Executable metadata unavailable", Some(code)) }
    }
}
