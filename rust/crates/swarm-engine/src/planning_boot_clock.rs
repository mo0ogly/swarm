//! Linux suspend-aware planner clock and pollable timer; neither grants authority.
use rustix::time::{
    clock_gettime_dynamic, timerfd_create, timerfd_settime, DynamicClockId, Itimerspec,
    TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec,
};
use std::{
    io,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
    time::Duration,
};

/// A monotonic CLOCK_BOOTTIME observation, including suspended time. Values are
/// meaningful only within the same running kernel boot, never civil timestamps,
/// persisted lease identities or proof that a provider was making progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlanningBootInstant(Duration);
impl PlanningBootInstant {
    /// Read the actual fallible kernel clock, without wall-clock/Instant fallback.
    /// Kernel/representation errors preserve their causes and start no process.
    pub fn now() -> io::Result<Self> {
        let value = clock_gettime_dynamic(DynamicClockId::Boottime).map_err(io::Error::from)?;
        let seconds = u64::try_from(value.tv_sec)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let nanos = u32::try_from(value.tv_nsec)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if nanos >= 1_000_000_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "planning-clock-nanoseconds-invalid",
            ));
        }
        Ok(Self(Duration::new(seconds, nanos)))
    }

    /// Return this observed boot-relative duration without IO. It is not a Unix
    /// timestamp and must not be compared across reboots or substituted for a
    /// persisted ownership lease's separately checked timestamp.
    pub fn since_boot(self) -> Duration {
        self.0
    }

    /// Add an explicitly supplied interval without clamping or a policy ceiling.
    /// Refuse Duration overflow; the kernel timer's strict ktime domain is separately
    /// validated before arming. This changes no clock, descriptor or lease.
    pub fn checked_add(self, duration: Duration) -> io::Result<Self> {
        self.0.checked_add(duration).map(Self).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "planning-clock-duration-overflow",
            )
        })
    }

    /// Compute an observed interval within this boot. Reversed observations
    /// refuse instead of saturating to zero or fabricating progress. Callers own
    /// the same-boot/lifetime precondition; there is no wall-clock fallback.
    pub fn duration_since(self, earlier: Self) -> io::Result<Duration> {
        self.0.checked_sub(earlier.0).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "planning-clock-observation-reversed",
            )
        })
    }
}

/// Owned nonblocking, close-on-exec CLOCK_BOOTTIME timerfd. It becomes readable
/// at the next observed deadline, including across suspension; it does not wake
/// a suspended machine, signal a child, renew a lease or stop remote billing.
/// The process owner must poll this descriptor alongside its actual IO/control
/// sources and recheck the current authority/policy on wake. Drop closes best
/// effort; use close to observe errors on normal/failure teardown.
pub struct PlanningBootTimer(OwnedFd);
impl PlanningBootTimer {
    /// Create an initially disarmed kernel timer; return actual kernel errors
    /// without falling back to another clock or creating an execution policy.
    pub fn new() -> io::Result<Self> {
        timerfd_create(
            TimerfdClockId::Boottime,
            TimerfdFlags::NONBLOCK | TimerfdFlags::CLOEXEC,
        )
        .map(Self)
        .map_err(io::Error::from)
    }

    /// Arm exactly one absolute boot deadline, or explicitly disarm with None.
    /// Zero absolute time is refused because the kernel would interpret it as
    /// disarm. Deadlines at or beyond Linux KTIME_SEC_MAX seconds are refused
    /// before syscall, preserving the previous timer: the kernel otherwise
    /// silently saturates them. This is a representation bound, not operator
    /// policy. See <https://github.com/torvalds/linux/blob/v6.17/include/linux/ktime.h>.
    /// A successful kernel rearm clears old counts.
    /// A past positive deadline is due immediately; no minimum delay is inserted.
    /// Kernel failure is returned, never a silent timeout or successful stop.
    pub fn arm(&self, deadline: Option<PlanningBootInstant>) -> io::Result<()> {
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let at = match deadline {
            None => zero,
            Some(at) => {
                if at.0.is_zero() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "planning-clock-zero-deadline",
                    ));
                }
                // ktime_set saturates when seconds >= KTIME_SEC_MAX, including
                // the otherwise representable fractional tail of that second.
                if at.0.as_secs() >= (i64::MAX as u64) / 1_000_000_000 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        PlanningClockFault("planning-clock-kernel-deadline-representation"),
                    ));
                }
                Timespec {
                    tv_sec: at
                        .0
                        .as_secs()
                        .try_into()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
                    tv_nsec: at.0.subsec_nanos().into(),
                }
            }
        };
        timerfd_settime(
            &self.0,
            TimerfdTimerFlags::ABSTIME,
            &Itimerspec {
                it_interval: zero,
                it_value: at,
            },
        )
        .map(|_| ())
        .map_err(io::Error::from)
    }

    /// Consume the kernel's native-endian u64 expiration count. WouldBlock is
    /// explicitly None (no pending wake), not a completed provider or decision.
    /// Preserve interrupted/other IO errors for owner handling; a short read or
    /// zero count is invalid data, never fabricated readiness. No polling occurs.
    pub fn take_expirations(&self) -> io::Result<Option<u64>> {
        let mut bytes = [0u8; 8];
        match rustix::io::read(&self.0, &mut bytes) {
            Err(rustix::io::Errno::AGAIN) => Ok(None),
            Err(error) => Err(error.into()),
            Ok(count) if count == bytes.len() => {
                let count = u64::from_ne_bytes(bytes);
                if count == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "planning-clock-zero-expirations",
                    ));
                }
                Ok(Some(count))
            }
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "planning-clock-short-expirations",
            )),
        }
    }

    /// Consume and close this owned descriptor, retaining any close error. No
    /// retry follows an uncertain close; callers must preserve original teardown
    /// errors separately rather than treating this as child termination proof.
    pub fn close(self) -> io::Result<()> {
        nix::unistd::close(self.0).map_err(io::Error::from)
    }
}
impl AsFd for PlanningBootTimer {
    /// Borrow only this timer's descriptor for an owner-controlled poll. The
    /// borrow grants no process, gate or persisted ownership capability.
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

/// Safe clock representation failure; carries no private kernel or path text.
#[derive(Debug)]
pub(crate) struct PlanningClockFault(pub(crate) &'static str);
impl std::fmt::Display for PlanningClockFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for PlanningClockFault {}
