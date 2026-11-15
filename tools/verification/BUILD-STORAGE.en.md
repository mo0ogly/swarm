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

## Registered external temporary targets (STOCK-EXT-01)

The external disk holds evidence, fixtures and temporary compiler caches/outputs
only. Checkout, `.git`, configuration, mission data, service and installed image
remain local. `verify_native_profile.py` still freezes/verifies the executable in
the local `.cache/rust-migration-binaries`; the builder/launcher qualify this
local copy. No service is installed from the external cache.

The default target remains inside the checkout. An exact external target requires
a local `.cargo/external-targets.json` registration (unversioned operator file,
no secret). Existing selection remains `deploy/local-web.json` `rust_target_dir`,
the historical `SWARM_RUST_TARGET_DIR` override, or an explicit builder argument.
Selecting a path grants no admission. Register it using:

```sh
python3 tools/verification/build_storage.py --root /CHECKOUT register --target /MOUNT/cache/target --mountpoint /MOUNT --uuid OBSERVED_UUID --fstype ext4
```

Schema: `schema_version:1`, `targets`, each with `path`, `mountpoint`, `uuid`,
`fstype`. Canonical absolute paths, unknown/duplicate keys, file owner/mode, aliases,
actual mount and identity are validated. A nonblocking cooperative writer lock
refuses concurrent registration without deleting the lock. Adjacent candidate
bytes are reread/fsynced then atomically replaced. Pre-replacement failure keeps
the old policy and retains the temporary for diagnosis; post-replacement failure
is no rollback claim. Existing directory modes are preserved; availability
against an actor able to modify the checkout namespace is not guaranteed.

`build_storage.admitted()` is the sole public lifetime entrypoint. Builder and
collector retain descriptors across the subprocess: exact findmnt UUID/fstype,
rw/without noexec, major/minor, anchor/target dev/inode, path and policy/helper
bytes are checked before Cargo and after execution. Directory-FD traversal with
O_NOFOLLOW refuses a different-device component before Cargo. There is no missing
mount fallback. The FD blocks ordinary unmount; this is no same-UID sandbox or
protection against privileged forced/lazy detach. Post-execution refusal retains
unqualified output and cannot establish that nothing was written.

Python 3.11+ is required for grouped causes. ext4 is the technically qualified
Unix-witness scope; another filesystem needs its own qualification, not an
operator policy ceiling. Host/output reserve is unchanged. Registry/helper hashes
are bound to effective inputs/archives; invocation/results retain admission facts.
This mechanism does not archive the complete `deploy/local-web.json`.

Permanent recipe: `recipes/external-build-storage.json`. Its simulations do not
prove a real mount or Cargo build; actual mounted-device witnesses and the
exercised native profile remain separate. Historical local targets without an
external registry remain compatible; an absent registry grants no external target.

### Completed subprocess and postcheck failures

`native_build_outcome.execute` is shared by builder/collector. It closes/reads
channels without hiding exit/signal, attempts immutable `collector-result.json`
or `cargo-result.json` **before** the postcheck, then writes a distinct
`*-admission-after.json` with process outcome, admission and first-publication
status. The final `failure.json` includes structured `native_outcome` and nested
causes even if either receipt publication fails. A failed spawn keeps unknown
returncode as null; zero is never invented. Any failure blocks successful
qualification. Channel/close/hash errors are not empty successes.

The outcome recipe is `recipes/native-build-outcomes.json`. It exercises five
simulated failure/recovery cases and two targeted metadata consumers, with no
Cargo process. Original refusal evidence and earlier successful production
profiles remain distinct; changed collector methods require fresh qualification.
The external-admission unit recipe does not require a live workstation registry:
its mount/registration data is isolated. Actual builds still bind the real
optional registry and control-method hashes.
