//! Only this isolated one-test worker changes its signal disposition/mask.
use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
use std::{mem::MaybeUninit, process::Command};
use zeroize::Zeroizing;
/// With default unblocked SIGPIPE, valid control precedes closed-peer EPIPE.
/// Native NOSIGNAL must preserve worker life; parent/host/server remain untouched.
#[test]
fn closed_peer_send_with_default_unblocked_sigpipe_survives() {
    const NAME: &str =
        "kernel::send::tests::closed_peer_send_with_default_unblocked_sigpipe_survives";
    const KEY: &str = "PAEA_PACKET_SIGPIPE_ISOLATED_WORKER";
    if std::env::var_os(KEY).is_none() {
        let out = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env(KEY, "yes")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "isolated worker: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed; 0 failed"));
        return;
    }
    let (a, b) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut out = crate::configure(&a).unwrap();
    let mut input = crate::configure(&b).unwrap();
    // SAFETY: initialized native buffers and writable old-state outputs. Only this
    // isolated worker's disposition and calling thread's mask change. No host state.
    let mut action = unsafe { MaybeUninit::<libc::sigaction>::zeroed().assume_init() };
    action.sa_sigaction = libc::SIG_DFL;
    let mut old_action = MaybeUninit::<libc::sigaction>::uninit();
    let mut unblock = MaybeUninit::<libc::sigset_t>::uninit();
    let mut old_mask = MaybeUninit::<libc::sigset_t>::uninit();
    // SAFETY: correctly aligned owned ABI storage; sigemptyset initializes the set
    // before sigaddset/read. pthread_sigmask returns errno directly, not last errno.
    let installed = unsafe {
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGPIPE, &action, old_action.as_mut_ptr())
    };
    assert_eq!(installed, 0);
    // SAFETY: initialized set includes only SIGPIPE, previous mask copied to output.
    let unblocked = unsafe {
        libc::sigemptyset(unblock.as_mut_ptr());
        libc::sigaddset(unblock.as_mut_ptr(), libc::SIGPIPE);
        libc::pthread_sigmask(libc::SIG_UNBLOCK, unblock.as_ptr(), old_mask.as_mut_ptr())
    };
    assert_eq!(unblocked, 0);
    out.send_packet(b"valid-control").unwrap();
    let packet = input.receive(Zeroizing::new(vec![0; 128])).unwrap();
    assert_eq!(packet.data(), b"valid-control");
    packet.close().unwrap();
    drop(input);
    drop(b);
    let failure = out.send_packet(b"closed-peer");
    // SAFETY: both successful installations initialized the previous-state buffers;
    // restore before interpreting the result, with independent restoration checks.
    let restored_action =
        unsafe { libc::sigaction(libc::SIGPIPE, old_action.as_ptr(), std::ptr::null_mut()) };
    // SAFETY: restore the calling worker thread's exact previous signal mask.
    let restored_mask = unsafe {
        libc::pthread_sigmask(libc::SIG_SETMASK, old_mask.as_ptr(), std::ptr::null_mut())
    };
    assert_eq!(restored_action, 0);
    assert_eq!(restored_mask, 0);
    let e = failure.unwrap_err();
    assert!(e.attempted);
    assert_eq!(e.cause.causes()[0].1.raw_os_error(), Some(libc::EPIPE));
}
