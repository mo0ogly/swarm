#![cfg(target_os = "linux")]
use std::{os::fd::AsFd, time::Duration};
use swarm_engine::{PlanningBootInstant, PlanningBootTimer};

/// Observe actual Linux BOOTTIME bracketed by two raw kernel reads. Check
/// same-boot arithmetic and reject reversed/overflowing inputs without IO mutation.
#[test]
fn planning_boot_clock_matches_kernel_and_rejects_invalid_order_or_overflow() {
    let raw_before =
        rustix::time::clock_gettime_dynamic(rustix::time::DynamicClockId::Boottime).unwrap();
    let before = PlanningBootInstant::now().unwrap();
    let raw_after =
        rustix::time::clock_gettime_dynamic(rustix::time::DynamicClockId::Boottime).unwrap();
    let raw_duration = |t: rustix::time::Timespec| {
        Duration::new(t.tv_sec.try_into().unwrap(), t.tv_nsec.try_into().unwrap())
    };
    assert!(raw_duration(raw_before) <= before.since_boot());
    assert!(before.since_boot() <= raw_duration(raw_after));
    let after = PlanningBootInstant::now().unwrap();
    let elapsed = after.duration_since(before).unwrap();
    let future = after.checked_add(Duration::from_secs(1)).unwrap();
    assert!(after.duration_since(future).is_err());
    assert!(after.checked_add(Duration::MAX).is_err());
    assert_eq!(
        future.duration_since(after).unwrap(),
        Duration::from_secs(1)
    );
    assert!(future.duration_since(before).unwrap() >= elapsed);
}

/// Create only a fixture-owned timerfd and inspect actual descriptor flags.
/// No deadline must remain unreadable; disarm and checked close must succeed.
#[test]
fn planning_boot_timer_is_nonblocking_cloexec_and_disabled_without_deadline() {
    use rustix::{
        fs::{fcntl_getfl, OFlags},
        io::{fcntl_getfd, FdFlags},
        time::timerfd_gettime,
    };
    let timer = PlanningBootTimer::new().unwrap();
    assert!(fcntl_getfd(timer.as_fd())
        .unwrap()
        .contains(FdFlags::CLOEXEC));
    assert!(fcntl_getfl(timer.as_fd())
        .unwrap()
        .contains(OFlags::NONBLOCK));
    assert_eq!(timerfd_gettime(timer.as_fd()).unwrap().it_value.tv_sec, 0);
    assert_eq!(timer.take_expirations().unwrap(), None);
    timer.arm(None).unwrap();
    assert_eq!(timer.take_expirations().unwrap(), None);
    timer.close().unwrap();
}

/// Wait on a real past absolute kernel deadline, consume once, then disarm.
/// Before representation fault, arm a valid future fixture and require it stays
/// armed after refusal. No provider, sleep or test-wide watchdog is involved.
#[test]
fn planning_boot_timer_absolute_past_deadline_consumes_once_and_disarms() {
    use rustix::event::{poll, PollFd, PollFlags};
    let timer = PlanningBootTimer::new().unwrap();
    // A real previously observed instant is already due. Wait for that kernel
    // contract, without a test-wide watchdog or arbitrary sleep/attempt ceiling.
    let past = PlanningBootInstant::now().unwrap();
    timer.arm(Some(past)).unwrap();
    let mut ready = [PollFd::new(&timer, PollFlags::IN)];
    loop {
        match poll(&mut ready, None) {
            Err(rustix::io::Errno::INTR) => continue,
            result => {
                assert_eq!(result.unwrap(), 1);
                break;
            }
        }
    }
    assert!(ready[0].revents().contains(PollFlags::IN));
    assert_eq!(timer.take_expirations().unwrap(), Some(1));
    assert_eq!(timer.take_expirations().unwrap(), None);
    let future = PlanningBootInstant::now()
        .unwrap()
        .checked_add(Duration::from_secs(60))
        .unwrap();
    timer.arm(Some(future)).unwrap();
    // This is a representation rejection, never a policy ceiling or clamping.
    let impossible = PlanningBootInstant::now()
        .unwrap()
        .checked_add(Duration::from_secs(u64::MAX / 2 + 1))
        .unwrap();
    assert!(timer.arm(Some(impossible)).is_err());
    let remaining = rustix::time::timerfd_gettime(timer.as_fd()).unwrap();
    assert!(
        remaining.it_value.tv_sec > 0,
        "Invalid arm erased the previous deadline"
    );
    timer.arm(None).unwrap();
    assert_eq!(timer.take_expirations().unwrap(), None);
    assert_eq!(
        rustix::time::timerfd_gettime(timer.as_fd())
            .unwrap()
            .it_value
            .tv_sec,
        0
    );
    timer.close().unwrap();
}

/// Validate a genuine future timer before the kernel's signed-nanosecond
/// conversion boundary. An in-range timespec in the saturation second must be
/// refused BEFORE replacing that timer; the last strictly representable instant
/// must still arm exactly. No sleep or future-deadline watchdog is introduced.
#[test]
fn planning_boot_timer_rejects_kernel_saturation_before_mutating_prior_timer() {
    use rustix::time::timerfd_gettime;
    let timer = PlanningBootTimer::new().unwrap();
    let now = PlanningBootInstant::now().unwrap();
    timer
        .arm(Some(now.checked_add(Duration::from_secs(60)).unwrap()))
        .unwrap();
    let before = timerfd_gettime(timer.as_fd()).unwrap();
    assert!(before.it_value.tv_sec > 0);
    // Linux ktime_set saturates for seconds >= KTIME_SEC_MAX even though the
    // supplied tv_sec still fits i64. This is kernel representation, not policy.
    let boundary = Duration::from_secs((i64::MAX as u64) / 1_000_000_000);
    let bad = now
        .checked_add(boundary.checked_sub(now.since_boot()).unwrap())
        .unwrap();
    assert!(
        timer.arm(Some(bad)).is_err(),
        "Kernel silently changed an accepted absolute deadline"
    );
    let after = timerfd_gettime(timer.as_fd()).unwrap();
    assert!(after.it_value.tv_sec > 0 && after.it_value.tv_sec <= before.it_value.tv_sec);
    let exact = boundary - Duration::from_nanos(1);
    timer
        .arm(Some(
            now.checked_add(exact.checked_sub(now.since_boot()).unwrap())
                .unwrap(),
        ))
        .unwrap();
    let lower = PlanningBootInstant::now().unwrap().since_boot();
    let value = timerfd_gettime(timer.as_fd()).unwrap().it_value;
    let upper = PlanningBootInstant::now().unwrap().since_boot();
    let remaining = Duration::new(
        value.tv_sec.try_into().unwrap(),
        value.tv_nsec.try_into().unwrap(),
    );
    assert!(lower + remaining <= exact && exact <= upper + remaining);
    timer.arm(None).unwrap();
    timer.close().unwrap();
}
