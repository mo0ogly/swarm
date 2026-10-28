//! Linux regular artifacts opened beneath an owned directory, with checked closes.
//! A held descriptor is not a later lease, Root identity or OS confinement.
use crate::LocalPathError;
use std::{
    error::Error,
    fmt,
    fs::File,
    io,
    os::fd::IntoRawFd,
    path::{Path, PathBuf},
};
/// Internal access capability, not an operator limit or permission grant.
#[derive(Clone, Copy)]
pub(crate) enum Access {
    Contents,
    Metadata,
}
/// Preserve original path/reader/OS failures and every independent owned close.
/// Display is a static safe family; private paths and reader content stay hidden.
pub(crate) enum AnchoredArtifactError {
    Path(LocalPathError),
    Cause {
        code: &'static str,
        cause: Box<dyn Error + Send + Sync>,
    },
    Refused(&'static str),
    Cleanup {
        primary: Option<Box<Self>>,
        code: &'static str,
        cause: io::Error,
    },
}
impl AnchoredArtifactError {
    /// Static failure family; no retry, permission or state-change inference.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Path(_) => "anchored-artifact-path",
            Self::Cause { code, .. } | Self::Cleanup { code, .. } | Self::Refused(code) => code,
        }
    }
    /// Borrow an original callback failure through owned cleanup wrappers.
    /// This is an error projection only, not general trait-object reflection.
    pub(crate) fn reader_cause(&self) -> Option<&(dyn Error + Send + Sync + 'static)> {
        match self {
            Self::Cause {
                code: "anchored-artifact-reader",
                cause,
            } => Some(cause.as_ref()),
            Self::Cleanup {
                primary: Some(p), ..
            } => p.reader_cause(),
            _ => None,
        }
    }
    /// Earliest independent close error, for legacy single-cleanup consumers.
    /// Full traversal must also inspect source and this node's direct cleanup.
    pub(crate) fn first_cleanup(&self) -> Option<&io::Error> {
        match self {
            Self::Cleanup { primary, cause, .. } => primary
                .as_ref()
                .and_then(|p| p.first_cleanup())
                .or(Some(cause)),
            _ => None,
        }
    }
    /// This node's owned close failure only; source retains any earlier failure.
    pub(crate) fn direct_cleanup(&self) -> Option<&io::Error> {
        match self {
            Self::Cleanup { cause, .. } => Some(cause),
            _ => None,
        }
    }
    /// Original primary branch, excluding a direct cleanup already projected.
    pub(crate) fn primary(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Path(e) => Some(e),
            Self::Cause { cause, .. } => Some(cause.as_ref()),
            Self::Cleanup {
                primary: Some(p), ..
            } => Some(p.as_ref()),
            _ => None,
        }
    }
}
impl fmt::Display for AnchoredArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for AnchoredArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for AnchoredArtifactError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.primary()
            .or_else(|| self.direct_cleanup().map(|e| e as &(dyn Error + 'static)))
    }
}
/// Preserve callback/earlier-close errors when another owned descriptor closes.
fn close<T>(
    result: Result<T, AnchoredArtifactError>,
    fd: impl IntoRawFd,
    code: &'static str,
) -> Result<T, AnchoredArtifactError> {
    match (result, nix::unistd::close(fd).map_err(io::Error::from)) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(cause)) => Err(AnchoredArtifactError::Cleanup {
            primary: None,
            code,
            cause,
        }),
        (Err(primary), Err(cause)) => Err(AnchoredArtifactError::Cleanup {
            primary: Some(Box::new(primary)),
            code,
            cause,
        }),
    }
}
/// Resolve contained lexical symlinks, then open the canonical relative path
/// atomically beneath an owned Root-directory descriptor without new symlinks.
/// A pre-open observer supports controlled race regression; production readers
/// supply a no-op. Metadata uses O_PATH and adds no content-read requirement.
/// Validate regular type and explicitly close file then directory, preserving
/// original and both cleanup causes. No retry or weaker unsupported-kernel
/// fallback. Pathnames remain observations; no immutable content/Root authority,
/// hardlink provenance, later-action lease or non-Linux safety is established.
pub(crate) fn with<T, E: Error + Send + Sync + 'static>(
    root: &Path,
    name: &str,
    access: Access,
    before_open: impl FnOnce(&Path),
    read: impl FnOnce(&mut File) -> Result<T, E>,
) -> Result<(PathBuf, T), AnchoredArtifactError> {
    use rustix::fs::{open, openat2, Mode, OFlags, ResolveFlags};
    let base = crate::canonical_project_root(root).map_err(AnchoredArtifactError::Path)?;
    let path = crate::local_file(&base, name).map_err(AnchoredArtifactError::Path)?;
    let relative = path
        .strip_prefix(&base)
        .map_err(|_| AnchoredArtifactError::Refused("anchored-artifact-root-changed"))?;
    before_open(&path);
    let directory = open(
        &base,
        OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    )
    .map_err(|cause| AnchoredArtifactError::Cause {
        code: "anchored-artifact-root-open",
        cause: Box::new(io::Error::from(cause)),
    })?;
    let result = (|| {
        let flags = match access {
            Access::Contents => OFlags::RDONLY | OFlags::NONBLOCK,
            Access::Metadata => OFlags::PATH,
        } | OFlags::CLOEXEC
            | OFlags::NOFOLLOW;
        let mut file = File::from(
            openat2(
                &directory,
                relative,
                flags,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
            )
            .map_err(|cause| AnchoredArtifactError::Cause {
                code: "anchored-artifact-open",
                cause: Box::new(io::Error::from(cause)),
            })?,
        );
        let observed = (|| {
            if !file
                .metadata()
                .map_err(|cause| AnchoredArtifactError::Cause {
                    code: "anchored-artifact-metadata",
                    cause: Box::new(cause),
                })?
                .is_file()
            {
                return Err(AnchoredArtifactError::Refused(
                    "anchored-artifact-regular-file-required",
                ));
            }
            read(&mut file).map_err(|cause| AnchoredArtifactError::Cause {
                code: "anchored-artifact-reader",
                cause: Box::new(cause),
            })
        })();
        close(observed, file, "anchored-artifact-file-close")
    })();
    close(result, directory, "anchored-artifact-root-close").map(|value| (path, value))
}
