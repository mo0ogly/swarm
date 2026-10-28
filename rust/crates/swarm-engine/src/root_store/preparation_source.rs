//! Private static method sources: no user-selected path, provider or writes.
use super::{finished, preparation_read::cause, RootStoreError as E};
use rustix::fs::{self, Mode, OFlags};
use std::{fs::File, io::Read, os::fd::IntoRawFd, path::Path};

/// Explicitly close the owned descriptor; retain the original typed close cause.
fn close(file: File) -> Result<(), E> {
    nix::unistd::close(file.into_raw_fd()).map_err(|e| cause("preparation-method-close", e))
}
/// Validate complete method text, never a truncated substitute. The byte cap is
/// persisted operator policy; UTF-8/nonempty/no-NUL are the method text format.
fn text(bytes: Vec<u8>, limit: Option<usize>) -> Result<Vec<u8>, E> {
    if bytes.is_empty() || bytes.contains(&0) { return Err(E::Refused("preparation-method-text-invalid")); }
    if limit.is_some_and(|limit| bytes.len() > limit) { return Err(E::Refused("preparation-method-source-over-limit")); }
    std::str::from_utf8(&bytes).map_err(|e| cause("preparation-method-utf8", e))?;
    Ok(bytes)
}
/// Copy bundled fallback only after an actual missing component and successful
/// parent close. Allocation/text failures propagate; no project error is hidden.
fn bundled(bytes: &[u8], limit: Option<usize>) -> Result<Vec<u8>, E> {
    if limit.is_some_and(|limit| bytes.len() > limit) { return Err(E::Refused("preparation-method-source-over-limit")); }
    let mut out = Vec::new();
    out.try_reserve(bytes.len()).map_err(|e| cause("preparation-method-allocation", e))?;
    out.extend_from_slice(bytes); text(out, limit)
}
/// Read one internal catalogue path descriptor by descriptor, without following
/// child symlinks or blocking on a substituted device/FIFO. Only ENOENT selects
/// shipped bytes; unsafe/inaccessible files never select fallback. Require a
/// regular leaf and complete UTF-8 text under the supplied saved cap (None is
/// unlimited). Retain primary and every explicit close cause; no state changes,
/// source authentication, future freshness lease or execution authority follows.
pub(super) fn read_source(root: &Path, path: &str, fallback: &[u8], limit: Option<usize>) -> Result<Vec<u8>, E> {
    if path.is_empty() || path.starts_with('/') || path.contains(['\\', '\0'])
        || path.split('/').any(|part| matches!(part, "" | "." | "..")) {
        return Err(E::Refused("preparation-method-path-invalid"));
    }
    let mut current: File = fs::open(root, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC, Mode::empty())
        .map_err(|e| cause("preparation-method-root", e))?.into();
    let mut parts = path.split('/').peekable();
    while let Some(part) = parts.next() {
        let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
        if parts.peek().is_some() { flags |= OFlags::DIRECTORY; }
        match fs::openat(&current, part, flags, Mode::empty()) {
            Err(rustix::io::Errno::NOENT) => { close(current)?; return bundled(fallback, limit); }
            Err(e) => return finished(Err(cause("preparation-method-open", e)), close(current)),
            Ok(next) => {
                let child: File = next.into();
                if let Err(primary) = close(current) { return finished(Err(primary), close(child)); }
                current = child;
            }
        }
    }
    let result = (|| {
        if !current.metadata().map_err(|e| cause("preparation-method-metadata", e))?.is_file() {
            return Err(E::Refused("preparation-method-not-regular"));
        }
        let mut out = Vec::new(); let mut buffer = [0u8; 8192]; // IO chunk, not an operator limit.
        loop {
            let request = limit.map_or(buffer.len(), |cap| cap.saturating_sub(out.len()).saturating_add(1).min(buffer.len()));
            let n = match current.read(&mut buffer[..request]) {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(cause("preparation-method-read", e)),
            };
            if n == 0 { break; }
            out.try_reserve(n).map_err(|e| cause("preparation-method-allocation", e))?;
            out.extend_from_slice(&buffer[..n]);
            if limit.is_some_and(|cap| out.len() > cap) { return Err(E::Refused("preparation-method-source-over-limit")); }
        }
        text(out, limit)
    })();
    finished(result, close(current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::symlink, path::PathBuf};
    struct Fixture(PathBuf);
    impl Fixture {
        /// Own an isolated filesystem only; preserve it on failure for diagnosis.
        fn new() -> Self {
            let path = std::env::temp_dir().join(crate::generate_identifier("preparation-source-").unwrap());
            fs::create_dir(&path).unwrap(); Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { if !std::thread::panicking() { fs::remove_dir_all(&self.0).unwrap(); } }
    }
    /// Actual complete source precedes cap/text/symlink refusals; restoring the
    /// owned source restores the read. Unsafe local files never use shipped data.
    #[test]
    fn preparation_source_local_refusal_and_recovery() {
        let root = Fixture::new(); let path = root.0.join("method.md");
        fs::write(&path, b"actual").unwrap();
        assert_eq!(read_source(&root.0, "method.md", b"bundled", Some(6)).unwrap(), b"actual");
        assert!(read_source(&root.0, "method.md", b"bundle", Some(5)).is_err());
        fs::write(&path, b"a\0b").unwrap(); assert!(read_source(&root.0, "method.md", b"bundle", None).is_err());
        fs::write(&path, [0xff]).unwrap(); assert!(read_source(&root.0, "method.md", b"bundle", None).is_err());
        fs::write(&path, b"actual").unwrap(); assert_eq!(read_source(&root.0, "method.md", b"bundle", None).unwrap(), b"actual");
        fs::remove_file(&path).unwrap(); symlink("missing-target", &path).unwrap();
        assert!(read_source(&root.0, "method.md", b"bundle", None).is_err());
        fs::remove_file(&path).unwrap(); fs::write(&path, b"restored").unwrap();
        assert_eq!(read_source(&root.0, "method.md", b"bundle", None).unwrap(), b"restored");
    }
    /// Missing local component permits actual packaged bytes, with the same
    /// policy and text checks; a valid parent directory precedes symlink refusal.
    #[test]
    fn preparation_source_missing_only_fallback_and_parent_binding() {
        let root = Fixture::new();
        assert_eq!(read_source(&root.0, "missing/method.md", b"bundle", Some(6)).unwrap(), b"bundle");
        assert!(read_source(&root.0, "missing.md", b"bundle", Some(5)).is_err());
        assert!(read_source(&root.0, "missing.md", b"", None).is_err());
        assert!(read_source(&root.0, "../method.md", b"bundle", None).is_err());
        let directory = root.0.join("local"); fs::create_dir(&directory).unwrap(); fs::write(directory.join("method.md"), b"valid").unwrap();
        assert_eq!(read_source(&root.0, "local/method.md", b"bundle", None).unwrap(), b"valid");
        fs::rename(&directory, root.0.join("real")).unwrap(); symlink("real", &directory).unwrap();
        assert!(read_source(&root.0, "local/method.md", b"bundle", None).is_err());
        fs::remove_file(&directory).unwrap(); fs::rename(root.0.join("real"), &directory).unwrap();
        assert_eq!(read_source(&root.0, "local/method.md", b"bundle", None).unwrap(), b"valid");
    }
}
