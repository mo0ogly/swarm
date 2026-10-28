//! Native environment observations. None authenticates a human, authorizes
//! execution or establishes monotonic time or freshness of acceptance evidence.
use crate::{text::trim_space, GitState};
#[cfg(not(target_os = "linux"))]
use std::process::Command;
use std::{
    error::Error,
    fmt, io,
    path::Path,
    process::ExitStatus,
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeOperation {
    Entropy,
    Clock,
    Hostname,
    Output,
    Platform,
}
pub struct RuntimeError {
    operation: RuntimeOperation,
    cause: Box<dyn Error + Send + Sync>,
}
impl RuntimeError {
    pub fn new(operation: RuntimeOperation, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation,
            cause: Box::new(cause),
        }
    }
    pub fn operation(&self) -> RuntimeOperation {
        self.operation
    }
    pub fn code(&self) -> &'static str {
        match self.operation {
            RuntimeOperation::Entropy => "runtime-entropy",
            RuntimeOperation::Clock => "runtime-clock",
            RuntimeOperation::Hostname => "runtime-hostname",
            RuntimeOperation::Output => "runtime-output",
            RuntimeOperation::Platform => "runtime-platform",
        }
    }
}
impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for RuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}
/// Go's 12 random bytes and lowercase hex are an ID representation, not a
/// configurable attempt or execution ceiling. Prefix is caller-owned data.
pub fn identifier_from_entropy(prefix: &str, bytes: &[u8; 12]) -> String {
    let mut out = String::with_capacity(prefix.len() + 24);
    out.push_str(prefix);
    for b in bytes {
        out.push(char::from(b"0123456789abcdef"[usize::from(b >> 4)]));
        out.push(char::from(b"0123456789abcdef"[usize::from(b & 15)]));
    }
    out
}
pub fn generate_identifier(prefix: &str) -> Result<String, RuntimeError> {
    let mut bytes = [0u8; 12];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| RuntimeError::new(RuntimeOperation::Entropy, e))?;
    Ok(identifier_from_entropy(prefix, &bytes))
}
fn format_datetime(date: time::OffsetDateTime) -> String {
    let year = if date.year() < 0 {
        format!("-{:04}", date.year().unsigned_abs())
    } else {
        format!("{:04}", date.year())
    };
    let mut value = format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}",
        u8::from(date.month()),
        date.day(),
        date.hour(),
        date.minute(),
        date.second()
    );
    if date.nanosecond() != 0 {
        value.push('.');
        value.push_str(format!("{:09}", date.nanosecond()).trim_end_matches('0'));
    }
    value.push('Z');
    value
}
/// Like time.Unix(seconds,nanos).UTC().Format(RFC3339Nano), within time's
/// representable date range. Nanos may require normalization, including negatives.
pub fn format_utc_timestamp(seconds: i64, nanos: i64) -> Result<String, RuntimeError> {
    time::OffsetDateTime::from_unix_timestamp_nanos(
        i128::from(seconds) * 1_000_000_000 + i128::from(nanos),
    )
    .map(format_datetime)
    .map_err(|e| RuntimeError::new(RuntimeOperation::Clock, e))
}
pub fn system_timestamp() -> Result<String, RuntimeError> {
    let now = SystemTime::now();
    let nanos = match now.duration_since(UNIX_EPOCH) {
        Ok(d) => i128::try_from(d.as_nanos())
            .map_err(|e| RuntimeError::new(RuntimeOperation::Clock, e))?,
        Err(e) => -i128::try_from(e.duration().as_nanos())
            .map_err(|e| RuntimeError::new(RuntimeOperation::Clock, e))?,
    };
    time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .map(format_datetime)
        .map_err(|e| RuntimeError::new(RuntimeOperation::Clock, e))
}
/// Go JSON substitutes one U+FFFD per invalid byte, unlike from_utf8_lossy for
/// some truncated sequences. Quote arbitrary command output, then use its codec.
pub fn go_command_text(raw: &[u8]) -> Result<String, RuntimeError> {
    go_text(raw).map(|s| trim_space(&s).into())
}
pub(crate) fn go_text(raw: &[u8]) -> Result<String, RuntimeError> {
    let mut json = Vec::with_capacity(raw.len() + 2);
    json.push(b'"');
    for b in raw {
        match b {
            b'"' | b'\\' => {
                json.push(b'\\');
                json.push(*b)
            }
            0..=31 => json.extend_from_slice(format!("\\u{b:04x}").as_bytes()),
            _ => json.push(*b),
        }
    }
    json.push(b'"');
    swarm_wire_json::decode::<String>(&json)
        .map_err(|e| RuntimeError::new(RuntimeOperation::Output, e))
}
pub fn operator_identity() -> Result<String, RuntimeError> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let host = nix::unistd::gethostname()
            .map_err(|e| RuntimeError::new(RuntimeOperation::Hostname, e))?;
        // No TrimSpace: hostnames are attributed verbatim by the actual Go helper.
        let host = go_text(host.as_os_str().as_bytes())?;
        Ok(format!("{host}/uid:{}", nix::unistd::getuid().as_raw()))
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::new(
            RuntimeOperation::Platform,
            io::Error::new(
                io::ErrorKind::Unsupported,
                "operator identity requires Unix",
            ),
        ))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitField {
    Head,
    Branch,
    Changes,
}
pub enum GitIssueCause {
    Spawn(io::Error),
    #[cfg(target_os = "linux")]
    CheckedMetadata(Box<crate::GitReportError>),
    Policy(&'static str),
    Exit {
        status: ExitStatus,
        stderr: Vec<u8>,
    },
}
pub struct GitIssue {
    pub field: GitField,
    pub cause: GitIssueCause,
}
impl fmt::Display for GitIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "git observation unavailable: {:?}", self.field)
    }
}
impl fmt::Debug for GitIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for GitIssue {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.cause {
            GitIssueCause::Spawn(e) => Some(e),
            #[cfg(target_os = "linux")]
            GitIssueCause::CheckedMetadata(e) => Some(e.as_ref()),
            GitIssueCause::Policy(_) => None,
            GitIssueCause::Exit { .. } => None,
        }
    }
}
/// Optional recorded display state. Failures remain explicit per field, and must
/// be reported by public consumers. Empty changes alone never attests cleanliness.
pub struct GitObservation {
    pub state: GitState,
    pub issues: Vec<GitIssue>,
}
impl fmt::Debug for GitObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitObservation")
            .field("issue_count", &self.issues.len())
            .finish_non_exhaustive()
    }
}
/// Legacy lower-level observation with explicitly unbounded process lifetime.
/// Linux uses the same filter-safe checked reader as Root; authoritative Root
/// operations use git_with_settings instead. No actor authentication is implied.
#[cfg(target_os = "linux")]
pub fn observe_git(root: &Path) -> Result<GitObservation, RuntimeError> {
    crate::git_metadata::observe_git_commands(root, None, crate::SettingCount(0))
        .map(crate::git_metadata::into_environment)
        .map_err(|e| RuntimeError::new(RuntimeOperation::Output, e))
}
#[cfg(not(target_os = "linux"))]
pub fn observe_git(root: &Path) -> Result<GitObservation, RuntimeError> {
    let mut result = GitObservation {
        state: GitState::default(),
        issues: vec![],
    };
    for (field, args) in [
        (GitField::Head, vec!["rev-parse", "HEAD"]),
        (GitField::Branch, vec!["branch", "--show-current"]),
        (
            GitField::Changes,
            vec!["status", "--porcelain", "--untracked-files=normal"],
        ),
    ] {
        let output = Command::new("git").arg("-C").arg(root).args(args).output();
        let value = match output {
            Ok(o) if o.status.success() => go_command_text(&o.stdout)?,
            Ok(o) => {
                result.issues.push(GitIssue {
                    field,
                    cause: GitIssueCause::Exit {
                        status: o.status,
                        stderr: o.stderr,
                    },
                });
                String::new()
            }
            Err(e) => {
                result.issues.push(GitIssue {
                    field,
                    cause: GitIssueCause::Spawn(e),
                });
                String::new()
            }
        };
        match field {
            GitField::Head => result.state.head = value,
            GitField::Branch => result.state.branch = value,
            GitField::Changes => result.state.changes = value,
        }
    }
    Ok(result)
}
pub trait WorkEnvironment {
    fn identifier(&mut self, prefix: &str) -> Result<String, RuntimeError>;
    fn timestamp(&mut self) -> Result<String, RuntimeError>;
    fn operator(&mut self) -> Result<String, RuntimeError>;
    fn git(&mut self, root: &Path) -> Result<GitObservation, RuntimeError>;
    /// Root's current policy boundary. Supplied environments remain explicit
    /// development observations; SystemEnvironment consumes the real journal.
    fn git_with_settings(
        &mut self,
        root: &Path,
        _settings: &crate::EngineSettingsDocument,
    ) -> Result<GitObservation, RuntimeError> {
        self.git(root)
    }
}
#[derive(Default)]
pub struct SystemEnvironment;
impl WorkEnvironment for SystemEnvironment {
    fn identifier(&mut self, prefix: &str) -> Result<String, RuntimeError> {
        generate_identifier(prefix)
    }
    fn timestamp(&mut self) -> Result<String, RuntimeError> {
        system_timestamp()
    }
    fn operator(&mut self) -> Result<String, RuntimeError> {
        operator_identity()
    }
    fn git(&mut self, root: &Path) -> Result<GitObservation, RuntimeError> {
        observe_git(root)
    }
    #[cfg(target_os = "linux")]
    fn git_with_settings(
        &mut self,
        root: &Path,
        settings: &crate::EngineSettingsDocument,
    ) -> Result<GitObservation, RuntimeError> {
        settings
            .validate()
            .map_err(|e| RuntimeError::new(RuntimeOperation::Output, e))?;
        if settings.values.git_command_timeout_seconds.is_none() {
            // Legacy optional display metadata stays explicitly unavailable;
            // never execute outside policy or invent a fallback deadline.
            return Ok(GitObservation {
                state: GitState::default(),
                issues: [GitField::Head, GitField::Branch, GitField::Changes]
                    .into_iter()
                    .map(|field| GitIssue {
                        field,
                        cause: GitIssueCause::Policy("settings-git-policy-not-configured"),
                    })
                    .collect(),
            });
        }
        crate::observe_git_metadata(root, settings)
            .map(crate::git_metadata::into_environment)
            .map_err(|e| RuntimeError::new(RuntimeOperation::Output, e))
    }
}

/// Bind Git observations during one Root mutation attempt to its locked settings.
/// Other environment inputs are delegated unchanged; no authority is inferred.
pub(crate) struct PolicyEnvironment<'a, E: WorkEnvironment> {
    pub inner: &'a mut E,
    pub settings: &'a crate::EngineSettingsDocument,
}
impl<E: WorkEnvironment> WorkEnvironment for PolicyEnvironment<'_, E> {
    fn identifier(&mut self, prefix: &str) -> Result<String, RuntimeError> {
        self.inner.identifier(prefix)
    }
    fn timestamp(&mut self) -> Result<String, RuntimeError> {
        self.inner.timestamp()
    }
    fn operator(&mut self) -> Result<String, RuntimeError> {
        self.inner.operator()
    }
    fn git(&mut self, root: &Path) -> Result<GitObservation, RuntimeError> {
        self.inner.git_with_settings(root, self.settings)
    }
}
