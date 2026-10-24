//! Filesystem observations matching Go path contracts. A resolved path is no
//! read permit, retained file handle or protection against later substitution.
use crate::{GitState, Request, Work, WorkDefinitionError};
use std::{
    error::Error,
    fmt, io,
    path::{Component, Path, PathBuf},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathOperation {
    WorkingDirectory,
    CanonicalRoot,
    ResolveFile,
    FileMetadata,
    ResolveOwned,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathRefusal {
    RelativeRequired,
    OutsideProject,
    RegularFileRequired,
    DirectoryRequired,
    AbsoluteNormalizedRequired,
    RedirectedOwnedPath,
}
pub enum LocalPathError {
    Refused(PathRefusal),
    Io {
        operation: PathOperation,
        cause: io::Error,
    },
}
impl LocalPathError {
    pub fn code(&self) -> &'static str {
        "path"
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Refused(PathRefusal::RelativeRequired) => "relative-file-required",
            Self::Refused(PathRefusal::OutsideProject) => "path-outside-project",
            Self::Refused(PathRefusal::RegularFileRequired) => "regular-file-required",
            Self::Refused(PathRefusal::DirectoryRequired) => "directory-required",
            Self::Refused(PathRefusal::AbsoluteNormalizedRequired) => {
                "absolute-normalized-path-required"
            }
            Self::Refused(PathRefusal::RedirectedOwnedPath) => "owned-child-redirected",
            Self::Io { operation, .. } => match operation {
                PathOperation::WorkingDirectory => "working-directory-unavailable",
                PathOperation::CanonicalRoot => "project-root-unresolved",
                PathOperation::ResolveFile => "file-unresolved",
                PathOperation::FileMetadata => "file-metadata-unavailable",
                PathOperation::ResolveOwned => "owned-path-unresolved",
            },
        }
    }
}
impl fmt::Display for LocalPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for LocalPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for LocalPathError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
fn io_error(operation: PathOperation, cause: io::Error) -> LocalPathError {
    LocalPathError::Io { operation, cause }
}
/// filepath.Clean on an absolute path, before resolving symlinks (Linux qualified).
fn clean_absolute(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}
/// Go filepath.EvalSymlinks' technical traversal bound, not an operator policy.
pub const GO_SYMLINK_TRAVERSAL_LIMIT: usize = 255;
#[cfg(unix)]
fn resolve_symlinks(path: &Path) -> io::Result<PathBuf> {
    use std::{
        collections::VecDeque,
        ffi::OsString,
        os::unix::ffi::{OsStrExt, OsStringExt},
    };
    let mut pending: VecDeque<Vec<u8>> = path
        .as_os_str()
        .as_bytes()
        .split(|b| *b == b'/')
        .map(<[u8]>::to_vec)
        .collect();
    let mut dest = PathBuf::from("/");
    let mut links = 0usize;
    while let Some(part) = pending.pop_front() {
        if part.is_empty() || part == b"." {
            continue;
        }
        if part == b".." {
            dest.pop();
            if dest.as_os_str().is_empty() {
                dest.push("/");
            }
            continue;
        }
        dest.push(OsString::from_vec(part));
        let metadata = std::fs::symlink_metadata(&dest)?;
        if !metadata.file_type().is_symlink() {
            if !metadata.is_dir() && !pending.is_empty() {
                return Err(io::Error::from(io::ErrorKind::NotADirectory));
            }
            continue;
        }
        links += 1;
        if links > GO_SYMLINK_TRAVERSAL_LIMIT {
            return Err(io::Error::other("Go EvalSymlinks traversal limit exceeded"));
        }
        let target = std::fs::read_link(&dest)?;
        dest.pop();
        let bytes = target.as_os_str().as_bytes();
        if bytes.starts_with(b"/") {
            dest = PathBuf::from("/");
        }
        for component in bytes.split(|b| *b == b'/').rev() {
            pending.push_front(component.to_vec());
        }
    }
    Ok(dest)
}
#[cfg(not(unix))]
fn resolve_symlinks(path: &Path) -> io::Result<PathBuf> {
    std::fs::canonicalize(path)
}
pub fn canonical_project_root(root: &Path) -> Result<PathBuf, LocalPathError> {
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| io_error(PathOperation::WorkingDirectory, e))?
            .join(root)
    };
    resolve_symlinks(&clean_absolute(&absolute))
        .map_err(|e| io_error(PathOperation::CanonicalRoot, e))
}
pub fn local_file(root: &Path, name: &str) -> Result<PathBuf, LocalPathError> {
    if !crate::text::nonempty(name) || Path::new(name).is_absolute() {
        return Err(LocalPathError::Refused(PathRefusal::RelativeRequired));
    }
    let root = canonical_project_root(root)?;
    let resolved = resolve_symlinks(&clean_absolute(&root.join(name)))
        .map_err(|e| io_error(PathOperation::ResolveFile, e))?;
    if !resolved.starts_with(&root) {
        return Err(LocalPathError::Refused(PathRefusal::OutsideProject));
    }
    if !std::fs::metadata(&resolved)
        .map_err(|e| io_error(PathOperation::FileMetadata, e))?
        .is_file()
    {
        return Err(LocalPathError::Refused(PathRefusal::RegularFileRequired));
    }
    Ok(resolved)
}
/// Go lexical cleanup before physical resolution; observation, not confinement.
pub fn local_directory(root: &Path, name: &str) -> Result<PathBuf, LocalPathError> {
    if Path::new(name).is_absolute() {
        return Err(LocalPathError::Refused(PathRefusal::RelativeRequired));
    }
    let root = canonical_project_root(root)?;
    let resolved = resolve_symlinks(&clean_absolute(&root.join(name)))
        .map_err(|e| io_error(PathOperation::ResolveFile, e))?;
    if !resolved.starts_with(&root) {
        return Err(LocalPathError::Refused(PathRefusal::OutsideProject));
    }
    if !std::fs::metadata(&resolved)
        .map_err(|e| io_error(PathOperation::FileMetadata, e))?
        .is_dir()
    {
        return Err(LocalPathError::Refused(PathRefusal::DirectoryRequired));
    }
    Ok(resolved)
}
pub fn canonical_owned_project_path(root: &Path, path: &Path) -> Result<PathBuf, LocalPathError> {
    let canonical = canonical_project_root(root)?;
    if !path.is_absolute() || clean_absolute(path).as_os_str() != path.as_os_str() {
        return Err(LocalPathError::Refused(
            PathRefusal::AbsoluteNormalizedRequired,
        ));
    }
    let ancestors: Vec<_> = path.ancestors().collect();
    for prefix in ancestors.into_iter().rev() {
        if resolve_symlinks(prefix).is_ok_and(|p| p == canonical) {
            let relative = path
                .strip_prefix(prefix)
                .map_err(|_| LocalPathError::Refused(PathRefusal::OutsideProject))?;
            let actual =
                resolve_symlinks(path).map_err(|e| io_error(PathOperation::ResolveOwned, e))?;
            if actual != canonical.join(relative) {
                return Err(LocalPathError::Refused(PathRefusal::RedirectedOwnedPath));
            }
            return Ok(actual);
        }
    }
    Err(LocalPathError::Refused(PathRefusal::OutsideProject))
}
pub enum CheckpointError {
    Definition(WorkDefinitionError),
    Summary,
    Path(LocalPathError),
}
impl CheckpointError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Definition(e) => e.code(),
            Self::Path(_) | Self::Summary => "format",
        }
    }
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Definition(e) => e.reason(),
            Self::Summary => "checkpoint-summary-required",
            Self::Path(e) => e.reason(),
        }
    }
}
impl fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}
impl fmt::Debug for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for CheckpointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Definition(e) => Some(e),
            Self::Path(e) => Some(e),
            Self::Summary => None,
        }
    }
}
/// Observe every requested file before changing Work. Git is caller-supplied.
pub fn apply_checkpoint(
    w: &mut Work,
    r: &Request,
    root: &Path,
    git: GitState,
) -> Result<(), CheckpointError> {
    crate::validate_work_request("checkpoint", r).map_err(CheckpointError::Definition)?;
    if !crate::text::nonempty(&r.summary) {
        return Err(CheckpointError::Summary);
    }
    for file in r.memory.as_deref().unwrap_or_default() {
        local_file(root, file).map_err(CheckpointError::Path)?;
    }
    w.summary = r.summary.clone();
    w.next = r.next.clone();
    w.memory = r.memory.clone();
    w.git = git;
    Ok(())
}
