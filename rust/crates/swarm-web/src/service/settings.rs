//! Native Admin settings routes reuse Root's saved validation, freshness and history.
use super::{close, root, HttpServiceError, NativeHttpService, RecordedConfiguration, RouteBody};
use http::{Method, Request};
use swarm_engine::{
    EngineSettingsCatalog, EngineSettingsChange, EngineSettingsDocument, RootStore, RootStoreError,
    SystemEnvironment,
};
/// Server-selected supported operation, never deserialized as caller authority.
#[derive(Clone, Copy)]
pub(super) enum SettingsOperation {
    Show,
    Describe,
    Preview,
    Apply,
}
#[derive(serde::Serialize)]
struct AppliedSettings {
    document: EngineSettingsDocument,
    replayed: bool,
}
impl NativeHttpService {
    /// Gate exact methods, strictly decode bounded native DTO then reopen the
    /// configured Root and check its retained descriptor before settings access.
    /// Preview has no publication; apply/replay delegates revision/token/history
    /// to Root, captures known state before encoding and preserves it through close.
    /// No default adoption, Work reset, provider call or network account proof.
    pub(super) fn engine_settings(
        &self,
        operation: SettingsOperation,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, HttpServiceError> {
        let post = matches!(
            operation,
            SettingsOperation::Preview | SettingsOperation::Apply
        );
        if (post && request.method() != Method::POST)
            || (!post && request.method() != Method::GET && request.method() != Method::HEAD)
        {
            return Err(HttpServiceError::Method(if post {
                "POST"
            } else {
                "GET, HEAD"
            }));
        }
        let change: Option<EngineSettingsChange> = if post {
            swarm_engine::check_command_input_size(request.body())
                .map_err(HttpServiceError::Input)?;
            Some(serde_json::from_slice(request.body()).map_err(HttpServiceError::Json)?)
        } else {
            None
        };
        let store = RootStore::open(&self.root, matches!(operation, SettingsOperation::Apply))
            .map_err(|e| root("open", e))?;
        let mut recorded = false;
        let mut configuration = None;
        let result = (|| {
            self.check_binding(&store)?;
            match operation {
                SettingsOperation::Describe => {
                    let catalog = EngineSettingsCatalog::installation()
                        .map_err(|e| root("settings-catalogue", RootStoreError::Settings(e)))?;
                    swarm_engine::encode_model(&catalog)
                        .map(RouteBody::plain)
                        .map_err(HttpServiceError::Model)
                }
                SettingsOperation::Show => {
                    let document = store.settings().map_err(|e| root("settings-read", e))?;
                    swarm_engine::encode_model(&document)
                        .map(RouteBody::plain)
                        .map_err(HttpServiceError::Model)
                }
                SettingsOperation::Preview => {
                    let change = change
                        .as_ref()
                        .ok_or(HttpServiceError::Refused("web-settings-input-missing"))?;
                    let preview = store
                        .preview_settings(change)
                        .map_err(|e| root("settings-preview", e))?;
                    swarm_engine::encode_model(&preview)
                        .map(RouteBody::plain)
                        .map_err(HttpServiceError::Model)
                }
                SettingsOperation::Apply => {
                    let change = change
                        .as_ref()
                        .ok_or(HttpServiceError::Refused("web-settings-input-missing"))?;
                    let (document, replayed) = store
                        .apply_settings(change, &mut SystemEnvironment)
                        .map_err(|e| root("settings-apply", e))?;
                    recorded = true;
                    let entry = document
                        .history
                        .iter()
                        .find(|entry| entry.change.event_id == change.event_id)
                        .ok_or(HttpServiceError::Refused("web-settings-receipt-missing"))?;
                    configuration = Some(RecordedConfiguration {
                        event_id: entry.change.event_id.clone(),
                        event_revision: entry.revision,
                        current_revision: document.revision,
                        replayed,
                    });
                    swarm_engine::encode_model(&AppliedSettings { document, replayed })
                        .map(|bytes| RouteBody { auxiliary_effects:Vec::new(), provider_probe:None, preparation_receipt:None, trust_publication:None, observation_causes: Vec::new(),
                            bytes,
                            recorded_work: None,
                            configuration_recorded: true,
                            recorded_configuration: configuration.clone(),
                        })
                        .map_err(HttpServiceError::Model)
                }
            }
        })();
        close(result, store.close()).map_err(|primary| {
            if let Some(configuration) = configuration {
                HttpServiceError::ConfigurationRecorded {
                    primary: Box::new(primary),
                    configuration,
                }
            } else if recorded || primary.configuration_may_be_recorded() {
                HttpServiceError::ConfigurationState {
                    primary: Box::new(primary),
                }
            } else {
                primary
            }
        })
    }
}
