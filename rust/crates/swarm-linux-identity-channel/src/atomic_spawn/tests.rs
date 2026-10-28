//! Post-native failure aggregation only; actual cleanup error is synthetic,
//! while the returned child descriptor and exit are from a real native launch.
use super::*;
use std::{ffi::CString, fs::OpenOptions, os::fd::AsFd};
/// Explicit harmless fast program; no inherited environment or operator defaults.
fn with_spec<T>(run: impl FnOnce(&Spec<'_>) -> T) -> T {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/null")
        .unwrap();
    let program = CString::new("/usr/bin/true").unwrap();
    let directory = CString::new("/").unwrap();
    let args = [CString::new("true").unwrap()];
    let mask = SigSet::empty();
    let defaults = SigSet::all();
    let spec = Spec {
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
    run(&spec)
}
/// A valid owner precedes injected post-call cause. Complete child ownership
/// survives, can be waited/reaped and closed; no actual OS destroy failure claim.
#[test]
fn atomic_post_spawn_error_retains_actual_child_and_all_causes() {
    with_spec(|spec| {
        let mut c = launch(spec).unwrap();
        assert!(c.wait().unwrap().success());
        c.close().unwrap()
    });
    let mut error = with_spec(|spec| {
        let mut attempt = kernel::atomic_spawn::run(spec);
        assert!(attempt.primary.is_none() && attempt.process.is_some());
        attempt.primary = Some(Error::refused("fixture-post-spawn"));
        attempt.cleanup.attributes = Some(Error::syscall(
            "fixture-attributes-cleanup",
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "synthetic cleanup"),
        ));
        finish(attempt).err().unwrap()
    });
    assert!(error.invoked && error.child.is_some());
    assert_eq!(error.error_branches().len(), 2);
    let mut child = error.child.take().unwrap();
    assert!(child.wait().unwrap().success());
    child.close().unwrap();
    assert_eq!(error.cause.code(), "fixture-post-spawn");
    assert!(error.cleanup.attributes.is_some());
}
