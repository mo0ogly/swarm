//! Descriptor-rooted documentary file discovery; observations are not leases.
use super::{
    finished, preparation_capture as capture, preparation_read::cause, RootStoreError as E,
};
use crate::PreparationSourcePolicy;
use rustix::fs::{self, AtFlags, FileType, RawDir};
use serde::Serialize;
use std::{fs::File, mem::MaybeUninit};

/// Go-compatible bounded discovery. `limited` reports an operator listing/visit
/// limit, not a proof of complete filesystem coverage. No file contents or
/// provider state is read. Concurrent tree changes may refuse the observation.
#[derive(Serialize)]
pub struct PreparationSourceFiles {
    pub directories: Vec<String>,
    pub paths: Vec<String>,
    pub limited: bool,
}
struct Entry {
    name: String,
    kind: FileType,
}
/// Join a discovered basename to the root sentinel or a relative scope; this
/// produces display/query data, never a filesystem open or permission decision.
fn child_path(parent: &str, name: &str) -> String {
    if parent == "." {
        name.into()
    } else {
        format!("{parent}/{name}")
    }
}
/// Enumerate one borrowed descriptor in lexical order without an implicitly
/// owned ReadDir handle. The getdents buffer grows only after documented EINVAL
/// (record too large), retaining progress on the same FD. Its initial8192 bytes
/// are an IO chunk, not a path/entry/operator ceiling. Allocation, UTF8, stat and
/// read errors propagate; the caller explicitly closes the owned directory.
fn entries(
    directory: &File,
    scope: &str,
    policy: &PreparationSourcePolicy,
) -> Result<Vec<Entry>, E> {
    let mut bytes: Vec<MaybeUninit<u8>> = Vec::new();
    bytes
        .try_reserve(8192)
        .map_err(|e| cause("preparation-source-allocation", e))?;
    bytes.resize_with(8192, MaybeUninit::uninit);
    let mut out = Vec::new();
    loop {
        let mut grow = false;
        let mut iter = RawDir::new(directory, &mut bytes);
        while let Some(entry) = iter.next() {
            let entry = match entry {
                Ok(e) => e,
                Err(rustix::io::Errno::INTR) => continue,
                Err(rustix::io::Errno::INVAL) => {
                    grow = true;
                    break;
                }
                Err(e) => return Err(cause("preparation-source-directory-read", e)),
            };
            let name = entry
                .file_name()
                .to_str()
                .map_err(|e| cause("preparation-source-name-utf8", e))?;
            if matches!(name, "." | "..") {
                continue;
            }
            let kind = entry.file_type();
            let kind = if kind == FileType::Unknown
                && capture::allowed_path(&child_path(scope, name), policy)
            {
                let stat = fs::statat(directory, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(|e| cause("preparation-source-entry-stat", e))?;
                FileType::from_raw_mode(stat.st_mode)
            } else {
                kind
            };
            out.try_reserve(1)
                .map_err(|e| cause("preparation-source-allocation", e))?;
            out.push(Entry {
                name: name.into(),
                kind,
            });
        }
        if !grow {
            break;
        }
        let new = bytes
            .len()
            .checked_mul(2)
            .ok_or(E::Refused("preparation-source-buffer-overflow"))?;
        bytes
            .try_reserve(new - bytes.len())
            .map_err(|e| cause("preparation-source-allocation", e))?;
        bytes.resize_with(new, MaybeUninit::uninit);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}
/// List safe source paths under an already bound project descriptor. All caps
/// and generated-name/extension filters come from saved policy. Scope empty
/// means root sentinel `.` as in Go; no path is followed through a symlink.
/// Lexical enumeration precedes visit limits, so these are neither memory nor
/// time bounds. IO failures refuse rather than returning an empty success.
/// No document, visit, reservation, provider or filesystem mutation follows.
pub(super) fn list(
    project: &File,
    scope: &str,
    query: &str,
    policy: &PreparationSourcePolicy,
) -> Result<PreparationSourceFiles, E> {
    if policy.query_max_bytes.is_some_and(|n| query.len() > n) {
        return Err(E::Refused("preparation-source-query-over-limit"));
    }
    let scope = if scope.is_empty() { "." } else { scope };
    let directory = capture::directory(project, scope, policy)?;
    let first = finished(
        entries(&directory, scope, policy),
        capture::close(directory),
    )?;
    let mut out = PreparationSourceFiles {
        directories: vec![],
        paths: vec![],
        limited: false,
    };
    for entry in &first {
        let path = child_path(scope, &entry.name);
        if entry.kind == FileType::Directory && capture::allowed_path(&path, policy) {
            if policy
                .directory_limit
                .is_some_and(|n| out.directories.len() >= n)
            {
                out.limited = true;
                break;
            }
            out.directories
                .try_reserve(1)
                .map_err(|e| cause("preparation-source-allocation", e))?;
            out.directories.push(path);
        }
    }
    let query = crate::text::scalar_lower(query);
    let mut visited = 0usize;
    // Stack carries names only; each directory descriptor is explicitly closed
    // before descendants are examined, including all refusal/limit exits.
    let mut stack = vec![(scope.to_owned(), FileType::Directory, Some(first))];
    while let Some((path, kind, cached)) = stack.pop() {
        visited = visited
            .checked_add(1)
            .ok_or(E::Refused("preparation-source-visit-overflow"))?;
        if policy.scan_limit.is_some_and(|n| visited > n) {
            out.limited = true;
            break;
        }
        if path != "." && !capture::allowed_path(&path, policy) {
            continue;
        }
        if kind == FileType::Directory {
            let children = if let Some(entries) = cached {
                entries
            } else {
                let directory = capture::directory(project, &path, policy)?;
                finished(
                    entries(&directory, &path, policy),
                    capture::close(directory),
                )?
            };
            stack
                .try_reserve(children.len())
                .map_err(|e| cause("preparation-source-allocation", e))?;
            for child in children.into_iter().rev() {
                stack.push((child_path(&path, &child.name), child.kind, None));
            }
        } else if kind == FileType::RegularFile
            && capture::allowed_extension(&path, policy)
            && crate::text::scalar_lower(&path).contains(&query)
        {
            if policy.file_limit.is_some_and(|n| out.paths.len() >= n) {
                out.limited = true;
                break;
            }
            out.paths
                .try_reserve(1)
                .map_err(|e| cause("preparation-source-allocation", e))?;
            out.paths.push(path);
        }
    }
    Ok(out)
}
