//! Protected preparation reads and draft create/save/method selection/brief adoption/plan validation; no dialogue or execution authority.
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
            "id" if id.is_none() && matches!(path, "/api/v1/preparations/show" | "/api/v1/preparations/history")
                && swarm_engine::valid_preparation_key(&value) => id = Some(value.into_owned()),
            "document" if document.is_none() && path == "/api/v1/preparations/history"
                && matches!(value.as_ref(), "besoin" | "brief" | "plan") => document = Some(value.into_owned()),
            "before" if before.is_none() && path == "/api/v1/preparations/history" => {
                before = Some(value.parse::<i64>().ok().filter(|v| *v > 0).ok_or(E::Refused("preparation-query-invalid"))?);
            }
            _ => return Err(E::Refused("preparation-query-invalid")),
        }
    }
    if matches!(path, "/api/v1/preparations/show" | "/api/v1/preparations/history") && id.is_none()
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
    serde_json::to_vec(&observed.value).map_err(E::Json)
}
impl NativeHttpService {
    /// Read list/show/history/methods using the same saved policy and projection
    /// as CLI. GET/HEAD only, no body; authenticate/bind through the shared service,
    /// open readonly, close before publication. Missing rows remain404, malformed
    /// stored data/SQL/IO/close remain500, query mistakes400. Unavailable method
    /// entries and non-current plan_ready never establish acceptance or launch.
    pub(super) fn preparations(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) { return Err(E::Method("GET, HEAD")); }
        if !request.body().is_empty() { return Err(E::Refused("preparation-query-invalid")); }
        let (id, document, before) = query(request)?;
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let mut causes = Vec::new();
        let result = (|| {
            self.check_binding(&store)?;
            let bytes = match request.uri().path() {
                "/api/v1/preparations" => encode(store.preparation_list(), &mut causes)?,
                "/api/v1/preparations/methods" => encode(store.preparation_methods(), &mut causes)?,
                "/api/v1/preparations/show" => encode(store.preparation_show(id.as_deref().ok_or(E::Refused("preparation-query-invalid"))?), &mut causes)?,
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
    /// later owner-observer delivery failures. No provider, conversion or execution authority is exposed.
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
