//! Protected session consultation write and explicit native readback extension.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
/// One explicit Work; identities/time/freshness never come from query aliases.
/// Unknown, duplicate, absent or invalid keys refuse before Root IO.
fn query(raw: Option<&str>) -> Result<String, E> {
    let mut work = None;
    for (key, value) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        if key != "work" || work.replace(value.into_owned()).is_some() {
            return Err(E::Refused("visit-query-invalid"));
        }
    }
    work.filter(|v| swarm_identity_contract::name(v.as_bytes()))
        .ok_or(E::Refused("visit-query-invalid"))
}
/// Encode a server-owned response document. Encoding failures retain their
/// typed cause; this helper neither reads Root nor infers any write outcome.
/// The caller separately preserves receipts and checked Root close failures.
fn response_json(value: &impl serde::Serialize) -> Result<Vec<u8>, E> {
    serde_json::to_vec(value).map_err(E::ResponseJson)
}
impl NativeHttpService {
    /// Historical POST /visit remains POST-only and accepts an empty legacy body;
    /// an optional strict schema1 decimal expected_revision adds native fencing.
    /// GET/HEAD /visit-state is a separate native extension. Normal Host/session/
    /// CSRF/admission and connected Root binding remain authoritative. Only the
    /// server operator's bookmark can change; no actor/time/grant is accepted.
    /// Preserve typed receipts through JSON/Root close and owner transport/peer
    /// failures, never infer no write from Err. No Work/task/provider mutation.
    pub(super) fn session_visit(
        &self,
        request: &Request<Vec<u8>>,
        mark: bool,
    ) -> Result<RouteBody, E> {
        if mark && *request.method() != Method::POST {
            return Err(E::Method("POST"));
        }
        if !mark && !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !mark && !request.body().is_empty() {
            return Err(E::Refused("visit-body-invalid"));
        }
        let work = query(request.uri().query())?;
        let input = if request.body().is_empty() {
            swarm_engine::VisitMarkInput::historical()
        } else {
            swarm_engine::decode_visit_mark(request.body()).map_err(|e| {
                root(
                    "visit-input",
                    swarm_engine::RootStoreError::Launch(Box::new(e)),
                )
            })?
        };
        let store = swarm_engine::RootStore::open(&self.root, mark).map_err(|e| root("open", e))?;
        let mut effect = None;
        let result = (|| {
            self.check_binding(&store)?;
            if mark {
                let value = store
                    .mark_session_visit(&work, &input, &mut swarm_engine::SystemEnvironment)
                    .map_err(|e| root("visit-mark", e))?;
                effect = Some(value.effect.clone());
                let bytes = response_json(&value)?;
                let mut body = RouteBody::plain(bytes);
                body.auxiliary_effects.push(value.effect);
                Ok(body)
            } else {
                let value = store
                    .session_visit(&work, &mut swarm_engine::SystemEnvironment)
                    .map_err(|e| root("visit-read", e))?;
                response_json(&value).map(RouteBody::plain)
            }
        })();
        let result = close(result, store.close());
        match (result, effect) {
            (Err(primary), Some(effect)) => Err(E::AuxiliaryState {
                primary: Box::new(primary),
                effect,
            }),
            (result, _) => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    /// An intentionally unencodable fixture establishes the response boundary,
    /// not a claim that the current fixed visit DTO can actually fail encoding.
    struct Unencodable;
    impl serde::Serialize for Unencodable {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("fixture-response-encoding"))
        }
    }
    #[test]
    fn response_encoding_failure_is_server_error_and_retains_cause() {
        let error = response_json(&Unencodable).unwrap_err();
        let outcome = super::super::NativeJobOutcome::from_error(error);
        assert_eq!(outcome.status, http::StatusCode::INTERNAL_SERVER_ERROR);
        assert!(outcome.auxiliary_effects.is_empty());
        assert!(outcome
            .failure
            .as_ref()
            .unwrap()
            .source()
            .unwrap()
            .is::<serde_json::Error>());
    }
}
