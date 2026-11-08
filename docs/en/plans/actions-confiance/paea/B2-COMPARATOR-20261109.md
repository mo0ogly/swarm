# B2 — hardened-workflow comparator (lot G6)

9 November 2026 · [Français](../../../../plans/actions-confiance/paea/B2-COMPARATOR-20261109.md)

Lot G6 of the [gap-closure plan](GAP-CLOSURE-PLAN-20261103.md): "Review
package, contracts/vectors, collection and **B2 comparator with the same
business protections**". Of the four G6 objects, two are delivered
(contracts/vectors, collection — increments 14-25), one is human-blocked
(preregistration: sponsor designates evaluator/reserved corpus/volumes/order,
H3). This document delivers the third: the **B2 comparator** — the
deterministic hardened workflow required by the [E3
protocol](E3-PROTOCOL.md) ("Main comparator: deterministic B2 workflow with
the same known business protections. Do not weaken B2 to manufacture a Swarm
advantage.").

## 1. Design choice: reimplement nothing

B2 here is not an imitation of the laboratory — it is the laboratory itself,
script-driven. `tools/paea/b2_comparator.py` imports
[tests/paea_pilot_lab_test.py](../../../../../tests/paea_pilot_lab_test.py)
unmodified (importlib by file path) and runs only the qualified recipe's
existing primitives:

- **pinned-CLI canonicalization** with before/after hash re-check on every
  subprocess (`PilotLab.report`) — the authentic report of the real public
  CLI `protection canonicalize`, never a Python model;
- **epoch authority** (`lab.authority` approve/revoke, `BEGIN IMMEDIATE`) —
  the same persisted `ordering` journal;
- **intent-stable sinks** (SQLite `effects(intent PRIMARY KEY)`; git:
  `refs/paea-lab/intents/<intent>` marker binding all canonical content via a
  blob, one `update-ref --stdin` transaction);
- **separate read-only oracle** (`pilot_oracle.py`, distinct process,
  `mode=ro`/`query_only`, never consulted from replies or decision logs);
- **declared reconciliation** (`RECONCILED_COMMITTED` /
  `RECONCILED_NOT_COMMITTED` / `UNKNOWN` with cause);
- **witness before every fault** (`witness`) — a valid effect under the same
  preconditions precedes every injection.

The runner's usage contract is therefore identical for B2 and for the
evaluated candidate: same `contract-v1.json` fixtures, same pinned CLI binary
re-checked at every `tearDownClass`, same fresh evidence directory created by
`setUpClass`, same pre-effect refusals (exit 2, cause on stderr). Any future
change to the pilot file changes the comparator too — intended: one
admission contract.

## 2. Scenarios (three families × two domains, fixed order)

| Scenario | Domains | What is measured (observed B2 behavior) |
|---|---|---|
| T04-substitution | payment, git | substituted content refused pre-effect (`laboratory-approval-missing`, exit 2), sink unchanged at the oracle, equivalent JSON accepted afterwards |
| T06-T21-crash-recovery | payment, git | exit 86 post-effect → one exact effect/marker, `RECONCILED_COMMITTED`, replay with a new transport key and same report accepted, intent conflict refused (`laboratory-intent-conflict`), declared `UNKNOWN`+cause |
| T05-T17-revocation-order | payment, git | revocation before first use → refusal (`laboratory-epoch-revoked`), zero effects, `RECONCILED_NOT_COMMITTED`, durable order `approve,1→revoke,2→approve,2→commit,2`, reverse order (commit before revoke) held |

Each trial is a fresh `PilotLab` on a fresh evidence subdirectory
(`T04-payment`, …, `T05-T17-git`), witnesses included. Trials are
sequential and deterministic; no accumulation of assertions counts as
independent trials ([GUIDE-INTEGRATION §16](GUIDE-INTEGRATION.md)).

## 3. Closed report and honest labels

The `paea-b2-report.json` report (schema v1, published atomically via
`tools/verification/recipe.py::publish`) carries per trial: scenario, domain,
decision (`refused-then-accepted`, `unknown-recovered-once`,
`revoked-refused-then-current-grant-commits`), measured duration, exit code,
notes. Counters `refused_pre_effect`/`recovered_unknown`/`deviations`;
nearest-rank p50/p95 latencies of trial durations.

The protocol's honest labels are unchanged and reported as-is:
`authority='simulated'`, `isolation='same-uid'`,
`independent_evaluation='not-established'`,
`protected_invariants='not-established'`, `scope='isolated-laboratory'`. A
green B2 comparator establishes neither author, nor real authority, nor
veridical oracle, nor independent evaluation.

Publication rule: **any deviation refuses publication** (exit 1, `DEVIATION`
on stderr, evidence kept). A deviation is never scored; it is retained with
its cause. Pre-effect refusals (already-existing evidence directory, wrong
binary hash, missing or unreadable binary) refuse before any trial and leave
no residue.

## 4. Actual result (9 November 2026)

Execution through the public path (recipe
`tools/verification/recipes/paea-b2-comparator.json`, pinned variable
`SWARM_PAEA_TEST_BINARY=bin/swarm` sha256
`a166e2b8759ba1d16db50651ed552c732e3340e5cb288d0297d421eee33d09c9`, delivered
binary of 9 November):

- six trials, **0 deviations**, publication OK
  (`PAEA_B2_REPORT_PUBLISHED trials=6 p50=1.976s p95=2.350s`, report hash
  `a0d2e550f9e606ae484449daf88d1ac827a170f4046ebe802d0edff9e3522c7c`);
- six `PAEA_PILOT_BINARY_STABLE` lines (one per trial); independent
  verification of the recipe evidence: `{"integrity": "VERIFIED",
  "recorded_status": "PASS"}`;
- refusal variants verified (direct development runs): existing directory →
  refusal, wrong hash → refusal, missing binary → `binary-unreadable`, no
  residue;
- deviation path probed: a wrong binary hash at `setUpClass` level (binary
  replaced between trials) is recorded as a deviation with cause
  `ValueError: existing-binary-hash-mismatch` and publication is refused;
- evidence retained under `.swarm-fixtures/b2-recipe-run2/` (campaign) and
  `b2-recipe-evidence2/` (recipe, non-erasable — originals); never under
  /tmp.

## 5. What this comparator does not conclude

- It measures the **hardened B2 workflow**, not Swarm. No Swarm advantage is
  inferred from these six trials; the candidate will be measured on the same
  scenarios under the same contract.
- Trial independence is not established: one trial = one fresh sink in the
  same same-UID laboratory; no statistical repetition claim beyond the
  measured durations.
- The driver's T05/T17 contention extension (real overlapping processes) is
  **not** carried into the B2 runner — the B2 scenario stays the persisted
  order; real contention remains qualified by the full pilot recipe.
- G6 preregistration (human designations H3) remains the remaining gate
  before any compared campaign; the review package is G6's last local object.

## 6. Concrete follow-up (resume)

1. Assemble the G6 **review package** (last local object): closed inventory
   of E3 deliverables, fingerprints, requirement→evidence mappings.
2. The compared campaign (candidate vs B2) waits for preregistration (H3)
   then the designated evaluator; no accumulation of assertions stands in.
3. Any change to `tests/paea_pilot_lab_test.py`, `pilot_lab.py`,
   `pilot_oracle.py` or the `contract-v1.json` fixtures invalidates this
   result — the B2 recipe declares them as inputs and will refuse it
   (`recipe-source-changed-during-archive` / non-reproducible evidence).
