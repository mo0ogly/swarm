//! One native runner adapter for ordinary and complete-cohort supplied replies.
use crate::{AgentLaunchError as E, EngineSettingsValues, ReviewQuoteOriginal};
use crate::managed_review_protocol::ManagedReviewProtocol;

/// One owned decoded opinion; shared transport observations retain original
/// bytes/facts and grant no acceptance or authenticated provider identity.
pub(super) struct Output {
    pub transport: crate::ProviderResponse,
    /// Test-only aggregate observation; no production publication authority.
    #[cfg(test)]
    pub state:crate::ReviewReplyState,
}
/// Select the exact source-domain decoder once, without ordinary fallback for a
/// managed protocol. Owner establishes actual wait/EOF separately. No Debug of
/// private reports/context and no new operator policy or invocation grant.
pub(super) enum Stream<'a> {
    Ordinary(crate::ReviewOutputStream<'a>),
    Managed(crate::managed_review_output::ManagedReviewOutputStream<'a>),
}
impl<'a> Stream<'a> {
    /// Bind saved policy to the explicit complete managed protocol or the actual
    /// ordinary source slice. Failure retains typed schema/limit/source causes;
    /// no retries, truncation, process, budget or permission is created.
    pub fn new(values:&EngineSettingsValues,count:usize,originals:&'a [ReviewQuoteOriginal<'a>],protocol:Option<&'a ManagedReviewProtocol>)->Result<Self,E>{
        match protocol {
            Some(protocol)=>crate::managed_review_output::ManagedReviewOutputStream::from_saved_settings(values,protocol).map(Self::Managed),
            None=>crate::ReviewOutputStream::from_saved_settings(values,count,originals).map(Self::Ordinary),
        }
    }
    /// Feed unchanged bytes through the selected tool-free shared transport.
    pub fn feed(&mut self,raw:&[u8])->Result<(),E>{match self{Self::Ordinary(s)=>s.feed(raw),Self::Managed(s)=>s.feed(raw)}}
    /// Finish exactly once; validate the entire selected domain before retaining
    /// transport for journaling. Publication independently redecodes the original.
    pub fn finish(&mut self)->Result<Output,E>{match self{Self::Ordinary(s)=>s.finish().map(|o|Output{transport:o.transport,#[cfg(test)]state:o.review.state}),Self::Managed(s)=>s.finish().map(|o|Output{transport:o.transport,#[cfg(test)]state:o.review.state})}}
    /// Preserve exact bounded rejected original; success transfers those bytes.
    pub fn retained_reply(&self)->Option<&[u8]>{match self{Self::Ordinary(s)=>s.retained_reply(),Self::Managed(s)=>s.retained_reply()}}
    /// Test-only observation of original unverified shared transport quality.
    #[cfg(test)]
    pub fn facts_snapshot(&self)->crate::agent_provider_facts::ProviderFactsSnapshot{match self{Self::Ordinary(s)=>s.facts_snapshot(),Self::Managed(s)=>s.facts_snapshot()}}
    /// Retain original unverified facts after success/failure, never settlement.
    pub fn provider_facts(&self)->Result<crate::GoRaw,E>{match self{Self::Ordinary(s)=>s.provider_facts(),Self::Managed(s)=>s.provider_facts()}}
    /// Latch owner/capture IO failure so terminal-looking output cannot heal it.
    pub fn incomplete_transport(&mut self){match self{Self::Ordinary(s)=>s.incomplete_transport(),Self::Managed(s)=>s.incomplete_transport()}}
}
/// Decode an internally rebuilt protocol through actual canonical JSON bytes so
/// opaque receipts preserve their GoRaw representation. Caller cannot supply it.
/// Codec failure propagates; None denotes an actual ordinary input only.
pub(super) fn protocol(inputs:&crate::TaskReviewInputs)->Result<Option<ManagedReviewProtocol>,crate::RootStoreError>{
    inputs.managed_protocol.as_ref().map(|value| {
        let bytes=crate::encode_model(value).map_err(crate::RootStoreError::Encode)?;
        serde_json::from_slice(&bytes).map_err(|e|crate::RootStoreError::Launch(Box::new(E::caused("review-managed-protocol-invalid",e))))
    }).transpose()
}
