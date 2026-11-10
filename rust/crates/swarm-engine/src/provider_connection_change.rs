//! Strict connection update wire and pure representation validation, no IO.
use crate::{AgentLaunchError as E, model_wire::{Fields,Kind}, provider_connection_catalogue::{ConnectionState,connection_shape}};
use serde::{Deserialize,Serialize};
/// Go connection-change fields with explicit native preview binding. Transport
/// uses the strict decoder; Root revalidates typed requests before source IO.
/// Deserialize alone grants no validation/authority; no Debug/Serialize leaks keys.
#[derive(Default,Deserialize)]
#[serde(default)]
pub struct ApiConnectionChange {
    pub(crate) version:i64,
    pub(crate) expected_digest:String,
    pub(crate) connection:Option<Box<ConnectionState>>,
    pub(crate) replace_key:bool,
    pub(crate) preview:bool,
    pub(crate) preview_token:String,
}
impl ApiConnectionChange {
    /// Route transport intent only; true never authorizes a file mutation.
    pub fn preview_requested(&self)->bool { self.preview }
}
/// Explicit request vocabulary; nested declarations share the inherited source
/// shape, including Go signed64 version/null/case/duplicate merge semantics, not serde aliases.
fn shape()->Fields { &[("version",Kind::Int64),("expected_digest",Kind::String),("connection",Kind::Object(connection_shape)),("replace_key",Kind::Bool),("preview",Kind::Bool),("preview_token",Kind::String)] }
/// Decode a caller-bounded update requiring version1 and a source digest.
/// Nested fields retain inherited Go zero/null/merge semantics; Root validates
/// the required connection before private-source IO. Format/type/unknown failures
/// retain safe typed causes; no contact, credentials or execution are established.
pub fn decode_api_connection_change(raw:&[u8])->Result<ApiConnectionChange,E> {
    let r:ApiConnectionChange=crate::work::decode_mode(raw,shape(),true).map_err(|e|E::caused("api-connection-change-invalid",e))?;
    if r.version!=1 || !crate::provider_configuration::configuration_digest_valid(&r.expected_digest) || r.connection.is_none() { return Err(E::Refused("api-connection-change-invalid")); }
    Ok(r)
}
/// Metadata-only preview/application outcome. saved follows actual apply, changed
/// distinguishes no-op; backup is a private Root content reference, not an audit
/// identity/order. No connection/key/endpoint or implied model readiness returned.
#[derive(Serialize)]
pub struct ApiConnectionMutation {
    pub version:i64,pub preview:bool,pub applied:bool,pub saved:bool,pub changed:bool,
    pub before_digest:String,pub after_digest:String,pub preview_token:String,
    pub backup_reference:Option<String>,
}
/// Validate inherited ID/model formats and safe endpoint/header representation.
/// ID max64 is the existing connection-v1 identifier format (not concurrency or
/// execution policy); model uses existing v1 grammar. Label/key/file count have
/// no copied Go policy cap: actual saved file/transport limits govern their bytes.
/// URI/header parsers perform no DNS/network/IO and all errors retain safe causes.
pub(crate) fn validated(r:&ApiConnectionChange)->Result<ConnectionState,E> {
    if r.version!=1 || !crate::provider_configuration::configuration_digest_valid(&r.expected_digest) {return Err(E::Refused("api-connection-change-invalid"));}
    let mut c=r.connection.as_deref().ok_or(E::Refused("api-connection-change-invalid"))?.clone();
    c.base_url=crate::text::trim_space(&c.base_url).trim_end_matches('/').into();
    c.model=crate::text::trim_space(&c.model).into();
    const CONNECTION_V1_ID_MAX_BYTES:usize=64;
    if c.id.len()>CONNECTION_V1_ID_MAX_BYTES || !swarm_identity_contract::name(c.id.as_bytes()) || c.id.starts_with("api-") {return Err(E::Refused("api-connection-id-invalid"));}
    if crate::text::trim_space(&c.label).is_empty() || !crate::model_routing_policy::model_policy_v1_name(&c.model) {return Err(E::Refused("api-connection-label-model-invalid"));}
    let uri:c_http::Uri=c.base_url.parse().map_err(|e|E::caused("api-connection-endpoint-invalid",e))?;
    if !matches!(uri.scheme_str(),Some("http"|"https")) || uri.authority().is_none_or(|a|a.host().is_empty() || a.as_str().contains('@')) || uri.path_and_query().is_some_and(|p|p.query().is_some()) || c.base_url.contains('#') {return Err(E::Refused("api-connection-endpoint-invalid"));}
    c_http::HeaderValue::from_bytes(c.key.as_bytes()).map_err(|e|E::caused("api-connection-key-invalid",e))?;
    Ok(c)
}
