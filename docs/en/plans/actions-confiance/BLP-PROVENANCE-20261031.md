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

## Evidence verified on 1 November 2026

- `verification-blp/20261031-prompt-provenance-01/`: nine initial passing tests,
  before the origin-name correspondence defect was discovered.
- `verification-blp/20261031-prompt-provenance-02-counterexample/`: retained
  **FAIL**, swapping origins was accepted despite unchanged bytes/digests.
  The correction checks the assembly grammar and actual JSON keys as well.
- `verification-blp/20261031-prompt-provenance-04/`: **11 tests PASS** after
  correction: assembly (4), native envelope/consumer (5), historical recovery (2).
  Evidence integrity was verified again on 1 November.
- `verification-blp/20261031-provenance-public-01/`: 29 CLI assertions passed;
  HTTP remained incomplete because the fixture guard still required `/tmp`.
  This failure is retained. Python helper and Rust sidecar now require the
  explicit canonical configured area, a direct fixture child and its marker;
  `/tmp` and out-of-scope placement remain refused.
- `verification-blp/20261031-provenance-public-02/`: **PASS**, sidecar guard
  (1 test), separate sidecar build, CLI (**29 assertions**) and HTTP
  (**34 assertions**). Actual inert-provider stdin matches the retained digest.
  Recipe integrity was verified again.
- `verification-blp/20261031-flow-inventory-02/`: updated inventory and five
  verifier tests PASS, integrity rechecked. This does not prove BLP mediation.
- `verification-blp/20261031-provenance-installed-01/`: **284 installation verified**,
  loaded image `874feeb290283e464711021f072e92e8973e6ac75be07b2d7994733eaaba954e`,
  protected endpoint, compiled notes and FR/EN Version dialog (**14 assertions**:
  commit link, dirty disclosure, Escape/focus, no page error or POST). The probe
  finished after interruption and retained PASS.

The initial sandbox `running:false` observation could not reach the local
listener and did not establish a stopped server. The authorized outside-sandbox
check found 283 running before replacement by 284. The first build compiled but
failed its listener probe; the authorized local qualification subsequently passed.
No original failure is reinterpreted as PASS.

On 1 November another delivery installed **290**, image
`2f104936daae347cbe46d21410a49c1485e135da9a2cfba15011e6d4eab39e2b`,
commit `18bb6bb6b5ee987e12a1f51c5ccda70fcd9db508`, with uncommitted changes.
This resumption preserves that service and does not reinstall 284. Results above
qualify their historical candidate, **not 290**. Current observation and selected
source comparison are in `verification-blp/20261101-provenance-resume-01/`.
Five of six selected files match the retained proof; `root_store.rs` changed.
Workspace/dependencies also changed, so this comparison establishes no current
global conformance.

## Contract review bound to examined bytes

Source digests still matching historical proof (self-review, no independent acceptance):

| Source | SHA256 |
|---|---|
| `preparation_prompt.rs` | `86d08b88a025ba481bd7665aaab92d17257086de41180eb3ff68ad4163cd88c4` |
| `preparation_call.rs` | `615fcf28f61903a3baeeb8363b0966cda7bac96a1e4ccbffb36a36f24050d2d1` |
| `rust_preparation_cli_calls.py` | `7d0ffdd8d1e983370cd7e5263af5f481d1ecfb9c21898ca7d9ac2b95d1dfd575` |
| `rust_project_memory_source.py` | `e5f3530c67dc22bf93a1ac28685f983e215f58aaad71097a6936346044fa5d82` |
| `memory_source_fixture.rs` | `5dfb1b3d211724eab108b3736a5648ad7fd21d305942f88983ba1064d09c9e7f` |

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

Verified lesson: valid digests do not bind semantic origin names to bytes. Retain
the swap counterexample and verify actual JSON keys/framing. For inherited recipes,
pass the configured fixture area through to the native consumer without dropping
its guard. Permanent recipes and sources carry these contracts into later work.

This resumption changes only FR/EN tracking and plan pointers. No code, setting,
embedded resource or product behavior changes; no new build, restart or evolution
note is required.
