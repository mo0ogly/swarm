//! Physical identity of an already connected SQLite main database on Linux.
//!
//! This module requires the bundled, unmodified `sqlite_filestat` builtin and a
//! qualified Unix VFS. It opens a *new O_PATH witness*, through the process's
//! trusted procfs FD link, while the real connection's query is still borrowed.
//! It never owns, duplicates or closes SQLite's ordinary data descriptor. The
//! witness pins an inode; it establishes no permission, provenance of a caller's
//! JSON, OS confinement, or protection against hostile same-process FD changes.
#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]

use rusqlite::Connection;
use rustix::fs::{self, Mode, OFlags};
use std::{error::Error, fmt, fs::File, io, os::unix::fs::MetadataExt};

/// Stage at which observation or explicit resource cleanup failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConnectedDatabaseIdentityOperation {
    Prepare,
    Query,
    Diagnostic,
    OpenWitness,
    InspectWitness,
    Finalize,
    #[cfg(test)]
    Duplicate,
    Close,
}

/// Failed observation with its original cause and any secondary cleanup error.
/// No error authorizes pathname fallback or proves that a surrounding SQL write
/// did not occur. This component itself issues only its fixed diagnostic SELECT.
pub(crate) struct ConnectedDatabaseIdentityError {
    pub(crate) operation: ConnectedDatabaseIdentityOperation,
    cause: Box<dyn Error + Send + Sync>,
    pub(crate) cleanup: Option<Box<ConnectedDatabaseIdentityError>>,
}

impl ConnectedDatabaseIdentityError {
    /// Preserve a concrete error without including the private diagnostic JSON.
    fn new(
        operation: ConnectedDatabaseIdentityOperation,
        cause: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            operation,
            cause: Box::new(cause),
            cleanup: None,
        }
    }

    /// Retain successive cleanup failures instead of replacing the first cause.
    fn append_cleanup(&mut self, error: Self) {
        let mut slot = &mut self.cleanup;
        while let Some(existing) = slot {
            slot = &mut existing.cleanup;
        }
        *slot = Some(Box::new(error));
    }
}

impl fmt::Display for ConnectedDatabaseIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "connected-database-identity: {:?}", self.operation)
    }
}
impl fmt::Debug for ConnectedDatabaseIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for ConnectedDatabaseIdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

/// Owned inode witness captured from the real connected main database.
///
/// The cached tuple came from this O_PATH descriptor's metadata. Keep the witness
/// alive for as long as that identity backs retained project authority; a copied
/// tuple alone does not prevent inode reuse. Root's shared claim/BeforeGrant
/// checks must compare it to the intended instance before granting execution.
pub(crate) struct ConnectedDatabaseIdentity {
    witness: File,
    identity: (u64, u64),
}

impl fmt::Debug for ConnectedDatabaseIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectedDatabaseIdentity")
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

/// Validate the builtin's main-file diagnostic, never a caller-supplied handle.
/// Unknown additional diagnostic fields are irrelevant. `db.h` must fit a Linux
/// FD and `db.vfs` must be present; VFS implementation qualification belongs to
/// the linked SQLite build, not to an invented name allowlist here.
fn main_descriptor(raw: &str) -> Result<i32, ConnectedDatabaseIdentityError> {
    use ConnectedDatabaseIdentityOperation::Diagnostic;
    let document: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| ConnectedDatabaseIdentityError::new(Diagnostic, error))?;
    let main = document.get("db").and_then(serde_json::Value::as_object);
    let descriptor = main
        .and_then(|main| main.get("h"))
        .and_then(serde_json::Value::as_i64)
        .and_then(|descriptor| i32::try_from(descriptor).ok())
        .filter(|descriptor| *descriptor >= 0);
    let vfs = main
        .and_then(|main| main.get("vfs"))
        .and_then(serde_json::Value::as_str)
        .filter(|vfs| !vfs.is_empty());
    match (descriptor, vfs) {
        (Some(descriptor), Some(_)) => Ok(descriptor),
        _ => Err(ConnectedDatabaseIdentityError::new(
            Diagnostic,
            io::Error::new(
                io::ErrorKind::InvalidData,
                "connected main-file diagnostic required",
            ),
        )),
    }
}

/// Close only our O_PATH file, preserving a possible POSIX close error. Never
/// retry close: a released descriptor number could already have been reused.
fn close_witness(file: File) -> Result<(), ConnectedDatabaseIdentityError> {
    nix::unistd::close(file).map_err(|error| {
        ConnectedDatabaseIdentityError::new(ConnectedDatabaseIdentityOperation::Close, error)
    })
}

/// Preserve a failed metadata check and an independent witness-close failure.
fn reject_witness(
    mut error: ConnectedDatabaseIdentityError,
    witness: File,
) -> ConnectedDatabaseIdentityError {
    if let Err(cleanup) = close_witness(witness) {
        error.append_cleanup(cleanup);
    }
    error
}

impl ConnectedDatabaseIdentity {
    /// Observe the actual connected main-file inode and retain an O_PATH witness.
    ///
    /// Requires trusted procfs, the genuine builtin enabled by
    /// SQLITE_ENABLE_FILESTAT, a qualified Unix VFS, and no reentrant SQL,
    /// function replacement or raw FD manipulation during this synchronous call.
    /// Root owns the connection; arbitrary third-party connections are outside
    /// this contract. A named persistent main file must already be open. Missing
    /// builtin, null/malformed diagnostics, invalid descriptors, procfs errors,
    /// non-private objects and SQL finalization failures all refuse explicitly.
    /// There is no fallback to the database pathname. Both the statement and any
    /// rejected witness are explicitly closed, with secondary errors retained.
    pub(crate) fn capture(connection: &Connection) -> Result<Self, ConnectedDatabaseIdentityError> {
        use ConnectedDatabaseIdentityOperation::{Finalize, Prepare, Query};
        let mut statement = connection
            .prepare("SELECT sqlite_filestat('main')")
            .map_err(|error| ConnectedDatabaseIdentityError::new(Prepare, error))?;
        let result = statement
            .query_row([], |row| {
                let raw: String = row.get(0)?;
                Ok(Self::from_diagnostic(&raw))
            })
            .map_err(|error| ConnectedDatabaseIdentityError::new(Query, error))
            .and_then(|result| result);
        let finalized = statement
            .finalize()
            .map_err(|error| ConnectedDatabaseIdentityError::new(Finalize, error));
        match (result, finalized) {
            (Ok(identity), Ok(())) => Ok(identity),
            (Err(error), Ok(())) => Err(error),
            (Err(mut error), Err(cleanup)) => {
                error.append_cleanup(cleanup);
                Err(error)
            }
            (Ok(identity), Err(mut error)) => {
                if let Err(cleanup) = identity.close() {
                    error.append_cleanup(cleanup);
                }
                Err(error)
            }
        }
    }

    /// Open the procfs magic link of the genuine builtin's live FD while the
    /// query closure retains the connection. O_PATH intentionally follows that
    /// link; this never changes SQLite's own SQLITE_OPEN_NOFOLLOW flags.
    fn from_diagnostic(raw: &str) -> Result<Self, ConnectedDatabaseIdentityError> {
        use ConnectedDatabaseIdentityOperation::{InspectWitness, OpenWitness};
        let descriptor = main_descriptor(raw)?;
        let witness: File = fs::open(
            format!("/proc/self/fd/{descriptor}"),
            OFlags::PATH | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| ConnectedDatabaseIdentityError::new(OpenWitness, error))?
        .into();
        let metadata = match witness.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                return Err(reject_witness(
                    ConnectedDatabaseIdentityError::new(InspectWitness, error),
                    witness,
                ))
            }
        };
        // Same owner/private/regular/nlink contract as LocalDirectory::open_file.
        if !metadata.is_file()
            || metadata.uid() != nix::unistd::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
            || metadata.nlink() != 1
        {
            return Err(reject_witness(
                ConnectedDatabaseIdentityError::new(
                    InspectWitness,
                    io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "owned private regular main file required",
                    ),
                ),
                witness,
            ));
        }
        Ok(Self {
            identity: (metadata.dev(), metadata.ino()),
            witness,
        })
    }

    /// Borrow only our O_PATH witness for private gate transfer; never SQLite's
    /// ordinary data FD. The binding owner retains it through send completion.
    #[cfg(test)]
    pub(crate) fn gate_descriptor(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.witness.as_fd()
    }

    /// Adopt a private gate's received witness before any child SQLite open.
    /// Require O_PATH/CLOEXEC and the existing private regular single-link owner
    /// contract. No data FD is opened or duplicated; rejected received descriptors
    /// are closed explicitly with both causes retained. Sender/permission checks
    /// remain mandatory outside this metadata-only primitive.
    #[cfg(test)]
    pub(crate) fn adopt_gate_descriptor(fd: std::os::fd::OwnedFd) -> Result<Self, ConnectedDatabaseIdentityError> {
        use ConnectedDatabaseIdentityOperation::InspectWitness;
        let witness: File = fd.into();
        let result = (|| {
            let flags = fs::fcntl_getfl(&witness)
                .map_err(|e| ConnectedDatabaseIdentityError::new(InspectWitness, e))?;
            let descriptor_flags = rustix::io::fcntl_getfd(&witness)
                .map_err(|e| ConnectedDatabaseIdentityError::new(InspectWitness, e))?;
            let metadata = witness.metadata()
                .map_err(|e| ConnectedDatabaseIdentityError::new(InspectWitness, e))?;
            if !flags.contains(OFlags::PATH)
                || !descriptor_flags.contains(rustix::io::FdFlags::CLOEXEC)
                || !metadata.is_file() || metadata.uid() != nix::unistd::geteuid().as_raw()
                || metadata.mode() & 0o077 != 0 || metadata.nlink() != 1
            {
                return Err(ConnectedDatabaseIdentityError::new(InspectWitness,
                    io::Error::new(io::ErrorKind::PermissionDenied, "private gate O_PATH main-file witness required")));
            }
            Ok((metadata.dev(), metadata.ino()))
        })();
        match result {
            Ok(identity) => Ok(Self { identity, witness }),
            Err(error) => Err(reject_witness(error, witness)),
        }
    }

    /// Return the originally observed identity; the owned witness keeps its inode
    /// pinned even if the database is later renamed or unlinked. This is no grant.
    pub(crate) fn identity(&self) -> (u64, u64) {
        self.identity
    }

    /// Duplicate our O_PATH witness for another owner with the same lifetime
    /// obligation. This does not duplicate SQLite's ordinary data FD. Closing a
    /// duplicate is still a FMODE_PATH close on the qualified Linux contract.
    #[cfg(test)]
    pub(crate) fn duplicate(&self) -> Result<Self, ConnectedDatabaseIdentityError> {
        let witness = self.witness.try_clone().map_err(|error| {
            ConnectedDatabaseIdentityError::new(
                ConnectedDatabaseIdentityOperation::Duplicate,
                error,
            )
        })?;
        Ok(Self {
            witness,
            identity: self.identity,
        })
    }

    /// Explicitly release this O_PATH owner and report close failure. Dropping
    /// without calling close still closes via File, but cannot report that error.
    pub(crate) fn close(self) -> Result<(), ConnectedDatabaseIdentityError> {
        close_witness(self.witness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs as disk,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
        path::PathBuf,
    };

    /// Fresh private on-disk SQLite fixture. Failed tests retain files for diagnosis.
    struct Fixture(PathBuf);
    impl Fixture {
        /// Create a private owned test directory without touching a live Root.
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(crate::generate_identifier("sqlite-inode-").unwrap());
            disk::create_dir(&path).unwrap();
            disk::set_permissions(&path, disk::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
        /// Create a named private database and populate ordinary fixture data.
        /// Existing files are never overwritten or reused.
        fn open(&self, name: &str) -> Connection {
            let path = self.0.join(name);
            let file = disk::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .unwrap();
            nix::unistd::close(file).unwrap();
            let connection = Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .unwrap();
            connection.execute_batch("CREATE TABLE fixture_value(value TEXT NOT NULL); INSERT INTO fixture_value VALUES('original');").unwrap();
            connection
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if !std::thread::panicking() {
                disk::remove_dir_all(&self.0).unwrap();
            }
        }
    }

    #[test]
    fn connected_database_diagnostic_refuses_missing_main_wrong_types_and_fd_overflow() {
        for raw in [
            "{}",
            "null",
            "{",
            r#"{"db":null}"#,
            r#"{"db":{"h":-1,"vfs":"unix"}}"#,
            r#"{"db":{"h":2147483648,"vfs":"unix"}}"#,
            r#"{"db":{"h":"3","vfs":"unix"}}"#,
            r#"{"db":{"h":3,"vfs":""}}"#,
            r#"{"journal":{"h":3,"vfs":"unix"}}"#,
        ] {
            assert_eq!(
                main_descriptor(raw).unwrap_err().operation,
                ConnectedDatabaseIdentityOperation::Diagnostic
            );
        }
        assert_eq!(
            main_descriptor(r#"{"db":{"h":3,"vfs":"unix","inode":{"nRef":2}},"journal":{"h":7}}"#)
                .unwrap(),
            3
        );
    }

    #[test]
    fn connected_database_capture_pins_actual_file_across_path_replacement_and_duplication() {
        let fixture = Fixture::new();
        let connection = fixture.open("state.db");
        let original = disk::metadata(fixture.0.join("state.db")).unwrap();
        let witness = ConnectedDatabaseIdentity::capture(&connection).unwrap();
        assert_eq!(witness.identity(), (original.dev(), original.ino()));
        disk::rename(fixture.0.join("state.db"), fixture.0.join("original.db")).unwrap();
        let replacement = fixture.open("state.db");
        let replaced = ConnectedDatabaseIdentity::capture(&replacement).unwrap();
        assert_ne!(replaced.identity(), witness.identity());
        let still_original = ConnectedDatabaseIdentity::capture(&connection).unwrap();
        assert_eq!(still_original.identity(), witness.identity());
        let duplicate = witness.duplicate().unwrap();
        witness.close().unwrap();
        connection.close().unwrap();
        disk::remove_file(fixture.0.join("original.db")).unwrap();
        assert_eq!(duplicate.identity(), still_original.identity());
        assert_eq!(duplicate.witness.metadata().unwrap().ino(), original.ino());
        duplicate.close().unwrap();
        still_original.close().unwrap();
        replaced.close().unwrap();
        replacement.close().unwrap();
    }

    #[test]
    fn connected_database_capture_refuses_unopened_memory_main_and_query_failure() {
        let fixture = Fixture::new();
        let connection = fixture.open("state.db");
        ConnectedDatabaseIdentity::capture(&connection)
            .unwrap()
            .close()
            .unwrap();
        let memory = Connection::open_in_memory().unwrap();
        assert_eq!(
            ConnectedDatabaseIdentity::capture(&memory)
                .unwrap_err()
                .operation,
            ConnectedDatabaseIdentityOperation::Diagnostic
        );
        memory.close().unwrap();
        // This test-only replacement proves query-error preservation, not the
        // no-FILESTAT compile profile. It never fabricates a successful identity.
        connection
            .create_scalar_function(
                "sqlite_filestat",
                1,
                rusqlite::functions::FunctionFlags::SQLITE_UTF8,
                |_| -> rusqlite::Result<String> { Err(rusqlite::Error::InvalidQuery) },
            )
            .unwrap();
        let error = ConnectedDatabaseIdentity::capture(&connection).unwrap_err();
        assert_eq!(error.operation, ConnectedDatabaseIdentityOperation::Query);
        assert!(error.source().is_some());
        connection.close().unwrap();
    }

    /// Run the same bundled SQLite test executable in an independent process.
    /// The parent's transaction is already established, so exit status is a
    /// deterministic handshake; no timeout, polling or delay substitutes for it.
    fn assert_lock_in_child(path: &std::path::Path, expected: &str) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "connected_database_identity::tests::connected_database_lock_child_process",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("SWARM_SQLITE_IDENTITY_LOCK_CHILD_PATH", path)
            .env("SWARM_SQLITE_IDENTITY_LOCK_CHILD_EXPECTED", expected)
            .output()
            .unwrap();
        print!("{}", String::from_utf8_lossy(&output.stdout));
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(
            output.status.success(),
            "independent lock child failed: status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains(&format!("OBSERVED independent SQLite lock {expected}")),
            "child must exercise and report the real BEGIN IMMEDIATE result: {stdout}"
        );
    }

    /// Subprocess-only entry: zero busy wait is a discriminating test observation,
    /// not a persisted retry policy. The parent owns the tested transaction.
    #[test]
    #[ignore = "invoked only by the independent-process lock regression"]
    fn connected_database_lock_child_process() {
        let path = std::env::var_os("SWARM_SQLITE_IDENTITY_LOCK_CHILD_PATH").unwrap();
        let expected = std::env::var("SWARM_SQLITE_IDENTITY_LOCK_CHILD_EXPECTED").unwrap();
        let connection = Connection::open_with_flags(
            PathBuf::from(path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .unwrap();
        connection.busy_timeout(std::time::Duration::ZERO).unwrap();
        let observed = connection.execute_batch("BEGIN IMMEDIATE");
        match expected.as_str() {
            "busy" => match observed {
                Err(rusqlite::Error::SqliteFailure(error, _)) => {
                    assert_eq!(error.code, rusqlite::ErrorCode::DatabaseBusy);
                }
                other => panic!("expected independent BUSY, observed {other:?}"),
            },
            "available" => {
                observed.unwrap();
                connection.execute_batch("ROLLBACK").unwrap();
            }
            _ => panic!("unknown lock assertion mode"),
        }
        connection.close().unwrap();
        println!("OBSERVED independent SQLite lock {expected}");
    }

    #[test]
    fn connected_database_opath_capture_duplicate_close_preserve_cross_process_sqlite_lock() {
        let fixture = Fixture::new();
        let connection = fixture.open("state.db");
        connection.execute_batch("BEGIN IMMEDIATE").unwrap();
        let path = fixture.0.join("state.db");
        // Valid control before introducing witness opens/closes.
        assert_lock_in_child(&path, "busy");
        let witness = ConnectedDatabaseIdentity::capture(&connection).unwrap();
        let duplicate = witness.duplicate().unwrap();
        witness.close().unwrap();
        duplicate.close().unwrap();
        assert_lock_in_child(&path, "busy");
        connection.execute_batch("ROLLBACK").unwrap();
        assert_lock_in_child(&path, "available");
        connection.close().unwrap();
    }

    #[test]
    fn connected_database_capture_refuses_public_or_hardlinked_files_and_recovers() {
        let fixture = Fixture::new();
        let connection = fixture.open("state.db");
        let control = ConnectedDatabaseIdentity::capture(&connection).unwrap();
        control.close().unwrap();
        let path = fixture.0.join("state.db");
        disk::set_permissions(&path, disk::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            ConnectedDatabaseIdentity::capture(&connection)
                .unwrap_err()
                .operation,
            ConnectedDatabaseIdentityOperation::InspectWitness
        );
        disk::set_permissions(&path, disk::Permissions::from_mode(0o600)).unwrap();
        disk::hard_link(&path, fixture.0.join("extra-link")).unwrap();
        assert_eq!(
            ConnectedDatabaseIdentity::capture(&connection)
                .unwrap_err()
                .operation,
            ConnectedDatabaseIdentityOperation::InspectWitness
        );
        disk::remove_file(fixture.0.join("extra-link")).unwrap();
        ConnectedDatabaseIdentity::capture(&connection)
            .unwrap()
            .close()
            .unwrap();
        connection.close().unwrap();
    }
}
