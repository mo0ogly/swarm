//! Protected run-limit administration; no provider, mission transition or Agent edit.
use super::{close,root,HttpServiceError as E,NativeHttpService,RouteBody};
use http::{Method,Request};
use swarm_engine::{RootStore,RootStoreError,AgentLaunchError};

/// Exact query allowlist with explicit duplicate/unknown refusal. Values remain
/// data, never SQL/path fragments. Missing scope is not silently defaulted.
fn query(request:&Request<Vec<u8>>,names:&[&str])->Result<Vec<String>,E>{
    let mut values=vec![String::new();names.len()];let mut seen=vec![false;names.len()];
    for (key,value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()){
        let Some(index)=names.iter().position(|name|*name==key) else{return Err(E::Refused("run-limits-query-invalid"));};
        if seen[index]{return Err(E::Refused("run-limits-query-invalid"));}seen[index]=true;values[index]=value.into_owned();
    }
    Ok(values)
}
/// Native extensions retain the typed frame while top-level entry fields also
/// preserve Go apply/rollback consumers. Original data is JSON, never HTML.
fn mutation_value(view:&swarm_engine::RunLimitsMutation,scope:&str,mission:&str,key:&str)->Result<serde_json::Value,E>{
    let mut value=serde_json::to_value(view).map_err(E::ResponseJson)?;
    let object=value.as_object_mut().ok_or(E::Refused("run-limits-response-invalid"))?;
    if !view.preview{
        let entry=serde_json::to_value(&view.entry).map_err(E::ResponseJson)?;
        let fields=entry.as_object().ok_or(E::Refused("run-limits-response-invalid"))?;
        object.extend(fields.iter().map(|(key,value)|(key.clone(),value.clone())));
    }else{
        object.insert("scope".into(),scope.into());object.insert("mission_id".into(),mission.into());object.insert("scope_key".into(),key.into());
        if !view.revision_ok && !view.replayed{object.insert("warning".into(),"run-limits-revision-stale".into());}
    }
    Ok(value)
}
impl NativeHttpService{
    /// Gate methods/body/query before bound Root access. Scope/schema/values,
    /// saved ceilings, source freshness/replay and SQL readback remain Root-owned.
    /// Missing policy/degraded read causes stay explicit; known/uncertain apply
    /// state survives response encoding/Root close/transport failure.
    pub(super) fn run_limits_admin(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E>{
        let path=request.uri().path();let reading=matches!(path,"/api/v1/run-limits"|"/api/v1/run-limits/effective");let rollback=matches!(path,"/api/v1/run-limits/rollback"|"/api/v1/run-limits/rollback/preview");let preview=matches!(path,"/api/v1/run-limits/preview"|"/api/v1/run-limits/rollback/preview");
        if (reading&&!matches!(*request.method(),Method::GET|Method::HEAD)) || (!reading&&request.method()!=Method::POST){return Err(E::Method(if reading{"GET, HEAD"}else{"POST"}));}
        if reading&&!request.body().is_empty(){return Err(E::Refused("run-limits-read-body-invalid"));}
        if !reading&&request.uri().query().is_some(){return Err(E::Refused("run-limits-query-invalid"));}
        let mut change=None;let mut restore=None;
        if !reading{
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
            if rollback{restore=Some(swarm_engine::decode_api_run_limits_rollback(request.body()).map_err(|e|root("run-limits-input",RootStoreError::Launch(Box::new(e))))?);}
            else{
                let mut r=swarm_engine::decode_api_run_limits_config_change(request.body()).map_err(|e|root("run-limits-input",RootStoreError::Launch(Box::new(e))))?;
                // Go API v1 supplies this header when omitted; explicit unsupported
                // versions still refuse. This is wire compatibility, never policy.
                if r.schema_version==0{r.schema_version=1;}change=Some(r);
            }
        }
        let args=if reading{query(request,if path.ends_with("/effective"){&["mission_id","role","task"]}else{&["scope","mission_id","scope_key"]})?}else{Vec::new()};
        let mut store=RootStore::open(&self.root,!reading&&!preview).map_err(|e|root("open",e))?;let mut known=false;let mut causes=Vec::new();
        let result=(||{
            self.check_binding(&store)?;
            let value=if reading{
                if path.ends_with("/effective"){
                    let effective=store.run_limits_effective(&args[0],&args[1],&args[2]).map_err(|e|root("run-limits-effective",e))?;
                    serde_json::json!({"mission_id":args[0],"role":args[1],"task":args[2],"effective":effective,"scope":"administrative-current-policy","note":"Provider/mission/previous-attempt admission is established by agent preview; already frozen attempts are unchanged."})
                }else{
                    let mut view=store.run_limits_view(&args[0],&args[1],&args[2]).map_err(|e|root("run-limits-read",e))?;
                    causes.append(&mut view.observation_causes);serde_json::to_value(&view).map_err(E::ResponseJson)?
                }
            }else if let Some(r)=restore.as_ref(){
                let change=r.as_change();let view=if preview{store.preview_run_limits_rollback(&change,r.to_revision)}else{store.rollback_run_limits(&change,r.to_revision,&mut swarm_engine::SystemEnvironment)}.map_err(|e|root(if preview{"run-limits-preview"}else{"run-limits-rollback"},e))?;
                known=view.applied;mutation_value(&view,&r.scope,&r.mission,&r.key)?
            }else{
                let r=change.as_ref().ok_or(E::Refused("run-limits-request-invalid"))?;
                let view=if preview{store.preview_run_limits(r)}else{store.apply_run_limits(r,&mut swarm_engine::SystemEnvironment)}.map_err(|e|root(if preview{"run-limits-preview"}else{"run-limits-apply"},e))?;
                known=view.applied;mutation_value(&view,&r.scope,&r.mission,&r.key)?
            };
            let mut body=RouteBody::plain(serde_json::to_vec(&value).map_err(E::ResponseJson)?);body.configuration_recorded=known;body.observation_causes=std::mem::take(&mut causes);Ok(body)
        })();
        close(result,store.close()).map_err(|primary|if known||primary.configuration_may_be_recorded(){E::ConfigurationState{primary:Box::new(primary)}}else{primary})
    }
}
/// Only submitted scope/header/values errors classify as400. Stored JSON, SQL,
/// representation/source/cleanup failures remain500, never empty successful data.
pub(super) fn caller_refusal(operation:&str,error:&RootStoreError)->bool{
    if !operation.starts_with("run-limits-"){return false;}
    if matches!(error,RootStoreError::Refused("run-limits-effective-scope-invalid")){return true;}
    let RootStoreError::Launch(error)=error else{return false};
    match error.as_ref(){
        AgentLaunchError::Cause{code:"run-limits-document-invalid",..}=>operation=="run-limits-input",
        AgentLaunchError::Cause{code:"runtime-limit-outside-saved-policy"|"runtime-observation-mode-invalid",..}=>matches!(operation,"run-limits-preview"|"run-limits-apply"|"run-limits-rollback"),
        AgentLaunchError::Refused(code)=>matches!(*code,"run-limits-scope-project-fields"|"run-limits-scope-mission-key"|"run-limits-scope-mission-name"|"run-limits-scope-role-task-names"|"run-limits-scope-unknown"|"run-limits-schema-version"|"run-limits-event-id-name"|"run-limits-observation-requires-mission-reason"|"run-limits-observation-mode"|"run-limits-negative-value"),
        _=>false,
    }
}
