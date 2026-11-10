//! Protected historical activity page, never permission or an autonomous-action proof.
use super::{NativeHttpService,HttpServiceError as E,RouteBody,root,close};
use http::{Request,Method};
/// Parse one strict native query; omitted/0 page limit selects saved default.
/// The before cursor is opaque recorded data. Unknown/duplicate/noncanonical
/// counts or decision flags refuse before IO, unlike Go's ignored Atoi errors.
fn query(raw:Option<&str>)->Result<(String,String,Option<i64>,bool),E>{
 let(mut work,mut before,mut limit,mut decisions)=(None,None,None,None);for(k,v)in form_urlencoded::parse(raw.unwrap_or("").as_bytes()){let slot=match k.as_ref(){"work"=>&mut work,"before"=>&mut before,"limit"=>&mut limit,"decisions"=>&mut decisions,_=>return Err(E::Refused("activity-query-invalid"))};if slot.replace(v.into_owned()).is_some(){return Err(E::Refused("activity-query-invalid"))}}
 let work=work.filter(|v|swarm_identity_contract::name(v.as_bytes())).ok_or(E::Refused("activity-query-invalid"))?;let count=limit.map(|v|{let n=v.parse::<i64>().map_err(|_|E::Refused("activity-query-invalid"))?;if n<0||n.to_string()!=v{return Err(E::Refused("activity-query-invalid"))}Ok(n)}).transpose()?;let only=match decisions.as_deref(){None|Some("0")=>false,Some("1")=>true,_=>return Err(E::Refused("activity-query-invalid"))};Ok((work,before.unwrap_or_default(),count,only))
}
impl NativeHttpService{
 /// GET/HEAD with session/admission/Root binding and empty body. Read existing
 /// Work/source rows under their SQL snapshot and observed saved policy. Go core
 /// page fields plus native policy metadata; no hidden page cap or mutation.
 /// Source/policy/codec/rollback/close errors never become successful empty data.
 pub(super) fn activity(&self,r:&Request<Vec<u8>>)->Result<RouteBody,E>{if !matches!(*r.method(),Method::GET|Method::HEAD){return Err(E::Method("GET, HEAD"))}if !r.body().is_empty(){return Err(E::Refused("activity-body-invalid"))}let(work,before,limit,decisions)=query(r.uri().query())?;let store=swarm_engine::RootStore::open(&self.root,false).map_err(|e|root("open",e))?;let result=(||{self.check_binding(&store)?;let view=store.activity(&work,&before,limit,decisions).map_err(|e|root("activity-read",e))?;serde_json::to_vec(&view).map(RouteBody::plain).map_err(E::ResponseJson)})();close(result,store.close())}
}
