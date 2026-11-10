//! Existing embedded framing/work templates; no selection grants authority.
use super::{preparation_read::cause,RootStoreError as E};
use crate::{ActionPlan,AgentWorkflow,EngineSettingsValues};
use serde::{Deserialize,Serialize};
use std::collections::{BTreeMap,BTreeSet};

/// Localized editorial template, enriched only with the compiled role framing.
/// Work models are draft data, never adopted plans, agents or executable edges.
#[derive(Clone,Default,Deserialize,Serialize)]
#[serde(default)]
pub struct PreparationTemplate {
    #[serde(skip_serializing_if="Option::is_none")]
    pub work_model:Option<WorkModel>,
    pub id:String,pub version:i64,pub title:BTreeMap<String,String>,
    pub need:BTreeMap<String,String>,pub phases:Vec<String>,pub recommended_method:String,
    pub questions:Vec<Question>,pub team:Vec<Role>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub subject:Option<Subject>,
}
pub(super) type Template=PreparationTemplate;
/// Template subject and its explicitly proposed interventions, translated FR/EN.
#[derive(Clone,Default,Deserialize,Serialize)]
#[serde(default)]
pub struct Subject {
    pub summary:BTreeMap<String,String>,pub outcome:BTreeMap<String,String>,
    pub example:BTreeMap<String,String>,pub deliverables:BTreeMap<String,Vec<String>>,
    pub evidence:BTreeMap<String,Vec<String>>,pub risks:BTreeMap<String,Vec<String>>,
    pub interventions:Vec<Intervention>,
}
/// Editorial intervention, not an authorization or automatic branch.
#[derive(Clone,Default,Deserialize,Serialize)]
#[serde(default)]
pub struct Intervention {
    pub id:String,pub title:BTreeMap<String,String>,pub guidance:BTreeMap<String,String>,
    pub recommended_method:String,
}
/// One question whose placeholder is substituted only by the authoritative check.
#[derive(Clone,Default,Deserialize,Serialize)]
#[serde(default)]
pub struct Question {
    pub id:String,pub label:BTreeMap<String,String>,pub line:BTreeMap<String,String>,pub required:bool,
}
/// Proposed responsibility with engine-compiled instructions, never an agent.
#[derive(Clone,Default,Deserialize,Serialize)]
#[serde(default)]
pub struct Role {
    pub role:String,pub title:BTreeMap<String,String>,pub responsibility:BTreeMap<String,String>,
    pub method_guidance:BTreeMap<String,String>,pub workflow:Option<AgentWorkflow>,
}
/// Draft graph and explanatory recovery decisions; no second scheduler.
#[derive(Clone,Default,Deserialize,Serialize)]
#[serde(default)]
pub struct WorkModel {
    pub version:i64,pub increments:Vec<Increment>,pub decisions:BTreeMap<String,Vec<String>>,
}
/// One translated increment using the actual public ActionPlan data contract.
#[derive(Clone,Default,Deserialize,Serialize)]
#[serde(default)]
pub struct Increment {
    pub id:String,pub title:BTreeMap<String,String>,pub plans:BTreeMap<String,ActionPlan>,
}

/// Validate an embedded graph against current plan/task caps without adopting it.
/// Requires FR/EN plans, titles and decision explanations, unique protocol IDs,
/// product structure and the shared partial-plan validator. Missing answers are
/// permitted here; final writer validation/adoption remains mandatory. Safe
/// schema/codec/plan causes propagate, no filesystem/provider/SQL mutation occurs.
fn validate_model(model:&WorkModel,settings:&EngineSettingsValues)->Result<(),E>{
    if model.version!=1 || model.increments.is_empty(){return Err(E::Refused("preparation-template-model-invalid"));}
    let policy=settings.preparation_plan_policy().map_err(E::Settings)?;
    let task=settings.mutation_limits().map_err(E::Settings)?.task;
    let mut seen=BTreeSet::new();
    for increment in &model.increments {
        if !swarm_identity_contract::name(increment.id.as_bytes()) || !seen.insert(&increment.id){return Err(E::Refused("preparation-template-increment-invalid"));}
        for language in ["fr","en"] {
            if increment.title.get(language).is_none_or(String::is_empty) || model.decisions.get(language).is_none_or(Vec::is_empty){return Err(E::Refused("preparation-template-model-translation-invalid"));}
            let plan=increment.plans.get(language).ok_or(E::Refused("preparation-template-model-translation-invalid"))?;
            if plan.product.is_none(){return Err(E::Refused("preparation-template-model-product-required"));}
            let raw=crate::encode_model(plan).map_err(|e|cause("preparation-template-encode",e))?;
            super::preparation_plan::validate_draft(&raw,"ks-product",&policy,&task)?;
        }
    }
    Ok(())
}
/// Decode the current compiled catalogue and enrich roles from the same framing
/// producer used for launches, with its saved body cap. Validate each work graph
/// using current policies; errors refuse the catalogue rather than returning an
/// empty/partial success. Role hashes are observations, not authentication or
/// receipt/future freshness. No project files, agents, drafts or authority change.
pub(super) fn catalogue(settings:&EngineSettingsValues)->Result<Vec<Template>,E>{
    let mut templates:Vec<Template>=serde_json::from_slice(include_bytes!("../../../../../tools/agent-workflows/templates/preparations.json")).map_err(|e|cause("preparation-template-catalog-invalid",e))?;
    let body_limit=settings.agent_workflow_body_limit().map_err(E::Settings)?;
    for template in &mut templates {
        if let Some(model)=&template.work_model {validate_model(model,settings)?;}
        for role in &mut template.team {
            role.workflow=Some(crate::assemble_agent_workflow(&role.role,body_limit).map_err(E::Workflow)?.workflow);
        }
    }
    Ok(templates)
}
/// Resolve historical IDs to the current editorial catalogue. Aliases are a
/// compatibility protocol, not operator policy or a launch choice. Unknown IDs
/// refuse without quoting user text; no catalogue/storage mutation occurs.
pub(super) fn select(templates:&[Template],id:&str)->Result<Template,E>{
    let current=match id {"ks-feature"=>"application","ks-bugfix"=>"correction","ks-interface"=>"interface","ks-product"=>"product",other=>other};
    templates.iter().find(|t|t.id==current).cloned().ok_or(E::Refused("preparation-template-not-found"))
}
