# Security and evidence — APEX

Adopted by the user's “ok on lance”, 5 November 2026. Owner: Codex, direct
development, no delegation, paid provider or publication. Supplements
[BLP-APEX](BLP-APEX.md) without closing its seven work packages.

## Analysis and decision

BLP administration already uses `web/trust-confidentiality.js` and authenticated
`blp-show`, `blp-policy-preview`, `blp-policy-apply`. This form is neither end-to-end
coverage nor a qualification registry. `tools/verification/recipe.py` retains
declared-source checks, distinct from compiler-graph qualification and independent
acceptance. Initial observation: HEAD d91d2391a4, running version dev.332; shared
dirty checkout including admission/dialogue and HTTP service. Preserve those changes;
these observations do not qualify them.

Options: extend only the Biba form (simpler, mixes domains and duplicates evidence),
or a reusable engine projection (selected, incremental delivery). BLP is the first
domain. Biba and BLP/Biba composition require distinct criteria.

## Requirements

SP-01 separates configuration, effective enforcement, verification and acceptance.
SP-02 identifies boundaries, limits, owners and resumption conditions.
SP-03 binds results to actual sources/policy/configuration/profile/exercised binary.
SP-04 preserves historical outcomes separately from current/unknown/stale freshness.
SP-05 shares authoritative validation, preview and revisioned application.
SP-06 runs only registered authorized recipes on isolated fixtures.
SP-07 separates reading, configuration, execution, review and acceptance rights.
SP-08 protects reports and exports as potentially sensitive data.
SP-09 requires FR/EN, both themes, keyboard/focus and mobile.
SP-10 shares infrastructure without conflating BLP, Biba or composition criteria.

No overall score inferred from test volume. Result, freshness and acceptance are
independent axes. A declared catalogue never establishes PASS. A digest binds bytes,
not author or authority. External certification requires an established body,
reference and scope; the UI uses internal qualification. `runtime_enforced:false`
remains visible.

## Work packages

| Package | Dependencies | Delivery | Acceptance / recovery | State |
| --- | --- | --- | --- | --- |
| SP-L01 | — | Contracts, sources, rights, BLP scope/inventory | Authoritative sources and explicit unknowns | FRAMED — readonly contract and source inventory |
| SP-L02 | L01 | Readonly engine/API/CLI metadata projection | Current I3 admin, strict schema, no mutation; absent/stale policy | IMPLEMENTED / LOCALLY VERIFIED (readonly scope) |
| SP-L03 | L02 | Security and evidence: policy, coverage, checks, evidence, qualification | No fabricated PASS; FR/EN, themes, keyboard, errors, mobile, actual assets | PARTIAL — fixture UX verified; real Authority session open |
| SP-L04 | L02–03 | Structured administration reusing existing form | No-op preview, stale revision refusal, persistence/restart, shared validation | PLANNED |
| SP-L05 | L02–03 | Recipe adapter, rights, progress/stop, durable evidence | No arbitrary command/path; unknown recipe refused; failure/interruption/recovery retained | PLANNED |
| SP-L06 | L04–05 | Qualification dossier, review, decisions, protected export | Missing/failed/stale mandatory criteria block acceptance; exact candidate/reviewer | PLANNED |
| SP-L07 | L01–06 | Public recipes, review, local installation, retrospective | Active image, FR/EN notes, served flow; independent review distinct | IN PROGRESS — image active; global review/qualification/RETEX open |

Codex owns implementation. Authorized operators own policy decisions, independent
reviewers own review, acceptance authorities own decisions. This document grants no
role. Initial metadata access requires the existing current I3 admin; distinct read
rights remain L06. Reuse the protected Trust path without adding Authority rights;
an Authority session is not a Trust I3 credential.

## Verification and delivery

Targeted tests with a valid control before faults: missing/invalid/non-admin/disabled
identity, malformed/unknown request fields, absent/current/stale policy, no leaked
grants/content/paths/secrets, byte-identical retained state. CLI/API contract parity.
Browser: loading/empty/error, safe text, FR/EN/themes/focus/keyboard and widths
1440/1024/760/390/320; actual embedded HTTP bytes, non-fixture profile, active version
and bilingual notes. No hidden timeout/cap for future execution: operator policy
must persist and be editable. Interrupted/uncertain runs cannot become PASS.
No paid providers.

Future evidence prefers mounted/writable/sufficient Lexar; Unix fixtures/staging stay
under configured fixture storage. Archive metadata, sync/read back/hash USB bytes,
retain local location/hash/restore index. Preserve historical originals. If unavailable,
record local fallback and available space.

## 5 November replan

The previous plan lived only in chat; this file and its French counterpart preserve
continuity. Do not repeat stale BLP states: BF-16a code exists but the last commit
still declares its public evidence UNVERIFIED. Inspect contracts/hashes before reuse;
concurrent edits are outside our acceptance.
First increment: shared read projection and BLP UX (L01–03), linking existing policy
administration. Recipe execution and acceptance remain unavailable until L05–06.
No work package is removed by this incremental delivery.


## First activated increment — 5 November 2026

Image v0.1.1-dev.336, SHA256
`23e4a1d64d958cbaed472e5a44f6b103f9b2e1f2997bba25488976e02b3f5214`,
compiled commit d91d2391a4c5e0bf767c9cd1f09e1a2e34986965, modified:true.
Qualified build `rust-xzx_n63t`: normal profile, 4 core artifacts with empty features,
no fixture features, 764 archived effective inputs; 15 scoped inputs match compiled
bytes. Exact prepared restart: zero newer compiled inputs pending, observed ready
PID 2268485. Authority active on 38433, internal engine 38434,
installed_matches_build:true. This dated observation does not cover future restarts.

Evidence: `.cache/security-proofs-20261105-01`; durable
[delivery/restore index](../../../plans/actions-confiance/security-proofs/DELIVERY-20261105.json).
Engine recipe `engine-after-02`: VERIFIED/PASS, 1 named test. Public recipe
`public-recipe-04`: VERIFIED/PASS, 22 CLI/HTTP and 77 browser assertions covering
FR/EN/themes/keyboard/5 widths/normal text contrast after finite animations finish.
Inventory `inventory-recipe-02`: VERIFIED/PASS, 69 references, 25 OPEN flows,
5 checker refusal/recovery tests. Same-image owned Version dialog fixture:
56 additional assertions, FR/EN notes/commit link/dirty disclosure (`version-fixture-01`).
Counts establish neither conformity rates nor independent acceptance.

`installed-01` remains PARTIAL_AUTHORITY_UI_NOT_VERIFIED: image/version/22 known
BLP inputs verified, Authority mode and three owner-key UI denials confirmed.
The real view was opened and reached Swarm login; no account used, new right or
fabricated identity. Open criterion owned by the user: existing human login, then
real view/Version dialog checks. The 77 browser assertions remain fixture evidence.

Next: L04 structured administration reusing producers, L05 registered recipes with
rights/progress/stop/recovery and persisted Admin policies, L06 protected dossier
and candidate-bound acceptance, L07 independent review and global retrospective.
L03 still needs these interactions and real-session coverage. No historical evidence
import, content export, qualification decision, Biba/composition integration or real
classified data. Program remains open; runtime_enforced:false is retained.

### Increment retrospective and discoveries

- Chat-only plan now persisted FR/EN and linked from BLP-APEX; future resumption
  reads this record. No embedded native agent context changed.
- Original `before.*` failure: security-show did not exist. Original refusal kept;
  the named regression now passes.
- `public-recipe-01` FAIL: pinned Puppeteer Chrome absent, no browser interaction.
  Selected installed Chrome with shared Node/npm/Chrome input collector; fresh
  positive control and actual versions/hashes retained. Never assume browser cache
  or inherit an implicit CDP watchdog.
- Stale inventory: seven changed files reread for anchors, including concurrent
  edits; hashes/lines refreshed without retroactive BF-16a/storage qualification.
  Original drift preserved. `inventory-recipe-01` still failed despite a green
  component because its assertion expected 67 references; updated manifest to 69,
  retained two new sources and ran fresh complete recipe/5 tests. Component success
  is not runner success.
- Long mobile screenshots led to explicit disclosures for boundaries/recipes,
  keeping blocking reasons visible; effect checked at five widths and by keyboard.
- Shared disk/IO pressure during a dremio_ai Docker writer: request routed, no
  other's process stopped; process ended naturally and space recovered. First
  build 9m50, second 1m30; correlation is not exclusive causal proof or a timeout
  policy. No evidence cleanup or watchdog added.
- Changed verification inputs require fresh scoped checks; historical outcomes
  stay preserved and reused only within their exact unchanged scope.
