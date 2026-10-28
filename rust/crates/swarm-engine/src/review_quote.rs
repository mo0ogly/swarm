//! Original-text matching only; a present quote proves no verdict, origin or authority.
use std::{collections::TryReserveError, error::Error, fmt};

/// Fallible matcher buffer allocation, distinct from an absent quote.
pub struct ReviewQuoteError(TryReserveError);
impl ReviewQuoteError {
    /// Stable diagnostic family, excluding supplied original/quote contents.
    pub fn code(&self) -> &'static str {
        "review-quote-allocation"
    }
}
impl fmt::Display for ReviewQuoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for ReviewQuoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ReviewQuoteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

/// Reserve a raw-input-sized buffer fallibly; no policy limit or truncation is introduced.
fn buffer(capacity: usize) -> Result<String, ReviewQuoteError> {
    let mut value = String::new();
    value.try_reserve(capacity).map_err(ReviewQuoteError)?;
    Ok(value)
}

/// Append Go Fields-style prose with backticks removed and a line-join separator.
/// The caller reserves the full raw input length first; normalization cannot
/// increase its byte count, including the newline replaced between input lines.
fn append_normalized(out: &mut String, input: &str, separate: bool) {
    let mut space = separate && !out.is_empty();
    for c in input.chars() {
        if c == '`' {
            continue;
        }
        if crate::text::go_space(c) {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(c);
    }
}
/// Recognize syntax of three or more matching ASCII fence markers, without a policy ceiling.
/// A closing suffix contains whitespace only; the opener may carry language text.
fn fence(line: &str) -> Option<(u8, usize, bool)> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let width = line.bytes().take_while(|&c| c == marker).count();
    if width < 3 {
        return None;
    }
    Some((
        marker,
        width,
        crate::text::trim_space(&line[width..]).is_empty(),
    ))
}
/// Match a supplied quote in original bytes or permitted Markdown prose runs.
///
/// Exact substrings match for every kind/name, including the empty substring.
/// Callers must enforce reply completeness and admissible quote lengths first;
/// this predicate authenticates no source or reviewer and grants no acceptance.
/// Only case-sensitive `.md` names of kind `source`/`diff` allow the fallback:
/// remove backticks and join Go whitespace inside a prose paragraph. The explicit
/// `minimum_normalized_bytes` applies only to that fallback, never as a hidden
/// default or a new persisted Root policy. Root consumption still requires its
/// authoritative configured response policy, which is not introduced by this API.
///
/// Normalized runs cross no blank paragraph, diff hunk/header or +/-/context
/// boundary. Fenced code requires an exact substring; closing requires the same
/// marker, at least its opening length, and whitespace-only suffix. This is a
/// conservative fence recognizer, not a full Markdown parser or OS/file check.
/// No I/O or mutation occurs. Allocation failure returns an error, not `false`.
pub fn review_evidence_present(
    name: &str,
    kind: &str,
    content: &str,
    evidence: &str,
    minimum_normalized_bytes: usize,
) -> Result<bool, ReviewQuoteError> {
    if content.contains(evidence) {
        return Ok(true);
    }
    if !name.ends_with(".md") || !matches!(kind, "source" | "diff") {
        return Ok(false);
    }
    let mut quote = buffer(evidence.len())?;
    append_normalized(&mut quote, evidence, false);
    if quote.len() < minimum_normalized_bytes {
        return Ok(false);
    }
    if quote.len() > content.len() {
        return Ok(false);
    }
    let mut run = buffer(content.len())?;
    let mut prefix = 0u8;
    let mut active_fence = None;
    for mut line in content.split('\n') {
        let mut next = 0u8;
        if kind == "diff" {
            if line.is_empty()
                || line.starts_with("+++")
                || line.starts_with("---")
                || !matches!(line.as_bytes()[0], b'+' | b'-' | b' ')
            {
                if run.contains(&quote) {
                    return Ok(true);
                }
                run.clear();
                prefix = 0;
                continue;
            }
            next = line.as_bytes()[0];
            line = &line[1..];
        }
        let trimmed = crate::text::trim_space(line);
        if let Some((marker, width, closing)) = fence(trimmed) {
            match active_fence {
                None => {
                    if run.contains(&quote) {
                        return Ok(true);
                    }
                    active_fence = Some((marker, width));
                }
                Some((old, old_width)) if marker == old && width >= old_width && closing => {
                    active_fence = None
                }
                Some(_) => {}
            }
            run.clear();
            prefix = next;
            continue;
        }
        if active_fence.is_some() {
            run.clear();
            prefix = next;
            continue;
        }
        if trimmed.is_empty() || next != prefix {
            if run.contains(&quote) {
                return Ok(true);
            }
            run.clear();
        }
        prefix = next;
        if !trimmed.is_empty() {
            append_normalized(&mut run, line, true);
        }
    }
    Ok(run.contains(&quote))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Capacity overflow is an explicit allocation error, not an absent quote or a process-wide OOM injection.
    #[test]
    fn allocation_representation_failure_preserves_cause_and_recovers() {
        let error = buffer(usize::MAX).unwrap_err();
        assert_eq!(error.code(), "review-quote-allocation");
        assert!(error.source().is_some());
        assert_eq!(format!("{error:?}"), "review-quote-allocation");
        let mut value = buffer(16).unwrap();
        append_normalized(&mut value, "Valid `proof`", false);
        assert_eq!(value, "Valid proof");
    }
}
