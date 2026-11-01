//! Authority daemon configuration (phase B): session lifetimes and Argon2
//! costs. Never hard-coded in code: the installation defaults are the shipped
//! file `config/authority-defaults.json` (embedded), and the operator's saved
//! file `authority.json` in the declared configuration directory overrides
//! them as a whole. Admin editing arrives with B8 through the daemon API.
//!
//! Validation: lifetimes strictly positive with idle <= absolute; Argon2
//! costs accepted by the Argon2 parameter contract; relay body limit
//! strictly positive. Unknown fields refuse.
use serde::{Deserialize, Serialize};
use swarm_enrollment_authority::credentials::Parameters;

/// Validated configuration snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityConfig {
    pub schema_version: i64,
    /// Maximum lifetime of a session from sign-in, in seconds.
    pub session_absolute_seconds: i64,
    /// Inactivity after which a session ends, in seconds (<= absolute).
    pub session_idle_seconds: i64,
    /// Argon2id costs used for new password verifiers.
    pub argon2: Parameters,
    /// Largest request body the daemon reads before relaying to the engine.
    pub relay_body_max_bytes: i64,
}

/// Installation default, read from the shipped configuration file.
pub const DEFAULTS: &str = include_str!("../config/authority-defaults.json");

impl AuthorityConfig {
    /// Decode and validate raw JSON. Errors: `authority-config-format` for a
    /// malformed or unknown-field document, `authority-config-invalid` for
    /// out-of-contract values. No IO.
    pub fn decode(raw: &[u8]) -> Result<Self, &'static str> {
        let c: Self = serde_json::from_slice(raw).map_err(|_| "authority-config-format")?;
        c.validate()?;
        Ok(c)
    }
    /// Shipped installation default (always valid; checked by tests).
    pub fn installation() -> Result<Self, &'static str> {
        Self::decode(DEFAULTS.as_bytes())
    }
    /// Contract checks; see module docs.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1
            || self.session_absolute_seconds < 1
            || self.session_idle_seconds < 1
            || self.session_idle_seconds > self.session_absolute_seconds
            || self.argon2.validate().is_err()
            || self.relay_body_max_bytes < 1
        {
            return Err("authority-config-invalid");
        }
        Ok(())
    }
    /// Load `dir/authority.json` if present, else the installation default.
    /// A present but invalid file refuses (no silent fallback to defaults);
    /// read errors other than not-found propagate as `authority-config-io`.
    pub fn load(dir: &std::path::Path) -> Result<Self, &'static str> {
        match std::fs::read(dir.join("authority.json")) {
            Ok(raw) => Self::decode(&raw),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::installation(),
            Err(_) => Err("authority-config-io"),
        }
    }
}
