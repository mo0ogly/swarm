//! Local cooperative stop requests, never process, mission or agent authority.
use std::{
    io::{self, Write},
    os::{
        fd::{AsFd, BorrowedFd},
        unix::net::UnixStream,
    },
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};
/// Closed local transport vocabulary. This is an observation of the caller's
/// stop request, not persisted consent, a timeout, signal permission or a grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionInterruptReason {
    Interrupt,
    Terminate,
    ListenerFailure,
    /// The owning conductor is exiting after an error, not an observed OS signal.
    ControllerShutdown,
}
struct Inner {
    reason: AtomicU8,
    reader: UnixStream,
    writer: UnixStream,
}
/// Cloneable in-process refusal/wakeup source. No serialized value constructs
/// it and it never signals a PID. The caller owns its scope and must still use
/// normal admission, TERM/grace/wait/receipt and close paths. Shared descriptors
/// stay alive through all borrows and close on final drop; no launch channel or
/// capability is published. Same-UID memory/filesystem trust is unchanged.
#[derive(Clone)]
pub struct ExecutionInterrupt(Arc<Inner>);
impl ExecutionInterrupt {
    /// Create nonblocking local CLOEXEC streams. IO failures preserve their cause;
    /// no Root, process, timer, policy, thread or stop request is created.
    pub fn new() -> io::Result<Self> {
        let (reader, writer) = UnixStream::pair()?;
        reader.set_nonblocking(true)?;
        writer.set_nonblocking(true)?;
        Ok(Self(Arc::new(Inner {
            reason: AtomicU8::new(0),
            reader,
            writer,
        })))
    }
    /// Record the first request and wake a borrowed poll. Repeated requests do not
    /// retry a failed publication or change the original reason. An IO error may
    /// follow a recorded request; the original caller must preserve it. Exactly
    /// one byte is a technical wake protocol, not an operator limit or queue.
    pub fn request(&self, reason: ExecutionInterruptReason) -> io::Result<()> {
        let value = match reason {
            ExecutionInterruptReason::Interrupt => 1,
            ExecutionInterruptReason::Terminate => 2,
            ExecutionInterruptReason::ListenerFailure => 3,
            ExecutionInterruptReason::ControllerShutdown => 4,
        };
        if self
            .0
            .reason
            .compare_exchange(0, value, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            (&self.0.writer).write_all(&[1])?;
        }
        Ok(())
    }
    /// Read the observed local reason without IO/mutation. Unknown internal codes
    /// refuse rather than silently permitting continuation. No delivery, cleanup
    /// or permission is established by a nonempty result.
    pub fn reason(&self) -> io::Result<Option<ExecutionInterruptReason>> {
        match self.0.reason.load(Ordering::Acquire) {
            0 => Ok(None),
            1 => Ok(Some(ExecutionInterruptReason::Interrupt)),
            2 => Ok(Some(ExecutionInterruptReason::Terminate)),
            3 => Ok(Some(ExecutionInterruptReason::ListenerFailure)),
            4 => Ok(Some(ExecutionInterruptReason::ControllerShutdown)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "execution-interrupt-state-invalid",
            )),
        }
    }
    /// Borrow a readable wake until the controller enters its existing stop mode.
    /// The byte remains readable; consumers must stop polling this descriptor
    /// after handling it, so existing grace timers do not become a busy loop.
    pub(crate) fn descriptor(&self) -> BorrowedFd<'_> {
        self.0.reader.as_fd()
    }
}
impl std::fmt::Debug for ExecutionInterrupt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExecutionInterrupt(<local>)")
    }
}
/// Interpret only an already observed kernel wait status for clean-stop metadata.
/// Success or the runner's supported TERM/KILL vocabulary may complete a local
/// stop; independent exit codes/signals remain failures. This creates no wait,
/// permission or publication proof and does not classify untrusted stdout.
pub(crate) fn stopped_status(status:&std::process::ExitStatus)->bool{
    use std::os::unix::process::ExitStatusExt;
    status.success() || status.signal().is_some_and(|value|value==rustix::process::Signal::TERM.as_raw()||value==rustix::process::Signal::KILL.as_raw())
}
/// Live role classification, never a serialized grant or operator policy.
#[derive(Clone, Copy)]
pub(crate) enum InterruptedRole {
    Controls,
    Reviewer,
}
/// Private marker constructed by a role ONLY after clean transport and durable
/// failed-call publication. It preserves every original expected failure cause;
/// the outer known Work and actual signal remain the caller's obligations.
/// No constructor validates those facts by itself or grants any authority.
pub(crate) struct CompletedRoleInterrupt {
    role: InterruptedRole,
    primary: Box<crate::RootStoreError>,
}
impl CompletedRoleInterrupt {
    /// Closed diagnostic vocabulary, excluding private role/job/actor data.
    pub(crate) fn role(&self) -> &'static str {
        match self.role {
            InterruptedRole::Controls => "controls",
            InterruptedRole::Reviewer => "reviewer",
        }
    }
}
impl std::fmt::Display for CompletedRoleInterrupt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("execution-interrupted")
    }
}
impl std::fmt::Debug for CompletedRoleInterrupt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::error::Error for CompletedRoleInterrupt {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.primary.as_ref())
    }
}
/// Wrap only the live publisher's proven completed interruption. Original error
/// trees remain inspectable, with safe formatting; this creates no Work or grant.
pub(crate) fn completed(
    role: InterruptedRole,
    primary: crate::RootStoreError,
) -> crate::RootStoreError {
    crate::RootStoreError::Runtime(crate::RuntimeError::new(
        crate::RuntimeOperation::Platform,
        CompletedRoleInterrupt {
            role,
            primary: Box::new(primary),
        },
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    /// Synthetic OS status vocabulary only, not a real process or wait. A valid
    /// completion precedes independent exit/signal refusals and recovery.
    #[test]
    fn local_interrupt_status_rejects_independent_exit_and_signal(){
        use std::os::unix::process::ExitStatusExt;
        assert!(stopped_status(&std::process::ExitStatus::from_raw(0)));
        for raw in [42<<8,rustix::process::Signal::SEGV.as_raw()]{
            assert!(!stopped_status(&std::process::ExitStatus::from_raw(raw)));
            assert!(stopped_status(&std::process::ExitStatus::from_raw(0)));
        }
        for signal in [rustix::process::Signal::TERM,rustix::process::Signal::KILL]{assert!(stopped_status(&std::process::ExitStatus::from_raw(signal.as_raw())));}
    }

    /// Actual readable kernel source, clone lifetime and first-reason retention;
    /// numeric codes/one byte are private transport facts, not product policies.
    #[test]
    fn local_interrupt_wakes_and_retains_first_reason() {
        let source = ExecutionInterrupt::new().unwrap();
        for fd in [&source.0.reader, &source.0.writer] {
            assert!(rustix::io::fcntl_getfd(fd)
                .unwrap()
                .contains(rustix::io::FdFlags::CLOEXEC));
            assert!(rustix::fs::fcntl_getfl(fd)
                .unwrap()
                .contains(rustix::fs::OFlags::NONBLOCK));
        }
        assert_eq!(source.reason().unwrap(), None);
        source.request(ExecutionInterruptReason::Interrupt).unwrap();
        let clone = source.clone();
        drop(source);
        clone.request(ExecutionInterruptReason::Terminate).unwrap();
        assert_eq!(
            clone.reason().unwrap(),
            Some(ExecutionInterruptReason::Interrupt)
        );
        let mut fds = [rustix::event::PollFd::new(
            &clone.0.reader,
            rustix::event::PollFlags::IN,
        )];
        rustix::event::poll(
            &mut fds,
            Some(&rustix::time::Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }),
        )
        .unwrap();
        assert!(fds[0].revents().contains(rustix::event::PollFlags::IN));
    }
    /// A local shutdown remains distinct from an OS signal; first reason wins.
    /// This source test establishes neither process cleanup nor a clean CLI stop.
    #[test]
    fn local_controller_shutdown_is_not_a_signal() {
        let source=ExecutionInterrupt::new().unwrap();
        source.request(ExecutionInterruptReason::ControllerShutdown).unwrap();
        assert_eq!(source.reason().unwrap(),Some(ExecutionInterruptReason::ControllerShutdown));
        source.request(ExecutionInterruptReason::Interrupt).unwrap();
        assert_eq!(source.reason().unwrap(),Some(ExecutionInterruptReason::ControllerShutdown));
    }
    /// A malformed private state is an explicit refusal, never None/permission.
    #[test]
    fn local_interrupt_invalid_state_refuses() {
        let source = ExecutionInterrupt::new().unwrap();
        source.0.reason.store(255, Ordering::Release);
        assert_eq!(
            source.reason().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
