//! Protected project-profile status, suggestions and explicit preview/configuration.
use super::{close,root,HttpServiceError as E,NativeHttpService,RouteBody};
use http::{Method,Request};
use swarm_engine::RootStore;
impl NativeHttpService{
    /// GET/HEAD with no input/query; shared scoped session and datastore binding
    /// precede the guarded Root/status read. No permission/settings contents or
    /// prompts are returned; reader/encoding/close causes refuse, never empty success.
    pub(super) fn project_profile_status(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E>{
        if !matches!(*request.method(),Method::GET|Method::HEAD){return Err(E::Method("GET, HEAD"));}
        if !request.body().is_empty() || request.uri().query().is_some(){return Err(E::Refused("project-profile-query-invalid"));}
        let store=RootStore::open(&self.root,false).map_err(|e|root("open",e))?;
        let result=(||{self.check_binding(&store)?;let value=store.project_profile_status().map_err(|e|root("project-profile-read",e))?;serde_json::to_vec(&value).map(RouteBody::plain).map_err(E::ResponseJson)})();
        close(result,store.close())
    }

    /// Catalog GET/HEAD is observational; preview/apply/select require the shared
    /// protected POST path and input transport bound. Root owns current SHA,
    /// saved caps, source validation and physical custody. A successful mutation
    /// marks late encoding, service binding, Root close or response failures as
    /// configuration-preserved; no rollback, provider call or permission is implied.
    pub(super) fn project_profile_configuration(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E>{
        let catalog=request.uri().path().ends_with("/catalog");
        let preview=request.uri().path().ends_with("/preview");
        let writing=!catalog && !preview;
        if catalog {if !matches!(*request.method(),Method::GET|Method::HEAD){return Err(E::Method("GET, HEAD"));}if !request.body().is_empty(){return Err(E::Refused("project-profile-query-invalid"));}}
        else {if request.method()!=Method::POST{return Err(E::Method("POST"));}swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;}
        if request.uri().query().is_some(){return Err(E::Refused("project-profile-query-invalid"));}
        let operation=if catalog{"project-profile-catalog"}else if preview{"project-profile-preview"}else{"project-profile-write"};
        let store=RootStore::open(&self.root,writing).map_err(|e|root("open",e))?;
        let mut recorded=false;
        let result=(||{
            self.check_binding(&store)?;
            let bytes=if catalog{serde_json::to_vec(&store.project_profile_catalog().map_err(|e|root(operation,e))?)}
                else if preview{serde_json::to_vec(&store.project_profile_preview(request.body()).map_err(|e|root(operation,e))?)}
                else{
                    let value=if request.uri().path().ends_with("/select"){store.project_profile_select_document(request.body())}else{store.project_profile_apply(request.body())}.map_err(|e|root(operation,e))?;
                    recorded=true;serde_json::to_vec(&value)
                }.map_err(E::ResponseJson)?;
            self.check_binding(&store)?;
            let mut body=RouteBody::plain(bytes);body.configuration_recorded=recorded;Ok(body)
        })();
        close(result,store.close()).map_err(|primary|if recorded{E::ConfigurationState{primary:Box::new(primary)}}else{primary})
    }
}

/// Classify only candidate/input failures at the new writable profile boundary.
/// Original IO, malformed active configuration and any cleanup retain server
/// diagnostics. The dedicated unported-memory mapping must precede this check.
pub(super) fn caller_invalid(operation:&str,error:&swarm_engine::RootStoreError)->bool{
    if !matches!(operation,"project-profile-preview"|"project-profile-write"){return false;}
    match error{
        swarm_engine::RootStoreError::Context(e) if e.cleanup_cause().is_none()=>e.is_invalid() || e.reason()=="project-profile-request-invalid",
        _=>false,
    }
}

/// Publish static project-context reasons on the profile read and preparation
/// call boundary. Compound failures are not flattened into a semantic refusal. Raw cause
/// strings, paths, prompts and native settings are never included. Compound or
/// cleanup failures keep their owning transport diagnostic instead of flattening
/// it into a misleading profile reason. No error classification/authority changes.
pub(super) fn public_reason(error:&E)->Option<&'static str>{
    match error {
        E::Root{operation,cause} if matches!(*operation,"project-profile-catalog"|"project-profile-preview"|"project-profile-write")=>match cause.as_ref(){
            swarm_engine::RootStoreError::Context(e) if e.cleanup_cause().is_none()=>Some(e.reason()),
            swarm_engine::RootStoreError::Path(e)=>Some(e.reason()),
            _=>None,
        },
        E::Root{operation:"preparation-call",cause}=>match cause.as_ref(){
            swarm_engine::RootStoreError::Context(e) if e.cleanup_cause().is_none()=>Some(e.reason()),
            _=>None,
        },
        E::Root{operation:"project-profile-read",cause}=>match cause.as_ref(){
            swarm_engine::RootStoreError::Context(e) if e.cleanup_cause().is_none()=>Some(e.reason()),
            swarm_engine::RootStoreError::Path(e)=>Some(e.reason()),
            _=>None,
        },
        _=>None,
    }
}
