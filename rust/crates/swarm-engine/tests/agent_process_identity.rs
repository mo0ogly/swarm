//! Actual kernel observations are not authenticated agent execution authority.
#![cfg(target_os = "linux")]
use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};
use swarm_engine::observe_linux_process;
/// Observe an actual child held on stdin, including its own dedicated process
/// group. Close the gate and wait explicitly; observations confer no signal right.
#[test]
fn child_process_observation_has_actual_parent_group_stamp_and_safe_debug() {
    let mut child = Command::new("/bin/cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .unwrap();
    let observed = observe_linux_process(child.id());
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());
    let observed = observed.unwrap();
    assert_eq!(observed.pid(), child.id());
    assert_eq!(observed.parent_pid(), std::process::id());
    assert_eq!(observed.process_group_id(), child.id());
    assert!(!observed.process_stamp().is_empty());
    assert!(!format!("{observed:?}").contains(observed.process_stamp()));
}
/// Invalid PID representations refuse after a current-process positive control;
/// no path/empty stamp fallback occurs and subsequent kernel observation recovers.
#[test]
fn process_observation_invalid_pid_never_becomes_empty_identity() {
    for invalid in [0, u32::MAX] {
        assert_eq!(
            observe_linux_process(std::process::id()).unwrap().pid(),
            std::process::id()
        );
        let error = observe_linux_process(invalid).unwrap_err();
        assert_eq!(error.code(), "supervisor-identity-unavailable");
        assert_eq!(
            observe_linux_process(std::process::id()).unwrap().pid(),
            std::process::id()
        );
    }
}
/// Kernel pidfd readiness proves an owned unreaped leader exited. Such a zombie
/// refuses live identity; only then reap it. No sleep, arbitrary deadline or PID
/// reuse assumption is used, and the fixture never signals another process.
#[test]
fn unreaped_zombie_is_not_a_live_process_identity() {
    use rustix::{
        event::{poll, PollFd, PollFlags},
        process::{pidfd_open, Pid, PidfdFlags},
    };
    assert!(observe_linux_process(std::process::id()).is_ok());
    let mut child = Command::new("/bin/true").spawn().unwrap();
    let pid = Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap();
    let fd = pidfd_open(pid, PidfdFlags::empty()).unwrap();
    let mut polls = [PollFd::new(&fd, PollFlags::IN)];
    loop {
        match poll(&mut polls, None) {
            Err(rustix::io::Errno::INTR) => continue,
            Ok(_) => break,
            Err(e) => panic!("pidfd observation: {e}"),
        }
    }
    assert!(polls[0].revents().contains(PollFlags::IN));
    let observed = observe_linux_process(child.id());
    assert!(child.wait().unwrap().success());
    nix::unistd::close(fd).unwrap();
    assert!(observed.is_err());
    assert!(observe_linux_process(std::process::id()).is_ok());
}
