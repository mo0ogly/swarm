//! Native structural advice through the existing mission read/write permissions.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use serde::Deserialize;
use swarm_engine::RootStore;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Approval {
    revision: String,
    proposal: String,
}

impl NativeHttpService {
    /// Read the current assessment or record explicit approval under normal
    /// session/CSRF and mission-write gates. Bind the same Root before use.
    /// A conflicted or uncertain submission is resolved by reading the receipt.
    pub(super) fn workflow_architecture(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        let write = request.method() == Method::POST;
        if !write && !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD, POST"));
        }
        let mut params = form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes());
        let (key, id) = params
            .next()
            .ok_or(E::Refused("workflow-architecture-work-invalid"))?;
        if key != "work" || params.next().is_some() || !swarm_identity_contract::name(id.as_bytes())
        {
            return Err(E::Refused("workflow-architecture-work-invalid"));
        }
        let approval: Option<Approval> = if write {
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
            Some(
                serde_json::from_slice(request.body())
                    .map_err(|_| E::Refused("workflow-architecture-approval-invalid"))?,
            )
        } else {
            if !request.body().is_empty() {
                return Err(E::Refused("workflow-architecture-work-invalid"));
            }
            None
        };
        let store = RootStore::open(&self.root, write).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let approved = approval
                .as_ref()
                .map(|a| (a.revision.as_str(), a.proposal.as_str()));
            let value = store
                .workflow_architecture(&id, approved)
                .map_err(|e| root("workflow-architecture", e))?;
            serde_json::to_vec(&value)
                .map(RouteBody::plain)
                .map_err(E::ResponseJson)
        })();
        close(result, store.close())
    }
}
