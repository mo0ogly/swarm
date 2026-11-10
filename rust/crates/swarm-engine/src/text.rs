//! Go text compatibility shared by validators and historical readers.
/// Go Unicode whitespace classification shared by trimming and prose matching.
pub(crate) fn go_space(c: char) -> bool {
    matches!(c,'\u{0009}'..='\u{000d}'|'\u{0020}'|'\u{0085}'|'\u{00a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}')
}
/// Trim exactly the Go whitespace set, retaining interior text and zero-width characters.
pub(crate) fn trim_space(value: &str) -> &str {
    value.trim_matches(go_space)
}

pub(crate) fn nonempty(value: &str) -> bool {
    !trim_space(value).is_empty()
}
/// Locale-independent scalar lowercasing for Go strings.ToLower consumers.
/// Rust str lowercasing can expand dotted I or apply final-sigma context; Go
/// maps one scalar at a time. Taking the first simple-lower scalar preserves
/// these cases; tables still follow the compiled Unicode version. Pure, with
/// no normalization, IO, authentication or operator policy.
pub(crate) fn scalar_lower(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            c.to_lowercase()
                .next()
                .expect("Unicode lowercase mapping is nonempty")
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    /// A valid ordinary word precedes dotted-I and contextual-sigma witnesses.
    #[test]
    fn scalar_lower_retains_go_private_name_and_search_semantics() {
        assert_eq!(scalar_lower("Credentials.JSON"), "credentials.json");
        assert_eq!(scalar_lower("credentİals.json"), "credentials.json");
        assert_eq!(scalar_lower("ΟΣ"), "οσ");
        assert_ne!("ΟΣ".to_lowercase(), scalar_lower("ΟΣ"));
    }
    #[test]
    fn trim_space_keeps_go_unicode_boundaries_and_original_interior() {
        for c in [
            '\t', '\n', '\u{000b}', '\u{000c}', '\r', ' ', '\u{0085}', '\u{00a0}', '\u{1680}',
            '\u{2000}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}',
        ] {
            assert!(!nonempty(&c.to_string()));
            assert_eq!(
                trim_space(&format!("{c}inside {c} text{c}")),
                format!("inside {c} text")
            );
        }
        for c in ['\0', '\u{001c}', '\u{200b}', '\u{feff}'] {
            assert!(nonempty(&c.to_string()));
        }
    }
}

/// Decode arbitrary bytes with Go encoding/json per-invalid-byte replacement.
/// Valid text is preserved; no trim, normalization, IO, role or mutation. The
/// validated-prefix expectation follows stdlib valid_up_to, never user trust.
pub(crate) fn go_utf8(raw: &[u8]) -> String {
    let mut result = String::new();
    let mut remaining = raw;
    while !remaining.is_empty() {
        match std::str::from_utf8(remaining) {
            Ok(s) => {
                result.push_str(s);
                break;
            }
            Err(e) => {
                let valid = e.valid_up_to();
                result.push_str(
                    std::str::from_utf8(&remaining[..valid]).expect("validated UTF8 prefix"),
                );
                let count = e.error_len().unwrap_or(remaining.len() - valid);
                for _ in 0..count {
                    result.push('\u{fffd}')
                }
                remaining = &remaining[valid + count..];
            }
        }
    }
    result
}

#[cfg(test)]
mod go_utf8_tests {
    use super::go_utf8;
    /// Valid UTF8 is the control; malformed multibyte sequences must emit one
    /// replacement per invalid byte, unlike Rust's grouped lossy conversion.
    #[test]
    fn arbitrary_sql_bytes_match_go_per_byte_replacement() {
        assert_eq!(go_utf8("valid é 😀".as_bytes()), "valid é 😀");
        assert_eq!(go_utf8(&[0xe2, 0x82]), "\u{fffd}\u{fffd}");
        assert_eq!(go_utf8(&[b'a', 0xff, 0xff, b'b']), "a\u{fffd}\u{fffd}b");
        assert_eq!(go_utf8(&[0, b'x']), "\0x");
    }
}
