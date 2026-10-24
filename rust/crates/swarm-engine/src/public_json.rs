//! Public JSON indentation without decoding opaque numeric/string lexemes.
/// Validate Go-compatible JSON, then indent it like the CLI's two-space encoder.
/// Preserves raw event bytes, including Go RawMessage string bytes and numeric
/// spellings; no f64 conversion, truncation, actor validation or output is added.
/// Syntax/depth failure preserves its cause. One final newline terminates output.
pub fn pretty_public_json(raw: &[u8]) -> Result<Vec<u8>, crate::WorkError> {
    swarm_wire_json::decode::<serde::de::IgnoredAny>(raw).map_err(crate::WorkError::Json)?;
    let (mut out, mut i, mut depth, mut inside, mut escaped) =
        (Vec::new(), 0, 0usize, false, false);
    fn line(out: &mut Vec<u8>, depth: usize) {
        out.push(b'\n');
        out.extend(std::iter::repeat_n(b' ', depth * 2));
    }
    while i < raw.len() {
        let b = raw[i];
        if inside {
            out.push(b);
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                inside = false;
            }
            i += 1;
            continue;
        }
        match b {
            b' ' | b'\n' | b'\r' | b'\t' => {}
            b'"' => {
                inside = true;
                out.push(b);
            }
            b'{' | b'[' => {
                out.push(b);
                let close = if b == b'{' { b'}' } else { b']' };
                let mut next = i + 1;
                while raw
                    .get(next)
                    .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
                {
                    next += 1;
                }
                if raw.get(next) == Some(&close) {
                    out.push(close);
                    i = next + 1;
                    continue;
                }
                depth += 1;
                line(&mut out, depth);
            }
            b'}' | b']' => {
                depth -= 1;
                line(&mut out, depth);
                out.push(b);
            }
            b',' => {
                out.push(b);
                line(&mut out, depth);
            }
            b':' => out.extend_from_slice(b": "),
            _ => out.push(b),
        }
        i += 1;
    }
    out.push(b'\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_indentation_keeps_raw_lexemes_invalid_string_bytes_and_empty_values() {
        let raw = b"{\"n\":[1e999,-0,1.00],\"s\":\"\xff\\\"x\",\"e\":[{},[]]}";
        let pretty = pretty_public_json(raw).unwrap();
        assert_eq!(pretty,b"{\n  \"n\": [\n    1e999,\n    -0,\n    1.00\n  ],\n  \"s\": \"\xff\\\"x\",\n  \"e\": [\n    {},\n    []\n  ]\n}\n");
        assert!(pretty_public_json(b"{}{}").is_err());
        assert_eq!(pretty_public_json(b" {} \n").unwrap(), b"{}\n");
    }
}
