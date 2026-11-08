# Local build storage

[Français](BUILD-STORAGE.md)

Future Cargo calls from this checkout or its descendants load
`.cargo/config.toml`. The `dev` and `test` profiles set `debug = 0` and
`incremental = false`, reducing debugging information and disabling rustc's
incremental cache. Optimization, debug assertions, overflow checks, panic
handling and features are unchanged. The `release` profile is unchanged.
Existing outputs are neither removed, moved nor rewritten by this configuration.
New settings can require recompilation; old artifacts are not automatically removed.

## Compiler admission

Cargo runs `tools/verification/cargo_space_guard.py` as `rustc-wrapper`.
Before each command containing `--out-dir`, the guard reads
`.cargo/storage-policy.json` and measures user-available bytes on both the
checkout and output volumes. The user's November 7, 2026 decision sets
`minimum_free_bytes = 21474836480`: **20 GiB**, approximately 21.5 decimal GB.
Below this threshold, the compiler is not started and Cargo fails with an
actionable measurement/threshold diagnostic. Compiler discovery queries remain
available. Missing/invalid policy or failed measurement refuses compilation,
without a fallback threshold.

```sh
python3 tools/verification/cargo_space_guard.py --status
python3 tools/verification/cargo_space_guard.py --set-minimum-free-bytes 21474836480
```

The JSON file is the sole threshold source. CLI writes are atomic and synced:
errors before replacement preserve the original; a directory-sync error after
replacement does not imply rollback. This workstation build-tool policy is
separate from Swarm engine runtime budgets, timeouts and Admin/API/CLI policies.

The guard deletes nothing, runs no background monitor and stops no running
process. Explicit recipe target paths and production/fixture separation remain
intact. Calls outside this tree using only `--manifest-path` must load the config
explicitly with `--config` or run from the checkout. Cargo environment/CLI profile
and wrapper overrides remain possible and must be retained in build evidence.

## Limits and recovery

**This admission check is neither a quota nor an atomic disk reservation.**
An admitted compiler, concurrent builds, build scripts, rustdoc, another program
or direct commands bypassing Cargo may consume space after measurement. Already
running processes are unchanged. The guard cannot guarantee that the filesystem
will never fill.

After refusal, remove only unused caches under the actual producer lock, or
select a suitable volume. Retry with the same 20 GiB threshold; do not lower it
to manufacture passing evidence. No automatic purge, proof deletion or active
service modification is performed. Explicit source debugging may use
`CARGO_PROFILE_DEV_DEBUG=2` / `CARGO_PROFILE_TEST_DEBUG=2`, retaining the guard
and accounting for larger output. Do not change a profile during a test run.

A strict limit needs a dedicated bounded Unix filesystem/quota: qualify Cargo
locks and affected recipes, then route new executions only. Do not build on
FAT32 or use SSHFS without lock qualification; preserve active outputs. On this
host, mounting a dedicated filesystem requires administrator authentication
unavailable to the agent; this strict ceiling is not installed. Smaller symbols
or repeated cleanup do not establish such a ceiling. Several consumers rely on
`debug/deps`, so globally changing `build.build-dir` also requires adapting and
verifying those flows.

## Verification and provenance

The permanent `recipes/build-storage.json` recipe covers refusal before compiler
startup, recovery, boundaries, measurement/publication errors, persisted policy,
settings consumed by actual Cargo and qualification input binding. Fixture
thresholds are isolated test data, never production-policy overrides.

`verify_native_profile.py` prebinds and archives the guard/policy bytes as well:
changes/removal invalidate reuse. Historical proofs remain unchanged; previous
qualification does not qualify the new settings. The guard is not a security
boundary, native delivery or acceptance proof.

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/build-storage.json --output .cache/verification/build-storage-NEW
python3 tools/verification/recipe.py verify --evidence .cache/verification/build-storage-NEW
```

Use a fresh evidence directory on a Unix filesystem, then archive verified proof
on USB with a local index. Recipe publication uses Unix hard links, so direct
FAT32 evidence directories are unsupported. See the [verification guide](README.md)
and [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html).
