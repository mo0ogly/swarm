//! Validate the inherited API-connection namespace without executing an API.
//! Secrets stay in read-only input bytes; only a whole-source hash is returned.
#![cfg(target_os = "linux")]
use crate::{
    model_wire::{Fields, Kind},
    AgentLaunchError, Providers,
};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Only the namespace discriminator is consumed. The projector validates every
/// inherited field's type before dropping unused endpoint/model/key data.
#[derive(Default, Deserialize)]
#[serde(default)]
struct ConnectionState {
    disabled: bool,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Projection {
    connections: Option<BTreeMap<String, ConnectionState>>,
}

/// Internal wrapper around a dynamic-key Go map, not a new persisted schema.
fn wrapper_shape() -> Fields {
    &[("connections", Kind::Map(&Kind::Object(connection_shape)))]
}
/// Existing Go AIConnection wire fields. Endpoint/key values grant no capability
/// and are never returned, serialized, logged or used to create a process here.
fn connection_shape() -> Fields {
    &[
        ("id", Kind::String),
        ("label", Kind::String),
        ("base_url", Kind::String),
        ("model", Kind::String),
        ("key", Kind::String),
        ("disabled", Kind::Bool),
    ]
}
/// Parse exactly one original JSON map/null, then use the existing Go-compatible
/// projector for typed fields, case folding, duplicate merging and nulls. Original
/// validation precedes wrapping so raw text cannot inject a second wrapper field.
/// Unknown/type/UTF8/JSON failures retain typed causes behind a safe static code.
/// No IO, mutations, fallback catalogue, endpoint validation or API call occurs.
fn decode(raw: &[u8]) -> Result<Projection, AgentLaunchError> {
    swarm_wire_json::decode::<serde::de::IgnoredAny>(raw)
        .map_err(|e| AgentLaunchError::caused("agent-api-connections-catalogue-invalid", e))?;
    let mut wrapped = b"{\"connections\":".to_vec();
    wrapped.extend_from_slice(raw);
    wrapped.push(b'}');
    crate::work::decode_mode(&wrapped, wrapper_shape(), true)
        .map_err(|e| AgentLaunchError::caused("agent-api-connections-catalogue-invalid", e))
}
/// Check namespace conflicts and selected-provider support against one complete
/// read-only API catalogue and the decoded CLI catalogue. Absent/map-null/empty
/// metadata permits unrelated CLI selection; active `api-<map-key>` names collide
/// exactly as in Go. Disabled entries synthesize no provider. An active selected
/// API always refuses explicitly until native execution is ported, never falling
/// through to a CLI. No connection key, endpoint or full source leaves this call.
/// Return only source SHA (None means actual absence), for admission rereads;
/// equality proves unchanged bytes, not authenticity or an enduring lease.
pub(crate) fn guard(
    raw: Option<&[u8]>,
    cli: &Providers,
    requested: &str,
) -> Result<Option<String>, AgentLaunchError> {
    let Some(raw) = raw else { return Ok(None) };
    let entries = decode(raw)?.connections.unwrap_or_default();
    let mut selected = false;
    for (id, state) in entries {
        if state.disabled {
            continue;
        }
        let name = format!("api-{id}");
        if cli
            .providers
            .as_ref()
            .is_some_and(|p| p.contains_key(&name))
        {
            return Err(AgentLaunchError::Refused(
                "agent-provider-api-name-conflict",
            ));
        }
        selected |= name == requested;
    }
    if selected {
        return Err(AgentLaunchError::Refused(
            "agent-api-provider-execution-unported",
        ));
    }
    Ok(Some(crate::agent_launch_preflight::hash(raw)))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// An explicit empty CLI map used only for pure namespace checks.
    fn cli() -> Providers {
        Providers {
            schema_version: 1,
            providers: Some(BTreeMap::new()),
        }
    }
    /// Go map/null and discriminator shapes permit unrelated CLI routing without
    /// inventing an executable or exposing key/endpoint/model values.
    #[test]
    fn connection_catalogue_accepts_unrelated_empty_and_go_shapes() {
        let cli = cli();
        assert!(guard(None, &cli, "fixture").unwrap().is_none());
        for raw in [b"null".as_slice(),b"{}",b"{\"local\":null}",b"{\"local\":{\"disabled\":null}}",b"{\"local\":{\"DISABLED\":true,\"key\":\"PRIVATE\",\"model\":\"model\",\"base_url\":\"not contacted\"}}"] {
            assert_eq!(guard(Some(raw),&cli,"fixture").unwrap(),Some(crate::agent_launch_preflight::hash(raw)));
        }
    }
    /// Valid controls precede malformed/type/unknown/injection cases. No denied
    /// source is converted into an empty successful catalogue; errors redact data.
    #[test]
    fn connection_catalogue_refuses_invalid_sources_with_safe_causes() {
        let cli = cli();
        for raw in [
            b"".as_slice(),
            b"[]",
            b"true",
            b"1",
            b"{} {}",
            b"{},\"connections\":{}",
            b"{\"local\":{\"disabled\":\"false\"}}",
            b"{\"local\":{\"key\":42}}",
            b"{\"local\":{\"PRIVATE_UNKNOWN\":true}}",
            b"{\"local\":{\"key\":\"\xff\"}}",
        ] {
            guard(Some(b"{}"), &cli, "fixture").unwrap();
            let error = guard(Some(raw), &cli, "fixture").unwrap_err();
            assert_eq!(error.code(), "agent-api-connections-catalogue-invalid");
            assert!(!error.to_string().contains("PRIVATE"));
            guard(Some(b"{}"), &cli, "fixture").unwrap();
        }
    }
    /// Active aliases collide globally; disabled ones do not. A selected active
    /// API refuses even with no collision; the namespace never grants execution.
    #[test]
    fn connection_catalogue_preserves_conflicts_disabled_and_api_refusal() {
        let mut cli = cli();
        cli.providers
            .as_mut()
            .unwrap()
            .insert("api-local".into(), crate::Provider::default());
        guard(Some(b"{}"), &cli, "fixture").unwrap();
        assert_eq!(
            guard(Some(b"{\"local\":{}}"), &cli, "fixture")
                .unwrap_err()
                .code(),
            "agent-provider-api-name-conflict"
        );
        guard(Some(b"{\"local\":{\"disabled\":true}}"), &cli, "fixture").unwrap();
        cli.providers.as_mut().unwrap().clear();
        guard(Some(b"{\"local\":{}}"), &cli, "fixture").unwrap();
        for raw in [
            b"{\"local\":{}}".as_slice(),
            b"{\"local\":{\"disabled\":null}}",
            b"{\"local\":{\"disabled\":true},\"local\":{\"disabled\":false}}",
        ] {
            guard(Some(raw), &cli, "fixture").unwrap();
            assert_eq!(
                guard(Some(raw), &cli, "api-local").unwrap_err().code(),
                "agent-api-provider-execution-unported"
            );
        }
    }
}
