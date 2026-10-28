//! Only this isolated subprocess changes its own SIGCHLD disposition.
//! Parent harness, server and host configuration remain unchanged.
use super::*;
use crate::atomic_spawn;
use nix::{
    poll::{poll, PollFd, PollFlags, PollTimeout},
    sys::signal::SigSet,
};
use std::{ffi::CString, fs::OpenOptions, process::Command};
/// Atomic handle survives native fast exit with SIGCHLD auto-reaping. No PID
/// lookup fallback or wait-status success is fabricated when status is unavailable.
#[test]
fn atomic_handle_survives_sigchld_autoreap_in_isolated_worker() {
    const KEY: &str = "PAEA_ATOMIC_AUTOREAP_WORKER";
    if std::env::var_os(KEY).is_none() {
        let out=Command::new(std::env::current_exe().unwrap()).args(["--exact","kernel::atomic_spawn::tests::atomic_handle_survives_sigchld_autoreap_in_isolated_worker","--nocapture"]).env(KEY,"yes").output().unwrap();
        assert!(
            out.status.success(),
            "isolated worker: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed; 0 failed"));
        return;
    }
    let mut old = MaybeUninit::<libc::sigaction>::uninit();
    // SAFETY: ABI sigaction fields initialized, kernel captures previous disposition.
    // Only this one-test subprocess is changed; all success/error results are captured
    // before restoring it and asserting. No user process or host signal policy changes.
    let mut action = unsafe { MaybeUninit::<libc::sigaction>::zeroed().assume_init() };
    action.sa_sigaction = libc::SIG_IGN;
    // SAFETY: valid initialized native sigset storage and output action buffer.
    let install = unsafe {
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGCHLD, &action, old.as_mut_ptr())
    };
    assert_eq!(install, 0);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/usr/bin/true").unwrap();
    let directory = CString::new("/").unwrap();
    let argv = [CString::new("true").unwrap()];
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let spec = Spec {
        program: &program,
        arguments: &argv,
        environment: &[],
        directory: &directory,
        stdin: file.as_fd(),
        stdout: file.as_fd(),
        stderr: file.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let launched = atomic_spawn::launch(&spec);
    let observed = launched.map(|mut child| {
        let mut fds = [PollFd::new(child.anchor().process(), PollFlags::POLLIN)];
        let ready = poll(&mut fds, PollTimeout::NONE);
        let waited = child.try_wait();
        let closed = child.close();
        (ready, waited, closed)
    });
    // SAFETY: successful install initialized old; restore before interpreting results.
    let restored = unsafe { libc::sigaction(libc::SIGCHLD, old.as_ptr(), std::ptr::null_mut()) };
    assert_eq!(restored, 0);
    let (ready, waited, closed) = observed.unwrap();
    assert!(ready.unwrap() > 0);
    let error = waited.err().unwrap();
    assert!(error
        .causes()
        .iter()
        .any(|(_, e)| e.raw_os_error() == Some(libc::ECHILD)));
    closed.unwrap();
}
