# Finite-input custody (CD01-FINITE-STDIN-17)

Standalone Python-stdlib helper for staging raw bytes from a
caller-leased finite input descriptor into an exclusive private output
directory. It is separate from the existing rustc capture wrapper and
from Cargo: nothing wires it into a build, no existing file is edited
by this packet, and the wrapper and collector sources stay unchanged.

- Helper: `tools/verification/finite_input_custody.py`
- Unit regressions: `tools/verification/test_finite_input_custody.py`
- Recipe: `tools/verification/recipes/finite-input-custody-inert.json`
- French contract: `FINITE-INPUT-CUSTODY.md`
- English contract: `FINITE-INPUT-CUSTODY.en.md` (this file)

## Scope boundaries

- No compiler, subprocess or service execution; no Cargo wiring.
- No environment reads, no argv eligibility, no assumption about
  descriptor 0 or `sys.stdin`.
- No timeout, size cap, retry, fallback, deletion or cleanup.
- No historical stdin reconstruction and no build, profile or
  full-graph qualification; nothing here admits, installs, merges or
  accepts anything, and no PASS verdict is issued or implied.

## Caller preconditions (declared, never proven)

`capture(input_fd, output_dir, allocation_id)` expects:

- `input_fd`: a descriptor the caller owns under a finite cooperative
  single-writer/single-reader fixture lease whose producer will close.
  The descriptor is borrowed: it is read, never closed, replaced or
  duplicated for return.
- `output_dir`: an exclusive, fresh, private directory owned by the
  current uid with no group/other permission bits, created by the
  caller; the helper never creates the output parent. The caller also
  holds the mount/reserve descriptors for the backing volume
  externally.
- `allocation_id`: a nonempty provenance string. It is never
  authentication and grants no eligibility.

The helper validates argument types, the directory (real directory, no
symlink, current-uid owned, private mode), the absence of the four
custody names and the statability of the input descriptor before
consuming any input byte.

## What a successful capture does

1. Creates `input.partial` exclusively (`O_CREAT|O_EXCL`, mode 0600).
2. Reads `input_fd` to observed EOF; only a zero-length read counts as
   EOF. Every chunk goes through a full-write loop that rejects
   zero-progress writes and loops over short writes.
3. fsyncs the staging file and closes the writer exactly once.
4. Hardlinks `input.partial` to `input.bin` no-clobber; a collision
   fails and leaves the original untouched.
5. Reopens `input.bin` `O_RDONLY` and checks offset 0.
6. Writes `manifest.partial` exclusively, fsyncs and closes it.
7. Publishes `manifest.json` exclusively by no-clobber hardlink from
   `manifest.partial`. Raw bytes and JSON never share a file.
8. fsyncs the output directory, closes it, and only then returns
   `(read_only_fd, manifest)`: a caller-owned read-only descriptor
   positioned at offset 0 plus the published manifest dictionary.

SHA-256 and the byte count describe only the bytes staged in
`input.bin`. After success the four names `input.partial`,
`input.bin`, `manifest.partial` and `manifest.json` exist; staging
names are retained, never deleted.

## Read-only validation of a staged record (validate_staged)

`validate_staged(output_dir, allocation_id)` is a strict read-only
consumer of an already published custody record, added without any
change to `capture`. It writes nothing, creates no link, renames,
deletes or chmods nothing, executes nothing, reads no environment
variable, returns no descriptor, and never mutates a record, not even
on failure. It re-checks the caller preconditions that are
filesystem-checkable: `allocation_id` a nonempty provenance string
(never authentication or confinement), `output_dir` an existing real
directory (lstat, no symlink), current-uid owned, with no group/other
permission bits. It then requires exactly the four custody names
`input.partial`, `input.bin`, `manifest.partial` and `manifest.json`
and nothing else in the directory; each must be a regular file (lstat,
so symlinks are refused), current-uid owned and private-mode; and
`input.partial`/`input.bin` and `manifest.partial`/`manifest.json`
must have current hardlink-pair identity (same device and inode), without proving historical creation. Only
FileNotFoundError establishes a missing name; any other error,
including a permission error, keeps its original cause and is never
treated as absence.

The validator then reads `manifest.partial` and `manifest.json` and
requires identical bytes, parses the JSON refusing any duplicate
member at any depth, and validates the exact producer schema: the
complete member set with no missing and no foreign member, the fixed
staging and publication names and publication tokens, the expected
`allocation_id` (equality with the caller's expected provenance
string), `eof_observed` and `staged_copy_complete_of_observed_stream`
exactly the boolean true, `byte_count` an exact non-negative int (a
bool is refused), `sha256` exactly 64 lowercase hex digits, and the
complete `not_claimed` block. Finally it recomputes the byte count and
SHA-256 of `input.bin` by reading it exactly once, read-only, and
fails unless both match the manifest: this recomputation, never the
names, labels or JSON alone, is the hash evidence. On success it
returns the validated manifest mapping.

Failures raise `FiniteInputCustodyError` with the same static message
and typed state fields, at stages `validate`, `read-manifest`,
`parse-manifest`, `schema-manifest` or `verify-input`, with original
causes retained unrendered; `ready_fd_close_attempted` always stays
false for validator failures. A first close failure is primary; cleanup close errors are secondary. There is no retry, fallback,
timeout, size cap, environment read or cleanup; and a successful
validation is not evidence that any receiver runs, consumes bytes or
succeeds, nor of any compiler, lineage or full-graph fact, nor of any
historical input; upstream intent stays unknown even at observed EOF.

## What the manifest claims, and refuses to claim

The manifest records `eof_observed`, `byte_count`, `sha256`, the
staging and publication names, and an explicit `not_claimed` block:
upstream intent completeness stays unknown even if EOF was observed;
fd readiness and API return are facts of the call, not of the file;
receiver consumption, delegation intent and consumer success stay
unknown. The manifest states complete staged input only, never
API-return, receiver or delegation success.

## FIFO-to-regular-file changes (declared, no equivalence claim)

When the leased descriptor is a FIFO or pipe, staging changes both
timing and seekability. Timing: the producer must run to EOF before
capture returns (pre-read/spool); afterwards receiver reads no longer
block on or race with producer timing. Seekability: the staged copy is
a seekable, re-readable regular file with a visible size, while a FIFO
is consume-once and non-seekable. The receiver consumes the staged
file, never the original stream; this is a declared transformation,
not an equivalence.

## Errors

Failures of validation, read, write (including zero-progress), fsync,
close, link, reopen or manifest publication raise
`FiniteInputCustodyError`: one static message with typed `stage`,
`primary` and `secondary` state fields holding bounded op/name/errno
tokens (no input bytes, no environment values). Every descriptor is
closed at most once; error-path close failures are recorded as
secondary causes; a ready `input.bin` descriptor may be closed by the
failure path (`ready_fd_close_attempted`). Partial files are retained; nothing
is deleted; rollback is never inferred; input is never re-read after a
failure; and no failure report file is guaranteed to be written. There
is no successful return after a failure.

## Tests and recipe

`python3 -B tools/verification/test_finite_input_custody.py -v` (the
recipe uses the `{root}` placeholder) runs 29 unittest regressions with
fresh fixtures under the caller-selected TMPDIR. Every injected-fault
test runs a normal control capture before the fault and a fresh
recovery capture after it. Coverage: the control contract, empty and
non-UTF-8 streams, fragmented reads, short writes, zero-progress
writes, read/fsync/close/link/reopen/manifest faults, exclusive
collision with an untouched original, the typed static-message error
state, and a real `os.pipe` fed by a cooperative in-process producer
and read by the test's owned inert receiver double, plus the read-only
`validate_staged` consumer (healthy control, tampered raw content,
wrong provenance, missing or foreign manifest member, wrong hardlink
pair or extra entry, malformed or duplicate JSON, type-invalid EOF and
count, preserved IO-error cause). The `ValidateStagedTests` class can
also be run alone (`python3 -B
tools/verification/test_finite_input_custody.py ValidateStagedTests
-v`) without any historical replay. Producers and receivers in the
tests are in-process test doubles; an actual production receiver does
not exist in this packet, and nothing here claims receiver
consumption. No compiler and no subprocess are used, and there is no
historical replay and no timeout harness.

Executing the recipe runs only these unit regressions. It is not an
acceptance, a PASS verdict, an installation or a merge: no historical
replay, no full build, and no qualification of any wrapper or
collector is performed or implied by this packet.


Root supplement: original_exception and secondary_exceptions preserve original unrendered causes, including interruption; ready_fd_close_attempted means an attempt, never proven closure after failure. Name lookup treats only FileNotFoundError as absence. Fixtures are retained in TMPDIR. The 65536-byte buffer is transport chunking, not a cap.


Consumer scope: hardlink identity and current byte equality are observations under a stable caller lease, not proof of historical no-clobber creation or absence of a concurrent same-uid writer. Read-only means no content or namespace writes; filesystem read metadata such as atime may change. A close error can be primary when it is the first failure, secondary only during cleanup. Missing-name errors retain their original exception and errno. Nine new validator methods are run separately; the20unchanged capture PASS are reused.
