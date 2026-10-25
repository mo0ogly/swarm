//! Cold-start compiled identity and read-only saved-policy candidate observation.
use crate::{Arguments, CliError, json_bytes, write_output};
use serde::Serialize;
use std::io::Write;
use swarm_engine::mission_status_contract::{BinaryVersion, VersionDiagnostic, VersionObservation};
#[derive(Serialize)]
struct Response {
    schema_version: i64,
    binary: BinaryVersion,
    diagnostic: VersionDiagnostic,
}
/// Genuine absent policy/server observation, never a claim that a failed probe
/// succeeded. Supplied static hints exclude paths, session URLs and private data.
fn unavailable(provenance: &str, hint: &str) -> VersionObservation {
    VersionObservation {
        available: false,
        version: None,
        commit: None,
        modified: None,
        provenance: provenance.into(),
        hint: hint.into(),
    }
}
/// Observe whether Git discovery has any repository marker to inspect. Walk
/// canonical ancestors for .git (worktrees/submodules) or HEAD (bare repositories).
/// Marker presence never proves validity: existing Git probes must still verify
/// it. Missing markers mean source identity unavailable at this observation only,
/// not a clean repository or a filesystem snapshot. Permission/path errors stay
/// fatal; no error text/exit-code heuristic or project mutation is used.
#[cfg(target_os = "linux")]
fn has_repository_marker(root: &str) -> Result<bool, std::io::Error> {
    let canonical = std::fs::canonicalize(root)?;
    for parent in canonical.ancestors() {
        for name in [".git", "HEAD"] {
            match std::fs::symlink_metadata(parent.join(name)) {
                Ok(_) => return Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(false)
}
/// Observe current Root Git only with its actual persisted settings. Missing
/// datastore is an explicit cold-start limitation and creates nothing. Present
/// Root/protection/Git failures refuse, preserving every cause and close error.
/// A genuine absence of repository markers leaves only source identity unknown;
/// it does not invalidate the separately decoded compiled binary identity.
/// Fixed Git commands inherit the authoritative saved policy, not a new timeout.
#[cfg(target_os = "linux")]
fn candidate(root: &str) -> Result<VersionObservation, CliError> {
    use swarm_engine::{RootStore, RootStoreError};
    let path = std::path::Path::new(root).join(".swarm/state.db");
    match std::fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(unavailable(
                "git",
                "Aucun magasin Root : la politique Git persistée n’est pas disponible.",
            ));
        }
        Err(e) => return Err(CliError::Input(e)),
    }
    let wrap = |error| CliError::Root(Box::new(error));
    let store = RootStore::open(std::path::Path::new(root), false).map_err(wrap)?;
    let result = (|| {
        if !has_repository_marker(root).map_err(CliError::Input)? {
            return Ok(unavailable("git", "Aucun dépôt Git observé : identité des sources indisponible."));
        }
        let git = store.inspect_git_metadata().map_err(wrap)?;
        let mut errors = git
            .issues
            .into_iter()
            .map(|issue| RootStoreError::Git(issue.cause));
        if let Some(first) = errors.next() {
            let mut failure = wrap(first);
            for cause in errors {
                failure = CliError::RootCleanup {
                    primary: Box::new(failure),
                    cleanup: Box::new(cause),
                };
            }
            return Err(failure);
        }
        if !git.diagnostics.is_empty() {
            return Err(wrap(RootStoreError::Refused(
                "version-git-observation-incomplete",
            )));
        }
        if git.state.head.is_empty() {
            return Ok(unavailable(
                "git",
                "Vérifiez --root et que le candidat est une copie Git lisible.",
            ));
        }
        Ok(VersionObservation {
            available: true,
            version: None,
            commit: Some(Box::new(git.state.head)),
            modified: Some(Box::new(!git.state.changes.is_empty())),
            provenance: "git".into(),
            hint: String::new(),
        })
    })();
    match (result, store.close()) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(wrap(error)),
        (Err(primary), Err(cleanup)) => Err(CliError::RootCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
/// This build has no saved-policy Root observer on other platforms; compiled
/// declarations remain portable, while current source observation is unavailable.
#[cfg(not(target_os = "linux"))]
fn candidate(_root: &str) -> Result<VersionObservation, CliError> {
    Ok(unavailable(
        "git",
        "Observation Root Git indisponible sur cette plateforme.",
    ))
}
/// Render the legacy text fields from already validated safe declarations.
/// This is presentation, not source equality or authentication of a binary.
fn text(response: &Response) -> String {
    let b = &response.binary;
    let unknown = "unknown";
    let mut result = format!(
        "Swarm {} (commit {}; modified={}; built={}; provenance={})\n",
        b.version,
        b.commit.as_deref().map_or(unknown, String::as_str),
        b.modified
            .as_deref()
            .map_or_else(|| unknown.into(), bool::to_string),
        b.build_date.as_deref().map_or(unknown, String::as_str),
        b.provenance
    );
    let show = |v: &VersionObservation| {
        if !v.available {
            "inconnue".into()
        } else {
            let mut parts = Vec::new();
            if let Some(s) = &v.version {
                parts.push(s.as_str())
            }
            if let Some(s) = &v.commit {
                parts.push(s.as_str())
            }
            if parts.is_empty() {
                "inconnue".into()
            } else {
                parts.join(" · ")
            }
        }
    };
    let d = &response.diagnostic;
    result.push_str(&format!(
        "CLI installée : {}\nServeur actif : {}\nCandidat : {}\nÉtat : {}\n",
        show(&d.installed_cli),
        show(&d.active_server),
        show(&d.candidate),
        d.state
    ));
    for difference in d.differences.as_deref().unwrap_or_default() {
        result.push_str(&format!(
            "Divergence : {} / {}\n",
            difference.left, difference.right
        ));
    }
    if !d.next_step.is_empty() {
        result.push_str(&format!("Action : {}\n", d.next_step));
    }
    result
}
/// Public version/--version, text or historical JSON schema1. Decode immutable
/// compile-time declarations first, then open configured Root nominally read-only.
/// No mutation is requested; opening SQLite may perform journal recovery, so
/// this is not an unconditional zero-effect guarantee. Idle fixtures are checked.
/// --server explicitly refuses until its local HTTP/session policy is ported;
/// it never reports an unattempted request as a successful observation. Encode
/// before writing and flush; IO failures may follow partial bytes, with no retry.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos.len() != 1
        || !a.input.is_empty()
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    if a.server {
        return Err(CliError::Unsupported);
    }
    let binary = swarm_engine::binary_version().map_err(|e| {
        CliError::Root(Box::new(swarm_engine::RootStoreError::Runtime(
            swarm_engine::RuntimeError::new(swarm_engine::RuntimeOperation::Platform, e),
        )))
    })?;
    let installed = swarm_engine::binary_version_observation(&binary);
    let server = unavailable("http", "Aucune URL de session serveur fournie.");
    let diagnostic =
        swarm_engine::compare_version_observations(installed, server, candidate(&a.root)?);
    let response = Response {
        schema_version: 1,
        binary,
        diagnostic,
    };
    let bytes = if a.json {
        json_bytes(&response).map_err(CliError::Output)?
    } else {
        text(&response).into_bytes()
    };
    write_output(out, &bytes)?;
    Ok(0)
}
