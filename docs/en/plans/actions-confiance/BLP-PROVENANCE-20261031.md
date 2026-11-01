# BLP-04 resumption — prompt provenance, 31 October 2026

## Need, decision and scope

Resumes BF-05 → BF-07 → BF-16 from the BLP plan. Owner: Codex, authorized direct
development, no delegation or paid call. `PreparationSend` has no authenticated
BLP principal. Accepting client-supplied labels/identity was rejected because it
would invent authority. The selected additive step records actual producer bytes
and origins, without enabling incomplete protection. Persistent attribution and
the current authenticated principal remain prerequisites to transport admission.

Implemented: byte-covering assembler, required private native v3 manifest,
validation on native reads before dispatch/worker/replay, inconsistent rows refuse
without mutation. Covers aggregate project context, engine instructions, plan
contract and all nine JSON fields. No confidentiality label is assigned; aggregate
sources are not resolved to individual file versions. BLP-03 migration, BLP-04
attribution/inheritance, BLP-05 sink mediation and BLP-06 declassification remain
open. Biba requirements continue to apply.

## Evidence and review incident

- `verification-blp/20261031-prompt-provenance-01/`: nine passing tests, but the
  initial verifier did not bind origin names to actual JSON keys.
- `verification-blp/20261031-prompt-provenance-02-counterexample/`: swapping two
  origin names preserves bytes and digests. This must refuse; counterexample
  execution and correction are pending verification.
- The public CLI recipe now checks v3 and compares producer digest against stdin
  actually received by an inert program. Execution is pending qualification.
- Before delivery, `./swarm.sh status` reports `running:false`, configured URL
  `127.0.0.1:38433`. This record claims no installed server.

## Contract review (final hashes pending)

Policies: schema versions 1/3, origin names and nine fields are representation
invariants. No new operator timeouts, attempts, budgets or ceilings. Existing
context limits continue to apply to actual prompt bytes. Previous engine
instruction text is preserved exactly.

Nontrivial functions: `assemble`, `Manifest::verify`, `append`, `hash`,
`native_metadata_valid`, `decode_present_provenance`, producer `preparation_send`,
consumer `record`, test helpers and `private_shape`. Contracts document absent
identity/authority and the datastore-writer limit. Public DTOs omit provenance.

Errors: schema/encoding failures occur before assembly is saved; provenance
failures occur during native read before worker handoff and worker state/budget
mutation. Existing SQL/codec/finalization causes remain preserved. Missing/null
v3 provenance never becomes empty success or historical fallback. Historical
formats explicitly remain without provenance or added BLP guarantees.

Next observable prerequisites: persistent classification of every input and an
authenticated principal; then allowed delivery witness, denied sends with zero
received bytes, revocation, source changes and response inheritance. This record
closes none of these criteria and claims no independent acceptance.
