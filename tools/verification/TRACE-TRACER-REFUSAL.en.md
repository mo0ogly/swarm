# Tracer refusal, primary unknown

`run_tracer_refusal(out, admission, strace, python)` retains a tracer refusal
requested with output under a regular file. A fresh exit0 control precedes the
fault and fresh exit0 recovery follows, through the unchanged process nucleus.
The primary remains UNKNOWN and capture incomplete: missing trace/empty stdout
never establishes that it did not execute.

Caller requirements: qualified storage/FD/admission,20GiB host/output reserve,
fresh private0700leaf, umask077 and stable source/helper/tool pins. String sizing
derives from the declared argv/env only; no parent environment is inherited.
Exclusive0600files are retained without retry or timeout.

The native receipt describes the directly launched tracer. Only its sole expected
numeric ValueError is allowed, with a real positive code, no signal/extra issue,
successful publication/post-admission and intact channels. Stdout/stderr facts
are retained without parsing error words. Blocker/fault receipts must be unchanged
after recovery. Later validation, recovery or publication failure groups both
errors with a native carrier containing earlier_expected_tracer_refusal and
following_failure; the newer diagnostic does not erase the earlier refusal.
BaseException interruptions remain in a propagated BaseExceptionGroup; they
never become success or an accepted sole numeric refusal.

recipes/trace-tracer-refusal-inert.json checks only that consumer contract using
mocks: healthy control, injected stdout-hashEIO after synthetic refusal1, fresh
recovery and retained carrier. SWARM_TRACE_TEST_PARENT is explicitly required.
No child, real strace or storage fault executes there. Real observation needs a
separate allocation meeting caller preconditions.
A second step injects an interruption after the known refusal and requires the
same consumer carrier plus the retained interruption in the exception group.

Results keep complete_build_qualification=false and behavioral_admission=false.
No Cargo, provider, installed Swarm, Rust body, general trace semantics, arbitrary
tree or OS isolation is qualified. The primary's actual exit remains unestablished.
