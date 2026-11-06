//! Read-only security metadata. Compiled scope is not successful test evidence.
use super::{authentication::{self, Source}, confidentiality, state::State, TrustError};
use crate::{RootStoreError as E, RuntimeError, RuntimeOperation};
use serde::Deserialize;
use serde_json::{json, Value};

const INVENTORY: &[u8] = include_bytes!("../../../../../docs/architecture/blp-flow-inventory.json");
const CATALOGUE: &[u8] = include_bytes!("../../config/security-catalogue.json");

/// Closed input; no supplied result, acceptance, filesystem path or command.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request { schema_version: u32 }

/// Observe persisted configuration and compiled BLP scope under the caller's
/// shared Trust reservation. Requires a current authenticated I3/admin and exactly
/// schema_version=1. Returns build declarations, catalogue/inventory hashes, safe
/// flow summaries and a digest of the complete observation before its digest field.
/// Invalid requests/catalogues/build identities refuse with retained causes. No
/// grants, attributions, private paths, content, credentials or identity list are
/// disclosed. No writes, receipt scan, provider/process IO or evidence import.
/// The digest proves neither signed provenance nor an actual binary hash/profile;
/// its change does not establish which historic tests must rerun. Verification and
/// qualification remain unestablished pending their separate L05/L06 contracts.
pub(crate) fn observe(st: &State, raw: &[u8], source: Source<'_>) -> Result<Vec<u8>, E> {
    let trust = |e| E::Trust(Box::new(e));
    st.require_supported().map_err(trust)?;
    let actor = authentication::authenticated(st, source).map_err(trust)?;
    authentication::require_administrator(&actor).map_err(trust)?;
    let request: Request = swarm_wire_json::decode(raw).map_err(|e|trust(TrustError::from(e)))?;
    if request.schema_version != 1 { return Err(trust(TrustError::Invalid("security-schema-unsupported"))); }
    let inventory: Value = serde_json::from_slice(INVENTORY).map_err(|e|trust(TrustError::from(e)))?;
    let catalogue: Value = serde_json::from_slice(CATALOGUE).map_err(|e|trust(TrustError::from(e)))?;
    if inventory["schema_version"] != 1 || inventory["runtime_enforced"] != false
        || catalogue["schema_version"] != 1 || !catalogue["recipes"].is_array()
    { return Err(trust(TrustError::Integrity("security-catalogue-invalid"))); }
    let flows = inventory["flows"].as_array().filter(|v|!v.is_empty())
        .ok_or_else(||trust(TrustError::Integrity("security-inventory-invalid")))?;
    let mut ids=std::collections::BTreeSet::new();
    let mut safe=Vec::new();
    for flow in flows {
        let valid=["id","name_fr","name_en","owner"].iter().all(|k|flow[k].as_str().is_some_and(|s|!s.is_empty()));
        if !valid || !ids.insert(flow["id"].as_str().unwrap_or("")) || flow["status"]!="OPEN"
            || !flow["missing_blp"].as_array().is_some_and(|v|v.iter().all(Value::is_string))
        { return Err(trust(TrustError::Integrity("security-inventory-invalid"))); }
        safe.push(json!({"id":flow["id"],"name_fr":flow["name_fr"],"name_en":flow["name_en"],
            "owner":flow["owner"],"status":flow["status"],"missing_blp":flow["missing_blp"]}));
    }
    let binary=crate::binary_version().map_err(|e|E::Runtime(RuntimeError::new(RuntimeOperation::Platform,e)))?;
    let mut view=json!({"schema_version":1,"kind":"security-observation-v1",
        "trust_revision":st.revision.to_string(),"binary":binary,
        "catalogue_sha256":super::hash(CATALOGUE),"inventory_sha256":super::hash(INVENTORY),
        "observation_scope":"configuration-and-compiled-declared-inventory-only",
        "domains":[{"id":"blp","runtime_enforced":false,"configuration":confidentiality::capability(st).map_err(trust)?,
            "flows":safe,"recipes":catalogue["recipes"],
            "verification":{"result":"NOT_TESTED","freshness":"unknown","evidence_loaded":false},
            "qualification":{"state":"not_qualified","independent_review":false,"external_certification":false}}]});
    view["observation_digest"]=super::digest(&view).map_err(trust)?.into();
    super::encoded(&view).map_err(trust)
}
