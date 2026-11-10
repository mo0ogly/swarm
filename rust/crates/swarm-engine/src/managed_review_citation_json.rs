//! Pure Go-compatible segmentation of complete supplied UTF-8 originals.
use super::CHUNK;
use crate::ReviewReplyError as E;

/// A complete ordinary span or decoded canonical JSON-string content. Quotes
/// remain in adjacent ordinary spans; reconstruction must re-encode JSON spans.
pub(super) struct CitationSegment {
    pub text: String,
    pub json_string: bool,
}

/// Return Go's canonical escaped string content without surrounding quotes.
/// Preserve serializer failures; supplied data is not authenticated by encoding.
pub(super) fn json_string_content(text: &str) -> Result<String, E> {
    let raw = crate::encode_model(&text).map_err(E::Format)?;
    let body = raw
        .strip_prefix(b"\"")
        .and_then(|v| v.strip_suffix(b"\""))
        .ok_or(E::Refused("citation-json-string-encoding"))?;
    String::from_utf8(body.to_vec()).map_err(|_| E::Refused("citation-json-string-utf8"))
}

/// Segment exact original bytes without clipping or whole-document rewriting.
/// Short/non-JSON input stays ordinary. Long JSON strings are decoded only when
/// Go re-encoding reproduces their exact quoted bytes; noncanonical escapes stay
/// original. Rebuilding JSON spans with json_string_content and ordinary spans
/// verbatim recovers the complete input, including closing quotes and whitespace.
/// Rust input is UTF-8; codec failures propagate without IO or authority. The
/// shared stack-safe decoder avoids an extra Serde recursion-depth policy.
pub(super) fn segments(text: &str) -> Result<Vec<CitationSegment>, E> {
    if text.len() <= CHUNK
        || swarm_wire_json::decode::<Box<serde_json::value::RawValue>>(text.as_bytes()).is_err()
    {
        return Ok(vec![CitationSegment {
            text: text.into(),
            json_string: false,
        }]);
    }
    let bytes = text.as_bytes();
    let mut output = Vec::new();
    let (mut last, mut index) = (0, 0);
    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() && bytes[index] != b'"' {
            if bytes[index] == b'\\' {
                index += 1;
            }
            index += 1;
        }
        if index >= bytes.len() {
            return Err(E::Refused("citation-json-string-incomplete"));
        }
        let raw = &text[start..=index];
        if let Ok(decoded) = serde_json::from_str::<String>(raw) {
            if crate::encode_model(&decoded).map_err(E::Format)? == raw.as_bytes() {
                output.push(CitationSegment {
                    text: text[last..start + 1].into(),
                    json_string: false,
                });
                output.push(CitationSegment {
                    text: decoded,
                    json_string: true,
                });
                // Go retains the closing quote in the next ordinary span.
                last = index;
            }
        }
        index += 1;
    }
    output.push(CitationSegment {
        text: text[last..].into(),
        json_string: false,
    });
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reassembly follows span encoding, not a concatenation of decoded strings.
    fn rebuild(parts: &[CitationSegment]) -> String {
        parts
            .iter()
            .map(|part| {
                if part.json_string {
                    json_string_content(&part.text).unwrap()
                } else {
                    part.text.clone()
                }
            })
            .collect()
    }

    #[test]
    fn managed_citation_json_short_control_and_long_escaped_roundtrip() {
        let short = "ordinary \"source\"";
        let parts = segments(short).unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(rebuild(&parts), short);
        let body = format!("{}\"\\\n<>&\u{2028}é", "a".repeat(800));
        let original =
            String::from_utf8(crate::encode_model(&serde_json::json!({"source":body})).unwrap())
                .unwrap();
        let parts = segments(&original).unwrap();
        assert!(parts
            .iter()
            .any(|part| part.json_string && part.text == body));
        assert_eq!(rebuild(&parts), original);
    }

    #[test]
    fn managed_citation_json_noncanonical_and_invalid_remain_complete() {
        let original = format!(
            "{{\"padding\":\"{}\",\"noncanonical\":\"\\u0041\"}}",
            "p".repeat(800)
        );
        let parts = segments(&original).unwrap();
        assert!(!parts
            .iter()
            .any(|part| part.json_string && part.text == "A"));
        assert_eq!(rebuild(&parts), original);
        let invalid = format!("{{\"source\":\"{}\\x41\"}}", "p".repeat(800));
        let parts = segments(&invalid).unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(rebuild(&parts), invalid);
    }
}
