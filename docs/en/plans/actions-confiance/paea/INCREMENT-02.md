# PAEA — increment 02 with GLM, public recipe without compilation

[Français](../../../../plans/actions-confiance/paea/INCREMENT-02.md) ·
[Plan](../PAEA-APEX.md) · [Contract](../../../PAEA-CONTRACT.md)

On October 25, 2026 the user authorized continuing “with glm”. The prior instruction
to avoid compiling while their other session is active remains in force. Codex
applies APEX/replan and personal `skynet-agents`; direct HTTP delegation gives the
model no tools, repository access, integration rights or acceptance authority.

## Decision and result

Two options were considered: rebuild Rust for active contract extensions, or
strengthen the public checks on an existing binary. The first remains unavailable
under the user's instruction and plan dependencies. The second is implemented:
a maintained offline CLI recipe, both domains, temporary roots and frozen oracles.
No Rust/Cargo source, server, mission datastore or active setting is changed.

The [final recipe](../../../../plans/actions-confiance/paea/evidence/20261025-glm-contract-bf9e66eb/public-recipe-02/result.json)
passes: **4 named tests, 88 invocations**, 17 expected refusals and 71 successes.
Refusals comprise 16 document mutations and `/dev/full` output failure. Every
fault has a valid control and recovery in the same root. Evidence integrity is
VERIFIED. Binary SHA256 stays
`addcc5eb6b32dd7d2ed963a01a80062155ccd9613985fb518c079f5a461f794f`.

| Local requirement | Observation and limit | Evidence |
|---|---|---|
| Both domains, FR/EN, exact amounts, Unicode Git | All 14 vectors match exact bytes, digests, intent IDs and explicit no-authority markers, using file/stdin inputs. | [Original transcript](../../../../plans/actions-confiance/paea/evidence/20261025-glm-contract-bf9e66eb/public-transcript-02.json), `test_all_frozen_vectors_match_public_bytes_in_both_languages`. |
| Ambiguity and forged authority refuse | Root/nested/Unicode duplicates, floats/exponents, i64 overflow, ambiguous amount, invalid UTF-8/Unicode, second document, invented private field, Git alias/mixed OIDs/no-op. Empty stdout, exit2, expected `failure.code`, no private canary echo. | `test_raw_ambiguities_refuse_between_valid_control_and_recovery`, 16 controlled cases. |
| Equivalent encodings | Whitespace, property/label order and valid Unicode escape keep exact expected canonical bytes. | `test_equivalent_input_encodings_preserve_exact_content`. |
| Failed output | `/dev/full` yields exit2 and `failure.code=storage_unavailable`, then the same valid document succeeds. No project file appears in the temporary root. | `test_output_failure_is_nonzero_after_valid_control_then_recovers`. |
| Existing artifact only | Explicit identity, stable before/after hash; selected local build metadata retained, without reconstructing a compiler graph. | [Binary observation](../../../../plans/actions-confiance/paea/evidence/20261025-glm-contract-bf9e66eb/binary-observation.json). |

Increment 01's 26 tests retain their historical evidence; these four extend the
public path. The 88 invocations are neither 88 independent tests, T01…T22 campaigns,
installation nor an E2/E3 verdict.

## GLM contribution actually used

Observed local routing maps alias `glm-5.3-flash` to `openai/glm-5.2`. Requested and
reported API model is `glm-5.2`; this does not attest the real backend version.
No shared proxy configuration is changed.

| Call | Service-reported result | Principal decision |
|---|---|---|
| [Full review](../../../../plans/actions-confiance/paea/evidence/20261025-glm-contract-bf9e66eb/glm-review/result.json) | 13,316 input tokens, 4,096 output; `length`, no final text. Requested `low` effort does not establish its application. | Unusable contribution, no verdict adopted. Original preserved, no automatic retry or raised output allocation. |
| [Focused recovery](../../../../plans/actions-confiance/paea/evidence/20261025-glm-contract-bf9e66eb/glm-focused/result.json) | Complete raw decoder/visitor only; 1,100 input tokens, 1,032 output, `stop`, final text received. | Static “no demonstrated defect” for that code alone; three regression suggestions adapted and executed by Codex. |

Total reported usage: 19,544 tokens across two calls. No monetary cost inferred
without known pricing. The exhausted first response does not mean zero consumption.
The second is neither a global contract review nor an independent evaluator role.
GLM runs no tests.

Verified adaptation: abstract unknown keys `a`/`v` would also fail schema checks.
Instead, use a valid `kind` key duplicated through Unicode, ambiguous
`schema_version`, and a second document after a valid PAEA witness. The expected
public code is `protection-document-invalid`; internal visitor details are not the
CLI error contract.

## Retained failures and replan

The [first recipe](../../../../plans/actions-confiance/paea/evidence/20261025-glm-contract-bf9e66eb/public-recipe-01/result.json)
is FAIL: two positive tests pass, but refusal assertions incorrectly read root
`code`, causing 17 `KeyError`s. The product already emits `failure.code`. Reading
actual `report()` and the transcript led to correcting the two test consumers,
then a fresh run on the same binary. Original evidence is untouched; no product
fix is inferred from this recipe defect.

[Recovery decisions](../../../../plans/actions-confiance/paea/evidence/20261025-glm-contract-bf9e66eb/public-recipe-replan.json)
also retain a refused edit using a stale documentation anchor. No partial edits
had applied; actual text was read before retrying.

Final collection preserves integrity versus success: `recipe.py verify` exits1
for intact evidence recording FAIL, and exits0 for intact PASS evidence. An initial
collector assertion incorrectly expected exit0 for the failed run. Reading the
verifier corrected that interpretation without changing verdicts or originals.

## Byte-bound review and transmission

Implemented files: `tests/paea_public_contract_test.py`, new
`tools/verification/recipes/paea-public-existing-binary.json`, bilingual contract
and tracking. The [final source review](../../../../plans/actions-confiance/paea/evidence/20261025-glm-contract-bf9e66eb/source-review.json)
is targeted self-review, not a migration qualification collector.

- Policies: no new operator choice. Vector/fault/test counts inventory this recipe;
  exit2/envelope are existing CLI contracts, SHA256 a technical representation.
  No deadline, retry, native agent or product budget. Environment variables select
  test artifacts, not active Swarm configuration.
- Functions: encoding/substitution, allocation, invocation/assertion helpers, four
  tests and `main` document inputs, preconditions, effects, errors and limits.
  Substitution requires one occurrence to reject stale mutations. `main` refuses
  missing identity/fresh evidence before collection; changed binary bytes invalidate
  the run.
- Errors: original bytes retained; read/launch/publication failures propagate,
  with nonzero failure. Existing atomic exclusive publishing preserves evidence.
  Absence of `.swarm` is no confinement proof; no effect service opens here.
- Documentation: bilingual contract, plan and queue connected. No additional local
  evolution note: recipe/docs changes alter no shipped code and no build/install
  is authorized here. No embedded context or already running process changes through
  these documentation updates.

Verified lesson: consume the actual producer envelope, keep valid controls before
mutations, and recover exhausted GLM review around one complete boundary rather
than only raising output allocation. This focused success does not guarantee later
requests.

## Remaining prerequisites

PA-00/PA-01 remain partial: authenticated mandates/receipts, active API/CLI and
sink-side proof need IA3 producer connections. Preserve concurrent Rust work.
Rebuilding or installing requires lifting the no-compilation instruction and an
available shared candidate. Activation retains Rust/IA3/AC-02 dependencies.
The sponsor still lacks both external PA-11 roles; protocol, campaigns and global
retrospective remain open. This increment automatically starts no additional paid
campaign or delegation.
