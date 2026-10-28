//! Protected redacted API connection reads and source-bound local publication.
use super::{close,root,HttpServiceError as E,NativeHttpService,RouteBody};
use http::{Method,Request};
use swarm_engine::{RootStore,RootStoreError,AgentLaunchError};
impl NativeHttpService {
    /// GET/HEAD inventory or POST explicit preview/application. Common Host,
    /// session, CSRF and connected-Root binding apply. Strict input/cap before
    /// Root; no key is returned. Persisted limits, CAS/token, lock/snapshot/atomic
    /// publication live in Root and are shared with CLI. No provider contact or
    /// mission mutation. Known/uncertain file effects survive JSON/close/output
    /// errors, including no-op durability confirmation; reread before recovery.
    pub(super) fn api_connections(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E> {
        let post=request.method()==Method::POST;
        if !post && request.method()!=Method::GET && request.method()!=Method::HEAD {return Err(E::Method("GET, HEAD, POST"));}
        if request.uri().query().is_some() {return Err(E::Refused("api-connections-query-invalid"));}
        let change=if post {
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
            Some(swarm_engine::decode_api_connection_change(request.body()).map_err(|e|root("api-connection-input",RootStoreError::Launch(Box::new(e))))?)
        } else {if !request.body().is_empty(){return Err(E::Refused("api-connections-read-body-invalid"));}None};
        let store=RootStore::open(&self.root,change.as_ref().is_some_and(|r|!r.preview_requested())).map_err(|e|root("open",e))?;
        let mut changed=false;
        let result=(|| {
            self.check_binding(&store)?;
            let bytes=if let Some(change)=change.as_ref() {
                let view=if change.preview_requested(){store.preview_api_connection(change).map_err(|e|root("api-connection-preview",e))?}
                    else{store.apply_api_connection(change).map_err(|e|root("api-connection-apply",e))?};
                changed=view.applied;
                serde_json::to_vec(&view).map_err(E::ResponseJson)?
            }else{serde_json::to_vec(&store.api_connections().map_err(|e|root("api-connections-read",e))?).map_err(E::ResponseJson)?};
            let mut body=RouteBody::plain(bytes);body.configuration_recorded=changed;Ok(body)
        })();
        close(result,store.close()).map_err(|primary|if changed || primary.configuration_may_be_recorded(){E::ConfigurationState{primary:Box::new(primary)}}else{primary})
    }
}
/// Only submitted representation/destination/source-policy refusals classify as
/// caller errors, within input/preview/apply operations. Stored decode/read/IO/
/// settings/cleanup causes remain server errors, never an empty successful view.
pub(super) fn caller_refusal(operation:&str,cause:&RootStoreError)->bool {
    if operation=="api-connection-input" {return matches!(cause,RootStoreError::Launch(e) if matches!(e.as_ref(),AgentLaunchError::Refused("api-connection-change-invalid")|AgentLaunchError::Cause{code:"api-connection-change-invalid",..}));}
    matches!(operation,"api-connection-preview"|"api-connection-apply"|"api-connection-probe") && (
        matches!(cause,RootStoreError::Refused("api-connection-key-destination-replacement-required"|"api-connection-cli-name-conflict"|"api-connection-file-outside-saved-policy"|"api-connection-preview-not-application"|"api-connection-preview-required")) ||
        matches!(cause,RootStoreError::Launch(e) if matches!(e.as_ref(),AgentLaunchError::Refused("api-connection-change-invalid"|"api-connection-id-invalid"|"api-connection-label-model-invalid"|"api-connection-endpoint-invalid")|AgentLaunchError::Cause{code:"api-connection-endpoint-invalid"|"api-connection-key-invalid",..})))
}
