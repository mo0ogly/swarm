# PAEA — increment 12: actual CLI without a new build

[Français](../../../../plans/actions-confiance/paea/INCREMENT-12.md) ·
[Plan](../PAEA-APEX.md) · [Contract](../../../PAEA-CONTRACT.md)

## Resumption and artifact

User request: “next”. Root Codex continues alone in the direct checkout under
**no compilation/restart**. All 25 increment-11 inputs initially match. A new
precondition exists: the other session built **dev77** containing exact-input v2.
Instead of waiting for a new authorized build, freeze this existing artifact and
exercise it plus the workspace screen through a fixture bridge. Only tests,
recipes and documentation change; no product source, installation or real effect.

Evidence: `docs/plans/actions-confiance/paea/evidence/20261025-existing12-23lstywl/`.
Frozen SHA256 `7e7563dd5e1bdb848fec42f6ae2e619c31b048889221191517553fe45e56f6d3`;
retained version `v0.1.1-dev.77`, declared commit
`d566ed35c42a362f59e48a025d58b2b68a25b179`, modified, build
`2026-10-25T21:54:26Z`. This identifies the artifact, not the running server.

Retained build outputs, argv, Cargo streams, metadata, inventories and archive are
copied to `retained-existing-build/`; Cargo was not invoked. **434 archive members
verified**, seven current PAEA source inputs match. Observed non-test profile,
no fixture feature; protection features empty, identity features `serde`. The
collector's four core artifacts alone do not describe all feature unification;
actual Cargo messages were inspected. The new PAEA native route/assets are absent
from this archive, so it does not prove native UX delivery.

The deploy metadata's candidate path was already absent. The original retained
build result points to a content-addressed copy; its hash matches, so it was
frozen without retrying the absent path or rebuilding. A version read using a
relative path from `/tmp` also refused before execution. Tests use the absolute
frozen path. These method incidents are retained, not PAEA product defects.

## Results

| Flow | Result | Limit |
|---|---|---|
| Public CLI corpus | **18 tests PASS**, 973 invocations: 705 successes/268 expected refusals; binary stable | Python on existing artifact, not Cargo/Rust test execution. |
| Exact v2 | 331 `receipt-report` calls, both domains/FR/EN, hashes, refusal/recovery, `/dev/full` | Data correspondence, no authenticated receipt or observed effect. |
| Legacy compatibility | 231 `check-receipt` calls, exact v1 shape; same output can follow changed observation | Intentional v1 limitation retained, no implicit schema change. |
| Other checks on new artifact | 232 canonicalizations, 163 comparisons, 16 unavailable active-operation refusals | Rerun justified by changed binary identity; no live mission. |
| Screen → actual CLI | **174 assertions PASS, 48 calls**: 36 successes/12 expected refusals | HTTP fixture bridge/test session, not native HTTP. |
| Three retained recipes | **PASS and integrity VERIFIED** | Declared inputs/retained archive, no independent acceptance. |
| Component on current catalogue | **219 assertions PASS, 40 simulated requests** | Replay justified by concurrent translation change, distinct from 174 actual-CLI assertions. |
| Native API/install/end-to-end invariants | **NOT TESTED / NOT INSTALLED** | Two native HTTP Rust tests unexecuted; restart prohibited. |

Four new public tests cover whole v2/retained v1, raw document/receipt/observation
and formatting distinctions, malformed/missing/duplicate inputs, exact binding,
UNKNOWN/terminal states, runtime options/stdin/arity refusals, failed output and
recovery. Each fault follows a valid witness. Every invocation retains argv/raw
input/streams and checks fixture file inventory and bytes. No live Root, provider,
policy or effect is consulted.

`tests/paea_cli_ui_test.cjs` serves workspace assets on an ephemeral local address
and forwards one fixed operation to the actual CLI, without fabricated checker
responses. Both browser languages/domains cover exact report, wrong service,
recovery, observation change, UNKNOWN/reconciliation, forbidden transition,
invalid receipt JSON and reformatted document. Whole reports match independent
frozen vectors/raw bytes. Initial reuse of increment-11's 219 assertions was
refused during final review: another session changed the shared catalogue after
the bridge run. Retained diff replaces one preparation help entry, with no PAEA
translation changed. The component was therefore **replayed: 219 assertions and
40 simulated requests PASS** on the current catalogue, separate from 174
actual-CLI assertions. Original proofs and reuse refusal remain retained. The
bridge keeps its archived prior catalogue; unchanged PAEA labels and the current
component complete that scope without repeating its 48 CLI calls.

Permanent recipes: `paea-public-existing-binary.json` (18 checks),
`paea-cli-ux.json` and `paea-ux.json` (simulated component). Require explicit binary/SHA256; fresh transcript for the former
and fresh `SWARM_PAEA_UI_EVIDENCE` for the latter. Existing parents, fresh recipe
output then `recipe.py verify`. No global watchdog, shell, retry or hidden
subprocess maxBuffer in the streaming bridge.

## Review and resumption

`source-review.json`, static checks, increment diff and final inventory bind this
author's review to actual bytes. No independent review or delegation.

- Policy candidates: count 18, case names, ephemeral port, fixture CSRF, exit0/2,
  HTTP200/400 and domains are test expectations/setup, no product operator policy.
  Bridge executable is explicitly hashed; command/operation/files are fixed,
  service/state are data. No arbitrary shell command or Root selection.
- Functions: offline `invoke_files` allowlist gains only v2; `reported`/four
  methods and bridge helpers are documented. Python/JS syntax and consumed
  contracts checked.
- Failures: process causes/bytes retained. CLI2 becomes fixture HTTP refusal;
  bridge errors produce500 and failed tests, never empty success. UI clears stale
  reports, retains inputs and requires explicit retry. No effect to reconcile.
- FR/EN documentation now distinguishes executed v2 observations, seven unlaunched
  Rust tests, fixture bridge and native admission. **No new `version_history.json`
  release note**: tests/docs only, no product build or delivery here.

Lesson: a transient candidate pathname is insufficient artifact identity. Use
retained build results, verify SHA256 and retain provenance before execution.
Verification owner; applied here without editing the other session's metadata
or claiming a retention guarantee.

PAEA-OBS-01 v2 is **locally verified for raw-byte identity**: receipts producing
identical v1 may yield distinct v2 reports. Hashes are not signatures or proof of
effects. Historical v1 limitation and originals remain intact; PAEA-UX-02 stays
tracked in11. Resume native type/build/HTTP/embedded asset qualification and local
delivery after explicit lifting of the instruction; protected producers/approvals
need PA-02…07 and E3 needs third parties. **0/17 invariants demonstrated end to end**;
PA-08, installation, E2/E3 and RETEX remain open.
