# Primary-process SIGTERM witness

`trace_signal_witness.run_signal_witness(out, admission, strace, python)` observes
a control worker, a worker sending SIGTERM to itself, and fresh recovery in a
different directory. It reuses the transport nucleus publisher, hex decoder and
native executor. The earlier0/7/0 witness remains distinct and unchanged.

The caller retains qualified storage/FDs, source/helper/tool pins, umask077,
a fresh empty0700leaf and20GiB host/output reserve. Files must be0600. The inert
environment is exact without inheritance; string allocation derives from known
argv/env, never arbitrary build policy.

Primary PID, executable/argv/env, signal terminal and numeric tracer result are
checked separately. The installed strace manual describes tracer termination
with the same signal; the witness must observe the actual SIGTERM/negative-return
mapping. Only the sole expected numeric ValueError is accepted; other causes
remain blocking and linked to the native receipt. Signal evidence is compared
before/after fresh recovery. Refusal retains partial files and original causes.

Scope is Linux and these three declared no-fork workers only. No other process
is signalled. No Cargo, provider, installed Swarm or Rust test body is invoked.
Results keep complete_build_qualification=false and behavioral_admission=false.
Other signals, tracer faults, corrupt capture, consumed files and arbitrary trees
remain unqualified.

Known-result custody during later failures/interruptions now follows the
[shared contract](TRACE-OUTCOME-CUSTODY.en.md). Native result is cached before
fallible diagnostics; primary facts are observed only after trace comparison.
The new guards are synthetic and do not replay historical SIGTERM observations.
