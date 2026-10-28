//! Protected log/terminal display read policy; shared authoritative Root policy.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{AgentLaunchError, RootStore, RootStoreError};
impl NativeHttpService {
    /// Read/HEAD or preview/apply with common session/CSRF/Root custody. No network
    /// or mission mutation. Applied/replayed state survives encode/output/close.
    pub(super) fn log_read_policy(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        let path = request.uri().path();
        let write = path != "/api/v1/log-read-policy";
        if (write && request.method() != Method::POST)
            || (!write && request.method() != Method::GET && request.method() != Method::HEAD)
        {
            return Err(E::Method(if write { "POST" } else { "GET, HEAD" }));
        }
        if request.uri().query().is_some() {
            return Err(E::Refused("log-read-policy-query-invalid"));
        }
        let change = if write {
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
            Some(
                swarm_engine::decode_api_log_read_change(request.body()).map_err(|e| {
                    root("log-read-policy-input", RootStoreError::Launch(Box::new(e)))
                })?,
            )
        } else {
            if !request.body().is_empty() {
                return Err(E::Refused("log-read-policy-body-invalid"));
            }
            None
        };
        let apply = path.ends_with("/apply");
        let store = RootStore::open(&self.root, apply).map_err(|e| root("open", e))?;
        let mut known = false;
        let result = (|| {
            self.check_binding(&store)?;
            let bytes = if let Some(r) = change.as_ref() {
                let view = if apply {
                    store.apply_log_read_policy(r)
                } else {
                    store.preview_log_read_policy(r)
                }
                .map_err(|e| root("log-read-policy-change", e))?;
                known = view.applied;
                serde_json::to_vec(&view).map_err(E::ResponseJson)?
            } else {
                {
                    let view = store
                        .log_read_policy()
                        .map_err(|e| root("log-read-policy-read", e))?;
                    serde_json::to_vec(&view).map_err(E::ResponseJson)?
                }
            };
            let mut body = RouteBody::plain(bytes);
            body.configuration_recorded = known;
            Ok(body)
        })();
        close(result, store.close()).map_err(|primary| {
            if known || primary.configuration_may_be_recorded() {
                E::ConfigurationState {
                    primary: Box::new(primary),
                }
            } else {
                primary
            }
        })
    }
}
/// Classify submitted data only; stored/source/settings/cleanup causes remain500.
pub(super) fn caller_refusal(op: &str, e: &RootStoreError) -> bool {
    if op == "log-read-policy-input" {
        return matches!(e, RootStoreError::Launch(_));
    }
    if op != "log-read-policy-change" {
        return false;
    }
    matches!(
        e,
        RootStoreError::Refused("log-read-policy-file-outside-saved-policy")
    ) || matches!(e,RootStoreError::Launch(v) if matches!(v.as_ref(),AgentLaunchError::Refused("log-read-change-invalid"|"log-read-values-invalid")))
}
