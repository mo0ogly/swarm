//! Protected filtered log page, recorded observations without execution rights.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
/// Parse native exact Work/Agent, signed canonical after and literal filters.
/// Missing after is protocol start0; no row count policy lives in this query.
/// Unknown/duplicate keys and invalid identities/counts refuse before Root IO.
fn query(raw: Option<&str>) -> Result<(String, String, String, String, i64), E> {
    let (mut work, mut agent, mut filter, mut kind, mut after) = (None, None, None, None, None);
    for (k, v) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        let slot = match k.as_ref() {
            "work" => &mut work,
            "agent" => &mut agent,
            "q" => &mut filter,
            "kind" => &mut kind,
            "after" => &mut after,
            _ => return Err(E::Refused("log-query-invalid")),
        };
        if slot.replace(v.into_owned()).is_some() {
            return Err(E::Refused("log-query-invalid"));
        }
    }
    let name = |v: Option<String>| {
        v.filter(|s| swarm_identity_contract::name(s.as_bytes()))
            .ok_or(E::Refused("log-query-invalid"))
    };
    let cursor = after
        .map(|v| {
            let n = v
                .parse::<i64>()
                .map_err(|_| E::Refused("log-query-invalid"))?;
            if n.to_string() != v {
                return Err(E::Refused("log-query-invalid"));
            }
            Ok(n)
        })
        .transpose()?
        .unwrap_or(0);
    Ok((
        name(work)?,
        name(agent)?,
        filter.unwrap_or_default(),
        kind.unwrap_or_default(),
        cursor,
    ))
}
impl NativeHttpService {
    /// Session/admission/Root-bound GET/HEAD only with empty body. Shared producer
    /// preserves scope, saved row choice, SQL snapshot, filter/retention metadata,
    /// exact native decimal cursor extension and close errors. No log/process/
    /// provider/permission mutation; a source failure is not empty success.
    pub(super) fn log_query(&self, r: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*r.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !r.body().is_empty() {
            return Err(E::Refused("log-query-body-invalid"));
        }
        let (work, agent, filter, kind, after) = query(r.uri().query())?;
        let store =
            swarm_engine::RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let v = store
                .query_logs(&work, &agent, &filter, &kind, after)
                .map_err(|e| root("log-query-read", e))?;
            serde_json::to_vec(&v)
                .map(RouteBody::plain)
                .map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
