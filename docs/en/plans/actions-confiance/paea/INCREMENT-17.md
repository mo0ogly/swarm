# PAEA — increment 17: invoice resolution from the menu

## Recovery and decision

On October 26, `go` resumes after dev82 (observed PID 3871902, SHA256
`2a328c56775fad8c9715d1c0bc2ca22b9d60fdbf258a25484f1b3f0cf2154e0d`).
Increment 16's resolver/evidence are preserved. IA3 EP-01/04/05/07/10 and AC-02
remain open; “go” assigns no human roles. Root Codex alone owns this increment.

PA-08 can progress independently: expose the resolver through **PAEA checks**.
Options: two interdependent form states in one page, or a linked resolution page
using the same file controller. Choose a separate page, shared styles/controller,
preserving receipt correspondence and avoiding duplicate session/transport logic.

## Contract and observable criteria

1. Link from PAEA checks, return links to receipt checks/cockpit. Two explicit
   registry/request files; no approval or payment submission.
2. `POST /api/v1/protection/resolve-payment` shares Host/session/origin/CSRF and
   transport bounds. Closed v1 envelope transports exact UTF-8 strings. Output
   equals the CLI resolver; no per-operation Root read/mutation.
3. Fatal UTF-8, BOM preserved; input numbers never parsed in JS. Read report i64
   revision from its exact JSON lexeme, following the existing proof contract;
   incapable browsers refuse visibly, never round it.
4. Show debt object, invoice version, integer amount/currency/account and hashes;
   download raw report, explicitly establishing no authority/authenticity/effect.
   Refusal/input change clears stale results; in-flight inputs prevent duplicate
   gestures, with no retry or new execution policy.
5. Named HTTP tests, refusal/recovery, normal-profile native browser checks:
   FR/EN, themes, 1440/390/320 px, keyboard, contrast, overflow, network errors,
   incorrect reports and exact numbers. Maintained recipes, byte-bound semantic
   policies/functions/errors review, then supported local installation.

## Limits and resumption

This checks unauthenticated files; it creates no protected registry, approval or
settlement. Durable registry, ID continuity, human roles and current authority
under commitment order remain PA-02/03. The two business UX do not become two
qualified applications. 0/17 end-to-end invariants; E2/E3/final program retrospective
remain open. No implicit new commit/push; preserve unrelated work and index.

Initial status: decided; before-change proof/results/retrospective follow.

## Replans and qualification guard

The first test used wrong `HttpReply` DTO fields (`status/body` instead of
`response.status()/body()`). The compile failure remains in `baseline.*`, without
a behavioral conclusion. After reading the producer/correcting the test: valid
receipt control then resolution route 404, retained in `baseline-02.*` before
handler implementation.

The component recipe still expected a Biba return despite increment 15's verified
cockpit change. Failure 01 is retained; update the assertion to cockpit text/href,
without changing the product to satisfy it. Replay 02 passes 219 assertions and
its component refusal/recovery checks.

Our build prepares dev83; shared delivery metadata subsequently identifies
dev84 with eight unrelated engine inputs. Hashes/archive are retained;
`previous-delivery-delta.json` lists 17 changed inputs since dev82. Our journey
was qualified on frozen dev84, not an assumed path/version. PAEA tests establish
no qualification of those unrelated functions.

Whole-web-crate Clippy initially fails on two inherited defects: adjacent
conditions in error classification and a complex preparation-query tuple.
Options: hide lints (rejected) or minimal qualification repair (chosen). One
newline preserves tokens/order; a documented alias preserves the exact query type
and body. Reverse substitutions reproduce the previous bytes exactly
(`clippy-fix-equivalence.json`). Preparation behavior is unchanged; no mirror
test is invented for an alias. `--no-deps --lib -D warnings` Clippy and Rustdoc
with private items pass; unrelated engine warnings remain, with no global
migration verdict. HTTP/final production-profile/runtime requalification follows
these two changes.

The pending-wait follow-up from retrospective 16 is implemented in the native
recipe: unexpected browser failures or owned-server exit reject pending waits.
A valid run precedes deliberately blocked script loading, with no arbitrary
deadline; received cause and expected failure are verified. Maintained tests and
this plan transmit the lesson; already running native agent contexts are not
changed. One tool call was also rejected for an extra JS parenthesis before any
action; corrected syntax caused no partial mutation.

## Delivered result and increment retrospective

**Installed: v0.1.1-dev.87**, observed PID 70398, SHA256
`d98fbaa4aa6f65c0a80bfbb06bc3515f602ed84b774c516a9579c31c07d05446`.
Actual displayed Git base: `4749f36eeca6b1bfc769d4ec7f01e55b67837dcb`, a concurrent
engine commit. Our PAEA changes remain in the working tree and are not attributed
to that commit. Version displays uncommitted changes. This increment performs
no commit/push.

| Criterion | Actual result | Evidence under `evidence/20261026-ui17-xzzkd1qc/` |
|---|---|---|
| HTTP contracts/admission | 4 schema + 4 HTTP tests PASS; valid controls, refusal/recovery, i64 maximum, unchanged Root snapshot | `http-recipe-04` |
| Preserved receipt component | 219 assertions PASS, explicitly simulated transport; stale navigation expectation repaired | `component-recipe-02` |
| Final normal profile | VERIFIED_DEFAULT_NATIVE_PROFILE; empty core features/test=false, no fixture feature | `profile-87/profile/result.json` and effective source archive |
| Frozen dev86 native browser | 452 assertions PASS, 12 receipt + 12 resolution configurations: FR/EN × light/dark × 1440/390/320; 36 actual expected refusals, no unexpected diagnostics | `native-recipe-03`, `native-browser-03` |
| Numbers/export | Exact i64-max revision/amount, original file hashes, actual completed download identical to the native report | `*-resolution/native-report.json` and exported files in `native-browser-03` |
| Client refusal/recovery | Seven browser-only altered reports rejected after native controls; missing numeric lexeme, invalid UTF-8 and duplicate gesture exercised with recovery | `payment-client-*` / `payment-*` assertions; substitutions prove no server correctness |
| Native test supervision guard | Valid control then browser-only blocked script: expected exit 1, `/paea.js` cause received, pending wait interrupted, owned server stopped cleanly | `native-recipe-03` step 2 |
| Actually served dev87 | 30 read-only checks PASS; ten unique asset byte sets, menus/empty inputs/limits, actual Version/FR/EN notes/themes/Git link/dirty/focus | `live-recipe-04`, `live-browser-04` |
| Live-check guard | Read-only control then browser-only blocked script: expected failure, service never interrupted | `live-recipe-04` step 2 |
| Static/docs | Web `--no-deps --lib -D warnings` Clippy and private Rustdoc PASS; semantic review of our delta, no independent verdict | `clippy-web-03.*`, `rustdoc-web-02.*`, `source-review.json` |

After dev85 qualification, a concurrent preparation change made an intermediate
dev86 build invalid: `PreparationDocument` instead of `Box<PreparationDocument>`
in `preparation_write.rs`. Original collector failure is retained in
`failed-profile-86`/`failed-profile-errors.json`; the launcher kept dev84 running.
The subsequently corrected evolution provides a complete, current, stable dev86
profile. We do not change that business logic. Our handlers/assets remain
identical to dev85; the 452 native checks are replayed on frozen dev86. No
unchanged retry without new evidence.

The concurrent engine commit then changes build identity to dev87. **All 448
dev86/dev87 delivery inputs are identical**, with equal default profile and
ambient overrides; only compiled version/commit/date differ. The 452 assertions
exercise dev86; the 30 live checks exercise dev87. `metadata-only-delta.json`
retains this bounded reuse, without inventing a 452-check dev87 replay. Earlier
metadata expectations that encountered dev84/dev87 remain recorded, without
claiming installation of the expected candidate.

The live recipe uses neither a real mission database as a fixture nor live POST.
Qualification POSTs use an isolated Root; malformed reports are client injections
only. Inspected FR-light-390/EN-dark-320 resolution screenshots show collapsed
help, readable controls/exact numbers and no text/hash overflow. Screenshots do
not replace browser assertions and observed responses.

Owned-scope diff and retained integrity accompany final review; unrelated global
diff findings remain open where encountered. No global migration conformance or
independent review claim. Maintained sources/recipes, README, canonical plan and
both languages transmit the verified lesson, not an invented native-context update.

Usage: **PAEA checks → Resolve an invoice reference**, select registry/request,
then **Resolve reference** and read/download the result. Repository examples
remain demonstration inputs. The registry stays unauthenticated, granting no
approval or settlement. PA-08 remains partial; protected PA-02/03, human roles,
continuity/freshness, UX campaigns, E2/E3 and final program retrospective remain
open. **0/17 end-to-end invariants**. Protected continuation requires IA3/AC-02
handoffs and actual authorities; tests never invent those prerequisites.

## Requested commit — 26 October 2026

The explicit “commit” request covers increments 16 and 17. The Git candidate
isolates their sources, tests, schemas, recipes, FR/EN documentation and two PAEA
version notes. The preexisting index is preserved; unrelated version notes and
verification sections are excluded. Local APEX queues and private raw evidence
remain local; the proof paths above therefore do not promise clone availability.

PAEA functions and journeys retain the bytes of the recorded qualifications.
The README now describes current recipes: 38 contract tests (the fourth schema
case was added in increment 17), 30 live checks and ten assets; historical 37/26
increment-16 results are preserved. Publication review retains fingerprints and
candidate isolation under `evidence/20261026-ui17-xzzkd1qc/commit-9um2zctn/`.
This Git operation is neither a new installation nor a push; the delivery
identities above remain historical observations. Open protected criteria and the
final program retrospective are unchanged.
