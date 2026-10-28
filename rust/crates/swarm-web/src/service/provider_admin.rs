//! Authenticated operator inventory; no secret or provider execution surface.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;

impl NativeHttpService {
    /// GET/HEAD an explicit CLI inventory under shared Host/session/Root binding.
    /// No query/body, probe, process or write. Attribute unavailable command/helper
    /// metadata with original causes. Fatal catalogue/read/encoding/close failures
    /// preserve both the primary and all earlier observation causes; they cannot
    /// become empty/ready success. Command paths are intentional operator data;
    /// args/env/API secrets remain omitted. Health never proves admission.
    pub(super) fn provider_admin(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if request.method() != Method::GET && request.method() != Method::HEAD { return Err(E::Method("GET, HEAD")); }
        if request.uri().query().is_some() { return Err(E::Refused("provider-admin-query-invalid")); }
        if !request.body().is_empty() { return Err(E::Refused("provider-admin-read-body-invalid")); }
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let mut causes = Vec::new();
        let result = (|| {
            self.check_binding(&store)?;
            let mut observation = store.provider_administration().map_err(|e| root("provider-admin-read", e))?;
            causes.append(&mut observation.causes);
            serde_json::to_vec(&observation.value).map(RouteBody::plain).map_err(E::ResponseJson)
        })();
        match close(result, store.close()) {
            Ok(mut body) => { body.observation_causes = causes; Ok(body) },
            Err(mut primary) => {
                for cause in causes { primary = E::Close { primary: Some(Box::new(primary)), cleanup: Box::new(cause) }; }
                Err(primary)
            }
        }
    }
}
