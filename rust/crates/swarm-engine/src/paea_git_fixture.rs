//! Fixture-only observation hooks for the concurrent public mandate
//! withdrawal recette.
//!
//! Profile: declared in `lib.rs` only under
//! `cfg(all(target_os = "linux", feature = "test-fixtures"))`, beside
//! `AtomicStorageFault`. Feature-only, not delivered: with the feature off,
//! or off Linux, every call site compiles completely out and no hook state,
//! default, retry or setting is introduced for non-fixture builds. Actual
//! delivered-profile exclusion requires its separate qualification. The hooks
//! are pure observation points with an abort outcome for the calling
//! production step; they never read or write authority, session, token,
//! configuration or result state and carry no production default.
//!
//! Contract: each kind may be installed at most once per thread and scope.
//! `with_shared_section_hook` scopes one callback consumed by the Commit
//! branch of `call_paea_git_promote` after the final matched agreement and
//! before the commit effect, while the Trust shared reservation is held.
//! `with_ex_lock_hook` scopes one callback consumed by
//! `LocalStorage::reserve_trust` in the exclusive branch only, after the
//! checked file open and before the blocking flock. Registration is refused
//! only before the body runs; the active registration is tracked separately
//! from the taken callback, so nested or reentrant registration of the same
//! kind is refused even after the callback was consumed. Callbacks always
//! run outside the internal `RefCell` borrow. Scope cleanup on normal
//! return and on unwind clears both the callback and the active
//! registration and preserves body outcomes and cleanup panic separately.

use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::{
    any::Any,
    fmt,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};

/// Callback consumed at the Commit shared-section point.
type SharedSectionCallback = Box<dyn FnOnce() -> io::Result<()>>;

/// Callback consumed in the exclusive branch of `reserve_trust`; receives
/// the exact checked production lock file by shared borrow and must not
/// close it.
type ExclusiveLockCallback = Box<dyn FnOnce(&File) -> io::Result<()>>;

/// Per-kind thread-local hook state. `active` tracks the live scope
/// separately from the not-yet-consumed callback so reentrant registration
/// stays refused after consumption.
#[derive(Default)]
struct HookState {
    shared_active: bool,
    shared_callback: Option<SharedSectionCallback>,
    exclusive_active: bool,
    exclusive_callback: Option<ExclusiveLockCallback>,
}

thread_local! {
    static HOOKS: RefCell<HookState> = RefCell::new(HookState::default());
}

/// Fixed refusal returned when a hook of the requested kind is already
/// registered on the current thread, including after its callback was
/// consumed while the scope is still alive. The body never runs on refusal.
/// Carries no dynamic content by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegistrationRefused;

impl RegistrationRefused {
    /// Stable fixed diagnostic code for fixture reporting.
    pub fn code(&self) -> &'static str {
        "paea-git-fixture-hook-registration-refused"
    }
}

/// Original panic payload retained without rendering private capture contents.
pub type PanicPayload = Box<dyn Any + Send>;

/// Both body and capture-destruction panics, retained as one unwind payload.
/// Consumers can downcast and inspect both original payloads after joining.
pub struct HookUnwind {
    pub body: PanicPayload,
    pub cleanup: PanicPayload,
}

/// Registration refusal, or a capture cleanup panic after a returned body.
/// A cleanup failure retains the body's exact typed outcome rather than
/// converting publication into absence. Debug does not render captures.
pub enum ScopeError<T> {
    RegistrationRefused,
    Cleanup { outcome: T, panic: PanicPayload },
}
impl<T> fmt::Debug for ScopeError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RegistrationRefused => "hook-registration-refused",
            Self::Cleanup { .. } => "hook-capture-cleanup-panicked",
        })
    }
}

/// RAII cleanup records capture-destructor panic instead of unwinding again.
/// Extract under the RefCell borrow, destroy outside it while still active,
/// then clear active. Reentrant registration during destruction is refused.
struct ScopeGuard {
    shared: bool,
    cleanup: Rc<RefCell<Option<PanicPayload>>>,
}
impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let (shared, exclusive) = HOOKS.with(|hooks| {
            let mut hooks = hooks.borrow_mut();
            if self.shared {
                (hooks.shared_callback.take(), None)
            } else {
                (None, hooks.exclusive_callback.take())
            }
        });
        let destroyed = catch_unwind(AssertUnwindSafe(|| drop((shared, exclusive))));
        HOOKS.with(|hooks| {
            let mut hooks = hooks.borrow_mut();
            if self.shared {
                hooks.shared_active = false;
            } else {
                hooks.exclusive_active = false;
            }
        });
        if let Err(panic) = destroyed {
            *self.cleanup.borrow_mut() = Some(panic);
        }
    }
}

/// Run a body and cleanup without dropping callback captures under a borrow.
/// Preserve a returned body in Cleanup, or both panic payloads in HookUnwind.
/// There is no cleanup-induced second unwind while the body is unwinding.
fn scoped<T>(guard: ScopeGuard, body: impl FnOnce() -> T) -> Result<T, ScopeError<T>> {
    let cleanup = guard.cleanup.clone();
    let outcome = catch_unwind(AssertUnwindSafe(body));
    drop(guard);
    let cleanup = cleanup.borrow_mut().take();
    match (outcome, cleanup) {
        (Ok(value), None) => Ok(value),
        (Ok(outcome), Some(panic)) => Err(ScopeError::Cleanup { outcome, panic }),
        (Err(body), None) => std::panic::resume_unwind(body),
        (Err(body), Some(cleanup)) => {
            std::panic::resume_unwind(Box::new(HookUnwind { body, cleanup }))
        }
    }
}

/// Install one scoped shared callback; refuse active same-kind registration.
fn install_shared(callback: SharedSectionCallback) -> Result<ScopeGuard, RegistrationRefused> {
    HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        if hooks.shared_active {
            return Err(RegistrationRefused);
        }
        hooks.shared_active = true;
        hooks.shared_callback = Some(callback);
        Ok(ScopeGuard {
            shared: true,
            cleanup: Rc::new(RefCell::new(None)),
        })
    })
}

/// Install one scoped exclusive callback; refuse active same-kind registration.
fn install_exclusive(callback: ExclusiveLockCallback) -> Result<ScopeGuard, RegistrationRefused> {
    HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        if hooks.exclusive_active {
            return Err(RegistrationRefused);
        }
        hooks.exclusive_active = true;
        hooks.exclusive_callback = Some(callback);
        Ok(ScopeGuard {
            shared: false,
            cleanup: Rc::new(RefCell::new(None)),
        })
    })
}

/// Run `body` with exactly one shared-section observation callback installed
/// on the current thread.
///
/// Purpose: scope the callback consumed by the Commit branch of
/// `call_paea_git_promote` at its shared-section point (after the final
/// matched agreement, before the commit effect, while the Trust shared
/// reservation is held).
///
/// Preconditions: no shared-section hook may already be registered on this
/// thread. The callback is parked in thread-local state, so it must be
/// `'static`; it runs at most once, on this thread, outside any internal
/// borrow. The body runs immediately and needs no bound.
///
/// Output: `Ok(body outcome)` after normal cleanup; a capture-drop panic
/// returns the exact body outcome inside `ScopeError::Cleanup`.
///
/// Errors: `Err(ScopeError::RegistrationRefused)` when a shared-section hook is already
/// registered on this thread (including after consumption while the outer
/// scope lives); the body does not run in that case. A panicking body
/// propagates its original panic after cleanup; simultaneous capture-drop panic
/// is retained with it as `HookUnwind`. Cleanup panic after a returned body
/// returns `ScopeError::Cleanup` with the exact body outcome.
///
/// Side effects: thread-local hook state only; cleared on normal return and
/// on unwind. No authority, session, token, configuration or result state is
/// read or written.
///
/// Profile limits: compiled only on Linux with `test-fixtures`;
/// fixture-only observation, not a delivered feature, not a production hook
/// point, policy, retry or setting.
pub fn with_shared_section_hook<T>(
    callback: impl FnOnce() -> io::Result<()> + 'static,
    body: impl FnOnce() -> T,
) -> Result<T, ScopeError<T>> {
    let guard = install_shared(Box::new(callback)).map_err(|_| ScopeError::RegistrationRefused)?;
    scoped(guard, body)
}

/// Run `body` with exactly one exclusive-lock observation callback installed
/// on the current thread.
///
/// Purpose: scope the callback consumed by `LocalStorage::reserve_trust` in
/// its exclusive branch only, after the actual checked file open and before
/// the blocking flock. The callback receives the exact checked production
/// lock file by shared borrow and must not close it; the shared path never
/// invokes it.
///
/// Preconditions: no exclusive-lock hook may already be registered on this
/// thread. The callback is parked in thread-local state, so it must be
/// `'static`; it runs at most once per consumed registration, on this
/// thread, outside any internal borrow. The body runs immediately and needs
/// no bound.
///
/// Output: `Ok(body outcome)` after normal cleanup; a capture-drop panic
/// returns the exact body outcome inside `ScopeError::Cleanup`.
///
/// Errors: `Err(ScopeError::RegistrationRefused)` when an exclusive-lock hook is already
/// registered on this thread (including after consumption while the outer
/// scope lives); the body does not run in that case. A callback failure
/// reaches `reserve_trust` as `Some(Err(..))` and is mapped there into the
/// existing typed `Lock` error with the original cause preserved by
/// `close_after`. A panicking body keeps its payload; a simultaneous
/// capture-drop panic is retained with it as `HookUnwind`.
///
/// Side effects: thread-local hook state only; cleared on normal return and
/// on unwind. No authority, session, token, configuration or result state is
/// read or written.
///
/// Profile limits: compiled only on Linux with `test-fixtures`;
/// fixture-only observation, not a delivered feature, not a production hook
/// point, policy, retry or setting.
pub fn with_ex_lock_hook<T>(
    callback: impl FnOnce(&File) -> io::Result<()> + 'static,
    body: impl FnOnce() -> T,
) -> Result<T, ScopeError<T>> {
    let guard =
        install_exclusive(Box::new(callback)).map_err(|_| ScopeError::RegistrationRefused)?;
    scoped(guard, body)
}

/// Consume the installed shared-section callback, if any, and run it outside
/// the internal borrow.
///
/// Output: `None` when no callback is registered in this fixture build.
/// Non-fixture builds contain neither this module nor its call sites. `Some(result)` is the
/// callback's own outcome; a failed callback aborts the calling production
/// step through that step's existing typed error mapping.
///
/// Side effects: takes the callback out of thread-local state; the scope
/// stays active until its guard drops, so reentrant registration of the same
/// kind remains refused.
///
/// Profile limits: `pub(crate)` runtime plumbing for the gated call sites;
/// compiled out with the module.
pub(crate) fn run_shared_section_hook() -> Option<io::Result<()>> {
    let callback = HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks.shared_callback.take()
    });
    callback.map(|callback| callback())
}

/// Consume the installed exclusive-lock callback, if any, and run it with
/// the exact checked production lock file, outside the internal borrow.
///
/// Output: `None` when no hook is installed — the blocking flock proceeds
/// unchanged. `Some(result)` is the callback's own outcome; the file borrow
/// ends before the blocking flock and the callback must not close the file.
///
/// Side effects: takes the callback out of thread-local state; the scope
/// stays active until its guard drops, so reentrant registration of the same
/// kind remains refused.
///
/// Profile limits: `pub(crate)` runtime plumbing for the gated call sites;
/// compiled out with the module.
pub(crate) fn run_ex_lock_hook(file: &File) -> Option<io::Result<()>> {
    let callback = HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks.exclusive_callback.take()
    });
    callback.map(|callback| callback(file))
}

#[cfg(test)]
mod scoped_hook_guards {
    use super::{
        run_ex_lock_hook, run_shared_section_hook, with_ex_lock_hook, with_shared_section_hook,
        HookUnwind, ScopeError,
    };
    use std::io;
    use std::os::unix::fs::MetadataExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Control: one normal consumption inside the body, absence outside and
    /// clean re-registration after the scope.
    #[test]
    fn shared_hook_consumes_once_inside_body_and_is_absent_outside() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let outcome = with_shared_section_hook(
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            || {
                assert!(matches!(run_shared_section_hook(), Some(Ok(()))));
                7
            },
        );
        assert_eq!(outcome.unwrap(), 7);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(run_shared_section_hook().is_none());
        assert!(with_shared_section_hook(|| Ok(()), || ()).is_ok());
        assert!(run_shared_section_hook().is_none());
    }

    /// Control: an early body error is returned unchanged; the exclusive
    /// callback is not consumed and nothing leaks after the scope.
    #[test]
    fn body_error_is_returned_unchanged_and_nothing_leaks() {
        let outcome: Result<Result<(), &'static str>, ScopeError<Result<(), &'static str>>> =
            with_ex_lock_hook(|_| Ok(()), || Err("body refused"));
        assert_eq!(outcome.unwrap(), Err("body refused"));
        let probe = tempfile::tempfile().unwrap();
        assert!(run_ex_lock_hook(&probe).is_none());
    }

    /// Control: the callback receives the exact file and a failing callback
    /// reports its own failure to the runtime caller.
    #[test]
    fn exclusive_hook_receives_the_exact_file_and_reports_failure() {
        let probe = tempfile::tempfile().unwrap();
        let expected = probe.metadata().unwrap().ino();
        let seen = with_ex_lock_hook(
            move |file| {
                use std::os::unix::fs::MetadataExt;
                if file.metadata().unwrap().ino() != expected {
                    return Err(io::Error::new(io::ErrorKind::Other, "wrong file"));
                }
                Err(io::Error::new(io::ErrorKind::Other, "probe abort"))
            },
            || {
                assert!(matches!(run_ex_lock_hook(&probe), Some(Err(_))));
            },
        );
        assert!(seen.is_ok());
        assert!(run_ex_lock_hook(&probe).is_none());
    }

    /// Control: a panicking body clears the registration during unwind.
    #[test]
    fn body_unwind_clears_registrations() {
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let scoped: Result<(), ScopeError<()>> =
                with_shared_section_hook(|| Ok(()), || panic!("body unwind"));
            let _ = scoped;
        }));
        assert!(unwound.is_err());
        assert!(run_shared_section_hook().is_none());
        assert!(run_ex_lock_hook(&tempfile::tempfile().unwrap()).is_none());
        assert!(with_shared_section_hook(|| Ok(()), || ()).is_ok());
    }

    /// Control: same-kind reentrancy is refused even after consumption while
    /// the other kind stays independent.
    #[test]
    fn reentrant_same_kind_registration_refused_even_after_consumption() {
        let inner = with_shared_section_hook(
            || Ok(()),
            || {
                assert!(matches!(run_shared_section_hook(), Some(Ok(()))));
                let refused: Result<i32, ScopeError<i32>> =
                    with_shared_section_hook(|| Ok(()), || 1);
                assert!(matches!(refused, Err(ScopeError::RegistrationRefused)));
                let other: Result<(), ScopeError<()>> = with_ex_lock_hook(|_| Ok(()), || ());
                assert!(other.is_ok());
                "inner outcome"
            },
        );
        assert_eq!(inner.unwrap(), "inner outcome");
        assert!(run_ex_lock_hook(&tempfile::tempfile().unwrap()).is_none());
    }

    /// Control: registration is thread-local; other threads observe no hook
    /// and register independently.
    #[test]
    fn registration_is_thread_local() {
        let nested = with_shared_section_hook(
            || Ok(()),
            || {
                let inner = std::thread::spawn(|| {
                    assert!(run_shared_section_hook().is_none());
                    with_shared_section_hook(|| Ok(()), || 42).unwrap()
                });
                let joined = inner.join().unwrap();
                assert!(run_shared_section_hook().is_some());
                joined
            },
        );
        assert_eq!(nested.unwrap(), 42);
        assert!(run_shared_section_hook().is_none());
    }
    /// Valid normal cleanup precedes reentrant capture destruction. The old
    /// Drop borrows RefCell across capture drop and fails this guard.
    #[test]
    fn capture_drop_reentry_is_refused_without_refcell_panic() {
        assert!(with_shared_section_hook(|| Ok(()), || 1).is_ok());
        struct Reenter;
        impl Drop for Reenter {
            fn drop(&mut self) {
                assert!(with_shared_section_hook(|| Ok(()), || ()).is_err());
            }
        }
        let captured = Reenter;
        assert!(with_shared_section_hook(
            move || {
                drop(captured);
                Ok(())
            },
            || 7
        )
        .is_ok());
        assert!(with_shared_section_hook(|| Ok(()), || ()).is_ok());
    }

    /// A destructor panic retains a returned body outcome and clears scope.
    #[test]
    fn capture_drop_panic_retains_returned_body() {
        struct Panics;
        impl Drop for Panics {
            fn drop(&mut self) {
                panic!("capture-cleanup");
            }
        }
        let captured = Panics;
        let result = with_shared_section_hook(
            move || {
                drop(captured);
                Ok(())
            },
            || 17,
        );
        let Err(ScopeError::Cleanup { outcome, panic }) = result else {
            panic!("cleanup not retained");
        };
        assert_eq!(outcome, 17);
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"capture-cleanup"));
        assert!(with_shared_section_hook(|| Ok(()), || ()).is_ok());
    }

    /// Catch body panic before cleanup, keeping both payloads without abort.
    #[test]
    fn body_and_capture_panics_are_retained_together() {
        struct Panics;
        impl Drop for Panics {
            fn drop(&mut self) {
                panic!("capture-cleanup");
            }
        }
        let captured = Panics;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_shared_section_hook(
                move || {
                    drop(captured);
                    Ok(())
                },
                || panic!("body-original"),
            )
        }));
        let panic = result.err().expect("body must unwind");
        let both = panic
            .downcast::<HookUnwind>()
            .ok()
            .expect("both payloads retained");
        assert_eq!(both.body.downcast_ref::<&str>(), Some(&"body-original"));
        assert_eq!(
            both.cleanup.downcast_ref::<&str>(),
            Some(&"capture-cleanup")
        );
        assert!(with_shared_section_hook(|| Ok(()), || ()).is_ok());
    }
}
