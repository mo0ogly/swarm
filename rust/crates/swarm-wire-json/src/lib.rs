#![forbid(unsafe_code)]
//! Only Go encoding/json's string behavior. Schema, field order and integer
//! representation remain the responsibility of each protocol's typed producer.
#[cfg(feature = "timestamp")]
pub mod go_rfc3339;
pub mod projection;
use serde::{de::DeserializeOwned, Serialize};

pub fn encode(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let raw = serde_json::to_string(value)?;
    let mut output = Vec::with_capacity(raw.len());
    for character in raw.chars() {
        match character {
            '<' => output.extend_from_slice(b"\\u003c"),
            '>' => output.extend_from_slice(b"\\u003e"),
            '&' => output.extend_from_slice(b"\\u0026"),
            '\u{2028}' => output.extend_from_slice(b"\\u2028"),
            '\u{2029}' => output.extend_from_slice(b"\\u2029"),
            value => {
                let mut buffer = [0; 4];
                output.extend_from_slice(value.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }
    Ok(output)
}

fn unicode_unit(raw: &[u8], position: usize) -> Option<u16> {
    let escape = raw.get(position..position + 6)?;
    if &escape[..2] != b"\\u" || !escape[2..].iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    u16::from_str_radix(std::str::from_utf8(&escape[2..]).ok()?, 16).ok()
}

// Go substitutes U+FFFD for each invalid UTF-8 byte and each unpaired UTF-16
// escape inside strings. No schema/type/syntax checks are removed: Serde still
// rejects unknown, duplicate, missing, non-string and trailing envelope fields.
fn go_string_bytes(raw: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(raw.len());
    let mut inside = false;
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'"' {
            inside = !inside;
            output.push(raw[i]);
            i += 1;
        } else if !inside {
            output.push(raw[i]);
            i += 1;
        } else if raw[i] == b'\\' {
            if let Some(unit) = unicode_unit(raw, i) {
                if (0xd800..=0xdbff).contains(&unit) {
                    if unicode_unit(raw, i + 6).is_some_and(|low| (0xdc00..=0xdfff).contains(&low))
                    {
                        output.extend_from_slice(&raw[i..i + 12]);
                        i += 12;
                    } else {
                        output.extend_from_slice("�".as_bytes());
                        i += 6;
                    }
                } else if (0xdc00..=0xdfff).contains(&unit) {
                    output.extend_from_slice("�".as_bytes());
                    i += 6;
                } else {
                    output.extend_from_slice(&raw[i..i + 6]);
                    i += 6;
                }
            } else if i + 1 < raw.len() {
                // Copy an ordinary escape as a pair, including escaped quotes.
                output.extend_from_slice(&raw[i..i + 2]);
                i += 2;
            } else {
                output.push(raw[i]);
                i += 1;
            }
        } else if raw[i] >= 0x80 {
            let width = match raw[i] {
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf4 => 4,
                _ => 1,
            };
            if let Some(bytes) = raw
                .get(i..i + width)
                .filter(|bytes| std::str::from_utf8(bytes).is_ok())
            {
                output.extend_from_slice(bytes);
                i += width;
            } else {
                output.extend_from_slice("�".as_bytes());
                i += 1;
            }
        } else {
            output.push(raw[i]);
            i += 1;
        }
    }
    output
}

// encoding/json/scanner.go maxNestingDepth, a syntax/representation bound.
// RawValue/IgnoredAny can skip deeply nested unknown values; apply Go's bound
// before protocol schema validation to preserve error classification as well.
fn go_depth_valid(raw: &[u8]) -> bool {
    let (mut depth, mut inside, mut escaped) = (0usize, false, false);
    for &b in raw {
        if inside {
            if escaped {
                escaped = false
            } else if b == b'\\' {
                escaped = true
            } else if b == b'"' {
                inside = false
            }
        } else {
            match b {
                b'"' => inside = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 10_000 {
                        return false;
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => (),
            }
        }
    }
    true
}
pub fn decode<T: DeserializeOwned>(raw: &[u8]) -> Result<T, serde_json::Error> {
    if !go_depth_valid(raw) {
        return Err(serde::de::Error::custom(
            "Go JSON maximum nesting depth exceeded",
        ));
    }
    serde_json::from_slice(&go_string_bytes(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_escapes_html_and_javascript_separators_without_changing_text() {
        let value = "<>&\u{2028}\u{2029}\\\"é";
        let raw = encode(&value).unwrap();
        assert_eq!(
            raw,
            r#""\u003c\u003e\u0026\u2028\u2029\\\"é""#.as_bytes().to_vec()
        );
        assert_eq!(decode::<String>(&raw).unwrap(), value);
    }

    #[test]
    fn lossy_strings_preserve_pairs_and_reject_bad_json_syntax() {
        assert_eq!(decode::<String>(br#""\ud800\udc00""#).unwrap(), "\u{10000}");
        assert_eq!(
            decode::<String>(br#""\ud800\ud800\udc00\udfff""#).unwrap(),
            "�\u{10000}�"
        );
        assert_eq!(decode::<String>(&[b'"', 0xe2, 0x82, b'"']).unwrap(), "��");
        for raw in [
            br#""\uXX00""#.as_slice(),
            br#""\q""#,
            b"\"unterminated",
            b"\"a\" {}",
        ] {
            assert!(decode::<String>(raw).is_err());
        }
    }
}

#[cfg(feature = "timestamp")]
pub mod timestamp;
