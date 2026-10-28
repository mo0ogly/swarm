//! Read-only project-profile status projection, Go `ProjectProfileStatus`
//! compatible.
//!
//! Observations only: Claude/GLM native settings detection is metadata-only
//! (names are canonicalized and required to be contained regular files, but
//! contents are never opened or read), and role contexts come from the
//! existing [`crate::read_project_context`] reader, which alone owns format,
//! byte-cap, secret and managed-memory validation. The existing reader creates a transient prompt, which is dropped; no prompt
//! or source text is persisted or returned; no provider call, config mutation,
//! content import or unsafe code is introduced. Projection helpers do not lock;
//! the Root wrapper below owns the material/settings reservation.

use std::{io, path::Path};

use crate::{LocalPathError, PathOperation, PathRefusal};

/// Go-compatible project-profile status. `roles` and
/// `claude_settings_detected` are always present, non-nil arrays; `name`
/// follows the Go `omitempty` string contract. Metadata only: never embeds
/// prompt, source or settings content.
#[derive(serde::Serialize)]
pub struct ProjectProfileStatus {
    /// True once at least one protocol role has observed a project profile.
    pub enabled: bool,
    /// Profile name from the last observed role context; omitted when empty
    /// (Go `name,omitempty`).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Observed role contexts in protocol order; always serialized.
    pub roles: Vec<crate::ProjectContext>,
    /// Admitted Claude/GLM native settings names, in detection order; always
    /// serialized.
    pub claude_settings_detected: Vec<String>,
}

/// Fixed Claude/GLM native settings filenames, in detection order. Protocol
/// capability vocabulary, not an operator ceiling.
const CLAUDE_SETTINGS_FILENAMES: &[&str] = &[
    ".claude/settings.json",
    ".claude/settings.local.json",
    ".claude/glm-settings.json",
];

/// Fixed protocol roles, in observation order. Protocol capability
/// vocabulary, not an operator ceiling.
const PROJECT_PROFILE_ROLES: &[&str] =
    &["preparation", "planner", "subplanner", "worker", "reviewer"];

/// Observe project-profile status at `root` under `limits`.
///
/// Inputs: `root` is the project root; `limits` is the effective context
/// policy, forwarded untouched to the existing reader.
///
/// Effects: metadata-only detection of the fixed settings names via
/// [`crate::local_file`], then one sequential [`crate::read_project_context`]
/// observation per protocol role; only the returned context is consumed and
/// the observation prompt is dropped. `Ok(None)` (profile absent) contributes
/// nothing.
///
/// Errors: reader failures map to [`super::RootStoreError::Context`]; every
/// [`crate::LocalPathError`] outside the absent-file classifications maps to
/// [`super::RootStoreError::Path`]. Errors are never swallowed into an empty
/// success.
///
/// Limits: sequential observations, not an atomic filesystem snapshot, future
/// lease or authority; no locking and no `Root` methods here — the caller
/// wraps the single Root/settings material reservation.
pub(super) fn status(
    root: &Path,
    limits: &crate::ContextPolicyLimits,
) -> Result<ProjectProfileStatus, super::RootStoreError> {
    let mut out = ProjectProfileStatus {
        enabled: false,
        name: String::new(),
        roles: Vec::new(),
        claude_settings_detected: Vec::new(),
    };
    detect_claude_settings(root, &mut out.claude_settings_detected)?;
    for &role in PROJECT_PROFILE_ROLES {
        // Only the projected context is consumed; the observation prompt and
        // any source text are dropped, never returned.
        if let Some(observation) = crate::read_project_context(root, role, limits)
            .map_err(super::RootStoreError::Context)?
        {
            out.enabled = true;
            out.name = observation.context.name.clone();
            out.roles.push(observation.context);
        }
    }
    Ok(out)
}

/// Metadata-only detection of the fixed Claude/GLM native settings names, in
/// detection order.
///
/// Effects: each name is checked with [`crate::local_file`], which
/// canonicalizes and requires a contained regular file but never opens or
/// reads contents; only admitted names are appended to `out`.
///
/// Errors: absent-file classifications are skipped (see
/// [`is_absent_metadata`]); every other [`crate::LocalPathError`] propagates
/// as [`super::RootStoreError::Path`], including access/resource failures and
/// malformed roots.
fn detect_claude_settings(
    root: &Path,
    out: &mut Vec<String>,
) -> Result<(), super::RootStoreError> {
    for &name in CLAUDE_SETTINGS_FILENAMES {
        match crate::local_file(root, name) {
            Ok(_) => out.push(name.to_string()),
            Err(err) if is_absent_metadata(&err) => {}
            Err(err) => return Err(super::RootStoreError::Path(err)),
        }
    }
    Ok(())
}

/// Classify a [`crate::LocalPathError`] from [`crate::local_file`] as "the
/// name is not present as a contained regular file", which detection skips.
///
/// Skips refusals `OutsideProject` and `RegularFileRequired`, and `Io` on
/// `ResolveFile` or `FileMetadata` whose cause kind is `NotFound` or
/// `NotADirectory` (std maps Linux `ENOTDIR` to `NotADirectory`). Everything
/// else — root resolution failures, access and resource errors — is not
/// absence and must propagate.
fn is_absent_metadata(err: &LocalPathError) -> bool {
    match err {
        LocalPathError::Refused(PathRefusal::OutsideProject)
        | LocalPathError::Refused(PathRefusal::RegularFileRequired) => true,
        LocalPathError::Io { operation, cause }
            if matches!(
                operation,
                PathOperation::ResolveFile | PathOperation::FileMetadata
            ) =>
        {
            matches!(
                cause.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            )
        }
        _ => false,
    }
}
impl super::RootStore {
    /// Read actual project-context metadata under one saved policy/material
    /// reservation. Linux also retains the project descriptor bound to its opened
    /// datastore while observing roles. Existing validation remains authoritative; role reads
    /// are sequential observations, not an atomic filesystem or execution lease.
    /// No prompt/native permission content, profile mutation or agent follows.
    pub fn project_profile_status(&self)->Result<ProjectProfileStatus,super::RootStoreError>{
        #[cfg(target_os="linux")]
        {self.with_project_profile(false,|project,limits|status(&project.process_path(),limits))}
        #[cfg(not(target_os="linux"))]
        {super::material_path::with_material_read(self,||{
            let limits=self.read_settings()?.values.context_policy_limits().map_err(super::RootStoreError::Settings)?;
            status(&self.root,&limits)
        })}
    }
}
