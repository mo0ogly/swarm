# tools/verification/file_access_fixture_worker.py
"""Read-only fixture-access worker (stdlib only; runs under `python -I -S -B`).

Full contract: see main() docstring. Requires Python >= 3.11 (BaseExceptionGroup).
"""

import argparse
import hashlib
import json
import mmap
import os
import stat
import sys

_MODES = ("open", "read", "map")


def _snapshot(fd):
    """Observe an opened fd and return its stat plus selected metadata.

    Propagates fstat errors. This does not read content or establish origin,
    immutable lifetime, ownership authority or complete filesystem metadata.
    """
    st = os.fstat(fd)
    return st, {
        "dev": st.st_dev,
        "ino": st.st_ino,
        "size": st.st_size,
        "mtime_ns": st.st_mtime_ns,
        "ctime_ns": st.st_ctime_ns,
    }


def _read_declared(fd, declared_size):
    """Read exactly declared_size bytes, always requesting the remaining
    length (no fixed chunks, no caps); refuse unexpected EOF and drift.

    Caller supplies a leased nonempty regular fd and observed positive size.
    Returns actual bytes; does not close the fd. The one-byte EOF probe is a
    boundary check, not an operator quota. Read errors propagate to cleanup.
    """
    chunks = []
    received = 0
    while received < declared_size:
        chunk = os.read(fd, declared_size - received)
        if not chunk:
            raise ValueError(
                f"unexpected EOF: got {received} of {declared_size} declared bytes"
            )
        chunks.append(chunk)
        received += len(chunk)
    if os.read(fd, 1):  # 1-byte EOF probe: any byte here means trailing drift
        raise ValueError(
            f"size drift: more than {declared_size} declared bytes present"
        )
    return b"".join(chunks)


def _run(path, mode):
    """Read one leased fixture using a CLI-validated mode and return metadata.

    Path/mode validation and ownership/lifetime lease belong to main/caller.
    Open read-only, optionally read/hash or map/hash, then require unchanged
    selected fd metadata. Healthy return contains a historical descriptor
    number after cleanup, not a currently usable descriptor. All body/cleanup
    causes propagate together; cleanup is attempted, not guaranteed on error.
    No printing, writes, confinement or compiler-input proof is provided.
    """
    fd = None
    mapping = None
    body_error = None
    result = None
    try:
        fd = os.open(path, os.O_RDONLY)  # read-only, no O_CREAT, only this path
        st, before = _snapshot(fd)
        if not stat.S_ISREG(st.st_mode):
            raise ValueError(f"not a regular file: {path}")
        if st.st_size <= 0:
            raise ValueError(f"empty fixture: {path}")
        declared_size = st.st_size  # fixture data from caller's lease, not a new quota
        sha256 = None
        if mode == "read":
            sha256 = hashlib.sha256(_read_declared(fd, declared_size)).hexdigest()
        elif mode == "map":
            mapping = mmap.mmap(fd, declared_size, access=mmap.ACCESS_READ)
            sha256 = hashlib.sha256(mapping).hexdigest()  # hash the mapped buffer directly
        _, after = _snapshot(fd)
        if before != after:
            raise ValueError("fixture metadata changed during access")
        result = {
            "pid": os.getpid(),
            "mode": mode,
            "input": path,
            "fd": fd,
            "before": before,
            "after": after,
            "sha256": sha256,  # None in mode "open": uncomputed, not proof OS did not read
        }
    except BaseException as exc:
        body_error = exc  # recorded; re-raised after cleanup below
    finally:
        cleanup_errors = []
        if mapping is not None:
            try:
                mapping.close()
            except BaseException as exc:
                cleanup_errors.append(exc)
        if fd is not None:
            try:
                os.close(fd)
            except BaseException as exc:
                cleanup_errors.append(exc)
        if body_error is not None:
            if cleanup_errors:
                raise BaseExceptionGroup(
                    "file_access_fixture_worker: body and cleanup both failed",
                    [body_error, *cleanup_errors],
                )
            raise body_error
        if cleanup_errors:
            raise BaseExceptionGroup(
                "file_access_fixture_worker: cleanup failed after successful body",
                cleanup_errors,
            )
    return result


def main(argv=None):
    """Open a caller-leased fixture file read-only and report metadata (+ digest).

    Inputs:
        --input: absolute path to a regular, nonempty, caller-owned fixture.
        --mode:  "open" (open+fstat+close only; sha256 null = uncomputed, not
                 evidence the OS did not read), "read" (consume exactly the
                 fstat-declared bytes via os.read of the remaining length;
                 refuse unexpected EOF or trailing drift; SHA256 of actual
                 bytes), "map" (one read-only mmap of the declared size;
                 SHA256 of the actual mapped buffer; a mapping event alone
                 does not prove byte consumption in general).

    Preconditions:
        Caller already created and leases the fixture and owns its lifetime.
        This worker intentionally opens only --input (O_RDONLY, no O_CREAT,
        no writes) and no other path; the fstat-derived size is fixture data,
        not a new allowance.

    Outputs:
        On success, exactly one JSON object on stdout: pid, mode, input, fd
        (actual descriptor number), before/after fstat snapshots retaining
        dev, ino, size, mtime_ns, ctime_ns, and sha256 (hex digest or null).

    Errors:
        Everything propagates: traceback on stderr, nonzero exit. If body and
        cleanup both fail, causes are preserved together in a
        BaseExceptionGroup; mapping/fd cleanup is attempted without masking.
        A failed close does not prove closure.
        No timeout, retry, attempt limit, or output cap is applied.

    Effects:
        One read-only open of --input; in "map" mode one read-only memory
        mapping; reads bounded by the declared size. Needs Python >= 3.11.

    Limits:
        Regular files only; the whole fixture is held in memory ("read") or
        mapped ("map"); retained metadata is limited to dev/ino/size/
        mtime_ns/ctime_ns. Inert controlled-fixture helper: no environment
        inspection, network, subprocess, OS confinement, complete file graph,
        compiler, or behavior admission.
    """
    parser = argparse.ArgumentParser(
        prog="file_access_fixture_worker",
        description="Read-only access worker for a caller-leased fixture (open/read/map).",
    )
    parser.add_argument(
        "--input", required=True,
        help="absolute path to a regular, nonempty, caller-owned fixture file",
    )
    parser.add_argument(
        "--mode", required=True, choices=_MODES,
        help="open: fstat only; read: consume declared bytes; map: read-only mmap",
    )
    args = parser.parse_args(argv)
    if not os.path.isabs(args.input):
        parser.error("--input must be an absolute path")
    print(json.dumps(_run(args.input, args.mode)))


if __name__ == "__main__":
    main()
