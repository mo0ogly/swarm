//! Sole Linux FFI boundary. Types/constants come from locked libc 0.2.186.
//! SCM_PIDFD is absent there: Linux 6.17 include/linux/socket.h defines 0x04
//! as a read-only kernel pidfd control. No architecture-dependent ioctl literals.
use crate::{Error, Namespaces, Packet};
use std::{
    io, mem,
    os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
};
use zeroize::{Zeroize, Zeroizing};
const SCM_PIDFD: libc::c_int = 0x04;
// Linux UAPI representation invariants, never operator execution limits. Refuse
// compilation on an unsupported layout instead of silently reinterpreting it.
const _: () = {
    assert!(mem::size_of::<libc::c_int>() == 4);
    assert!(mem::size_of::<libc::ucred>() == 12);
    assert!(mem::size_of::<libc::cmsghdr>() == mem::size_of::<usize>() + 8);
};

/// Read one integer socket option through a borrowed live FD; reject short ABI
/// output. No descriptor ownership changes and no uninitialized value is read.
fn get(fd: BorrowedFd<'_>, option: libc::c_int) -> Result<libc::c_int, Error> {
    let mut value: libc::c_int = 0;
    let mut len = mem::size_of_val(&value) as libc::socklen_t;
    // SAFETY: valid borrowed FD, aligned initialized int/length, exact writable
    // object extent; no reference escapes the synchronous libc call.
    let result = unsafe {
        libc::getsockopt(
            fd.as_raw_fd(),
            libc::SOL_SOCKET,
            option,
            (&mut value as *mut libc::c_int).cast(),
            &mut len,
        )
    };
    if result < 0 {
        return Err(Error::syscall(
            "identity-channel-get-option",
            io::Error::last_os_error(),
        ));
    }
    if len as usize != mem::size_of_val(&value) {
        return Err(Error::refused("identity-channel-option-size"));
    }
    Ok(value)
}
/// Require current Unix seqpacket type; this never identifies a message sender.
fn kind(fd: BorrowedFd<'_>) -> Result<(), Error> {
    if get(fd, libc::SO_TYPE)? != libc::SOCK_SEQPACKET || get(fd, libc::SO_DOMAIN)? != libc::AF_UNIX
    {
        return Err(Error::refused("identity-channel-kind"));
    }
    Ok(())
}
/// Recheck both effective options; no token assumes aliases cannot change them.
fn ready(fd: BorrowedFd<'_>) -> Result<(), Error> {
    kind(fd)?;
    if get(fd, libc::SO_PASSCRED)? != 1 || get(fd, libc::SO_PASSPIDFD)? != 1 {
        return Err(Error::refused("identity-channel-options-required"));
    }
    Ok(())
}
/// Set required kernel options before sender release. A later failure may leave
/// earlier options enabled; returned refusal conveys no configured channel.
pub(super) fn configure(fd: BorrowedFd<'_>) -> Result<(), Error> {
    kind(fd)?;
    let enabled: libc::c_int = 1;
    for option in [libc::SO_PASSCRED, libc::SO_PASSPIDFD] {
        // SAFETY: borrowed FD, initialized aligned int and exact readonly extent.
        let result = unsafe {
            libc::setsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                option,
                (&enabled as *const libc::c_int).cast(),
                mem::size_of_val(&enabled) as libc::socklen_t,
            )
        };
        if result < 0 {
            return Err(Error::syscall(
                "identity-channel-set-option",
                io::Error::last_os_error(),
            ));
        }
    }
    ready(fd)
}
/// Linux cmsghdr alignment uses sizeof(size_t). All callers use checked bounds;
/// this fixed small transport capacity is not an operator execution policy.
fn align(n: usize) -> Option<usize> {
    let a = mem::size_of::<usize>();
    n.checked_add(a - 1).map(|v| v & !(a - 1))
}
/// Read a checked native int from initialized kernel output, without casting a
/// slice to a potentially misaligned struct or nonzero PID representation.
fn integer(raw: &[u8]) -> i32 {
    i32::from_ne_bytes(raw.try_into().expect("checked native int width"))
}
/// Collect only kernel-installed FD integers into unique owners. This private
/// function is called solely on output from recvmsg, never caller-supplied bytes.
/// Reject duplicate/negative values before constructing another owner.
///
/// # Safety
/// `raw` must originate from this call's successful recvmsg installation, not
/// untrusted integers or another owner. Nonnegative/duplicate checks are local.
unsafe fn own(raw: i32, fds: &mut Vec<OwnedFd>) -> bool {
    if raw < 0 || fds.iter().any(|fd| fd.as_raw_fd() == raw) {
        return false;
    }
    // SAFETY: successful recvmsg installed this nonnegative descriptor for this
    // process in SCM_RIGHTS/SCM_PIDFD. It is not exposed or closed before transfer;
    // duplicate numbers have been rejected. No safe public raw-FD constructor.
    fds.push(unsafe { OwnedFd::from_raw_fd(raw) });
    true
}
/// Parse initialized kernel controls and take ownership of every returned known
/// FD before deciding refusal. Kernel-generated headers obey the Linux ABI;
/// malformed observations still reject, never decode a successful prefix.
///
/// # Safety
/// Bytes must be actual initialized recvmsg output for this process; embedded
/// installed descriptors must not have been exposed, closed or taken by another
/// owner. Never call on request data or arbitrary synthetic/fuzzed FD integers.
unsafe fn controls(raw: &[u8]) -> (Option<(i32, u32, u32)>, Vec<OwnedFd>, bool) {
    let width = mem::size_of::<usize>();
    let header = mem::size_of::<libc::cmsghdr>();
    let mut at = 0;
    let mut cred = None;
    let mut fds = Vec::new();
    let mut invalid = false;
    let mut pidfds = 0;
    while raw.len().saturating_sub(at) >= header {
        let len = usize::from_ne_bytes(
            raw[at..at + width]
                .try_into()
                .expect("checked size_t width"),
        );
        if len < header || len > raw.len() - at {
            invalid = true;
            break;
        }
        let level = integer(&raw[at + width..at + width + 4]);
        let ty = integer(&raw[at + width + 4..at + width + 8]);
        let data = &raw[at + header..at + len];
        if level == libc::SOL_SOCKET && (ty == libc::SCM_RIGHTS || ty == SCM_PIDFD) {
            if ty == libc::SCM_RIGHTS {
                invalid = true
            } else {
                pidfds += 1;
                if data.len() != 4 {
                    invalid = true
                }
            }
            if data.len() % 4 != 0 {
                invalid = true
            }
            for part in data.chunks_exact(4) {
                // SAFETY: this private parser receives only the owned kernel output
                // of one recvmsg; local own checks prevent duplicate owners.
                if !unsafe { own(integer(part), &mut fds) } {
                    invalid = true
                }
            }
        } else if level == libc::SOL_SOCKET && ty == libc::SCM_CREDENTIALS {
            if cred.is_some() || data.len() != mem::size_of::<libc::ucred>() {
                invalid = true
            } else {
                let pid = integer(&data[..4]);
                let uid = u32::from_ne_bytes(data[4..8].try_into().expect("ucred uid width"));
                let gid = u32::from_ne_bytes(data[8..12].try_into().expect("ucred gid width"));
                if pid <= 0 || uid == u32::MAX || gid == u32::MAX {
                    invalid = true
                }
                cred = Some((pid, uid, gid));
            }
        } else {
            invalid = true
        }
        match align(len).and_then(|n| at.checked_add(n)) {
            Some(next) if next > at => at = next,
            _ => {
                invalid = true;
                break;
            }
        }
    }
    if cred.is_none() || pidfds != 1 || fds.len() != 1 {
        invalid = true
    }
    (cred, fds, invalid)
}
/// One synchronous receive; initialized guarded payload and aligned initialized
/// ancillary allocation remain live throughout libc. Parse/close returned FDs
/// even when MSG_TRUNC/MSG_CTRUNC refuses the packet. On success wipe/trim unused
/// initialized payload without reallocating, for an exact guarded decoder borrow.
/// No automatic EINTR retry.
pub(super) fn receive(fd: BorrowedFd<'_>, mut buffer: Zeroizing<Vec<u8>>) -> Result<Packet, Error> {
    if buffer.is_empty() {
        return Err(Error::refused("identity-channel-buffer-required"));
    }
    ready(fd)?;
    let header = align(mem::size_of::<libc::cmsghdr>()).expect("fixed header");
    let capacity = header
        + align(mem::size_of::<libc::ucred>()).expect("fixed cred")
        + header
        + align(mem::size_of::<libc::c_int>()).expect("fixed pidfd");
    let mut ancillary = vec![0usize; capacity.div_ceil(mem::size_of::<usize>())];
    let mut iov = libc::iovec {
        iov_base: buffer.as_mut_ptr().cast(),
        iov_len: buffer.len(),
    };
    // SAFETY: all-zero msghdr is valid (null optional pointers/integer fields).
    let mut message: libc::msghdr = unsafe { mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = ancillary.as_mut_ptr().cast();
    message.msg_controllen = capacity;
    // SAFETY: borrowed live FD; iovec/payload and ancillary refer to distinct,
    // initialized writable allocations of exactly the advertised extents. Their
    // storage cannot move or be accessed concurrently during this call.
    let n = unsafe { libc::recvmsg(fd.as_raw_fd(), &mut message, libc::MSG_CMSG_CLOEXEC) };
    if n < 0 {
        return Err(Error::syscall(
            "identity-channel-receive",
            io::Error::last_os_error(),
        ));
    }
    let used = message.msg_controllen.min(capacity);
    // SAFETY: initialized usize allocation spans capacity bytes, used<=capacity;
    // returned slice borrows it only while alive. No uninitialized bytes exposed.
    let raw = unsafe { std::slice::from_raw_parts(ancillary.as_ptr().cast::<u8>(), used) };
    // SAFETY: raw is from the successful synchronous recvmsg above. No FD
    // embedded there has yet been exposed, closed or assigned another owner.
    let (cred, mut fds, invalid) = unsafe { controls(raw) };
    let reason = if message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0
        || message.msg_controllen > capacity
        || n as usize > buffer.len()
    {
        Some("identity-channel-truncated")
    } else if n == 0 {
        Some("identity-channel-empty-or-closed")
    } else if invalid {
        Some("identity-channel-unattributed")
    } else {
        None
    };
    if let Some(reason) = reason {
        let mut error = Error::refused(reason);
        for f in fds {
            error.cleanup(f, "close-rejected-control")
        }
        return Err(error);
    }
    // The caller may have supplied initialized nonzero reserve bytes. Wipe them
    // before trimming the logical packet, with no secret-bearing reallocation.
    // Zeroizing<Vec> also wipes the full capacity when the packet is dropped.
    buffer[n as usize..].zeroize();
    buffer.truncate(n as usize);
    let process = fds.pop().expect("validated unique kernel handle");
    let (pid, uid, gid) = cred.expect("validated kernel credentials");
    Ok(Packet {
        buffer,
        used: n as usize,
        pid,
        uid,
        gid,
        process,
    })
}
/// Acquire one namespace FD using the held pidfd, never `/proc/<pid>`. The ioctl
/// creates a new descriptor or returns failure; no pointer argument is supplied.
fn namespace(
    process: &OwnedFd,
    request: libc::Ioctl,
    phase: &'static str,
) -> Result<OwnedFd, Error> {
    // SAFETY: borrowed valid pidfd, fixed no-pointer namespace request exported
    // by libc. Linux validates descriptor kind, live task and ptrace permissions.
    let raw = unsafe { libc::ioctl(process.as_raw_fd(), request, 0) };
    if raw < 0 {
        return Err(Error::syscall(phase, io::Error::last_os_error()));
    }
    // SAFETY: successful namespace ioctl returns a fresh uniquely owned FD.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}
/// Preserve partial acquisition cleanup failure separately from the primary
/// second-ioctl cause. Both successful handles require an explicit consumer close.
pub(super) fn namespaces(process: &OwnedFd) -> Result<Namespaces, Error> {
    let mount = namespace(
        process,
        libc::PIDFD_GET_MNT_NAMESPACE,
        "identity-channel-mount-namespace",
    )?;
    match namespace(
        process,
        libc::PIDFD_GET_PID_NAMESPACE,
        "identity-channel-pid-namespace",
    ) {
        Ok(pid) => Ok(Namespaces { mount, pid }),
        Err(mut error) => {
            error.cleanup(mount, "close-partial-mount-namespace");
            Err(error)
        }
    }
}

/// Linux UAPI include/uapi/linux/magic.h PID_FS_MAGIC ("PIDF"), absent in locked
/// libc0.2.186. This filesystem kind is a kernel identity invariant, not policy.
const PID_FS_MAGIC: libc::c_long = 0x50494446;

/// Open the caller's independently owned, not-yet-reaped Child. No packet or
/// external PID argument enters this boundary. Caller prevents external reapers
/// and SIGCHLD auto-reap during try_wait/open; this is not an atomic spawn API.
/// try_wait can reap a terminated child; no signal/kill/wait-loop is performed.
pub(super) fn child_process(child: &mut std::process::Child) -> Result<OwnedFd, Error> {
    if child
        .try_wait()
        .map_err(|e| Error::syscall("identity-channel-child-state", e))?
        .is_some()
    {
        return Err(Error::refused("identity-channel-process-exited"));
    }
    let pid = libc::pid_t::try_from(child.id())
        .map_err(|_| Error::refused("identity-channel-child-pid"))?;
    if pid <= 0 {
        return Err(Error::refused("identity-channel-child-pid"));
    }
    // SAFETY: fixed no-pointer Linux syscall, checked positive PID derived only
    // from this unreaped std Child, flags0. Kernel validates the process and
    // returns a fresh CLOEXEC descriptor or negative errno. Parent/reaper custody
    // is an explicit caller precondition, not inferred from the numeric value.
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0 as libc::c_uint) };
    if raw < 0 {
        return Err(Error::syscall(
            "identity-channel-child-pidfd",
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: successful pidfd_open returns a new uniquely owned Linux fd (int),
    // not a caller-supplied raw descriptor. Linux fd range fits libc::c_int.
    Ok(unsafe { OwnedFd::from_raw_fd(raw as libc::c_int) })
}
/// Require actual pidfs and full64bit inode representation before comparing
/// retained objects. Linux32 needs inode generation/file handles, unsupported
/// here; old anon_inode must refuse, never accidentally match all processes.
/// No PID/path reopening, FD mutation or authority is inferred.
pub(super) fn process_identity(fd: BorrowedFd<'_>) -> Result<(u64, u64), Error> {
    if mem::size_of::<libc::ino_t>() < 8 {
        return Err(Error::refused("identity-channel-process-identity-width"));
    }
    let fs = nix::sys::statfs::fstatfs(fd)
        .map_err(|e| Error::syscall("identity-channel-process-statfs", io::Error::from(e)))?;
    if fs.filesystem_type().0 != PID_FS_MAGIC {
        return Err(Error::refused("identity-channel-process-pidfs-required"));
    }
    let stat = nix::sys::stat::fstat(fd)
        .map_err(|e| Error::syscall("identity-channel-process-stat", io::Error::from(e)))?;
    Ok((u64::from(stat.st_dev), u64::from(stat.st_ino)))
}
/// Immediate pidfd termination observation only. PollTimeout::ZERO is a
/// nonblocking query, not a test watchdog, execution deadline or retry policy.
/// Returned exited/error/invalid states never become live; EINTR is preserved.
pub(super) fn process_live(fd: BorrowedFd<'_>) -> Result<(), Error> {
    use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
    let mut fds = [PollFd::new(fd, PollFlags::POLLIN)];
    let count = poll(&mut fds, PollTimeout::ZERO)
        .map_err(|e| Error::syscall("identity-channel-process-poll", io::Error::from(e)))?;
    let flags = fds[0]
        .revents()
        .ok_or_else(|| Error::refused("identity-channel-process-poll-state"))?;
    if flags.intersects(PollFlags::POLLERR | PollFlags::POLLNVAL) {
        return Err(Error::refused("identity-channel-process-poll-state"));
    }
    if flags.intersects(PollFlags::POLLIN | PollFlags::POLLHUP) {
        return Err(Error::refused("identity-channel-process-exited"));
    }
    if count != 0 || !flags.is_empty() {
        return Err(Error::refused("identity-channel-process-poll-state"));
    }
    Ok(())
}

/// Sole GNU process-creation/wait FFI; no PID capture fallback.
#[cfg(target_env = "gnu")]
pub(crate) mod atomic_spawn;

/// One-packet outbound transport with retained invocation/acceptance evidence.
pub(crate) mod send;
