//! Go time.Time JSON representation used by canonical enrollment protocols.
use serde::{Deserialize, Deserializer, Serializer};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
pub fn zero(value: OffsetDateTime) -> bool {
    value.unix_timestamp() == -62_135_596_800 && value.nanosecond() == 0
}
pub fn serialize<S: Serializer>(value: &OffsetDateTime, s: S) -> Result<S::Ok, S::Error> {
    let utc = value
        .checked_to_offset(UtcOffset::UTC)
        .ok_or_else(|| serde::ser::Error::custom("UTC calendar representation overflow"))?;
    serialize_preserving_offset(&utc, s)
}
pub fn serialize_preserving_offset<S: Serializer>(
    value: &OffsetDateTime,
    s: S,
) -> Result<S::Ok, S::Error> {
    // Go keeps the local wall clock, truncates offset seconds toward zero,
    // and writes +00:00 if a nonzero subminute offset rounds to zero.
    let seconds = value.offset().whole_seconds();
    let minutes = seconds / 60 * 60;
    let rounded = value
        .replace_offset(UtcOffset::from_whole_seconds(minutes).map_err(serde::ser::Error::custom)?);
    let mut raw = rounded
        .format(&Rfc3339)
        .map_err(serde::ser::Error::custom)?;
    if seconds != 0 && minutes == 0 {
        raw.pop();
        raw.push_str("+00:00");
    }
    s.serialize_str(&raw)
}
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<OffsetDateTime, D::Error> {
    let raw = String::deserialize(d)?;
    OffsetDateTime::parse(&raw, &Rfc3339).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(serde::Serialize)]
    struct Clock(#[serde(serialize_with = "serialize_preserving_offset")] OffsetDateTime);
    #[test]
    fn subminute_offsets_match_go_without_rounding_the_local_clock() {
        let utc = OffsetDateTime::parse("2026-10-17T12:00:00Z", &Rfc3339).unwrap();
        for (seconds, expected) in [
            (1, "2026-10-17T12:00:01+00:00"),
            (-1, "2026-10-17T11:59:59+00:00"),
            (61, "2026-10-17T12:01:01+00:01"),
            (-61, "2026-10-17T11:58:59-00:01"),
        ] {
            let value = utc.to_offset(UtcOffset::from_whole_seconds(seconds).unwrap());
            assert_eq!(
                serde_json::to_string(&Clock(value)).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }
}
