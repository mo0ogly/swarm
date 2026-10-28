# Go → Rust functional audit — October 24, 2026

[Français](../../../plans/migration-rust/AUDIT-FONCTIONNEL-20261024.md) · [Structured matrix](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/functional-matrix.json) · [Exact candidate and inputs](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/inputs.json)

**Verdict: functionality discussed and historically delivered has not all been
preserved in the observed installed Rust version.** Deterministic control and
human validation have positive witnesses. Versioned memory, instruction learning,
Trust and several product journeys remain incomplete. These findings establish
availability/parity gaps, not data deletion or a demonstrated security bypass.

## Scope, method and baseline

User request: audit preservation of the engine’s implemented behavior, not file
presence. `audit-pdca`, **audit-only**, with `replan` for evidence gaps. No engine
change, build, installation, real mission, delegation or paid provider call. The
local-install preference for development does not authorize installing concurrent
work as part of this audit.

Actual process: **v0.1.1-dev.50**, compiled commit `d566ed35c42a362f59e48a025d58b2b68a25b179`,
modified=true; SHA256 `0880206e3a0b780daaabf8697f3867adc96012d935098a9f8a341ceba0083a54`, initial PID 1642977.
The executable was retained from `/proc/PID/exe` and matched to managed state.
CLI probes use this fixed image; HTTP uses a valid session without recording its
secret. Process identity was checked before/after the probes.

The Go source capture contains **885 files** (398 production, 487 tests), plus
module files; 71 existing files differ from the reference commit and 53 were
added. A HEAD-only comparison would omit uncommitted work. This is retained
source evidence, not an approved baseline, compiler graph or embed inventory.
Older Go tests remain historical, not fresh Rust parity evidence.

A **542-source/manifest Rust snapshot** was retained. Another session then changed
Trust adapters: [observed drift](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/source-drift.json) and
[path-qualified patches](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/source-drift-patches.json).
Ongoing workspace code and the tested dev.50 image are separate candidates.
No new Trust implementation is qualified by old-binary probes.

## Executed observations

- **44 FR/EN CLI observations**: two version controls, 40 typed `command_not_ported`
  refusals, two shape refusals for prepare create. Shape refusal alone is not
  absence proof; source and declared capabilities limit preparation to reads.
  The isolated absent Root remains absent; unported commands have empty stdout.
- **23 authenticated HTTP observations**: four 200 responses and 19 404 responses
  (14 APIs, five pages). Positive capabilities/version controls distinguish
  unavailable routes from refused authentication. Native replacements are
  considered before declaring a feature missing.
- **Three maintained recipes replayed**, nine named scenarios: human/automatic/
  prior-gate validation (3), delegation/handoff and missing-control refusal (2),
  rejected-proposal recovery (4). All pass. Provider processes are inert local
  fixtures, not real models or independent model reviewers.

[Indexed probes](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/public-probes.json) ·
[Human validation](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/human-validation.result.json) ·
[Public supervision](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/supervision.result.json) ·
[Refusal and recovery](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/recovery.result.json).
Original channels, argv and recipe input hashes are retained. Observation and
scenario counts are different units and do not form a migration percentage.

## Contract matrix

PASS covers only its stated witness. PARTIAL does not qualify the entire domain;
FAIL denotes a parity/availability gap, not an unauthorized successful operation.
NOT TESTED and future proposals remain explicitly distinct.

| ID | Feature | Status | Evidence and limit | Owner connection |
|---|---|---|---|---|
| FP-01 | Deterministic engine, WorkEngine/RootStore | PARTIAL | Public mutations, controls and replay exercised below; global parity unqualified. | G3 |
| FP-02 | Human validation in the loop | PASS | Actual control → pending_human; implicit acceptance refused; explicit operator decision remains separate. Three human/automatic/prior-gate variants pass. | G3 |
| FP-03 | Supervisor, delegation, worker, handoff | PARTIAL | Real inert processes, handoff received by planner; missing controls refused before launch. No real-model review or global autonomous closure. | G3 |
| FP-04 | Diagnostics, refusal and replan | PARTIAL | Four scenarios: diagnostic received, corrected proposal applied, equivalent refusal without repeat spending, tampering/zero capacity refused then recovery. | G3 |
| FP-05 | Versioned persistent memory, selection/withdrawal | FAIL | memory and project-memory page/API unavailable; field-guide explicitly refused until its version guard is ported. Data models do not make the service usable. | G8 |
| FP-06 | Launching historical Work with memory | PARTIAL | Launch guard refuses nonempty Work.memory, project_memory, selections and memory decisions. Explicit source-level limitation; no data deletion demonstrated. | G8/G3 |
| FP-07 | Attributed instruction learning and rejection | FAIL | Base, revisions, composition and used_by_attempt exist; reject-instructions refuses and historical revise-instructions is not covered by examined decisions. | G4/G3 |
| FP-08 | Trusted actions, mandates, approvals, Biba | FAIL | Public pilot absent from dev.50. Earlier Biba audit confirmed; concurrent workspace Trust adapters are not qualified by these probes. | G7 |
| FP-09 | Trust secrets, evidence and laboratory | FAIL | evidence/laboratory/verify commands and matching Trust APIs unported in the observed binary; distinct from provider settings. | G7 |
| FP-10 | Agent identity and protected enrollment | PARTIAL | Native libraries and bootstrap-check exist. IA3–IA5, protected service and public admission were already incomplete in the Go plan; no loss of a previously delivered integration is claimed. | G7 |
| FP-11 | Preparations: edit, discuss, adopt | PARTIAL | list/methods/show/history integrated; creation/editing/dialogue/conversion/adoption not exposed by examined module; prepare.html absent. | G5 |
| FP-12 | Planning and graph editor | PARTIAL | Nine HTTP actions advertised; advanced commands, planning-bundle and graph-drafts are not equivalent to this subset; historical recovery paths need mapping. | G4 |
| FP-13 | Providers, connections, profiles, skills | PARTIAL | Native resolution/configuration and provider fixture work; tested legacy profile/skills/connections/role-model namespaces and APIs refuse. Native configuration is not wholly absent. | G6/G8 |
| FP-14 | Automation and dialogue resumption | FAIL | automation/exchange and automation/dialogue-resume-preview APIs absent in dev.50; Live alerts do not replace business triggers. | G10 |
| FP-15 | Terminal and legacy log views | PARTIAL | terminal.html and log-console.html absent; native logs/transcript exist, but interactive terminal-session parity is not established. | G2/G1 |
| FP-16 | Live supervision, OTLP and LangWatch | PARTIAL | live.html returns 200; telemetry/alert routes exist. Ingestion, probes, losses, retention and reviewer/planning coverage not replayed in this audit. | G1/G3 |
| FP-17 | Budgets, policies and persisted settings | PARTIAL | Recipes use public settings and retain reservations/attempts; not qualification of the entire catalogue, accounting or Admin journeys. | G9 |
| FP-18 | Historical data and Root migration/recovery | PARTIAL | Memory/instruction fields retained in types; no full migration, restoration or all-data non-regression qualified here. Retained Go sources are not an approved compiler graph. | R0/R6 |
| FP-19 | Identity of the actual running binary | PASS | CLI, protected API and process image identified as dev.50, with retained SHA256. No rebuild or installation. | R7 |
| FP-20 | Future AC-02 menus, YAML/SQLite and confinement | NOT APPLICABLE | Plan explicitly saved without starting; nonimplementation is not a Go→Rust regression. Same distinction for JUGE-L4 and future memory rights. | AC-02 |
| FP-21 | FR/EN, themes, focus and rendered UI | NOT TESTED | CLI refusals probed in both languages; no complete browser theme/keyboard/focus replay across all features in this audit. | G11 |
| FP-22 | Proportionate review, evidence and acceptance | PARTIAL | Actual controls do not accept the task; native review structures exist. Independent model review, large dossiers and all historical coordination not requalified here. | G3/G4 |
| FP-23 | APEX/replan methods and embedded contexts | NOT TESTED | Skills/loaders exist; documents alone do not establish all obligations actually received by each role. | G3/G5 |

## Findings and next work

| Finding | Severity/impact | Discriminating evidence | Next check and owner |
|---|---|---|---|
| FPA-01 | P1 — historical versioned-memory journey unavailable | FP-05: missing CLI/API/page and explicit versioned-source refusal | G8: catalogue, preview/apply, select, replace/withdraw and final-consumer freshness. Preserve applicable M01–M33 criteria without claiming unfinished M20 rights delivered. |
| FPA-02 | P1 — memory-enriched historical Work cannot use every native launch branch | FP-06: nonempty memory fields refused by supported_work, models retained | G8/G3: valid historical Work, actual received context, withdrawal/recovery. Retain guard until protected consumer exists; do not destructively strip data. |
| FPA-03 | P1 — attributed instruction-learning loop incomplete | FP-07: original/revision composition exists, rejection unavailable and examined decisions lack historical revise-instructions integration | G4/G3: owner-authorized additions, rejection before reservation, concurrent edits, immutable original and actual received context; fresh replay/review. |
| FPA-04 | P1 — Trust/Biba, evidence/secrets/laboratory unavailable in observed installed candidate | FP-08/09 and [prior Biba audit](../../../plans/migration-rust/evidence/20261024-biba-audit/AUDIT.md) | Existing G7 BIBA-RUST owner: finish and qualify its own candidate; valid flow, forgery/revocation/approval/effect/recovery. Do not duplicate the concurrent port. |
| FPA-05 | P1 — preparation editing/adoption and graph journey incomplete | FP-11/12: read-only preparation subset, nine planning API actions, missing consumers | G4/G5: operation/schema/rights/effect/refusal/recovery mapping, then connected editor. A diagram or subset is not full behavior parity. |
| FPA-06 | P1 — historical automation and dialogue unavailable | FP-14 CLI/API refusal and dialogue exclusion | G10: authorized triggers, identity/budget, idempotence, dialogue/context recovery; Live alerts are not business triggers. |
| FPA-07 | P2 — terminal/connections/profiles/skills consumers incomplete | FP-13/15: native config exists, historical consumers absent | G2/G6/G8: qualify a replacement for each historical contract or retain the open gap; same-candidate UI/CLI/error evidence. |
| FPA-08 | Blocks a “migration complete” claim — final qualification incomplete | FP-18/21/22/23; unapproved Go baseline and unreplayed global criteria | R0/R6/R7: actual baseline, delivered build profile, data/restore/journeys, same-candidate evidence, required review and retrospective. Plan synchronization is not functionality parity. |

Findings attach to existing groups; no new mission. Root Codex owns integration;
active BIBA-RUST and other reserved files retain their owners. Implementing these
repairs is outside the audit-only request.

## Already unfinished before Rust

Identity IA3–IA5/protected public admission, M20 usage rights and AC-02 confinement
were not completed Go features. Native identity libraries and storage are not
public protected authority. AC-02 and JUGE-L4 stay future projects; their absence
is not a regression. Instruction bases are preserved while authorized revisions
are possible: security requires attribution, authorization, guards and received
context evidence, not a claim that all instructions are immutable. Policy-required
human validation remains central and has a positive witness in this audit.

## Resumed October 25, 2026

The managed service remains on dev.50 with the same PID and hash: October 24
observations remain bound to that image. [Retained recheck](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/resume-20261025.json).
The active plan distinguishes verified but uninstalled frozen-base increment1480
from ongoing BIBA-RUST; this audit requalifies neither. New ports need installation
evidence before being declared available.

Durable Go/Rust source and recipe-helper copies are indexed in
[retention.json](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/retention.json).
The nine passing scenarios are not replayed without changed relevant inputs;
final checks verify retained evidence and reports. The matrix does not exhaust
all API, data or UI variants.

## Limits and handoff

This scoped audit/matrix is complete; global conformity is **not established**.
No global score. No fresh Go differential bench, real model, BPF run, full restore,
independent audit or universal browser replay. Effective compiler graph/features
were not reconstructed; executable observations concern the exact installed image.
Same-context semantic review is not independent acceptance.

Method failures (initially assumed paths and an ambiguous derived patch basename)
are retained and corrected in [method replan](../../../plans/migration-rust/evidence/20261024-functional-parity-aki6hq9e/method-replan.json);
none counts as missing-feature evidence. Source originals remain intact. Large
snapshots/fixtures remain in the owned cache referenced by inputs.json; indexed
results/channels are retained here. No proof cleanup, live database mutation or
restart occurred.

Recommended handoff: finish active G7, then adopt bounded G8 memory and G4 learning
slices with public witnesses, preserving other active-plan priorities. Use contract
coverage rather than file/line or same-name counts. After correction, replay the
affected scope and dependencies on the newly installed candidate.

## Implementation follow-up on 25 October — FPA-03

The user's request to execute the evolutions with APEX/GLM now authorizes
implementation. FPA-03 is locally delivered and verified in dev59: attributed
learning, CLI/API/cockpit rejection, reservation, saved Admin policy and native
context. See [FPA-03 and limitations](FPA03-INSTRUCTIONS-20261025.md). The dev50
observation above stays historical; other findings are neither erased nor marked
resolved by this slice.

Same-slice follow-up: dev60 also fixes refresh during rejection; final autonomous recipe and installation verified. The dev59 evidence and method refusals stay historical.

## October25 follow-up — FPA-02a

Dev65 retains the historical `Work.memory` consumer delivered in62, its manifest
and source guards before execution, and fixes three permission-withdrawal races.
Memory/S1 journeys are replayed on65; the62 SQLite and instruction regression
proofs remain separate. Installation65 is verified; see the
[FPA-02a report](FPA02A-MEMORY-20261025.md).
FPA-01 and remaining FPA-02 stay open: versioned catalogue/admission/selection and
fresh RETEX sources are not integrated by this tranche. Dev50 observations remain.
