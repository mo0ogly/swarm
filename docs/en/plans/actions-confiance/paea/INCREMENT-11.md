# PAEA — increment 11: UX access to correspondence checks

[Français](../../../../plans/actions-confiance/paea/INCREMENT-11.md) ·
[Adopted plan](../PAEA-APEX.md) · [Contract](../../../PAEA-CONTRACT.md)

## Need and decision

The user asked how to use the tests, whether a UX menu existed, then explicitly
requested adding it. Owner: root Codex, direct development, no delegation. The
active **no compilation/restart** instruction remains; no commit/push/real effect.
PA-08 existed in the plan but had no PAEA view/API. Options: wait for all protected
PA-02…07 interfaces or expose the available data checker now. Chosen: a bounded
first journey without authority/effect, preserving protected-approval dependencies.
This does not launch repository test suites from the browser.

## Source changes

- **PAEA checks** entry in the native cockpit menu and Trusted actions page.
- `/paea.html`: select action/receipt UTF-8 JSON files, explicit expected service
  and previous state, correspondence check, visible refusal/recovery, declared
  state and downloadable v2 report.
- `/api/v1/protection/receipt-report`: native Host/session/origin/CSRF gate, closed
  envelope, shared technical bound and the same `check_receipt_documents` used
  by CLI; no per-request Root open, policy change, mutation or effect.
- Inner JSON remains original strings without JS integer rounding. Invalid UTF-8
  refuses locally, BOM is preserved; response raw hashes must match selected
  bytes. Inner semantic validation remains the shared checker's responsibility.
- Editing/new request/refusal invalidates old success. Inputs and duplicate
  submissions are blocked while loading. `UNKNOWN` grants no further commitment;
  authority/authenticity/effect stay not established.
- Local FR/EN assets, themes, keyboard and catalogue generation. One local
  dependency `swarm-web → swarm-protection-contract`; no new external dependency,
  feature or execution policy. Schema/recipes/guides/plan/queue/local notes updated.

## Retained evidence

Canonical evidence: `docs/plans/actions-confiance/paea/evidence/20261025-ux11-xu84gb27/`.

| Criterion | Evidence | Result and limit |
|---|---|---|
| Missing initial entry | `browser-before.stderr`, plan and before sources | Original fails because `web/paea.html` is missing. |
| Initial component | `browser-01/result.json` | 170 assertions, 35 requests; no contrast assertion yet. |
| Light contrast/translation | `browser-02.stderr`, 01 screenshots | Contrast failure reproduced, French link observed in English. Fixed through shared action pair/catalogue. |
| Final component | `browser-03/result.json`, `recipe-01/` | **219 assertions PASS, 40 requests**, 20 language/theme/width combinations; explicit HTTP double. |
| Retention | recipe `verify` | **VERIFIED, recorded PASS**, declared inputs only. |
| Native route/assets | `swarm-web/tests/paea.rs` | **2 written, 0 executed**; compilation prohibited. |
| Installation/real protection/E2/E3 | PA-02…13 | **NOT TESTED / NOT INSTALLED**, program open. |

Final checks cover primary contrast, navigation translation, keyboard,
1440/1024/760/390/320 px, overflow, empty/loading, refusal/malformed/hash/authority
responses, recovery, invalid UTF-8, raw BOM/large-integer preservation,
invalidation/download and unavailable session. No global watchdog/provider/native
business response. Two diagnostic reads failed: a malformed diagnostic script,
then DevTools connection refused after recipe completion. Initial run actually
passed; no product stall was established. One documentation patch refused an
inexact anchor without observed partial changes, then was corrected after reading
the actual source. These method errors are not product-failure evidence.

## Policies, functions, errors and documentation

`source-review.json` binds this author's review to current hashes; `owned.diff`
and `before/` distinguish this increment from previously dirty work. No independent
review or native acceptance is claimed.

- Versions 1/2, ten states/five outcomes, SHA256/CSRF 64 hex and MIME/routes are
  protocol invariants. Local busy/disabled flags guard one gesture. No retry,
  timeout/quota/authority fallback. The bound remains shared transport. Widths,
  colors and 4.5:1 follow the UX contract; HTTP-double faults stay test data.
- Handler/envelope, `code/status/source`, assets/menu, UI and test helper contracts
  reviewed against comments/consumers. Rust/JS syntax and Rustfmt checked, **no
  Rust type checking, compiled Rustdoc or Rust tests**. Comments prove no execution.
- Admission precedes decoding; envelope/JSON/binding/lifecycle failures retain
  typed causes and safe public codes. UI retains inputs, clears stale reports,
  shows refusals and requires explicit retry. Native delivery/flush remain
  unexercised; this pure checker has no business effect to reconcile.
- FR/EN docs distinguish component, source API, served binary, approval and effect;
  local links and generated translation catalogue checked. No executable rebuilt.

## Improvements and resumption

**PAEA-UX-01, component-verified:** use semantic action foreground/background,
following the reproduced light-theme contrast defect. UX owner; regression
computes contrast from actual rendered colors.

**PAEA-UX-02, open:** legacy `.trust-primary` uses the same accent/nav pair instead
of action tokens. Candidate found during adjacent-source review, not a measured
full Trust journey. Owner: Trusted actions UX. Resume with valid witness, rendered
Trust button contrast, general shared-token correction and its own recipe.
Deferred to preserve PAEA scope; no Trust compliance claim follows from this test.

Next boundary: after explicit lifting of the active instruction, type/build/run
two HTTP tests, then real native browser checks on a normal-profile candidate,
protected embedded asset bytes, version/notes and supported local delivery.
PA-08 stays partial: protected producers, exact approval, separate product UX,
configured/applied/observed and vigilance remain open. This data screen does not
replace PA-02…07, seventeen invariants or external E3/RETEX resources.
