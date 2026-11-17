# Controlled file-access samples

`file_access_fixture_worker.py` is a small Python ≥ 3.11 stdlib producer, run
with `-I -S -B`. The caller supplies and leases a nonempty regular fixture and
owns its lifetime. The worker does not enforce that authority or confinement.
Python startup also loads its resources: “one file” describes the worker’s
explicit operations, not the process’s complete filesystem access.

`--input` requires an absolute path; `--mode` requires `open`, `read` or `map`.
`open` opens read-only, observes and closes without computing a digest. `read`
consumes exactly the observed size, refuses premature EOF or an extra byte,
then computes SHA-256. `map` creates a read-only mapping and hashes its buffer.
The worker requires equal before/after dev, ino, size, mtime_ns and ctime_ns.
This establishes neither immutability between observations nor byte origin.
A mapping event alone does not prove byte consumption.

Healthy stdout JSON contains pid, mode, input, historical fd (already closed),
before, after and sha256 (null for `open`, meaning uncomputed). Body and cleanup
errors are retained together; a refused close does not establish closure. No
retry, global watchdog or quota is added: size comes from the fixture and the
one-byte EOF probe is a boundary check.

`recipes/file-access-fixture-inert.json` runs only the synthetic metadata-drift
guard. Supply `SWARM_TRACE_TEST_PARENT` with an admitted fixture location.
Healthy control, injected timestamp drift at unchanged size, and fresh healthy
recovery access only newly created fixture files. The recipe runs no strace or
compiler. The runner’s own temporary-file and inherited-environment contract
does not establish isolation of the actual worker.

Five actual samples retained on SWARM in `root00-cd01-file-access-4s_1x_t_`
on 17 November are read/open/read/map/read. The retained one-off
`actual-wrapper.py` uses `strace -P` for this fixture only, `-y -xx -v` and
`raw=` for reads (never content-dumping `-e read=`). Original traces, argv,
results, pinned sources/tools and reserve measurements are retained. This is
neither a generic controller nor a qualified parsing grammar.

The read samples show 64 bytes then EOF; the filtered open sample shows no
content read; map shows PROT_READ/MAP_SHARED with fd duplication. This filtered
sample does not retain munmap: it establishes no complete OS cleanup proof.
Worker marker and matching PID terminal agree in all five samples. The literal
content-absence check applies only to this fixture payload.

Complete compiler-input qualification, C-D01 admission, execution of the 12+3
bodies, installation and global acceptance remain unestablished.
