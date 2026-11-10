//! Linux process registration observations, not authenticated agent authority.
#![cfg(target_os = "linux")]
use std::{
    error::Error,
    fmt,
    fs::File,
    io::{self, Read},
    os::{fd::IntoRawFd, unix::ffi::OsStrExt},
};

/// Safe classification with original kernel/IO/UTF8 cause and checked cleanup.
/// Neither a successful observation nor a stored stamp grants execution rights.
pub struct SupervisorIdentityError {
    operation: &'static str,
    cause: Box<dyn Error + Send + Sync>,
    cleanup: Option<io::Error>,
}
impl SupervisorIdentityError {
    fn new(operation: &'static str, cause: impl Error + Send + Sync + 'static) -> Self {
        Self {
            operation,
            cause: Box::new(cause),
            cleanup: None,
        }
    }
    /// Stable diagnostic excludes host identifiers and private kernel contents.
    pub fn code(&self) -> &'static str {
        "supervisor-identity-unavailable"
    }
    /// Classify a typed missing/zombie observation for completion replay. A
    /// concurrent cleanup failure must keep the observation unresolved; this
    /// predicate grants no process signal, launch or task acceptance.
    pub(crate) fn confirms_absence(&self) -> bool {
        self.cleanup.is_none()
            && self
                .source()
                .and_then(|e| e.downcast_ref::<io::Error>())
                .is_some_and(|e| e.kind() == io::ErrorKind::NotFound)
    }
    /// Failing observation step, never a process-signalling capability.
    pub fn operation(&self) -> &'static str {
        self.operation
    }
    /// A close failure accompanying an original read error, if one occurred.
    pub fn cleanup(&self) -> Option<&io::Error> {
        self.cleanup.as_ref()
    }
}
impl fmt::Display for SupervisorIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.operation)
    }
}
impl fmt::Debug for SupervisorIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for SupervisorIdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

/// Read one fixed Linux procfs identity source and explicitly close its handle.
/// Read/close/UTF8 failures retain causes; never substitute an empty identity.
pub(crate) fn proc_text(path: &str, operation: &'static str) -> Result<String, SupervisorIdentityError> {
    let mut file = File::open(path).map_err(|e| SupervisorIdentityError::new(operation, e))?;
    let mut bytes = Vec::new();
    let result = file
        .read_to_end(&mut bytes)
        .map_err(|e| SupervisorIdentityError::new(operation, e));
    let closed = nix::unistd::close(file.into_raw_fd()).map_err(io::Error::from);
    match (result, closed) {
        (Ok(_), Ok(())) => {
            String::from_utf8(bytes).map_err(|e| SupervisorIdentityError::new(operation, e))
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(SupervisorIdentityError::new("close-proc-identity", error)),
        (Err(mut primary), Err(cleanup)) => {
            primary.cleanup = Some(cleanup);
            Err(primary)
        }
    }
}

/// Parse Linux `/proc/PID/stat` starttime after the last closing comm parenthesis.
/// Spaces/parentheses in comm do not shift fields. A zombie yields None as in Go;
/// missing/invalid starttime refuses rather than treating malformed input as dead.
/// Field22 is a kernel ABI position, not an operator limit. No IO or authority.
pub fn parse_process_start_stamp(raw: &str) -> Result<Option<String>, SupervisorIdentityError> {
    let invalid = || {
        SupervisorIdentityError::new(
            "parse-process-stat",
            io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid process stat representation",
            ),
        )
    };
    let (_, tail) = raw.rsplit_once(')').ok_or_else(invalid)?;
    let fields: Vec<_> = tail.split_ascii_whitespace().collect();
    let state = fields.first().ok_or_else(invalid)?;
    let stamp = fields.get(19).ok_or_else(invalid)?;
    if state.len() != 1 || stamp.is_empty() || !stamp.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    if *state == "Z" {
        return Ok(None);
    }
    Ok(Some((*stamp).into()))
}

/// A registration for this real Linux process. Fields cannot be client supplied;
/// the constructor observes PID/starttime and host/boot/PID namespace. This is
/// PID-reuse protection only, not cryptographic agent identity or launch consent.
pub struct SupervisorIdentity {
    pub(crate) pid: i64,
    pub(crate) stamp: String,
    pub(crate) host: String,
}
impl SupervisorIdentity {
    /// Observe the current process and Linux host tuple, preserving any failure.
    /// No fallback identity, PID signal, reservation or provider execution occurs.
    pub fn current() -> Result<Self, SupervisorIdentityError> {
        let hostname =
            nix::unistd::gethostname().map_err(|e| SupervisorIdentityError::new("hostname", e))?;
        let hostname = std::str::from_utf8(hostname.as_bytes())
            .map_err(|e| SupervisorIdentityError::new("hostname-utf8", e))?;
        let boot = proc_text("/proc/sys/kernel/random/boot_id", "boot-id")?;
        let boot = crate::text::trim_space(&boot);
        let namespace = std::fs::read_link("/proc/self/ns/pid")
            .map_err(|e| SupervisorIdentityError::new("pid-namespace", e))?;
        let namespace = std::str::from_utf8(namespace.as_os_str().as_bytes())
            .map_err(|e| SupervisorIdentityError::new("pid-namespace-utf8", e))?;
        if hostname.is_empty() || boot.is_empty() || namespace.is_empty() {
            return Err(SupervisorIdentityError::new(
                "host-identity",
                io::Error::new(io::ErrorKind::InvalidData, "incomplete host identity"),
            ));
        }
        let stamp = parse_process_start_stamp(&proc_text("/proc/self/stat", "process-stat")?)?
            .ok_or_else(|| {
                SupervisorIdentityError::new(
                    "process-not-live",
                    io::Error::new(io::ErrorKind::NotFound, "current process is not live"),
                )
            })?;
        Ok(Self {
            pid: i64::from(std::process::id()),
            stamp,
            host: format!("{hostname}:{boot}:{namespace}"),
        })
    }
    /// Observed current PID; never use this integer alone to signal a process.
    pub fn pid(&self) -> i64 {
        self.pid
    }
    /// Kernel starttime paired with PID; not an agent authentication token.
    pub fn process_stamp(&self) -> &str {
        &self.stamp
    }
    /// Go-compatible host:boot:PID-namespace tuple, not a trusted actor name.
    pub fn host(&self) -> &str {
        &self.host
    }
}
impl fmt::Debug for SupervisorIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SupervisorIdentity { observation: redacted }")
    }
}

/// One Linux procfs observation, not a process lease, signal capability or
/// authenticated agent identity. Private fields prevent JSON construction; a
/// stored/cloned observation may become stale immediately after its read.
pub struct LinuxProcessObservation {
    pid: u32,
    parent: u32,
    group: u32,
    stamp: String,
    state: char,
}
impl LinuxProcessObservation {
    /// Observed PID, never sufficient alone to authorize a signal.
    pub fn pid(&self) -> u32 {
        self.pid
    }
    /// Observed kernel parent PID; caller must establish its own current identity.
    pub fn parent_pid(&self) -> u32 {
        self.parent
    }
    /// Observed process group ID; no ownership or right to kill the group implied.
    pub fn process_group_id(&self) -> u32 {
        self.group
    }
    /// Starttime pairs with PID to detect reuse, not actor impersonation.
    pub fn process_stamp(&self) -> &str {
        &self.stamp
    }
    /// Actual live kernel state at the read; stopped/sleeping does not mean progress.
    pub fn state(&self) -> char {
        self.state
    }
}
impl fmt::Debug for LinuxProcessObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LinuxProcessObservation { kernel data: redacted }")
    }
}
/// Read PID/parent/group/starttime/state from one actual Linux `/proc/PID/stat`.
/// Numeric PID and kernel ABI indices are representation rules, not operator
/// policy. Invalid/missing/closed proc data and zombies/dead states refuse with
/// typed causes; no empty stamp or claimed health is substituted. Reads/close
/// only: no process, signal, SQL mutation, execution permission or authentication.
/// Callers performing effects must hold the appropriate live process capability
/// and recheck their ownership/freshness; this observation alone grants none.
pub fn observe_linux_process(pid: u32) -> Result<LinuxProcessObservation, SupervisorIdentityError> {
    if pid == 0 || i32::try_from(pid).is_err() {
        return Err(SupervisorIdentityError::new(
            "process-pid-representation",
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid Linux PID representation",
            ),
        ));
    }
    let raw = proc_text(&format!("/proc/{pid}/stat"), "observed-process-stat")?;
    let stamp = parse_process_start_stamp(&raw)?.ok_or_else(|| {
        SupervisorIdentityError::new(
            "observed-process-not-live",
            io::Error::new(io::ErrorKind::NotFound, "observed process is not live"),
        )
    })?;
    let invalid = || {
        SupervisorIdentityError::new(
            "observed-process-structure",
            io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid process identity structure",
            ),
        )
    };
    let (_, tail) = raw.rsplit_once(')').ok_or_else(invalid)?;
    let fields: Vec<_> = tail.split_ascii_whitespace().collect();
    let state = fields.first().ok_or_else(invalid)?;
    if state.len() != 1 || matches!(*state, "Z" | "X" | "x") {
        return Err(invalid());
    }
    let number = |at: usize| -> Result<u32, SupervisorIdentityError> {
        let value = fields.get(at).ok_or_else(invalid)?;
        let parsed = value
            .parse::<u32>()
            .map_err(|e| SupervisorIdentityError::new("observed-process-identifier", e))?;
        if i32::try_from(parsed).is_err() {
            return Err(invalid());
        }
        Ok(parsed)
    };
    Ok(LinuxProcessObservation {
        pid,
        parent: number(1)?,
        group: number(2)?,
        stamp,
        state: state.chars().next().ok_or_else(invalid)?,
    })
}

/// Arm the current owned helper's Linux creator-parent death signal before
/// provider permission is consumed. SIGKILL preserves the existing no-orphan
/// leader lifetime contract; it is not an operator duration/grace policy. Read
/// the actual kernel parent before/after arming and refuse a raced identity.
/// A raced relationship is unknown/invalid data, not canonical parent absence.
/// Kernel/procfs errors preserve typed causes; no supplied PID/actor is trusted.
/// This changes only this process's death signal and does not roll it back on
/// refusal. The gate must still verify its registered parent/child/permission.
/// Linux ties this to the creator task; ordinary exec preserves it, but credential
/// changes can clear it. No descendant termination, sandbox, billing settlement
/// or crash recovery is established by this point observation.
pub(crate) fn arm_parent_death_signal() -> Result<(), SupervisorIdentityError> {
    let parent = u32::try_from(nix::unistd::getppid().as_raw())
        .map_err(|e| SupervisorIdentityError::new("parent-pid-representation", e))?;
    let before = observe_linux_process(parent)?;
    nix::sys::prctl::set_pdeathsig(nix::sys::signal::Signal::SIGKILL)
        .map_err(|e| SupervisorIdentityError::new("parent-death-arm", e))?;
    let current = u32::try_from(nix::unistd::getppid().as_raw())
        .map_err(|e| SupervisorIdentityError::new("parent-pid-representation", e))?;
    if current != parent {
        return Err(SupervisorIdentityError::new(
            "parent-death-raced",
            io::Error::new(io::ErrorKind::InvalidData, "parent changed while arming"),
        ));
    }
    let after = observe_linux_process(current)?;
    if before.process_stamp() != after.process_stamp() {
        return Err(SupervisorIdentityError::new(
            "parent-death-raced",
            io::Error::new(
                io::ErrorKind::InvalidData,
                "parent identity changed while arming",
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod completion_observation_tests {
    use super::*;
    /// Close failures accompanying a missing observation cannot become success;
    /// permission/invalid data are also unknown, with original causes retained.
    #[test]
    fn clean_absence_excludes_cleanup_and_other_observation_failures() {
        let mut missing = SupervisorIdentityError::new(
            "observed-process-stat",
            io::Error::new(io::ErrorKind::NotFound, "private missing observation"),
        );
        assert!(missing.confirms_absence());
        missing.cleanup = Some(io::Error::other("private close failure"));
        assert!(!missing.confirms_absence());
        assert!(missing.cleanup().is_some());
        for kind in [io::ErrorKind::PermissionDenied, io::ErrorKind::InvalidData] {
            assert!(
                !SupervisorIdentityError::new("observed-process-stat", io::Error::from(kind))
                    .confirms_absence()
            );
        }
    }
}
