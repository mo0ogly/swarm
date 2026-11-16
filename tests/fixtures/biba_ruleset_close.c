#define _GNU_SOURCE
/* Fixture-only close observer: never load into the product/service.
 * Only the exact anonymous ruleset tag is observed. A healthy child must prove
 * actual close interposition before a separate child may inject. This source
 * does not establish the running kernel's tag. No syscall/fcntl varargs hook.
 */
#include <dlfcn.h>
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static int (*real_close)(int);
static int event_fd = -1, mode, phase, failed, injected_low, injected_high;
static const char tag[] = "anon_inode:[landlock-ruleset]";

/* Resolve once before driver main; never open a file or recurse from close.
 * Malformed fixture configuration exits only this owned child, with no retry.
 */
__attribute__((constructor)) static void initialize(void) {
    real_close = (int (*)(int))dlsym(RTLD_NEXT, "close");
    const char *raw = getenv("SWARM_BIBA_CLOSE_EVENTS_FD");
    const char *which = getenv("SWARM_BIBA_CLOSE_MODE");
    if (!real_close || !raw || !which) _exit(112);
    char *end = NULL; errno = 0; long value = strtol(raw, &end, 10);
    if (errno || !end || *end || value <= 2 || value > INT_MAX) _exit(112);
    event_fd = (int)value;
    if (!strcmp(which, "control")) mode = 0;
    else if (!strcmp(which, "original")) mode = 1;
    else if (!strcmp(which, "both")) mode = 2;
    else _exit(112);
}

/* Driver phases are a closed fixture protocol: 0 setup,1 prepare,2 unused-close,
 * 3 restoration. This export grants no product API or process authority.
 */
int swarm_biba_close_probe_phase(int next) {
    if (next < 0 || next > 3) { failed = 1; return -1; }
    phase = next; return 0;
}
int swarm_biba_close_probe_status(void) { return failed; }

/* Match a kernel-object kind, not an arbitrary filename or descriptor number.
 * The witness must start with no ruleset, observe one low ruleset close, and
 * locate exactly one high ruleset afterwards. A tag mismatch blocks the case.
 */
static int ruleset(int fd) {
    char path[sizeof("/proc/self/fd/") + sizeof(int) * 3 + 2];
    char target[sizeof(tag)];
    int length = snprintf(path, sizeof(path), "/proc/self/fd/%d", fd);
    if (length < 0 || (size_t)length >= sizeof(path)) { failed = 1; return 0; }
    ssize_t count = readlink(path, target, sizeof(target));
    return count == (ssize_t)(sizeof(tag) - 1) && !memcmp(target, tag, sizeof(tag) - 1);
}

/* Each event describes a real close attempt and the result delivered to Rust.
 * Writes may be partial; output failure marks the proof incomplete, never PASS.
 * No raw path, private value or strerror is rendered.
 */
static void event(int fd, int real_rc, int real_errno, int delivered, int synthetic) {
    char text[256];
    int n = snprintf(text, sizeof(text), "close\t%d\t%d\t%d\t%d\t%d\t%d\n",
                     phase, fd, real_rc, real_errno, delivered, synthetic);
    if (n < 0 || (size_t)n >= sizeof(text)) { failed = 1; return; }
    size_t at = 0;
    while (at < (size_t)n) {
        ssize_t wrote = write(event_fd, text + at, (size_t)n - at);
        if (wrote <= 0) { failed = 1; return; }
        at += (size_t)wrote;
    }
}

/* Call the real close exactly once. Synthetic EIO/EBADF is delivered ONLY after
 * real close returned0; it is not an actual OS close failure or leak proof.
 * Never retry a descriptor that might have been recycled. Other closes retain
 * their exact return/errno, and no other process or parent's fd table is touched.
 */
int close(int fd) {
    if (!real_close) _exit(112);
    int before_errno = errno;
    int observed = (phase == 1 || phase == 2) && fd >= 0 && ruleset(fd);
    errno = before_errno;
    int rc = real_close(fd);
    int actual_errno = rc < 0 ? errno : 0;
    int delivered = actual_errno, synthetic = 0;
    if (observed && rc == 0 && mode && fd < 3 && !injected_low) {
        injected_low = 1; delivered = EIO; synthetic = 1;
    } else if (observed && rc == 0 && mode == 2 && fd >= 3 && injected_low && !injected_high) {
        injected_high = 1; delivered = EBADF; synthetic = 1;
    }
    if (observed) event(fd, rc, actual_errno, delivered, synthetic);
    errno = synthetic ? delivered : (rc < 0 ? actual_errno : before_errno);
    return synthetic ? -1 : rc;
}
