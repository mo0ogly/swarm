# Local delivery qualification evidence

The launcher validates retained proof before reusing an unchanged image. `_read_prepared` shares explicit `--prepared-build` checks: schema, digest, normal profile without fixtures, metadata and exact input archive. Missing files require a fresh build; incoherent evidence refuses before stopping the server. Compiler success or a surviving binary alone does not establish qualification.

For managed `start` or `restart` with an active server, the version check and identity revalidation apply to freshly built as well as preselected candidates. An unreadable version, changed identity or older/unorderable version refuses before the pair check and stop. Identical image bytes remain allowed by the existing comparator; version labels do not authenticate sources. Build-only preparation does not replace an active server. Refusal cleanup does not remove the installed binary path; if candidate cleanup also fails, both causes are retained. The comparator keeps its historical `prepared-build-*` error identifiers for both producers. Recipe `local-managed-version-gate.json` checks this ordering with eight inert methods, controls and recovery; it qualifies neither an actual installation, selection of the current engine for the Authority guard, nor numbering across checkouts.

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

## Pure numbering decision — caller not wired

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

The calculation performs no IO, process launch or input mutation. Python's native integer-conversion limits may still raise; no cap or interpreter setting is added. A number authenticates neither source nor actor. **Status: not wired** to `prepare`, `main`, cache admission, `SWARM_VERSION` or actual image capture. Those contracts remain open; recipe `local-managed-numbering-pure.json` qualifies only the function and its historical compatibility. This establishes no build, delivery flow or newly installed binary.
