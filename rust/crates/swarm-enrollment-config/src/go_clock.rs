//! Keep configuration decoding errors stable while sharing the Go grammar.
use crate::Error;
use serde::{Deserialize, Deserializer};
use time::OffsetDateTime;
pub(super) fn parse(raw: &str) -> Result<OffsetDateTime, Error> {
    swarm_wire_json::go_rfc3339::parse(raw).map_err(|_| Error::Format)
}
pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<OffsetDateTime, D::Error> {
    let raw = String::deserialize(d)?;
    parse(&raw).map_err(serde::de::Error::custom)
}
