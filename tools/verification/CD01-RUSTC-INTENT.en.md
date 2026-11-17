# Rustc intent producer — experimental scope

`cd01_capture_rustc_wrapper.py` produces **pre-exec** receipts for a proposed future C-D01 collection. It qualifies no binary, compiler, profile or graph and runs no tests itself. Cargo integration/config precedence, actual selected paths/proxies/interpreters and the main collection still need separate review/allocation.

The protocol receives compiler at argv[1], followed by arguments. `CD01_CAPTURE_DIR` must be an existing absolute private euid-owned directory; `CD01_SPACEGUARD_PATH` an absolute trusted executable guard selected by the coordinator. No configuration default, fallback, retry or timeout is introduced. The future collector owns volume identity, target admission, disk reserve, source stability and tool/interpreter hashes; this wrapper establishes none of them.

Each invocation exclusively creates a private directory. It fully writes ASCII JSON into `intent.partial`, fsyncs/closes it, then publishes a no-clobber hardlink `intent.json` and syncs the directory. The partial remains. Compiler/guard paths, arguments and cwd use reversible byte hex; the configured guard is explicitly retained. No arbitrary environment dump is recorded. Arguments may contain secrets: the caller must authorize/review the data; avoiding environment dumps does not guarantee argument secrecy.

After publication, `os.execv` attempts same-process replacement by `[guard, compiler, arguments…]`, leaving the environment unchanged. The receipt describes intent only, outcome UNKNOWN, never an observed execution chain. Held descriptors are CLOEXEC. Collisions refuse without overwrite/retry. Requested 0700/0600 modes remain subject to umask; access failures are refusals, never successful empty results.

On failure, partials remain and an exclusive `failure.json` is attempted. It describes the primary cause and known publication facts; cleanup causes reach the safe diagnostic. A visible link after fsync failure is neither declared durable nor rolled back. If stderr fails, `WrapperReportingError` retains primary and secondary causes, including reporting failure, in memory with a static message and suppressed displayed context. This does not guarantee durable receipts when all channels fail. Exit codes78/1/130 and schemas are technical conventions, separate from guard operator policy. Invalid configuration can also fail a system call and be classified operational failure.

Final-component no-follow/private permissions assume a stable caller lease, not same-UID adversarial confinement. There is no cleanup, configuration mutation or product installation.

## Verification and limits

Seven named guards in `test_cd01_capture_rustc_wrapper.py` cover diagnostic write/flush, empty/spaced/nonUTF8 arguments, short/zero writes, sync after visible link, primary+close errors, and exclusive collision. All mock exec: no real guard/compiler execution follows from them. Valid controls precede injection; recoveries use fresh destinations.

The diagnostic guard reproduced RP13-ERR01: 1 failure before, 1 pass after the principal's consumer correction. Four new guards and two boundary guards then passed separately, each bound to exact source bytes. No combined seven-guard replay was performed. `recipes/cd01-rustc-intent-guards.json` is retained for a future justified need; schema/input validation is not execution. Evidence: `root00-cd01-rustc-producer13-hvkrrd47` on SWARM, indexed with hashes in the C-D01 registry.


## Owned inert witness observed after source review

On17November, a fresh isolated allocation executed the actual producer, existing guard in its no-`--out-dir` query path (not compiler admission), then a coordinator-owned inert receiver. Three invocations produced expected exits0/7/0. Final receiver PID matches the intent, and empty/spaced/nonUTF8 argument bytes match. Intent outcomes remain UNKNOWN: numeric outcomes come from the outer native controller. Seven retained checks preserve mount/UUID/FD/reserve and selected source hashes. No compiler, Cargo or C-D01 body execution. Separate evidence: `inert-chain/RESULT.json`, channels and native receipts under producer13 evidence. This does not qualify Cargo precedence, profile or fullgraph. New observation review remains distinct from earlier source/guard approval.
