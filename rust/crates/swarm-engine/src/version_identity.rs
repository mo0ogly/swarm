//! Explicit compiled build declarations, never runtime environment or actor proof.
use crate::mission_status_contract::{
    BinaryVersion, VersionDiagnostic, VersionDifference, VersionObservation,
};
use serde::Deserialize;
use std::{error::Error, fmt};
/// Safe metadata failure. Original codec/date causes remain inspectable without
/// putting supplied build JSON or environment values in Display/Debug.
pub struct BuildIdentityError {
    code: &'static str,
    cause: Option<Box<dyn Error + Send + Sync>>,
}
impl BuildIdentityError {
    /// Static reason only; this classifies refusal, never rollback or authenticity.
    pub fn code(&self) -> &'static str {
        self.code
    }
}
impl fmt::Display for BuildIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl fmt::Debug for BuildIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}
impl Error for BuildIdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}
fn refused(code: &'static str) -> BuildIdentityError {
    BuildIdentityError { code, cause: None }
}
fn caused(code: &'static str, error: impl Error + Send + Sync + 'static) -> BuildIdentityError {
    BuildIdentityError {
        code,
        cause: Some(Box::new(error)),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    schema_version: i64,
    version: String,
    commit: Option<String>,
    modified: Option<bool>,
    build_date: Option<String>,
}
/// Strict legacy SemVer with v prefix; numeric prerelease identifiers cannot
/// have leading zeros. This is wire syntax, not an operator version policy.
pub(crate) fn release_version(s: &str) -> bool {
    let Some(s) = s.strip_prefix('v') else {
        return false;
    };
    let parts: Vec<_> = s.split('+').collect();
    if parts.len() > 2 {
        return false;
    }
    let identifiers = |s: &str| {
        !s.is_empty()
            && s.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    };
    if parts.len() == 2 && !identifiers(parts[1]) {
        return false;
    }
    let (core, pre) = parts[0]
        .split_once('-')
        .map_or((parts[0], None), |(a, b)| (a, Some(b)));
    let core: Vec<_> = core.split('.').collect();
    if core.len() != 3
        || core.iter().any(|p| {
            p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
        })
    {
        return false;
    }
    pre.is_none_or(|p| {
        identifiers(p)
            && p.split('.').all(|i| {
                !(i.len() > 1 && i.starts_with('0') && i.bytes().all(|b| b.is_ascii_digit()))
            })
    })
}
/// Legacy Git SHA-1 object-name syntax. No path/label/current checkout supplies
/// or authenticates a compiled commit; other object formats need a new contract.
fn commit(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// Decode supplied build declarations only. None is truthful unqualified devel;
/// malformed present JSON/unknown fields/version/commit/date refuse without
/// falling back to a clean or current checkout. Non-devel needs all build facts.
/// Dates normalize to UTC seconds like Go RFC3339 output. No filesystem, network,
/// environment mutation, agent permission or source-byte attestation follows.
pub fn decode_binary_version(raw: Option<&str>) -> Result<BinaryVersion, BuildIdentityError> {
    let Some(raw) = raw else {
        return Ok(BinaryVersion {
            version: "devel".into(),
            commit: None,
            modified: None,
            build_date: None,
            provenance: "unknown".into(),
        });
    };
    let input: Metadata =
        serde_json::from_str(raw).map_err(|e| caused("build-identity-json-invalid", e))?;
    if input.schema_version != 1 {
        return Err(refused("build-identity-schema-unsupported"));
    }
    if input.version != "devel" && !release_version(&input.version) {
        return Err(refused("build-identity-version-invalid"));
    }
    if input.commit.as_ref().is_some_and(|value| !commit(value)) {
        return Err(refused("build-identity-commit-invalid"));
    }
    if input.version != "devel"
        && (input.commit.is_none() || input.modified.is_none() || input.build_date.is_none())
    {
        return Err(refused("build-identity-release-incomplete"));
    }
    let date = input
        .build_date
        .map(|raw| {
            let value = swarm_wire_json::go_rfc3339::parse(&raw)
                .map_err(|e| caused("build-identity-date-invalid", e))?;
            value
                .to_offset(time::UtcOffset::UTC)
                .replace_nanosecond(0)
                .map_err(|e| caused("build-identity-date-invalid", e))?
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|e| caused("build-identity-date-invalid", e))
        })
        .transpose()?;
    Ok(BinaryVersion {
        version: input.version,
        commit: input.commit.map(Box::new),
        modified: input.modified.map(Box::new),
        build_date: date.map(Box::new),
        provenance: "injected".into(),
    })
}
/// Observe only the metadata embedded by the compiler. Runtime SWARM_* changes
/// cannot alter this identity. A direct Cargo build without explicit injection
/// remains unknown; malformed embedded declarations refuse, never borrow HEAD.
/// Official build tooling must separately verify actual Git and source inputs.
pub fn binary_version() -> Result<BinaryVersion, BuildIdentityError> {
    decode_binary_version(option_env!("SWARM_NATIVE_BUILD_METADATA"))
}
/// Project a supplied binary declaration for legacy diagnostics. Availability
/// means data exists, not that nullable commit/modified fields are known.
pub fn binary_version_observation(binary: &BinaryVersion) -> VersionObservation {
    VersionObservation {
        available: true,
        version: Some(Box::new(binary.version.clone())),
        commit: binary.commit.clone(),
        modified: binary.modified.clone(),
        provenance: binary.provenance.clone(),
        hint: String::new(),
    }
}
/// Compare declared commit names with the legacy three-role contract. Missing
/// commit yields unknown; equal commits mean declared-commit equality only,
/// never equality of dirty trees, build flags, binary bytes or authenticated code.
/// No IO, source adoption, permission or installation occurs.
pub fn compare_version_observations(
    installed: VersionObservation,
    server: VersionObservation,
    candidate: VersionObservation,
) -> VersionDiagnostic {
    let mut missing = Vec::new();
    let mut differences = Vec::new();
    let roles = [
        ("installed_cli", &installed),
        ("active_server", &server),
        ("candidate", &candidate),
    ];
    for (name, value) in roles {
        if !value.available || value.commit.as_ref().is_none_or(|v| !commit(v)) {
            missing.push(name);
        }
    }
    let (state, next_step) = if !missing.is_empty() {
        (
            "unknown",
            format!(
                "Rendez accessibles les identités manquantes ({}) puis relancez le diagnostic. Pour le serveur actif, fournissez son URL de session avec --server.",
                missing.join(", ")
            ),
        )
    } else {
        for i in 0..roles.len() {
            for j in i + 1..roles.len() {
                if roles[i].1.commit != roles[j].1.commit {
                    differences.push(VersionDifference {
                        left: roles[i].0.into(),
                        right: roles[j].0.into(),
                    });
                }
            }
        }
        if differences.is_empty() {
            ("identical", String::new())
        } else {
            ("divergent","Installez ou redémarrez le composant divergent avec le candidat voulu, puis relancez le diagnostic.".into())
        }
    };
    VersionDiagnostic {
        installed_cli: installed,
        active_server: server,
        candidate,
        state: state.into(),
        differences: Some(differences),
        next_step,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Declared fixture only, not the real repository commit or build evidence.
    fn good() -> serde_json::Value {
        serde_json::json!({"schema_version":1,"version":"v1.2.3-dev.1","commit":"a".repeat(40),"modified":true,"build_date":"2026-10-24T03:04:05.123+02:00"})
    }
    /// Valid metadata before malformed schema/commit/version/date/unknown field.
    /// Safe error formatting never copies private input and serde causes survive.
    #[test]
    fn version_identity_decode_defaults_valid_and_refusal() {
        let plain = decode_binary_version(None).unwrap();
        assert_eq!(plain.version, "devel");
        assert_eq!(plain.provenance, "unknown");
        assert!(plain.commit.is_none() && plain.modified.is_none() && plain.build_date.is_none());
        let valid = decode_binary_version(Some(&good().to_string())).unwrap();
        assert_eq!(valid.provenance, "injected");
        assert_eq!(
            valid.build_date.as_deref().map(String::as_str),
            Some("2026-10-24T01:04:05Z")
        );
        assert_eq!(valid.modified.as_deref(), Some(&true));
        for (key, value) in [
            ("schema_version", serde_json::json!(2)),
            ("commit", serde_json::json!("PRIVATE")),
            ("version", serde_json::json!("v1.2.3-01")),
            ("build_date", serde_json::json!("PRIVATE")),
            ("PRIVATE", serde_json::json!("SECRET")),
        ] {
            let mut input = good();
            input[key] = value;
            let error = decode_binary_version(Some(&input.to_string()))
                .err()
                .expect("invalid metadata must refuse");
            assert!(!format!("{error:?} {error}").contains("PRIVATE"));
            assert!(decode_binary_version(Some(&good().to_string())).is_ok());
        }
        let error = decode_binary_version(Some("{PRIVATE")).err().unwrap();
        assert!(error.source().is_some());
    }
    /// No release completion from implicit values; null metadata is not absent.
    #[test]
    fn version_identity_release_needs_complete_declared_facts() {
        let mut input = good();
        input["modified"] = serde_json::Value::Null;
        assert!(decode_binary_version(Some(&input.to_string())).is_err());
        assert!(decode_binary_version(Some("null")).is_err());
        for version in ["v0.0.0", "v1.2.3+build.01", "v1.2.3-dev.0"] {
            assert!(release_version(version));
        }
        for version in ["1.2.3", "v01.2.3", "v1.2.3-01", "v1.2.3+", "v1.2.3-dev..1"] {
            assert!(!release_version(version));
        }
    }
    /// Equality covers declared commits only. Dirty-tree equality and actual
    /// source/binary conformance require independent source/profile evidence.
    #[test]
    fn version_identity_comparison_preserves_unknown_and_declared_commit_scope() {
        let a =
            binary_version_observation(&decode_binary_version(Some(&good().to_string())).unwrap());
        let unknown = binary_version_observation(&decode_binary_version(None).unwrap());
        assert_eq!(
            compare_version_observations(a.clone(), unknown, a.clone()).state,
            "unknown"
        );
        assert_eq!(
            compare_version_observations(a.clone(), a.clone(), a.clone()).state,
            "identical"
        );
        let mut b = a.clone();
        b.commit = Some(Box::new("b".repeat(40)));
        let different = compare_version_observations(a.clone(), b, a);
        assert_eq!(different.state, "divergent");
        assert_eq!(different.differences.unwrap().len(), 2);
    }
}
