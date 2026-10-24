//! Go RFC3339 parsing shared by data protocols. This authenticates no clock.
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime, UtcOffset};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoTimestampError;
impl std::fmt::Display for GoTimestampError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid Go RFC3339 timestamp")
    }
}
impl std::error::Error for GoTimestampError {}
fn digits(raw: &[u8]) -> Result<u32, GoTimestampError> {
    if raw.is_empty() || !raw.iter().all(u8::is_ascii_digit) {
        return Err(GoTimestampError);
    }
    Ok(raw.iter().fold(0, |n, b| n * 10 + u32::from(b - b'0')))
}
pub fn parse(raw: &str) -> Result<OffsetDateTime, GoTimestampError> {
    let b = raw.as_bytes();
    if b.len() < 19
        || b.get(4) != Some(&b'-')
        || b.get(7) != Some(&b'-')
        || b.get(10) != Some(&b'T')
    {
        return Err(GoTimestampError);
    }
    digits(&b[..4])?;
    digits(&b[5..7])?;
    digits(&b[8..10])?;
    let c = &b[11..];
    let hlen = if c.get(1) == Some(&b':') {
        1
    } else if c.get(2) == Some(&b':') {
        2
    } else {
        return Err(GoTimestampError);
    };
    let hour = digits(&c[..hlen])?;
    let minute = digits(c.get(hlen + 1..hlen + 3).ok_or(GoTimestampError)?)?;
    if c.get(hlen + 3) != Some(&b':') {
        return Err(GoTimestampError);
    }
    let second = digits(c.get(hlen + 4..hlen + 6).ok_or(GoTimestampError)?)?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(GoTimestampError);
    }
    let mut pos = hlen + 6;
    let mut fraction = String::new();
    if matches!(c.get(pos), Some(b'.' | b',')) {
        pos += 1;
        let start = pos;
        while c.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1
        }
        if pos == start {
            return Err(GoTimestampError);
        }
        fraction.push('.');
        fraction.push_str(
            std::str::from_utf8(&c[start..pos.min(start + 9)]).map_err(|_| GoTimestampError)?,
        );
    }
    let zone = c.get(pos..).ok_or(GoTimestampError)?;
    let seconds = if zone == b"Z" {
        0
    } else {
        if zone.len() != 6 || !matches!(zone[0], b'+' | b'-') || zone[3] != b':' {
            return Err(GoTimestampError);
        }
        let hour = digits(&zone[1..3])?;
        let minute = digits(&zone[4..6])?;
        if hour > 24 || minute > 60 {
            return Err(GoTimestampError);
        }
        let n = i32::try_from(hour * 3600 + minute * 60).map_err(|_| GoTimestampError)?;
        if zone[0] == b'-' {
            -n
        } else {
            n
        }
    };
    let normalized = format!(
        "{}T{hour:02}:{minute:02}:{second:02}{fraction}Z",
        &raw[..10]
    );
    let utc = OffsetDateTime::parse(&normalized, &Rfc3339)
        .map_err(|_| GoTimestampError)?
        .checked_sub(Duration::seconds(i64::from(seconds)))
        .ok_or(GoTimestampError)?;
    // Go accepts offsets of 24h / 60m while Rust's offset type cannot represent
    // all of them. Keep the exact instant in UTC; original bytes remain subject
    // to canonical comparison and can never be accepted as a repaired request.
    if let Ok(offset) = UtcOffset::from_whole_seconds(seconds) {
        utc.checked_to_offset(offset).ok_or(GoTimestampError)
    } else {
        Ok(utc)
    }
}
