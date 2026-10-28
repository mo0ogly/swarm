//! Protected preparation reads/edits, locked conversion and explicit release.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use serde::Serialize;
use swarm_engine::{PreparationReadObservation, RootStore, RootStoreError};

/// Optional preparation ID, document kind and exclusive history cursor.
/// This alias changes no query validation, representation or policy.
type PreparationQuery = (Option<String>, Option<String>, Option<i64>);

/// Parse the exact path-specific query before IO. IDs are names, not paths;
/// before is an exclusive positive i64. Duplicate/unknown/empty fields refuse.
fn query(request: &Request<Vec<u8>>) -> Result<PreparationQuery, E> {
    let mut id = None; let mut document = None; let mut before = None;
    let path = request.uri().path();
    for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
        match key.as_ref() {
            "id" if id.is_none() && matches!(path, "/api/v1/preparations/show" | "/api/v1/preparations/history" | "/api/v1/preparations/release-review" | "/api/v1/preparations/budget" | "/api/v1/preparations/dialogue")
                && swarm_engine::valid_preparation_key(&value) => id = Some(value.into_owned()),
            "document" if document.is_none() && path == "/api/v1/preparations/history"
                && matches!(value.as_ref(), "besoin" | "brief" | "plan") => document = Some(value.into_owned()),
            "before" if before.is_none() && path == "/api/v1/preparations/history" => {
                before = Some(value.parse::<i64>().ok().filter(|v| *v > 0).ok_or(E::Refused("preparation-query-invalid"))?);
            }
            _ => return Err(E::Refused("preparation-query-invalid")),
        }
    }
    if matches!(path, "/api/v1/preparations/show" | "/api/v1/preparations/history" | "/api/v1/preparations/release-review" | "/api/v1/preparations/budget" | "/api/v1/preparations/dialogue") && id.is_none()
        || path == "/api/v1/preparations/history" && document.is_none() {
        return Err(E::Refused("preparation-query-invalid"));
    }
    Ok((id, document, before))
}
/// Select the public DTO and retain safe original method-unavailability causes
/// for later close/output failure traversal. No source content is serialized.
fn encode<T: Serialize>(read: Result<PreparationReadObservation<T>, RootStoreError>, causes: &mut Vec<RootStoreError>) -> Result<Vec<u8>, E> {
    let mut observed = read.map_err(|e| root("preparation-read", e))?;
    causes.append(&mut observed.causes);
    serde_json::to_vec(&observed.value).map_err(E::ResponseJson)
}
/// Exact documentary query vocabulary. No paths are opened by parsing; line
/// integers preserve signed64-bit representation and are checked by the reader.
fn source_query(request:&Request<Vec<u8>>)->Result<(String,String,String,String,i64,i64),E>{
    let source=request.uri().path()=="/api/v1/preparations/source";
    let mut fields=std::collections::BTreeMap::new();
    for (key,value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()){
        let key=key.into_owned();let value=value.into_owned();
        if !matches!(key.as_str(),"id") && !(if source{matches!(key.as_str(),"path"|"start"|"end")}else{matches!(key.as_str(),"scope"|"q")}) || fields.insert(key,value).is_some(){return Err(E::Refused("preparation-query-invalid"));}
    }
    let id=fields.remove("id").ok_or(E::Refused("preparation-query-invalid"))?;
    if source{
        let path=fields.remove("path").ok_or(E::Refused("preparation-query-invalid"))?;
        let start=fields.remove("start").ok_or(E::Refused("preparation-query-invalid"))?.parse::<i64>().map_err(|_|E::Refused("preparation-query-invalid"))?;
        let end=fields.remove("end").ok_or(E::Refused("preparation-query-invalid"))?.parse::<i64>().map_err(|_|E::Refused("preparation-query-invalid"))?;
        Ok((id,path,String::new(),String::new(),start,end))
    }else{Ok((id,String::new(),fields.remove("scope").unwrap_or_else(||".".into()),fields.remove("q").unwrap_or_default(),0,0))}
}
impl NativeHttpService {
    /// Read list/show/history/methods/budget using the same saved policy and projection
    /// as CLI. release-review also returns the current recorded Work for explicit
    /// review; two reads are not atomic, and both revisions must bind release.
    /// GET/HEAD only, no body; authenticate/bind through the shared service,
    /// open readonly, close before publication. Missing rows remain404, malformed
    /// stored data/SQL/IO/close remain500, query mistakes400. Unavailable method
    /// entries and non-current plan_ready never establish acceptance or launch.
    pub(super) fn preparations(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) { return Err(E::Method("GET, HEAD")); }
        if !request.body().is_empty() { return Err(E::Refused("preparation-query-invalid")); }
        let documentary=matches!(request.uri().path(),"/api/v1/preparations/files"|"/api/v1/preparations/source");
        let documentary_query=if documentary{Some(source_query(request)?)}else{None};
        let (id, document, before) = if documentary{(None,None,None)}else{query(request)?};
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let mut causes = Vec::new();
        let result = (|| {
            self.check_binding(&store)?;
            let bytes = match request.uri().path() {
                "/api/v1/preparations/files"|"/api/v1/preparations/source"=>{
                    let (id,path,scope,query,start,end)=documentary_query.as_ref().ok_or(E::Refused("preparation-query-invalid"))?;
                    if request.uri().path().ends_with("/files"){encode(store.preparation_files(id,scope,query),&mut causes)?}
                    else{encode(store.preparation_source(id,path,*start,*end,&mut swarm_engine::SystemEnvironment),&mut causes)?}
                }
                "/api/v1/preparations" => encode(store.preparation_list(), &mut causes)?,
                "/api/v1/preparations/methods" => encode(store.preparation_methods(), &mut causes)?,
                "/api/v1/preparations/templates"=>encode(store.preparation_templates(),&mut causes)?,
                "/api/v1/preparations/show" => encode(store.preparation_show(id.as_deref().ok_or(E::Refused("preparation-query-invalid"))?), &mut causes)?,
                // Shared CLI/HTTP observation; both revisions still bind release.
                "/api/v1/preparations/release-review" => encode(store.preparation_release_review(id.as_deref().ok_or(E::Refused("preparation-query-invalid"))?), &mut causes)?,
                "/api/v1/preparations/dialogue" => encode(store.preparation_dialogue(id.as_deref().ok_or(E::Refused("preparation-query-invalid"))?), &mut causes)?,
                "/api/v1/preparations/budget" => encode(store.preparation_budget(id.as_deref().ok_or(E::Refused("preparation-query-invalid"))?), &mut causes)?,
                "/api/v1/preparations/history" => encode(store.preparation_history(id.as_deref().ok_or(E::Refused("preparation-query-invalid"))?, document.as_deref().ok_or(E::Refused("preparation-query-invalid"))?, before), &mut causes)?,
                _ => return Err(E::Refused("web-route-unported")),
            };
            Ok(RouteBody::plain(bytes))
        })();
        match close(result, store.close()) {
            Ok(mut body) => { body.observation_causes = causes; Ok(body) },
            Err(mut primary) => {
                for cause in causes { primary = E::Close { primary: Some(Box::new(primary)), cleanup: Box::new(cause) }; }
                Err(primary)
            }
        }
    }
}

impl NativeHttpService {
    /// Protected POST draft creation/save/method/brief-adoption/plan-validation command through the same writer as CLI. Shared
    /// transport bounds, session/CSRF and owned Root binding apply before writes.
    /// Close before responding; exact known receipt survives encoding/close and
    /// later owner-observer delivery failures. New-work conversion retains launch locks; only release-plan
    /// explicitly releases them. Neither action synchronously calls a provider.
    pub(super) fn preparation_command(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E>{
        if request.method()!=Method::POST{return Err(E::Method("POST"));}
        if request.uri().query().is_some(){return Err(E::Refused("preparation-query-invalid"));}
        swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        let store=RootStore::open(&self.root,true).map_err(|e|root("open",e))?;
        let mut receipt=None;
        let mut causes=Vec::new();
        let result=(||{
            self.check_binding(&store)?;
            let mut observation=store.preparation_command(request.body(),&mut swarm_engine::SystemEnvironment).map_err(|e|root("preparation-command",e))?;
            causes.append(&mut observation.causes);
            let p=observation.value;
            receipt=Some((p.id.clone(),p.revision));
            let mut body=RouteBody::plain(swarm_engine::encode_model(&p).map_err(E::Model)?);
            body.preparation_receipt=receipt.clone();Ok(body)
        })();
        match (close(result,store.close()),receipt){
            (Err(mut primary),receipt)=>{for cause in causes {primary=E::Close{primary:Some(Box::new(primary)),cleanup:Box::new(cause)};}match receipt{Some((id,revision))=>Err(E::PreparationRecorded{primary:Box::new(primary),id,revision}),None=>Err(primary)}},
            (Ok(mut body),_)=>{body.observation_causes=causes;Ok(body)}
        }
    }
}

impl NativeHttpService {
    /// Protected questionnaire projection through the same readonly Root as CLI.
    /// POST/CSRF/shared body bounds and owned datastore binding apply. Close
    /// before publishing and preserve observation/cleanup causes. Completeness
    /// grants no draft adoption, execution, budget or receipt; SQL is unchanged.
    pub(super) fn preparation_template_check(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E>{
        if request.method()!=Method::POST{return Err(E::Method("POST"));}
        if request.uri().query().is_some(){return Err(E::Refused("preparation-query-invalid"));}
        swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        let store=RootStore::open(&self.root,false).map_err(|e|root("open",e))?;
        let mut causes=Vec::new();
        let result=(||{self.check_binding(&store)?;let bytes=encode(store.preparation_template_check(request.body()),&mut causes)?;Ok(RouteBody::plain(bytes))})();
        match close(result,store.close()) {
            Ok(mut body)=>{body.observation_causes=causes;Ok(body)},
            Err(mut error)=>{for cause in causes{error=E::Close{primary:Some(Box::new(error)),cleanup:Box::new(cause)};}Err(error)}
        }
    }
}

impl NativeHttpService {
    /// Protected explicit send/stop and readonly capabilities. Root/session/CSRF
    /// binding and transport limits precede mutation. A dispatch/close/output
    /// failure cannot imply rollback of an admitted turn: replay the same event.
    /// No real provider contact occurs in this request handler; owned gated Rust
    /// worker applies the separately saved preparation policy and reservation.
    /// Historical active replay remains observation; native preflight precedes spawn.
    pub(super) fn preparation_calls(&self,request:&Request<Vec<u8>>)->Result<RouteBody,E>{
        let capability=request.uri().path().ends_with("/capabilities");
        if capability{if !matches!(*request.method(),Method::GET|Method::HEAD){return Err(E::Method("GET, HEAD"));}if !request.body().is_empty(){return Err(E::Refused("preparation-query-invalid"));}}
        else{if request.method()!=Method::POST{return Err(E::Method("POST"));}swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;}
        if request.uri().query().is_some(){return Err(E::Refused("preparation-query-invalid"));}
        let store=RootStore::open(&self.root,!capability).map_err(|e|root("open",e))?;
        let result=(||{
            self.check_binding(&store)?;
            let bytes=if capability{serde_json::to_vec(&store.preparation_call_capabilities().map_err(|e|root("preparation-call",e))?).map_err(E::ResponseJson)?}
            else{
                let turn=if request.uri().path().ends_with("/send"){
                    let t=store.preparation_send(request.body(),&mut swarm_engine::SystemEnvironment).map_err(|e|root("preparation-call",e))?;
                    if t.status=="pending" && t.supervisor_pid==0{store.preparation_dispatch_if_native(&t.id).map_err(|e|root("preparation-call",e))?;}t
                }else{
                    let r=swarm_engine::decode_preparation_stop(request.body()).map_err(|e|root("preparation-call",e))?;
                    store.preparation_stop(&r.preparation_id,&r.turn_id,&mut swarm_engine::SystemEnvironment).map_err(|e|root("preparation-call",e))?
                };serde_json::to_vec(&turn).map_err(E::ResponseJson)?
            };self.check_binding(&store)?;Ok(RouteBody::plain(bytes))
        })();close(result,store.close())
    }
}
