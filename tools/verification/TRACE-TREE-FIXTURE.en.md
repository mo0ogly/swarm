# Controlled multi-process fixture worker

`trace_tree_fixture_worker.py` produces a controlled parent/child/grandchild
tree, without tracing or compiling. Python ≥3.11, `-I -S -B`; explicit --root,
--python and --role required. Caller leases a fresh private directory with a
nonempty read-only seed, validates paths/mount/identity and holds the volume.
The worker establishes neither that authority nor confinement. Carry the exact
interpreter through every role: child argv[0] deliberately differs from its
executable. Each child gets constructed LC_ALL=C only, no inheritance or shell;
this qualifies no arbitrary descendant.

Child changes cwd, opens seed by dirfd, duplicates an already owned fd,
reads/hashes the declared bytes, exclusively creates derived, renames it to
renamed, has grandchild read it, then unlinks it. Never remove root/seed. A
pre-rename absence check is not an atomic adversarial no-clobber primitive.
Worker fstat observations are not automatically identities of every tracer
event. ctime is not invariant across rename: separate production/rename metadata
from snapshots within a stable reading phase.

Each spawner exclusively retains its child's raw stdout/stderr under root
(caller restrictive umask), caches PID/code before flush/close/hash/decode,
then validates healthy JSON. Raw diagnostics stay in those files, not errors.
Body/cleanup causes and known receipts survive grouping; failures emit safe
role/receipt metadata on stderr. A wait error does not erase an exit already
known by Popen. An incomplete wait triggers attempted kill/wait of that owned
child only, without retry/watchdog; successful cleanup or descendant cancellation
is not inferred. Partial files remain.

Sizes are fixture data; role/argv0 literals are fixture values. EOF probing and
zero-write-progress refusal are technical boundaries, not operator quotas.
Whole-file memory use is explicit. No product policy/resource limit, service or
installation is changed.

`recipes/trace-tree-fixture-inert.json` declares five synthetic guards, requiring
SWARM_TRACE_TEST_PARENT: late stdout, late unlink, added close failure, wait error after known exit and deferred stdout flush. Each uses healthy control and fresh recovery; Popen is
replaced, no real child/strace. Evidence is 2 FAIL→2 PASS, then selected close
1 FAIL→1 PASS, then selected wait 1 FAIL→1 PASS and selected flush 1 FAIL→1 PASS. The complete recipe was not
executed; do not claim five freshly replayed guards on one revision.

External evidence: root00-cd01-tree-worker-c1jqksj3. GLM10 supplied no fragment
before the verified owned client was interrupted; backend execution/usage remain
unknown, no retry. Root supplied the worker within the authorized fallback.
One actual unfiltered-path capture retains three exact exec/argv/env/terminal
matches, clone3/wait4, dup2, chdir, renameat and unlinkat. Selected strace
file/process/desc/memory classes are not all syscalls. String allocation derives
from every argv/env string of these three declared roles only.

The controller failed after capture on its excessive ctime assertion. Preserve
ACTUAL-TREE-FAILURE.json; ACTUAL-TREE-REANALYSIS.json corrects interpretation of
the same evidence without replay. Capture binds worker SHA
`564552fc9efdae57a2f941cfe332eeb90a2c3fa98b94b2ab6fb59a6055a78bf3`;
the later wait correction has only its targeted guard, not reassigned old traces.
Sources/tools, output/PIDs and hashes are retained. Only the retained native
post-admission numeric reserve observation is attributed to execution; later
measurements do not reconstruct its lost resident list.

No generic grammar, compiler closure, C-D01 admission, 12+3 bodies, installation
or global acceptance. Startup accesses remain in the original traces but are
not qualified as a complete input graph.

TW-01 independent review: explicitly flush healthy JSON inside its receipt
boundary. Buffered write may succeed but later flush fail at shutdown; the
new guard reproduces that path without children, FAIL before/PASS after. No
historical capture replay.
