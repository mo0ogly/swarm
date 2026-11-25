//! Authority daemon configuration (phase B): session lifetimes and Argon2
//! costs. Never hard-coded in code: the installation defaults are the shipped
//! file `config/authority-defaults.json` (embedded), and the operator's saved
//! file `authority.json` in the declared configuration directory overrides
//! them as a whole. Edited from the Admin through the daemon API (B8,
//! [`crate::admin`]), applied live.
//!
//! Validation: lifetimes strictly positive with idle <= absolute; Argon2
//! costs accepted by the Argon2 parameter contract; relay body limit
//! strictly positive; audit retention (when set) at least one day; log
//! level is from LOG_LEVELS. Unknown fields refuse;
//! `audit_retention_days` and `log_level` are optional on read so documents
//! written before them still decode (they are always written back).
use serde::{Deserialize, Serialize};
use swarm_enrollment_authority::credentials::Parameters;

/// Levels of the daemon diagnostics log, least to most verbose.
pub const LOG_LEVELS: [&str; 5] = ["critical", "error", "warn", "info", "debug"];

/// Validated configuration snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Audit lines older than this many days are purged daily by the daemon
    /// (`None` = no automatic purge; the manual purge stays available).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_retention_days: Option<i64>,
    /// Verbosity of the daemon diagnostics file (`debug`…`error`).
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Saved diagnostics policy. Older files inherit the shipped diagnostics policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticsConfig>,
}

/// Operator settings for the authority log. Paths remain under its declared directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticsConfig {
    pub max_bytes: u64,
    pub archives: usize,
    pub page_rows: usize,
}

/// Compatibility default for older configuration files comes from the same
/// shipped catalogue as fresh installations. Invalid packaged data then fails
/// the shared validation.
fn default_log_level() -> String {
    #[derive(Deserialize)]
    struct Defaults {
        log_level: String,
    }
    serde_json::from_str::<Defaults>(DEFAULTS).map(|value| value.log_level).unwrap_or_default()
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
            || self.audit_retention_days.is_some_and(|d| d < 1)
            || !LOG_LEVELS.contains(&self.log_level.as_str())
        {
            return Err("authority-config-invalid");
        }
        self.diagnostics()?;
        Ok(())
    }
    /// Return saved policy or the packaged policy for older configuration files.
    /// A malformed shipped policy is a configuration error, never an empty default.
    pub fn diagnostics(&self) -> Result<DiagnosticsConfig, &'static str> {
        #[derive(Deserialize)]
        struct Defaults {
            diagnostics: DiagnosticsConfig,
        }
        let config = match self.diagnostics {
            Some(config) => config,
            None => serde_json::from_str::<Defaults>(DEFAULTS)
                .map_err(|_| "authority-config-format")?.diagnostics,
        };
        if config.max_bytes == 0 || config.archives == 0 || config.page_rows == 0 {
            return Err("authority-config-invalid");
        }
        Ok(config)
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
