//! Provider-file configuration is retained data, never admission or permission.
use crate::{Providers, WorkError};
use std::{error::Error, fmt};

/// Safe configuration-format failures. Original JSON/type causes are retained
/// privately; formatting never includes supplied paths, arguments or credentials.
pub enum ProviderConfigurationError {
    Refused(&'static str),
    Format(WorkError),
}
impl ProviderConfigurationError {
    /// Stable safe family; this classification grants neither mutation nor exec.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(code) => code,
            Self::Format(_) => "provider-configuration-invalid",
        }
    }
}
impl fmt::Display for ProviderConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for ProviderConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ProviderConfigurationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Format(e) => Some(e),
            Self::Refused(_) => None,
        }
    }
}
/// Require the inherited provider-file schema, without inventing commands or
/// default model policies. This is a storage-format check, not availability,
/// model selection, environment permission, sandbox validation or admission.
pub(crate) fn validate_provider_configuration(
    config: &Providers,
) -> Result<(), ProviderConfigurationError> {
    if config.schema_version != 1 {
        return Err(ProviderConfigurationError::Refused(
            "provider-configuration-schema-invalid",
        ));
    }
    Ok(())
}
/// Decode one strict Go-compatible Providers document. Unknown fields, malformed
/// JSON/types/UTF8 and an unsupported outer schema refuse with retained causes.
/// Nested declarations are retained rather than approved: runtime admission still
/// checks their supported policy, actual sources, executable, rights and budget.
/// No defaults, IO, file mutation, provider discovery or process is performed.
pub fn decode_provider_configuration(raw: &[u8]) -> Result<Providers, ProviderConfigurationError> {
    let config = crate::decode_providers(raw).map_err(ProviderConfigurationError::Format)?;
    validate_provider_configuration(&config)?;
    Ok(config)
}
/// Exact local CLI-file readback. Not the merged API/runtime catalogue; the API
/// secret file is never read here. Explicitly requested configuration may contain
/// private user-supplied CLI arguments; it is not an authentication/launch receipt.
/// Consumers must not treat this serializable view as permission or readiness.
#[derive(serde::Serialize)]
pub struct ProviderConfiguration {
    pub schema_version: i64,
    pub digest: String,
    pub configuration: Providers,
    pub created: bool,
    pub scope: &'static str,
    pub provider_execution_started: bool,
}
impl ProviderConfiguration {
    /// Bind observed/published bytes, without asserting their continued validity.
    /// No default or execution happens; `created` distinguishes this operation
    /// from a readonly read, not an authority or a provider capability.
    pub(crate) fn observed(configuration: Providers, raw: &[u8], created: bool) -> Self {
        use sha2::{Digest, Sha256};
        Self {
            schema_version: 1,
            digest: format!("{:x}", Sha256::digest(raw)),
            configuration,
            created,
            scope: "cli_configuration_file_only_unverified",
            provider_execution_started: false,
        }
    }
}

/// One proposed file-content update or an exact content-snapshot restoration.
/// Private fields keep callers on the strict decoder; neither proposal nor token
/// is actor authentication, provider readiness or execution authority.
#[derive(Clone)]
pub struct ProviderConfigurationChange {
    pub(crate) expected_digest: String,
    pub(crate) proposal: ProviderConfigurationProposal,
    pub(crate) preview_token: String,
}
#[derive(Clone)]
pub(crate) enum ProviderConfigurationProposal {
    Configuration(Providers),
    Restore(String),
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfigurationChange {
    schema_version: i64,
    expected_digest: String,
    #[serde(default)]
    configuration: Option<Box<serde_json::value::RawValue>>,
    #[serde(default)]
    restore_reference: Option<String>,
    #[serde(default)]
    preview_token: String,
}
/// Lowercase SHA256 wire representation, not a configurable operator ceiling.
pub(crate) fn configuration_digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
/// Return the claimed content digest from the reserved snapshot-name grammar.
/// Optional engine-generated snapshot suffix and legacy Go names are supported.
/// This only validates a reference; the descriptor, actual bytes/hash and format
/// must still be checked before restoration. No arbitrary path is accepted.
pub(crate) fn configuration_backup_digest(reference: &str) -> Option<&str> {
    let body = reference
        .strip_prefix("providers-before-")?
        .strip_suffix(".json")?;
    let digest = body.get(..64)?;
    if !configuration_digest_valid(digest) {
        return None;
    }
    let rest = &body[64..];
    if !rest.is_empty() {
        let suffix = rest.strip_prefix("-snapshot-")?;
        if suffix.is_empty()
            || !suffix
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return None;
        }
    }
    Some(digest)
}
/// Hash all observed bytes; callers enforce their saved file/transport bounds.
/// An integrity digest establishes neither identity nor authorization.
pub(crate) fn configuration_digest(raw: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(raw))
}
/// Strict native request: exactly one complete configuration or snapshot ref.
/// Duplicate/unknown fields and invalid schema/digest/reference refuse. Nested
/// configuration uses the same strict Go projection as init, never serde's
/// permissive generated DTO directly. Empty token is allowed only for preview;
/// apply separately binds current source/policy/proposal. No IO or mutation.
pub fn decode_provider_configuration_change(
    raw: &[u8],
) -> Result<ProviderConfigurationChange, ProviderConfigurationError> {
    let r: RawConfigurationChange = swarm_wire_json::decode(raw)
        .map_err(|e| ProviderConfigurationError::Format(WorkError::Json(e)))?;
    if r.schema_version != 1 || !configuration_digest_valid(&r.expected_digest) {
        return Err(ProviderConfigurationError::Refused(
            "provider-configuration-change-invalid",
        ));
    }
    let proposal = match (r.configuration, r.restore_reference) {
        (Some(raw), None) => ProviderConfigurationProposal::Configuration(
            decode_provider_configuration(raw.get().as_bytes())?,
        ),
        (None, Some(reference)) if configuration_backup_digest(&reference).is_some() => {
            ProviderConfigurationProposal::Restore(reference)
        }
        _ => {
            return Err(ProviderConfigurationError::Refused(
                "provider-configuration-change-invalid",
            ))
        }
    };
    Ok(ProviderConfigurationChange {
        expected_digest: r.expected_digest,
        proposal,
        preview_token: r.preview_token,
    })
}
/// Explicit metadata-only inspection, including malformed current content. A
/// successful inspection is not a successful configuration parse or admission;
/// validity/error fields are authoritative for this outer-format observation.
#[derive(serde::Serialize)]
pub struct ProviderConfigurationInspection {
    pub schema_version: i64,
    pub digest: String,
    pub reference: Option<String>,
    pub expected_backup_digest: Option<String>,
    pub matches_reference: Option<bool>,
    pub format_valid: bool,
    pub format_error_code: Option<&'static str>,
    pub scope: &'static str,
}
/// Snapshot names only. They are unverified references, not authenticated history
/// or an ordering/audit ledger. Restore always rereads and validates actual bytes.
#[derive(serde::Serialize)]
pub struct ProviderConfigurationBackups {
    pub schema_version: i64,
    pub references: Vec<String>,
    pub scope: &'static str,
}
/// Source/policy-bound preview or completed update. Boolean fields describe this
/// operation, not permissions. Snapshots preserve content versions, not a signed
/// chronological operator audit. No provider executes through this response.
#[derive(serde::Serialize)]
pub struct ProviderConfigurationMutation {
    pub schema_version: i64,
    pub preview: bool,
    pub applied: bool,
    pub changed: bool,
    pub before_digest: String,
    pub after_digest: String,
    pub preview_token: String,
    pub backup_reference: Option<String>,
    pub restore_reference: Option<String>,
    pub configuration: Providers,
    pub provider_execution_started: bool,
    pub scope: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Supplied model/argv data survives parsing exactly, without inferred names.
    #[test]
    fn provider_configuration_retains_explicit_data_without_model_defaults() {
        let raw=br#"{"schema_version":1,"providers":{"local":{"command":"/fixture/program","args":["--literal","x"],"env_allow":[]}}}"#;
        let config = decode_provider_configuration(raw).unwrap();
        let p = &config.providers.as_ref().unwrap()["local"];
        assert!(p.model_policy.is_none());
        assert_eq!(p.command, "/fixture/program");
        assert_eq!(p.args.as_deref().unwrap(), ["--literal", "x"]);
        let view = ProviderConfiguration::observed(config, raw, false);
        assert!(!view.created && !view.provider_execution_started);
        assert_eq!(view.scope, "cli_configuration_file_only_unverified");
    }
    /// A valid schema witness precedes every invalid/unknown/private input; no
    /// failure becomes an empty successful configuration or reveals raw data.
    #[test]
    fn provider_configuration_refuses_invalid_schema_and_shape_safely() {
        for raw in [
            b"".as_slice(),
            b"{}",
            b"[]",
            b"{\"schema_version\":2}",
            b"{\"schema_version\":1,\"PRIVATE_UNKNOWN\":true}",
            b"{\"schema_version\":1,\"providers\":[]}",
            b"{\"schema_version\":1,\"providers\":{\"x\":{\"args\":42}}}",
            b"{} {}",
        ] {
            decode_provider_configuration(br#"{"schema_version":1,"providers":{}}"#).unwrap();
            let error = decode_provider_configuration(raw).unwrap_err();
            assert!(!error.to_string().contains("PRIVATE"));
            decode_provider_configuration(br#"{"schema_version":1,"providers":{}}"#).unwrap();
        }
    }
    /// Null/missing maps preserve Go representation and are not advertised as
    /// discovery or model readiness. Hashing changes when the source bytes do.
    #[test]
    fn provider_configuration_empty_maps_and_source_hash_are_explicit() {
        let a = br#"{"schema_version":1,"providers":null}"#;
        let b = br#"{"schema_version":1,"providers":{}}"#;
        let x =
            ProviderConfiguration::observed(decode_provider_configuration(a).unwrap(), a, false);
        let y = ProviderConfiguration::observed(decode_provider_configuration(b).unwrap(), b, true);
        assert!(x.configuration.providers.is_none());
        assert!(y.configuration.providers.as_ref().unwrap().is_empty());
        assert_ne!(x.digest, y.digest);
        assert!(y.created && !y.provider_execution_started);
    }
    /// Native mutation requests reject ambiguous/unknown input, including inside
    /// the Go-compatible configuration value. No source/permission is inferred.
    #[test]
    fn provider_configuration_changes_require_exactly_one_strict_proposal() {
        let digest = "1".repeat(64);
        let valid = serde_json::json!({"schema_version":1,"expected_digest":digest,"configuration":{"schema_version":1,"providers":{}}});
        decode_provider_configuration_change(&serde_json::to_vec(&valid).unwrap()).unwrap();
        let mut invalid = Vec::new();
        let mut x = valid.clone();
        x["schema_version"] = 2.into();
        invalid.push(serde_json::to_vec(&x).unwrap());
        let mut x = valid.clone();
        x["expected_digest"] = "".into();
        invalid.push(serde_json::to_vec(&x).unwrap());
        let mut x = valid.clone();
        x["PRIVATE_UNKNOWN"] = true.into();
        invalid.push(serde_json::to_vec(&x).unwrap());
        let mut x = valid.clone();
        x["configuration"]["providers"]["x"] = serde_json::json!({"PRIVATE_UNKNOWN":true});
        invalid.push(serde_json::to_vec(&x).unwrap());
        let mut x = valid.clone();
        x["restore_reference"] = format!("providers-before-{digest}.json").into();
        invalid.push(serde_json::to_vec(&x).unwrap());
        invalid.push(format!(r#"{{"schema_version":1,"schema_version":1,"expected_digest":"{digest}","configuration":{{"schema_version":1}}}}"#).into_bytes());
        for raw in invalid {
            decode_provider_configuration_change(&serde_json::to_vec(&valid).unwrap()).unwrap();
            let e = decode_provider_configuration_change(&raw).err().unwrap();
            assert!(!e.to_string().contains("PRIVATE"));
        }
        let restored = serde_json::json!({"schema_version":1,"expected_digest":digest,"restore_reference":format!("providers-before-{digest}.json")});
        decode_provider_configuration_change(&serde_json::to_vec(&restored).unwrap()).unwrap();
    }
    /// Legacy and unique snapshot names are data references only; arbitrary paths,
    /// wrong hashes/case and empty suffixes cannot become filesystem selectors.
    #[test]
    fn provider_configuration_snapshot_references_never_accept_arbitrary_paths() {
        let digest = "ab".repeat(32);
        let valid = format!("providers-before-{digest}-snapshot-abc123.json");
        assert_eq!(configuration_backup_digest(&valid), Some(digest.as_str()));
        for reference in [
            "../providers.json".into(),
            "providers.json".into(),
            format!("providers-before-{}.json", "A".repeat(64)),
            format!("providers-before-{digest}-snapshot-.json"),
            format!("providers-before-{digest}-snapshot-../x.json"),
            format!("providers-before-{digest}-wrong-abc.json"),
        ] {
            assert_eq!(configuration_backup_digest(&valid), Some(digest.as_str()));
            assert!(configuration_backup_digest(&reference).is_none());
        }
    }
}
