//! Native optional freshness for the historical consultation write.
use crate::AgentLaunchError as E;
use serde::Deserialize;
#[derive(Default)]
enum Expected {
    #[default]
    Absent,
    Text(String),
}
impl<'de> Deserialize<'de> for Expected {
    /// A present expected revision must be text; null/number are not absence.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(Self::Text)
    }
}
/// Strict native request. Empty historical HTTP bodies are explicitly converted
/// with historical(), not defaulted from malformed/missing schema or null fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisitMarkInput {
    schema_version: i64,
    #[serde(default)]
    expected_revision: Expected,
}
impl VisitMarkInput {
    /// Historical current-at-call behavior, with no caller-supplied actor/time.
    pub fn historical() -> Self {
        Self {
            schema_version: 1,
            expected_revision: Expected::Absent,
        }
    }
    /// Canonical nonnegative i64 revision, or explicit legacy absence. This is a
    /// protocol fence, not an operator budget or JavaScript business ceiling.
    /// Parse/syntax/schema failures preserve typed causes before Root mutation.
    pub fn expected_revision(&self) -> Result<Option<i64>, E> {
        if self.schema_version != 1 {
            return Err(E::Refused("visit-input-invalid"));
        }
        let Expected::Text(text) = &self.expected_revision else {
            return Ok(None);
        };
        let value = text
            .parse::<i64>()
            .map_err(|e| E::caused("visit-revision-invalid", e))?;
        if value < 0 || value.to_string() != *text {
            return Err(E::Refused("visit-revision-invalid"));
        }
        Ok(Some(value))
    }
}
/// Decode one strict native intent before Root IO. Unknown/duplicate keys,
/// missing schema, null/malformed expected revision refuse, never mean latest.
pub fn decode_visit_mark(raw: &[u8]) -> Result<VisitMarkInput, E> {
    let value: VisitMarkInput =
        swarm_wire_json::decode(raw).map_err(|e| E::caused("visit-input-invalid", e))?;
    value.expected_revision()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_revision_and_explicit_historical_absence() {
        assert_eq!(
            VisitMarkInput::historical().expected_revision().unwrap(),
            None
        );
        for text in ["0", "9007199254740993", "9223372036854775807"] {
            let raw = format!(r#"{{"schema_version":1,"expected_revision":"{text}"}}"#);
            assert_eq!(
                decode_visit_mark(raw.as_bytes())
                    .unwrap()
                    .expected_revision()
                    .unwrap(),
                Some(text.parse().unwrap())
            );
        }
        assert_eq!(
            decode_visit_mark(br#"{"schema_version":1}"#)
                .unwrap()
                .expected_revision()
                .unwrap(),
            None
        );
    }
    #[test]
    fn malformed_or_ambiguous_intent_never_means_latest() {
        for raw in [
            r#"{}"#,
            r#"{"schema_version":2}"#,
            r#"{"schema_version":1,"schema_version":1}"#,
            r#"{"schema_version":1,"actor":"forged"}"#,
            r#"{"schema_version":1,"expected_revision":null}"#,
            r#"{"schema_version":1,"expected_revision":1}"#,
        ] {
            assert!(decode_visit_mark(raw.as_bytes()).is_err(), "{raw}");
        }
        for text in ["", "-1", "+1", "01", " 1", "1.0", "9223372036854775808"] {
            let raw = format!(r#"{{"schema_version":1,"expected_revision":"{text}"}}"#);
            assert!(decode_visit_mark(raw.as_bytes()).is_err(), "{raw}");
        }
    }
}
