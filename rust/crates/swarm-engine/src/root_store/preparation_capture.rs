//! Preparation source capture: descriptor-verified reads of reviewed document
//! files for a preparation's evidence set.
//!
//! Protocol invariants identified here (security protocol; policy cannot
//! weaken them):
//!
//! * Every access goes through descriptors opened `O_NOFOLLOW`; a name is
//!   never trusted after a check — the opened descriptor itself is verified.
//! * `project` proves the opened root is the connected instance by fstat
//!   dev/ino of its .swarm child descriptor and of an `O_PATH` witness of this
//!   project's `.swarm/state.db`, the witness being reached by `openat`
//!   through the project descriptor (its child), never by path again. Both
//!   auxiliary descriptors are closed explicitly; the external root guards
//!   are held by the caller.
//! * The digest is SHA-256 over the complete file bytes and the secret scan
//!   runs over the complete file, never only over the requested lines; the
//!   excerpt end is clipped to the file (Go parity).
//! * Private-path refusals are protocol: relative slash paths without empty,
//!   `.` or `..` components; no dot-prefixed component except the protocol
//!   exceptions `.claude` and `.agents`; no backslash, NUL, CR or LF; no
//!   component containing `credential`/`secret` or starting with
//!   `id_rsa`/`id_ed25519`. These component rules subsume Go's
//!   `filepath.Clean(path) == path` check for relative paths.
//! * Extensions (`allowed_extensions`) and generated-directory names
//!   (`excluded_components`) are operator policy lists, not constants.
//! * Every numeric cap comes from `PreparationSourcePolicy`; `None` is
//!   explicitly unlimited and `Some(0)` blocks nonempty data. No hidden
//!   12 000 / 200 / 256 Kio constants exist in this module.
//! * Aggregate caps (`total_max_bytes`, `max_count`) and enumeration caps
//!   (`directory_limit`, `file_limit`, `scan_limit`, `query_max_bytes`)
//!   belong to the calling assembly/enumeration functions, not here.

use super::{finished, preparation_read::cause, RootInstanceIdentity, RootStoreError as E};
use crate::{PreparationSource, PreparationSourcePolicy};
use regex::Regex;
use sha2::{Digest, Sha256};

/// Operator reference only; text/time are captured by the engine, never supplied.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(super) struct SourceRef {
    pub path: String,
    #[serde(skip_serializing_if = "zero")]
    pub start_line: i64,
    #[serde(skip_serializing_if = "zero")]
    pub end_line: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sha256: String,
}
fn zero(n: &i64) -> bool {
    *n == 0
}
/// Decode the exact reference fields with the shared Go wire projection.
/// No filesystem access, captured content or authority follows from decoding.
pub(super) fn reference(raw: &serde_json::Value) -> Result<SourceRef, E> {
    let bytes = serde_json::to_vec(raw).map_err(|e| cause("preparation-request-invalid", e))?;
    crate::work::decode_mode(
        &bytes,
        crate::preparation_models::shape_preparation_reference(),
        true,
    )
    .map_err(|e| cause("preparation-request-invalid", e))
}

use rustix::fs::{self, Mode, OFlags};
use std::{fs::File, io::Read, os::fd::IntoRawFd, os::unix::fs::MetadataExt, path::Path};

/// Explicitly close an owned descriptor; the original typed close cause is
/// retained so joined cleanups never hide the primary failure.
pub(super) fn close(file: File) -> Result<(), E> {
    nix::unistd::close(file.into_raw_fd()).map_err(|e| cause("preparation-source-close", e))
}

/// Open and prove the connected project root.
///
/// `root` itself is opened `O_NOFOLLOW` (a symlinked root is refused), then
/// `.swarm` is opened by `openat` through that descriptor, then an `O_PATH`
/// witness for `state.db` through `.swarm`. The .swarm descriptor and the
/// witness are fstat'ed and compared with `expected` (device, inode); the
/// witness and `.swarm` descriptors are closed explicitly with causes joined
/// through `finished`. On success the project descriptor is returned open.
/// No root mutation, provider call or timeout happens here.
pub(super) fn project(root: &Path, expected: &RootInstanceIdentity) -> Result<File, E> {
    let project_dir: File = fs::open(
        root,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    )
    .map_err(|e| cause("preparation-project-root", e))?
    .into();
    let verified = (|| -> Result<(), E> {
        let swarm: File = fs::openat(
            &project_dir,
            ".swarm",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|e| cause("preparation-project-swarm", e))?
        .into();
        let witness_check = (|| -> Result<(), E> {
            // expected.directory is the connected .swarm directory, NOT the
            // project root. Compare its actual child descriptor before the DB.
            let dir_stat = swarm
                .metadata()
                .map_err(|e| cause("preparation-project-fstat", e))?;
            if (dir_stat.dev(), dir_stat.ino()) != expected.directory {
                return Err(E::Refused("preparation-project-directory-identity"));
            }
            // O_PATH witness: no read/write rights, no symlink following.
            let witness: File = fs::openat(
                &swarm,
                "state.db",
                OFlags::PATH | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::empty(),
            )
            .map_err(|e| cause("preparation-project-witness", e))?
            .into();
            let compared = (|| -> Result<(), E> {
                let db_stat = witness
                    .metadata()
                    .map_err(|e| cause("preparation-project-fstat", e))?;
                if (db_stat.dev(), db_stat.ino()) != expected.database {
                    return Err(E::Refused("preparation-project-database-identity"));
                }
                Ok(())
            })();
            finished(compared, close(witness))
        })();
        finished(witness_check, close(swarm))
    })();
    match verified {
        Ok(()) => Ok(project_dir),
        Err(e) => finished(Err(e), close(project_dir)),
    }
}

/// Open the listing scope: `.` (empty string means `.`) or a safe relative
/// directory path admitted by `allowed_path`. Every component is opened
/// `O_DIRECTORY|O_NOFOLLOW|O_NONBLOCK`, so no symlink parent, symlink scope
/// or substituted FIFO can be opened; the returned descriptor is the scope
/// itself, independent of later renames of the project path.
pub(super) fn directory(
    project: &File,
    path: &str,
    policy: &PreparationSourcePolicy,
) -> Result<File, E> {
    let scope = if path.is_empty() { "." } else { path };
    if scope != "." && !allowed_path(scope, policy) {
        return Err(E::Refused("preparation-source-scope-refused"));
    }
    let mut owned: Option<File> = None;
    let opened = (|| -> Result<File, E> {
        for part in scope.split('/') {
            // A directory scope is a directory at every component.
            let base: &File = owned.as_ref().unwrap_or(project);
            let next: File = fs::openat(
                base,
                part,
                OFlags::RDONLY
                    | OFlags::DIRECTORY
                    | OFlags::NOFOLLOW
                    | OFlags::NONBLOCK
                    | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| open_error("preparation-source-scope-open", e))?
            .into();
            if let Some(previous) = owned.take() {
                if let Err(close_error) = close(previous) {
                    return finished(Err(close_error), close(next));
                }
            }
            owned = Some(next);
        }
        // A validated scope has at least one component.
        owned
            .take()
            .ok_or(E::Refused("preparation-source-scope-empty"))
    })();
    match opened {
        Ok(file) => Ok(file),
        Err(e) => finished(Err(e), close_owned(owned)),
    }
}

/// Whether a document path is inside the permitted perimeter at all.
///
/// Protocol rules (non-overridable): relative, slash-separated, no empty,
/// `.` or `..` component, no dot-prefixed component except `.claude` and
/// `.agents`, no backslash/NUL/CR/LF, and no component that contains
/// `credential`/`secret` or starts with `id_rsa`/`id_ed25519`.
/// Policy rules: byte length under `path_max_bytes` (`None` unlimited) and
/// exclusion by the operator's `excluded_components` list (compared
/// case-insensitively, like Go's lowercase literals).
pub(super) fn allowed_path(path: &str, policy: &PreparationSourcePolicy) -> bool {
    if path.is_empty() {
        return false;
    }
    if let Some(max) = policy.path_max_bytes {
        if path.len() > max {
            return false;
        }
    }
    if path.starts_with('/') {
        return false;
    }
    if path.contains(['\\', '\0', '\n', '\r']) {
        return false;
    }
    for part in path.split('/') {
        match part {
            "" | "." | ".." => return false,
            ".claude" | ".agents" => {}
            _ if part.starts_with('.') => return false,
            _ => {}
        }
        let lowered = crate::text::scalar_lower(part);
        if policy
            .excluded_components
            .iter()
            .any(|name| crate::text::scalar_lower(name) == lowered)
        {
            return false;
        }
        if matches!(
            lowered.as_str(),
            "providers.json" | "settings.json" | "settings.local.json"
        ) || lowered.contains("credential")
            || lowered.contains("secret")
        {
            return false;
        }
        if lowered.starts_with("id_rsa") || lowered.starts_with("id_ed25519") {
            return false;
        }
    }
    true
}

/// Operator extension admission (Go `preparationSourceType` with the literal
/// switch replaced by `allowed_extensions`). The extension is the final
/// component's suffix from its last dot, lowercased; entries are compared
/// case-insensitively and are dot-prefixed (".md", not "md").
pub(super) fn allowed_extension(path: &str, policy: &PreparationSourcePolicy) -> bool {
    let base = match path.rsplit_once('/') {
        Some((_, base)) => base,
        None => path,
    };
    // Go's filepath.Ext: suffix from the last dot of the final component.
    let extension = match base.rfind('.') {
        Some(dot) => &base[dot..],
        None => "",
    };
    let lowered = crate::text::scalar_lower(extension);
    policy
        .allowed_extensions
        .iter()
        .any(|allowed| crate::text::scalar_lower(allowed) == lowered)
}

/// Capture one reviewed excerpt.
///
/// Validates the 1-based inclusive range (span capped by `max_lines`,
/// checked before any IO), `allowed_path` and the operator extension list,
/// opens the file component by component without following symlinks,
/// requires a regular leaf within `file_max_bytes`, reads it whole, refuses
/// non-UTF-8, NUL or secret-shaped content (scan over the complete file),
/// then slices the requested lines with `end_line` clipped to the file and
/// `text` capped by `excerpt_max_bytes`. `sha256` covers the complete file
/// bytes; `at` is the caller-supplied capture timestamp. No state changes
/// and no execution authority follow. Concurrent file edits can produce a mixed
/// byte observation: the digest describes bytes read, not an atomic filesystem
/// snapshot or future lease. The writer reopens and compares the reviewed hash.
pub(super) fn excerpt(
    project: &File,
    reference: &SourceRef,
    policy: &PreparationSourcePolicy,
    at: &str,
) -> Result<PreparationSource, E> {
    if reference.start_line < 1 || reference.end_line < reference.start_line {
        return Err(E::Refused("preparation-source-range"));
    }
    if let Some(max_lines) = policy.max_lines {
        let span = usize::try_from(reference.end_line - reference.start_line)
            .map_err(|e| cause("preparation-source-range", e))?;
        if span >= max_lines {
            return Err(E::Refused("preparation-source-range"));
        }
    }
    if !allowed_path(&reference.path, policy) {
        return Err(E::Refused("preparation-source-path-refused"));
    }
    if !allowed_extension(&reference.path, policy) {
        return Err(E::Refused("preparation-source-type-refused"));
    }
    let mut owned: Option<File> = None;
    let opened = (|| -> Result<File, E> {
        let mut parts = reference.path.split('/').peekable();
        while let Some(part) = parts.next() {
            // NONBLOCK protects against a swapped FIFO/device; NOFOLLOW
            // against a swapped symlink, at every component.
            let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
            if parts.peek().is_some() {
                flags |= OFlags::DIRECTORY;
            }
            let base: &File = owned.as_ref().unwrap_or(project);
            let next: File = fs::openat(base, part, flags, Mode::empty())
                .map_err(|e| open_error("preparation-source-open", e))?
                .into();
            if let Some(previous) = owned.take() {
                if let Err(close_error) = close(previous) {
                    return finished(Err(close_error), close(next));
                }
            }
            owned = Some(next);
        }
        owned
            .take()
            .ok_or(E::Refused("preparation-source-path-empty"))
    })();
    let file = match opened {
        Ok(file) => file,
        Err(e) => return finished(Err(e), close_owned(owned)),
    };
    let captured = (|| -> Result<PreparationSource, E> {
        // Verify the opened descriptor: regular leaf, policy size cap.
        let stat = file
            .metadata()
            .map_err(|e| cause("preparation-source-fstat", e))?;
        if !stat.is_file() {
            return Err(E::Refused("preparation-source-not-regular"));
        }
        if let Some(cap) = policy.file_max_bytes {
            if stat.len() > cap as u64 {
                return Err(E::Refused("preparation-source-file-over-limit"));
            }
        }
        let bytes = read_capped(&file, policy.file_max_bytes)?;
        let complete =
            std::str::from_utf8(&bytes).map_err(|e| cause("preparation-source-utf8", e))?;
        if bytes.contains(&0) {
            return Err(E::Refused("preparation-source-binary"));
        }
        // Secret scan over the complete file, not the excerpt (Go parity).
        if secret_detector()?.is_match(complete) {
            return Err(E::Refused("preparation-source-secret"));
        }
        let line_count = i64::try_from(complete.split('\n').count())
            .map_err(|e| cause("preparation-source-range", e))?;
        let start = reference.start_line;
        if start > line_count {
            return Err(E::Refused("preparation-source-start-beyond-file"));
        }
        let end = reference.end_line.min(line_count);
        let skip = usize::try_from(start - 1).map_err(|e| cause("preparation-source-range", e))?;
        let take =
            usize::try_from(end - start + 1).map_err(|e| cause("preparation-source-range", e))?;
        let mut text = String::new();
        for (index, line) in complete.split('\n').skip(skip).take(take).enumerate() {
            let needed = line
                .len()
                .checked_add(usize::from(index > 0))
                .and_then(|n| text.len().checked_add(n))
                .ok_or(E::Refused("preparation-source-size-overflow"))?;
            if policy.excerpt_max_bytes.is_some_and(|cap| needed > cap) {
                return Err(E::Refused("preparation-source-excerpt-over-limit"));
            }
            text.try_reserve(needed - text.len())
                .map_err(|e| cause("preparation-source-allocation", e))?;
            if index > 0 {
                text.push('\n');
            }
            text.push_str(line);
        }
        Ok(PreparationSource {
            path: reference.path.clone(),
            start_line: start,
            end_line: end,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            text,
            captured_at: at.to_string(),
        })
    })();
    finished(captured, close(file))
}

/// Close a descriptor still held after a failed walk, keeping its cause.
fn close_owned(owned: Option<File>) -> Result<(), E> {
    match owned {
        Some(file) => close(file),
        None => Ok(()),
    }
}

/// Read the descriptor to completion, at most `cap + 1` bytes when capped
/// (`Some(0)` therefore refuses any nonempty file), growing only through
/// `try_reserve` so allocation failure surfaces as a cause, never a panic.
fn read_capped(mut file: &File, cap: Option<usize>) -> Result<Vec<u8>, E> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192]; // IO chunk, not an operator limit.
    loop {
        let request = cap.map_or(buffer.len(), |cap| {
            cap.saturating_sub(bytes.len())
                .saturating_add(1)
                .min(buffer.len())
        });
        let n = match file.read(&mut buffer[..request]) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(cause("preparation-source-read", e)),
        };
        if n == 0 {
            return Ok(bytes);
        }
        bytes
            .try_reserve(n)
            .map_err(|e| cause("preparation-source-allocation", e))?;
        bytes.extend_from_slice(&buffer[..n]);
        if cap.is_some_and(|cap| bytes.len() > cap) {
            return Err(E::Refused("preparation-source-file-over-limit"));
        }
    }
}

/// Secret detector pattern: protocol constant, not operator policy. Go's
/// ASCII `\s` and ASCII `\b` are reproduced exactly with `(?-u:...)`; since
/// the regex crate rejects a negated `(?-u:...)` class over `&str` (it could
/// match invalid UTF-8), Go's `[^\s"'{}]` is spelled as the scalar
/// complement of exactly Go's `\s` set (0x09, 0x0A, 0x0C, 0x0D, 0x20) plus
/// `"`/`'`/`{`/`}`, so `{6,}` keeps counting UTF-8 scalars like Go's runes.
const SECRET_PATTERN: &str = r#"(?i)(-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:api[_-]?key|password|passwd|access[_-]?token|client[_-]?secret)["']?(?-u:\s*)[:=](?-u:\s*)["']?[^\x09\x0A\x0C\x0D\x20"'{}]{6,}|(?-u:\b)(?:sk-[A-Za-z0-9_-]{16,}|AKIA[A-Z0-9]{16})(?-u:\b))"#;

/// Compile the detector fallibly; the regex cause is preserved, no unwrap.
fn secret_detector() -> Result<Regex, E> {
    Regex::new(SECRET_PATTERN).map_err(|e| cause("preparation-source-secret-regex", e))
}

/// Classify only explicit path selection failures as caller refusals while
/// preserving their errno; resource/IO failures keep a server diagnostic family.
fn open_error(context: &'static str, error: rustix::io::Errno) -> E {
    let code = if matches!(
        error,
        rustix::io::Errno::NOENT
            | rustix::io::Errno::LOOP
            | rustix::io::Errno::NOTDIR
            | rustix::io::Errno::ACCESS
    ) {
        "preparation-source-path-refused"
    } else {
        context
    };
    cause(code, error)
}
