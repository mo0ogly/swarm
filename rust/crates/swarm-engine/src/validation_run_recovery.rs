//! Nonblocking recovery of actual retained children, never a PID signaling API.
/// Traverse the real native error graph, including independent cleanup/custom IO
/// branches and known concrete boxes. Cycles conservatively prevent release.
/// This is child ownership evidence only, never OS containment or retry authority.
pub(crate) fn has_unresolved_process(error: &(dyn Error + 'static)) -> bool {
    fn visit(error: &(dyn Error + 'static), path: &mut Vec<*const (dyn Error + 'static)>) -> bool {
        let id = error as *const dyn Error;
        if path.iter().any(|p| std::ptr::eq(*p, id)) {
            return true;
        }
        path.push(id);
        let here = error
            .downcast_ref::<crate::ValidationRunError>()
            .is_some_and(|e| {
                e.unresolved_process_id().is_some() || e.ownership_lost_process_id().is_some()
            });
        let found = here
            || crate::native_error_links(error)
                .children
                .into_iter()
                .any(|(_, e)| visit(e, path));
        path.pop();
        found
    }
    visit(error, &mut vec![])
}

use super::ValidationRunError;
use crate::{AgentLaunchError, GitReportError, RootStoreError, SettingCount};
use std::{error::Error, io};
/// Poll retained children in the listed native Git/Root wrappers. This never
/// starts, signals or retries a command. Successful try_wait records its actual
/// status and releases that Child; ECHILD records ownership loss and cannot be
/// used to release dependent resources. Other errors leave the Child retained.
/// Unknown wrapper families are not traversed mutably; callers must separately
/// inspect the full native error graph and refuse close while any child is open.
/// This proves leader reap only, not containment/termination of escaped children.
pub(crate) fn poll_retained_processes(e: &mut (dyn Error + 'static)) -> Vec<io::Error> {
    if let Some(e) = e.downcast_mut::<ValidationRunError>() {
        let Some(child) = e.unreaped.as_mut() else {
            return vec![];
        };
        let id = child.id();
        match child.try_wait() {
            Ok(Some(status)) => {
                e.reaped_status = Some(status);
                e.unreaped = None;
                vec![]
            }
            Ok(None) => vec![],
            Err(error) => {
                if error.raw_os_error() == Some(nix::errno::Errno::ECHILD as i32) {
                    e.ownership_lost_id = Some(id);
                    e.unreaped = None;
                }
                vec![error]
            }
        }
    } else if let Some(e) = e.downcast_mut::<RootStoreError>() {
        match e {
            RootStoreError::Launch(e) => poll_retained_processes(e.as_mut()),
            RootStoreError::Cleanup { primary, cleanup } => {
                let mut first = match primary {
                    Some(e) => poll_retained_processes(e.as_mut()),
                    None => vec![],
                };
                let second = poll_retained_processes(cleanup.as_mut());
                {
                    first.extend(second);
                    first
                }
            }
            // Recorded/published/busy wrappers retain an arbitrary original
            // cause that may hold a spawned child; leaving them untraversed
            // made such jobs unclosable (close refuses while a child is open).
            // Purely additive traversal; no semantic change for covered forms.
            RootStoreError::WorkRecorded { primary, .. }
            | RootStoreError::TrustPublished { primary, .. }
            | RootStoreError::PreparationRecorded { primary, .. }
            | RootStoreError::LifecycleRecorded { primary }
            | RootStoreError::AuxiliaryRecorded { primary, .. }
            | RootStoreError::Published { primary } => poll_retained_processes(primary.as_mut()),
            // Busy wraps a WorkEngineError, not a RootStoreError; it is not
            // traversed here (no owned Child is retained in that cause type).
            RootStoreError::AgentStorageBusy { cause, .. } => {
                poll_retained_processes(cause.as_mut())
            }
            RootStoreError::SqlTransaction(e) => poll_retained_processes(e.as_mut()),
            _ => vec![],
        }
    } else if let Some(e) = e.downcast_mut::<AgentLaunchError>() {
        match e {
            AgentLaunchError::Cause { cause, .. } => poll_retained_processes(cause.as_mut()),
            AgentLaunchError::Busy { cause, .. } => poll_retained_processes(cause.as_mut()),
            AgentLaunchError::Cleanup { primary, cleanup } => {
                let mut first = poll_retained_processes(primary.as_mut());
                let second = poll_retained_processes(cleanup.as_mut());
                {
                    first.extend(second);
                    first
                }
            }
            AgentLaunchError::Commit { cause, cleanup, .. } => {
                let mut first = poll_retained_processes(cause.as_mut());
                let second = match cleanup {
                    Some(e) => poll_retained_processes(e.as_mut()),
                    None => vec![],
                };
                {
                    first.extend(second);
                    first
                }
            }
            _ => vec![],
        }
    } else if let Some(e) = e.downcast_mut::<GitReportError>() {
        match e {
            GitReportError::Process { cause, .. } => poll_retained_processes(cause.as_mut()),
            _ => vec![],
        }
    } else if let Some(e) = e.downcast_mut::<crate::GitPackError>() {
        match e.process_error_mut() {
            Some(e) => poll_retained_processes(e),
            None => vec![],
        }
    } else if let Some(e) = e.downcast_mut::<io::Error>() {
        match e.get_mut() {
            Some(e) => poll_retained_processes(e),
            None => vec![],
        }
    } else if let Some(e) = e.downcast_mut::<Box<RootStoreError>>() {
        poll_retained_processes(e.as_mut())
    } else if let Some(e) = e.downcast_mut::<Box<AgentLaunchError>>() {
        poll_retained_processes(e.as_mut())
    } else if let Some(e) = e.downcast_mut::<Box<GitReportError>>() {
        poll_retained_processes(e.as_mut())
    } else if let Some(e) = e.downcast_mut::<Box<ValidationRunError>>() {
        poll_retained_processes(e.as_mut())
    } else if let Some(e) = e.downcast_mut::<Box<crate::GitPackError>>() {
        poll_retained_processes(e.as_mut())
    } else {
        vec![]
    }
}
/// Attach a real owned fixture child to a synthetic error for owner tests only.
/// This is not evidence that the production runner naturally took this path.
#[cfg(test)]
pub(crate) fn retained_child_for_owner_test() -> ValidationRunError {
    let child = std::process::Command::new("true")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut e = ValidationRunError::new("recovery-fixture", io::Error::other("retained-fixture"));
    e.unreaped = Some(Box::new(child));
    e
}
#[cfg(test)]
mod tests {
    use super::*;
    /// A real owned child is attached to a synthetic error wrapper for adapter
    /// testing. This does not claim the production runner took this failure path.
    fn retained() -> ValidationRunError {
        retained_child_for_owner_test()
    }
    /// Poll the actual child; repeated observation waits for kernel progress without
    /// a whole-test deadline or forged success receipt. No product policy changes.
    fn reap(e: &mut (dyn Error + 'static)) {
        loop {
            assert!(poll_retained_processes(e).is_empty());
            if !has_unresolved_process(e) {
                break;
            }
            std::thread::yield_now();
        }
    }
    /// Direct/custom-IO/boxed/Root and two independent launch branches are retained
    /// and actually reaped. Original error text remains an error after recovery.
    #[test]
    fn retained_process_recovery_wrapper_matrix() {
        let mut direct = retained();
        assert!(direct.unresolved_process_id().is_some());
        reap(&mut direct);
        assert!(direct.reaped_exit_status().unwrap().success());
        let mut boxed: Box<ValidationRunError> = Box::new(retained());
        reap(&mut boxed);
        assert!(boxed.reaped_exit_status().unwrap().success());
        let mut io = io::Error::other(retained());
        reap(&mut io);
        assert!(io
            .get_ref()
            .unwrap()
            .downcast_ref::<ValidationRunError>()
            .unwrap()
            .reaped_exit_status()
            .unwrap()
            .success());
        let report = GitReportError::Process {
            cause: Box::new(retained()),
            output: Box::new(crate::GitCommandOutput::from_parts(vec![], vec![])),
        };
        let mut native = RootStoreError::Launch(Box::new(AgentLaunchError::Cause {
            code: "managed-git-process",
            cause: Box::new(report),
        }));
        assert!(has_unresolved_process(&native));
        reap(&mut native);
        assert!(!has_unresolved_process(&native));
        let mut root = RootStoreError::Cleanup {
            primary: Some(Box::new(RootStoreError::Launch(Box::new(
                AgentLaunchError::Cause {
                    code: "fixture-primary",
                    cause: Box::new(retained()),
                },
            )))),
            cleanup: Box::new(RootStoreError::Launch(Box::new(AgentLaunchError::Cause {
                code: "fixture-cleanup",
                cause: Box::new(retained()),
            }))),
        };
        assert!(has_unresolved_process(&root));
        reap(&mut root);
        assert!(!has_unresolved_process(&root));
        assert_eq!(root.code(), "root-cleanup");
    }
    /// Recorded/published/busy wrappers were previously untraversed: a retained
    /// child inside their primary/cause made the owning job unclosable. Each
    /// wrapper form must now expose its real inner child to the poller.
    #[test]
    fn retained_process_recorded_wrappers_recovered() {
        let mut cases: Vec<(&'static str, RootStoreError)> = vec![
            (
                "work-recorded",
                RootStoreError::WorkRecorded {
                    primary: Box::new(RootStoreError::Launch(Box::new(
                        AgentLaunchError::Cause {
                            code: "fixture-work",
                            cause: Box::new(retained()),
                        },
                    ))),
                    work: "fixture".into(),
                    revision: 1,
                },
            ),
            (
                "trust-published",
                RootStoreError::TrustPublished {
                    primary: Box::new(RootStoreError::Launch(Box::new(
                        AgentLaunchError::Cause {
                            code: "fixture-trust",
                            cause: Box::new(retained()),
                        },
                    ))),
                    publication: "fixture",
                },
            ),
            (
                "agent-storage-busy",
                RootStoreError::AgentStorageBusy {
                    cause: Box::new(RootStoreError::Launch(Box::new(
                        AgentLaunchError::Cause {
                            code: "fixture-busy",
                            cause: Box::new(retained()),
                        },
                    ))),
                    attempts: 1,
                    settings_revision: SettingCount(1),
                },
            ),
        ];
        for (name, case) in cases.iter_mut() {
            assert!(has_unresolved_process(case), "{name}");
            reap(case);
            assert!(!has_unresolved_process(case), "{name}");
        }
    }
    /// Lost ownership is never converted to an owned Child or a reap receipt.
    #[test]
    fn retained_process_lost_ownership_stays_blocked() {
        let mut e = retained();
        let id = e.unresolved_process_id().unwrap();
        nix::sys::wait::waitpid(nix::unistd::Pid::from_raw(i32::try_from(id).unwrap()), None)
            .unwrap();
        let errors = poll_retained_processes(&mut e);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].raw_os_error(),
            Some(nix::errno::Errno::ECHILD as i32)
        );
        assert_eq!(e.ownership_lost_process_id(), Some(id));
        assert!(e.unresolved_process_id().is_none());
        assert!(e.reaped_exit_status().is_none());
        assert!(has_unresolved_process(&e));
    }
    /// An unregistered mutable adapter is not silently treated as recovered:
    /// its declared read-only source still protects dependent job resources.
    #[test]
    fn retained_process_unknown_wrapper_stays_blocked() {
        #[derive(Debug)]
        struct Opaque(ValidationRunError);
        impl std::fmt::Display for Opaque {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("opaque-fixture")
            }
        }
        impl Error for Opaque {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                Some(&self.0)
            }
        }
        let mut error = Opaque(retained());
        assert!(has_unresolved_process(&error));
        assert!(poll_retained_processes(&mut error).is_empty());
        assert!(has_unresolved_process(&error));
        reap(&mut error.0);
        assert!(!has_unresolved_process(&error));
    }
}
