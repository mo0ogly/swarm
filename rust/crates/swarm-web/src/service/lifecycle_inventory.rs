//! Protected readonly lifecycle metadata; no archive/restore/delete capability.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;
impl NativeHttpService {
    /// Read the complete current lifecycle inventory through the retained Root
    /// binding and shared admission. Only GET/HEAD, no body or query parameters.
    /// Root owns one SQL snapshot and physical/settings checks; response data
    /// grants no restore, deletion or lifecycle authority. Preserve original
    /// read/encode/close causes, with no partial/empty fallback or mutation.
    pub(super) fn lifecycle_inventory(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() || request.uri().query().is_some_and(|q| !q.is_empty()) {
            return Err(E::Refused("lifecycle-inventory-input-invalid"));
        }
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let value = store
                .lifecycle_missions()
                .map_err(|e| root("lifecycle-inventory-read", e))?;
            serde_json::to_vec(&value)
                .map(RouteBody::plain)
                .map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
