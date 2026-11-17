# Known facts after a later failure

`trace_outcome_custody.raise_with_custody` propagates a failure/interruption while
retaining native facts already observed by its caller. It performs no IO,
execution, trace parsing, retry or business decision. `native_build_outcome.py`
is unchanged.

`trace_process_witness.observe_case` and `trace_signal_witness.observe` cache the
tracer result immediately after executor return or its error carrier, before
fallible publication/validation. Tracer exit never establishes primary exit.
Primary stays UNKNOWN until exact marker/PID/argv/env/terminal comparison, then
the read facts become OBSERVED, without global admission. Output hashing stays
inside the error-custody boundary.

The native_outcome carrier contains supplied subprocess facts, original
native_executor_receipt when available, primary_observation, following_failure
snapshotted before mutation, and earlier_failure when supplied. Caller-owned
internal dictionaries must be JSON-safe and are shallow copied. No timestamp,
PID, publication status or zero code is invented. Missing result means unknown,
never proof of no execution. Callers explicitly supply UNKNOWN/OBSERVED status;
otherwise the helper uses a known:false marker without inventing primary facts.

Ordinary errors retain type/cause. An interruption with a distinct earlier
failure, or chaining that would erase an existing cause, propagates a
BaseExceptionGroup containing the original objects. No-earlier/same-object paths
create no self-cause. Existing diagnostic metadata stays in its pre-mutation
snapshot, not a link to the new carrier. The helper always raises and grants no
acceptance.

recipes/trace-caller-custody-inert.json requires explicit SWARM_TRACE_TEST_PARENT.
Eight caller guards cover known0/7/-15 followed by EIO/KeyboardInterrupt, with
healthy controls and fresh recovery. Two pure-helper guards cover unknown/same
identity and a distinct existing cause. Mocks replace executors and forbid all
children. These are injected exceptions, not system signals or real storage
faults. Earlier real observations remain bound to historical source bytes;
no trace or compiler replay occurs here. No build, provider, service or Rust
test body. Complete compiler input closure and behavioral admission remain open.

When adding a project helper, update input inventories of recipes importing its
callers too: declaring only the caller does not cover its new dependency. CC-01
was corrected in both existing recipes. A local-import check supports review;
it does not establish complete dynamic-import, Python or system closure.
