#![cfg(all(target_os = "linux", target_env = "gnu"))]
#![forbid(unsafe_code)]
//! Actual native atomically-created descriptor; no account/image/rights approval.
use nix::sys::{
    signal::{SigSet, Signal},
    socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType},
};
use std::{
    ffi::CString,
    os::fd::{AsFd, AsRawFd},
    sync::Mutex,
};
use swarm_linux_identity_channel::{
    atomic_spawn::{self, Spec},
    configure,
};
use zeroize::Zeroizing;
static SERIAL: Mutex<()> = Mutex::new(());
/// Explicit test-only program/argv/env/cwd/mask, no inherited environment default.
fn valid() {
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut channel = configure(&rx).unwrap();
    let null = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/usr/bin/python3").unwrap();
    let directory = CString::new("/").unwrap();
    let code="import os,socket,signal; s=socket.socket(fileno=0); s.send((os.getcwd()+'|'+str(os.environ.get('PAEA_ATOMIC_TEST'))+'|'+str(os.environ.get('PATH'))+'|'+str(signal.SIGUSR1 in signal.pthread_sigmask(signal.SIG_BLOCK,[]))).encode()); s.recv(1)";
    let argv = [
        CString::new("python3").unwrap(),
        CString::new("-c").unwrap(),
        CString::new(code).unwrap(),
    ];
    let env = [CString::new("PAEA_ATOMIC_TEST=present").unwrap()];
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGUSR1);
    let defaults = SigSet::all();
    let spec = Spec {
        program: &program,
        arguments: &argv,
        environment: &env,
        directory: &directory,
        stdin: tx.as_fd(),
        stdout: null.as_fd(),
        stderr: null.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let mut child = atomic_spawn::launch(&spec).unwrap();
    let packet = channel.receive(Zeroizing::new(vec![0; 256])).unwrap();
    let matched = child.anchor().matches(&packet);
    let content = packet.data().to_vec();
    let pending = child.try_wait();
    send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL).unwrap();
    let exit = child.wait().unwrap();
    assert!(exit.success());
    assert!(matched.unwrap());
    assert_eq!(content, b"/|present|None|True");
    assert!(pending.unwrap().is_none());
    assert_eq!(child.try_wait().unwrap(), Some(exit));
    packet.close().unwrap();
    child.close().unwrap();
}
/// Valid process before each refusal and recovery; resources belong to this target.
#[test]
fn atomic_spawn_actual_subject_explicit_environment_directory_signals_and_wait() {
    let _g = SERIAL.lock().unwrap();
    valid();
}
/// Native exec refusal keeps invocation classification without fabricated handle;
/// a fresh valid call succeeds afterwards. No live database or provider invoked.
#[test]
fn atomic_spawn_exec_refusal_then_fresh_control() {
    let _g = SERIAL.lock().unwrap();
    valid();
    let null = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/nonexistent/paea-atomic-executable").unwrap();
    let directory = CString::new("/").unwrap();
    let argv = [CString::new("missing").unwrap()];
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let spec = Spec {
        program: &program,
        arguments: &argv,
        environment: &[],
        directory: &directory,
        stdin: null.as_fd(),
        stdout: null.as_fd(),
        stderr: null.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let error = atomic_spawn::launch(&spec).err().unwrap();
    assert!(error.child.is_none());
    assert!(error.invoked);
    assert!(error
        .cause
        .causes()
        .iter()
        .any(|(_, e)| e.raw_os_error() == Some(2)));
    valid();
}

/// Fast exit returns an owner, never a liveness error that discards known launch.
/// Relative input refuses before invocation; old owner observation is retained.
#[test]
fn atomic_spawn_fast_exit_and_preflight_refusal_keep_explicit_state() {
    let _g = SERIAL.lock().unwrap();
    valid();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/usr/bin/true").unwrap();
    let directory = CString::new("/").unwrap();
    let args = [CString::new("true").unwrap()];
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let mut spec = Spec {
        program: &program,
        arguments: &args,
        environment: &[],
        directory: &directory,
        stdin: file.as_fd(),
        stdout: file.as_fd(),
        stderr: file.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let mut child = atomic_spawn::launch(&spec).unwrap();
    assert!(child.wait().unwrap().success());
    child.close().unwrap();
    let relative = CString::new("relative-program").unwrap();
    spec.program = &relative;
    let error = atomic_spawn::launch(&spec).err().unwrap();
    assert!(!error.invoked && error.child.is_none());
    assert_eq!(error.code(), "atomic-spawn-input");
    valid();
}
/// An unrelated deliberately inheritable descriptor is absent from the executed
/// child. Compare pipe object identity, not an FD number that Python may reuse.
#[test]
fn atomic_spawn_closes_unrelated_inheritable_descriptor() {
    use nix::{
        fcntl::{fcntl, FcntlArg, FdFlag},
        sys::stat::fstat,
        unistd::pipe,
    };
    let _g = SERIAL.lock().unwrap();
    valid();
    let (marker, _writer) = pipe().unwrap();
    fcntl(&marker, FcntlArg::F_SETFD(FdFlag::empty())).unwrap();
    let stamp = fstat(&marker).unwrap();
    let (tx, rx) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut channel = configure(&rx).unwrap();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/usr/bin/python3").unwrap();
    let directory = CString::new("/").unwrap();
    let code=format!("import os,socket; s=socket.socket(fileno=0)\ntry: t=os.fstat({}); same=(t.st_dev,t.st_ino)==({},{})\nexcept OSError: same=False\ns.send(str(same).encode()); s.recv(1)",marker.as_raw_fd(),stamp.st_dev,stamp.st_ino);
    let args = [
        CString::new("python3").unwrap(),
        CString::new("-c").unwrap(),
        CString::new(code).unwrap(),
    ];
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let spec = Spec {
        program: &program,
        arguments: &args,
        environment: &[],
        directory: &directory,
        stdin: tx.as_fd(),
        stdout: file.as_fd(),
        stderr: file.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let mut child = atomic_spawn::launch(&spec).unwrap();
    let p = channel.receive(Zeroizing::new(vec![0; 64])).unwrap();
    let response = p.data().to_vec();
    let flags = fcntl(child.anchor().process(), FcntlArg::F_GETFD).unwrap();
    send(rx.as_raw_fd(), b"A", MsgFlags::MSG_NOSIGNAL).unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(response, b"False");
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
    p.close().unwrap();
    child.close().unwrap();
    valid();
}
