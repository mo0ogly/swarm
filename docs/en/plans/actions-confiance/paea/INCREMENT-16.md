# PAEA — increment 16: stable debt resolution

## Recovery and decision

On October 26, `next` resumes PA-02/03 after dev80, commit
`462abbeb9bcf58e828c2fc9db5ef69b43c020bbc`. Observed service: PID 2612428,
SHA256 `f55654100c7b14684e5de2a855e9ed660d82cdb881ae35384789beba6cf95299`.
192 unrelated tracked entries remain preserved (106 staged deletions, two staged
additions, 84 modifications). Root Codex alone owns this increment.

PAEA-GUIDE-03 already has a retained counterexample: two raw aliases produce two
laboratory effects. Existing signed enrollment provides data and itself requires
current protected authority at use; Trust accepts a free resource string. Neither
interface supplies a protected business debt registry.

Options: change v1 intent to supplier/reference (migration, still vulnerable to
aliases), or preserve v1 and first resolve a stable object from an explicit
registry. Choose the latter: reusable pure library, offline CLI and a real fixture
sink/oracle check. A supplied file and this CLI never become an authority.

## Contract and observable criteria

1. Closed versioned registry: project, revision, debts; unique debt IDs and unique
   supplier/reference pairs. No automatic merging or implicit normalization.
2. Closed versioned request: project, supplier, reference, expected registry and
   invoice versions, transport ID. Registry supplies integer amount, currency and
   account. The v1 `invoice_id` receives `debt_id`.
3. Wrong project, stale versions, absent references, ambiguous mappings, duplicate
   JSON keys or invalid representations refuse without proposal or effect.
4. Aliases converge; identical references under different suppliers may identify
   different debts. Corrected terms preserve intent and change exact content.
5. Reports bind raw input bytes and explicitly establish no authority. Read,
   write and flush causes propagate; no Root is opened or mutated.
6. Named tests, permanent fresh recipes and retained integrity; normal production
   profile, local build/install and actual public flow before delivery claims.
   Policies/functions/errors review binds final source bytes.

## Limits and resumption

A protected producer must authenticate the registry, ensure freshness and stable
IDs across revisions, bind the observation into context before approval, then
revalidate under commitment order. Independent snapshots cannot detect semantic
duplicates with disjoint references or malicious reassignment between revisions.
Business/risk ownership, human approver, separate accounts, IA3/AC-02 and an
independent evaluator remain open. No real payment, protected activation, E2/E3
or end-to-end invariant is established.

Initial status: decided; implementation and verification results follow below.

## Interim verification and method incidents

`paea-payment-registry` passes: 3 schema, 6 resolver, 3 public CLI, 17 unchanged
v1 and 8 existing CLI cases: **37 named tests**, retained integrity VERIFIED.
Targeted `swarm-protection-contract --lib -D warnings` Clippy and no-dependency
Rustdoc pass. No global Clippy claim: CLI compilation retains warnings in unrelated
Rust work (`managed_repository`/`managed_review_reply`), untouched here. Owner:
Rust migration; resume by checking usage/removal with its original tests.

The first control selected `bin/swarm-local` from stale condensed context;
`/proc/2612428/exe` actually identifies `bin/swarm`. The older binary refused
storage version 37. No business success or absence of effects is inferred. The
harness asserted before saving streams: the original was not retained; this
method defect is disclosed in `method-incident.json`. Corrected preconditions:
observed process identity, verified hash, isolated cwd and streams saved BEFORE
assertions. Exact dev80 control passed, then missing command reproduced (0/2).
The maintained local delivery recipe now compares current note contents instead
of the fixed note count in increment 15's one-off script. Maintained test sources,
recipe and README transmit these lessons to subsequent contexts.

## Verified local delivery, evidence and increment retrospective

**Installed locally: v0.1.1-dev.82**, base commit
`462abbeb9bcf58e828c2fc9db5ef69b43c020bbc`, with disclosed uncommitted changes.
SHA256 `2a328c56775fad8c9715d1c0bc2ca22b9d60fdbf258a25484f1b3f0cf2154e0d`.
PID 3868897 was replaced by 3871902 during the first browser check; the initiator
is unattributed. Identical binary, interrupted script loads: UI 01 is retained
as incomplete, never PASS. The initial stale-selector hypothesis is refuted:
`#version-dialog` is correct; its scripts had not loaded. Unexpected network,
console or page failures now interrupt pending DOM waits with retained causes,
without retry or watchdog. A subsequent dark-theme attempt clicked a control in
a closed disclosure (UI 03 FAIL retained); the corrected flow opens the panel
and waits for actual finite animations. These were harness defects requiring
no product correction. Maintained test sources and this report transmit recovery.

| Contract | Actual result | Local evidence under `evidence/20261026-debt16-_la9qpcs/` |
|---|---|---|
| Schema/resolver/CLI/v1 compatibility | 37 tests PASS, integrity VERIFIED | `payment-contract-01` |
| Delivered normal profile | VERIFIED_DEFAULT_NATIVE_PROFILE; empty core features, test=false, no fixture feature | `production-profile/profile/result.json` and effective source archive |
| Final public CLI | 2 tests, 33 calls PASS, stable binary hash; documented example exercised | `public-recipe-02`, `payment-public-observations-02`, `documented-example.*` |
| Actual isolated effects | 8 tests PASS; 151 processes, 28 roots; resolved aliases: one exact effect; unresolved two-effect counterexample retained | `pilot-final-recipe-01`, `pilot-observations-02` |
| Identified instance and Version | 26 assertions PASS, FR light/EN dark, 9 unique assets, exact notes, Git link/dirty disclosure/focus, no unexpected errors | `live-recipe-04`, `live-browser-04` |
| Verification failure guard | Valid control then deliberately blocked script: expected exit 1, interrupted wait and retained cause; no server interruption | `live-recipe-04` step 2, `live-browser-04-fault-script` |
| Policies/functions/errors and diff | Semantic review of our delta bound to final hashes; no independent/global verdict | `source-review.json`, `scoped-diff-check.*` |

The dev81/dev82 candidates have 446 compared delivery inputs; only
`tools/rust-migration/build_native.py` changes (collector diagnostic preservation
by another session). Runtime sources remain identical between these candidates;
public/lab tests nevertheless run on final dev82. Profile `installed:false`
reflects qualification before installation; separate PID/binary/HTTP/Version
observations establish activation. No new commit/push for this increment; foreign
index entries remain preserved. Local delivery differs from prior publication.

Executed example, granting no authority or effect:

```sh
./bin/swarm protection resolve-payment tests/fixtures/paea/payment-registry-v1.json tests/fixtures/paea/payment-request-v1.json --json --lang en
```

The existing **PAEA checks** menu remains action/receipt correspondence. New
resolution uses the library and CLI; no new screen. PAEA-GUIDE-03 is addressed
for explicit cooperative mappings, not closed for a protected registry.
**0/17 end-to-end invariants**: protected PA-02/03, IA3/AC-02, human approvals, UX
campaigns, E2/E3 and final program retrospective remain open. Resume with a
protected producer bound to the opened project, persisted stable debt identity
and continuity/freshness under commitment order, then qualification with actual
roles. Business/risk owner and human approver designations were requested; this
report invents or assigns neither.

Follow-up owner Verification: examine the same pending-wait problem in other UI
recipes, using a valid control then a refused script. Increment 15's one-off
script is historical; its maintained successor is fixed and verified here.
The earlier native recipe owns its fixture server, a different concurrency
context; its script-loss scenario remains unqualified. No broader guarantee.

Global `git diff --check` retains the same two EOF blank-line findings already
open in increment 15: `docs/plans/migration-rust/DELIVERY-PREPARATIONS-20261025.md:85`
and `rust/crates/swarm-engine/src/root_store/task_validation/managed_receipt.rs:293`.
Our 31-path check passes. Owner of those unrelated deltas: Rust migration;
resume with its increment. Neither file is changed or included in our delta;
no global conformance is claimed.
