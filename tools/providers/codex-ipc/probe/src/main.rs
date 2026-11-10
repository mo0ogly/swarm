//! Source-level reproducer for Codex's Tokio signal/child driver under the
//! unchanged Swarm profile. Only synthetic caller-owned roots are accepted by
//! the surrounding recipe. No provider, model, timeout, retry or acceptance.
use std::{error::Error, future::Future, path::Path, task::Poll};

/// Enter the existing profile before runtime construction, then receive signals
/// and reap a real child in two successive reactors. The second reactor checks
/// the shared global receiver/dup lifecycle. IO/kernel failures propagate;
/// unmodified Tokio intentionally panics at its UnixStream construction.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(args.get(1).ok_or("root required")?);
    let workspace = Path::new(args.get(2).ok_or("workspace required")?);
    let confined = args.get(3).ok_or("mode required")? == "confined";
    if confined {
        swarm_biba_linux::Sandbox::prepare(root, workspace)?.enforce()?;
    }
    println!("IPC_PROBE profile-ready confined={confined}");
    let mut first_fd_count = None;
    for stage in ["first-reactor", "recreated-reactor"] {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let mut signal = tokio::signal::unix::signal(
                tokio::signal::unix::SignalKind::user_defined1(),
            )?;
            println!("IPC_PROBE {stage} signal-registered");
            // SAFETY: SIGUSR1 targets this owned process after registration.
            if unsafe { libc::kill(libc::getpid(), libc::SIGUSR1) } != 0 {
                return Err(std::io::Error::last_os_error());
            }
            signal.recv().await.ok_or_else(|| std::io::Error::other("signal stream closed"))?;
            // Poll the actual ctrl_c API until it has registered, without a
            // sleep or assumed scheduler ordering, then deliver SIGINT.
            let mut interrupt = Box::pin(tokio::signal::ctrl_c());
            std::future::poll_fn(|cx| match interrupt.as_mut().poll(cx) {
                Poll::Pending => Poll::Ready(Ok(())),
                Poll::Ready(_) => Poll::Ready(Err(std::io::Error::other("unexpected early SIGINT"))),
            }).await?;
            // SAFETY: this process registered the current SIGINT receiver.
            if unsafe { libc::kill(libc::getpid(), libc::SIGINT) } != 0 {
                return Err(std::io::Error::last_os_error());
            }
            interrupt.await?;
            let status = tokio::process::Command::new("/usr/bin/true").status().await?;
            if !status.success() {
                return Err(std::io::Error::other("child status failed"));
            }
            Ok::<_, std::io::Error>(())
        })?;
        drop(runtime);
        let descriptors = std::fs::read_dir("/proc/self/fd")?.count();
        if let Some(first) = first_fd_count {
            if descriptors != first {
                return Err("reactor recreation leaked descriptors".into());
            }
        } else {
            first_fd_count = Some(descriptors);
        }
        println!("IPC_PROBE {stage} signal-and-child PASS");
    }
    println!("IPC_PROBE actual-SIGINT-and-fd-lifecycle PASS");
    let stream = std::os::unix::net::UnixStream::pair();
    let datagram = std::os::unix::net::UnixDatagram::pair();
    let outside = std::fs::write(root.join("outside-effect"), b"UNTRUSTED_EFFECT");
    if confined {
        if stream.err().and_then(|e| e.raw_os_error()) != Some(libc::EPERM)
            || datagram.err().and_then(|e| e.raw_os_error()) != Some(libc::EPERM)
            || outside.is_ok()
        {
            return Err("existing boundary weakened".into());
        }
        println!("IPC_PROBE unix-and-outside-refusals PASS");
    } else if stream.is_err() || datagram.is_err() || outside.is_err() {
        return Err("unconfined control invalid".into());
    }
    std::fs::write(workspace.join("ipc-result"), b"SIGNALS_AND_CHILDREN_OK\n")?;
    println!("IPC_PROBE PASS");
    Ok(())
}
