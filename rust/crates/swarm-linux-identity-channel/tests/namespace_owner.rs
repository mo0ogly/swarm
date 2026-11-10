#![cfg(all(target_os = "linux", target_env = "gnu"))]
#![forbid(unsafe_code)]
//! Namespace-owner inspection only, same external UID. No separate-UID, approval,
//! confinement, daemon or host-policy proof. All FFI stays in production kernel;
//! only the disposable Python child invokes its own prctl fixture.
use nix::{
    sys::stat::fstat,
    sys::{
        signal::SigSet,
        socket::{socketpair, AddressFamily, SockFlag, SockType},
    },
};
use std::{
    ffi::CString,
    fs::{self, OpenOptions},
    os::{fd::AsFd, unix::fs::MetadataExt},
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
};
use swarm_linux_identity_channel::{
    atomic_spawn::{self, Spec},
    configure, Namespaces,
};
use zeroize::Zeroizing;
const SCRIPT: &str = include_str!("../../../../tests/fixtures/paea_namespace_owner.py");
/// Observe the parent effective set, not its capabilities in child namespaces.
fn effective_caps() -> u64 {
    let text = fs::read_to_string("/proc/self/status").unwrap();
    let value = text
        .lines()
        .find_map(|s| s.strip_prefix("CapEff:"))
        .unwrap();
    u64::from_str_radix(value.trim(), 16).unwrap()
}
/// Run one trusted child with actual pidfd ownership, fixed packet phases and
/// explicit reaping/close even after assertion failure. Observed handles compare
/// fixture continuity only, never grant/adopt a first caller as an approved domain.
fn probe(owned_namespace: bool) {
    assert_eq!(effective_caps(), 0, "host capability precondition");
    let (child_end, parent_end) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut channel = configure(&parent_end).unwrap();
    let null = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    // Child native/Python refusal diagnostics flow into retained harness stderr.
    let stderr = std::io::stderr();
    let directory = CString::new("/").unwrap();
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let program = CString::new(if owned_namespace {
        "/usr/bin/unshare"
    } else {
        "/usr/bin/python3"
    })
    .unwrap();
    let args: Vec<CString> = if owned_namespace {
        [
            "unshare",
            "--user",
            "--map-root-user",
            "--mount",
            "/usr/bin/python3",
            "-c",
            SCRIPT,
        ]
        .into_iter()
        .map(|s| CString::new(s).unwrap())
        .collect()
    } else {
        ["python3", "-c", SCRIPT]
            .into_iter()
            .map(|s| CString::new(s).unwrap())
            .collect()
    };
    let spec = Spec {
        program: &program,
        arguments: &args,
        environment: &[],
        directory: &directory,
        stdin: child_end.as_fd(),
        stdout: null.as_fd(),
        stderr: stderr.as_fd(),
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let launched = atomic_spawn::launch(&spec);
    drop(child_end);
    let mut child = match launched {
        Ok(c) => c,
        Err(mut e) => {
            drop(channel);
            drop(parent_end);
            if let Some(mut c) = e.child.take() {
                let waited = c.wait();
                let closed = c.close();
                eprintln!("launch-refusal cleanup_wait={waited:?} cleanup_close={closed:?}")
            };
            panic!("fixture launch refused: {e}")
        }
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut first: Option<Namespaces> = None;
        for (index, stage) in [
            b"before".as_slice(),
            b"nondumpable".as_slice(),
            b"restored".as_slice(),
        ]
        .into_iter()
        .enumerate()
        {
            let packet = channel.receive(Zeroizing::new(vec![0; 128])).unwrap();
            assert_eq!(packet.data(), stage);
            assert_eq!(
                packet.credentials().1,
                nix::unistd::geteuid().as_raw(),
                "same external UID only"
            );
            assert!(child.anchor().matches(&packet).unwrap());
            let observed = packet.namespaces();
            if index == 1 && !owned_namespace {
                let error = observed
                    .err()
                    .expect("parent userns nondumpable must refuse");
                assert!(
                    error
                        .causes()
                        .iter()
                        .any(|(_, e)| e.raw_os_error() == Some(libc::EACCES)),
                    "actual ptrace access denial required"
                );
                println!("NAMESPACE_OWNER host-nondumpable-refused");
            } else {
                let observed = observed.unwrap();
                if index == 0 {
                    let current = fs::metadata("/proc/self/ns/mnt").unwrap();
                    let target = fstat(observed.mount()).unwrap();
                    assert_eq!(
                        (target.st_dev, target.st_ino) != (current.dev(), current.ino()),
                        owned_namespace,
                        "actual private mount required"
                    );
                    first = Some(observed)
                } else {
                    assert!(observed.same_domain(first.as_ref().unwrap()).unwrap());
                    observed.close().unwrap()
                };
                println!(
                    "NAMESPACE_OWNER profile={} stage={} accepted",
                    if owned_namespace {
                        "owned-userns"
                    } else {
                        "host"
                    },
                    index
                );
            }
            packet.close().unwrap();
            channel.send_packet(b"go").unwrap();
        }
        first.take().unwrap().close().unwrap();
    }));
    drop(channel);
    drop(parent_end);
    let waited = child.wait();
    let closed = child.close();
    if let Err(panic) = result {
        eprintln!("fixture cleanup_wait={waited:?} cleanup_close={closed:?}");
        resume_unwind(panic)
    }
    assert!(waited.unwrap().success());
    closed.unwrap();
    assert_eq!(effective_caps(), 0, "parent capability state changed");
}
/// Valid control, actual same-domain ptrace refusal and recovery precede the
/// owned user-namespace case, whose nondumpable child remains inspectable.
#[test]
fn namespace_owner_nondumpable_child_refuses_host_domain_but_accepts_owned_userns() {
    probe(false);
    probe(true);
    println!(
        "NAMESPACE_OWNER_PASS profiles=2 stages=6 external_uid_shared=true host_caps_zero=true"
    );
}

/// Valid packet precedes invalid prctl in both sink/capture configurations.
/// Pipe closure and reaping retain the child's original failure without altering
/// process-wide signal settings or introducing a deadline, retry or product mode.
fn diagnostic_probe(capture: bool) -> Vec<u8> {
    use std::io::Read;
    let (child_end, parent_end) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    )
    .unwrap();
    let mut channel = configure(&parent_end).unwrap();
    let (reader, writer) = nix::unistd::pipe2(nix::fcntl::OFlag::O_CLOEXEC).unwrap();
    let null = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/usr/bin/python3").unwrap();
    let directory = CString::new("/").unwrap();
    let args = ["python3", "-c", SCRIPT, "invalid-dumpable"]
        .into_iter()
        .map(|s| CString::new(s).unwrap())
        .collect::<Vec<_>>();
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let spec = Spec {
        program: &program,
        arguments: &args,
        environment: &[],
        directory: &directory,
        stdin: child_end.as_fd(),
        stdout: null.as_fd(),
        stderr: if capture {
            writer.as_fd()
        } else {
            null.as_fd()
        },
        signal_mask: &mask,
        signal_defaults: &defaults,
    };
    let launched = atomic_spawn::launch(&spec);
    drop(child_end);
    drop(writer);
    let mut child = match launched {
        Ok(child) => child,
        Err(mut error) => {
            drop(channel);
            drop(parent_end);
            drop(reader);
            let had_child = error.child.is_some();
            if let Some(mut child) = error.child.take() {
                let waited = child.wait();
                let closed = child.close();
                eprintln!("diagnostic launch refusal retained_child_initially={had_child} cleanup_wait={waited:?} cleanup_close={closed:?}");
            }
            panic!("diagnostic fixture launch refused: {error}");
        }
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        let p = channel.receive(Zeroizing::new(vec![0; 128])).unwrap();
        assert_eq!(p.data(), b"before-fault");
        assert!(child.anchor().matches(&p).unwrap());
        p.namespaces().unwrap().close().unwrap();
        p.close().unwrap();
        channel.send_packet(b"go").unwrap();
        let after = channel.receive(Zeroizing::new(vec![0; 128]));
        assert!(
            after.is_err(),
            "child error must not become a successful empty packet"
        );
    }));
    drop(channel);
    drop(parent_end);
    let waited = child.wait();
    let closed = child.close();
    let mut raw = Vec::new();
    std::fs::File::from(reader).read_to_end(&mut raw).unwrap();
    if let Err(panic) = result {
        eprintln!("diagnostic cleanup_wait={waited:?} cleanup_close={closed:?}");
        resume_unwind(panic)
    };
    let exit = waited.unwrap();
    assert!(!exit.success());
    assert_eq!(exit.status, 1);
    closed.unwrap();
    raw
}
/// Reproduce lost native failure with null stderr, then capture that same EINVAL
/// after a valid control in each configuration. Original private traceback kept.
#[test]
fn namespace_owner_child_failure_keeps_native_diagnostic() {
    let lost = diagnostic_probe(false);
    assert!(lost.is_empty(), "null-sink control changed");
    let raw = diagnostic_probe(true);
    let text = String::from_utf8(raw).unwrap();
    assert!(text.contains("fixture-set-dumpable"));
    assert!(text.contains(&format!("[Errno {}]", libc::EINVAL)));
    eprintln!("retained private fixture diagnostic:\n{text}");
    println!("NAMESPACE_OWNER_DIAGNOSTIC_PASS controls=2 original_loss_observed=true errno_retained=true");
}
