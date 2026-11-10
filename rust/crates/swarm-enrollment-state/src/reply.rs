use super::{valid_id, Error, Head, Record};
use base64::{
    alphabet,
    engine::{
        general_purpose::{GeneralPurpose, GeneralPurposeConfig, STANDARD},
        DecodePaddingMode,
    },
    Engine,
};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::value::RawValue;
use std::fmt;
use swarm_wire_json as wire;
pub const ACTIVE_REPLY_SCHEMA: i64 = 1;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActiveReply {
    pub schema_version: i64,
    pub head: Head,
    pub record: Record,
}
pub(crate) fn serialize_body<S: Serializer>(body: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&STANDARD.encode(body))
}
// Preserve field order/duplicates for Go's sequential unmarshal semantics. Unknown
// fields and null scalars are tolerated here, then exact reencoding rejects them.
// Unsupported schema is classified before canonical/semantic checks, as in Go.
#[derive(Default)]
struct Object(Vec<(String, Box<RawValue>)>);
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = Object;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object or null")
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Object, E> {
                Ok(Object::default())
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Object, A::Error> {
                let mut values = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    values.push(pair)
                }
                Ok(Object(values))
            }
        }
        d.deserialize_any(ObjectVisitor)
    }
}
fn integer(raw: &RawValue, slot: &mut i64) -> Result<(), Error> {
    if raw.get() != "null" {
        *slot = wire::projection::signed_integer(raw.get().as_bytes()).map_err(|_| Error::Reply)?
    }
    Ok(())
}
fn string(raw: &RawValue, slot: &mut String) -> Result<(), Error> {
    if raw.get() != "null" {
        *slot = serde_json::from_str(raw.get()).map_err(|_| Error::Reply)?
    }
    Ok(())
}
fn object(raw: &RawValue) -> Result<Object, Error> {
    serde_json::from_str(raw.get()).map_err(|_| Error::Reply)
}
fn head(raw: &RawValue, out: &mut Head) -> Result<(), Error> {
    for (key, value) in object(raw)?.0 {
        match key.to_uppercase().as_str() {
            "SEQUENCE" => integer(&value, &mut out.sequence)?,
            "DIGEST" => string(&value, &mut out.digest)?,
            _ => (),
        }
    }
    Ok(())
}
fn record(
    raw: &RawValue,
    id: &mut String,
    body: &mut Option<Vec<u8>>,
    sequence: &mut i64,
) -> Result<(), Error> {
    for (key, value) in object(raw)?.0 {
        match key.to_uppercase().as_str() {
            "ID" => string(&value, id)?,
            "ENROLLED_SEQUENCE" => integer(&value, sequence)?,
            "BODY" => {
                if value.get() == "null" {
                    *body = None
                } else {
                    // Go []byte accepts a JSON array too; canonical output is still base64.
                    if value.get().starts_with('[') {
                        let entries: Vec<Option<u8>> =
                            serde_json::from_str(value.get()).map_err(|_| Error::Reply)?;
                        *body = Some(entries.into_iter().map(|n| n.unwrap_or_default()).collect());
                    } else {
                        let s: String =
                            serde_json::from_str(value.get()).map_err(|_| Error::Reply)?;
                        let s = s.replace(['\r', '\n'], "");
                        let engine = GeneralPurpose::new(
                            &alphabet::STANDARD,
                            GeneralPurposeConfig::new()
                                .with_decode_padding_mode(DecodePaddingMode::RequireCanonical)
                                .with_decode_allow_trailing_bits(true),
                        );
                        *body = Some(engine.decode(s).map_err(|_| Error::Reply)?);
                    }
                }
            }
            _ => (),
        }
    }
    Ok(())
}
/// Decode authenticated bytes' format only: no signature, authority or freshness.
pub fn decode_active_reply(raw: &[u8]) -> Result<ActiveReply, Error> {
    let fields: Object = wire::decode(raw).map_err(|_| Error::Reply)?;
    let mut schema = 0;
    let mut checkpoint = Head::default();
    let mut id = String::new();
    let mut body = None;
    let mut generation = 0;
    for (key, value) in fields.0 {
        match key.to_uppercase().as_str() {
            "SCHEMA_VERSION" => integer(&value, &mut schema)?,
            "HEAD" => head(&value, &mut checkpoint)?,
            "RECORD" => record(&value, &mut id, &mut body, &mut generation)?,
            _ => (),
        }
    }
    if schema != ACTIVE_REPLY_SCHEMA {
        return Err(Error::ReplyVersion);
    }
    let body = body.ok_or(Error::Reply)?;
    let out = ActiveReply {
        schema_version: schema,
        head: checkpoint,
        record: Record {
            id,
            body,
            enrolled_sequence: generation,
        },
    };
    if wire::encode(&out).map_err(|_| Error::Reply)? != raw
        || out.head.sequence <= 0
        || !out.head.valid()
        || !valid_id(&out.record.id)
        || out.record.enrolled_sequence <= 0
        || out.record.enrolled_sequence > out.head.sequence
    {
        return Err(Error::Reply);
    }
    Ok(out)
}
