//! Protected mission quota routes using the common Root transaction and policy.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{RootStore, SystemEnvironment};
/// Fixed route choice; client fields cannot select another mutation kind.
#[derive(Clone, Copy)]
pub(super) enum Operation {
    Show,
    Preview,
    Apply,
}
/// Require exactly one safe Work name; duplicate/unknown/empty URL keys refuse.
/// URL decoding supplies data only. No path, default Work or identity is inferred.
fn work_query(query: Option<&str>) -> Result<String, E> {
    let mut work = None;
    for (key, value) in form_urlencoded::parse(query.unwrap_or("").as_bytes()) {
        if key != "work" || work.replace(value.into_owned()).is_some() {
            return Err(E::Refused("quota-query-invalid"));
        }
    }
    work.filter(|w| swarm_identity_contract::name(w.as_bytes()))
        .ok_or(E::Refused("quota-query-invalid"))
}
impl NativeHttpService {
    /// Run behind shared session/Host/CSRF and configured Admin capacity. Exact
    /// method/query/body admission precedes opening the bound Root. Preview has
    /// no write; apply uses saved policy/revision/replay and preserves known Work
    /// through encoding/close failure. No provider, consumption reset, live owner
    /// takeover or authenticated human identity is introduced by this adapter.
    pub(super) fn quotas(
        &self,
        operation: Operation,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, E> {
        let post = !matches!(operation, Operation::Show);
        if post && request.method() != Method::POST
            || !post && request.method() != Method::GET && request.method() != Method::HEAD
        {
            return Err(E::Method(if post { "POST" } else { "GET, HEAD" }));
        }
        let work = work_query(request.uri().query())?;
        if post {
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        } else if !request.body().is_empty() {
            return Err(E::Refused("quota-read-body-invalid"));
        }
        let mut store = RootStore::open(&self.root, matches!(operation, Operation::Apply))
            .map_err(|e| root("open", e))?;
        let mut recorded = None;
        let result = (|| {
            self.check_binding(&store)?;
            let bytes = match operation {
                Operation::Show => swarm_engine::encode_model(
                    &store.quotas(&work).map_err(|e| root("quotas-read", e))?,
                ),
                Operation::Preview => swarm_engine::encode_model(
                    &store
                        .preview_quotas(&work, request.body(), &mut SystemEnvironment)
                        .map_err(|e| root("quotas-preview", e))?,
                ),
                Operation::Apply => {
                    let outcome = store
                        .execute_external(
                            &work,
                            "quotas.configure",
                            request.body(),
                            &mut SystemEnvironment,
                        )
                        .map_err(|e| root("quotas-apply", e))?;
                    let current = outcome.result.work;
                    recorded = Some((current.id.clone(), current.revision));
                    swarm_engine::encode_model(&current)
                }
            }
            .map_err(E::Model)?;
            Ok(RouteBody { preparation_receipt:None, trust_publication:None, observation_causes: Vec::new(),
                bytes,
                recorded_work: recorded.clone(),
                configuration_recorded: false,
                recorded_configuration: None,
            })
        })();
        match (close(result, store.close()), recorded) {
            (Err(primary), Some((work, revision))) => Err(E::Recorded {
                primary: Box::new(primary),
                work,
                revision,
            }),
            (result, _) => result,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Valid URL witness precedes ambiguous and traversal-shaped refusals.
    #[test]
    fn quota_work_query_refuses_ambiguity_without_default() {
        assert_eq!(work_query(Some("work=w-one")).unwrap(), "w-one");
        assert_eq!(work_query(Some("work=w%2Done")).unwrap(), "w-one");
        for query in [
            None,
            Some(""),
            Some("work="),
            Some("work=w&work=x"),
            Some("work=w&actor=engine"),
            Some("work=%2E%2E%2Fx"),
        ] {
            assert_eq!(work_query(query).unwrap_err().code(), "quota-query-invalid");
        }
    }
}
