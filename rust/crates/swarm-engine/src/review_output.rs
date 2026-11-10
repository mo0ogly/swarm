//! Review-domain validation over the shared tool-free response transport.
//! Data decoding grants no process, reviewer identity or acceptance authority.
//! Shared transport preserves historical planning-* diagnostic names.
use crate::{
    AgentLaunchError as E, EngineSettingsValues, ParsedReviewReply, ProviderResponse,
    ProviderResponseStream, ReviewQuoteOriginal, ReviewReplyLimits,
};
use std::{
    collections::BTreeSet,
    io::{self, Write},
};

/// Keep original members until canonical names/uniqueness are established.
/// Allowed keys are native reply protocol vocabulary, not an operator policy.
/// Go-compatible domain parsing runs afterwards; private keys/values never enter
/// diagnostics and original boundary causes remain behind the protocol family.
pub(crate) fn canonical_fields<'a>(
    raw: &'a [u8],
    boundaries: &crate::model_wire::Boundaries,
    allowed: &[&str],
) -> Result<Vec<(String, &'a [u8])>, E> {
    let fields = crate::model_wire::object(raw.trim_ascii(), boundaries)
        .map_err(|e| E::caused("review_fragment_protocol", e))?;
    let mut seen = BTreeSet::new();
    for (key, _) in &fields {
        let reason = if !allowed.contains(&key.as_str()) {
            Some("review-reply-noncanonical-field")
        } else if !seen.insert(key.as_str()) {
            Some("review-reply-duplicate-field")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(E::caused(
                "review_fragment_protocol",
                crate::ReviewReplyError::Refused(reason),
            ));
        }
    }
    Ok(fields)
}
/// Refuse native reply ambiguity before the legacy Go-compatible projector can
/// merge duplicates or aliases. Null/absent criteria retain zero-count semantics;
/// this establishes no source origin, identity, execution or review authority.
fn unambiguous_reply(raw: &[u8]) -> Result<(), E> {
    std::str::from_utf8(raw).map_err(|e| E::caused("review_fragment_protocol", e))?;
    let boundaries = crate::agent_tool_guard::validated_json(raw)
        .map_err(|e| E::caused("review_fragment_protocol", e))?;
    let fields = canonical_fields(raw, &boundaries, &["reason", "criteria"])?;
    for (key, value) in fields {
        if key == "criteria" && value.trim_ascii() != b"null" {
            for criterion in crate::model_wire::array(value.trim_ascii(), &boundaries)
                .map_err(|e| E::caused("review_fragment_protocol", e))?
            {
                canonical_fields(criterion, &boundaries, &["index", "verdict", "evidence"])?;
            }
        }
    }
    Ok(())
}

/// Revalidate a retained original under actual current saved reviewer limits.
/// This uses the same canonical/domain guards as live finish, without inventing
/// JSONL events, terminal completeness, usage or process authority. Caller must
/// independently validate the original journal, ledger, sources and known wait.
/// Invalid/missing policy, caps, ambiguous fields and quote/domain errors retain
/// safe typed causes. The native publisher revalidates its original using this
/// shared decoder; no files, mutation, EOF or provider call occurs here.
pub(crate) fn decode_retained_review(
    raw: &[u8],
    values: &EngineSettingsValues,
    count: usize,
    originals: &[ReviewQuoteOriginal<'_>],
) -> Result<ParsedReviewReply, E> {
    values.validate().map_err(|e| E::caused(e.reason(), e))?;
    let cap = values
        .review_response_limit()
        .map_err(|e| E::caused(e.reason(), e))?;
    if cap.is_some_and(|n| raw.len() > n) {
        return Err(E::Refused("review-publication-reply-cap"));
    }
    let limits = values
        .review_reply_limits()
        .map_err(|e| E::caused(e.reason(), e))?;
    unambiguous_reply(raw)?;
    crate::parse_review_reply(raw, count, originals, limits).map_err(|e| E::caused(e.code(), e))
}

/// Complete original transport and strictly decoded review data. Provider usage
/// and model announcements remain unverified; a passed criterion is not engine
/// acceptance, independent reviewer provenance or authenticated source evidence.
pub struct ReviewProviderOutput {
    pub review: ParsedReviewReply,
    pub transport: ProviderResponse,
}
/// Borrow immutable supplied originals/count and freeze saved review-text limits.
/// No source authentication is inferred. The owning runtime must bind all inputs
/// and the explicit response cap to its persisted policy and fresh authorization.
pub struct ReviewOutputStream<'a> {
    stream: ProviderResponseStream,
    limits: ReviewReplyLimits,
    criterion_count: usize,
    originals: &'a [ReviewQuoteOriginal<'a>],
}
impl<'a> ReviewOutputStream<'a> {
    /// Freeze the saved reviewer-specific reply cap and existing role/event/text
    /// policies. Missing historical policy refuses before any bytes or process;
    /// null is explicit unlimited and zero rejects nonempty output. No planner
    /// fallback, mutation, call reservation or future invocation permit is added.
    /// Original typed policy/transport failures remain under their safe family.
    pub fn from_saved_settings(
        values: &EngineSettingsValues,
        criterion_count: usize,
        originals: &'a [ReviewQuoteOriginal<'a>],
    ) -> Result<Self, E> {
        let cap = values
            .review_response_limit()
            .map_err(|e| E::caused(e.reason(), e))?;
        Self::new(values, cap, criterion_count, originals)
    }
    /// Validate saved settings and freeze the review policy and supplied sources.
    /// response_limit is bytes: Some(0) admits no nonempty reply; None explicitly
    /// removes this reply cap, while saved event limits still apply. Callers must
    /// obtain that choice from their authoritative policy, never a planning cap
    /// fallback. Missing historical review policy and invalid settings refuse.
    /// No files/processes, budget reservation, IO, source authentication or review
    /// authority are established. Known tool declarations are forbidden, without
    /// proving OS confinement or absence of unreported external actions.
    pub fn new(
        values: &EngineSettingsValues,
        response_limit: Option<usize>,
        criterion_count: usize,
        originals: &'a [ReviewQuoteOriginal<'a>],
    ) -> Result<Self, E> {
        values.validate().map_err(|e| E::caused(e.reason(), e))?;
        let limits = values
            .review_reply_limits()
            .map_err(|e| E::caused(e.reason(), e))?;
        Ok(Self {
            stream: ProviderResponseStream::new_tool_free(values, response_limit)?,
            limits,
            criterion_count,
            originals,
        })
    }
    /// Feed exact JSONL bytes under frozen caps; framing/tool failures latch and
    /// retain their safe typed causes. This is neither EOF nor a review verdict.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.stream.feed(bytes)
    }
    /// Serialize current unverified usage/model observations, including after a
    /// failed reply. Invalid/incomplete facts retain their diagnostic history;
    /// absent cost is never zero, settlement or a refund. Encoding failures keep
    /// their cause. This consumes no reply and performs no IO or budget mutation.
    pub fn provider_facts(&self) -> Result<crate::GoRaw, E> {
        self.stream.provider_facts()
    }
    /// Clone current unverified provenance/quality, including after output failure.
    /// No serialization, invoice, settlement, process or authority is inferred.
    /// Candidate-only until the production reviewer controller consumes it.
    #[cfg(test)]
    pub(crate) fn facts_snapshot(&self) -> crate::agent_provider_facts::ProviderFactsSnapshot {
        self.stream.facts_snapshot()
    }
    /// Borrow exact already-bounded candidate reply bytes for private evidence,
    /// even after rejected domain decoding or incomplete transport. This grants
    /// no valid verdict, terminal/output completeness or publication authority.
    /// Outer undecodable JSONL is not reconstructed here; raw capture remains a
    /// separate saved-policy obligation. No allocation/IO; successful finish
    /// transfers bytes to ReviewProviderOutput, so this then returns None.
    /// Used by the native journal; rejected bytes remain evidence, never a verdict.
    pub(crate) fn retained_reply(&self) -> Option<&[u8]> {
        self.stream.retained_reply()
    }
    /// Latch an owner-observed IO/drain failure so a complete-looking reply cannot
    /// later succeed. This reports no measured usage, process reap or authority.
    pub fn incomplete_transport(&mut self) {
        self.stream.incomplete_transport();
    }
    /// Finish once after the owner observed EOF. Decode exact original bytes with
    /// the frozen criterion count, supplied quote sources and saved text limits.
    /// Missing/ambiguous terminal, tools, prior IO or domain errors refuse without
    /// partial success or private input in diagnostics. Native keys must be canonical
    /// and unique before Go-compatible domain projection; original codec/quote causes
    /// survive. A failed decode cannot be retried on this stream. Returned data
    /// does not establish reviewer independence, authenticated sources or acceptance.
    pub fn finish(&mut self) -> Result<ReviewProviderOutput, E> {
        let limits = self.limits;
        let count = self.criterion_count;
        let originals = self.originals;
        let (transport, review) = self.stream.finish_decoded(|raw| {
            unambiguous_reply(raw)?;
            crate::parse_review_reply(raw, count, originals, limits)
                .map_err(|e| E::caused(e.code(), e))
        })?;
        Ok(ReviewProviderOutput { review, transport })
    }
}
impl Write for ReviewOutputStream<'_> {
    /// Preserve actual accepted byte count and typed parser errors through IO.
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stream.write(bytes)
    }
    /// Memory flush is no EOF, reviewer verdict or permission to apply a result.
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(test)]
mod original_tests {
    use super::*;
    const REPLY: &str = r#"{"reason":"Valid rationale","criteria":[{"index":1,"verdict":"pass","evidence":"Original exact evidence"}]}"#;
    /// Protocol fixture encoding, not response policy or provider identity.
    fn codex(reply: &str) -> Vec<u8> {
        format!("{}\n{}\n", serde_json::json!({"type":"item.completed","item":{"type":"agent_message","id":"reply","text":reply}}), serde_json::json!({"type":"turn.completed","usage":{"input_tokens":7,"output_tokens":8}})).into_bytes()
    }
    /// Same-policy/source valid witness before each fault; successful finish
    /// transfers exact bytes rather than leaving a second successful copy.
    fn healthy(values: &EngineSettingsValues, originals: &[ReviewQuoteOriginal<'_>]) {
        let mut stream = ReviewOutputStream::from_saved_settings(values, 1, originals).unwrap();
        stream.feed(&codex(REPLY)).unwrap();
        assert_eq!(stream.retained_reply(), Some(REPLY.as_bytes()));
        assert_eq!(
            stream.finish().unwrap().transport.response,
            REPLY.as_bytes()
        );
        assert!(stream.retained_reply().is_none());
    }
    /// Preserve invalid private domain bytes after failure without emitting them
    /// in safe diagnostics or permitting a second finish as a successful result.
    #[test]
    fn review_original_reply_domain_failure_preserves_original() {
        let values = EngineSettingsValues::installation_defaults().unwrap();
        let originals = [ReviewQuoteOriginal {
            name: "report.md",
            kind: "source",
            content: "Original exact evidence from the supplied report",
        }];
        healthy(&values, &originals);
        let mut stream = ReviewOutputStream::from_saved_settings(&values, 1, &originals).unwrap();
        let original = "PRIVATE invalid domain reply";
        stream.feed(&codex(original)).unwrap();
        assert_eq!(stream.retained_reply(), Some(original.as_bytes()));
        let error = match stream.finish() {
            Ok(_) => panic!("invalid domain unexpectedly accepted"),
            Err(error) => error,
        };
        assert!(!error.to_string().contains("PRIVATE"));
        assert_eq!(stream.retained_reply(), Some(original.as_bytes()));
        assert!(stream.finish().is_err());
        assert_eq!(stream.retained_reply(), Some(original.as_bytes()));
    }
    /// A complete-looking reply remains private evidence after IO failure;
    /// canonical usage becomes incomplete/unknown rather than measured zero.
    #[test]
    fn review_original_reply_io_failure_keeps_data_without_success() {
        let values = EngineSettingsValues::installation_defaults().unwrap();
        let originals = [ReviewQuoteOriginal {
            name: "report.md",
            kind: "source",
            content: "Original exact evidence from the supplied report",
        }];
        healthy(&values, &originals);
        let mut stream = ReviewOutputStream::from_saved_settings(&values, 1, &originals).unwrap();
        stream.feed(&codex(REPLY)).unwrap();
        stream.incomplete_transport();
        assert!(stream.finish().is_err());
        assert_eq!(stream.retained_reply(), Some(REPLY.as_bytes()));
        let facts = stream.facts_snapshot();
        assert_eq!(
            facts.usage_state,
            crate::agent_provider_facts::FactState::Incomplete
        );
        assert!(facts.usage.is_none());
    }
    /// Absent and cap-rejected candidates stay absent; no empty original or
    /// successful reply is manufactured. Zero here is explicit fixture policy.
    #[test]
    fn review_original_reply_absence_and_cap_refusal_stay_unknown() {
        let values = EngineSettingsValues::installation_defaults().unwrap();
        let originals = [ReviewQuoteOriginal {
            name: "report.md",
            kind: "source",
            content: "Original exact evidence from the supplied report",
        }];
        healthy(&values, &originals);
        let mut empty = ReviewOutputStream::from_saved_settings(&values, 1, &originals).unwrap();
        assert!(empty.retained_reply().is_none());
        assert!(empty.finish().is_err());
        assert!(empty.retained_reply().is_none());
        let mut blocked = ReviewOutputStream::new(&values, Some(0), 1, &originals).unwrap();
        assert!(blocked.feed(&codex(REPLY)).is_err());
        assert!(blocked.finish().is_err());
        assert!(blocked.retained_reply().is_none());
    }
}
