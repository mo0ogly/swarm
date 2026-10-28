//! Owned local directory operations anchored to an open descriptor. This does
//! not protect against same-UID attackers, root substitution before opening or
//! whole-backup rollback. External identity/witness/confinement remains separate.
#![cfg(unix)]
use crate::{generate_identifier, RuntimeError};
use rustix::fs::{self as fs, AtFlags, FlockOperation, Mode, OFlags};
use std::{
    error::Error,
    fmt,
    fs::File,
    io::{self, Read, Write},
    os::unix::fs::MetadataExt,
    path::Path,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationState {
    NotPublished,
    RenamedDurabilityUnknown,
}
/// Filesystem operation semantics chosen by the caller's create/replace API,
/// not a persisted execution policy, retry or permission.
#[derive(Clone, Copy)]
enum PublicationMode {
    Replace,
    #[cfg(target_os = "linux")]
    CreateNew,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicStorageFault {
    None,
    #[cfg(feature = "test-fixtures")]
    PartialWriteNoSpace,
    #[cfg(feature = "test-fixtures")]
    SyncFile,
    #[cfg(feature = "test-fixtures")]
    CloseFile,
    #[cfg(feature = "test-fixtures")]
    Rename,
    #[cfg(feature = "test-fixtures")]
    SyncDirectory,
    #[cfg(feature = "test-fixtures")]
    SyncAndCleanup,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalStorageOperation {
    OpenDirectory,
    DuplicateDirectory,
    OpenFile,
    Metadata,
    #[cfg(target_os = "linux")]
    Permissions,
    Read,
    CreateTemporary,
    CreateDirectory,
    Write,
    SyncFile,
    CloseFile,
    Rename,
    SyncDirectory,
    Lock,
    Unlock,
    CloseLock,
    RemoveTemporary,
    Entropy,
}
pub struct LocalStorageError {
    pub operation: LocalStorageOperation,
    pub publication: PublicationState,
    cause: Box<dyn Error + Send + Sync>,
    pub cleanup: Option<Box<LocalStorageError>>,
}
impl LocalStorageError {
    fn append_cleanup(&mut self, error: Self) {
        let mut slot = &mut self.cleanup;
        while let Some(existing) = slot {
            slot = &mut existing.cleanup;
        }
        *slot = Some(Box::new(error));
    }
    fn new(operation: LocalStorageOperation, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation,
            publication: PublicationState::NotPublished,
            cause: Box::new(cause),
            cleanup: None,
        }
    }
    pub fn code(&self) -> &'static str {
        if self.publication == PublicationState::RenamedDurabilityUnknown {
            "local-publication-uncertain"
        } else {
            "local-storage"
        }
    }
}
impl fmt::Display for LocalStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.code(), self.operation)
    }
}
impl fmt::Debug for LocalStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for LocalStorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
fn close(file: File, operation: LocalStorageOperation) -> Result<(), LocalStorageError> {
    nix::unistd::close(file).map_err(|e| LocalStorageError::new(operation, e))
}
fn close_after<T>(
    result: Result<T, LocalStorageError>,
    file: File,
) -> Result<T, LocalStorageError> {
    match (result, close(file, LocalStorageOperation::CloseFile)) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e),
        (Err(mut e), Err(cleanup)) => {
            e.cleanup = Some(Box::new(cleanup));
            Err(e)
        }
    }
}
fn owned(file: &File, directory: bool) -> Result<(), LocalStorageError> {
    owned_mode(file, directory, true)
}
fn owned_mode(file: &File, directory: bool, private: bool) -> Result<(), LocalStorageError> {
    let metadata = file
        .metadata()
        .map_err(|e| LocalStorageError::new(LocalStorageOperation::Metadata, e))?;
    if (if directory {
        !metadata.is_dir()
    } else {
        !metadata.is_file()
    }) || metadata.uid() != nix::unistd::geteuid().as_raw()
        || (private && metadata.mode() & 0o077 != 0)
        || (!directory && metadata.nlink() != 1)
    {
        return Err(LocalStorageError::new(
            LocalStorageOperation::Metadata,
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "owned private local object required",
            ),
        ));
    }
    Ok(())
}
pub(crate) struct LocalDirectory {
    file: File,
}
/// Set private permissions on the inode pinned by our owned O_PATH descriptor.
/// Trusted procfs resolves this process-local magic link even after rename; no
/// data descriptor is opened/closed, preserving SQLite POSIX locks. This is only
/// initialization repair of an owned regular single-link file, never a caller
/// FD or pathname grant. Missing procfs/chmod/metadata failures refuse, with no
/// fallback to state.db or unsupported fchmodat2/AT_EMPTY_PATH emulation.
#[cfg(target_os = "linux")]
fn privatize_database(file: &File) -> Result<(), LocalStorageError> {
    use std::os::fd::AsRawFd;
    owned_mode(file, false, false)?;
    fs::chmodat(
        fs::CWD,
        format!("/proc/self/fd/{}", file.as_raw_fd()),
        Mode::RUSR | Mode::WUSR,
        AtFlags::empty(),
    ).map_err(|error| LocalStorageError::new(LocalStorageOperation::Permissions, error))?;
    owned(file, false)
}

impl LocalDirectory {
    /// Open/create one private child relative to this retained descriptor.
    /// Names cannot traverse; existing symlinks/nonprivate directories refuse.
    /// Creation is durable before return; ownership/open/close errors survive.
    #[cfg(target_os = "linux")]
    pub(crate) fn child(&self, name: &str, create: bool) -> Result<Self, LocalStorageError> {
        Self::name(name)?;
        if create {
            match fs::mkdirat(&self.file, name, Mode::RWXU) {
                Ok(()) => self.file.sync_all().map_err(|e|LocalStorageError::new(LocalStorageOperation::SyncDirectory,e))?,
                Err(e) if e==rustix::io::Errno::EXIST => {},
                Err(e) => return Err(LocalStorageError::new(LocalStorageOperation::CreateDirectory,e)),
            }
        }
        let file:File=fs::openat(&self.file,name,OFlags::RDONLY|OFlags::DIRECTORY|OFlags::NOFOLLOW|OFlags::CLOEXEC,Mode::empty())
            .map_err(|e|LocalStorageError::new(LocalStorageOperation::OpenDirectory,e))?.into();
        if let Err(e)=owned(&file,true) { return close_after(Err(e),file); }
        Ok(Self{file})
    }
    /// Procfs reference to OUR retained directory for subprocess cwd/arguments.
    /// Caller keeps this descriptor alive until the process has been reaped.
    /// No caller-supplied PID/FD, pathname reopen or actor authentication occurs.
    #[cfg(target_os = "linux")]
    pub(crate) fn process_path(&self) -> std::path::PathBuf {
        use std::os::fd::AsRawFd;
        std::path::PathBuf::from(format!("/proc/{}/fd/{}",std::process::id(),self.file.as_raw_fd()))
    }
    /// Move one retained source child into this directory without replacement.
    /// Both simple names are validated; fsync both parents after visible rename.
    /// A sync failure retains publication uncertainty; no rollback/deletion claim.
    #[cfg(target_os = "linux")]
    pub(crate) fn rename_from(&self, source:&Self, from:&str, to:&str)->Result<(),LocalStorageError>{
        Self::name(from)?;Self::name(to)?;
        fs::renameat_with(&source.file,from,&self.file,to,fs::RenameFlags::NOREPLACE)
            .map_err(|e|LocalStorageError::new(LocalStorageOperation::Rename,e))?;
        let a=self.file.sync_all();let b=source.file.sync_all();
        let mut error=None;
        for result in [a,b] {
            if let Err(cause)=result {
                let mut e=LocalStorageError::new(LocalStorageOperation::SyncDirectory,cause);
                e.publication=PublicationState::RenamedDurabilityUnknown;
                if let Some(primary)=error.as_mut(){LocalStorageError::append_cleanup(primary,e);}else{error=Some(e);}
            }
        }
        if let Some(error)=error {Err(error)}else{Ok(())}
    }
    /// Borrow the retained directory only for the private gate transport. No
    /// pathname reopen, duplicate ownership or execution permission is created.
    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn gate_descriptor(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.file.as_fd()
    }

    /// Adopt a descriptor received by the private gate before any child SQLite
    /// open. Require the real RDONLY directory mode, CLOEXEC and existing private
    /// owner contract; never repair permissions. Rejection closes the descriptor
    /// explicitly and retains validation plus close causes. Metadata alone does
    /// not authenticate the sender or grant execution.
    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn adopt_gate_descriptor(fd: std::os::fd::OwnedFd) -> Result<Self, LocalStorageError> {
        let file: File = fd.into();
        let result = (|| {
            let flags = fs::fcntl_getfl(&file)
                .map_err(|e| LocalStorageError::new(LocalStorageOperation::Metadata, e))?;
            let descriptor_flags = rustix::io::fcntl_getfd(&file)
                .map_err(|e| LocalStorageError::new(LocalStorageOperation::Metadata, e))?;
            if flags & OFlags::ACCMODE != OFlags::RDONLY
                || !flags.contains(OFlags::DIRECTORY) || flags.contains(OFlags::PATH)
                || !descriptor_flags.contains(rustix::io::FdFlags::CLOEXEC)
            {
                return Err(LocalStorageError::new(LocalStorageOperation::Metadata,
                    io::Error::new(io::ErrorKind::PermissionDenied, "private gate directory descriptor mode required")));
            }
            owned(&file, true)
        })();
        match result {
            Ok(()) => Ok(Self { file }),
            Err(error) => close_after(Err(error), file),
        }
    }
    pub(crate) fn open(path: &Path) -> Result<Self, LocalStorageError> {
        let file: File = fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| LocalStorageError::new(LocalStorageOperation::OpenDirectory, e))?
        .into();
        if let Err(error) = owned(&file, true) {
            return close_after(Err(error), file);
        }
        Ok(Self { file })
    }
    /// Open a standalone resource directory owned by this user without requiring
    /// private directory permissions. Telemetry parents and managed source walks
    /// may retain ordinary 0755 modes; private-file reads still enforce their
    /// separate mode contract. Root bindings
    /// continue to use open(), and this is not same-UID OS confinement.
    pub(crate) fn open_owned(path: &Path) -> Result<Self, LocalStorageError> {
        let file: File = fs::open(path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty()).map_err(|error| LocalStorageError::new(LocalStorageOperation::OpenDirectory,error))?.into();
        if let Err(error) = owned_mode(&file, true, false) {
            return close_after(Err(error), file);
        }
        Ok(Self { file })
    }
    /// Observe a retained user-owned directory, preserving standalone telemetry's
    /// non-private parent compatibility. Root identity() remains strictly private.
    /// File descriptor ownership, not this tuple, prevents inode reuse.
    #[cfg(target_os = "linux")]
    pub(crate) fn owned_identity(&self) -> Result<(u64,u64),LocalStorageError> {
        owned_mode(&self.file,true,false)?;
        self.file.metadata().map(|metadata| (metadata.dev(),metadata.ino()))
            .map_err(|error| LocalStorageError::new(LocalStorageOperation::Metadata,error))
    }
    fn name(name: &str) -> Result<(), LocalStorageError> {
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains('/')
            || name.contains('\0')
        {
            Err(LocalStorageError::new(
                LocalStorageOperation::OpenFile,
                io::Error::new(io::ErrorKind::InvalidInput, "simple local name required"),
            ))
        } else {
            Ok(())
        }
    }
    fn open_file(&self, name: &str, flags: OFlags) -> Result<File, LocalStorageError> {
        Self::name(name)?;
        let file: File = fs::openat(
            &self.file,
            name,
            flags | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|e| LocalStorageError::new(LocalStorageOperation::OpenFile, e))?
        .into();
        if let Err(error) = owned(&file, false) {
            return close_after(Err(error), file);
        }
        Ok(file)
    }
    /// Read at most a caller-specified file-format bound plus one refusal byte.
    /// Uses the same owned/private/regular/NOFOLLOW checks and checked close as
    /// other reads. Excess bytes refuse without truncation or state mutation;
    /// this parameter is representation/resource protection, not a budget.
    pub(crate) fn read_bounded(
        &self,
        name: &str,
        maximum: usize,
    ) -> Result<Vec<u8>, LocalStorageError> {
        let mut file = self.open_file(name, OFlags::RDONLY)?;
        let result = (|| {
            let bound = u64::try_from(maximum)
                .ok()
                .and_then(|n| n.checked_add(1))
                .ok_or_else(|| {
                    LocalStorageError::new(
                        LocalStorageOperation::Read,
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "file format bound cannot be represented",
                        ),
                    )
                })?;
            let mut bytes = Vec::new();
            (&mut file)
                .take(bound)
                .read_to_end(&mut bytes)
                .map_err(|e| LocalStorageError::new(LocalStorageOperation::Read, e))?;
            if bytes.len() > maximum {
                return Err(LocalStorageError::new(
                    LocalStorageOperation::Read,
                    io::Error::new(io::ErrorKind::InvalidData, "file format bound exceeded"),
                ));
            }
            Ok(bytes)
        })();
        close_after(result, file)
    }
    /// Read an owned local configuration under an explicit optional byte cap.
    /// None means the saved policy permits complete reading, not a hidden fallback.
    /// Open/read/close failures retain causes; callers validate the decoded schema.
    pub(crate) fn read_capped(
        &self,
        name: &str,
        maximum: Option<usize>,
    ) -> Result<Vec<u8>, LocalStorageError> {
        if let Some(maximum) = maximum {
            return self.read_bounded(name, maximum);
        }
        let mut file = self.open_file(name, OFlags::RDONLY)?;
        let mut bytes = Vec::new();
        let result = file
            .read_to_end(&mut bytes)
            .map(|_| bytes)
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::Read, e));
        close_after(result, file)
    }
    /// Historical trust-actions.lock, shared/exclusive blocking flock. This is
    /// the existing serialization protocol, no retry, timeout or operator cap.
    /// The owned directory descriptor rejects path/symlink substitution; unlock
    /// and close remain explicit and preserve their original causes.
    pub(crate) fn reserve_trust(&self, exclusive: bool) -> Result<LocalReservation, LocalStorageError> {
        let file=self.open_file("trust-actions.lock",OFlags::RDWR|OFlags::CREATE)?;
        if let Err(error)=fs::flock(&file,if exclusive {FlockOperation::LockExclusive}else{FlockOperation::LockShared}) {
            return close_after(Err(LocalStorageError::new(LocalStorageOperation::Lock,error)),file);
        }
        Ok(LocalReservation{file})
    }

    /// Reserve the actual Go pricing lock with its blocking exclusive semantics.
    /// No timeout, retries or alternative lock name is invented. Open/flock/close
    /// failures preserve causes; callers must close the returned reservation.
    pub(crate) fn reserve_pricing(&self) -> Result<LocalReservation, LocalStorageError> {
        let file = self.open_file("model-rates.lock", OFlags::RDWR | OFlags::CREATE)?;
        if let Err(error) = fs::flock(&file, FlockOperation::LockExclusive) {
            return close_after(
                Err(LocalStorageError::new(LocalStorageOperation::Lock, error)),
                file,
            );
        }
        Ok(LocalReservation { file })
    }
    /// Acquire the real Go providers.lock exclusively without an implicit wait,
    /// like other Root try-lock guards. A busy/error result preserves its cause;
    /// caller can retry only after a changed precondition. No retry/deadline policy
    /// is invented, no other process interrupted, and close remains explicit.
    #[cfg(target_os = "linux")]
    pub(crate) fn reserve_provider_configuration(
        &self,
    ) -> Result<LocalReservation, LocalStorageError> {
        self.reserve_configuration("providers.lock")
    }
    /// Acquire Go's provider-wait lock with checked private regular-file custody.
    /// Nonblocking contention/errors preserve causes; no retry or policy deadline.
    #[cfg(target_os = "linux")]
    pub(crate) fn reserve_provider_wait(&self) -> Result<LocalReservation, LocalStorageError> {
        self.reserve_configuration("provider-wait.lock")
    }
    /// Go browser waiting policy lock, owned private nonblocking custody.
    /// No new retry/deadline. Original open/flock/unlock/close causes remain.
    #[cfg(target_os = "linux")]
    pub(crate) fn reserve_web_requests(&self) -> Result<LocalReservation, LocalStorageError> {
        self.reserve_configuration("web-requests.lock")
    }
    /// Acquire the Go graph objectives lock under private checked custody.
    /// Nonblocking errors retain causes; caller must close, no retry or deadline.
    #[cfg(target_os = "linux")]
    pub(crate) fn reserve_graph_performance(&self) -> Result<LocalReservation, LocalStorageError> {
        self.reserve_configuration("graph-performance.lock")
    }
    /// Reserve the native activity policy file under owned nonblocking custody.
    /// No retry/deadline or data grant; caller explicitly checks close.
    #[cfg(target_os="linux")]
    pub(crate) fn reserve_activity_read_policy(&self)->Result<LocalReservation,LocalStorageError>{self.reserve_configuration("activity-policy.lock")}
    /// Reserve the saved log/terminal read policy under owned nonblocking custody.
    /// Caller checks close; no retry, deadline, row default or data permission.
    #[cfg(target_os="linux")]
    pub(crate) fn reserve_log_read_policy(&self)->Result<LocalReservation,LocalStorageError>{self.reserve_configuration("log-read-policy.lock")}
    /// Shared checked flat-name configuration lock. Only crate-owned fixed
    /// protocol names are passed; no external path/authority, truncation or wait.
    /// Caller must explicitly close the returned owned reservation.
    #[cfg(target_os = "linux")]
    fn reserve_configuration(&self, name: &str) -> Result<LocalReservation, LocalStorageError> {
        let file = self.open_file(name, OFlags::RDWR | OFlags::CREATE)?;
        if let Err(e) = fs::flock(&file, FlockOperation::NonBlockingLockExclusive) {
            return close_after(
                Err(LocalStorageError::new(LocalStorageOperation::Lock, e)),
                file,
            );
        }
        Ok(LocalReservation { file })
    }
    /// Duplicate the checked directory descriptor with CLOEXEC. The retained
    /// descriptor prevents inode reuse while a datastore binding is live; this
    /// creates no authentication, path lock or authority. Lowest available FD0
    /// is an OS allocation parameter, never operator policy. Caller owes close.
    #[cfg(target_os = "linux")]
    pub(crate) fn duplicate(&self) -> Result<Self, LocalStorageError> {
        let fd = rustix::io::fcntl_dupfd_cloexec(&self.file, 0)
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::DuplicateDirectory, e))?;
        Ok(Self { file: fd.into() })
    }
    /// Observe the actual checked directory descriptor's device/inode for local
    /// preview target binding. This is a host observation, not authenticated Root
    /// identity or an enduring lease. Metadata/ownership failures retain causes.
    #[cfg(target_os = "linux")]
    pub(crate) fn identity(&self) -> Result<(u64, u64), LocalStorageError> {
        owned(&self.file, true)?;
        let metadata = self
            .file
            .metadata()
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::Metadata, e))?;
        Ok((metadata.dev(), metadata.ino()))
    }
    /// Prepare a private SQLite main file before connecting or mutating SQL.
    /// Existing owned regular single-link files are repaired through an O_PATH
    /// descriptor and trusted procfs magic link. Missing files are first written
    /// privately under a unique temporary name, synced/closed, then published by
    /// NOREPLACE: no data-file close occurs after publication. A concurrent target
    /// is an explicit conflict, never overwritten or silently retried. Errors
    /// preserve partial publication and cleanup causes; no SQL or schema is run.
    #[cfg(target_os = "linux")]
    pub(crate) fn prepare_database_for_initialization(&self) -> Result<(), LocalStorageError> {
        let open = || fs::openat(
            &self.file, "state.db",
            OFlags::PATH | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        );
        match open() {
            Ok(fd) => {
                let file: File = fd.into();
                close_after(privatize_database(&file), file)
            },
            Err(error) if error == rustix::io::Errno::NOENT => {
                self.publish_new_with_fault("state.db", b"", AtomicStorageFault::None)?;
                // The process umask can remove owner bits from the private temp.
                // Set exact initialization permissions through an owned O_PATH
                // witness after publication, without any data descriptor close.
                let file: File = open()
                    .map_err(|error| LocalStorageError::new(LocalStorageOperation::OpenFile, error))?.into();
                close_after(privatize_database(&file), file)
            },
            Err(error) => Err(LocalStorageError::new(LocalStorageOperation::OpenFile, error)),
        }
    }
    /// Observe the owned private state.db inode relative to this retained directory.
    /// O_PATH pins the file during metadata inspection without a data-file close
    /// that could release SQLite's process-wide POSIX locks. NOFOLLOW and the
    /// usual ownership/regular/private/single-link checks remain mandatory.
    /// Errors preserve metadata and explicit-close causes; no data is read or
    /// written and the resulting tuple grants no authority by itself.
    #[cfg(target_os = "linux")]
    pub(crate) fn database_identity(&self) -> Result<(u64, u64), LocalStorageError> {
        self.regular_identity("state.db")
    }
    /// Observe a simple-name private regular single-link file relative to this
    /// retained directory. O_PATH pins its inode only during inspection; callers
    /// retaining authority need their own owned witness. No data descriptor is
    /// opened or closed, and no pathname outside this directory is followed.
    /// Metadata and explicit O_PATH close failures preserve both causes.
    #[cfg(target_os = "linux")]
    pub(crate) fn regular_identity(&self, name: &str) -> Result<(u64, u64), LocalStorageError> {
        let file = self.open_file(name, OFlags::PATH)?;
        let result = file.metadata()
            .map(|metadata| (metadata.dev(), metadata.ino()))
            .map_err(|error| LocalStorageError::new(LocalStorageOperation::Metadata, error));
        close_after(result, file)
    }
    /// Confirm an existing private regular file and this directory are synced.
    /// Used by an explicit no-change apply after a visible/uncertain replacement;
    /// no bytes are rewritten. Open/sync/close causes remain joined, no retry or
    /// durability success is invented. Caller holds its content/CAS reservation.
    #[cfg(target_os = "linux")]
    pub(crate) fn synchronize_existing(&self, name: &str) -> Result<(), LocalStorageError> {
        let file = self.open_file(name, OFlags::RDONLY)?;
        let synced = file
            .sync_all()
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::SyncFile, e));
        close_after(synced, file)?;
        self.file
            .sync_all()
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::SyncDirectory, e))
    }
    /// Enumerate names from a fresh descriptor anchored to this checked directory.
    /// No path traversal, secret content reads or silent partial result. Linux
    /// dirent64.d_reclen is u16: its representational maximum plus alignment slack
    /// bounds one transfer buffer, not entry count, retention or operator policy.
    /// Explicit close joins read/close failures. Results are unverified names;
    /// consumers must check selected file ownership/type/bytes again before use.
    #[cfg(target_os = "linux")]
    pub(crate) fn names(&self) -> Result<Vec<Vec<u8>>, LocalStorageError> {
        let fd = fs::openat(
            &self.file,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| LocalStorageError::new(LocalStorageOperation::OpenDirectory, e))?;
        let file: File = fd.into();
        let result = (|| {
            owned(&file, true)?;
            let mut buffer = vec![
                std::mem::MaybeUninit::<u8>::uninit();
                usize::from(u16::MAX) + 1 + std::mem::align_of::<u64>()
            ];
            let mut reader = fs::RawDir::new(&file, &mut buffer);
            let mut names = Vec::new();
            while let Some(entry) = reader.next() {
                let entry =
                    entry.map_err(|e| LocalStorageError::new(LocalStorageOperation::Read, e))?;
                names.push(entry.file_name().to_bytes().to_vec());
            }
            Ok(names)
        })();
        close_after(result, file)
    }

    /// Read a framed native receipt through the existing private anchored regular
    /// file checks. Actual extent/header/hash are checked before allocation; this
    /// adds no operator byte ceiling. Preserve read and close errors together.
    /// Returned bytes are untrusted domain data, not authenticated run authority.
    #[cfg(target_os = "linux")]
    pub(crate) fn read_native_receipt(&self, name: &str) -> Result<Vec<u8>, LocalStorageError> {
        let mut file = self.open_file(name, OFlags::RDONLY)?;
        let result = crate::native_receipt::read(&mut file)
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::Read, e));
        close_after(result, file)
    }
    pub(crate) fn read(&self, name: &str) -> Result<Vec<u8>, LocalStorageError> {
        let mut file = self.open_file(name, OFlags::RDONLY)?;
        let mut data = vec![];
        let result = file
            .read_to_end(&mut data)
            .map(|_| data)
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::Read, e));
        match (result, close(file, LocalStorageOperation::CloseFile)) {
            (Ok(data), Ok(())) => Ok(data),
            (Err(e), Ok(())) => Err(e),
            (Ok(_), Err(e)) => Err(e),
            (Err(mut e), Err(cleanup)) => {
                e.cleanup = Some(Box::new(cleanup));
                Err(e)
            }
        }
    }
    /// Only the inherited Go retry file permits public read bits and 4096 bytes.
    /// This does not weaken ownership, regular-file, hardlink or symlink checks.
    pub(crate) fn read_legacy_retry(&self) -> Result<Vec<u8>, LocalStorageError> {
        let mut file: File = fs::openat(
            &self.file,
            "storage-retry.json",
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| LocalStorageError::new(LocalStorageOperation::OpenFile, e))?
        .into();
        let result = (|| {
            owned_mode(&file, false, false)?;
            let mut data = Vec::new();
            (&mut file)
                .take(4097)
                .read_to_end(&mut data)
                .map_err(|e| LocalStorageError::new(LocalStorageOperation::Read, e))?;
            if data.len() > 4096 {
                return Err(LocalStorageError::new(
                    LocalStorageOperation::Read,
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "legacy retry file exceeds Go format bound",
                    ),
                ));
            }
            Ok(data)
        })();
        close_after(result, file)
    }
    pub(crate) fn initialize_layout(&self) -> Result<(), LocalStorageError> {
        self.validate_layout()?;
        match fs::mkdirat(&self.file, "views", Mode::RWXU) {
            Ok(()) => {}
            Err(e) if e == rustix::io::Errno::EXIST => {}
            Err(e) => {
                return Err(LocalStorageError::new(
                    LocalStorageOperation::CreateDirectory,
                    e,
                ))
            }
        }
        let views: File = fs::openat(
            &self.file,
            "views",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| LocalStorageError::new(LocalStorageOperation::OpenDirectory, e))?
        .into();
        close_after(owned(&views, true), views)?;
        // Existing user-owned ignore content is preserved, never overwritten.
        match self.open_file(".gitignore", OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL) {
            Ok(mut file) => {
                let result = file
                    .write_all(b"*\n")
                    .and_then(|()| file.sync_all())
                    .map_err(|e| LocalStorageError::new(LocalStorageOperation::Write, e));
                close_after(result, file)?;
            }
            Err(e)
                if e.source()
                    .and_then(|s| s.downcast_ref::<rustix::io::Errno>())
                    .is_some_and(|s| *s == rustix::io::Errno::EXIST) => {}
            Err(e) => return Err(e),
        }
        self.file
            .sync_all()
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::SyncDirectory, e))
    }
    pub(crate) fn validate_layout(&self) -> Result<(), LocalStorageError> {
        for name in ["views", "imports"] {
            match fs::openat(
                &self.file,
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            ) {
                Ok(fd) => {
                    let file: File = fd.into();
                    close_after(owned(&file, true), file)?;
                }
                Err(e) if e == rustix::io::Errno::NOENT => {}
                Err(e) => {
                    return Err(LocalStorageError::new(
                        LocalStorageOperation::OpenDirectory,
                        e,
                    ))
                }
            }
        }
        Ok(())
    }
    pub(crate) fn reserve(
        &self,
        exclusive: bool,
        create: bool,
    ) -> Result<LocalReservation, LocalStorageError> {
        let file = self.open_file(
            ".engine-settings.lock",
            OFlags::RDWR
                | if create {
                    OFlags::CREATE
                } else {
                    OFlags::empty()
                },
        )?;
        if let Err(error) = fs::flock(
            &file,
            if exclusive {
                FlockOperation::NonBlockingLockExclusive
            } else {
                FlockOperation::NonBlockingLockShared
            },
        ) {
            return close_after(
                Err(LocalStorageError::new(LocalStorageOperation::Lock, error)),
                file,
            );
        }
        Ok(LocalReservation { file })
    }
    pub(crate) fn publish(&self, name: &str, data: &[u8]) -> Result<(), LocalStorageError> {
        self.publish_with_fault(name, data, AtomicStorageFault::None)
    }
    /// Publish replacement bytes with the existing compile-gated fault device.
    /// Same fsync/close/rename/cleanup causes as publish_mode; never a policy.
    pub(crate) fn publish_with_fault(
        &self,
        name: &str,
        data: &[u8],
        fault: AtomicStorageFault,
    ) -> Result<(), LocalStorageError> {
        self.publish_mode(name, data, fault, PublicationMode::Replace)
    }
    /// Publish complete bytes only when the target name does not exist. Linux
    /// NOREPLACE is the atomic race boundary, including symlink targets. Unsupported
    /// kernels/filesystems refuse; no fallback replaces a target. Same private
    /// temporary/fsync/checked-close/directory-sync and cleanup contract as replace.
    /// Fault variants are absent in normal builds; no process or retry is created.
    #[cfg(target_os = "linux")]
    pub(crate) fn publish_new_with_fault(
        &self,
        name: &str,
        data: &[u8],
        fault: AtomicStorageFault,
    ) -> Result<(), LocalStorageError> {
        self.publish_mode(name, data, fault, PublicationMode::CreateNew)
    }
    /// Shared durable publication. The mode is create/replace operation semantics;
    /// a failed pre-rename write preserves the target. A visible rename with failed
    /// directory sync retains RenamedDurabilityUnknown; cleanup never implies rollback.
    fn publish_mode(
        &self,
        name: &str,
        data: &[u8],
        fault: AtomicStorageFault,
        mode: PublicationMode,
    ) -> Result<(), LocalStorageError> {
        #[cfg(not(feature = "test-fixtures"))]
        let _ = fault;
        Self::name(name)?;
        let temp = generate_identifier(".swarm-write-")
            .map_err(|e: RuntimeError| LocalStorageError::new(LocalStorageOperation::Entropy, e))?;
        let mut file = self.open_file(&temp, OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL)?;
        let written = (|| {
            #[cfg(feature = "test-fixtures")]
            if fault == AtomicStorageFault::PartialWriteNoSpace {
                file.write_all(data.get(..1).unwrap_or_default())
                    .map_err(|e| LocalStorageError::new(LocalStorageOperation::Write, e))?;
                return Err(LocalStorageError::new(
                    LocalStorageOperation::Write,
                    io::Error::from_raw_os_error(rustix::io::Errno::NOSPC.raw_os_error()),
                ));
            }
            file.write_all(data)
                .map_err(|e| LocalStorageError::new(LocalStorageOperation::Write, e))?;
            #[cfg(feature = "test-fixtures")]
            if matches!(
                fault,
                AtomicStorageFault::SyncFile | AtomicStorageFault::SyncAndCleanup
            ) {
                return Err(LocalStorageError::new(
                    LocalStorageOperation::SyncFile,
                    io::Error::other("fixture sync refusal"),
                ));
            }
            file.sync_all()
                .map_err(|e| LocalStorageError::new(LocalStorageOperation::SyncFile, e))
        })();
        let closed = close(file, LocalStorageOperation::CloseFile);
        #[cfg(feature = "test-fixtures")]
        let closed = closed.and_then(|()| {
            if fault == AtomicStorageFault::CloseFile {
                Err(LocalStorageError::new(
                    LocalStorageOperation::CloseFile,
                    io::Error::other("fixture close refusal"),
                ))
            } else {
                Ok(())
            }
        });
        let first = match (written, closed) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
            (Err(mut e), Err(cleanup)) => {
                e.cleanup = Some(Box::new(cleanup));
                Err(e)
            }
        };
        let result = first
            .and_then(|()| {
                #[cfg(feature = "test-fixtures")]
                if fault == AtomicStorageFault::Rename {
                    return Err(LocalStorageError::new(
                        LocalStorageOperation::Rename,
                        io::Error::other("fixture rename refusal"),
                    ));
                }
                match mode {
                    PublicationMode::Replace => fs::renameat(&self.file, &temp, &self.file, name),
                    #[cfg(target_os = "linux")]
                    PublicationMode::CreateNew => fs::renameat_with(
                        &self.file,
                        &temp,
                        &self.file,
                        name,
                        fs::RenameFlags::NOREPLACE,
                    ),
                }
                .map_err(|e| LocalStorageError::new(LocalStorageOperation::Rename, e))
            })
            .and_then(|()| {
                #[cfg(feature = "test-fixtures")]
                if fault == AtomicStorageFault::SyncDirectory {
                    let mut error = LocalStorageError::new(
                        LocalStorageOperation::SyncDirectory,
                        io::Error::other("fixture directory sync refusal"),
                    );
                    error.publication = PublicationState::RenamedDurabilityUnknown;
                    return Err(error);
                }
                self.file.sync_all().map_err(|e| {
                    let mut error = LocalStorageError::new(LocalStorageOperation::SyncDirectory, e);
                    error.publication = PublicationState::RenamedDurabilityUnknown;
                    error
                })
            });
        if let Err(mut error) = result {
            if error.publication == PublicationState::NotPublished {
                #[cfg(feature = "test-fixtures")]
                if fault == AtomicStorageFault::SyncAndCleanup {
                    error.append_cleanup(LocalStorageError::new(
                        LocalStorageOperation::RemoveTemporary,
                        io::Error::other("fixture cleanup refusal"),
                    ));
                    return Err(error);
                }
                if let Err(e) = fs::unlinkat(&self.file, &temp, AtFlags::empty()) {
                    if e != rustix::io::Errno::NOENT {
                        error.append_cleanup(LocalStorageError::new(
                            LocalStorageOperation::RemoveTemporary,
                            e,
                        ));
                    }
                }
            }
            return Err(error);
        }
        Ok(())
    }
    pub(crate) fn close(self) -> Result<(), LocalStorageError> {
        close(self.file, LocalStorageOperation::CloseFile)
    }
}
pub(crate) struct LocalReservation {
    file: File,
}
impl LocalReservation {
    pub(crate) fn close(self) -> Result<(), LocalStorageError> {
        let unlocked = fs::flock(&self.file, FlockOperation::Unlock)
            .map_err(|e| LocalStorageError::new(LocalStorageOperation::Unlock, e));
        let closed = close(self.file, LocalStorageOperation::CloseLock);
        match (unlocked, closed) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
            (Err(mut e), Err(cleanup)) => {
                e.cleanup = Some(Box::new(cleanup));
                Err(e)
            }
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod binding_tests {
    use super::*;
    /// Real directory/descriptor witness in the default image's unit suite.
    /// No fixture feature, inherited provider, policy or live Root is required.
    /// Failure retains the owned directory for diagnosis; success closes both FDs.
    #[test]
    fn directory_binding_duplicate_preserves_identity_and_cloexec() {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(generate_identifier("binding-storage-").unwrap());
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        let original = LocalDirectory::open(&path).unwrap();
        let duplicate = original.duplicate().unwrap();
        assert_eq!(original.identity().unwrap(), duplicate.identity().unwrap());
        assert!(rustix::io::fcntl_getfd(&duplicate.file)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC));
        duplicate.close().unwrap();
        original.close().unwrap();
        std::fs::remove_dir(&path).unwrap();
    }
}
#[cfg(all(test, feature = "test-fixtures"))]
mod tests {
    use super::*;
    use std::{fs as disk, os::unix::fs::PermissionsExt};
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(generate_identifier("atomic-storage-").unwrap());
            disk::create_dir(&p).unwrap();
            disk::set_permissions(&p, disk::Permissions::from_mode(0o700)).unwrap();
            Self(p)
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
    fn publication_faults_preserve_original_or_report_visible_uncertain_write_and_recover() {
        for fault in [
            AtomicStorageFault::PartialWriteNoSpace,
            AtomicStorageFault::SyncFile,
            AtomicStorageFault::CloseFile,
            AtomicStorageFault::Rename,
            AtomicStorageFault::SyncDirectory,
            AtomicStorageFault::SyncAndCleanup,
        ] {
            let f = Fixture::new();
            let dir = LocalDirectory::open(&f.0).unwrap();
            dir.publish("settings", b"original complete settings")
                .unwrap();
            assert_eq!(dir.read("settings").unwrap(), b"original complete settings");
            let error = dir
                .publish_with_fault("settings", b"next complete settings", fault)
                .unwrap_err();
            if fault == AtomicStorageFault::SyncDirectory {
                assert_eq!(
                    error.publication,
                    PublicationState::RenamedDurabilityUnknown
                );
                assert_eq!(dir.read("settings").unwrap(), b"next complete settings");
            } else {
                assert_eq!(error.publication, PublicationState::NotPublished);
                assert_eq!(dir.read("settings").unwrap(), b"original complete settings");
            }
            if fault == AtomicStorageFault::PartialWriteNoSpace {
                assert_eq!(
                    error
                        .source()
                        .unwrap()
                        .downcast_ref::<io::Error>()
                        .unwrap()
                        .raw_os_error(),
                    Some(rustix::io::Errno::NOSPC.raw_os_error())
                );
            }
            let temporary: Vec<_> = disk::read_dir(&f.0)
                .unwrap()
                .map(|v| v.unwrap())
                .filter(|v| {
                    v.file_name()
                        .as_encoded_bytes()
                        .starts_with(b".swarm-write-")
                })
                .collect();
            if fault == AtomicStorageFault::SyncAndCleanup {
                assert_eq!(
                    error.cleanup.unwrap().operation,
                    LocalStorageOperation::RemoveTemporary
                );
                assert_eq!(temporary.len(), 1);
            } else {
                assert!(temporary.is_empty());
            }
            dir.publish("settings", b"recovered complete settings")
                .unwrap();
            assert_eq!(
                dir.read("settings").unwrap(),
                b"recovered complete settings"
            );
            dir.close().unwrap();
        }
    }
    #[test]
    fn write_reservation_excludes_readers_and_writers_and_release_restores_access() {
        let f = Fixture::new();
        let dir = LocalDirectory::open(&f.0).unwrap();
        let writer = dir.reserve(true, true).unwrap();
        assert_eq!(
            dir.reserve(false, false).err().unwrap().operation,
            LocalStorageOperation::Lock
        );
        assert_eq!(
            dir.reserve(true, false).err().unwrap().operation,
            LocalStorageOperation::Lock
        );
        writer.close().unwrap();
        let a = dir.reserve(false, false).unwrap();
        let b = dir.reserve(false, false).unwrap();
        assert!(dir.reserve(true, false).is_err());
        a.close().unwrap();
        b.close().unwrap();
        dir.reserve(true, false).unwrap().close().unwrap();
        dir.close().unwrap();
    }
    /// The kernel no-replace boundary preserves regular and symlink targets;
    /// valid creation precedes refusal, and no failed candidate leaks a temp.
    #[cfg(target_os = "linux")]
    #[test]
    fn create_only_publication_preserves_existing_regular_and_symlink_targets() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        let dir = LocalDirectory::open(&f.0).unwrap();
        dir.publish_new_with_fault("target", b"original", AtomicStorageFault::None)
            .unwrap();
        let e = dir
            .publish_new_with_fault("target", b"replacement", AtomicStorageFault::None)
            .unwrap_err();
        assert_eq!(e.publication, PublicationState::NotPublished);
        assert_eq!(disk::read(f.0.join("target")).unwrap(), b"original");
        symlink("target", f.0.join("link")).unwrap();
        let e = dir
            .publish_new_with_fault("link", b"replacement", AtomicStorageFault::None)
            .unwrap_err();
        assert_eq!(e.publication, PublicationState::NotPublished);
        assert!(f.0.join("link").is_symlink());
        assert_eq!(disk::read(f.0.join("target")).unwrap(), b"original");
        let names: Vec<_> = disk::read_dir(&f.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 2);
        dir.close().unwrap();
    }
    /// Two independently opened publishers start together; exactly one creates
    /// the target. Barrier cardinality is a race fixture, not an operator cap.
    /// No timing sleep/deadline or shared-root mutation is used.
    #[cfg(target_os = "linux")]
    #[test]
    fn create_only_racing_publishers_never_overwrite_the_winning_bytes() {
        use std::sync::{Arc, Barrier};
        let f = Fixture::new();
        let dir = LocalDirectory::open(&f.0).unwrap();
        dir.publish_new_with_fault("control", b"control", AtomicStorageFault::None)
            .unwrap();
        assert_eq!(disk::read(f.0.join("control")).unwrap(), b"control");
        disk::remove_file(f.0.join("control")).unwrap();
        dir.close().unwrap();
        let gate = Arc::new(Barrier::new(3));
        let mut workers = Vec::new();
        for bytes in [b"first".as_slice(), b"second".as_slice()] {
            let path = f.0.clone();
            let gate = gate.clone();
            workers.push(std::thread::spawn(move || {
                gate.wait();
                let dir = LocalDirectory::open(&path).unwrap();
                let result = dir.publish_new_with_fault("target", bytes, AtomicStorageFault::None);
                let won = result.is_ok();
                if let Err(e) = result {
                    assert_eq!(e.publication, PublicationState::NotPublished);
                    assert_eq!(e.operation, LocalStorageOperation::Rename);
                }
                dir.close().unwrap();
                (won, bytes)
            }));
        }
        gate.wait();
        let outcomes: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(outcomes.iter().filter(|(won, _)| *won).count(), 1);
        assert_eq!(
            disk::read(f.0.join("target")).unwrap(),
            outcomes.iter().find(|(won, _)| *won).unwrap().1
        );
        assert_eq!(disk::read_dir(&f.0).unwrap().count(), 1);
    }
    /// Names are complete raw directory entries, not trusted file content. Fresh
    /// descriptor cursors make repeated listings work without following symlinks.
    #[cfg(target_os = "linux")]
    #[test]
    fn provider_configuration_directory_names_preserve_bytes_and_fresh_cursor() {
        use std::os::unix::{ffi::OsStrExt, fs::symlink};
        let f = Fixture::new();
        let dir = LocalDirectory::open(&f.0).unwrap();
        dir.publish("source", b"data").unwrap();
        symlink("missing", f.0.join("link")).unwrap();
        disk::write(
            f.0.join(std::ffi::OsStr::from_bytes(b"nonutf8\xff")),
            b"data",
        )
        .unwrap();
        for _ in 0..2 {
            let mut names = dir.names().unwrap();
            names.sort();
            let mut expected = vec![
                b".".to_vec(),
                b"..".to_vec(),
                b"source".to_vec(),
                b"link".to_vec(),
                b"nonutf8\xff".to_vec(),
            ];
            expected.sort();
            assert_eq!(names, expected);
        }
        dir.close().unwrap();
    }
    /// The shared Go lock name and immediate refusal are observable; no hidden
    /// wait/deadline/retry or alternate lock defeats another writer's reservation.
    #[cfg(target_os = "linux")]
    #[test]
    fn provider_configuration_lock_refuses_contention_then_recovers() {
        let f = Fixture::new();
        let dir = LocalDirectory::open(&f.0).unwrap();
        let first = dir.reserve_provider_configuration().unwrap();
        assert!(f.0.join("providers.lock").is_file());
        let error = dir.reserve_provider_configuration().err().unwrap();
        assert_eq!(error.operation, LocalStorageOperation::Lock);
        first.close().unwrap();
        dir.reserve_provider_configuration()
            .unwrap()
            .close()
            .unwrap();
        dir.close().unwrap();
    }

}


/// Default-profile initialization checks use no fixture-only fault code. Keep
/// these independent of the test-fixtures publication-injection test module.
#[cfg(all(test, target_os = "linux"))]
mod database_initialization_tests {
    use super::*;
    use std::{fs as disk, os::unix::fs::PermissionsExt};

    /// Own one fresh private filesystem root; never a running mission datastore.
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        /// Create the isolated private directory needed by real O_PATH calls.
        fn new() -> Self {
            let path = std::env::temp_dir().join(generate_identifier("database-init-").unwrap());
            disk::create_dir(&path).unwrap();
            disk::set_permissions(&path, disk::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        /// Remove only this owned fixture after success; preserve it on panic.
        fn drop(&mut self) {
            if !std::thread::panicking() { disk::remove_dir_all(&self.0).unwrap(); }
        }
    }
    /// Initialization publishes a private empty inode, repairs an existing owned
    /// inode without changing bytes, and refuses symlink/hardlink targets. These
    /// are owned filesystem fixtures, never runtime authority or SQL permission.
    #[cfg(target_os = "linux")]
    #[test]
    fn database_initialization_prepares_private_file_and_refuses_unsafe_targets() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        let dir = LocalDirectory::open(&f.0).unwrap();
        dir.prepare_database_for_initialization().unwrap();
        let path = f.0.join("state.db");
        assert_eq!(disk::metadata(&path).unwrap().mode() & 0o777, 0o600);
        assert_eq!(disk::read(&path).unwrap(), b"");
        disk::write(&path, b"retained original database bytes").unwrap();
        disk::set_permissions(&path, disk::Permissions::from_mode(0o644)).unwrap();
        let inode = disk::metadata(&path).unwrap().ino();
        dir.prepare_database_for_initialization().unwrap();
        assert_eq!(disk::metadata(&path).unwrap().ino(), inode);
        assert_eq!(disk::metadata(&path).unwrap().mode() & 0o777, 0o600);
        assert_eq!(disk::read(&path).unwrap(), b"retained original database bytes");
        disk::hard_link(&path, f.0.join("hardlink")).unwrap();
        assert!(dir.prepare_database_for_initialization().is_err());
        disk::remove_file(f.0.join("hardlink")).unwrap();
        disk::rename(&path, f.0.join("original.db")).unwrap();
        symlink("original.db", &path).unwrap();
        assert!(dir.prepare_database_for_initialization().is_err());
        assert!(path.is_symlink());
        assert_eq!(disk::read(f.0.join("original.db")).unwrap(), b"retained original database bytes");
        dir.close().unwrap();
    }

    /// A rename after descriptor acquisition must not retarget chmod. The procfs
    /// magic link remains attached to the owned original inode; no path fallback.
    #[cfg(target_os = "linux")]
    #[test]
    fn database_initialization_permission_repair_follows_owned_inode_after_rename() {
        let f = Fixture::new();
        let dir = LocalDirectory::open(&f.0).unwrap();
        dir.prepare_database_for_initialization().unwrap();
        let path = f.0.join("state.db");
        let held: File = fs::openat(&dir.file, "state.db", OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC, Mode::empty()).unwrap().into();
        disk::set_permissions(&path, disk::Permissions::from_mode(0o644)).unwrap();
        disk::rename(&path, f.0.join("original.db")).unwrap();
        disk::write(&path, b"replacement").unwrap();
        disk::set_permissions(&path, disk::Permissions::from_mode(0o644)).unwrap();
        privatize_database(&held).unwrap();
        assert_eq!(disk::metadata(f.0.join("original.db")).unwrap().mode() & 0o777, 0o600);
        assert_eq!(disk::metadata(&path).unwrap().mode() & 0o777, 0o644);
        assert_ne!(held.metadata().unwrap().ino(), disk::metadata(&path).unwrap().ino());
        close(held, LocalStorageOperation::CloseFile).unwrap();
        dir.close().unwrap();
    }

    /// Physical child BEGIN IMMEDIATE is the lock oracle before and after the
    /// O_PATH chmod/close path. Reuse the component's explicit ignored child entry;
    /// no elapsed timeout or parent-only SQLite query substitutes for this check.
    #[cfg(target_os = "linux")]
    #[test]
    fn database_initialization_opath_chmod_preserves_cross_process_sqlite_lock() {
        let f = Fixture::new();
        let dir = LocalDirectory::open(&f.0).unwrap();
        dir.prepare_database_for_initialization().unwrap();
        let path = f.0.join("state.db");
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE witness(value INTEGER); BEGIN IMMEDIATE").unwrap();
        let child = |expected: &str| {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--ignored", "--exact", "connected_database_identity::tests::connected_database_lock_child_process", "--nocapture", "--test-threads=1"])
                .env("SWARM_SQLITE_IDENTITY_LOCK_CHILD_PATH", &path)
                .env("SWARM_SQLITE_IDENTITY_LOCK_CHILD_EXPECTED", expected)
                .output().unwrap();
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            assert!(output.status.success(), "real child lock assertion failed");
            assert!(String::from_utf8(output.stdout).unwrap().contains(&format!("OBSERVED independent SQLite lock {expected}")));
        };
        child("busy");
        disk::set_permissions(&path, disk::Permissions::from_mode(0o644)).unwrap();
        dir.prepare_database_for_initialization().unwrap();
        child("busy");
        db.execute_batch("ROLLBACK").unwrap();
        child("available");
        db.close().unwrap();
        dir.close().unwrap();
    }

}
