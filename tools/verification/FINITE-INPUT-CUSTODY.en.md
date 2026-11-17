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
recipe uses the `{root}` placeholder) runs 20 unittest regressions with
fresh fixtures under the caller-selected TMPDIR. Every injected-fault
test runs a normal control capture before the fault and a fresh
recovery capture after it. Coverage: the control contract, empty and
non-UTF-8 streams, fragmented reads, short writes, zero-progress
writes, read/fsync/close/link/reopen/manifest faults, exclusive
collision with an untouched original, the typed static-message error
state, and a real `os.pipe` fed by a cooperative in-process producer
and read by the test's owned inert receiver double. Producers and
receivers in the tests are in-process test doubles; an actual
production receiver does not exist in this packet, and nothing here
claims receiver consumption. No compiler and no subprocess are used,
and there is no historical replay and no timeout harness.

Executing the recipe runs only these unit regressions. It is not an
acceptance, a PASS verdict, an installation or a merge: no historical
replay, no full build, and no qualification of any wrapper or
collector is performed or implied by this packet.


Root supplement: original_exception and secondary_exceptions preserve original unrendered causes, including interruption; ready_fd_close_attempted means an attempt, never proven closure after failure. Name lookup treats only FileNotFoundError as absence. Fixtures are retained in TMPDIR. The 65536-byte buffer is transport chunking, not a cap.
