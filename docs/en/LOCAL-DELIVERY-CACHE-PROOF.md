# Local delivery qualification evidence

The launcher validates retained proof before reusing an unchanged image. `_read_prepared` shares explicit `--prepared-build` checks: schema, digest, normal profile without fixtures, metadata and exact input archive. Without a server observation, missing cache proof can require a fresh build. With an observation and present history, missing or incoherent proof refuses before stopping the server; an absent history file remains a legitimate fresh source. Compiler success or a surviving binary alone does not establish qualification.

For a fresh managed preparation with an active server, `main` captures the owned image version/hash before the producer and passes that exact `installed` DTO. After preparation, `build`, `start` and `restart` capture those facts again; version or hash drift refuses before version ordering, the pair guard and any stop. Preselection keeps one post-preparation capture. Without an owned server, the producer keeps its legacy call without an observation. An unreadable, older or unorderable version refuses; identical bytes remain accepted by the comparator. Labels do not authenticate sources. `build` can observe an active server but never stops it. Refusal cleanup preserves the installed path and both causes if cleanup fails. The comparator retains shared `prepared-build-*` identifiers. Recipe `local-managed-version-gate.json` covers thirteen inert caller methods, including the old missing-DTO counterexample; it establishes neither native compilation nor actual installation.

Explicit selection does not rebuild newer sources and refuses version downgrades. Primary and cleanup causes remain distinct. This grants no publication or data migration authority. Retain evidence outside the cache before cleanup.

Recipes: `local-build-cache-proof.json` and `tests/test_swarm_prepared_build.py`. The latter requires `SWARM_TEST_BUILD_STATE` for its isolated real lifecycle; a missing prerequisite is a skip, never PASS. The permanent recipe covers three cache cases, seven stability guards and five selection/lifecycle checks. No independent or global acceptance follows.

The qualification area requests Unix mode 0700. A mkdir or chmod error propagates
with its original errno and object before any native child; no cleanup, retry or
alternative location follows refusal. The area may already exist or have been
created before failure: no rollback is inferred. Two inert guards in
`tests/test_swarm_version_fixture_area.py` cover simulated checkout and installed
launcher layouts, with a valid control, refusal reaching `verify_candidate` and
healthy recovery. Maintained recipe:
`tools/verification/recipes/local-delivery-fixture-area.json`. Existing source/home
selection stays unchanged; this check does not qualify use of the persisted
`runtime_temporary_directory` setting, a build or an installation.

Before `start`/`restart`, the Authority pair guard retains the record of a server previously identified as owned: lost identity refuses instead of falling back to an installation pathname. After a first ownership check, the reader opens its image through `/proc/<pid>/exe`, holds that descriptor across the public `engine-settings show --json` command, then rechecks identity and device/inode. The child inherits the descriptor through `pass_fds` and uses `/proc/self/fd/<fd>`; only the observed mode leaves after close. Acquisition, read and close failures refuse before stop; a secondary close failure retains the primary cause too. Without an owned server, direct callers keep using the current binary, but its absence on an initialized Root now refuses; the never-initialized case retains prior behavior. This Linux/procfs observation is neither confinement, a continuous lease, nor a guarantee against every swap between observations. Existing mode, daemon and protocol policies remain unchanged. Recipe `local-live-engine-reader.json` covers synthetic reader fixtures, the corrected historical refusal and caller version guards; it executes neither a real engine nor a service and does not qualify delivery across checkouts or an installed binary.

## Pure numbering decision

`select_version(published, digest, previous, *, installed=None)` preserves the historical three-argument calculation. The optional `installed` observation is a dedicated dict with exactly `version` (canonical `vM.m.p-dev.N`, no leading zeros) and `binary_sha256` (64 lowercase hex characters). It is calculation input, not evidence: the caller must establish image ownership/freshness, qualify its own history under locks and recheck before replacement. The caller must supply that source's own prior record; the calculation neither constructs nor copies one.

| Observed calculation inputs | Decision |
| --- | --- |
| No prior record, same family and observed counter 381 | Allocate 382 |
| Same-family prior below the observed counter | Allocate above the observed counter |
| Prior above the observation, unchanged source digest | Retain the prior number |
| Same observed number and same source | Retain only if the recorded image matches; malformed consumed hash refuses |
| Changed source | Allocate above the target-family prior while respecting the floor |
| Lower observed family | No counter floor across families; an unchanged target-family allocation may remain, even at counter zero |
| Observed or prior family above the target | Refuse without inventing a release lineage |
| Present history without a version, malformed observation or noncanonical version | Explicit refusal |

The calculation performs no IO, process launch or input mutation. Python's native integer-conversion limits may still raise; no cap or interpreter setting is added. A number authenticates neither source nor actor. `prepare` consumes the observation for numbering, cache admission and `SWARM_VERSION`; `main` supplies the capture for fresh preparation with an owned server. Recipe `local-managed-numbering-pure.json` still qualifies only the function and its historical compatibility, separately from producer and caller recipes. This establishes no build, delivery flow or newly installed binary.

## Build refusal for an external selection

External build refusal: `swarm_local.py build` refuses any resolved binary that is not exactly the project's `bin/swarm` or `bin/swarm-local`, whether selected via --binary, SWARM_BINARY or persisted settings. The refusal happens before any Root/runtime creation, locking, owned observation or build write, keeps the exit-2 contract and prints a clear French message asking to select a managed source path explicitly, with no configuration values or secrets. External start, restart and configure are unchanged: the binary stays external, with no rebuild, implicit fallback, allowlist extension or copied source history.

Settings and path metadata may already have been read; this selection check does not confine the process. Evidence uses inert calls to main with doubled process/network/producer boundaries: it establishes no native compilation, real replacement or installation. The observed wiring below does not change this external-selection refusal.

## Producer with optional observation

The internal `prepare(source, binary, installed=observation)` API passes the DTO to predicted and actual selectors, then admits the cache and `SWARM_VERSION`. For an active owned server, the launcher supplies this DTO under lifecycle/build locks and retains its post-build capture. This source wiring and its fixtures do not establish a newly installed running version.

With an observation, direct reading distinguishes an absent file from invalid present history (including `null`); other IO/JSON errors propagate. Present history needs schema 2/rust, a canonical image hash and a nonempty text source_digest. The existing source-contained `_read_prepared` checks must return the same record before numbering/cache use and again for cache reuse. Late missing proofs or changed history refuse; the historical missing-cache-proof rebuild fallback is reserved for calls without observation. These checks validate existing local evidence, not signed actor identity, an authenticated complete source graph or a newly qualified native profile.

Cached numbering survives only if the observed selector retains that exact label. A canonical dev override must be above the observation; equality is reserved for reusing a qualified identical image, never rebuilding an unknown image under that label. Without a DTO, historical numbering, caching, loading and error precedence remain. A separate general correction in this producer groups both original causes when primary failure is followed by cleanup failure; successful cleanup rethrows the primary. This uncaught group remains nonzero, with no exit-2 promise for a dual failure. Qualification commands have no sign-in link or token in their arguments; no new private-identifier output is added.

Manifest, source settings and toolchain may already have been read; there is no zero-IO claim. `prepare` does not replace the installed image. Staging/evidence directories may exist after refusal; a state publication/durability failure may have changed the record and proves no rollback. Tests are inert: builder, source identity, retained-proof reader and qualification are explicit doubles; they establish producer decisions/dispatch, not native compilation, a real archive validator or installation.


## Version capture on the owned image

Source behavior notes (not installed binaries): tools/swarm_local.py gains owned_running_version(state, address, root). After the pre-open ownership check it opens /proc/<recorded pid>/exe (O_RDONLY|O_CLOEXEC), revalidates full identity and the held descriptor's inode, queries '/api/v1/native-version' via session_request and urlopen(timeout=1) (the existing individual-probe deadline, no new global watchdog), validates schema_version 1 / binary dict / nonempty string binary.version (canonical form and version ordering stay with the producer), hashes the held descriptor to EOF via os.read with the stdlib buffer (no cap, deadline, retry or child process), revalidates identity/inode, closes the descriptor, then returns (DTO, sha256). running_version stays untouched (status and the initial start path). In main's managed block: a fresh build with an active managed server takes a pre snapshot before the producer and passes it the exact {'version', 'binary_sha256'} DTO via installed=; after the candidate, every active managed preparation in the build/start/restart block takes a new post snapshot and keeps the owned() recheck and require_prepared_not_older before any stop or successful build return; for a fresh pre snapshot the post (version, hash) must equal pre or the candidate is refused even when newer. --prepared-build keeps prepare_recorded with no pre snapshot, one post capture and the same-hash idempotent start. With no running server nothing is observed and no installed floor is claimed (legacy two-argument producer call). An active build therefore observes before and after but never stops the server and on refusal cleans only an unused candidate; the installed/cached image is never deleted; a pre-capture failure calls neither producer nor cleanup. _require_owned_image now carries generic image-observation wording (identité changée / image remplacée) with unchanged error, ownership and inode semantics. Version labels order local deliveries and prove no source authenticity.

Maintained recipe `local-owned-version-capture.json`: seven reader methods, thirteen caller methods, two existing identity/inode guards and four pair-guard main callers. The latter double both version captures and assert the exact producer DTO; the pair guard retains its synthetic routes, healthy control and pre-stop refusals. Prior results remain bound to historical inputs; shared diagnostic changes justify these two targeted checks, not campaign replays.
