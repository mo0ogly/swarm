//! Swarm cockpit authority daemon — phase B, slices B2.1 (declaration check)
//! and B2.2 (account registry, see [`accounts`]).
//!
//! The daemon is meant to run under a dedicated system UID (`service_uid`)
//! declared in a protected installation declaration, distinct from the
//! operator UID (`controller_uid`) that Swarm agents share. This slice only
//! loads and checks that declaration and provides the account registry; it
//! opens no socket or session and grants no authority by itself. See
//! `docs/plans/acces-cockpit/PHASE-B-AUTORITE.md`.
#![forbid(unsafe_code)]
use serde::Serialize;

/// Multi-account registry on the signed journal (slice B2.2).
pub mod accounts;

/// Facts established by [`check`], safe to print: UIDs and directories only,
/// never pins' private material (the declaration holds none).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Readiness {
    /// Protocol version of this report.
    pub schema_version: i64,
    /// UID the daemon runs as; equal to the current effective UID.
    pub service_uid: u32,
    /// Operator UID whose local peers the daemon will admit (with a human secret).
    pub controller_uid: u32,
    /// Private configuration directory, opened and verified by the loader.
    pub configuration_directory: String,
    /// Private witness directory, distinct from the configuration directory.
    pub witness_directory: String,
}

/// Load the installation declaration at `path`, whose SHA-256 must equal the
/// independently supplied `pin`, and report readiness.
///
/// Preconditions: Linux; `path` is a private regular file in a private
/// directory owned by the current effective UID. Delegates every check to
/// `swarm_enrollment_bootstrap::load`: exact pin, canonical document,
/// `service_uid` == euid, `service_uid` != `controller_uid`, disjoint private
/// roots, no symlink following.
/// Errors: the loader's typed error, unchanged (stable `code()`).
/// Effects: read-only; creates no lock, file, socket or account.
/// Limits: proves the declaration matches this process, not that agents run
/// under `controller_uid` nor that the host confines them.
#[cfg(target_os = "linux")]
pub fn check(path: &str, pin: &str) -> Result<Readiness, swarm_enrollment_bootstrap::Error> {
    let d = swarm_enrollment_bootstrap::load(path, pin)?;
    Ok(Readiness {
        schema_version: 1,
        service_uid: d.service_uid,
        controller_uid: d.controller_uid,
        configuration_directory: d.configuration_directory,
        witness_directory: d.witness_directory,
    })
}
