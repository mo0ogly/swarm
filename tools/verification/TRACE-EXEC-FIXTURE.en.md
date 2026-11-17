# Controlled exec fixture worker (`trace_exec_fixture_worker.py`)

## Scope

This module is a fixture-only producer of one controlled same-PID `execve`
or `execveat` attempt against a caller-leased fixed ELF image, so that a
later, separately authorized tracer observation can check argv and
environment byte fidelity. Python >= 3.11 on Linux is required for
`execveat`. It is not a tracer, decoder, generic capture framework, Cargo
collector or compiler admission path. The worker itself never launches
`strace`, chooses a string width, parses traces, collects Cargo output or
authorizes any use, and it does not replay the earlier three-process tree
fixture tests.

## Caller lease and observations

The absolute executable, private protocol document and new receipt leaf are
leased by the caller. The worker observes regularity, euid ownership and
permission modes and refuses unsuitable inputs, but these observations are
not authorization, path or inode authentication, or OS confinement, and no
authority is derived from names or digests. The executable may be a
root-owned system file such as `/usr/bin/true`; euid ownership is
deliberately not required there. ELF magic and the buffer digest are format
observations, not safety proofs; a fixed ELF image is required, which avoids
shebang interpreter special cases. The `execve` path relies on the caller
stable-pathname precondition: the fd-derived digest is not an atomic
pathname binding claim.

## Fictitious data and strict non-inheritance

The caller explicitly authorizes only the public fictitious argv and
environment bytes inside the document. The module does not explicitly read or inspect the parent environment or
merge it into the delegate mapping: that mapping contains exactly the declared
bytes. Python and dynamic-loader startup remain outside this observation;
no global confinement of those phases is established.

## Protocol `trace-exec-fixture/1`

The private JSON document carries exactly `schema`, `argv_hex` and
`environment_hex`. `argv_hex` is a nonempty array — a fixture-protocol
domain rule, not a general Linux claim — of lowercase even-length hex
strings; decoded arguments may be empty or non-UTF-8 but never contain NUL.
`environment_hex` maps lowercase hex names to lowercase hex values; decoded
names are nonempty without NUL or `=`, values may be empty or non-UTF-8
without NUL. Duplicate JSON members are refused at any depth. The raw buffer
is hashed and compared with the caller-declared digest before parsing, with
no size cap. The data descriptor is opened `O_NOFOLLOW | O_CLOEXEC |
O_NONBLOCK` before `fstat`, so a FIFO can never make the worker wait.

## Linux ABI

`execveat` mode calls libc `execveat(fd, empty, argv, envp, AT_EMPTY_PATH)`
through ctypes with explicit `restype` and `argtypes`; every raw bytes object
and both NULL-terminated arrays stay referenced for the whole native call.
`AT_EMPTY_PATH` (4096) is the Linux ABI constant, not operator policy. If
the symbol is unavailable or the call is refused, the original safe errno
surfaces; there is never an `execve` fallback. Both success paths replace
the process at the same PID, so the worker never returns zero, and an
unexpected mock return raises instead of reporting success.

## Publication, partial IO and UNKNOWN intent

Before exec the worker publishes one receipt: a fresh single-attempt 0600
temp leaf in a private euid-owned parent, a full partial-write-safe loop,
`fsync`, a no-clobber hard link onto the absent leaf, and a directory
`fsync`. Partial or failed publications are preserved as raw evidence;
nothing is deleted, retried or overwritten. The receipt carries only the
intent fields and `outcome` stays `UNKNOWN`; it never contains environment
values, and attempt or receipt facts never infer a successful exec. Every
owned descriptor is closed exactly once; primary and cleanup causes are
preserved as original objects in grouped errors with safe typed metadata,
and no `str()` of untrusted data is ever rendered in receipts or errors. A
handled CLI failure is nonzero — 130 for a known primary or cleanup
interruption — and if writing the stderr diagnostic fails, both causes are
raised together.

## Future root-controlled witness

A later formatter witness with installed strace 6.19, allocated and reviewed
separately under root control, may consume a trace of this attempt; this
source is not attestation of any installed binary. Paraphrased from the
primary sources: strace `execve.c` decodes `pathname`, `argv` and `envp`,
printing `envp` as a bare argument count (`printargc`) when abbreviation is
active, while `printargv` walks the pointer vector until NULL and marks
further truncation. strace.1 documents `-s`/`--string-limit` (default 32;
filenames are always full) and `-v`/`--no-abbrev`, which prints the
environment unabbreviated. A finite explicit width — `-s 0` is not
unlimited — plus abbreviation keep environment values out of collected
output. No arbitrary descendant capture is claimed or provided.

## References

- <https://raw.githubusercontent.com/strace/strace/v6.19/src/execve.c>
- <https://raw.githubusercontent.com/strace/strace/v6.19/doc/strace.1.in>

## Failure consumer and inert verification

Diagnostics retain nested group, cause and context types/errnos without exception
messages; aliases/cycles are marked. Runtime non-Exception interruptions, including
SystemExit(0) and cleanup occurrences, return 130. Parser help and invalid arguments
keep their separate 0/2 exits before runtime handling. Failed diagnostic publication
still raises both original objects. Seven named inert guards cover these contracts;
all exec delegates are mocked. Their results establish neither a real native exec
nor tracer fidelity, Cargo admission, installation or main integration. Original
fixture-precondition errors and source-before failures remain in packet26 evidence.

The executable entry_main boundary maps a diagnostic-publication failure to
130/1 without another stderr write or an uncaught original traceback. Library
cli_main still raises the original body/publication objects together; library
callers must protect that boundary rather than rendering their messages. A new
inert guard covers a first-write failure followed by recovered stderr, both an
ordinary error and an interruption. It formats the resulting terminal SystemExit
through Python traceback formatting; no subprocess/native exec is exercised.
