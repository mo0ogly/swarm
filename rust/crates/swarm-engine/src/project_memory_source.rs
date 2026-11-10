//! Read-only profile source guard over coherent historical Work memory.
//! No version is selected/admitted and no provider or agent is started.
use super::{invalid, ProjectContextError, Result};
use std::path::Path;

/// Preserve both Root operation and explicit cleanup causes. A close failure
/// never becomes an empty inventory or permission; known reads remain reads.
#[cfg(target_os = "linux")]
fn finish<T>(
    value: std::result::Result<T, crate::RootStoreError>,
    closed: std::result::Result<(), crate::RootStoreError>,
) -> std::result::Result<T, crate::RootStoreError> {
    match (value, closed) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (value, Err(cleanup)) => Err(crate::RootStoreError::Cleanup {
            primary: value.err().map(Box::new),
            cleanup: Box::new(cleanup),
        }),
    }
}

/// Guard the actual opened instruction's contained physical relative path and
/// complete UTF-8 body. Non-guide documents require no memory SQL inventory.
/// Guide sources use a separate read-only bound Root (never the caller's writer)
/// and one coherent complete Work SELECT. A second Root open must match retained
/// directory/connected-DB descriptors before returning; all handles explicitly
/// close and all primary/secondary causes survive behind a static diagnostic.
/// Local nonimported lessons remain managed after withdrawal; the index refuses
/// only actual historical references. No source is rewritten, imported, selected
/// as a version or granted permission. This is an observation, not a filesystem
/// freeze, agent receipt, OS confinement or promise of future freshness.
#[cfg(target_os = "linux")]
pub(super) fn check(root: &Path, relative: &Path, body: &str) -> Result<()> {
    if !relative.starts_with(".claude/field-guide") {
        return Ok(());
    }
    let canonical = crate::canonical_project_root(root)
        .map_err(|e| ProjectContextError::caused("project-memory-source-root", e))?;
    let project = canonical
        .to_str()
        .ok_or_else(|| invalid("project-memory-root-not-utf8"))?;
    let store = crate::RootStore::open(root, false)
        .map_err(|e| ProjectContextError::caused("project-memory-source-inventory", e))?;
    let result = (|| {
        let binding = store.bind_root()?;
        let result = (|| {
            let works = store.list_works()?;
            let paths =
                crate::memory_files::managed_paths(&works, &super::hash(project.as_bytes()));
            let observed = crate::RootStore::open(root, false)?;
            let same = binding.matches(&observed).and_then(|same| {
                if same {
                    Ok(paths)
                } else {
                    Err(crate::RootStoreError::Refused(
                        "project-memory-root-instance-changed",
                    ))
                }
            });
            finish(same, observed.close())
        })();
        finish(result, binding.close())
    })();
    let paths = finish(result, store.close())
        .map_err(|e| ProjectContextError::caused("project-memory-source-inventory", e))?;
    let name = relative
        .to_str()
        .ok_or_else(|| invalid("project-memory-path-not-utf8"))?;
    if paths.contains(name)
        || (name == ".claude/field-guide/index.md"
            && super::memory_guide::guide_references_managed(body, &paths))
    {
        return Err(invalid("project-source-memory-managed"));
    }
    Ok(())
}

/// Other targets retain an explicit refusal for guide sources instead of opening
/// an unbound database. Ordinary instructions keep their existing reader rules.
#[cfg(not(target_os = "linux"))]
pub(super) fn check(_root: &Path, relative: &Path, _body: &str) -> Result<()> {
    if relative.starts_with(".claude/field-guide") {
        Err(invalid("versioned-memory-source-platform-unsupported"))
    } else {
        Ok(())
    }
}
