# PAEA28 — derived personal secret and typed knowledge proof

[French plan](../../../../plans/actions-confiance/paea/INCREMENT-28.md).
27is verified/delivered. Current authority lacks actual personal-secret checking;
UID, cockpit cookie and human labels never substitute for it. Continuous direct
Codex work is authorized, without GLM. Implement canonical installation/principal/
generation-bound Argon2id v19records with OS-random salts and explicit persisted
KDF parameters, independent expected digest checked before work, constant-time
verification and an opaque borrowed knowledge proof. This creates no session,
grant, storage or human-authority decision. Tests cover correct/incorrect secrets,
substitutions, strict/invalid data, safe errors and nondeterministic salt.

No Swarm KDF defaults or hidden ceilings; caller supplies and persists memoryKiB,
passes and parallelism, validated by the library. Before a service exposes this,
installation/Admin/API/CLI must manage the same parameters. No unconfigured
server, TTL/retry/attempt policy is introduced. Next: transactional one-time
bootstrap, protected checkpoint binding, transport and actual service account/UX.
Primary sources: [RustCrypto](https://docs.rs/argon2/0.6.0/argon2/),
[Argon2 API](https://docs.rs/argon2/0.6.0/argon2/struct.Argon2.html).
Local Rust1.96 satisfies dependency MSRV1.85. Inaccessible Params/PHC web pages
will be replaced with inspection of actually resolved source, not repeated fetches.
No E2/E3 or protected global invariant established.


## Intermediate verification and recovery

Initial red is a missing API, with no executed scenario. First4tests pass.
Resolved0.6source shows feature zeroize does not wipe the allocated Blocks before
deallocation. Corrected code/doc uses an owned Zeroizing<Vec<Block>> with fallible
reserve and retained TryReserveError. Current **5named tests PASS**, including
RustCrypto's19456KiB/2passes/1lane example as a test input only, in1.96s;9core
regressions and denied-broken-link Rustdoc pass. Guard/source evidence does not
establish forensic erasure of every process copy.

`recipe-01` remains **FAIL**: the CLI step ran no scenario because concurrent
foreign cumulative-publication source was incomplete (history module/two fields).
Owner is the active chat “Auditer Swarm selon Scaling Agents”, inspected read-only;
no message or edits to its source.5+9passes never establish a full recipe PASS.
Preserve the original, resume CLI/build after corrected prerequisite. The running
server was not stopped by this test.28delivery remains open while independent
durable-bootstrap work proceeds. An initial recipe invocation used --evidence
instead of --output; parser refused before execution. A notes inspection assumed
a list; actual local_changes object/items was read before inserting the own note.


### Additive delivery evidence30

Current internal libraries are included in verified normal dev128.39current tests,
6CLI harness tests and56live UI assertions PASS; see [increment30](INCREMENT-30.md).
Historical refusals/incomplete acquisitions are preserved. No personal endpoint,
human enrollment, OS confinement or independent acceptance is established.
