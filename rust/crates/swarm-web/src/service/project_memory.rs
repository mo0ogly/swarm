//! Protected versioned-memory catalog read (Go `/api/v1/project-memory` GET).
//! Mutations and the M20 summary-source operation stay unported here: this
//! module serves the read-only catalog surface only, refusing the POST paths
//! with the typed unported code instead of inventing behavior.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::RootStore;

/// Accept only one `q` member (possibly empty), refuse unknown or repeated
/// members. Mirrors Go's `r.URL.Query().Get("q")` observable contract for the
/// supported shapes while staying strict on ambiguity.
fn catalog_query(raw: Option<&str>) -> Result<String, E> {
    let mut query = None;
    for (k, v) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        if k != "q" || query.replace(v.into_owned()).is_some() {
            return Err(E::Refused("project-memory-query-invalid"));
        }
    }
    Ok(query.unwrap_or_default())
}

impl NativeHttpService {
    /// GET/HEAD catalog read with an optional folded literal `q` filter; empty
    /// body, readonly Root, shared scoped session and datastore binding precede
    /// the guarded catalog read. Go's response shape (`entries`, `method`,
    /// `query`) is preserved by the engine type. A read/encode/close failure
    /// refuses — never an empty success. POST paths stay refused as unported:
    /// the mutation surface and M20 summary-source are later G2 increments.
    pub(super) fn project_memory(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if request.method() == Method::POST {
            return Err(E::Refused("project-memory-operation-unported"));
        }
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() {
            return Err(E::Refused("project-memory-query-invalid"));
        }
        let query = catalog_query(request.uri().query())?;
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let catalog = store.memory_catalog(&query).map_err(|e| root("project-memory-read", e))?;
            serde_json::to_vec(&catalog)
                .map(RouteBody::plain)
                .map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
