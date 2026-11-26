//! Protected independent reviewer configuration on the bound Root.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{ReviewerModelView, RootStore, RootStoreError, SystemEnvironment};

impl NativeHttpService {
    /// Decode a single mission key, then use the shared no-exec preview or the
    /// atomic Work operation. Any known committed revision survives output errors.
    pub(super) fn reviewer_model(&self, apply: bool, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if request.method() != Method::POST {
            return Err(E::Method("POST"));
        }
        let query = form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes());
        let mut pairs = query.into_owned();
        let (key, work) = pairs.next().ok_or(E::Refused("reviewer-model-query-invalid"))?;
        if key != "work" || !swarm_identity_contract::name(work.as_bytes()) || pairs.next().is_some() {
            return Err(E::Refused("reviewer-model-query-invalid"));
        }
        swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        let mut store = RootStore::open(&self.root, apply).map_err(|e| root("open", e))?;
        let mut recorded = None;
        let result = (|| {
            self.check_binding(&store)?;
            let view = if apply {
                let outcome = store.execute_external(&work, "reviewer.model", request.body(), &mut SystemEnvironment)
                    .map_err(|e| root("reviewer-model-apply", e))?;
                let current = outcome.result.work;
                recorded = Some((current.id.clone(), current.revision));
                ReviewerModelView::from_work(&current)
                    .map_err(|e| root("reviewer-model-read", RootStoreError::Launch(Box::new(e))))?
            } else {
                store.preview_reviewer_model(&work, request.body(), &mut SystemEnvironment)
                    .map_err(|e| root("reviewer-model-preview", e))?
            };
            let bytes = swarm_engine::encode_model(&view).map_err(E::Model)?;
            Ok(RouteBody { task_review_timing: None, auxiliary_effects: Vec::new(), provider_probe: None,
                preparation_receipt: None, trust_publication: None, observation_causes: Vec::new(),
                bytes, recorded_work: recorded.clone(), configuration_recorded: false,
                recorded_configuration: None })
        })();
        match (close(result, store.close()), recorded) {
            (Err(primary), Some((work, revision))) => Err(E::Recorded { primary: Box::new(primary), work, revision }),
            (result, _) => result,
        }
    }
}
