//! Narrow Linux ABI adapter. No policy or caller identity is inferred here.
use std::{io, mem, os::fd::{AsRawFd, FromRawFd, OwnedFd}};
/// Landlock ABI6 is required: file truncation, device ioctl and signal scopes.
pub(crate) fn abi() -> io::Result<i32> {
    // SAFETY: version query has no pointed input and returns an integer only.
    let v=unsafe {libc::syscall(libc::SYS_landlock_create_ruleset,std::ptr::null::<u8>(),0,1)};
    if v<0 {Err(io::Error::last_os_error())} else {Ok(v as i32)}
}
pub(crate) const READ:u64=(1<<0)|(1<<2)|(1<<3);
pub(crate) const ALL:u64=(1<<16)-1;
#[repr(C)] struct Ruleset {fs:u64,net:u64,scoped:u64}
#[repr(C,packed)] struct Beneath {access:u64,fd:i32}
/// Create a closed ruleset. Only the caller's later explicit leaves are allowed.
pub(crate) fn ruleset()->io::Result<OwnedFd>{
    let attr=Ruleset{fs:ALL,net:0,scoped:3};
    // SAFETY: initialized C-layout value and exact ABI size live through syscall.
    let fd=unsafe{libc::syscall(libc::SYS_landlock_create_ruleset,&attr,mem::size_of::<Ruleset>(),0)};
    if fd<0 {Err(io::Error::last_os_error())} else {
        // SAFETY: successful syscall transfers a fresh owned descriptor.
        Ok(unsafe{OwnedFd::from_raw_fd(fd as i32)})
    }
}
/// Add one already-opened inode. Caller chooses only this fixed profile's masks.
pub(crate) fn add(rules:&OwnedFd,path:&impl AsRawFd,access:u64)->io::Result<()> {
    let attr=Beneath{access,fd:path.as_raw_fd()};
    // SAFETY: packed Linux struct has the documented layout; borrowed FDs live.
    let v=unsafe{libc::syscall(libc::SYS_landlock_add_rule,rules.as_raw_fd(),1,&attr,0)};
    if v<0 {Err(io::Error::last_os_error())} else {Ok(())}
}
/// Irreversibly restrict this single-threaded helper and all future descendants.
pub(crate) fn restrict(rules:&OwnedFd)->io::Result<()> {
    // SAFETY: no pointers; no_new_privs can only reduce this thread's authority.
    if unsafe{libc::prctl(libc::PR_SET_NO_NEW_PRIVS,1,0,0,0)}!=0{return Err(io::Error::last_os_error())}
    // SAFETY: valid owned ruleset and no optional flags.
    if unsafe{libc::syscall(libc::SYS_landlock_restrict_self,rules.as_raw_fd(),0)}<0{return Err(io::Error::last_os_error())}
    // SAFETY: mark inherited non-stdio descriptors close-on-exec, not close-now.
    if unsafe{libc::syscall(libc::SYS_close_range,3u32,u32::MAX,4u32)}<0{return Err(io::Error::last_os_error())}
    Ok(())
}
/// Install a fixed seccomp deny profile after Landlock. No user filter/program.
pub(crate) fn seccomp()->io::Result<()> {
    #[cfg(target_arch="x86_64")] const ARCH:u32=0xc000003e;
    #[cfg(target_arch="aarch64")] const ARCH:u32=0xc00000b7;
    #[cfg(not(any(target_arch="x86_64",target_arch="aarch64")))] compile_error!("Biba seccomp requires audited native architecture");
    fn ins(code:u16,jt:u8,jf:u8,k:u32)->libc::sock_filter{libc::sock_filter{code,jt,jf,k}}
    let deny=0x00050000|libc::EPERM as u32;
    let mut f=vec![ins(0x20,0,0,4),ins(0x15,1,0,ARCH),ins(0x06,0,0,0x80000000),ins(0x20,0,0,0)];
    #[cfg(target_arch="x86_64")] f.extend([ins(0x35,0,1,0x40000000),ins(0x06,0,0,deny)]);
    // Metadata setters are outside Landlock's filesystem access-right coverage.
    let denied=[libc::SYS_ptrace,libc::SYS_process_vm_readv,libc::SYS_process_vm_writev,
        libc::SYS_pidfd_getfd,libc::SYS_mount,libc::SYS_umount2,libc::SYS_pivot_root,
        libc::SYS_chroot,libc::SYS_setns,libc::SYS_unshare,libc::SYS_open_by_handle_at,
        libc::SYS_io_uring_setup,libc::SYS_bpf,libc::SYS_perf_event_open,
        libc::SYS_fchmod,libc::SYS_fchmodat,libc::SYS_fchmodat2,
        libc::SYS_fchown,libc::SYS_fchownat,libc::SYS_utimensat,
        libc::SYS_setxattr,libc::SYS_lsetxattr,libc::SYS_fsetxattr,
        libc::SYS_removexattr,libc::SYS_lremovexattr,libc::SYS_fremovexattr];
    for n in denied {f.extend([ins(0x15,0,1,n as u32),ins(0x06,0,0,deny)]);}
    #[cfg(target_arch="x86_64")]
    for n in [libc::SYS_chmod,libc::SYS_chown,libc::SYS_lchown,libc::SYS_utime,libc::SYS_utimes,libc::SYS_futimesat] {f.extend([ins(0x15,0,1,n as u32),ins(0x06,0,0,deny)]);}
    // An outside Unix service must not become a same-UID authority bypass.
    f.extend([ins(0x15,0,3,libc::SYS_socket as u32),ins(0x20,0,0,16),ins(0x15,0,1,libc::AF_UNIX as u32),ins(0x06,0,0,deny),ins(0x20,0,0,0)]);
    let branch=f.len();f.push(ins(0x15,0,0,libc::SYS_ioctl as u32));f.push(ins(0x20,0,0,24));
    for request in [libc::FIONREAD as u32,libc::FIONBIO as u32,libc::TCGETS as u32,libc::TIOCGWINSZ as u32] {f.extend([ins(0x15,0,1,request),ins(0x06,0,0,0x7fff0000)]);}
    f.push(ins(0x06,0,0,deny));f[branch].jf=(f.len()-branch-1) as u8;
    f.push(ins(0x06,0,0,0x7fff0000));
    let program=libc::sock_fprog{len:f.len() as u16,filter:f.as_mut_ptr()};
    // SAFETY: initialized finite native BPF instructions remain alive through prctl.
    if unsafe{libc::prctl(libc::PR_SET_SECCOMP,2,&program)}!=0{Err(io::Error::last_os_error())}else{Ok(())}
}
