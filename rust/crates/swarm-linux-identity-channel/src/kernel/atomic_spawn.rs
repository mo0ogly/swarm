//! Native GNU ABI boundary. All allocation/input preparation stays in parent;
//! only libc's own spawn implementation executes before child exec.
use crate::{
    atomic_spawn::{Cleanup, Exit, ExitKind, Spec},
    Error,
};
use std::{
    ffi::CStr,
    io,
    mem::{self, MaybeUninit},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
};
/// Full attempt, including every independent initialized-resource close failure.
/// process is installed only after native rc0 with a valid returned new FD.
pub(crate) struct Attempt {
    pub process: Option<OwnedFd>,
    pub invoked: bool,
    pub primary: Option<Error>,
    pub validation: Option<Error>,
    pub cleanup: Cleanup,
}
impl Attempt {
    /// No process/invocation inferred before the observed native step.
    fn new() -> Self {
        Self {
            process: None,
            invoked: false,
            primary: None,
            validation: None,
            cleanup: Cleanup::default(),
        }
    }
}
/// Exact GNU bits/spawn_ext.h ABI, absent locked libc wrapper. Const pointer
/// spelling differs only in Rust mutability, not C calling convention/layout.
type Spawn = unsafe extern "C" fn(
    *mut libc::c_int,
    *const libc::c_char,
    *const libc::posix_spawn_file_actions_t,
    *const libc::posix_spawnattr_t,
    *const *mut libc::c_char,
    *const *mut libc::c_char,
) -> libc::c_int;
type Chdir =
    unsafe extern "C" fn(*mut libc::posix_spawn_file_actions_t, *const libc::c_char) -> libc::c_int;
type CloseFrom =
    unsafe extern "C" fn(*mut libc::posix_spawn_file_actions_t, libc::c_int) -> libc::c_int;
struct Symbols {
    spawn: Spawn,
    chdir: Chdir,
    closefrom: CloseFrom,
}
/// Resolve a required loaded GNU ABI before any process action. No dlopen of a
/// caller path, no version/PID fallback. Preserve dlerror text only in typed IO;
/// ordinary Error formatting redacts it. Trusted runtime/loader is a precondition.
fn symbol(name: &CStr) -> Result<*mut libc::c_void, Error> {
    // SAFETY: static NUL-terminated ABI name, RTLD_DEFAULT loaded-process scope.
    // dlerror string is TLS-owned valid until next dl* call and copied immediately.
    unsafe {
        libc::dlerror();
        let p = libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr());
        let err = libc::dlerror();
        if !err.is_null() {
            let text = CStr::from_ptr(err).to_string_lossy().into_owned();
            return Err(Error::syscall(
                "atomic-spawn-symbol",
                io::Error::new(io::ErrorKind::Unsupported, text),
            ));
        }
        if p.is_null() {
            return Err(Error::refused("atomic-spawn-symbol"));
        }
        Ok(p)
    }
}
/// Convert three verified non-null symbols to their installed GNU C signatures.
/// POSIX/Linux ELF dlsym function-pointer representation is required; no integer
/// or caller-supplied pointer is ever interpreted as a function.
fn symbols() -> Result<Symbols, Error> {
    let spawn = symbol(c"pidfd_spawn")?;
    let chdir = symbol(c"posix_spawn_file_actions_addchdir_np")?;
    let closefrom = symbol(c"posix_spawn_file_actions_addclosefrom_np")?;
    // SAFETY: names/signatures are the read installed GNU declarations; pointers
    // remain valid because the process libc is never unloaded by this component.
    Ok(unsafe {
        Symbols {
            spawn: mem::transmute::<*mut libc::c_void, Spawn>(spawn),
            chdir: mem::transmute::<*mut libc::c_void, Chdir>(chdir),
            closefrom: mem::transmute::<*mut libc::c_void, CloseFrom>(closefrom),
        }
    })
}
/// POSIX spawn APIs return errno directly, not through last_os_error.
fn rc(value: libc::c_int, phase: &'static str) -> Result<(), Error> {
    if value == 0 {
        Ok(())
    } else {
        Err(Error::syscall(phase, io::Error::from_raw_os_error(value)))
    }
}
/// Allocate pointer-only arrays before launch; no secret argv/env bytes copied.
/// Checked length is a representation bound, not an operator argument ceiling.
fn pointers(values: &[std::ffi::CString]) -> Result<Vec<*mut libc::c_char>, Error> {
    let n = values
        .len()
        .checked_add(1)
        .ok_or_else(|| Error::refused("atomic-spawn-array-size"))?;
    let mut out = Vec::new();
    out.try_reserve_exact(n)
        .map_err(|e| Error::syscall("atomic-spawn-allocation", io::Error::other(e)))?;
    out.extend(values.iter().map(|s| s.as_ptr().cast_mut()));
    out.push(std::ptr::null_mut());
    Ok(out)
}
/// Check pidfs64 support before untrusted execution, using only the current
/// still-executing process (never a child PID lookup/capture fallback). Close
/// the temporary descriptor explicitly and retain independent cleanup cause.
fn support() -> Result<(), Error> {
    // SAFETY: no-pointer syscall for this running process, whose PID cannot be
    // reaped/reused while this call executes. Success returns a new unique int FD.
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::getpid(), 0 as libc::c_uint) };
    if raw < 0 {
        return Err(Error::syscall(
            "atomic-spawn-support",
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: uniquely owned fresh successful syscall return, never caller data.
    let fd = unsafe { OwnedFd::from_raw_fd(raw as libc::c_int) };
    let result = super::process_identity(fd.as_fd());
    let failed = result.is_err();
    let mut e = result
        .err()
        .unwrap_or_else(|| Error::refused("atomic-spawn-support-close"));
    e.cleanup(fd, "close-atomic-support");
    if failed || !e.causes().is_empty() {
        Err(e)
    } else {
        Ok(())
    }
}
/// Duplicate each supplied stream above reserved POSIX0..2 with CLOEXEC. This
/// prevents dup2 ordering collisions, not a configurable FD-count ceiling.
fn duplicate(fd: BorrowedFd<'_>) -> Result<OwnedFd, Error> {
    // SAFETY: valid borrowed source FD, fixed F_DUPFD_CLOEXEC and POSIX boundary3;
    // no pointer argument, kernel returns a fresh unique FD or errno.
    let raw = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
    if raw < 0 {
        return Err(Error::syscall(
            "atomic-spawn-stream-duplicate",
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: successful fcntl duplicate has unique newly returned ownership.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}
/// Close one parent duplicate exactly once; preserve actual errno, never retry.
fn close(fd: Option<OwnedFd>, phase: &'static str) -> Option<Error> {
    fd.and_then(|fd| {
        nix::unistd::close(fd)
            .err()
            .map(|e| Error::syscall(phase, io::Error::from(e)))
    })
}
/// Prepare and invoke atomically, preserving all parent cleanup and returned
/// process ownership. Only successful native rc0 establishes ownership of the
/// output FD; an error output integer must never be treated as an owned FD.
pub(crate) fn run(spec: &Spec<'_>) -> Attempt {
    let mut out = Attempt::new();
    let mut streams: [Option<OwnedFd>; 3] = [None, None, None];
    let mut actions = MaybeUninit::<libc::posix_spawn_file_actions_t>::uninit();
    let mut attrs = MaybeUninit::<libc::posix_spawnattr_t>::uninit();
    let mut actions_ready = false;
    let mut attrs_ready = false;
    let result = (|| -> Result<(), Error> {
        if spec.program.to_bytes().first() != Some(&b'/')
            || spec.directory.to_bytes().first() != Some(&b'/')
            || spec.arguments.is_empty()
        {
            return Err(Error::refused("atomic-spawn-input"));
        }
        let symbols = symbols()?;
        support()?;
        let argv = pointers(spec.arguments)?;
        let env = pointers(spec.environment)?;
        for (i, fd) in [spec.stdin, spec.stdout, spec.stderr]
            .into_iter()
            .enumerate()
        {
            streams[i] = Some(duplicate(fd)?)
        }
        // SAFETY: writable correctly aligned storage; only rc0 marks it initialized.
        rc(
            unsafe { libc::posix_spawn_file_actions_init(actions.as_mut_ptr()) },
            "atomic-spawn-actions-init",
        )?;
        actions_ready = true;
        // SAFETY: same initialized-output contract as actions, with separate custody.
        rc(
            unsafe { libc::posix_spawnattr_init(attrs.as_mut_ptr()) },
            "atomic-spawn-attributes-init",
        )?;
        attrs_ready = true;
        for (i, fd) in streams.iter().enumerate() {
            // SAFETY: initialized actions; retained distinct source duplicate and target
            // POSIX stream index0..2; library records action without closing parent FDs.
            rc(
                unsafe {
                    libc::posix_spawn_file_actions_adddup2(
                        actions.as_mut_ptr(),
                        fd.as_ref().expect("created stream").as_raw_fd(),
                        i as libc::c_int,
                    )
                },
                "atomic-spawn-stream-action",
            )?;
        }
        // SAFETY: initialized actions and retained NUL cwd. Only child's cwd changes.
        rc(
            unsafe { (symbols.chdir)(actions.as_mut_ptr(), spec.directory.as_ptr()) },
            "atomic-spawn-directory-action",
        )?;
        // SAFETY: initialized actions, fixed boundary3 after stdio duplication; close
        // applies only in child and includes every unrelated inherited descriptor.
        rc(
            unsafe { (symbols.closefrom)(actions.as_mut_ptr(), 3) },
            "atomic-spawn-closefrom-action",
        )?;
        // SAFETY: initialized attributes and borrowed immutable native sigset_t values;
        // only explicit child attributes change, never current process signal state.
        rc(
            unsafe {
                libc::posix_spawnattr_setsigmask(attrs.as_mut_ptr(), spec.signal_mask.as_ref())
            },
            "atomic-spawn-signal-mask",
        )?;
        // SAFETY: same native sigset lifetime/layout, explicit caller defaults.
        rc(
            unsafe {
                libc::posix_spawnattr_setsigdefault(
                    attrs.as_mut_ptr(),
                    spec.signal_defaults.as_ref(),
                )
            },
            "atomic-spawn-signal-defaults",
        )?;
        // SAFETY: initialized attrs; fixed ABI flags apply the two supplied sets,
        // without any library-selected execution/timeout policy.
        rc(
            unsafe {
                libc::posix_spawnattr_setflags(
                    attrs.as_mut_ptr(),
                    (libc::POSIX_SPAWN_SETSIGMASK | libc::POSIX_SPAWN_SETSIGDEF) as libc::c_short,
                )
            },
            "atomic-spawn-attribute-flags",
        )?;
        let mut process: libc::c_int = -1;
        out.invoked = true;
        // SAFETY: exact resolved GNU ABI; separate initialized attrs/actions, retained
        // NUL paths/strings and null-terminated pointer arrays span synchronous call.
        // libc owns child setup; no Rust code/allocation runs in a forked child here.
        rc(
            unsafe {
                (symbols.spawn)(
                    &mut process,
                    spec.program.as_ptr(),
                    actions.as_ptr(),
                    attrs.as_ptr(),
                    argv.as_ptr(),
                    env.as_ptr(),
                )
            },
            "atomic-spawn-native",
        )?;
        if process < 0 {
            return Err(Error::refused("atomic-spawn-descriptor-missing"));
        }
        // SAFETY: only native rc0 guarantees a new unique pidfd in this output slot.
        out.process = Some(unsafe { OwnedFd::from_raw_fd(process) });
        Ok(())
    })();
    out.primary = result.err();
    if attrs_ready {
        // SAFETY: successful init established ownership, exactly one destroy attempt.
        out.cleanup.attributes = rc(
            unsafe { libc::posix_spawnattr_destroy(attrs.as_mut_ptr()) },
            "atomic-spawn-attributes-destroy",
        )
        .err();
    }
    if actions_ready {
        // SAFETY: separate initialized actions remain valid through spawn and destroy.
        out.cleanup.actions = rc(
            unsafe { libc::posix_spawn_file_actions_destroy(actions.as_mut_ptr()) },
            "atomic-spawn-actions-destroy",
        )
        .err();
    }
    out.cleanup.stdin = close(streams[0].take(), "atomic-spawn-close-stdin");
    out.cleanup.stdout = close(streams[1].take(), "atomic-spawn-close-stdout");
    out.cleanup.stderr = close(streams[2].take(), "atomic-spawn-close-stderr");
    if let Some(fd) = &out.process {
        out.validation = super::process_identity(fd.as_fd()).err()
    }
    out
}
/// Observe/reap by the held descriptor only. WNOHANG is an explicit query, not
/// execution timeout; blocking is caller-selected. External reaping keeps ECHILD
/// rather than a fabricated empty successful status or numeric PID fallback.
pub(crate) fn wait(fd: BorrowedFd<'_>, blocking: bool) -> Result<Option<Exit>, Error> {
    let mut info = MaybeUninit::<libc::siginfo_t>::zeroed();
    let flags = libc::WEXITED | if blocking { 0 } else { libc::WNOHANG };
    // SAFETY: valid held pidfd, exact writable initialized ABI buffer. Kernel fills
    // siginfo on exit; zero si_pid means WNOHANG observed no exit status.
    let result = unsafe {
        libc::waitid(
            libc::P_PIDFD,
            fd.as_raw_fd() as libc::id_t,
            info.as_mut_ptr(),
            flags,
        )
    };
    if result < 0 {
        return Err(Error::syscall(
            "atomic-spawn-wait",
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: initialized zero ABI storage and successful waitid; access libc's
    // documented SIGCHLD union fields only for this waitid result.
    let info = unsafe { info.assume_init() };
    let pid = unsafe { info.si_pid() };
    if pid == 0 {
        return Ok(None);
    }
    let kind = match info.si_code {
        libc::CLD_EXITED => ExitKind::Exited,
        libc::CLD_KILLED => ExitKind::Signaled,
        libc::CLD_DUMPED => ExitKind::CoreDumped,
        n => ExitKind::Other(n),
    };
    // SAFETY: same successful waitid SIGCHLD result, status is initialized.
    Ok(Some(Exit {
        kind,
        status: unsafe { info.si_status() },
    }))
}

#[cfg(test)]
mod tests;
