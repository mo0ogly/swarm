//! Protected compatible wait configuration; shared authoritative Root policy.
use super::{close,root,HttpServiceError as E,NativeHttpService,RouteBody};
use http::{Method,Request};
use swarm_engine::{RootStore,RootStoreError,AgentLaunchError};
impl NativeHttpService {
    /// Read/HEAD or preview/apply with common session/CSRF/Root custody. No network
    /// or mission mutation. Applied/replayed state survives encode/output/close.
    pub(super) fn provider_wait(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E> {
        let path=request.uri().path();let write=path!="/api/v1/provider-wait";
        if (write && request.method()!=Method::POST) || (!write && request.method()!=Method::GET && request.method()!=Method::HEAD){return Err(E::Method(if write{"POST"}else{"GET, HEAD"}));}
        if request.uri().query().is_some(){return Err(E::Refused("provider-wait-query-invalid"));}
        let change=if write {swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;Some(swarm_engine::decode_api_provider_wait_change(request.body()).map_err(|e|root("provider-wait-input",RootStoreError::Launch(Box::new(e))))?)}else{if !request.body().is_empty(){return Err(E::Refused("provider-wait-body-invalid"));}None};
        let apply=path.ends_with("/apply");let store=RootStore::open(&self.root,apply).map_err(|e|root("open",e))?;let mut known=false;
        let result=(|| {self.check_binding(&store)?;let bytes=if let Some(r)=change.as_ref(){let view=if apply{store.apply_provider_wait(r)}else{store.preview_provider_wait(r)}.map_err(|e|root("provider-wait-change",e))?;known=view.applied;serde_json::to_vec(&view).map_err(E::ResponseJson)?}else{serde_json::to_vec(&store.provider_wait().map_err(|e|root("provider-wait-read",e))?).map_err(E::ResponseJson)?};let mut body=RouteBody::plain(bytes);body.configuration_recorded=known;Ok(body)})();
        close(result,store.close()).map_err(|primary|if known || primary.configuration_may_be_recorded(){E::ConfigurationState{primary:Box::new(primary)}}else{primary})
    }
}
/// Classify submitted data only; stored/source/settings/cleanup causes remain500.
pub(super) fn caller_refusal(op:&str,e:&RootStoreError)->bool {
    (op=="provider-wait-input" && matches!(e,RootStoreError::Launch(v) if matches!(v.as_ref(),AgentLaunchError::Refused("api-provider-wait-change-invalid")|AgentLaunchError::Cause{code:"api-provider-wait-change-invalid",..}))) ||
    (op=="provider-wait-change" && (matches!(e,RootStoreError::Refused("api-provider-wait-change-invalid"|"provider-wait-preview-required"|"provider-wait-file-outside-saved-policy")) || matches!(e,RootStoreError::Launch(v) if matches!(v.as_ref(),AgentLaunchError::Refused("provider-wait-values-invalid"|"provider-wait-request-values-invalid")|AgentLaunchError::Cause{code:"provider-wait-lease-outside-policy",..}))))
}
