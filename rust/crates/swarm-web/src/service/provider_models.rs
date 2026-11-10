//! Protected configured-model metadata read, never a launch or availability test.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{AgentLaunchError, RootStore, RootStoreError};

/// Exact transport fields. Root authoritatively validates the finite ASCII
/// vocabulary/name before source IO, so lossy form decoding cannot admit a
/// replacement character as a valid provider, level or purpose. No path is read.
fn query(raw: Option<&str>) -> Result<(String, String, String), E> {
    let (mut provider, mut level, mut purpose) = (None, None, None);
    for (key, value) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        match key.as_ref() {
            "provider" if provider.is_none() => provider = Some(value.into_owned()),
            "level" if level.is_none() => level = Some(value.into_owned()),
            "purpose" if purpose.is_none() => purpose = Some(value.into_owned()),
            _ => return Err(E::Refused("provider-model-query-invalid")),
        }
    }
    Ok((provider.ok_or(E::Refused("provider-model-query-invalid"))?, level.unwrap_or_default(),
        purpose.ok_or(E::Refused("provider-model-query-invalid"))?))
}
impl NativeHttpService {
    /// GET/HEAD only, empty body and exact query shape. Common session/Host guard
    /// precedes readonly descriptor-bound Root checks. Return only the requested
    /// route (nullable custom/no-policy), preserving current operator defaults
    /// and automatic restrictions. No provider call, command, secret or admission.
    /// Source/IO/close failures remain failures rather than an empty route.
    pub(super) fn provider_model(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if request.method() != Method::GET && request.method() != Method::HEAD {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() { return Err(E::Refused("provider-model-read-body-invalid")); }
        let (provider, level, purpose) = query(request.uri().query())?;
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let route = store.resolve_provider_model(&provider, &level, &purpose)
                .map_err(|e| root("provider-model-read", e))?;
            serde_json::to_vec(&serde_json::json!({"route": route})).map(RouteBody::plain).map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
/// Only explicit request selection failures have client status. Malformed saved
/// policies/catalogues, IO and cleanup are not flattened into caller mistakes.
/// Saved routing denials are typed Cause entries from check_selection, not an
/// authentication grant. No cause is discarded or formatted by this predicate.
pub(super) fn selection_refusal(cause: &RootStoreError) -> bool {
    matches!(cause, RootStoreError::Refused("provider-model-request-invalid"))
        || matches!(cause, RootStoreError::Launch(error) if matches!(error.as_ref(),
            AgentLaunchError::Refused("agent-provider-not-configured" | "agent-model-level-unknown" | "agent-model-choice-not-announced"
                | "agent-provider-model-route-unported")
            | AgentLaunchError::Cause { code: "settings-model-automatic-level-denied"
                | "settings-model-restricted-level-denied", .. }))
}
