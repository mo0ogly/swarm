# Cargo depfile inventory

Current scope: post-build observation only. No compiler/test execution or admission authority.


### Scope
`cargo_depfile_inventory.py` is a post-build, read-only inventory tool with
scope `postbuild-depfile-inventory-only`. It reads one captured Cargo JSON
stdout file, requires `build-finished` success and a nonempty artifact set,
resolves each artifact's dep-info file (rlib, proc-macro `.so`, test
executable, custom-build inode sibling), parses coherent repeated Rust artifact rules and their source phony entries
with the established shlex convention; comments remain uninterpreted raw metadata, hashes and classifies the named
sources against caller-declared roots, and publishes one fresh JSON report.
Stdlib only; no command execution, no compilation, no ELF parsing, no
network, no test execution.

### Command (placeholders)
```
python3 tools/verification/cargo_depfile_inventory.py \
  --cargo-stdout <path-to-captured-cargo-json-stdout> \
  --workspace <workspace-root> \
  --target <cargo-target-directory> \
  --registry-root <registry-source-root> \
  --output <fresh-report-path.json>
```
Exit codes: `0` every artifact mapped and every source observed cleanly;
`1` report published with unresolved artifacts and/or source issues;
`2` fatal refusal/IO error; a new partial or complete output may remain visible. No rollback inferred.

### Caller responsibilities
The caller owns mount, reservation and file-descriptor admission, selects
every path (the tool has no defaults and reads no environment), freezes the
source and target trees for the duration of the run, and supplies the
trusted Cargo stdout it captured. The tool does not reimplement storage or
context orchestration and assumes no concurrent writer; it is not
protection against hostile same-UID replacement.

### Report semantics
`status` is always `OBSERVED_POSTBUILD_ONLY`, `scope` always
`postbuild-depfile-inventory-only`; `complete_build_qualification` and
`behavioral_admission` are always `false`, even when every artifact maps.
Explicit still-unproved flags: input bytes at compiler read time; exact
compiler/linker/C/ancestor configuration environment; generated/proc-macro
side inputs omitted from depfiles; full linkage/runtime closure. The tool
never emits any admission record and never asserts that all dependencies
are proven. `--registry-root` is a caller declaration used for
classification only, never evidence of authentic provenance.

### Refusals
Fatal (exit 2, no complete report guaranteed after IO): malformed or non-UTF-8 stdout, missing or failed
`build-finished` success, empty artifact set, missing directories, existing
output path, IO/fsync/cleanup failure (typed causes grouped). Per-artifact refusals are recorded with a
safe cause (missing/ambiguous/wrong-kind/wrong-inode/symlink/outside-target
artifacts or depfiles, unsupported depfile syntax, missing sources, drift or
conflicting observations) and make the run exit nonzero while preserving
partial observations. Read depfile bytes/hash survive syntax refusal. Output is private (0600), exclusive, file and directory synced. No content-only or mtime fallback; no replay, retry,
timeout or alternate paths.
