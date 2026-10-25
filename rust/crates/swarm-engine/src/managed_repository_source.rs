//! Raw tracked-source witnesses independent of mutable Git index flags.
use super::{finish, git, hash, LocalDirectory, E};
use crate::EngineSettingsDocument;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Component, Path},
};

/// Owned source bytes and executable mode at a particular tree entry. This is
/// an observation, not execution authority or protection from same-UID writers.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct TrackedSource {
    mode: String,
    blob: String,
    sha256: String,
    executable: bool,
}

/// Walk normal tree path components with owned no-follow directory descriptors.
/// Ordinary source parents may be 0755; no permissions are changed. Explicit
/// closure preserves the original failure together with every close failure.
fn at_parent<T>(
    copy: &LocalDirectory,
    path: &str,
    read: impl FnOnce(&Path) -> Result<T, E>,
) -> Result<T, E> {
    let mut parts = Path::new(path).components().collect::<Vec<_>>();
    if parts.is_empty()
        || parts
            .iter()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(E::Refused("managed-source-path-invalid"));
    }
    let leaf = parts.pop().unwrap().as_os_str();
    let mut parents = Vec::new();
    let outcome = (|| {
        for part in parts {
            let parent = parents.last().unwrap_or(copy);
            let next = LocalDirectory::open_owned(&parent.process_path().join(part.as_os_str()))
                .map_err(|e| E::caused("managed-source-parent", e))?;
            parents.push(next);
        }
        read(&parents.last().unwrap_or(copy).process_path().join(leaf))
    })();
    parents.into_iter().rev().fold(outcome, |result, parent| {
        finish(
            result,
            parent
                .close()
                .map_err(|e| E::caused("managed-source-parent-close", e)),
        )
    })
}

/// Hash a regular owned source without following a leaf symlink or blocking on
/// a FIFO. Streaming has no content policy cap; 8192 is an IO buffer only. Check
/// metadata before/after and retain read plus close causes. Concurrent writers
/// remain forbidden by the caller's completed-producer/control lifecycle.
fn regular(path: &Path) -> Result<(String, bool), E> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK | nix::libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| E::caused("managed-source-open", e))?;
    let outcome = (|| {
        let before = file
            .metadata()
            .map_err(|e| E::caused("managed-source-metadata", e))?;
        if !before.is_file() || before.uid() != nix::unistd::Uid::current().as_raw() {
            return Err(E::Refused("managed-source-not-owned-regular"));
        }
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|e| E::caused("managed-source-read", e))?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        let after = file
            .metadata()
            .map_err(|e| E::caused("managed-source-metadata", e))?;
        let stamp = |m: &std::fs::Metadata| {
            (
                m.len(),
                m.mtime(),
                m.mtime_nsec(),
                m.ctime(),
                m.ctime_nsec(),
            )
        };
        if stamp(&before) != stamp(&after) {
            return Err(E::Refused("managed-source-changed-during-read"));
        }
        Ok((
            format!("{:x}", digest.finalize()),
            before.mode() & 0o100 != 0,
        ))
    })();
    use std::os::fd::IntoRawFd;
    finish(
        outcome,
        nix::unistd::close(file.into_raw_fd()).map_err(|e| E::caused("managed-source-close", e)),
    )
}

/// Enumerate the immutable commit tree and read every tracked raw file/symlink.
/// Git index assume-unchanged/skip-worktree and conversion drivers cannot hide
/// byte changes. Symlink target bytes are hashed without following their target.
/// Gitlinks and non-UTF8 paths explicitly refuse until custody is implemented;
/// no entry is silently skipped. Errors retain causes, without writes or grants.
pub(super) fn tracked_sources(
    copy: &LocalDirectory,
    commit: &str,
    settings: &EngineSettingsDocument,
) -> Result<BTreeMap<String, TrackedSource>, E> {
    let raw = git(
        &copy.process_path(),
        &["ls-tree", "-r", "-z", "--full-tree", commit, "--"],
        settings,
    )?;
    let mut result = BTreeMap::new();
    for entry in raw.split(|b| *b == 0).filter(|entry| !entry.is_empty()) {
        let tab = entry
            .iter()
            .position(|b| *b == b'\t')
            .ok_or(E::Refused("managed-source-tree-format"))?;
        let header = std::str::from_utf8(&entry[..tab])
            .map_err(|e| E::caused("managed-source-tree-encoding", e))?;
        let fields = header.split(' ').collect::<Vec<_>>();
        if fields.len() != 3
            || !matches!(fields[2].len(), 40 | 64)
            || !fields[2]
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(E::Refused("managed-source-tree-format"));
        }
        let path = std::str::from_utf8(&entry[tab + 1..])
            .map_err(|e| E::caused("managed-source-path-encoding", e))?;
        let (sha256, executable) = at_parent(copy, path, |leaf| match (fields[0], fields[1]) {
            ("100644" | "100755", "blob") => regular(leaf),
            ("120000", "blob") => {
                let target =
                    std::fs::read_link(leaf).map_err(|e| E::caused("managed-source-symlink", e))?;
                Ok((hash(target.as_os_str().as_encoded_bytes()), false))
            }
            _ => Err(E::Refused("managed-source-tree-mode-unported")),
        })?;
        if result
            .insert(
                path.into(),
                TrackedSource {
                    mode: fields[0].into(),
                    blob: fields[2].into(),
                    sha256,
                    executable,
                },
            )
            .is_some()
        {
            return Err(E::Refused("managed-source-tree-duplicate"));
        }
    }
    Ok(result)
}
