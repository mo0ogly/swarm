//! Native whole-cohort data decoding over the shared tool-free JSONL transport.
//! Supplied framing/opinions never prove execution, identity or acceptance.
use crate::{AgentLaunchError as E, EngineSettingsValues, ProviderResponse, ProviderResponseStream,
    managed_review_protocol::ManagedReviewProtocol, managed_review_reply::{ParsedManagedReview, parse_managed_review_reply}};
use std::{collections::BTreeSet, io::{self, Write}};

/// Recheck internal context/catalogue identity before consuming supplied bytes.
/// Only the currently supported inspection-v1/citation-v2 scope is admitted;
/// these versions are wire invariants, not saved operator cutoffs. No IO occurs.
fn protocol_valid(protocol: &ManagedReviewProtocol) -> Result<(), E> {
    let context = &protocol.context;
    if context.citation_contract_version != 2 || context.inspection_scope_version != 1
        || !context.inspection_only || context.accepted_baseline.is_some() {
        return Err(E::Refused("review-managed-output-scope-unsupported"));
    }
    let tasks = context.tasks.as_deref().filter(|tasks| !tasks.is_empty())
        .ok_or(E::Refused("review-managed-output-cohort-required"))?;
    let mut seen = BTreeSet::new();
    if tasks.iter().any(|task| task.task.is_empty() || !seen.insert(task.task.as_str())) {
        return Err(E::Refused("review-managed-output-cohort-invalid"));
    }
    let digest = crate::provider_configuration::configuration_digest(
        &crate::encode_model(context).map_err(|e| E::caused("review_fragment_protocol", e))?);
    if digest != protocol.context_sha256 {
        return Err(E::Refused("review-managed-output-context-stale"));
    }
    protocol.catalog.resolver(&context.candidate_commit, &digest, &protocol.catalog_sha256)
        .map_err(|e| E::caused(e.code(), e))?;
    Ok(())
}

/// Preserve original names until exact canonical uniqueness is checked at every
/// native structured boundary. Historical Go-compatible parsing runs afterwards;
/// arbitrary supplied field/value text never enters these diagnostic reasons.
fn canonical_reply(raw: &[u8]) -> Result<(), E> {
    let boundaries = crate::agent_tool_guard::validated_json(raw)
        .map_err(|e| E::caused("review_fragment_protocol", e))?;
    let fields = crate::review_output::canonical_fields(raw, &boundaries,
        &["catalog_sha256", "candidate_commit", "tasks"])?;
    for (key,value) in fields {
        if key != "tasks" || value.trim_ascii() == b"null" { continue; }
        for task in crate::model_wire::array(value.trim_ascii(), &boundaries)
            .map_err(|e| E::caused("review_fragment_protocol", e))? {
            for (key,value) in crate::review_output::canonical_fields(task, &boundaries,
                &["task", "reason", "criteria"])? {
                if key != "criteria" || value.trim_ascii() == b"null" { continue; }
                for criterion in crate::model_wire::array(value.trim_ascii(), &boundaries)
                    .map_err(|e| E::caused("review_fragment_protocol", e))? {
                    crate::review_output::canonical_fields(criterion, &boundaries,
                        &["index", "verdict", "evidence", "evidence_ref"])?;
                }
            }
        }
    }
    Ok(())
}

/// Check exact retained reply bytes under saved policies and current supplied
/// context/catalogue. No terminal/EOF, usage, identity or permission is invented.
/// Invalid scope, stale digests, limits, fields, cohort and quotes retain causes;
/// this performs no IO, mutation, truncation or ordinary/v1 fallback.
#[allow(dead_code)] // Future retained native managed journal consumer remains guarded.
pub(crate) fn decode_retained_managed_review(raw: &[u8], values: &EngineSettingsValues,
    protocol: &ManagedReviewProtocol) -> Result<ParsedManagedReview,E> {
    values.validate().map_err(|e| E::caused(e.reason(), e))?;
    let cap = values.review_response_limit().map_err(|e| E::caused(e.reason(), e))?;
    let limits = values.review_reply_limits().map_err(|e| E::caused(e.reason(), e))?;
    decode(raw,protocol,limits,cap)
}

/// Shared exact domain validation after the caller freezes saved policy. Typed
/// errors propagate; no stream state or externally observed effect is inferred.
fn decode(raw:&[u8], protocol:&ManagedReviewProtocol, limits:crate::ReviewReplyLimits,
    cap:Option<usize>) -> Result<ParsedManagedReview,E> {
    protocol_valid(protocol)?;
    if cap.is_some_and(|cap| raw.len()>cap) {
        return Err(E::Refused("review-publication-reply-cap"));
    }
    canonical_reply(raw)?;
    parse_managed_review_reply(raw,&protocol.context,Some(&protocol.catalog),limits,cap)
        .map_err(|e| E::caused(e.code(), e))
}

/// Original bounded transport and whole-cohort opinion data, with no Debug of
/// private text. Provider facts are unverified declarations, never a settlement.
pub(crate) struct ManagedReviewProviderOutput {
    pub(crate) review: ParsedManagedReview,
    pub(crate) transport: ProviderResponse,
}

/// Tool-free shared transport with frozen saved limits and borrowed protocol.
/// The owner supplies/observes EOF and retains raw capture separately; this does
/// not launch a process or grant review publication/acceptance rights.
pub(crate) struct ManagedReviewOutputStream<'a> {
    stream: ProviderResponseStream,
    protocol: &'a ManagedReviewProtocol,
    limits: crate::ReviewReplyLimits,
    cap: Option<usize>,
}
impl<'a> ManagedReviewOutputStream<'a> {
    /// Freeze actual saved response/event/text policy and verify protocol identity
    /// before feed. Null cap is explicit unlimited, zero refuses nonempty reply;
    /// missing/invalid settings refuse. No extra ceiling, retry or timeout exists.
    pub(crate) fn from_saved_settings(values:&EngineSettingsValues, protocol:&'a ManagedReviewProtocol) -> Result<Self,E> {
        values.validate().map_err(|e| E::caused(e.reason(), e))?;
        protocol_valid(protocol)?;
        let cap=values.review_response_limit().map_err(|e| E::caused(e.reason(), e))?;
        let limits=values.review_reply_limits().map_err(|e| E::caused(e.reason(), e))?;
        Ok(Self {stream:ProviderResponseStream::new_tool_free(values,cap)?,protocol,limits,cap})
    }
    /// Feed exact event bytes; shared tools/framing/caps failures latch refusal.
    pub(crate) fn feed(&mut self, bytes:&[u8]) -> Result<(),E> { self.stream.feed(bytes) }
    /// Finish once after supplied/observed EOF. Domain failure keeps original
    /// bounded reply bytes and typed cause; a second finish can never succeed.
    pub(crate) fn finish(&mut self) -> Result<ManagedReviewProviderOutput,E> {
        let (protocol,limits,cap)=(self.protocol,self.limits,self.cap);
        let (transport,review)=self.stream.finish_decoded(|raw|decode(raw,protocol,limits,cap))?;
        Ok(ManagedReviewProviderOutput {review,transport})
    }
    /// Borrow exact bounded original even after rejection; success transfers it.
    #[allow(dead_code)]
    pub(crate) fn retained_reply(&self)->Option<&[u8]> { self.stream.retained_reply() }
    /// Retain current unverified usage/model facts, including failure quality.
    /// Encoding errors preserve causes; no invoice, refund or debit is implied.
    #[allow(dead_code)]
    pub(crate) fn provider_facts(&self)->Result<crate::GoRaw,E> { self.stream.provider_facts() }
    /// Latch an owner-observed IO/drain failure; a terminal reply cannot heal it.
    #[allow(dead_code)]
    pub(crate) fn incomplete_transport(&mut self) { self.stream.incomplete_transport(); }
}
impl Write for ManagedReviewOutputStream<'_> {
    /// Preserve accepted byte count and typed shared transport errors through IO.
    fn write(&mut self, bytes:&[u8])->io::Result<usize> { self.stream.write(bytes) }
    /// Memory flush is neither EOF nor successful domain decoding.
    fn flush(&mut self)->io::Result<()> { self.stream.flush() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ManagedReviewContext, ManagedReviewTaskContext, ManagedTaskReview, ReviewReplyState};

    /// Two-task supplied fixture, not accepted historical evidence or identity.
    fn fixture()->(EngineSettingsValues,ManagedReviewProtocol,serde_json::Value) {
        let values=EngineSettingsValues::installation_defaults().unwrap();
        let context=ManagedReviewContext {
            citation_contract_version:2,inspection_scope_version:1,inspection_only:true,
            candidate_commit:"a".repeat(40),
            tasks:Some(["first","second"].iter().map(|id|ManagedReviewTaskContext {
                task:(*id).into(), criteria:Some(vec!["Required behavior".into()]),
                report:format!("{id}: complete original evidence with Unicode élève"),
                binding:ManagedTaskReview { task:(*id).into(),report:format!("docs/{id}.md"),..Default::default() },
                ..Default::default()
            }).collect()),..Default::default()
        };
        let protocol=crate::managed_review_protocol::build(context,values.review_reply_limits().unwrap()).unwrap();
        let tasks:Vec<_>=protocol.context.tasks.as_ref().unwrap().iter().map(|task| {
            let reference=protocol.catalog.quotes.iter().find(|q|q.text==task.report).unwrap().id;
            serde_json::json!({"task":task.task,"reason":"Complete supplied evidence reviewed","criteria":[{"index":1,"verdict":"pass","evidence_ref":reference}]})
        }).collect();
        let reply=serde_json::json!({"catalog_sha256":protocol.catalog_sha256,"candidate_commit":protocol.context.candidate_commit,"tasks":tasks});
        (values,protocol,reply)
    }
    /// Exact shared Codex framing; absence of terminal is intentional when asked.
    fn events(reply:&[u8], terminal:bool)->Vec<u8> {
        let mut raw=format!("{}\n",serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"reply","text":std::str::from_utf8(reply).unwrap()}}));
        if terminal { raw+=&format!("{}\n",serde_json::json!({"type":"turn.completed","usage":{"input_tokens":7,"output_tokens":8}})); }
        raw.into_bytes()
    }
    /// Same-settings complete valid witness before each independently injected fault.
    fn healthy(values:&EngineSettingsValues,protocol:&ManagedReviewProtocol,reply:&[u8]) {
        let mut stream=ManagedReviewOutputStream::from_saved_settings(values,protocol).unwrap();
        let framed=events(reply,true);
        assert_eq!(stream.write(&framed).unwrap(),framed.len());stream.flush().unwrap();
        assert_eq!(stream.retained_reply(),Some(reply));
        let output=stream.finish().unwrap();assert_eq!(output.review.state,ReviewReplyState::Passed);
        assert_eq!(output.review.tasks.len(),2);assert_eq!(output.transport.response,reply);
        assert!(stream.retained_reply().is_none());assert!(stream.finish().is_err());
        assert_eq!(decode_retained_managed_review(reply,values,protocol).unwrap().tasks.len(),2);
    }
    #[test]
    fn complete_two_task_literal_citations_and_retained_revalidation() {
        let (values,protocol,reply)=fixture();healthy(&values,&protocol,&serde_json::to_vec(&reply).unwrap());
    }
    #[test]
    fn domain_failure_keeps_original_and_cannot_finish_again() {
        let (values,protocol,mut reply)=fixture();let raw=serde_json::to_vec(&reply).unwrap();healthy(&values,&protocol,&raw);
        reply["tasks"].as_array_mut().unwrap().pop();let raw=serde_json::to_vec(&reply).unwrap();
        let mut stream=ManagedReviewOutputStream::from_saved_settings(&values,&protocol).unwrap();stream.feed(&events(&raw,true)).unwrap();
        assert!(stream.finish().is_err());assert_eq!(stream.retained_reply(),Some(raw.as_slice()));
        assert!(stream.finish().is_err());assert_eq!(stream.retained_reply(),Some(raw.as_slice()));
        assert!(stream.provider_facts().is_ok());
    }
    #[test]
    fn canonical_escaped_duplicates_aliases_and_private_domain_refuse() {
        let (values,protocol,reply)=fixture();let raw=serde_json::to_vec(&reply).unwrap();healthy(&values,&protocol,&raw);
        let duplicate=format!("{{\"t\\u0061sks\":[],{}",std::str::from_utf8(&raw).unwrap().trim_start_matches('{'));
        let mut alias=reply.clone();alias["tasks"][0]["Criteria"]=serde_json::json!([]);
        let mut nested=reply.clone();nested["tasks"][0]["criteria"][0]["Verdict"]=serde_json::json!("pass");
        for bad in [duplicate.into_bytes(),serde_json::to_vec(&alias).unwrap(),serde_json::to_vec(&nested).unwrap(),b"PRIVATE invalid domain".to_vec()] {
            let mut stream=ManagedReviewOutputStream::from_saved_settings(&values,&protocol).unwrap();stream.feed(&events(&bad,true)).unwrap();
            let error=match stream.finish(){Err(e)=>e,Ok(_)=>panic!("invalid domain accepted")};
            assert!(!error.to_string().contains("PRIVATE"));assert_eq!(stream.retained_reply(),Some(bad.as_slice()));
            assert!(decode_retained_managed_review(&bad,&values,&protocol).is_err());
        }
    }
    #[test]
    fn tool_terminal_and_owner_io_failures_never_become_success() {
        let (values,protocol,reply)=fixture();let raw=serde_json::to_vec(&reply).unwrap();healthy(&values,&protocol,&raw);
        let mut partial=ManagedReviewOutputStream::from_saved_settings(&values,&protocol).unwrap();partial.feed(&events(&raw,false)).unwrap();partial.flush().unwrap();
        assert!(partial.finish().is_err());assert_eq!(partial.retained_reply(),Some(raw.as_slice()));
        let mut broken=ManagedReviewOutputStream::from_saved_settings(&values,&protocol).unwrap();broken.feed(&events(&raw,true)).unwrap();broken.incomplete_transport();
        assert!(broken.finish().is_err());assert_eq!(broken.retained_reply(),Some(raw.as_slice()));
        let mut tool=ManagedReviewOutputStream::from_saved_settings(&values,&protocol).unwrap();
        let event=b"{\"type\":\"item.started\",\"item\":{\"type\":\"command_execution\",\"id\":\"tool\",\"command\":\"echo forbidden\"}}\n";
        assert!(tool.feed(event).is_err());assert!(tool.finish().is_err());
    }
    #[test]
    fn saved_zero_reply_cap_and_stale_protocol_digest_refuse() {
        let (mut values,mut protocol,reply)=fixture();let raw=serde_json::to_vec(&reply).unwrap();healthy(&values,&protocol,&raw);
        let saved=values.review_response_max_bytes;values.review_response_max_bytes=Some(crate::SettingCap(Some(crate::SettingCount(0))));
        let mut stream=ManagedReviewOutputStream::from_saved_settings(&values,&protocol).unwrap();assert!(stream.feed(&events(&raw,true)).is_err());assert!(stream.finish().is_err());
        assert!(decode_retained_managed_review(&raw,&values,&protocol).is_err());values.review_response_max_bytes=saved;
        protocol.context_sha256="0".repeat(64);assert!(ManagedReviewOutputStream::from_saved_settings(&values,&protocol).is_err());
        protocol.context_sha256=crate::provider_configuration::configuration_digest(&crate::encode_model(&protocol.context).unwrap());
        protocol.catalog_sha256="0".repeat(64);assert!(ManagedReviewOutputStream::from_saved_settings(&values,&protocol).is_err());
    }
}
