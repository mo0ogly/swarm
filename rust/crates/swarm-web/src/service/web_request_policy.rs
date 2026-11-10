//! Protected browser waiting configuration; shared authoritative Root policy.
use super::{close,root,HttpServiceError as E,NativeHttpService,RouteBody};
use http::{Method,Request};
use swarm_engine::{RootStore,RootStoreError,AgentLaunchError};
impl NativeHttpService {
    /// Read/HEAD or preview/apply with common session/CSRF/Root custody. No network
    /// or mission mutation. Applied/replayed state survives encode/output/close.
    pub(super) fn web_request_policy(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E> {
        let path=request.uri().path();let script=path=="/api/v1/web-requests/script.js";let write=path!="/api/v1/web-requests"&&!script;
        if (write && request.method()!=Method::POST) || (!write && request.method()!=Method::GET && request.method()!=Method::HEAD){return Err(E::Method(if write{"POST"}else{"GET, HEAD"}));}
        if request.uri().query().is_some(){return Err(E::Refused("web-requests-query-invalid"));}
        let change=if write {swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;Some(swarm_engine::decode_api_web_request_change(request.body()).map_err(|e|root("web-requests-input",RootStoreError::Launch(Box::new(e))))?)}else{if !request.body().is_empty(){return Err(E::Refused("web-requests-body-invalid"));}None};
        let apply=path.ends_with("/apply");let store=RootStore::open(&self.root,apply).map_err(|e|root("open",e))?;let mut known=false;
        let result=(|| {self.check_binding(&store)?;let bytes=if let Some(r)=change.as_ref(){let view=if apply{store.apply_web_request_policy(r)}else{store.preview_web_request_policy(r)}.map_err(|e|root("web-requests-change",e))?;known=view.applied;serde_json::to_vec(&view).map_err(E::ResponseJson)?}else{{let view=store.web_request_policy().map_err(|e|root("web-requests-read",e))?;if script{let raw=swarm_engine::encode_model(&serde_json::json!({"revision":view.config.revision,"values":view.config.values})).map_err(|e|root("web-requests-script",RootStoreError::Launch(Box::new(AgentLaunchError::Cause{code:"web-requests-script-encode",cause:Box::new(e)}))))?;let mut bytes=b"globalThis.SwarmWebRequestConfig = ".to_vec();bytes.extend(raw);bytes.extend(b";\n");bytes}else{serde_json::to_vec(&view).map_err(E::ResponseJson)?}}};let mut body=RouteBody::plain(bytes);body.configuration_recorded=known;Ok(body)})();
        close(result,store.close()).map_err(|primary|if known || primary.configuration_may_be_recorded(){E::ConfigurationState{primary:Box::new(primary)}}else{primary})
    }
}
/// Classify submitted data only; stored/source/settings/cleanup causes remain500.
pub(super) fn caller_refusal(op:&str,e:&RootStoreError)->bool {
    if op=="web-requests-input" { return matches!(e,RootStoreError::Launch(_)); }
    if op!="web-requests-change" {return false;}
    matches!(e,RootStoreError::Refused("web-request-policy-file-outside-saved-policy")) || matches!(e,RootStoreError::Launch(v) if matches!(v.as_ref(),AgentLaunchError::Refused("web-request-change-invalid"|"web-request-values-invalid")))
}
