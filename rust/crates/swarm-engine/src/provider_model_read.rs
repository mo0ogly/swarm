//! Read-only configured provider resolution; no execution/admission capability.
use super::{finished, RootStore, RootStoreError as E};
use crate::{Launch, ModelRoute};

impl RootStore {
    /// Resolve only configured CLI provider metadata using current saved routing
    /// rules and local declarations. Validate name/level/purpose before file IO.
    /// Automatic page/work defaults remain automatic under operator restrictions.
    /// Return None only for an existing supported custom CLI without a model
    /// policy; unknown providers and unresolved known defaults refuse explicitly.
    /// No commands/env/keys are exposed; no Agent/Work write, cost reservation or
    /// provider process is created. CLI/API namespace collisions refuse as at admission.
    /// API connection metadata remains unported. Schema, source, catalogue, lock
    /// and close errors retain causes; success proves no model/account identity.
    pub fn resolve_provider_model(
        &self,
        provider: &str,
        level: &str,
        purpose: &str,
    ) -> Result<Option<ModelRoute>, E> {
        if !swarm_identity_contract::name(provider.as_bytes())
            || !(matches!(level, "" | "auto") || crate::model_routing_policy::model_level(level))
            || !crate::provider_model_route::valid_purpose(purpose)
        {
            return Err(E::Refused("provider-model-request-invalid"));
        }
        let reservation = self.directory.reserve(false, false).map_err(E::Storage)?;
        let result = (|| {
            let settings = self.read_settings()?;
            let policy = settings.values.agent_launch_policy().map_err(E::Settings)?;
            let launch = Launch { provider: provider.into(), level: level.into(), ..Default::default() };
            // Same capped CLI/API namespace and disabled/collision semantics as
            // admission. The private source fingerprint is no lease and is not
            // needed by this ephemeral read; no secret bytes are returned.
            let (configured, _source_fingerprint) = crate::agent_launch::provider(
                &self.directory, &launch, &policy,
            ).map_err(|cause| E::Launch(Box::new(cause)))?;
            let resolved = crate::provider_model_route::resolve_configuration_metadata_for_purpose(
                &configured, &launch, &settings.values, &policy, purpose,
            ).map_err(|cause| E::Launch(Box::new(cause)))?;
            Ok(resolved.route().cloned())
        })();
        finished(result, reservation.close().map_err(E::Storage))
    }
}
