//! Swarm cockpit authority: installation checks, accounts, sessions, policy,
//! signed assertions, audit and the HTTP front (see [`server`] and [`sessions`]).
//!
//! The service validates a pinned installation declaration and runs under its
//! dedicated `service_uid`, distinct from `controller_uid`. The HTTP front holds
//! sign-in sessions and relays authorized requests with signed assertions; the
//! engine validates those assertions. The read-only [`check`] operation alone
//! creates no socket, session or account. Failures retain typed refusal codes.
//!
//! These sources describe capabilities, not the identity of an installed image.
//! Cockpit UID separation does not establish protected PAEA effect authority,
//! human presence, agent confinement or qualification of the complete chain.
//! See `docs/plans/acces-cockpit/PHASE-B-AUTORITE.md` for delivery history.
#![forbid(unsafe_code)]
use serde::Serialize;

/// Multi-account registry on the signed journal (slice B2.2).
pub mod accounts;
/// Daemon configuration: session lifetimes, Argon2 costs (no hard-coded policy).
pub mod config;
/// Daemon-held sign-in sessions (slice B4).
pub mod sessions;
/// Signed per-request assertion relayed to the engine (slice B5.1).
pub mod assertion;
/// Attribute-based access decision: route classes, grants, Biba floor (B6).
pub mod admin;
pub mod audit;
/// Bounded diagnostics file of the daemon (ADM-09).
pub mod diag;
pub mod keys;
pub mod policy;
/// HTTP front: sign-in, sessions and signed relay to the engine (slice B5.3).
pub mod server;

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
