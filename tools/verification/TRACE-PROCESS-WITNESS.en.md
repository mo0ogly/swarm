# Process transport witness — limited scope

`trace_process_witness.run_witness(out, admission, strace, python)` observes three
known inert Python processes: exit0, explicit exit7, and fresh exit0 recovery.
It reuses `native_build_outcome.execute`. Its numeric receipt describes the
spawned tracer; primary PID, hex-encoded executable/argv/environment and terminal
trace are compared with the primary marker. Expected exit7 is valid only as the
sole issue, with no close/hash/publication/post-admission failure. Additional
causes remain blocking and chained to the original outcome.

The caller supplies a private process with umask077, a fresh empty mode0700 leaf
on qualified storage, absolute executable paths and their pins, stable source
and helper bytes, held descriptors, and the unchanged20GiB host/output guards.
Admission is checked before/after children and before publication. This module
neither chooses/mounts storage nor registers a target or configures a service.
It provides no protection against a hostile writer with the same UID.

The inert environment is exactly LC_ALL=C, PYTHONUTF8=1 and PYTHONCOERCECLOCALE=0;
no parent variables are inherited. The strace -s allocation is derived from the
actual byte lengths of this known no-descendant fixture's argv/environment.
It does not qualify unknown build arguments. Hex string comparison refuses
unexpected syntax. Files must be0600. Exclusive publication failures may leave
visible partial/complete files; cleanup causes propagate without overwrite/retry.

Results always retain complete_build_qualification=false and behavioral_admission=false.
No file-read, compiler, signal, fork, tracer-fault, corrupt-capture, publication or
cleanup-fault qualification is claimed. No Cargo, provider, Swarm service or Rust
test body is invoked. Callers retain raw traces, native receipts and source/tool
hashes; numeric exit alone never qualifies the witness.

The first step of recipes/trace-process-transport-inert.json runs the synthetic guard:
healthy0, clean expected7, refused exit7+EIO, fresh recovery0. Mocks prohibit all
child execution. Set SWARM_TRACE_TEST_PARENT explicitly to the admitted temporary
parent beforehand; missing configuration refuses without fallback. This recipe
proves no real strace behavior, mount admission or build result. The real three
process witness requires a separate allocation meeting all caller preconditions.

The second synthetic recipe guard injects ENOSPC after one written byte, followed
by EIO after releasing the descriptor. It requires both errno causes in their
group, retained partial bytes, successful fresh publication and refusal to
overwrite the original. This is no real volume saturation, storage/unmount fault
or child execution.
