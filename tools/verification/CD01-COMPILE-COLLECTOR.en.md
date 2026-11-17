# Instrumented C-D01 collector — unqualified collection

`cd01_compile_collector.py` is the single maintained copy of historical collector65a6f5806d7e5aca02950e184a4aac2d24d78af55116652c46795b78019c1030, extended with rustc intent observation and input prebinding. The original remains immutable evidence. This tool qualifies no binary/profile/fullgraph, runs no test body and creates no C-D01 admission record.

## Inputs and authority

Eight explicit arguments: `--root`, `--output`, `--target`, `--inventory`, `--rustc-intent-producer`, `--rustc-intent-producer-sha256`, `--root-guard`, `--root-guard-sha256`. Root is the owned export without .git/.swarm, checked against its original inventory, not the shared checkout. Output is a new evidence leaf. Target is an empty temporary target already admitted by storage configuration; the collector never registers or cleans it.

Mount path comes from `.cargo/external-targets.json` through the pinned strict helper: public registration → ownership/schema validation → target selection → actual mount observation → held FD/admission. Authorized SWARM UUID/ext4 and the21,474,836,480-byte host/output reserve remain required; no absent-mount fallback. Repository, live data, runtime and installed executable stay local.

Guard must be exactly `ROOT/tools/verification/cargo_space_guard.py` because its policy is location-relative. Producer/guard have explicit byte pins; the producer copy is executable, reread and archived. Shebang/PATH observations do not qualify interpreters, proxies or libraries. Arguments may contain private data and require caller authorization/review. There is no descendant environment dump or generic secrecy guarantee.

## Flow

Cargo argv remains `cargo test --manifest-path ROOT/rust/Cargo.toml -p swarm-engine --lib --no-run --locked --offline --message-format=json`, without fixture features. Ambient controlled overrides refuse before constructing declared `RUSTC_WRAPPER` and `CD01_CAPTURE_DIR`/`CD01_SPACEGUARD_PATH`. This explicitly overrides the configured wrapper; unchanged Cargo argv proves neither identical rustc argv nor codegen. No PATH shadow, CC or CFLAGS instrumentation.

Workspace candidates are archived before Cargo. Cargo.lock is hashed/parsed from one buffer. Locked packages map to exactly one complete src/cache namespace under explicit HOME; missing/ambiguous/unsupported/symlinked/checksum-invalid data refuses. Unpacked source and archive bytes are pinned separately, never claimed equivalent. Both Cargo config filenames are observed in cwd/ancestors/home without inventing precedence. Only FileNotFoundError means absent; other errors refuse. The config observer opens no credentials files; descendant reads remain unknown.

After Cargo, source checks, event/features and frozen image remain separate from intents. Actual producer-v1 receipts validate empty/nonUTF8 arguments, partial/final hardlinks and coherent delegation; version/print probes are not compiles. Crate-name/test correspondence is only an intent candidate for the selected artifact name: source/output and per-compiler outcome binding remain UNKNOWN. Final status is always `CD01_COMPILE_ONLY_UNQUALIFIED`, full_effective_inputs_unknown=true, tests0. Exit0 permits neither fifteen C-D01 bodies nor installation.

## Errors and evidence

The retained native helper caches primary outcomes before publication; Cargo0 is not test PASS. Postcheck failure may follow a completed compile. Cached values, typed causes, partial state and observation errors remain visible. If final diagnostics fail, an error group carries `collector_failure`; terminal `_error_tree` exposes known facts without fabricating native_outcome frames. This is in-memory custody, not durable delivery when all channels fail. Instrument archive candidates remain on success/failure. Failed publication never proves rollback.

Reserve, FDs and executed inputs must remain stable. Lease pre/post hashes are neither atomic snapshots nor proven read windows. Descendant env, native/header/linker/procmacro graphs, generated/deleted inputs and full toolchains remain unknown. PATH tool/final-name symlink observations do not prove every parent component or effective toolchain.

## Current verification

Fifteen distinct maintained guards use no compiler/provider/service: caller binding, actual receipts, configs/refusal, fresh target, archives, registry namespace, relative PATH, lock buffer, causes/cached values and intent scope. They ran separately for changed contracts; unchanged PASS evidence is reused. `recipes/cd01-collector-integration-guards.json` received schema/input validation only, no combined replay. Original counterexamples/corrections and exact hashes are retained under `root00-cd01-collector14-waja855_` on SWARM.

No Cargo or C-D01 body ran in this increment. Independent review, exact allocation and fresh prerequisites remain required before actual collection. Fixture guards do not accept the global plan. Verification-only tools change no product runtime, so no application installation/version_history update is warranted here.


## Library v2 consumer — packet21

`_parse_intent_records(capture, custody_validator=...)` accepts v2 only with the caller's explicitly supplied reviewed custody validator. Its identity/source and read-only behavior are caller lease preconditions; a callable is neither authentication nor confinement. Existing `run`/CLI passes none and still refuses v2. No CLI selector/manifest or compilation added.

V1 retains its two files and existing fields/results. V2 requires exactly the hardlinked intent, `stdin-custody` and `stdin-transfer.json`, never a failure.json or foreign member. Reader verifies exact schemas/fields/types, PENDING, PID/allocation/relative directory, replacement/close booleans, then compares digest/count/EOF/names to the current staged bytes checked by the callback. Bool and int are never interchangeable. Result stays UNKNOWN, never a test or compiler-consumption proof. Current links/bytes are observations under stable lease, not history or immutability proof; atime may change.

`CustodyValidationFailure` retains original error objects in memory and supplies `_error_tree` bounded categories/errno/known-vocabulary metadata and primary/cleanup interruption facts. Private text is not rendered; unsupported metadata is explicitly marked, never successful emptiness. Existing CLI failure status1 is unchanged; no native outcome fabricated.

Six new methods use synthetic structural fixtures built by actual renderers and the real helper, never dup2/guard/compiler/Cargo/provider/service. Each mutation starts from its own valid record, with fresh recovery and retained content/name snapshots. Initial5PASS/1ERROR (secondary metadata omitted by principal), then only failed guard1PASS after correction. Not six final reruns or replay of old15guards.21method recipe maintained, not globally executed.
