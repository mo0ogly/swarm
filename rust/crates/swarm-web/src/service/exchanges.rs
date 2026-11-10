//! Protected Work-scoped historical exchanges. No dispatch/ACK/identity grant.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
/// Require one nonempty Work name, no ambiguous/unknown query member. This is
/// native input validation, not the older Go first-duplicate query behavior.
fn query(raw: Option<&str>) -> Result<String, E> {
    let mut work = None;
    for (k, v) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        if k != "work" || work.replace(v.into_owned()).is_some() {
            return Err(E::Refused("exchange-query-invalid"));
        }
    }
    work.filter(|v| swarm_identity_contract::name(v.as_bytes()))
        .ok_or(E::Refused("exchange-query-invalid"))
}
impl NativeHttpService {
    /// GET/HEAD only with empty body and one Work. Session/admission remain common;
    /// bind the opened readonly datastore before its snapshot read. Go array shape,
    /// chronological order, omitempty and nil artifacts are preserved. Stored text
    /// is data for escaped rendering, never delivery or recipient authority.
    /// Input/read/encode/rollback/close failures remain visible; no empty success.
    pub(super) fn exchanges(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() {
            return Err(E::Refused("exchange-query-invalid"));
        }
        let work = query(request.uri().query())?;
        let store =
            swarm_engine::RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let rows = store
                .agent_exchanges(&work)
                .map_err(|e| root("exchange-read", e))?;
            serde_json::to_vec(&rows)
                .map(RouteBody::plain)
                .map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
