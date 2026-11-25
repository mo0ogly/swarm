//! Read-only orchestration metadata under the existing Journals permission.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;

impl NativeHttpService {
    /// Read one addressed Work with the existing session and Journals gates.
    /// Reject extra keys, unsafe IDs and bodies before opening the bound Root.
    /// Return explicit read and codec failures; do not mutate planning or logs.
    pub(super) fn orchestration_signals(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        let mut query = form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes());
        let (key, work) = query
            .next()
            .ok_or(E::Refused("orchestration-signal-query-invalid"))?;
        if key != "work"
            || query.next().is_some()
            || !request.body().is_empty()
            || !swarm_identity_contract::name(work.as_bytes())
        {
            return Err(E::Refused("orchestration-signal-query-invalid"));
        }
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let value = store
                .orchestration_signals(&work)
                .map_err(|e| root("orchestration-signals", e))?;
            serde_json::to_vec(&value)
                .map(RouteBody::plain)
                .map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
