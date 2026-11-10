# PAEA31 — private atomic documents for the personal coordinator

[French](../../../../plans/actions-confiance/paea/INCREMENT-31.md).
30has39named tests/seven steps and integrity PASS, Rustdoc/FR-EN Mermaid verified.
Normal build remains active under host paging pressure: preserve effective source
inputs while compiling and supervise in bounded steps (night heartbeat is separate).
Codec30supplies exact bytes, not protected file IO/checkpoint persistence.

Extract only existing configuration Store's shared FD-based read/persist/sync
operations, preserving authorization/files/fault guards; reuse them in a generic
private Documents owner, without fake configuration Pins or copied authority
machinery. Existing-only protected directory, retained FD, private lock, immediate
mutex/flock reservation, validated simple names, explicit read/create/replace/close,
retained primary/cleanup failures. No identity/authority from filesystem labels.
Test valid witnesses before path/link/type/mode/hardlink/substitution faults,
contention/cancel/recovery and write/partial/ENOSPC/sync/close/unlock failure.
Pre-publication failure preserves originals; post-rename uncertainty is explicit.
Isolated fixtures only; no guards in normal profile. Target Store/Coordinator
regressions because shared physical IO changes. No new execution limits/retries.

Next personal coordinator persists independent checkpoint and signed pending intent
before exact batch, acknowledges/checkpoints afterward and recovers crashes without
learning from the database alone. Authentication releases checkpoint FS reservation
before acquiring held current SQLite authority, avoiding an FS lock over external
callback/inverted lock order. Service/Admin/UX/OS account/confinement remain open;
actual human/independent acceptance never fabricated. No commit/push.


## Actual progress

30installed/qualified dev128 (39tests,6CLI,56UI+guard).31initial8tests PASS.
Review discovered old Store lock-name substitution: distinct reproducer then
shared-contract correction. Added actual NOREPLACE refusal and dropped-release
failure without panic.43tests/5steps expected; qualification/delivery still open.


## Results and local delivery31

43named tests/5steps PASS, integrity VERIFIED:9private documents,1actual NOREPLACE,
15Store,9Coordinator,9control. Lock substitution counterexample failed before
correction (lock-red.*), passes afterwards; original evidence retained. Initial
missing API is compilation refusal, no scenarios counted. Initial8test success
retained before strengthening. Config Rustdoc PASS with broken internal links denied.
Byte-bound local source-review covers13files; no independent review claimed.
Supported normal dev129build has no fixtures,497effective archived sources checked
byte for byte and stable actual dependencies. Frozen CLI harness6PASS:5process
journeys use exact candidate,1old linked library witness distinct from43current tests.
Supported restart succeeded: `v0.1.1-dev.129`, SHA256
`a5772a1b1d6f4fb6103aca46d97c5cee70d23979d094ec2d5b40a2806f97db98`,
commit cbd6fecd7e40016d6c259296ac3a21373a4bbd60, dirty disclosure.56protected readonly
UI assertions and expected missing-script refusal PASS; FR/ENVersion, current notes,
commit link, light/dark themes verified and screenshots reviewed.
[Evidence31](../../../../plans/actions-confiance/paea/evidence/20261026-private-files31-s4jplcta/).
Internal file functions create no human authority or exposed personal service.
Inode checking prevents observed split exclusion, not same-UID/root tampering or
OS confinement. Lesson reaches FR/ENdocs and permanent recipe including older
Store consumer; historical evidence preserved. PA-02partial,0/17global invariants;
human/confinement/E2/E3/PA13 remain open. Continue32durable intent before bootstrap
consumption, independent checkpoint and exact recovery. No commit/push.
