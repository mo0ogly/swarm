# PAEA — adopted APEX plan and execution record

[Français](../../../plans/actions-confiance/PAEA-APEX.md) ·
APEX queue (`../../APEX-QUEUE.md`, local tracking outside this publication) · AC-02 (`securite-execution-apex.md`, local tracking outside this publication)

## Authority and objective

October26 update: “fini push compile” authorizes compilation, scoped PAEA
commit/push and local delivery under standing authority. The compilation ban is
lifted; business authority and protected prerequisites remain unchanged.
See [increment 15](paea/INCREMENT-15.md).

On October 25, 2026 the user requested implementation of PA-00 through PA-13.
Root Codex owns integration. No implicit delegation or parallel execution.
Rust migration priority and prerequisites remain applicable. No commit, push,
real payment, external deployment or resumption of paused missions is granted.

Demonstrate for BOTH fictitious billing and software development that each
observed protected effect matches an authorization still valid for its exact
content at commitment. Activation is per project; missing required protection
denies the affected operation. The final target is experimental E3 with an
independent evaluator, a second implementation and preregistered comparison.
This is neither certification nor C3/C4 deployment authority.

The original PAEA v0.2 DOCX/PDF under `docs/security/` remain historical and
unchanged. Contract, code, local tests, installation and independent evaluation
must retain separate states.

The subsequently supplied [operational guide](../../../security/PAEA_v0.2_Guide_Operationnel.docx)
is a working implementation reference with [mapping and clarifications](paea/GUIDE-INTEGRATION.md).
Its examples replace neither versioned schemas nor session authority.

## Architecture and shared contracts

Reusing only in-process checks cannot protect control files/secrets from an
agent with the same UID. Retain existing kernels but separate agent execution,
protected control, effect services and observation under distinct trust boundaries.

```mermaid
flowchart LR
  U[Project / Business UX and CLI] --> A[Authenticated API]
  A --> C[Protected control]
  C --> L[Verified launch]
  L --> G[Confined agent and descendants]
  G --> W[Assigned workspace]
  G --> B[Broker]
  B --> C
  C --> F[Fictitious settlement service]
  C --> R[Fixture Git service]
  F --> O[Effect observer]
  R --> O
  C --> P[Attributed evidence]
  O --> P
```

Target architecture, not evidence of implementation. Control and keys stay
outside the agent domain under a distinct account. The first Linux profile covers
namespaces, mounts, privileges, FDs, processes and network. Authenticated peer UID
alone establishes no mandate. Bind the opened project instance, principal,
workload, scope and current authority. AC-02 policies retain SQLite persistence
and YAML exchange; existing global JSON settings are not silently relocated.
Admin/API/CLI share authoritative validation and effective values.

- Proposals are non-authoritative; control supplies admission context.
- Version schemas/canonicalization; reject duplicate/ambiguous/unknown fields.
- Derive business intent independently of transport request keys.
- Payment binds invoice ID/version, exact integer amount, currency and resolved
  account. Git binds assigned repository, direct ref, expected old OID, immutable
  candidate and existing checks/review/engine decision.
- Bind exact action, ordered sources and control versions. Relevant changes
  invalidate approval.
- Serialize current authority/revocation with effect commitment. Git/SQLite are
  not made atomic by an API call: retain durable intent and reconcile CAS updates.
- UNKNOWN is neither no-effect failure nor success; block potentially duplicating
  commitments until compatible reconciliation. Preserve prior effects/consumption.
- Legacy Trust paths must use protected control or explicitly refuse in protected
  projects. Errors preserve causes, safe codes and known/uncertain publication.

```text
PROPOSED → ADMITTED → APPROVAL_PENDING → AUTHORIZED → COMMIT_REQUESTED
COMMIT_REQUESTED → COMMITTED | DENIED | UNKNOWN
UNKNOWN → RECONCILED_COMMITTED | RECONCILED_NOT_COMMITTED
```

### PAEA-ERR-01

PAEA v0.2 §6.2 option (b): a content-bound single-use token can still be consumed
for the FIRST time after local revocation when the effect service does not check
that revocation. Uniqueness alone does not prove I17. Also require current
status/epoch under commitment order or disclose/prove propagation limits and
suspend uncovered critical use. Owner: PAEA author/effect control. Observable:
issue token → effective revocation → first redemption denied without effect.
Specified erratum; distributed evidence remains in PA-03/PA-12.

## Work packages and acceptance

| Package | Owner and dependencies | Work and observable output |
|---|---|---|
| PA-00 Baseline | Root Codex; current plans/source | Inventory Git/runtime, reservations, 17 invariants, evidence/resources and both controls. Bind exact bytes; never inherit Rust qualification from Go. |
| PA-01 Contracts | Codex; PA-00 | Schemas, canonicalization/vectors, business intents, commit order, errors, public operations, migration and bilingual diagrams. Testable contracts for both domains; data library grants nothing. |
| PA-02 Authority | Identity/integration; PA-01 + required IA3 output | Reuse enrollment/instances; separate control/keys, bind opened project, current mandates/approvals/revocation and shared Admin/API/CLI settings. Valid admission and attributed denials; missing protection fails closed. |
| PA-03 Effect services | Trust/managed repository; PA-01/02 | Separate fictitious ledger and conditional Git promotion, immutable candidate/checks/review, last-use authority, semantic uniqueness, durable intent and reconciliation. T04/05/06 in both journeys; new request keys never duplicate effects. |
| PA-04 Confinement | AC-02/A3; PA-02 + AC-02 prerequisites | Every launch/recovery/descendant; mounts/FS/FD/env/signals/detached processes, before untrusted code. Valid control and bypass test for each inventoried path. |
| PA-05 Network/secrets | AC-02/A4; PA-04 + AC-02/A2 | Mandatory broker, IPv4/6/DNS/redirects/sockets/persistent connections, control-side secrets, effective destination/labels, model/dependency/artifact flows. Valid traffic and zero observed forbidden sensitive bytes. |
| PA-06 Composition | Identity/Memory; PA-02/05 + required MEM-P2…P4 | Preserve instruction/data separation, provenance/versions/integrity/confidentiality over read/summary/transmission/compaction. Attenuated delegation and authenticated peers. No instruction/authority promotion in the defined scenarios. |
| PA-07 Tools/time | Launch/catalogue; PA-01/02/04 | Bind effective binary/schema/config, readmission on mutation, monotonic local leases and epochs/generations across restart. Inventory and ACS mapping only claim actual coverage. |
| PA-08 UX/approval | UX/integration; PA-02…07 interfaces | Admin with each policy; exact consequences/content/approver, visible denial/UNKNOWN, configured/applied/observed distinction, separate UX builds and measured vigilance. Both public journeys, FR/EN/themes/mobile/keyboard/focus/error/recovery. |
| PA-09 Evidence | Verification; PA-02…08, collect from first increment | Link proposal/admission/approval/commit/observation, preserve originals, separate key acquisition, detect tampering/omission/replay/rollback. Independent verification without believing agent reports. |
| PA-10 Local E2 | Codex + separate-context required review; PA-03…09 | Same candidate, actual no-fixture profiles, isolated CLI/API/browser, legacy bypass denial, supported build/install, process/hash/assets/version/FR-EN notes. Bounded E2 statement, not E3 closure. |
| PA-11 Prepare E3 | Sponsor + third-party team + evaluator | Designate independent roles; second effect boundary without Swarm control code, common schemas/vectors, preregister corpus/models/budgets/faults/metrics, B2/ablations and reserved attacks. Accepted resources and protocol before campaigns. |
| PA-12 E3 campaigns | Evaluator; PA-10/11 | T01…22/adaptive, direct effect oracles, equal-condition Swarm/B2/second implementation, ablations, authorized harmless human trials, full report and acknowledged defect handoff. |
| PA-13 Corrections/RETEX | Codex + evaluator; PA-12 | Reproduce, fix shared contracts, requalify affected earlier packages/verdicts, deliver verified candidate, final retrospective and learning transmission. Any required unresolved criterion keeps the program open. |

AC-02 retains Rust qualification prerequisites. Independent contract/protocol
preparation can proceed. Preserve IA3 → protected memory use → IA4 handoffs;
whole IA5/MEM-P5 closure is not an added circular dependency.

## Assurance and experimental protocol

| Invariant | Packages | Tests | Required observation |
|---|---|---|---|
| I01 Model non-authority | 01/02/06 | T01/T02 | Hostile contents cannot create permission. |
| I02 Identity/delegation | 02/06 | T03/T14 | Attributed principal and attenuated child rights. |
| I03 Provenance/flows | 05/06 | T02/T18 | Actual received context retains labels and versions. |
| I04 Exact approval | 01/03/08 | T04/T20 | Approved canonical content equals consumed action. |
| I05 Last-use authority | 02/03 | T05/T17 | Current authority at ordered commitment. |
| I06 Mediation | 04/05 | T07/T19 | Closed path inventory with actual enforcement. |
| I07 Secrets | 02/05 | T08/T17 | No critical unbounded effect credential in agent domain. |
| I08 Business uniqueness | 01/03 | T06/T09 | Same derived intent cannot repeat with a new key. |
| I09 UNKNOWN/recovery | 03/09 | T06/T21 | Reconcile service state without duplication/fabricated success. |
| I10 Human supervision | 08/12 | T10/T22 | Attributed qualified decisions and evaluated vigilance limits. |
| I11 Independent evidence | 09/12 | T12/T21 | Separate observer/anchors and attributable bounded proof. |
| I12 Safe failure | 03/04/05 | T10/T16 | No new sensitive effect without required preconditions; C4 excluded. |
| I13 Egress/confidentiality | 05/06 | T13/T19 | Effective destination and actually received bytes. |
| I14 A2A | 06 | T14/T18 | Peer, mandate, audience and reception linked. |
| I15 Tool qualification | 07 | T15/T20 | Changed effective tool requires readmission. |
| I16 Time/order | 02/03/07 | T16/T05 | Control clock/epochs, never agent-declared time. |
| I17 Sink revocation | 02/03/05 | T17/T21 | First redemption of pre-issued token fails after effective revocation. |

T11 additionally covers changed policy/source/candidate and stale proof. T09
includes split/cumulative effects across agents. Establish a valid control before
each injection. Read before/after service state; HTTP200/exit0/PASS labels alone
are insufficient. PA-01 local cases are not executed T01…T22 campaigns.

Keep forbidden effects, business/software success, false refusals, recovery/UNKNOWN,
p50/p95/p99 latency, cost/consumption/operator load and coverage separate.
Preregister volumes and budgets without universal magic numbers. Statistical
intervals require their assumptions. Publish low/no benefit against B2 honestly.

## Delivery and current state

Every increment retains exact inputs/candidate, policy/function-doc/error/document
reviews, actual compiler/features/binary/assets, permanent recipes and fresh
integrity-verified evidence. Changed inputs invalidate affected proofs. No arbitrary
global watchdog, erased history, fabricated acceptance or implicit paid calls.
Use replan with original cause, changed precondition, authorized action and observed
effect, then resume the objective.

Milestones: M0 baseline; M1 both controlled effects; M2 mediation; M3 composition;
M4 installed local E2; M5 independent E3 campaigns; M6 re-evaluated corrections and
final retrospective. Program remains open after M4. Disabling project protection
requires an authorized decision; no automatic permissive fallback.

October 25: PA-00 and PA-01 are partially implemented. The
`swarm-protection-contract` library, schema, fourteen vectors and public offline
`protection canonicalize` command cover data for both domains. This command
grants no authority and activates no protection. Complete authenticated mandate,
authority, approval and receipt contracts, and active CLI/API operations, remain
to be finalized with their protected producers. The [PA-11/E3 protocol](paea/E3-PROTOCOL.md)
is drafted, not preregistered. The user confirmed having no independent evaluator
or third-party team. No GLM/SKYNET call or delegation occurred in this increment.

The [retained recipe](../../../plans/actions-confiance/paea/evidence/20261025-contract-qekt2uwo/recipe-01/result.json)
ran 5 Python tests, 17 Rust contract tests and 4 CLI tests: 26 passed, zero failed.
Evidence integrity was verified. Clippy and Rustdoc also passed for the library
alone. These results establish no T01…T22 campaign, effect service, confinement or
E2/E3 qualification. The recipe inventory is declarative, not the full effective
compiler graph. Later documentation corrections are reviewed separately; original
evidence and corresponding executable inputs remain preserved.

### Replan: concurrent session and no compilation

The user is working in another session and requested avoiding compilation. No new
Cargo, Rustdoc, build, installation or restart will be launched by this conversation
under that instruction. Its last Rustdoc check had completed. Inspection found the
concurrent SCALING-S2 work in the Rust plan and effective managed-repository changes;
they remain with their owner. No other session's process is interrupted.

The instance observed before this instruction ran `v0.1.1-dev.65`; this PAEA
increment is not declared installed. Local integration owns delivery. Resume after
the instruction is lifted and the shared checkout is available, with full candidate
review and the supported delivery path. Global migration and required IA3 handoffs
remain open in inspected plans; AC-02 prerequisites remain in force. This foundation
starts no account, privilege, confinement or effect-service changes.

Owned files: this bilingual plan/contract, isolated library, schema/vectors/tests/
recipe, CLI `protection` module, targeted `swarm-cli/src/lib.rs` connections,
Cargo dependencies, bilingual `version_history.json` note, queue entry and own
evidence. No engine Trust, launch, supervision, active policy or live mission file
is reserved here. Preserve unrelated changes in shared files.

The [increment review](paea/INCREMENT-01.md) retains tested scope, all three review
axes, limits and resumption conditions. PA-00…PA-13 and its final retrospective
remain open.

### Increment 02 — authorized GLM contribution, without compilation

The user's “go next avec glm” authorizes targeted internal GLM contribution.
The no-compilation instruction remains active. The
[new public recipe](paea/INCREMENT-02.md) ran four tests and 88 invocations against
the existing binary: 16 document refusals and one output failure, with valid
controls/recovery. Evidence PASS/VERIFIED; no Rust edits or installation.
The usable GLM contribution covers the raw decoder only. Exhausted first call
and failed first recipe remain preserved with verified recovery. PA-00/01 and
previous resumption conditions stay open.

### Increment 03 — routing refusals with GLM

[Two tests added](paea/INCREMENT-03.md): six public tests total, 248 invocations,
69 expected refusals with exact recovery in both domains and FR/EN. Existing
binary stable, no compilation or restart. GLM proposal corrected before execution,
originals retained. PA-00/01 partial; IA3/Rust/AC-02/E3 dependencies unchanged.
No full program lot closed.

### Increment 04 — export consumer and corrected GLM method

[Correspondence tool implemented](paea/INCREMENT-04.md): nine named tests,
42 CLI observations across both domains, no compilation or installation.
No semantic admission/authority/effect established; PA-09 only started.
User correction removes default GLM client cap: nine local HTTP tests and actual
request without limit fields retained. PA-00/01 and Rust/IA3/AC-02/E3 prerequisites
remain open.

### Increment 05 — Rust comparison, not qualified

[Rust sources and three tests written](paea/INCREMENT-05.md), no compilation or
Rust test execution. New Python extension withdrawn after explicit user correction,
originals preserved. No CLI/API integration, installation or authority verdict.
PA-01/08/09 remain partial; qualification and IA3/AC-02 prerequisites open.

### Increment 06 — Rust CLI integration, not qualified

[Compare command and four CLI tests written](paea/INCREMENT-06.md), without
compilation or execution. Existing reader, two named files, output schema and
FR/EN help; recipe updated, not run. No project/effect/approval activated.
No-compilation persists; PA-01/08/09 and qualification remain open.

### Increment 07 — receipt/action/lifecycle source contract

[Rust checker and four tests written](paea/INCREMENT-07.md), uncompiled/unexecuted.
Reuses action/intent digests, expected sink and UNKNOWN/reconciliation lifecycle;
no authenticated receipt or effect service delivered. PA-01/03/09 and PA-02/AC-02
prerequisites stay open. No project activated.

### Increment 08 — receipt checker CLI consumer

The user resumed the program and explicitly reconfirmed **no compilation or
restart**. [Rust CLI adapter and four tests written](paea/INCREMENT-08.md),
uncompiled/unexecuted: two files, explicit sink/prior state, canonical recomputation
and binding/lifecycle checks. FR/EN help, schema, recipe and contract updated;
authority, authenticity and effect remain unestablished. No GLM call or service
started. Output-error tests are isolated outside the caller checkout; their effect
is unverified. PA-01/03/09, qualification, installation, protected dependencies and
RETEX remain open. Next proof: affected targets and recipe on frozen inputs after
explicit lifting of the instruction.

### Increment 09 — verification loop on an existing candidate

[14 public tests passed, 618 invocations](paea/INCREMENT-09.md), 167 expected
refusals, recipe PASS and integrity VERIFIED. Existing dev75 candidate from
another session frozen without compilation/restart here. All 432 effective input
archive members verified; eight PAEA inputs match observed sources. Two
increment-08 Rust assertions corrected to `failure.code`, unexecuted by Cargo.
Second loop: justified transitive Identity `serde`, removed temporal schema
descriptions and reexamined earlier lots without rewriting originals. PAEA-OBS-01
retains standalone report limitations with the applied transcript mitigation.
Six invariants have local structural observations; **0/17 demonstrated end to end**.
Protected PA-02/03/08/09, installation, E2/E3 and RETEX remain open. No-compilation/
restart instruction stays active: Rust tests after explicit lifting, effects after
protected IA3/PA-02 output, E3 with required third parties.

### Increment 10 — PAEA-OBS-01 implemented in source

[Standalone v2 report and seven tests written](paea/INCREMENT-10.md): exact raw
document/receipt hashes, canonicalization and the same binding/lifecycle checker.
New `receipt-report` command; v1 `check-receipt` shape retained in source, v1 inputs
unchanged. No new right, effect or dependency. FR/EN help/schema/recipe/local notes/
contract updated. **Uncompiled/unexecuted/uninstalled** under the active instruction.
Existing dev75 witness records v1 control, absent new route and recovery without
a build. PAEA-OBS-01 remains functionally unverified; increment-09 proofs retain
their archived source scope and do not qualify new code. PA-01/09/program open.


### Increment 11 — explicit UX request

After asking how to use the tests, the user explicitly requested adding UX access.
PA-08 now explicitly includes a menu entry. [First journey written](paea/INCREMENT-11.md):
**PAEA checks** in the native cockpit and Trusted actions; select action/receipt,
explicit service/state, correspondence check, visible refusal/UNKNOWN and v2 report.
Authenticated API uses the shared pure checker with no effect or activated
protection. Browser component is separately verifiable; native route, actual
binary assets and installation remain unqualified under no compilation/restart.
Two HTTP Rust tests written and permanent browser recipe added. PA-08 remains
partial: protected approvals/configured-applied-observed, both UX products and
vigilance still require PA-02…07 interfaces and the planned experiments.


### Increment 12 — qualification on an existing candidate

[Public and actual-CLI screen evidence](paea/INCREMENT-12.md): dev77 candidate
built by the other session, frozen without compilation/restart here. 434 effective
archive members verified, seven PAEA inputs match. 18 public tests passed, 973
invocations including 268 expected refusals; local v2 OBS-01 distinction verified,
v1 preserved. Screen: 174 assertions and 48 actual CLI calls through an HTTP fixture
bridge, FR/EN and both domains; no fabricated checker response. After another
session changed the catalogue, component replay passed 219 assertions/40 simulated
requests. Three recipes PASS, integrity VERIFIED. This bridge is not native HTTP: its two Rust tests, served
assets, installation and protected approvals remain open. Original 09…11 proofs
are preserved; no seventeen-invariant/E2/E3/RETEX closure.


### Increment 13 — operational guide and UX journey

The user supplied the operational guide to inform implementation.
[Complete mapping](paea/GUIDE-INTEGRATION.md) retains the original, maps chapters
to owners/plan/proofs and defines three dossiers plus target UX. Existing file
checking stays partial. Functional priority: effect card/owner/boundary/oracle
and three recipes in both pilots, preserving PA-02/03 and Rust/AC-02 prerequisites.
ERR-01 also applies to guide7.1; pedagogical JSON/figures do not replace schemas;
project+invoice intent requires protected project-unique debt resolution before
the sink. IAM/industrial execution remains out of scope. The [increment record](paea/INCREMENT-13.md)
also retains bounded PAEA-UX-02 contrast work, normal/hover/keyboard tests and
component-versus-installed-binary distinction.

### Increment 14 — two laboratory effects and separate observation

[Operational dossier](paea/OPERATIONAL-PILOT.md): payment/Git cards, missing
authorities, access graph and three executable recipes without a build.
SQLite and Git branch+canonical-marker transactions produce effects in 26
isolated roots; separate oracle process reads no receipt/decision journal.
**7 tests PASS, 132 subprocesses, recipe PASS/integrity VERIFIED**. T04, lost reply
and revocation before first use exercised in both domains with controls/recovery;
these are fixture orders, not distributed evidence. Git negative witness:
context-ignoring replay guard triggers the expected failing assertion.
PAEA-GUIDE-03 reproduces two entries for raw aliases without a business registry;
counterexample retained, protected debt resolution required before PA-03.
[Record and limits](paea/INCREMENT-14.md). Common UID/simulated authority: **0/17
end-to-end invariants**, protected PA-02/03, campaign menu, native qualification,
installation/E2/E3/RETEX open. No build or restart.

### Increment 15 — native qualification and authorized delivery

The user lifts the compilation ban. Checkout and targeted worktree on the
published base: **5 Python and45 Rust tests PASS**, including the two previously
unrun HTTP tests. Normal dev79 build prepared before any stop; real executable
and embedded assets in the browser: **237 assertions/12 configurations**,24
expected refusals, no unexpected diagnostics. Eight protected assets match
sources; FR/EN/themes/mobile, Version dialog, Git link and focus verified.
PAEA returns to the cockpit independently of Biba.
[Evidence, publication scope and delivery](paea/INCREMENT-15.md); generated local
result retains actual remote commit and active version/PID/hash. Correspondence
menu is qualified; lab campaigns remain outside UX. This delivery closes neither
protected PA-02/03, the **17 invariants**, nor program E2/E3/RETEX.

### Increment 16 — explicit debt identity

[Decision and criteria](paea/INCREMENT-16.md): preserve the v1 hash and resolve
supplier/reference to a stable debt object before admission. Pure library and
offline CLI reject collisions/stale versions; a separate fixture observation
checks one effect for two aliases. Supplied snapshots remain unauthenticated;
revision continuity and protected authority remain open.

Increment 16 verified and installed as **dev82**: 37 contract tests, 2 public tests
(33 calls), 8 fixture-effect scenarios and 26 live-instance checks PASS; browser
failure guard verified with an expected failure after a valid control. Sources
remain uncommitted; no new protected assurance or E2/E3 closure.

### Increment 17 — invoice resolution from the menu

[Contract, decision and evidence](paea/INCREMENT-17.md): PA-08 exposes the shared
resolver from PAEA checks, with a dedicated help/download page. Closed native API,
exact i64 values, refusal/recovery and explicit non-authority markers. Receipt
correspondence remains available; no protected registry activation.

Increment 17 delivered on **dev87**: 4 schema/4 HTTP tests, 219 component checks,
452 native assertions on dev86 with 448 identical delivery inputs, 30 actual
dev87 checks PASS. Build version/commit/date differences and bounded proof reuse
are retained. Clippy/Rustdoc PASS; script-loss guards verified after controls.
PAEA changes remain uncommitted; protected registries/approvals and E2/E3 open.

Publication requested on 26 October: isolated increments-16/17 commit, without
push or a new delivery; see the publication follow-up at the end of report 17.
The precommit statements above retain their historical scope.

### Increment 18 — GLM resumed

“reprends glm” authorizes a targeted internal contribution on delivered invoice
resolution/API/UI contracts. [Criteria and follow-up](paea/INCREMENT-18.md): each
finding must be reproduced or refuted, then corrected/verified when justified.
Protected handoffs and assurance limits remain open.

Increment 18 verified: one complete GLM contribution, two new exact Rust tests
PASS on stable frozen inputs, 28 native encoding assertions PASS. Four recipe
inventories corrected; no established product defect, product change or new
installation attributed to this slice. Full suites/protected guarantees stay open.

### Increment 19 — menu comparison with GLM

[Slice, criteria and evidence](paea/INCREMENT-19.md): expose the existing shared
comparator through native API/PAEA page, both domains, value-free report and exact
export. GLM proposes validation/rendering; Codex integrates/verifies existing and
new journeys and delivers locally. Protected authority remains unchanged.


Increment 19 installed locally as **dev95**: menu comparison, shared API/CLI,
report without values. Two schema/seven HTTP tests PASS; 1,310 browser assertions,
12 configurations × three views, 62 actual native refusals/recovery; 34 real live
delivery checks PASS. GLM returned HTTP 504 with no usable contribution; Codex
implemented and verified the connection. [Slice result and retrospective](paea/INCREMENT-19.md)
retain build refusals, qualified normal-candidate reuse, hashes and limits.
No commit/push. IA3/AC-02/human authorities, E2/E3 and protected acceptance remain open.


Next decision: the user authorizes committing increment 19, then continuing
directly with Codex, **without GLM**. Preserve protected prerequisites, rights
and budgets; select the next slice from the plan's actual state.


### Increment 20 — ready-to-download examples

[Direct slice without GLM](paea/INCREMENT-20.md): eight versioned fictional files,
help in all three views and actual native checks after completed downloads.
No automatic selection/submission; protected assurance unchanged.


Increment20 implemented without GLM and installed as **dev96**: eight examples
from all three help panels, 1,743 native assertions, 36 journeys/120 downloads,
and 40 live checks PASS.
Increment19 commit `f41003488`; no increment20 commit/push. Supported one-off
qualified normal-candidate activation preserves source configuration; concurrent
reviewer compilation (`Capture::new`, dev97 attempt) remains open. [Results and
resumption](paea/INCREMENT-20.md). No protected assurance added.


The user then requests committing installed increment20. Isolated Git recording,
without push or another delivery; dev96 results/limits remain in the
[increment20 record](paea/INCREMENT-20.md).


### Increment21 — exact action inspection

[Direct slice](paea/INCREMENT-21.md): byte-bound shared report, CLI/API and readable
terms/context/sources from the menu. No approval/effect; preserve canonical
contracts and qualify all four views.


Increment21 installed as **dev101**: **Inspect an action** in the PAEA menu, common
CLI/API report, exact terms/context/sources/revisions. Nine targeted tests,2,265
native assertions (four views),48 example journeys/144 downloads,87 native refusals
and46 live checks/20 assets PASS. [Slice result and retrospective](paea/INCREMENT-21.md)
retain initial freshness refusal, dev100/dev101 profiles/hashes and review scope.
No GLM, commit or push. IA3/AC02/humans, E2/E3 and0/17 protected invariants stay open.


User requests commit21 then next. Commit `72e0e32dc`,40 files,108 foreign staged
entries preserved,no push. [Increment22](paea/INCREMENT-22.md) connects supplied approval
binding to exact action/policy data,without protected activation.


Increment22 installed as **dev107**: **Compare a supplied approval** menu, common
CLI/API two-file-bound report,explicit match/mismatch/invalid refusal.11namedtests,
10generationchecks,3,086nativeassertions,60journeys/192downloads,113refusals and
52livechecks/23assets PASS. Primary1.28:1transitioncontrast reproduced and fixed
on the shared five-view component;36observations,min9.096:1. [Results/retrospective](paea/INCREMENT-22.md)
retain failures,recovery,profiles/hashes. No GLM/commit22/push; IA3/AC02/humans,
E2/E3 and0/17remainopen.


### Increment23 — laboratory campaigns in the menu

Commit22 `51a3ac828`,42files,108foreign staged paths preserved, no push.
[Slice23](paea/INCREMENT-23.md): explicit reproduction command and campaign
report reader from the menu; eight scenarios and preserved counterexample.
No web execution or protected activation is attributed to this slice.


Increment23 installed **dev109**: Laboratory campaigns menu,explicit command
and local report reader. Eight real tests,506nativeassertions,12downloads and
56livechecks/25assets PASS. Exact qualified normal candidate activated once,
source configuration preserved; concurrent engine compilation remains open.
[RETEX/evidence](paea/INCREMENT-23.md). No commit23/push; protected PA-02/03,
browser campaign execution,E2/E3 and0/17invariants stay open.


### Increment 24 — revocation/commit contention

Commit23 `97933d8b7`, 27 files, 108 foreign staged entries preserved.
[Qualification 24](paea/INCREMENT-24.md): extend T05/T17 to real overlapping
processes in both fixtures, contention refusal and explicit recovery. Criteria
and limits retained; no new protection or server installation claimed.


Qualification24 verified: four real SQLite/Git contentions with controls, four
expected early-release assertion failures and public eight-case campaign
PASS/VERIFIED. Pinned dev109 protection sources match its effective profile;
dev112 observed separately, without intervention by this slice. Recipe stream
defects corrected/originals retained, derived inventories completed; unchanged
browser not replayed. [Review/slice RETEX24](paea/INCREMENT-24.md). Protected
0/17 invariants, IA3/AC-02, humans, E2/E3 and final programme remain open.


### Increment25 — original laboratory evidence integrity

Commit24 `079635b29`,14 files,108 foreign staged paths preserved.
[Slice25](paea/INCREMENT-25.md): reproduced PAEA-EVIDENCE-25 on a copy (deleted
oracle, old export still possible); v2 call index and explicitly pinned inventory
with original-file verification. Keep report v1, no inferred authenticity or new
protection. Results pending recipe; programme remains open.


Slice25 verified: eight effect tests, eleven verifier regressions/68CLI checks,
1020observation and10recipe files bound,197v2calls. Final recipes and24contention
guard PASS/VERIFIED. Missing oracle refuses export; publication uncertainty is
UNKNOWN with expected pin and verified recovery. Forged-new-pin counterexample
retained; simulated authority/0of17protected, IA3/AC-02/humans/E2/E3/final RETEX
remain open. [Review/limits25](paea/INCREMENT-25.md). Checkout tooling, report v1
and server unchanged; no build/install/commit/push.


### Increment26 — native current-authority control core

PA-02 resumed directly by Codex without GLM. [Increment26](paea/INCREMENT-26.md):
retained connection to existing signed authority, exact current checks held
through real configuration/witness storage, revocation refusals and UNKNOWN
after possible effect. Library core only; no service activation or inferred
human identity. Protected transport/bootstrap, PA-03, E2/E3 and 0/17 global
invariants remain open.


Increment26 verified:41named tests, recipe PASS/VERIFIED,17effective local sources
matched for the normal build and four crates without test-fixtures; Rustdoc PASS.
Revocation ordering, UNKNOWN after effect, both causes and exact-event recovery
observed. [Review/RETEX26](paea/INCREMENT-26.md). No normal CLI consumer, hence no
installed service from this core; protected transport/bootstrap and local personal
authentication remain unconnected. PA-02 partial;0/17/E2/E3/final RETEX open.


### Increment27 — pinned startup and native preflight

Commit26 `cbd6fecd7`,15files,108foreign staged paths preserved.
[Increment27](paea/INCREMENT-27.md): closed v2, independent checkpoint and real
current-enrollment verification through the normal CLI, without starting a
service. 42 named tests and Rustdoc verified; dev.123 installed, parallel dev.124 observed,
normal profiles bound to served SHA, both CLI replays and 56 browser checks pass; protected human
identity/transport, PA-03, E2/E3 and0/17global invariants remain open.


### Increment28 — personal-secret knowledge

[Increment28](paea/INCREMENT-28.md): exact-record/context Argon2id v19 contract,
explicit parameters, guarded workspace and opaque typed knowledge proof.
Verification/integration in progress; no account/session/transport created. Next:
one-time bootstrap storage, actual service/authentication and UX, PA-03/04… .
Knowing a secret never implies consent or business rights. Continuous autonomous
technical work authorized; actual human enrollment and E3 roles remain independent.


### Increment29 — one-time consumption and current account

[Increment29](paea/INCREMENT-29.md): retained signed-journal atomic batch,
indivisible bootstrap revocation/account creation, exact replayable proposal,
current authentication held through usage and outcome preserved after errors.
Recipe/review/delivery in progress. Fixtures are not actual human enrollment;
service/transport, Admin/UX settings, separate account and confinement follow.


### Increment30 — exact personal recovery codec

[Increment30](paea/INCREMENT-30.md): canonical private proposal export/restore,
independent-root/exact-shape checking, signature preview without current state/IO,
and retained base64 causes. Initial3then strengthened3tests pass;39test recipe,
review/delivery pending.29has35verified tests but28/29delivery remains open after
changing foreign effective inputs. Next coordinator must protect durable intent/
checkpoint and fsync/recovery before service exposure. Human/confinement/E2/E3 remain open.


30 qualification:39named tests/7steps,6frozen CLI harness tests and56live readonly
assertions plus missing-script refusal PASS; normal495-source dev128installed and
Version FR/EN verified. [Details](paea/INCREMENT-30.md).28/29historical limits retained.
31private files is next, followed by protected intent/checkpoint coordinator.
PA-02partial;0/17global invariants; human/confinement/E2/E3/PA13remain open.


31private-documents implementation: shared descriptor IO, publication phase and
retained release errors, exact CAS/private files; old Store lock binding also
reviewed. [State31](paea/INCREMENT-31.md).43test qualification pending; not an active service.


31qualified/installed dev129:43named tests/5steps,6CLI harness tests,56readonly UI
assertions+fault refusal, Rustdoc and497-source normal profile verified. Shared
lock substitution regression closed locally; [details](paea/INCREMENT-31.md).
Continue [32](paea/INCREMENT-32.md), still planned; human/service/confinement/E2/E3open.


[32](paea/INCREMENT-32.md) implementation: protected exact intent before consumption,
checkpoint after ack, paired cancellation, current held auth after file release.
First5tests PASS; expanded31test recipe, review and delivery pending. No endpoint
or human enrollment claimed. PA-02partial/global0of17 unchanged.


32library qualification PASS:32tests, Rustdoc, FR/ENdiagrams, normal499-source
profile. Installation remains OPEN after launcher refused concurrent candidate
inventory change; dev129last verified. [Details32](paea/INCREMENT-32.md). Continue
[33](paea/INCREMENT-33.md)independently, retain pending delivery/PA-02/E2/E3/global0of17.


[33](paea/INCREMENT-33.md) implemented typed composition of held personal account
and business authority, exact principal/install binding and nested effect outcomes.
Private Debug counterexample fixed;14tests+1double-held-authority targeted PASS.
38test recipe/current normal delivery pending. No actual service/human/OS confinement.


[34](paea/INCREMENT-34.md) implemented pinned composite declaration/shared physical
loader and actual two-source opening, no account init/listener.16bootstrap+4startup
scenarios verified; aggregate59test recipe/review/delivery pending. Partial cleanup
errors preserved, actual production routing/service UID/confinement/E2/E3stillopen.


34internal APIs verified/included in installed dev136:59named tests/10steps, current
Rustdoc,515-source nofixture profile,6CLI harness tests,56live readonly checks+fault
guard. [Details](paea/INCREMENT-34.md).32/33pending delivery now additive evidence.
Continue35protected signer/lifecycle; no personal listener/actual human/OSconfinement,
PA-02partial/global0of17/E2/E3/PA13remainopen.


35started: protected signing-key loading, before lifecycle; global acceptance unchanged. [Plan35](paea/INCREMENT-35.md).


35protected signer verified/installed dev137:12namedtests,Rustdoc,516-source normal
profile,dalekzeroize,6CLI harness cases,56UIassertions+failureguard. [Proof35](paea/INCREMENT-35.md). Continue36owned lifecycle; no global acceptance/human/service.


36owned personal lifecycle started; [plan36](paea/INCREMENT-36.md), global acceptance unchanged.


36owned lifecycle verified/installed v0.1.1-dev.142:43namedtests,normal
519-input profile,6CLIharness,56UI+faultguard,currentconsolidatedFR/ENnotes.
[Proof36](paea/INCREMENT-36.md). Continue37native personal lifecycle; transport/UX/
OSseparation/passwordrotation/E2/E3/PA13/global0of17remainopen.


37native personal lifecycle started; [plan37](paea/INCREMENT-37.md), global acceptance unchanged.


37aoptionguard delivered/behaviorchecked147,7frozen145+named147guard+56UI,privateRustdoc. Currentglobal13sourcequalification stillopen; [details37](paea/INCREMENT-37.md). Continue37bpersonalCLI; global0of17/E2/E3/PA13open.


37b personal CLI delivered/verified154:32tests/7stable steps,7native default-binary journeys and56UI; [proof/limits37](paea/INCREMENT-37.md). Next38: protected initial provisioning then transport/UX. PA02partial,global0of17/E2/E3/PA13open.


38started: initial material before protected publication/recovery; [plan38](paea/INCREMENT-38.md). No implicit grant/service/human.


38initialmaterial verified/delivered156:27tests/4steps,1native generatedmaterialjourney,56UI; [proof/limits38](paea/INCREMENT-38.md). Next39guarded private read then publication/recovery. No installed publisher/service/human/confinement;global0of17/E2/E3/PA13open.


39guarded private read started before seedpublication; [plan39](paea/INCREMENT-39.md). Separate from existing nonsecretread.


39private reader/writeprecheck delivered157:51composedchecks(21reused+30fresh),7normalCLI,56UI; [proof/limits39](paea/INCREMENT-39.md). OriginalmanifestFAILkept.40publication/recovery next;global0of17/E2/E3/PA13open.


40protected operatorpackage started before41servicepublication; [plan40](paea/INCREMENT-40.md). Independentpin,noheadadoption.


41explicitpackage durability started;40deliveryOPENafter normalrefusals,157lastqualified. [Plan41](paea/INCREMENT-41.md).42servicepublicationnext,global0of17unchanged.


42service-onlyartifact started:personalkey/signedevents,operatorsecretsexcluded;[plan42](paea/INCREMENT-42.md).43publication/current40-41deliveryopen.


42: service-only handoff and local **v0.1.1-dev.165** delivery verified (18 library,7 CLI,56 browser plus script refusal).43file publication next, global0of17unchanged. [Plan42](paea/INCREMENT-42.md).


43private service-file publication started. [Plan43](paea/INCREMENT-43.md).


44publicCLIadapterfor43publisher started, shared delivery pending. [Plan44](paea/INCREMENT-44.md).


43/44locallydelivered **dev168**:50librarytests (6supplement),8CLIjourneys/59commands,7olderpersonaljourneys,56UIchecks plus scriptrefusal; actualprofile/ELF/assets/Versionverified. No human/separateUID/listener/E2/E3/globalinvariantacceptance. [Plan44](paea/INCREMENT-44.md).


45actualLinuxdiagnostic started;44historical168delivery, currentlock/cataloguerequalification afterconcurrentdrift. [Plan45](paea/INCREMENT-45.md).

43/44 requalification completed on dev174:50fresh library cases,8CLI/59commands,7regressions,56UI+guard,561-input/no-fixture profile and common actual ELF verified. Earlier evidence and concurrent refusals retained. [Details44](paea/INCREMENT-44.md).45Linux observation verified; uidmap installation requires local administrator authentication. No global invariant or OS separation acquired.

46 private request and personal service core started independently of missing Linux helpers; [plan46](paea/INCREMENT-46.md). No endpoint/separateUID/human/global acceptance gained.

46library19cases qualified;47public binding excluding secrets, response/after-effect cause retention24Rust cases and Rustdoc PASS; public delivery still to connect. [Plan47](paea/INCREMENT-47.md).

48bound consumer verified:29Rust+3schema/5steps, canonical/request/pins/history/Change checks, AfterUse counterexample corrected. [Plan48](paea/INCREMENT-48.md).49native operator adapter/grouped local delivery next; serviceUID/transport/human/global remain open.

46…49 locally delivered/verified **dev175**:39Rust+3schema,5new native journeys/56commands,8installation,7personal,56UI+guard,571-input/no-fixture profile and common actual ELF. [Evidence49](paea/INCREMENT-49.md).50attributed channel/client next; UIDhelpers/admin/human/E2/E3/PA13 and0of17 remain open.

50replan: Rustix getter PID0representation risk,4consumers identified;3old175gates refuse but report errno0, no effect. Raw-safe getter/diagnostics implemented, qualification/delivery pending. [Plan50](paea/INCREMENT-50.md). Resume channel after this shared prerequisite; globals unchanged.

Increment 50 delivered and verified in **dev180**: 4 engine + 6 CLI tests, strict Rustdoc, 5 personal journeys / 56 commands, 3 valid witnesses + 3 invisible-PID refusals, 56 UI checks and failed-script guard; 581-input normal profile and same actual ELF. [Evidence and limits](paea/INCREMENT-50.md). Resume attributed channel; UID, human, E2/E3, PA-13 and 0/17 globals remain open.

51: isolated Linux boundary verified as a library, nine Rust cases + two fixture journeys, strict Rustdoc and ABI/ownership review; no public consumer yet. [Evidence51](paea/INCREMENT-51.md). Resume52 admission/integration/grouped delivery; dev180 remains this task's latest qualified delivery, UID/human/globals open.

51–52foundations delivered/verified in **dev185**:41Rust+3schema,2fixturejourneys,5operator/56commands,3gates,56UI+guard;587-inputprofile/57owninputs, newlibrarynofixture andsameactualELF. [Evidence52](paea/INCREMENT-52.md). Protectedrole/exactpacket/UIDprecondition linkedaslibrary; completeadmission/daemon/realdistinctUID/human/E2/E3/PA13/global0of17 remainopen.53reservedUIDhypothesisnext,thenresumeintegration.

Increment 53 ongoing: [valid UID representation](paea/INCREMENT-53.md). Contract tests qualified; driver, native flow and delivery still require verification. Channel admission and global acceptance remain open.

Increment53 verified locally dev189:101targetedRust/8Go parents/3schema,9installation/5operator/7personal+3guards,3gates,56UI+guard; [evidence and limits](paea/INCREMENT-53.md). PA-02/global0of17 remain open; protected public preparation then channel next.

[Increment54](paea/INCREMENT-54.md) installed/verified dev192: private canonical request preflight without material generation;8Rust+3schema,5new journeys/84commands,9installation/5operator/7personal+3guards,2fixture-role guards,56UI+guard and3peer gates. PA-02/global0of17 remain open; public operator package/service export next.

[Increment55](paea/INCREMENT-55.md) installed/verified dev200:pinned operator package read, deterministic private service export/reconciliation;6journeys/154commands on200,56UI+guard,3gates. Historical199 regressions reuse unchanged PAEA/CLI bytes without all-200 ELF aggregation. PA-02/global0of17 open;protected initial creation then channel remain next.

[Increment56](paea/INCREMENT-56.md) started:immutable private anchor before public generation/publication; distinguish protected locator from independent pin without circular evidence. No new verified global criterion.

[Increment56](paea/INCREMENT-56.md) installed/verified dev201:immutable private anchor/CLI,5journeys/67commands,regressions6export/5preflight/9install/5call/7personal+3guards,56UI+guard and3gates.37targetedRust/16Python schema methods, no global percentage. Next57:durable intent before generation then pinned package/recovery. PA-02/global0of17 open.

[Increment57](paea/INCREMENT-57.md) started:durable intent before generation, anchor before package, existing private reservations and reconciliation without regeneration. Request pin binds exact input bytes; no service/human/current mandate created.


[Increment57](paea/INCREMENT-57.md) installed/verified dev205: durable intent before generation, anchor before package and reconciliation without regeneration;46targeted Rust/6schema methods/12swaps,6new journeys/113commands,regressions and56UI+guard on the same default ELF631/64. Orphan remediation and service-domain handoff next; PA-02/global0of17/E2/E3/PA13/RETEX open.


[Increment58](paea/INCREMENT-58.md) underway: kernel packet bound to retained UID and expected namespace objects before decode/dispatch, Reply and cleanup preserved;49targeted Rust/3schema, local delivery pending. Independent domain approval, daemon and actual separate UIDs open.


[Increment58](paea/INCREMENT-58.md) locally verified dev206 then current207 bound to identical65PAEA inputs:51distinct Rust/3schema; native call/personal206 retained separately, fresh207 UI56+guard and peer; default profile636/65. Only foreign runtime-health asset differs. No new daemon or actual distinct-UID business success; least-privilege launch and ptrace permission next. PA-02/global0of17/E2/E3/PA13/RETEX open.


[Increment59](paea/INCREMENT-59.md) started: bind kernel packet subject to independently spawned Child, not request PID or connection creator; retain ALL58checks. No daemon/ptrace permission/actual separate UID acquired.


[Increment59](paea/INCREMENT-59.md) locally verified dev208: conditional Child anchor retains ALL58checks;27Rust, default637/65,5call/7personal/3pairs/56UI+guards on same ELF. Native atomic creation next; launcher/daemon/actual UIDs/E2/E3/PA13/RETEX/global0of17 open.


[Increment60](paea/INCREMENT-60.md) atomic library verified:35distinct Rust,6source/3Rustdocpages, traced CLONE_PIDFD. Delivery pending: three profile refusals for concurrently changed shared inputs;208 retained. Resume after producer stability, no rebuild loop. Consumer/daemon/UID/human/E2/E3/RETEX/global0of17 open.


[Increment60](paea/INCREMENT-60.md) atomic library delivered/verified v0.1.1-dev.209:35distinct Rust,6sources/3Rustdocpages, traced CLONE_PIDFD, default645/67 and existing paths on same ELF. Consumer/daemon/actual UIDs/human/E2/E3/PA13/RETEX/global0of17 open.


[Increment61](paea/INCREMENT-61.md) single-packet response library verified:27distinct Rust(9new),4sources/4Rustdocpages; actualEPIPE/EMSGSIZE/EAGAIN and recovery. Delivery pending25concurrently changed Web inputs;209 retained. Resume qualified common candidate/68PAEA inputs without rebuild loop. Daemon/UID/human/E2/E3/PA13/RETEX/global0of17 open.


[Increment62](paea/INCREMENT-62.md) locally qualifies namespace-owner permission:2Rust tests/6phases, refusal outside childuserns then owned-child access with parentCapEff0. Same externalUID, no actual separation. Native fixture error retained, frozen proof and altered-copy refusal. Tests only, no62runtime version;61awaits stable candidate. PA02/global0of17/E2/E3/PA13/RETEX open.


[Increment61](paea/INCREMENT-61.md) single-packet response delivered/verified v0.1.1-dev.211:27distinct Rust(9new),4sources/4Rustdocpages, actualEPIPE/isolatedSIGPIPE, default650/68, existing native flows and56UI+guard on same ELF. Composition/daemon/actualUID/human/E2/E3/PA13/RETEX/global0of17 open.

[Increment63](paea/INCREMENT-63.md) one controlled exchange delivered/verified **v0.1.1-dev.215**:45distinct Rust/7new,3sources/4pages Rustdoc, default profile653/69, existing native CLI and56UI+guard on same ELF. Library qualification; daemon/actualUID/human/confinement/E2/E3/PA13/RETEX/global0of17open.

[Increment64](paea/INCREMENT-64.md) nativeLinux/CLI/API/page delivered/verified **v0.1.1-dev.216**:10Rust,111component/60installed/56Version+guards,37native/52CLI/16HTTP, default665/81 andsameELF. Metadataobservation only; UID/human/confinement/daemon/E2/E3/PA13/RETEX/global0of17open.

PA-03 resumption: [effect service integration decision](paea/PA03-SERVICE-INTEGRATION.md), inspected sources and conditional sequence. Analysis only; no additional protected effect or delivery established.

[Increment65](../../../plans/actions-confiance/paea/INCREMENT-65.md): separate exact PAEA permission, library verified and compiled in **v0.1.1-dev.218**, 8PAEA+3Trust tests+3compile refusals, strict Rustdoc and56Version/UI checks+guard. Normal671-input profile, same actual ELF. No protected effect service activated; PA02/03,0of17,E2/E3/PA13/RETEX remain open.

[Increment66](../../../plans/actions-confiance/paea/INCREMENT-66.md): separate fictitious settlement reference kernel installed and verified in **v0.1.1-dev.220**;12Rust tests, strict Rustdoc,56Version/UI assertions plus script guard. Normal profile677inputs, actual matching ELF. Enabled separation refuses pending attributed admission; no protected daemon activated. PA02/03 partial,0/17/E2/E3/PA13/RETEX remain open.

[Increment67](../../../plans/actions-confiance/paea/INCREMENT-67.md) verified/installed **v0.1.1-dev.221**: four inconsistent receipt metadata cases reproduced then refused;13Rust tests,strictRustdoc,56Version/UI assertions plus script guard,normal677inputs and matchingELF. No receipt authenticity or global protection inferred;PA02/03/0of17/E2/E3/PA13/RETEX remain open.

[Increment68](../../../plans/actions-confiance/paea/INCREMENT-68.md): six real Git experiments qualify prepared-lock/direct-ref/commit-abort primitives after a no-deref counterexample. Tests only; no new native service or delivery number,221 retained. Durable intent, native runner and reconciliation remain required;PA03/global stay open.

[Increment69](../../../plans/actions-confiance/paea/INCREMENT-69.md): native Git protocol library delivered/verified in **v0.1.1-dev.222**;8native cases/strictRustdoc,56Version/UI assertions plus script guard,normal678inputs and matchingliveELF. Persistent Interrupted cancellation replay reproduced then corrected; real publication without ACK remains uncertain. No Git effect service or admission activated;PA02/03/0of17/E2/E3/PA13/RETEX open.

### Increment70 — durable Git intents

Six native journal tests and nine real Git protocol cases PASS, plus strict
Rustdoc; core04/protocol05 evidence retained under `paea/evidence/20261028-git-intent70-tjriu_vr`.
Explicit journal provisioning, business-key uniqueness across transport/content
aliases, restart, real COMMIT uncertainty, two-connection race and corruption
refusal verified. A composed fixture observes committed SQL before Git COMMIT,
then keeps the original pending intent after lost ACK and reopen. This is not
protected admission, service activation or terminal effect reconciliation. Source-managed version223 installed via exact prepared candidate; actual ELF
7143f8c23550d1e00d4ec89f7fb7bec40ed7a38ca3af209ad2dee704d388592f, PID3128318
verified. Normal679 inputs,479 shared with tests04 (notes-only difference), no
fixture profile. Version08:56 assertions plus failed-script guard PASS/VERIFIED,
25 protected assets/compiled notes, FR light/EN dark, commit/dirty/focus checked.
No installed public consumer invokes the new journal or generic Git flow.
PA02/03 remain partial; 0/17 protected invariants, E2/E3 and final RETEX remain open.

### Increment71 — latest native candidate and historical originals

The shared publication reader now checks every earlier native original even for
latest-event replay. Actual223 public counterexample retained: replay accepted an
altered old certificate while Work read refused. New native reader tests05 PASS
(three named tests, six cumulative assertions and strict Rustdoc; VERIFIED), with
real two-publication inert fixture. Normal CLI08 PASS/VERIFIED: old/latest certificates refuse latest replay,
restore/recover, and Work/event/worker/provider state stays unchanged. Version225
installed via exact prepared ELF bba13470359ddfb0fecc17b958935ccab77dbc74ec40253c853288acab1aa8f5;
680 normal inputs/480 common with tests05, notes-only delta, no fixture features.
Version10:56 live assertions/25 protected assets/compiled notes, FR/EN and
script-failure guard PASS/VERIFIED. [Increment71](../../../plans/actions-confiance/paea/INCREMENT-71.md) · [Continuity71](../../../plans/actions-confiance/paea/CONTINUITE-71.md). No PAEA service authority, repository-id association, UID
isolation or global closure inferred; PA02/03partial,0/17,PA13/RETEX open.

### Increment72 — private Git resource binding

An explicitly provisioned immutable declaration binds retained service/project/
private bare directory identities, the operator's journal pin and actual managed
repository request digest. Resource ID is service-derived, never a proposal path.
Five native tests03 and strict Rustdoc PASS/VERIFIED: NOREPLACE/reopen, current
candidate, wrong context, substitution, symlink and canonical corruption/refusal/
recovery. Version229 installed via exact prepared ELF 1e2384be40e0383520204fa86bad0cb56f578316330b44626e4ca2ae4a93285e;681 normal inputs/481 common with tests03, notes-only delta, no fixture features. Version06:56 live assertions and script-failure guard PASS/VERIFIED,25 protected assets/compiled FR/EN notes. [Increment72](../../../plans/actions-confiance/paea/INCREMENT-72.md) · [Continuity72](../../../plans/actions-confiance/paea/CONTINUITE-72.md). No transfer, current mandate,
OLD/direct-ref condition, Git COMMIT, protected daemon or final reconciliation
inferred. PA02/03partial,0/17 and PA13/final RETEX remain open.

### Increment73 — actual SQLite writer and Git format

Three unit02 tests and six normal-library integration03 tests PASS/VERIFIED,
strict Rustdoc included. The internal reader requires actual main Write state,
rejects deferred/read/no writer, retains the caller transaction and checks the
same native candidate/binding without nested BEGIN. Private bare SHA1/SHA256
compatibility is observed before transfer or intent; mismatch refuses.
Version230 installed, exact prepared ELF 0b8cf6947596da380ed2f9ee862ed80597f949b2d3607d2147a787fd0d051287;681 normal inputs/481 common with integration03, notes-only delta, no fixtures. Version06:56 live assertions and script-failure guard PASS/VERIFIED,25 compiled assets/notes FR/EN. Internal writer helpers still have no normal public effect consumer. [Increment73](../../../plans/actions-confiance/paea/INCREMENT-73.md) · [Continuity73](../../../plans/actions-confiance/paea/CONTINUITE-73.md). No object transfer, current authority,
ref commitment or terminal reconciliation inferred; PA02/03partial,0/17,
PA13/final RETEX remain open.


### Increment74 — real pack protocol without ref movement

Six real-Git assertions PASS/VERIFIED: native71 candidate read by normal CLI230,
disk export/import and connectivity, repeat without ref movement, incompatible
format, truncated/corrupt packs and recovery, missing OID refusal. Partial writes
after failure remain inventoried. Protocol experiment only; native transfer,
current service authority and commitment remain to implement. No product bytes
changed, so no new build or installation;230 evidence stays separate.
[Increment74](../../../plans/actions-confiance/paea/INCREMENT-74.md) ·
[Continuity74](../../../plans/actions-confiance/paea/CONTINUITE-74.md).
PA02/03 partial,0/17,PA13/final RETEX open.


### Increment75 — native disk pack primitive delivered

Six real-Git integration cases and six native binding/format regression cases
PASS/VERIFIED, strict Rustdoc included. Fixed export/import streams to files,
refuses aliasing/overwrite, retains partial errors and typed process recovery.
Saved Git command/deadline setup shared byte-for-byte. Version231 installed with
exact prepared ELF `ce4750b00da3e8ff7723b51f64b944bf3def76d0f8961eef5415c2e9bee7c387`;682 normal inputs/482 common with
core02, no differences/features fixtures. Version06:56 live assertions and script
guard PASS/VERIFIED, protected assets/compiled notes FR/EN, screenshots inspected.
[Increment75](../../../plans/actions-confiance/paea/INCREMENT-75.md) ·
[Continuity75](../../../plans/actions-confiance/paea/CONTINUITE-75.md).
Native protected-service transfer/current authority/ref commitment remain open;
PA02/03 partial,0/17,PA13/final RETEX open.


### Increment76 — pack sync failure channels and primary cause retained

Actual normal-library counterexample FAIL/VERIFIED before repair; one synthetic
unit composition, two existing wrapper regressions and seven native integration
cases + strict Rustdoc PASS/VERIFIED.
Both actual kernel sync errors retain stdout/stderr identity through Box/IO;
primary process ownership and recovery survive. Version232 installed, exact ELF
`7cb94d74a30343d472f20e94f5786f9cfefff4b02415fb2afaf3f8733a0f52d7`;682 normal inputs/482 common with tests, no differences,
no fixtures. Version06:56 live assertions + script guard PASS/VERIFIED, compiled
assets/notes FR/EN, screenshots inspected. [Increment76](../../../plans/actions-confiance/paea/INCREMENT-76.md) · [Continuity76](../../../plans/actions-confiance/paea/CONTINUITE-76.md).
Protected-service transfer, current authority/ref commit, PA02/03 partial,0/17,
PA13/final RETEX remain open.
