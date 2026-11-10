//! Protected policy-only HTTP adapter; Root owns validation and file publication.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{decode_provider_policy_change, RootStore, RootStoreError};

impl NativeHttpService {
    /// GET/HEAD returns source-bound saved policies. POST dispatches the strictly
    /// decoded Go preview flag to the shared Root publisher; current source/token
    /// is required for writes. Common Host/session/CSRF and Root binding apply.
    /// Query, wrong method and read-body refuse before opening Root. No provider
    /// execution, settings event/revision, audit identity or retry is invented.
    /// Encoding/close failures preserve known/uncertain file publication; stored
    /// corrupt sources remain server errors, distinct from caller decode errors.
    /// A synchronized no-op is known prior configuration, not a new mutation;
    /// carry it through observer/serialization/close/output failures as on apply.
    pub(super) fn provider_policies(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        let post = request.method() == Method::POST;
        if !post && request.method() != Method::GET && request.method() != Method::HEAD {
            return Err(E::Method("GET, HEAD, POST"));
        }
        if request.uri().query().is_some() {
            return Err(E::Refused("provider-policy-query-invalid"));
        }
        let change = if post {
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
            Some(decode_provider_policy_change(request.body()).map_err(|cause|
                root("provider-policy-input", RootStoreError::Providers(cause)))?)
        } else {
            if !request.body().is_empty() {
                return Err(E::Refused("provider-policy-read-body-invalid"));
            }
            None
        };
        let writable = change.as_ref().is_some_and(|r| !r.preview_requested());
        let store = RootStore::open(&self.root, writable).map_err(|e| root("open", e))?;
        let mut configuration_known = false;
        let result = (|| {
            self.check_binding(&store)?;
            let bytes = if let Some(change) = change.as_ref() {
                let view = if change.preview_requested() {
                    store.preview_provider_policies(change).map_err(|e| root("provider-policy-preview", e))?
                } else {
                    store.apply_provider_policies(change).map_err(|e| root("provider-policy-apply", e))?
                };
                configuration_known = view.applied;
                serde_json::to_vec(&view).map_err(E::ResponseJson)?
            } else {
                let view = store.provider_policies().map_err(|e| root("provider-policy-read", e))?;
                serde_json::to_vec(&view).map_err(E::ResponseJson)?
            };
            Ok(RouteBody { auxiliary_effects:Vec::new(), provider_probe:None, bytes, configuration_recorded: configuration_known, recorded_work: None,
                recorded_configuration: None, trust_publication: None,
                preparation_receipt: None, observation_causes: Vec::new() })
        })();
        close(result, store.close()).map_err(|primary| {
            if configuration_known || primary.configuration_may_be_recorded() {
                E::ConfigurationState { primary: Box::new(primary) }
            } else { primary }
        })
    }
}

/// Classify only explicit submitted-policy refusals from the request-validation
/// phase. The same code during a saved read, nested cause, catalogue IO, file cap
/// or cleanup is not caller input and remains a server failure. No IO or retry.
pub(super) fn caller_policy_refusal(operation: &str, cause: &RootStoreError) -> bool {
    matches!(operation, "provider-policy-preview" | "provider-policy-apply")
        && matches!(cause, RootStoreError::Launch(error) if matches!(error.as_ref(),
            swarm_engine::AgentLaunchError::Refused(
                "agent-model-policy-invalid" | "agent-model-policy-levels-required"
                | "agent-model-policy-name-v1-invalid" | "agent-model-level-unknown"
                | "agent-model-choice-not-announced")))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// An invalid requested policy is caller input; the same stored read failure
    /// and IO/cleanup remain server errors. Typed controls precede each refusal.
    #[test]
    fn policy_content_status_preserves_input_and_stored_error_boundary() {
        use swarm_engine::{AgentLaunchError, ProviderConfigurationError};
        let invalid = || RootStoreError::Launch(Box::new(AgentLaunchError::Refused("agent-model-policy-invalid")));
        assert_eq!(root("provider-policy-input", RootStoreError::Providers(ProviderConfigurationError::Refused("provider-policy-request-invalid"))).status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(root("provider-policy-read", invalid()).status(), http::StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(root("provider-policy-preview", invalid()).status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(root("provider-policy-apply", invalid()).status(), http::StatusCode::BAD_REQUEST);
        assert_eq!(root("provider-policy-preview", RootStoreError::Providers(ProviderConfigurationError::Refused("provider-configuration-invalid"))).status(), http::StatusCode::INTERNAL_SERVER_ERROR);
    }
}
