//! One fixture-settings access and an owned immutable resolution context.
//!
//! Saved parent/leaf absence is checked against held directory descriptors.
//! Existing symlinks may be followed; descriptor identity and consumed-byte hash
//! are observations, not authentication, confinement or an atomic content snapshot.
//! No setting/directory is written. Only proven saved absence selects the shipped
//! file; all other failures remain failures. Public wrappers retain the first
//! complete result for their process; tests can own fresh contexts without reset.
use crate::engine_settings::EngineSettingsValues;
use rustix::fs::{open, openat, statat, AtFlags, Mode, OFlags};
use sha2::{Digest, Sha256};
use std::{cell::Cell, error::Error, fmt, fs::File, io::{self, Read},
    os::{fd::IntoRawFd, unix::fs::MetadataExt}, path::{Path, PathBuf}, sync::{Arc, OnceLock}};

type Cause = Arc<dyn Error + Send + Sync>;
pub(super) type Outcome = Result<Arc<ResolvedArea>, FixtureAreaError>;

/// Cloneable original resolution causes retained across the process cache/panic.
/// Display/Debug expose static stages only. Downcast the panic payload to this
/// exported type; source() retains the primary and explicit iterators retain
/// secondary close/observation causes. No retry, rollback or acceptance is implied.
#[derive(Clone)]
pub struct FixtureAreaError(Arc<Failure>);
#[derive(Clone)]
struct Failure {
    stage: &'static str,
    source: &'static str,
    primary: Cause,
    close: Vec<CloseIssue>,
    observation: Vec<Cause>,
    trace: Vec<AccessEvent>,
}
#[derive(Clone)]
struct CloseIssue { phase: &'static str, descriptor: i32, cause: Cause, ownership_uncertain: bool }

/// A same-access observation: phase, chosen source and optional inode/hash.
/// No supplied invalid values or arbitrary error messages are carried here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AccessEvent {
    pub phase: &'static str,
    pub source: &'static str,
    pub identity: Option<(u64, u64)>,
    pub consumed_sha256: Option<String>,
}

impl FixtureAreaError {
    /// Build a safe staged failure retaining its original typed cause.
    fn caused(stage: &'static str, source: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self(Arc::new(Failure { stage, source, primary: Arc::new(cause),
            close: Vec::new(), observation: Vec::new(), trace: Vec::new() }))
    }
    /// Refuse malformed/nonconfigured fixture data without echoing its contents.
    fn refuse(stage: &'static str, source: &'static str) -> Self {
        Self::caused(stage, source, io::Error::new(io::ErrorKind::InvalidData, stage))
    }
    /// Static phase of the first failure; no sensitive path/value is rendered.
    pub fn stage(&self) -> &'static str { self.0.stage }
    /// Origin of the primary failure: saved, default, validation or observation.
    pub fn origin(&self) -> &'static str { self.0.source }
    /// Every explicit close failure with its phase, former fd, uncertainty and
    /// original cause. An uncertain descriptor is never retried or silently adopted.
    pub fn close_causes(&self) -> impl Iterator<Item = (&'static str, i32, bool, &(dyn Error + Send + Sync + 'static))> + '_ {
        self.0.close.iter().map(|c| (c.phase, c.descriptor, c.ownership_uncertain, c.cause.as_ref()))
    }
    /// Secondary observation/classification failures retained with the primary,
    /// including an uncertain-absence stat. Original causes must be rendered safely.
    pub fn observation_causes(&self) -> impl Iterator<Item = &(dyn Error + Send + Sync + 'static)> + '_ {
        self.0.observation.iter().map(|c| c.as_ref())
    }
    /// Compare the retained refusal generation by shared ownership identity.
    /// Equal text/errno/inode alone never establishes the same resolution attempt.
    pub fn same_resolution(&self, other: &Self) -> bool { Arc::ptr_eq(&self.0, &other.0) }
    /// Same-access classifications and identities retained on refusal. Returns
    /// static phase/source and inode pairs only, never invalid settings text.
    pub fn access_observations(&self) -> impl Iterator<Item = (&'static str, &'static str, Option<(u64, u64)>)> + '_ {
        self.0.trace.iter().map(|e| (e.phase, e.source, e.identity))
    }
}
impl fmt::Display for FixtureAreaError {
    /// Render safe stages; arbitrary original cause text stays out of public output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fixture-area: {} ({})", self.0.stage, self.0.source)
    }
}
impl fmt::Debug for FixtureAreaError {
    /// Keep Debug as safe as Display, including when a panic hook prints a value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self, f) }
}
impl Error for FixtureAreaError {
    /// Retain the original primary cause; secondary branches use explicit accessors.
    fn source(&self) -> Option<&(dyn Error + 'static)> { Some(self.0.primary.as_ref()) }
}

/// Successful immutable generation, including source/base and same-access events.
/// The Path is not a lease on the directory and does not authorize allocation.
#[derive(Clone, Debug)]
pub(super) struct ResolvedArea {
    pub path: PathBuf,
    pub base: PathBuf,
    pub source: &'static str,
    pub trace: Vec<AccessEvent>,
}

/// Per-call receipt: whether this caller performed the sole resolution, plus the
/// retained complete outcome. Success Arc identity and same_resolution on refusals
/// distinguish a generation without a clock/counter, author or authenticity claim.
/// Cache consumers do not manufacture another access.
pub(super) struct Receipt { pub performed_access: bool, pub outcome: Outcome }

/// One owned context fixes the first complete result (including failures).
/// OnceLock serializes initialization; no initializer IO runs after a stored
/// refusal. A new context/process, rather than resetting this cell, permits adoption.
pub(super) struct Context { base: PathBuf, outcome: OnceLock<Outcome> }
impl Context {
    /// Create an empty owned context, without IO, allocation of fixture artifacts
    /// or interaction with the process-global context.
    pub fn new(base: PathBuf) -> Self { Self { base, outcome: OnceLock::new() } }
    /// Resolve once with inert hooks and return an explicit access/cache receipt.
    /// Errors are cloned by shared ownership, preserving all original causes.
    pub fn resolve(&self) -> Receipt { self.resolve_with(&mut NoHooks) }
    /// Invoke hooks only for the one winning initializer. Other callers obtain
    /// the exact retained result; hooks cannot trigger a cache reset or retry.
    pub fn resolve_with(&self, hooks: &mut impl Hooks) -> Receipt {
        let performed = Cell::new(false);
        let outcome = self.outcome.get_or_init(|| {
            performed.set(true);
            resolve_once(&self.base, hooks)
        }).clone();
        Receipt { performed_access: performed.get(), outcome }
    }
}

/// Fixture-only fault/observation seams; callbacks must return errors rather than
/// panic or recursively enter this Context. Unexpected panic/abort is incomplete,
/// not a retained outcome or proof of observed cleanup. Production uses NoHooks.
/// Errors from
/// observers are deferred; read/parent errors remain primary. A close hook runs
/// only after real release, so its failure is explicitly a released diagnostic,
/// never evidence of an actual kernel close failure.
pub(super) trait Hooks {
    /// Observe an event from this exact access, without a reconstruction read.
    fn observe(&mut self, _event: &AccessEvent) -> io::Result<()> { Ok(()) }
    /// Allow a controlled parent replacement after pinning and before leaf open.
    fn parent_opened(&mut self, _parent: &File) -> io::Result<()> { Ok(()) }
    /// Inject a fixture read refusal only after the actual opened type is checked.
    fn before_read(&mut self, _source: &'static str, _file: &File) -> io::Result<()> { Ok(()) }
    /// Add a synthetic close diagnostic after the actual descriptor was released.
    fn after_release(&mut self, _phase: &'static str) -> io::Result<()> { Ok(()) }
}
pub(super) struct NoHooks;
impl Hooks for NoHooks {}

struct Access<'a, H: Hooks> { hooks: &'a mut H, trace: Vec<AccessEvent>, observer_errors: Vec<Cause> }
impl<H: Hooks> Access<'_, H> {
    /// Construct an event, invoke the observer and retain any returned error,
    /// then append the event to the trace. Returned errors do not replace a
    /// primary resolution failure. Panic/reentry is outside the hook contract;
    /// a panicking callback does not guarantee that this event was appended.
    fn emit(&mut self, phase: &'static str, source: &'static str, identity: Option<(u64,u64)>, hash: Option<String>) {
        let event = AccessEvent { phase, source, identity, consumed_sha256: hash };
        if let Err(error) = self.hooks.observe(&event) { self.observer_errors.push(Arc::new(error)); }
        self.trace.push(event);
    }
    /// Consume sole descriptor ownership, try close exactly once and retain every
    /// failure. Kernel close failure leaves resource ownership uncertain; no Drop
    /// or close retry can target a recycled descriptor. Synthetic hooks run only
    /// on successful real release and are labeled as known released.
    fn held<T>(&mut self, mut file: File, source: &'static str, close_phase: &'static str,
        read: impl FnOnce(&mut Self, &mut File) -> Result<T, FixtureAreaError>) -> Result<T, FixtureAreaError> {
        let result = read(self, &mut file);
        let fd = file.into_raw_fd();
        let closing = match nix::unistd::close(fd) {
            Ok(()) => self.hooks.after_release(close_phase).map_err(|e| (e, false)),
            Err(error) => Err((io::Error::from(error), true)),
        };
        self.emit(if closing.is_ok() { "closed" } else { "close-failed" }, source, None, None);
        match (result, closing) {
            (value, Ok(())) => value,
            (Ok(_), Err((cause, uncertain))) => {
                let mut error = FixtureAreaError::caused(close_phase, source, cause);
                let primary = error.0.primary.clone();
                Arc::make_mut(&mut error.0).close.push(CloseIssue { phase: close_phase, descriptor: fd, cause: primary, ownership_uncertain: uncertain });
                Err(error)
            }
            (Err(mut error), Err((cause, uncertain))) => {
                Arc::make_mut(&mut error.0).close.push(CloseIssue { phase: close_phase, descriptor: fd, cause: Arc::new(cause), ownership_uncertain: uncertain });
                Err(error)
            }
        }
    }
    /// Check actual entry absence after failed open relative to the same held
    /// parent. A remaining dangling link or a failed stat preserves refusal.
    /// Entry creation after the absent observation belongs to a later access.
    fn absent(&mut self, parent: &File, name: &str, original: rustix::io::Errno) -> Result<bool, FixtureAreaError> {
        if original != rustix::io::Errno::NOENT {
            return Err(FixtureAreaError::caused("saved-open", "saved", io::Error::from(original)));
        }
        match statat(parent, name, AtFlags::SYMLINK_NOFOLLOW) {
            Err(error) if error == rustix::io::Errno::NOENT => {
                self.emit("absent", "saved", None, None);
                Ok(true)
            }
            Ok(_) => Err(FixtureAreaError::caused("saved-open-entry-present", "saved", io::Error::from(original))),
            Err(error) => {
                let mut refused = FixtureAreaError::caused("saved-absence-uncertain", "saved", io::Error::from(original));
                // The second classification error is not a close failure.
                Arc::make_mut(&mut refused.0).observation.push(Arc::new(io::Error::from(error)));
                Err(refused)
            }
        }
    }
    /// Read one opened regular file, retaining descriptor identity and hash of
    /// exactly the consumed bytes before JSON decoding. NONBLOCK avoids FIFO
    /// waiting at open; special files refuse before any content read. Mutable
    /// content may still change while read_to_end consumes it: no atomic snapshot.
    fn content(&mut self, root: &File, name: &str, source: &'static str) -> Result<Vec<u8>, FixtureAreaError> {
        let fd = openat(root, name, OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC, Mode::empty())
            .map_err(|e| FixtureAreaError::caused("content-open", source, io::Error::from(e)))?;
        self.open_content(File::from(fd), source)
    }
    /// Own/inspect/read/close an already opened descriptor. Every return path
    /// after ownership passes through held, including metadata/read/type failures.
    fn open_content(&mut self, file: File, source: &'static str) -> Result<Vec<u8>, FixtureAreaError> {
        self.held(file, source, "content-close", |access, file| {
            let metadata = file.metadata().map_err(|e| FixtureAreaError::caused("content-metadata", source, e))?;
            access.emit("opened", source, Some((metadata.dev(), metadata.ino())), None);
            if !metadata.is_file() { return Err(FixtureAreaError::refuse("regular-file-required", source)); }
            access.hooks.before_read(source, file).map_err(|e| FixtureAreaError::caused("content-read", source, e))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).map_err(|e| FixtureAreaError::caused("content-read", source, e))?;
            access.emit("consumed", source, Some((metadata.dev(),metadata.ino())), Some(format!("{:x}",Sha256::digest(&bytes))));
            Ok(bytes)
        })
    }
    /// Pin the saved parent, distinguish entry absence, then consume one leaf.
    /// Existing symlinks remain allowed. O_PATH does not add directory-listing
    /// permission to the traversal contract. No config or directory is created.
    fn saved(&mut self, root: &File) -> Result<Option<Vec<u8>>, FixtureAreaError> {
        let parent = match openat(root, ".swarm", OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC, Mode::empty()) {
            Ok(fd) => File::from(fd),
            Err(error) => { self.absent(root,".swarm",error)?; return Ok(None); }
        };
        self.held(parent, "saved", "saved-parent-close", |access,parent| {
            let metadata = parent.metadata().map_err(|e| FixtureAreaError::caused("saved-parent-metadata","saved",e))?;
            access.emit("parent-opened","saved",Some((metadata.dev(),metadata.ino())),None);
            access.hooks.parent_opened(parent).map_err(|e| FixtureAreaError::caused("saved-parent-observation","saved",e))?;
            let fd = match openat(&*parent,"engine-settings.json",OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,Mode::empty()) {
                Ok(fd) => fd,
                Err(error) => { access.absent(parent,"engine-settings.json",error)?; return Ok(None); }
            };
            access.open_content(File::from(fd),"saved").map(Some)
        })
    }
}

/// Decode the fixture document shape only, not the entire product document.
/// Missing/null saved field is not configured; other invalid types are errors.
/// No normalization, silent migration, metadata validation or config write occurs.
fn field(bytes: &[u8], source: &'static str) -> Result<String, FixtureAreaError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| FixtureAreaError::caused("settings-json",source,e))?;
    let document = value.as_object().ok_or_else(|| FixtureAreaError::refuse("settings-document-object",source))?;
    let values = if source == "saved" {
        match document.get("values") {
            None => return Err(FixtureAreaError::refuse("settings-runtime-temporary-not-configured",source)),
            Some(value) => value.as_object()
                .ok_or_else(|| FixtureAreaError::refuse("settings-values-object",source))?,
        }
    } else { document };
    match values.get("runtime_temporary_directory") {
        None | Some(serde_json::Value::Null) => Err(FixtureAreaError::refuse("settings-runtime-temporary-not-configured",source)),
        Some(serde_json::Value::String(raw)) => Ok(raw.clone()),
        Some(_) => Err(FixtureAreaError::refuse("settings-runtime-temporary-type",source)),
    }
}

/// Resolve a single access, preserving primary and all cleanup/observer causes.
/// The embedded installation profile remains an input of the existing validator,
/// distinct from the shipped file read only after proven saved absence. Hooks are
/// fixture seams, not authorization. Root/parent fd lifetimes end before return.
fn resolve_once(base: &Path, hooks: &mut impl Hooks) -> Outcome {
    let mut access = Access { hooks, trace: Vec::new(), observer_errors: Vec::new() };
    let result = (|| {
        let fd = open(base,OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,Mode::empty())
            .map_err(|e| FixtureAreaError::caused("repository-open","saved",io::Error::from(e)))?;
        access.held(File::from(fd),"saved","repository-close",|access,root| {
            let metadata = root.metadata().map_err(|e| FixtureAreaError::caused("repository-metadata","saved",e))?;
            access.emit("repository-opened","saved",Some((metadata.dev(),metadata.ino())),None);
            let (bytes,source) = match access.saved(root)? {
                Some(bytes) => (bytes,"saved"),
                None => (access.content(root,"rust/crates/swarm-engine/config/engine-settings.json","default")?,"default"),
            };
            let raw = field(&bytes,source)?;
            let mut probe = EngineSettingsValues::installation_defaults()
                .map_err(|e| FixtureAreaError::caused("embedded-validation-profile","validation",e))?;
            probe.runtime_temporary_directory = Some(raw);
            let validated = probe.runtime_temporary_directory()
                .map_err(|e| FixtureAreaError::caused("settings-path","validation",e))?;
            let path = PathBuf::from(validated);
            access.emit("resolved",source,None,None);
            Ok(ResolvedArea { path: if path.is_absolute() {path} else {base.join(path)},
                base: base.to_path_buf(), source, trace: Vec::new() })
        })
    })();
    let result = match result {
        Err(error) => Err(error),
        Ok(value) if access.observer_errors.is_empty() => Ok(value),
        Ok(_) => {
            let mut error = FixtureAreaError::refuse("resolution-observation","observation");
            Arc::make_mut(&mut error.0).primary = access.observer_errors[0].clone();
            Err(error)
        }
    };
    match result {
        Ok(mut value) => { value.trace = access.trace; Ok(Arc::new(value)) }
        Err(mut error) => {
            let failure = Arc::make_mut(&mut error.0);
            failure.trace = access.trace;
            failure.observation.extend(access.observer_errors);
            Err(error)
        }
    }
}
