//! Retained Go0.4.7 oracle, Unicode15 properties and exact clip/cluster contracts.
use super::*;
use sha2::{Digest, Sha256};
/// Compare each retained multi-code-point case, stateful boundaries and clip
/// width against the actual offline Go oracle. No permissions or IO are inferred.
#[test]
fn go_reference_graphemes_widths_and_clips() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/terminal_clip_go047.json"
    ))
    .unwrap();
    assert_eq!(oracle["unicode_version"], "15.0.0");
    let cases = oracle["items"].as_array().unwrap();
    assert_eq!(cases.len() as u64, oracle["case_count"].as_u64().unwrap());
    assert_eq!(cases.len(), 615);
    for (number, case) in cases.iter().enumerate() {
        let text = case["text"].as_str().unwrap();
        assert_eq!(
            text_width(text).unwrap() as u64,
            case["width"].as_u64().unwrap(),
            "width case {number}"
        );
        let mut clusters = Vec::new();
        let mut rest = text;
        let mut state = -1;
        let mut end = 0usize;
        while let Some(c) = cluster::first(rest, state).unwrap() {
            end += c.end;
            clusters.push(serde_json::json!({"end":end,"width":c.width}));
            rest = &rest[c.end..];
            state = c.state;
        }
        assert_eq!(
            serde_json::Value::Array(clusters),
            case["clusters"],
            "clusters case {number}"
        );
        for (width, expected) in case["clips"].as_object().unwrap() {
            assert_eq!(
                clip(text, width.parse().unwrap()).unwrap(),
                expected.as_str().unwrap(),
                "clip case {number} width {width}"
            );
        }
    }
}
/// All valid Unicode scalars, not a heuristic sample: hash scalar-u32LE plus
/// single-scalar width-u8 in the same order as the retained Go oracle.
#[test]
fn go_reference_widths_for_all_unicode_scalars() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/terminal_clip_go047.json"
    ))
    .unwrap();
    let mut hash = Sha256::new();
    let mut count = 0u64;
    for value in 0..=0x10ffffu32 {
        if let Some(character) = char::from_u32(value) {
            let mut utf8 = [0u8; 4];
            let text = character.encode_utf8(&mut utf8);
            let width = text_width(text).unwrap();
            let byte = u8::try_from(width).unwrap();
            hash.update(value.to_le_bytes());
            hash.update([byte]);
            count += 1;
        }
    }
    assert_eq!(count, 0x110000 - 0x800);
    assert_eq!(oracle["scalar_count"].as_u64().unwrap(), count);
    assert_eq!(
        format!("{:x}", hash.finalize()),
        oracle["scalar_width_sha256"].as_str().unwrap()
    );
}
