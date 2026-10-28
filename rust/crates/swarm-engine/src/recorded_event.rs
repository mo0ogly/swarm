//! Caller-owned SQL event reads and explicit JSON encoding. Raw history is data,
//! not evidence of identity, permission, completeness or a valid revision chain.
use rusqlite::{types::ValueRef, Connection};
use std::fmt;
#[derive(Clone, PartialEq, Eq)]
pub struct RecordedEvent {
    pub id: Vec<u8>,
    pub work_id: Vec<u8>,
    pub revision: i64,
    pub kind: Vec<u8>,
    pub at: Vec<u8>,
    /// Go modernc empty BLOB scans as nil. TEXT/NULL payloads are scan errors.
    pub payload: Option<Vec<u8>>,
}
impl fmt::Debug for RecordedEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecordedEvent { data: redacted }")
    }
}
pub enum RecordedEventError {
    Sql(rusqlite::Error),
    Payload(serde_json::Error),
    Finalize {
        primary: Box<Self>,
        cleanup: rusqlite::Error,
    },
}
impl RecordedEventError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sql(_) => "event-storage",
            Self::Payload(_) => "event-payload",
            Self::Finalize { .. } => "event-storage-cleanup",
        }
    }
}
impl fmt::Display for RecordedEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for RecordedEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for RecordedEventError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sql(e) => Some(e),
            Self::Payload(e) => Some(e),
            Self::Finalize { primary, .. } => Some(primary),
        }
    }
}
impl From<rusqlite::Error> for RecordedEventError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
/// Revision order matches Go, including nonpositive revisions. No Work lookup,
/// name policy, JSON parsing, transaction boundary, migration or retry is added.
pub fn list_recorded_events(
    db: &Connection,
    work: &str,
) -> Result<Vec<RecordedEvent>, RecordedEventError> {
    let mut stmt=db.prepare("SELECT id,work_id,revision,kind,at,payload FROM main.events WHERE work_id=? ORDER BY revision")?;
    let result = (|| {
        let mut rows = stmt.query([work])?;
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            let payload = match row.get_ref(5)? {
                ValueRef::Blob(v) => {
                    if v.is_empty() {
                        None
                    } else {
                        Some(v.to_vec())
                    }
                }
                v => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        5,
                        "payload".into(),
                        v.data_type(),
                    )
                    .into())
                }
            };
            events.push(RecordedEvent {
                id: crate::sqlite_value::string(row.get_ref(0)?, 0)?,
                work_id: crate::sqlite_value::string(row.get_ref(1)?, 1)?,
                revision: crate::sqlite_value::integer(row.get_ref(2)?, 2)?,
                kind: crate::sqlite_value::string(row.get_ref(3)?, 3)?,
                at: crate::sqlite_value::string(row.get_ref(4)?, 4)?,
                payload,
            });
        }
        Ok(events)
    })();
    match (result, stmt.finalize()) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e.into()),
        (Err(primary), Err(cleanup)) => Err(RecordedEventError::Finalize {
            primary: Box::new(primary),
            cleanup,
        }),
    }
}
fn string(out: &mut Vec<u8>, raw: &[u8], html: bool) {
    out.push(b'"');
    let mut i = 0;
    while i < raw.len() {
        let b = raw[i];
        match b {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            8 => out.extend_from_slice(b"\\b"),
            12 => out.extend_from_slice(b"\\f"),
            0..=31 => out.extend_from_slice(format!("\\u{b:04x}").as_bytes()),
            b'<' | b'>' | b'&' if html => out.extend_from_slice(format!("\\u{b:04x}").as_bytes()),
            32..=127 => out.push(b),
            _ => {
                let width = match b {
                    0xc2..=0xdf => 2,
                    0xe0..=0xef => 3,
                    0xf0..=0xf4 => 4,
                    _ => 1,
                };
                if let Some(chunk) = raw
                    .get(i..i + width)
                    .filter(|v| std::str::from_utf8(v).is_ok())
                {
                    match chunk {
                        b"\xe2\x80\xa8" => out.extend_from_slice(b"\\u2028"),
                        b"\xe2\x80\xa9" => out.extend_from_slice(b"\\u2029"),
                        _ => out.extend_from_slice(chunk),
                    }
                    i += width;
                    continue;
                }
                out.extend_from_slice(b"\\ufffd");
            }
        }
        i += 1;
    }
    out.push(b'"');
}
fn payload(out: &mut Vec<u8>, raw: &[u8], html: bool) -> Result<(), RecordedEventError> {
    // Validate Go syntax/depth without rewriting original raw bytes or lexemes.
    swarm_wire_json::decode::<serde::de::IgnoredAny>(raw).map_err(RecordedEventError::Payload)?;
    let (mut inside, mut escaped, mut i) = (false, false, 0);
    while i < raw.len() {
        let b = raw[i];
        if inside {
            if html && !escaped {
                let replacement = match b {
                    b'<' => Some(b"\\u003c".as_slice()),
                    b'>' => Some(b"\\u003e".as_slice()),
                    b'&' => Some(b"\\u0026".as_slice()),
                    _ => None,
                };
                if let Some(r) = replacement {
                    out.extend_from_slice(r);
                    i += 1;
                    continue;
                }
                if raw.get(i..i + 3) == Some(b"\xe2\x80\xa8") {
                    out.extend_from_slice(b"\\u2028");
                    i += 3;
                    continue;
                }
                if raw.get(i..i + 3) == Some(b"\xe2\x80\xa9") {
                    out.extend_from_slice(b"\\u2029");
                    i += 3;
                    continue;
                }
            }
            out.push(b);
            if escaped {
                escaped = false
            } else if b == b'\\' {
                escaped = true
            } else if b == b'"' {
                inside = false
            }
        } else if b == b'"' {
            inside = true;
            out.push(b)
        } else if !b.is_ascii_whitespace() {
            out.push(b)
        }
        i += 1;
    }
    Ok(())
}
/// Compact Go Event array, without newline. HTML escaping is a transport option
/// (Go Marshal=true, CLI Encoder=false). Invalid payload rejects the whole output.
/// Go RawMessage may retain invalid UTF-8 inside JSON strings; bytes are preserved.
pub fn encode_recorded_events(
    events: &[RecordedEvent],
    escape_html: bool,
) -> Result<Vec<u8>, RecordedEventError> {
    let mut out = vec![b'['];
    for (i, e) in events.iter().enumerate() {
        if i > 0 {
            out.push(b',')
        }
        out.extend_from_slice(b"{\"id\":");
        string(&mut out, &e.id, escape_html);
        out.extend_from_slice(b",\"work_id\":");
        string(&mut out, &e.work_id, escape_html);
        out.extend_from_slice(b",\"revision\":");
        out.extend_from_slice(e.revision.to_string().as_bytes());
        out.extend_from_slice(b",\"kind\":");
        string(&mut out, &e.kind, escape_html);
        out.extend_from_slice(b",\"at\":");
        string(&mut out, &e.at, escape_html);
        out.extend_from_slice(b",\"payload\":");
        if let Some(raw) = &e.payload {
            payload(&mut out, raw, escape_html)?
        } else {
            out.extend_from_slice(b"null")
        }
        out.push(b'}');
    }
    out.push(b']');
    Ok(out)
}
