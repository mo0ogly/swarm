//! Protected global Admin budget routes. No advice process is started here.
use super::{close, root, HttpServiceError, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{RootStore, SystemEnvironment};
/// Server-selected operation, never request-supplied authority.
#[derive(Clone, Copy)]
pub(super) enum Operation {
    Show,
    Preview,
    Apply,
}
impl NativeHttpService {
    /// Exact method/body handling behind shared session/Host/CSRF admission.
    /// Reopens and checks Root binding, delegates all validation/CAS/history to
    /// the engine and preserves known/uncertain publication through encode/close.
    /// Budget reservations, providers and paid calls cannot be started by this route.
    pub(super) fn admin_budget(
        &self,
        operation: Operation,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, HttpServiceError> {
        let post = !matches!(operation, Operation::Show);
        if post && request.method() != Method::POST
            || !post && request.method() != Method::GET && request.method() != Method::HEAD
        {
            return Err(HttpServiceError::Method(if post {
                "POST"
            } else {
                "GET, HEAD"
            }));
        }
        if post {
            swarm_engine::check_command_input_size(request.body())
                .map_err(HttpServiceError::Input)?
        }
        let mut store = RootStore::open(&self.root, matches!(operation, Operation::Apply))
            .map_err(|error| root("open", error))?;
        let mut known = false;
        let result = (|| {
            self.check_binding(&store)?;
            let bytes = match operation {
                Operation::Show => swarm_engine::encode_model(
                    &store
                        .admin_advice_budget()
                        .map_err(|error| root("admin-budget-read", error))?,
                ),
                Operation::Preview => swarm_engine::encode_model(
                    &store
                        .preview_admin_advice_budget(request.body())
                        .map_err(|error| root("admin-budget-preview", error))?,
                ),
                Operation::Apply => {
                    let reply = store
                        .apply_admin_advice_budget(request.body(), &mut SystemEnvironment)
                        .map_err(|error| root("admin-budget-apply", error))?;
                    known = true;
                    swarm_engine::encode_model(&reply)
                }
            }
            .map_err(HttpServiceError::Model)?;
            Ok(RouteBody { preparation_receipt:None, trust_publication:None, observation_causes: Vec::new(),
                bytes,
                recorded_work: None,
                configuration_recorded: known,
                recorded_configuration: None,
            })
        })();
        close(result, store.close()).map_err(|primary| {
            if known || primary.configuration_may_be_recorded() {
                HttpServiceError::ConfigurationState {
                    primary: Box::new(primary),
                }
            } else {
                primary
            }
        })
    }
}
