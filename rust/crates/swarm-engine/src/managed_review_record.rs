//! Frozen managed reviewer input/policy observations, never acceptance authority.
use crate::{AgentLaunchError as E, managed_review_protocol::ManagedReviewProtocol};
use serde::{Deserialize, Serialize};

/// Private original data bound by the framed native receipt and SQL body hash.
/// Context/reports have no Debug; frozen limits are actual saved settings, never
/// implicit historical defaults. A snapshot is not identity or a source lease.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub(crate) protocol: ManagedReviewProtocol,
    pub(crate) limits: crate::ReviewReplyLimits,
    pub(crate) response_limit_bytes: Option<usize>,
    pub(crate) prompt_sha256: String,
}
impl Snapshot {
    /// Freeze exact engine-built protocol and saved reviewer policy before exec.
    /// No process, write, debit or permission; invalid settings/codec preserve cause.
    pub(crate) fn capture(inputs:&crate::TaskReviewInputs,values:&crate::EngineSettingsValues)->Result<Option<Self>,E>{
        let Some(value)=&inputs.managed_protocol else{return Ok(None)};
        values.validate().map_err(|e|E::caused(e.reason(),e))?;
        let raw=crate::encode_model(value).map_err(|e|E::caused("review-managed-protocol-invalid",e))?;
        let protocol=serde_json::from_slice(&raw).map_err(|e|E::caused("review-managed-protocol-invalid",e))?;
        let result=Self{protocol,limits:values.review_reply_limits().map_err(|e|E::caused(e.reason(),e))?,response_limit_bytes:values.review_response_limit().map_err(|e|E::caused(e.reason(),e))?,prompt_sha256:inputs.prompt_sha256.clone()};
        result.validate()?;Ok(Some(result))
    }
    /// Validate saved format/context/catalogue identities without source IO or
    /// interpreting a failed raw reply as a valid opinion. No new defaults apply.
    pub(crate) fn validate(&self)->Result<(),E>{
        if !swarm_identity_contract::digest(self.prompt_sha256.as_bytes()) || self.limits.max_text_bytes.is_some_and(|n|n<self.limits.min_text_bytes){return Err(E::Refused("review-managed-original-policy-invalid"));}
        crate::managed_review_output::protocol_valid(&self.protocol)
    }
    /// Compare two typed frozen snapshots, after a JSON projection was decoded
    /// into this same model. Struct field order and JSONValue key order must not
    /// become false source drift; canonical opaque bytes, policy and prompt still
    /// compare exactly. Codec errors preserve cause; no IO or permission occurs.
    pub(crate) fn same_inputs(&self,other:&Self)->Result<bool,E>{
        self.validate()?;other.validate()?;
        Ok(self.prompt_sha256==other.prompt_sha256 && self.limits==other.limits
            && self.response_limit_bytes==other.response_limit_bytes
            && crate::encode_model(&self.protocol).map_err(|e|E::caused("review-managed-protocol-invalid",e))?
                ==crate::encode_model(&other.protocol).map_err(|e|E::caused("review-managed-protocol-invalid",e))?)
    }
    /// Revalidate original exact reply under the original frozen policy, not a
    /// later operator cap. Current publication must separately recheck current
    /// inputs/settings and ledger; this proves no criterion truth or acceptance.
    pub(crate) fn decode(&self,raw:&[u8])->Result<crate::managed_review_reply::ParsedManagedReview,E>{
        self.validate()?;
        crate::managed_review_output::decode(raw,&self.protocol,self.limits,self.response_limit_bytes)
    }
    /// Canonical original context bytes and content-addressed engine filename.
    /// Private host/process details are absent from the context producer. This
    /// computes data only; caller owns create-only publication and IO verification.
    pub(crate) fn context(&self)->Result<(String,Vec<u8>),E>{
        self.validate()?;
        let raw=crate::encode_model(&self.protocol.context).map_err(|e|E::caused("review-managed-context-encode",e))?;
        Ok((format!("native-review-context-v1-{}.json",self.protocol.context_sha256),raw))
    }
    /// Project a fully decoded whole-cohort opinion into existing managed review
    /// fields. Go-compatible aggregate rationale keeps every task reason when
    /// changes are requested. No other task is accepted or individually rewritten.
    pub(crate) fn apply(&self,raw:&[u8],review:&mut crate::IndependentReview,task:&str)->Result<(),E>{
        let parsed=self.decode(raw)?;
        let selected=parsed.tasks.iter().find(|entry|entry.task==task).ok_or(E::Refused("review-managed-target-missing"))?;
        review.criteria=selected.criteria.clone();
        review.state=match parsed.state{crate::ReviewReplyState::Passed=>"passed",crate::ReviewReplyState::ChangesRequested=>"changes_requested"}.into();
        review.reason=format!("Revue indépendante des critères sur le candidat {}",self.protocol.context.candidate_commit);
        if parsed.state!=crate::ReviewReplyState::Passed {for entry in &parsed.tasks{review.reason+=&format!(" ; {} : {}",entry.task,entry.reason);}}
        review.candidate_commit=self.protocol.context.candidate_commit.clone();
        review.previous_candidate=self.protocol.context.previous_candidate.clone();
        review.context_sha256=self.protocol.context_sha256.clone();
        review.context=format!(".swarm/{}",self.context()?.0);
        review.managed_tasks=Some(parsed.tasks);
        Ok(())
    }
    /// Check stored successful public result against this exact original reply
    /// and context; no filesystem read, current freshness or new permission.
    pub(crate) fn matches(&self,raw:&[u8],review:&crate::IndependentReview,task:&str)->Result<bool,E>{
        let mut expected=review.clone();self.apply(raw,&mut expected,task)?;
        Ok(expected.state==review.state && expected.reason==review.reason && expected.criteria==review.criteria
            && expected.managed_tasks==review.managed_tasks && expected.candidate_commit==review.candidate_commit
            && expected.previous_candidate==review.previous_candidate && expected.context==review.context
            && expected.context_sha256==review.context_sha256 && review.git_report.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Two current cohort tasks, no accepted baseline or claimed identity.
    fn fixture()->(Snapshot,Vec<u8>,crate::IndependentReview){
        let values=crate::EngineSettingsValues::installation_defaults().unwrap();
        let context=crate::ManagedReviewContext{citation_contract_version:2,inspection_scope_version:1,inspection_only:true,candidate_commit:"a".repeat(40),
            receipt:crate::model_wire::persisted_raw(&serde_json::json!({"unicode":"élève<tag>&","ordinal":17})).unwrap(),
            tasks:Some(["one","two"].iter().map(|id|crate::ManagedReviewTaskContext{task:(*id).into(),criteria:Some(vec!["Required observed behavior".into()]),report:format!("Original whole source for {id}"),binding:crate::ManagedTaskReview{task:(*id).into(),report:format!("{id}.md"),..Default::default()},..Default::default()}).collect()),..Default::default()};
        let limits=values.review_reply_limits().unwrap();let protocol=crate::managed_review_protocol::build(context,limits).unwrap();
        let tasks:Vec<_>=protocol.context.tasks.as_ref().unwrap().iter().map(|t|serde_json::json!({"task":t.task,"reason":"Current complete original source observed","criteria":[{"index":1,"verdict":"pass","evidence_ref":protocol.catalog.quotes.iter().find(|q|q.text==t.report).unwrap().id}]})).collect();
        let raw=crate::encode_model(&serde_json::json!({"catalog_sha256":protocol.catalog_sha256,"candidate_commit":protocol.context.candidate_commit,"tasks":tasks})).unwrap();
        (Snapshot{protocol,limits,response_limit_bytes:values.review_response_limit().unwrap(),prompt_sha256:"b".repeat(64)},raw,crate::IndependentReview::default())
    }
    #[test]
    fn managed_original_roundtrip_binds_cohort_context_and_go_rationale(){
        let (snapshot,raw,mut review)=fixture();snapshot.apply(&raw,&mut review,"one").unwrap();
        assert_eq!(review.managed_tasks.as_ref().unwrap().len(),2);assert_eq!(review.state,"passed");
        assert!(snapshot.matches(&raw,&review,"one").unwrap());
        let bytes=crate::encode_model(&snapshot).unwrap();let restored:Snapshot=serde_json::from_slice(&bytes).unwrap();
        assert!(restored.matches(&raw,&review,"one").unwrap());
        let projected=serde_json::to_value(&restored.protocol).unwrap();
        let typed:crate::managed_review_protocol::ManagedReviewProtocol=serde_json::from_slice(&crate::encode_model(&projected).unwrap()).unwrap();
        let current=Snapshot{protocol:typed,limits:restored.limits,response_limit_bytes:restored.response_limit_bytes,prompt_sha256:restored.prompt_sha256.clone()};
        assert!(snapshot.same_inputs(&current).unwrap(),"canonical typed/value roundtrip is not source drift");
        let (name,context)=restored.context().unwrap();assert!(review.context.ends_with(&name));
        assert_eq!(crate::provider_configuration::configuration_digest(&context),review.context_sha256);
        review.managed_tasks.as_mut().unwrap().pop();assert!(!restored.matches(&raw,&review,"one").unwrap());
    }
    #[test]
    fn managed_original_policy_and_binding_corruption_refuse(){
        let (mut snapshot,raw,mut review)=fixture();snapshot.apply(&raw,&mut review,"one").unwrap();
        snapshot.response_limit_bytes=Some(0);assert!(snapshot.decode(&raw).is_err());
        snapshot.response_limit_bytes=None;snapshot.limits.max_text_bytes=Some(0);assert!(snapshot.validate().is_err());
        let (mut snapshot,_,_)=fixture();snapshot.protocol.context_sha256="0".repeat(64);assert!(snapshot.validate().is_err());
        let (snapshot,raw,mut review)=fixture();snapshot.apply(&raw,&mut review,"one").unwrap();review.candidate_commit="0".repeat(40);assert!(!snapshot.matches(&raw,&review,"one").unwrap());
    }
}
