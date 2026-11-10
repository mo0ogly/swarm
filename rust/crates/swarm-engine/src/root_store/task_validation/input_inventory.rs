//! Initial regular candidate inputs, including undeclared local dependencies.
//! Generated outputs, external programs/environment and IPC are not a hermetic
//! dependency graph. Hashes never authorize an operation or reveal file contents.
use crate::{root_store::RootStoreError as E, AgentLaunchError as A};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

fn caused(code: &'static str, e: impl std::error::Error + Send + Sync + 'static) -> E {
    E::Launch(Box::new(A::caused(code, e)))
}
/// Observe a known changed regular input for explicit proof renewal. Missing
/// files count only on an actual NotFound metadata observation; access/encoding/
/// redirection errors remain errors. This does not establish new control success.
pub(super) fn changed(
    root: &Path,
    workspace: &Path,
    expected: &BTreeMap<String, String>,
) -> Result<bool, E> {
    let prefix = workspace
        .strip_prefix(root)
        .map_err(|e| caused("managed-input-inventory-workspace", e))?;
    let mut changed = false;
    for (name, hash) in expected {
        if !Path::new(name).starts_with(prefix) || Path::new(name).components().any(|p| !matches!(p,std::path::Component::Normal(_))) {
            return Err(E::Refused("managed-input-inventory-outside-workspace"));
        }
        match std::fs::symlink_metadata(root.join(name)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                changed = true;
                continue;
            }
            Err(e) => return Err(caused("managed-input-inventory-stat", e)),
            Ok(m) if !m.is_file() => {
                return Err(E::Refused("managed-input-inventory-type-changed"))
            }
            Ok(_) => {}
        }
        let (actual_path, actual) = crate::gate_evaluation::artifact_hash(root, name)
            .map_err(|e| caused("managed-input-inventory-read", e))?;
        if !actual_path.starts_with(workspace) {
            return Err(E::Refused("managed-input-inventory-outside-workspace"));
        }
        changed |= &actual != hash;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Same owned regular witness before drift, missing/type/escape faults and
    /// exact restoration. Later generated output does not change initial scope.
    #[test]
    fn initial_inputs_detect_undeclared_changes_and_preserve_generated_outputs() {
        let root = crate::fixture_root("gi-inputs-");
        let workspace = root.join("candidate");
        std::fs::create_dir(&workspace).unwrap();
        std::fs::write(workspace.join("config"), b"V1").unwrap();
        let initial = collect(&root, &workspace, &BTreeSet::new()).unwrap();
        assert_eq!(initial.len(), 1);
        assert!(!changed(&root, &workspace, &initial).unwrap());
        std::fs::write(workspace.join("output"), b"generated").unwrap();
        assert_eq!(observe(&root, &workspace, &initial).unwrap(), initial);
        std::fs::write(workspace.join("config"), b"V2").unwrap();
        assert!(changed(&root, &workspace, &initial).unwrap());
        assert!(observe(&root, &workspace, &initial).is_err());
        std::fs::write(workspace.join("config"), b"V1").unwrap();
        assert_eq!(observe(&root, &workspace, &initial).unwrap(), initial);
        std::fs::remove_file(workspace.join("config")).unwrap();
        assert!(changed(&root, &workspace, &initial).unwrap());
        assert!(observe(&root, &workspace, &initial).is_err());
        std::fs::write(workspace.join("config"), b"V1").unwrap();
        std::os::unix::fs::symlink("config", workspace.join("unknown-link")).unwrap();
        assert!(collect(&root, &workspace, &BTreeSet::new()).is_err());
        assert!(observe(
            &root,
            &workspace,
            &BTreeMap::from([("outside".into(), "x".into())])
        )
        .is_err());
    }
}
/// Collect every preexisting regular file below the pinned candidate workspace.
/// Tracked links already have a literal-target witness; untracked links refuse
/// rather than silently following an unbound dependency. Git's private metadata
/// and IPC nodes are outside this regular-file contract. Reads use the existing
/// anchored artifact boundary; IO/containment/type errors preserve their causes.
/// No writes, content export, cap, grant or same-UID filesystem lease is created.
pub(super) fn collect(
    root: &Path,
    workspace: &Path,
    tracked_links: &BTreeSet<PathBuf>,
) -> Result<BTreeMap<String, String>, E> {
    let mut directories = vec![workspace.to_path_buf()];
    let mut result = BTreeMap::new();
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(&directory)
            .map_err(|e| caused("managed-input-inventory-directory", e))?
        {
            let entry = entry.map_err(|e| caused("managed-input-inventory-entry", e))?;
            let path = entry.path();
            if directory == workspace && entry.file_name() == ".git" {
                continue;
            }
            let kind = entry
                .file_type()
                .map_err(|e| caused("managed-input-inventory-type", e))?;
            if kind.is_symlink() {
                if !tracked_links.contains(&path) {
                    return Err(E::Refused("managed-input-inventory-untracked-link"));
                }
            } else if kind.is_dir() {
                directories.push(path);
            } else if kind.is_file() {
                let name = path
                    .strip_prefix(root)
                    .ok()
                    .and_then(Path::to_str)
                    .ok_or(E::Refused("managed-input-inventory-outside-root"))?;
                let (actual_path, hash) = crate::gate_evaluation::artifact_hash(root, name)
                    .map_err(|e| caused("managed-input-inventory-read", e))?;
                if !actual_path.starts_with(workspace) {
                    return Err(E::Refused("managed-input-inventory-outside-workspace"));
                }
                result.insert(name.into(), hash);
            }
        }
    }
    Ok(result)
}
/// Rehash exactly the initial file selection, allowing later generated outputs.
/// Require every retained name inside the current candidate workspace. Missing,
/// unreadable, redirected or changed inputs refuse with typed diagnostics; a
/// matching hash is no future lease. Stored hashes are separately bound to the
/// native source origin by the caller, never trusted as caller authority.
pub(super) fn observe(
    root: &Path,
    workspace: &Path,
    expected: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, E> {
    let prefix = workspace
        .strip_prefix(root)
        .map_err(|e| caused("managed-input-inventory-workspace", e))?;
    let mut result = BTreeMap::new();
    for (name, hash) in expected {
        if !Path::new(name).starts_with(prefix) || Path::new(name).components().any(|p| !matches!(p,std::path::Component::Normal(_))) {
            return Err(E::Refused("managed-input-inventory-outside-workspace"));
        }
        let (actual_path, actual) = crate::gate_evaluation::artifact_hash(root, name)
            .map_err(|e| caused("managed-input-inventory-read", e))?;
        if !actual_path.starts_with(workspace) {
            return Err(E::Refused("managed-input-inventory-outside-workspace"));
        }
        if &actual != hash {
            return Err(E::Refused("managed-input-inventory-changed"));
        }
        result.insert(name.clone(), actual);
    }
    Ok(result)
}
