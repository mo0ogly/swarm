//! Snapshot staging must not execute repository-configured filter programs.
use super::{git, E};
use crate::EngineSettingsDocument;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Stage the owned worker copy and return its raw Git tree object-id output
/// without clean/smudge/process callbacks, including racy-index rehashing. This is
/// the engine's snapshot-operation invariant, not an operator execution policy
/// or permission to run a repository helper. Ordinary Git text normalization
/// remains; committed report bytes must still match the observed original.
///
/// Read only configuration names first, override each actual filter in argv
/// without editing configuration, then verify effective empty commands/false
/// required flags before add and write-tree. Unrepresentable config keys refuse before staging;
/// roundtrip verification prevents quoting/assignment ambiguity from restoring a
/// callback. Source values remain private and never enter a diagnostic string.
/// Caller owns the copy/Git descriptors and current attribution. Git uses the
/// saved per-command timeout; no shell, retry, model or new policy cap is added.
/// Errors retain original causes; a failed operation may have changed the index and
/// never proves rollback, candidate publication or acceptance.
pub(super) fn snapshot_tree(copy: &Path, settings: &EngineSettingsDocument) -> Result<Vec<u8>, E> {
    let names = git(
        copy,
        &["config", "--null", "--name-only", "--list"],
        settings,
    )?;
    let names = std::str::from_utf8(&names)
        .map_err(|e| E::caused("managed-git-config-name-encoding", e))?;
    let mut filters = BTreeSet::new();
    for key in names.split('\0').filter(|key| key.starts_with("filter.")) {
        if let Some((prefix, field)) = key.rsplit_once('.') {
            if matches!(field, "clean" | "smudge" | "process" | "required") {
                if prefix.contains(['=', '\n', '\r']) {
                    return Err(E::Refused("managed-git-filter-key-unrepresentable"));
                }
                filters.insert(prefix.to_owned());
            }
        }
    }
    let mut args = Vec::new();
    for prefix in &filters {
        for (field, value) in [
            ("clean", ""),
            ("smudge", ""),
            ("process", ""),
            ("required", "false"),
        ] {
            args.push("-c".to_owned());
            args.push(format!("{prefix}.{field}={value}"));
        }
    }
    if !filters.is_empty() {
        let mut inspect = args.iter().map(String::as_str).collect::<Vec<_>>();
        inspect.extend(["config", "--null", "--list"]);
        let raw = git(copy, &inspect, settings)?;
        let mut effective = BTreeMap::new();
        for entry in raw
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
        {
            let split = entry.iter().position(|byte| *byte == b'\n');
            let (key, value) = match split {
                Some(index) => (&entry[..index], &entry[index + 1..]),
                None => (entry, &[][..]),
            };
            effective.insert(key.to_vec(), value.to_vec());
        }
        for prefix in &filters {
            for (field, expected) in [
                ("clean", b"".as_slice()),
                ("smudge", b"".as_slice()),
                ("process", b"".as_slice()),
                ("required", b"false".as_slice()),
            ] {
                let key = format!("{prefix}.{field}");
                if effective.get(key.as_bytes()).map(Vec::as_slice) != Some(expected) {
                    return Err(E::Refused("managed-git-filter-override-unverified"));
                }
            }
        }
    }
    let mut stage = args.iter().map(String::as_str).collect::<Vec<_>>();
    stage.extend(["add", "-A", "--", "."]);
    git(copy, &stage, settings)?;
    // write-tree can rehash racily-clean entries and execute clean filters too.
    // Retain the same effective overrides through that complete index operation.
    let mut tree = args.iter().map(String::as_str).collect::<Vec<_>>();
    tree.push("write-tree");
    git(copy, &tree, settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Owned Git fixture and callback program outside the indexed repository.
    /// No native producer/permission is fabricated by this data-operation test.
    fn fixture(
        name: &str,
    ) -> (
        std::path::PathBuf,
        std::path::PathBuf,
        EngineSettingsDocument,
    ) {
        let root = std::env::temp_dir().join(crate::generate_identifier("managed-index-").unwrap());
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        println!("INDEX_FIXTURE {}", root.display());
        let repo = root.join("source");
        std::fs::create_dir(&repo).unwrap();
        let settings = EngineSettingsDocument::initial(
            crate::EngineSettingsValues::installation_defaults().unwrap(),
            "owned-fixture".into(),
            "2026-10-25T00:00:00Z".into(),
        )
        .unwrap();
        git(&repo, &["init", "--quiet"], &settings).unwrap();
        std::fs::write(repo.join("tracked.txt"), b"original raw source\n").unwrap();
        std::fs::write(
            repo.join(".gitattributes"),
            format!("tracked.txt filter={name}\n"),
        )
        .unwrap();
        snapshot_tree(&repo, &settings).unwrap();
        assert_eq!(
            git(&repo, &["show", ":tracked.txt"], &settings).unwrap(),
            b"original raw source\n"
        );
        (root, repo, settings)
    }

    #[test]
    fn managed_git_snapshot_neutralizes_clean_and_process_callbacks_without_config_writes() {
        for (name, field) in [
            ("fixture", "clean"),
            ("fixture", "process"),
            ("fixture.with.dots", "clean"),
        ] {
            let (root, repo, settings) = fixture(name);
            let program = root.join("callback.py");
            let marker = root.join("called");
            std::fs::write(&program,format!("#!/usr/bin/env python3\nimport sys\nfrom pathlib import Path\nPath({:?}).write_text('unwanted callback')\nif sys.argv[1]=='clean': sys.stdout.buffer.write(sys.stdin.buffer.read())\nelse: print('invalid filter protocol')\n",marker.to_str().unwrap())).unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
            let witness = std::process::Command::new(&program)
                .arg("process")
                .output()
                .unwrap();
            assert!(witness.status.success());
            assert!(marker.exists());
            std::fs::remove_file(&marker).unwrap();
            let command = format!("{} {field}", program.display());
            let key = format!("filter.{name}.{field}");
            git(&repo, &["config", &key, &command], &settings).unwrap();
            git(
                &repo,
                &["config", &format!("filter.{name}.required"), "true"],
                &settings,
            )
            .unwrap();
            let before = git(&repo, &["config", "--null", "--local", "--list"], &settings).unwrap();
            std::fs::write(repo.join("tracked.txt"), b"new raw source\n").unwrap();
            let tree = super::super::text(snapshot_tree(&repo, &settings).unwrap()).unwrap();
            assert!(!marker.exists());
            assert_eq!(
                git(&repo, &["show", &format!("{tree}:tracked.txt")], &settings).unwrap(),
                b"new raw source\n"
            );
            assert_eq!(
                git(&repo, &["show", ":tracked.txt"], &settings).unwrap(),
                b"new raw source\n"
            );
            assert_eq!(
                git(&repo, &["config", "--null", "--local", "--list"], &settings).unwrap(),
                before
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn managed_git_snapshot_unrepresentable_filter_key_refuses_before_index_change() {
        let (root, repo, settings) = fixture("danger=key");
        git(
            &repo,
            &["config", "filter.danger=key.clean", "must-not-execute"],
            &settings,
        )
        .unwrap();
        std::fs::write(repo.join("tracked.txt"), b"must remain unstaged\n").unwrap();
        assert_eq!(
            snapshot_tree(&repo, &settings).unwrap_err().code(),
            "managed-git-filter-key-unrepresentable"
        );
        assert_eq!(
            git(&repo, &["show", ":tracked.txt"], &settings).unwrap(),
            b"original raw source\n"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
