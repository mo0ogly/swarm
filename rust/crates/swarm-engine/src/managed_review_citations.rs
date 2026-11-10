//! Complete supplied-data citation catalogue, not identity or review acceptance.
//! Go version1 spans preserve every original byte; no truncation, file IO or calls.
//! Quote admission uses the caller's persisted review limits, not legacy constants.
use serde::{Deserialize, Serialize};

#[path = "managed_review_citation_json.rs"]
mod json;

use json::{json_string_content, segments};

use crate::provider_configuration::configuration_digest;
use crate::ReviewReplyError;

/// Go catalogue-v1 partition sizes. All bytes remain represented; these are
/// transport representation invariants, not reply/quote admission policy.
const CHUNK: usize = 768;
const TAIL_MIN: usize = 8;

fn no(value: &bool) -> bool {
    !*value
}

/// Match Go's hex.DecodeString plus32-byte representation check. It accepts
/// either hex case without normalizing the supplied identity. Computed hashes
/// remain lowercase and exact equality remains required; this grants no source
/// authenticity. Do not reuse the stricter configuration-digest wire policy.
fn citation_hash_valid(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn refused(msg: &'static str) -> ReviewReplyError {
    ReviewReplyError::Refused(msg)
}

/// Exact Go JSON field names.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CitationSpan {
    #[serde(rename = "json_string", default, skip_serializing_if = "no")]
    pub json_string: bool,
    #[serde(rename = "reference")]
    pub reference: i64,
    #[serde(rename = "byte_offset")]
    pub byte_offset: i64,
    #[serde(rename = "bytes")]
    pub bytes: i64,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CitationOriginal {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "sha256")]
    pub sha256: String,
    #[serde(rename = "bytes")]
    pub bytes: i64,
    #[serde(rename = "spans")]
    pub spans: Vec<CitationSpan>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CitationQuote {
    #[serde(rename = "id")]
    pub id: i64,
    #[serde(rename = "text")]
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CitationCatalog {
    #[serde(rename = "version")]
    pub version: i64,
    #[serde(rename = "candidate_commit")]
    pub candidate_commit: String,
    #[serde(rename = "context_sha256")]
    pub context_sha256: String,
    #[serde(rename = "originals")]
    pub originals: Vec<CitationOriginal>,
    #[serde(rename = "quotes")]
    pub quotes: Vec<CitationQuote>,
}

/// Input source (replaces Go managedReviewFragmentArtifact for this producer).
#[derive(Clone)]
pub(crate) struct CitationSource {
    pub name: String,
    pub kind: String,
    pub content: String,
}

#[derive(Clone)]
pub(crate) struct CitationResolver {
    quotes: Vec<String>,
    literal: Vec<bool>,
}

fn span_text(quote: &str, json_span: bool) -> Result<String, ReviewReplyError> {
    if json_span {
        json_string_content(quote)
    } else {
        Ok(quote.to_string())
    }
}

fn to_i64(v: usize, msg: &'static str) -> Result<i64, ReviewReplyError> {
    i64::try_from(v).map_err(|_| refused(msg))
}

fn to_usize(v: i64, msg: &'static str) -> Result<usize, ReviewReplyError> {
    usize::try_from(v).map_err(|_| refused(msg))
}

/// Exact Go splitManagedCitationText: UTF-8-boundary chunks of 768; if the
/// remainder after cutting would be < 8 bytes, stop and keep the ENTIRE
/// remaining text as the final tail (Go `break` policy, never truncates).
pub(crate) fn split_text(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let bytes = text.as_bytes();
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0usize;
    while bytes.len() - start > CHUNK {
        let mut end = start + CHUNK;
        while end > start && !is_char_boundary(bytes, end) {
            end -= 1;
        }
        if bytes.len() - end < TAIL_MIN {
            break;
        }
        parts.push(text[start..end].to_string());
        start = end;
    }
    parts.push(text[start..].to_string());
    parts
}

fn is_char_boundary(bytes: &[u8], i: usize) -> bool {
    i >= bytes.len() || (bytes[i] & 0xC0) != 0x80
}

/// Builds the catalog from supplied originals. All offsets/byte counts are
/// computed from reconstruction; conversions Go int => i64 are checked.
pub(crate) fn build_catalog(
    candidate: &str,
    context_digest: &str,
    originals: &[CitationSource],
) -> Result<CitationCatalog, ReviewReplyError> {
    if candidate.is_empty() || !citation_hash_valid(context_digest) {
        return Err(refused("invalid evidence catalog identity"));
    }
    let mut catalog = CitationCatalog {
        version: 1,
        candidate_commit: candidate.to_string(),
        context_sha256: context_digest.to_string(),
        originals: Vec::new(),
        quotes: Vec::new(),
    };
    let mut indexes: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for original in originals {
        if original.name.is_empty() || original.kind.is_empty() {
            return Err(refused("invalid catalog original"));
        }
        let segs = segments(&original.content)?;
        let mut entry = CitationOriginal {
            name: original.name.clone(),
            kind: original.kind.clone(),
            sha256: configuration_digest(original.content.as_bytes()),
            bytes: to_i64(original.content.len(), "original too large")?,
            spans: Vec::new(),
        };
        let mut offset: i64 = 0;
        for segment in segs {
            for text in split_text(&segment.text) {
                let id = match indexes.get(&text) {
                    Some(&id) => id,
                    None => {
                        let id = to_i64(catalog.quotes.len(), "too many quotes")?;
                        indexes.insert(text.clone(), id);
                        catalog.quotes.push(CitationQuote {
                            id,
                            text: text.clone(),
                        });
                        id
                    }
                };
                let rebuilt = span_text(
                    &catalog.quotes[to_usize(id, "bad id")?].text,
                    segment.json_string,
                )?;
                let len = to_i64(rebuilt.len(), "span too large")?;
                entry.spans.push(CitationSpan {
                    json_string: segment.json_string,
                    reference: id,
                    byte_offset: offset,
                    bytes: len,
                });
                offset = offset
                    .checked_add(len)
                    .ok_or_else(|| refused("span offset overflow"))?;
            }
        }
        if offset != entry.bytes {
            return Err(refused("citation-original-partition-incomplete"));
        }
        catalog.originals.push(entry);
    }
    Ok(catalog)
}

impl CitationCatalog {
    /// Canonical Go JSON digest of the catalog.
    pub(crate) fn digest(&self) -> Result<String, ReviewReplyError> {
        let raw = crate::encode_model(self).map_err(ReviewReplyError::Format)?;
        Ok(configuration_digest(&raw))
    }

    /// Validates the catalog against attribution and reconstructs the resolver.
    pub(crate) fn resolver(
        &self,
        candidate: &str,
        context_digest: &str,
        expected_digest: &str,
    ) -> Result<CitationResolver, ReviewReplyError> {
        if self.version != 1
            || candidate.is_empty()
            || self.candidate_commit != candidate
            || self.context_sha256 != context_digest
            || !citation_hash_valid(context_digest)
            || !citation_hash_valid(expected_digest)
            || self.digest()? != expected_digest
        {
            return Err(refused("stale or unattributed evidence catalog"));
        }
        let mut quotes: Vec<String> = Vec::with_capacity(self.quotes.len());
        let mut literal: Vec<bool> = vec![false; self.quotes.len()];
        for (i, q) in self.quotes.iter().enumerate() {
            if q.id != to_i64(i, "quote id overflow")? || q.text.len() > CHUNK + TAIL_MIN - 1 {
                return Err(refused("invalid catalog reference"));
            }
            quotes.push(q.text.clone());
        }
        let mut used = vec![false; quotes.len()];
        for original in &self.originals {
            if original.name.is_empty()
                || original.kind.is_empty()
                || !citation_hash_valid(&original.sha256)
                || original.spans.is_empty()
            {
                return Err(refused("unattributed catalog original"));
            }
            let mut text = String::new();
            for span in &original.spans {
                let r = to_usize(span.reference, "bad span reference")?;
                if span.reference < 0
                    || r >= quotes.len()
                    || span.byte_offset != to_i64(text.len(), "span offset overflow")?
                {
                    return Err(refused("missing or modified original range"));
                }
                let rebuilt = span_text(&quotes[r], span.json_string)?;
                if rebuilt.len() != to_usize(span.bytes, "bad span bytes")? {
                    return Err(refused("modified encoded original range"));
                }
                used[r] = true;
                if !span.json_string || rebuilt == quotes[r] {
                    literal[r] = true;
                }
                text.push_str(&rebuilt);
            }
            if text.len() != to_usize(original.bytes, "bad original bytes")?
                || configuration_digest(text.as_bytes()) != original.sha256
            {
                return Err(refused("modified catalog original"));
            }
        }
        if used.iter().any(|&u| !u) {
            return Err(refused("catalog reference without original"));
        }
        Ok(CitationResolver { quotes, literal })
    }
}

/// Validate supplied operator bounds; None is explicit unlimited.
fn check_limits(limits: crate::ReviewReplyLimits) -> Result<(), ReviewReplyError> {
    if limits
        .max_text_bytes
        .is_some_and(|max| max < limits.min_text_bytes)
    {
        return Err(refused("review-reply-limits-inconsistent"));
    }
    Ok(())
}
/// Apply the same raw maximum and Go-trimmed minimum as reply admission.
fn admissible(text: &str, limits: crate::ReviewReplyLimits) -> bool {
    crate::text::trim_space(text).len() >= limits.min_text_bytes
        && limits.max_text_bytes.is_none_or(|max| text.len() <= max)
}
impl CitationResolver {
    /// Recover a literal or exact JSON-encoded original. This proves byte
    /// membership only, not criterion coverage, source identity or freshness.
    fn original_quote(&self, reference: i64) -> Result<String, ReviewReplyError> {
        let index = to_usize(reference, "citation-reference-invalid")?;
        let text = self
            .quotes
            .get(index)
            .ok_or_else(|| refused("citation-reference-invalid"))?;
        if self.literal[index] {
            Ok(text.clone())
        } else {
            json_string_content(text)
        }
    }
    /// Resolve one reference under the actual persisted reply limits. Unknown
    /// IDs, inconsistent policy and inadmissible excerpts refuse without fallback.
    pub(crate) fn quote(
        &self,
        reference: i64,
        limits: crate::ReviewReplyLimits,
    ) -> Result<String, ReviewReplyError> {
        check_limits(limits)?;
        let text = self.original_quote(reference)?;
        if !admissible(&text, limits) {
            return Err(refused("citation-excerpt-outside-policy"));
        }
        Ok(text)
    }
    /// Enumerate admissible references for the provider schema with the SAME
    /// saved bounds as decoding. Encoding failures propagate, never disappear
    /// as an empty list; no supplied original or byte is removed from catalogue.
    pub(crate) fn admissible_references(
        &self,
        limits: crate::ReviewReplyLimits,
    ) -> Result<Vec<i64>, ReviewReplyError> {
        check_limits(limits)?;
        let mut references = Vec::new();
        for index in 0..self.quotes.len() {
            let id = to_i64(index, "citation-reference-overflow")?;
            if admissible(&self.original_quote(id)?, limits) {
                references.push(id);
            }
        }
        Ok(references)
    }
}

// ---------------------------------------------------------------------------
// Tests: empty / raw / Unicode / duplicate quotes / tampered span, ref, hash.
// All run purely in-memory; none executed by this delivery (no tool access).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> crate::ReviewReplyLimits {
        crate::ReviewReplyLimits {
            min_text_bytes: 8,
            max_text_bytes: Some(1000),
            min_normalized_quote_bytes: 8,
        }
    }
    const CD: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn src(name: &str, kind: &str, content: &str) -> CitationSource {
        CitationSource {
            name: name.into(),
            kind: kind.into(),
            content: content.into(),
        }
    }

    #[test]
    fn split_empty_and_boundaries() {
        assert_eq!(split_text(""), vec![String::new()]);
        let long = "é".repeat(400); // 800 bytes, 2-byte chars
        let parts = split_text(&long);
        assert!(parts.len() >= 2);
        assert_eq!(parts.concat(), long);
        for p in &parts {
            assert!(p.len() <= 768 || parts.len() == 1 || p == parts.last().unwrap());
        }
    }

    #[test]
    fn build_rejects_bad_identity() {
        let ok = src("a.rs", "file", "hello world");
        let valid = build_catalog("c", CD, &[ok.clone()]).unwrap();
        assert!(valid.resolver("c", CD, &valid.digest().unwrap()).is_ok());
        assert!(build_catalog("", CD, &[ok.clone()]).is_err());
        assert!(build_catalog("c", "nothex", &[ok]).is_err());
        let uppercase = CD.to_uppercase();
        let catalog = build_catalog("c", &uppercase, &[]).unwrap();
        assert!(catalog
            .resolver("c", &uppercase, &catalog.digest().unwrap())
            .is_ok());
    }

    #[test]
    fn roundtrip_raw_unicode_duplicate_quotes() {
        let content = "héllo wörld".repeat(3); // duplicate text => deduped quotes
        let cat = build_catalog("c0ffee", CD, &[src("f", "file", &content)]).unwrap();
        assert_eq!(cat.version, 1);
        let uniq: std::collections::HashSet<&str> =
            cat.quotes.iter().map(|q| q.text.as_str()).collect();
        assert_eq!(uniq.len(), cat.quotes.len());
        let d = cat.digest().unwrap();
        let r = cat.resolver("c0ffee", CD, &d).unwrap();
        assert!(!r.quote(0, limits()).unwrap().is_empty());
    }

    #[test]
    fn tampered_inputs_refused() {
        let cat = build_catalog("c0ffee", CD, &[src("f", "file", "enough text here")]).unwrap();
        let d = cat.digest().unwrap();
        assert_eq!(
            cat.resolver("c0ffee", CD, &d)
                .unwrap()
                .quote(0, limits())
                .unwrap(),
            "enough text here"
        );
        // wrong candidate
        assert!(cat.resolver("other", CD, &d).is_err());
        // tampered span bytes
        let mut bad = cat.clone();
        bad.originals[0].spans[0].bytes += 1;
        assert!(bad.resolver("c0ffee", CD, &bad.digest().unwrap()).is_err());
        // tampered original hash
        let mut bad2 = cat.clone();
        bad2.originals[0].sha256 = "f".repeat(64);
        assert!(bad2
            .resolver("c0ffee", CD, &bad2.digest().unwrap())
            .is_err());
        // tampered quote text (digest changes, and ref mismatch when rebuilt)
        let mut bad3 = cat.clone();
        bad3.quotes[0].text = "tampered!".into();
        assert!(bad3.resolver("c0ffee", CD, &d).is_err());
        // tampered catalog digest
        assert!(cat.resolver("c0ffee", CD, &"a".repeat(64)).is_err());
    }

    #[test]
    fn managed_citation_json_only_and_shared_literal_quotes_preserve_actual_encoding() {
        let body = format!("{}\n<>&é", "a".repeat(800));
        let raw =
            String::from_utf8(crate::encode_model(&serde_json::json!({"source":body})).unwrap())
                .unwrap();
        let cat = build_catalog("candidate", CD, &[src("source.json", "source", &raw)]).unwrap();
        let quote = cat.quotes.iter().find(|q| q.text.contains('\n')).unwrap();
        let escaped = json_string_content(&quote.text).unwrap();
        assert_ne!(escaped, quote.text);
        let resolver = cat
            .resolver("candidate", CD, &cat.digest().unwrap())
            .unwrap();
        assert_eq!(resolver.quote(quote.id, limits()).unwrap(), escaped);
        let shared = build_catalog(
            "candidate",
            CD,
            &[
                src("source.json", "source", &raw),
                src("literal.txt", "source", &quote.text),
            ],
        )
        .unwrap();
        let shared_quote = shared.quotes.iter().find(|q| q.text == quote.text).unwrap();
        let resolver = shared
            .resolver("candidate", CD, &shared.digest().unwrap())
            .unwrap();
        assert_eq!(
            resolver.quote(shared_quote.id, limits()).unwrap(),
            quote.text
        );
    }

    #[test]
    fn managed_citation_admission_tracks_supplied_policy_without_hidden_legacy_caps() {
        let cat = build_catalog("candidate", CD, &[src("source", "source", "proof")]).unwrap();
        let resolver = cat
            .resolver("candidate", CD, &cat.digest().unwrap())
            .unwrap();
        let allowed = crate::ReviewReplyLimits {
            min_text_bytes: 3,
            max_text_bytes: Some(5),
            min_normalized_quote_bytes: 0,
        };
        assert_eq!(resolver.quote(0, allowed).unwrap(), "proof");
        assert_eq!(resolver.admissible_references(allowed).unwrap(), vec![0]);
        let denied = crate::ReviewReplyLimits {
            max_text_bytes: Some(4),
            ..allowed
        };
        assert!(resolver.quote(0, denied).is_err());
        assert!(resolver.admissible_references(denied).unwrap().is_empty());
        let unlimited = crate::ReviewReplyLimits {
            max_text_bytes: None,
            ..allowed
        };
        assert_eq!(resolver.quote(0, unlimited).unwrap(), "proof");
        let invalid = crate::ReviewReplyLimits {
            min_text_bytes: 6,
            max_text_bytes: Some(5),
            ..allowed
        };
        assert!(resolver.admissible_references(invalid).is_err());
    }
}
