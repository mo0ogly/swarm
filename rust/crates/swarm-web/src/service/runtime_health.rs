//! Protected storage health of the original server-bound project, no SQL read.
use super::{mission, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
impl NativeHttpService {
    /// GET/HEAD, empty body/query, normal session/admission. Reuse the server's
    /// held Root descriptors and saved thresholds; no new SQLite connection is
    /// needed. Safe ancillary codes are in the DTO and original causes survive
    /// encoding and the existing owner-observed transport. This does not assert
    /// SQL integrity, actual launch eligibility or business completion. No
    /// provider, mission, settings, process or lease mutation is authorized here.
    pub(super) fn runtime_health(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() {
            return Err(E::Refused("runtime-health-body-invalid"));
        }
        if request.uri().query().is_some_and(|v| !v.is_empty()) {
            return Err(E::Refused("runtime-health-query-invalid"));
        }
        let read = self
            .binding
            .runtime_health(&self.root, &mut swarm_engine::SystemEnvironment)
            .map_err(|e| root("runtime-health-observe", e))?;
        match serde_json::to_vec(&read.view) {
            Ok(bytes) => {
                let mut body = RouteBody::plain(bytes);
                body.observation_causes = read.causes;
                Ok(body)
            }
            Err(error) => Err(mission::retain_observation_error(
                E::Json(error),
                read.causes,
            )),
        }
    }
}
