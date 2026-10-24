//! Strict supplied-data reply parsing. A parsed result grants no reviewer authority.
use crate::{
    model_wire::{Fields, Kind},
    ReviewCriterion, ReviewQuoteError, WorkError,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};

/// Explicit operator values: trimmed minimum, raw maximum and normalized fallback minimum.
/// Zero removes a minimum; None removes the maximum. No implicit installation default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewReplyLimits {
    pub min_text_bytes: usize,
    pub max_text_bytes: Option<usize>,
    pub min_normalized_quote_bytes: usize,
}

/// Borrowed supplied original text. Name/kind do not establish origin, rights or freshness.
pub struct ReviewQuoteOriginal<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub content: &'a str,
}

/// Syntactic aggregate of declared criterion verdicts, not an accepted review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewReplyState {
    Passed,
    ChangesRequested,
}

/// Complete parsed data; criteria retain nil/empty and supplied order as Go does.
/// Reason/evidence remain private text and are deliberately omitted from Debug.
#[derive(Serialize)]
pub struct ParsedReviewReply {
    pub state: ReviewReplyState,
    pub reason: String,
    pub criteria: Option<Vec<ReviewCriterion>>,
}
impl fmt::Debug for ParsedReviewReply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParsedReviewReply")
            .field("state", &self.state)
            .field("criteria", &self.criteria.as_ref().map(Vec::len))
            .finish()
    }
}

/// Safe failure preserving codec/allocation causes; supplied private text is never displayed.
pub enum ReviewReplyError {
    Format(WorkError),
    Refused(&'static str),
    Quote(ReviewQuoteError),
}
impl ReviewReplyError {
    /// Protocol errors share Go's stable family; allocation failure remains operational.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Quote(e) => e.code(),
            _ => "review_fragment_protocol",
        }
    }
    /// Safe diagnostic reason, separate from the public failure family.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Format(_) => "review-reply-format",
            Self::Refused(r) => r,
            Self::Quote(e) => e.code(),
        }
    }
}
impl fmt::Display for ReviewReplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}
impl fmt::Debug for ReviewReplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ReviewReplyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Format(e) => Some(e),
            Self::Quote(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Reply {
    reason: String,
    criteria: Option<Vec<ReviewCriterion>>,
}

/// Match the actual anonymous Go reply DTO, reusing the criterion producer shape.
fn shape() -> Fields {
    &[
        ("reason", Kind::String),
        (
            "criteria",
            Kind::Array(&Kind::Object(crate::models::shape_review_criterion)),
        ),
    ]
}

/// Validate byte lengths independently from JSON Schema's character-length hints.
fn text_valid(value: &str, limits: ReviewReplyLimits) -> bool {
    crate::text::trim_space(value).len() >= limits.min_text_bytes
        && limits.max_text_bytes.is_none_or(|max| value.len() <= max)
}

/// Decode and validate supplied reply data with Go strict/null/duplicate/key-case semantics.
///
/// `criterion_count` is the expected count, not a verified task. Every index must
/// appear once in 1..=count. Pass requires one literal/permitted prose match in
/// supplied originals; fail/unknown require text but no matching quote and produce
/// ChangesRequested. A zero-criterion reply follows Go data semantics. No I/O,
/// persistence, reviewer identity, independent judgment or task acceptance occurs.
/// Callers must obtain current policy and authenticated originals separately.
///
/// # Errors
/// Refuses malformed/unknown JSON, incomplete/incoherent data, unknown verdicts,
/// missing quotes or inconsistent limits, without a partial result or private echo.
/// Codec and fallible quote-allocation causes are retained. UTF-8 input is required.
pub fn parse_review_reply(
    raw: &[u8],
    criterion_count: usize,
    originals: &[ReviewQuoteOriginal<'_>],
    limits: ReviewReplyLimits,
) -> Result<ParsedReviewReply, ReviewReplyError> {
    if limits
        .max_text_bytes
        .is_some_and(|max| max < limits.min_text_bytes)
    {
        return Err(ReviewReplyError::Refused(
            "review-reply-limits-inconsistent",
        ));
    }
    let reply: Reply =
        crate::work::decode_mode(raw, shape(), true).map_err(ReviewReplyError::Format)?;
    if !text_valid(&reply.reason, limits)
        || reply.criteria.as_ref().map_or(0, Vec::len) != criterion_count
    {
        return Err(ReviewReplyError::Refused("review-reply-incomplete"));
    }
    let mut seen = BTreeSet::new();
    let mut state = ReviewReplyState::Passed;
    for criterion in reply.criteria.iter().flatten() {
        if criterion.index < 1
            || usize::try_from(criterion.index).map_or(true, |index| index > criterion_count)
            || !seen.insert(criterion.index)
            || !text_valid(&criterion.evidence, limits)
        {
            return Err(ReviewReplyError::Refused(
                "review-reply-criterion-incomplete",
            ));
        }
        match criterion.verdict.as_str() {
            "pass" => {
                let mut present = false;
                for original in originals {
                    if crate::review_evidence_present(
                        original.name,
                        original.kind,
                        original.content,
                        &criterion.evidence,
                        limits.min_normalized_quote_bytes,
                    )
                    .map_err(ReviewReplyError::Quote)?
                    {
                        present = true;
                        break;
                    }
                }
                if !present {
                    return Err(ReviewReplyError::Refused("review-reply-quote-absent"));
                }
            }
            "fail" | "unknown" => state = ReviewReplyState::ChangesRequested,
            _ => return Err(ReviewReplyError::Refused("review-reply-verdict-unknown")),
        }
    }
    Ok(ParsedReviewReply {
        state,
        reason: reply.reason,
        criteria: reply.criteria,
    })
}
