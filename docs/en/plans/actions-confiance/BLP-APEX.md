# Bell-LaPadula — PAEA technical subplan

Current operational state: [Bell-LaPadula in Swarm](../../BELL-LAPADULA.md). Dated sections below retain historical evidence and its limits.

[Français: detailed technical plan and historical handoff](../../../plans/actions-confiance/BLP-APEX.md).

## Authority and integration

User decisions: confidentiality across Project and Business Process journeys;
explicit migration of existing unclassified data; authenticated human approval
for exact declassification; only the filesystem/network subset necessary for BLP,
leaving the remainder of AC-02 deferred. Biba integrity remains separate.

The [PAEA plan](PAEA-APEX.md#bell-lapadula-integration--october-31-2026) maps BLP-01–07
to existing packages PA-00–13, invariants I01–I17 and tests T01–T22. It owns the
assurance criteria and register. BLP tracking owns technical implementation,
evidence and recovery, without a second assurance program or changed denominators.

| Subpackage | Deliverable | Dependencies | Parent PAEA packages |
|---|---|---|---|
| BLP-01 | Source/reader/output inventory, trust boundaries and threat model | — | PA-00/01 |
| BLP-02 | Dominance/join monitor, fixed domains, safe errors and independence from Biba | BLP-01 | PA-01/02/06 |
| BLP-03 | Revisioned persistent policy, explicit migration and Admin/API/CLI parity | BLP-02 | PA-02/08 |
| BLP-04 | Protected attribution, context/memory/output propagation and replays | BLP-03 | PARTIAL — private v3 prompt provenance and API/CLI object control; source classification, principal and inheritance open |
| BLP-05 | Actual filesystem/network/provider/descendant mediation, no unconfined fallback | BLP-04 | PA-04/05 |
| BLP-06 | Established human identity, approval of exact bytes/recipients, new copy | BLP-03–05 | PA-02/08/09 |
| BLP-07 | Retained recipes, candidate review, local delivery, served assets and RETEX | BLP-01–06 | IN PROGRESS (targeted increments verified; v0.1.1-dev.299 installed; transverse RETEX written [RETEX-BLP-20261109](../../../plans/actions-confiance/paea/RETEX-BLP-20261109.md); public aggregated recipe `blp-aggregate-public` PASS 6/6 + VERIFIED on single candidate 9 Nov; cargo/helper recipes outside the aggregate and independent acceptance open) |

## Current technical state

BLP-03 explicit migration is delivered (9 November 2026, Admin form BLP-03a
below, recipe `blp-object-attribute-public`); the migration API/CLI surface is
not added. BLP-04 remains partial: native v3 provenance, public object control
and the authenticated recorded-source diagnostic are verified and installed
in296. Producer source identity, original send-principal binding and output
inheritance remain open. BLP-07 retains global recipe/review/RETEX
requirements. See the verified-result section below for exact image/evidence.

## Required reassessment of previous PAEA work

Actions already performed, delivered or marked complete must be assessed against
all seven BLP-01–07 subpackages. The [PAEA reassessment requirement](PAEA-APEX.md#required-reassessment-of-work-already-performed)
defines per-action tracking: justified applicability, actual producers/consumers,
candidate/profile, reusable evidence, gaps, owner and recovery/closure condition.
Keep this record in existing tracking.

Preserve historical results without assigning unestablished confidentiality
guarantees. Reopen affected criteria and repeat affected checks while reusing
evidence whose inputs and scope remain valid. BLP-07 cannot close without this
reassessment and treatment of applicable gaps.

Updated state: [applicability reassessment completed](paea/BLP-REASSESSMENT-20261031.md),
covering 115 actions and 805 decisions in the [register](paea/BLP-REASSESSMENT-20261031-ANNEX.md).
Eight findings BLP-RET-01–08 have owners and recovery conditions; applicable
criteria are reopened or remain open without erasing historical results.
The initial 150 review inputs are archived with hashes; concurrently added
delivery/UI evidence is noted separately in the report. This review reinstalls
nothing and does not rewrite the separately recorded foundation delivery. Transversal BLP-07
cannot close before applicable integration/checks/findings, including
BLP-01/04/05/06, even with delivered policy administration and its completed
retrospective. Reception by the recovery session, corrections and end-to-end
verification remain open.

## BLP-01 — static flow inventory, October 31

[Bilingual contracts/threats](../../BLP-FLOWS.md) and the
[24 source-bound flows](../../../architecture/blp-flow-inventory.json) bind
43 anchors in 41 source files: actual assembly, CLI/HTTP transport, manual probe,
validation, telemetry, history, Git/settlement effects and exports. Each records
existing controls, owner, missing packages and required observable. All 24 actual
runtime BLP criteria remain open.

Permanent blp-flow-inventory recipe: two steps PASS; five checker tests actually
executed (control, mutation/refusal/recovery, anchor, graph and path escape).
This establishes declaration freshness, not disclosure prevention or dynamic
channel completeness. Fresh evidence: verification-blp/20261031-flow-inventory-01/.

Next chosen journey: BF-05 → BF-07 → BF-16, complete HTTP preparation context
attribution, transmission and response inheritance, using a local fixture
receiver. No implicit public classification, authorizing client label or free
string may replace the protected producer. Existing Biba, budget and authority
guards remain cumulative. BLP-01 stays partial until dynamic SDK/cache/tool/
proxy/descendant access is inventoried/qualified; BLP-RET-01/02/03 remain open.
This tracking/tooling increment changes no product behavior: no installation
or release-note update is required.

## Contract and evidence

Classification consists of an ordered level and compartments. Dominance requires
an equal/higher level and a superset of compartments; joining exposed inputs
takes the maximum level and the union of compartments. Unknown classifications
block new protected uses. Execution domains cannot be lowered after exposure;
summaries and LLM filtering grant no declassification. Recipients are bound to
the effective service/account/endpoint, not just a model name.

Policies require persistence, shared validation and Admin/API/CLI readback.
Each increment retains function documentation and source-bound review of policy
choices and errors. Independent acceptance remains distinct from local tests.
Every fault case follows a valid control; receiving oracles check non-disclosure.
No live mission database is a fixture and no project data belongs under `/tmp`.

## Historical snapshot and recovery

Commit `0aa7335e1` saved incomplete work after the user requested development to
stop and provided for a GLM/SKYNET handoff. It contains a data-only monitor,
policy administration, diagnostic operations and an Admin section. It establishes
no transversal protection or installation. Ten monitor tests passed before later
revision-binding edits; the final snapshot was not qualified. The French record
preserves the failed engine compilation and the HTTP routing defect found before
commit. Later changes by another session require their own evidence; these dated
observations must not be silently promoted into current source diagnoses.

The selected GLM/SKYNET recovery session must receive both plans and reference
PAEA requirements/tests in each handoff. Repository edits establish neither
reception by an already running session nor updated embedded engine instructions.
This documentary integration resumes no stopped development and launches no
provider, full AC-02 program, build, installation, commit or push. PAEA assurance
states and historical results remain unchanged by the mapping itself.

Increment review: documentation only; no runtime policy, schema, function or
error path changes. Added scenarios are requirements, not test results. Release
notes and installation are unchanged because this mapping delivers no product
behavior.

## Verification, local delivery and real-flow qualification — October 31 (Claude session)

The French record above remains the detailed canonical log. Summary:

- Fixed the two open defects from the WIP snapshot (HTTP routing in
  `service/trust.rs`, E0308 fix verified by compilation) and the integration
  fixture (third identity). Fresh evidence: `swarm-confidentiality` 10/10,
  `confidentiality` 5/5, Biba coexistence 14/14 + 11/11 + 3/3, permanent recipe
  `blp-confidentiality-rust.json` PASS (verify → VERIFIED).
- EN translations (13 keys), CLI help FR/EN, operator and architecture docs
  FR/EN with an honest Mermaid diagram (solid = verified, dotted = labeled
  target), release notes in `version_history.json`.
- Local delivery: disk rechecked (~72 GiB free) before `./swarm.sh build
  --no-open`; candidate **v0.1.1-dev.282** verified by the collector
  (`VERIFIED_NATIVE_CANDIDATE`, default native profile, commit 164915f73,
  sha256 ea072ccc…). `./swarm.sh restart --no-open` installed it on the single
  managed server; running binary identity confirmed (`/proc/<pid>/exe`,
  sha256 match, `server.json` and `swarm.sh status` agree).
- Real HTTP qualification on the installed server: POST `action=blp-show`
  without an I3 token → 403 `authorization_required` (typed business refusal,
  not 404/500); unknown BLP action → 400 `trust-action-unknown`; missing CSRF
  and unauthenticated asset GET → 403. The real root has no `trust-actions.json`
  (protocol unconfigured); positive BLP administration therefore requires the
  trust bootstrap, an operator decision not taken in this increment.
- Real browser qualification (Chrome headless, script and evidence under
  `.swarm-fixtures/`, retained in-repo at
  [verification-blp/20261031-ui-qualification/](../../../plans/actions-confiance/verification-blp/20261031-ui-qualification/)):
  served page + embedded BLP asset, Admin dialog, typed
  FR/EN refusal messages, 4-level no-clearance example, save disabled without
  preview, theme sombre→etat verified by attribute, no horizontal overflow at
  390 px — 9/9 checks, no unexpected console/page/HTTP errors.
- Still open: RETEX; no independent acceptance. BLP-01 partial; BLP-04–06
  open. `runtime_enforced:false` and `runtime_authorized:false` remain the
  runtime truth; no real content classified, no protected execution.

Transverse RETEX now written (9 November 2026):
[paea/RETEX-BLP-20261109.en.md](../../../plans/actions-confiance/paea/RETEX-BLP-20261109.en.md)
(French original `.md`) — method `retex-analyzer`, scope lots 01–06 over the
2–9 November interval: BLP-03a delivery and qualification (recipe run PASS +
verify VERIFIED re-checked), local delivery with the identity chain, served
resources (inventory 74 refs / 25 flows, 0 failures), the BLP-06 contract
transmitted, and a `version_history` traceability gap found and corrected
(+4 lines). BLP-RET-01–08 re-read: all OPEN. BLP-07 remains open for independent
acceptance (the public aggregated global recipe was delivered on 9 Nov, see
"Targeted global recipe" below); BLP-01 partial, BLP-04/05 partial, BLP-06
contract only.
`runtime_enforced:false` remains the runtime truth.

RETEX written: [paea/RETEX-BLP-20261031.en.md](../../../plans/actions-confiance/paea/RETEX-BLP-20261031.en.md)
(French original `.md`). BLP-07 stays open for independent acceptance;
BLP-01 (partial) and BLP-04–06 stay open.

Concurrent collision recorded at closure: a third-party session rebuilt and
installed **v0.1.1-dev.283** (same commit, sha e8961b39…) during
qualification; their `version_history.json` entry was re-inserted on top of
their state and the BLP entry rewritten atomically (both coexist). Server 283
does serve the repaired BLP routing (probe `blp-show` → identical 403
`authorization_required`); its release notes lack the BLP entry (built before
the rewrite).

Resolved: the next build **v0.1.1-dev.284** (third-party session, commit
`3b5191e2c5` = my delivery commit) contains the BLP entry — proof: the
build's `effective-sources.tar.gz` (`.cache/local-delivery/rust-e4t0j_65/`),
one real BLP entry. Probe re-qualified on the active 284 image:
`blp-show` without an I3 token → identical 403 `authorization_required`.

## Next resumption — prompt provenance

BLP-04 starts with actual assembled-byte provenance, without labels or disclosure
authority. See the [record and limits](BLP-PROVENANCE-20261031.md). The lot stays
open: migration, authenticated principal, sink checks and inheritance remain pending.

## Positive BLP administration on a fixture root — November 1 (Claude session)

The [RETEX](../../../plans/actions-confiance/paea/RETEX-BLP-20261031.en.md) next experiment, executed: the
full recorded administration flow through the real UI, independent of any
mission-root bootstrap decision.

- Dedicated fixture root in the project fixture area (never `/tmp`, never the
  mission root), initialized with the installed binary (`swarm init`), trust
  bootstrapped through the CLI operator path (`trust config-apply`, three
  fixture-labeled identities), real `swarm web` serving 127.0.0.1:38499.
- Real Chrome on `/trust-actions.html`: `blp-load` shows "No confidentiality
  policy recorded"; `blp-example` fills the 4-level no-clearance catalog;
  reason typed; `blp-preview` → "Preview validated; no change applied";
  `blp-save` → "Policy recorded and re-read; transversal protection not
  active"; the recorded policy is read back into the form.
- Server readback: `capability.configured:true`, `runtime_enforced:false`,
  `policy_revision:1`, one attributed history entry (actor `admin`, reason
  preserved). **7/7 checks, 0 errors.**
- Evidence retained:
  [verification-blp/20261101-blp-positive-ui/](../../../plans/actions-confiance/verification-blp/20261101-blp-positive-ui/)
  (harness, `checks.json`, screenshot, fixture-root trust state — fixture-only
  labeled data).

Scope and limits: this proves the **recorded administration path**
(persistence, attribution, readback, save-gated-on-preview) on the served
image; it establishes neither real-content classification, nor protected
execution (`runtime_enforced:false` unchanged), nor independent acceptance.
The mission root still has no trust state; the operator bootstrap decision
remains required before any real administration.

1 November resumption: provenance proofs (11 tests, CLI 29, HTTP 34) and historical
284 installation confirmed; [updated record](BLP-PROVENANCE-20261031.md). Current
local version is 290, preserved without reinstalling; those proofs do not qualify
290. BLP-04 remains partial, with no transversal BLP enforcement.

## 1 November resumption — attribution renewal and declassification

Discovered while resuming BLP-04: `blp-object-attribute` refused a second attribution
within one revision but admitted a lower label after policy renewal. Evidence
`20261101-attribution-renewal-01-counterexample` reproduces confidential → public
for the same digest/project in revision 2, allowing the public reader. This affects
the producer and all history consumers, including PAEA previews. Earlier proofs
retain their exercised scope and did not cover this revision transition.

Decision (Codex, authorized scope): preserve the plan and close this bypass before
adding another producer. Compared reader-only effective-label escalation against
producer and stored-history rejection. Selected rejection: do not serve a label
different from its recorded attribution or normalize corrupt history into success.

Criteria: same digest/project, preserve/increase level and preserve/extend all
compartments across revisions; lower/incompatible labels refuse without mutation;
equal/stricter renewal, replay and actual authorized reading pass; already-lowered
history refuses without disclosure or rewriting. A new revision or admin clearance
never substitutes for BLP-06 human consent. Binding remains the existing digest +
project-path contract; this fix establishes neither physical source-file identity,
complete prompt provenance nor protection of copies without lineage.

State: retained counterexample FAIL; correction/checks in progress through
`tools/verification/recipes/blp-attribution-renewal.json`. No new right, budget,
ceiling or operator setting. BLP-04 stays partial; BLP-06 remains unavailable.

Verification journal:

- `20261101-attribution-renewal-01-counterexample`: expected FAIL, lower label
  admitted and public reader allowed by the previous producer.
- `...-02`: 11/12 pass; test helper omitted the safe nested cause behind
  `trust-proof-integrity`. Fixed diagnosis without weakening refusal; fixture
  injection now reuses guarded atomic publication.
- `...-03`: 12/12 pass but recipe FAIL due to two concurrently changed authority
  sources. This is not stable-source evidence.
- `...-04`: no-symbol build finished and raw output has 12/12, but recipe received
  SIGTERM before final receipt (origin unknown): incomplete.
- `20261101-attribution-renewal-05`: **PASS, integrity verified**, 12 tests actually
  executed with stable declared sources, reusing the now-built test candidate.
  This changed prerequisite is retained; no deadline was added.
- `20261101-attribution-history-cli-01-baseline`: qualified delivered image 290
  passes equal-history control then fails to reject a lowered history. Synthetic
  fixture only; no live journal modified.

Only owned runs were interrupted for memory pressure (retained interruption JSON
records). Recipes use debug symbols 0 as delivery and one compiler job, with all
functional cases and no global watchdog. Cancellation is not PASS. Object
attribution is not yet exposed by the CLI; operator wiring stays in BLP-03. The
new CLI journey tests the consumer on explicitly injected fixture history,
without creating a testing right.

HTTP verification replan: the proposed `paea_git_call` extension was removed from
the candidate. That target requires `SWARM_PAEA_SOURCE_FIXTURE`, a previously
accepted writable native fixture mission; no other actor's fixture was borrowed.
Web attempts 01–05 are retained: interrupted compilation, new-test borrow error,
missing native prerequisite, and binder stdout contract corrected for compact
JSON. None is declared PASS. The recipe now compiles explicitly before binding,
retaining complete Cargo diagnostics.

`20261101-attribution-renewal-web-06`: **PASS, integrity verified**, one fresh
mission-free `confidentiality_http` test: real HTTP bootstrap/policy, engine
producer refusal without mutation, equal renewal, lowered-history HTTP refusal
without material in body/headers, exact recovery. The test image is bound before/
after with stable bytes. This does not replace the prepared-call PAEA-route
qualification, which remains for a verifier-owned fresh native fixture.

### Local correction result — 1 November 2026

| Check | Result and scope |
|---|---|
| Engine `20261101-attribution-renewal-06` | PASS, 12 tests; equal authorized control before history injection, exact reader cause, refusal without mutation and recovery |
| HTTP `20261101-attribution-renewal-web-06` | PASS, fresh mission-free Root, bound/stable test image, no borrowed native prerequisite |
| CLI `20261101-attribution-history-cli-02` | PASS, 15 assertions on qualified normal image 292: control, level/compartment decrease refusal, redaction, unchanged journal and exact recovery |
| Installed `20261101-attribution-installed-01` | PASS: loaded image, compiled notes and seven runtime sources match; 56 browser assertions covering FR/EN, themes, version/commit link and protected PAEA asset bytes against compiler inventory; no browser errors or writes |
| Inventory `20261101-flow-inventory-01` | PASS: 24 flows, 45 anchors, five verifier tests; declared freshness only, not complete mediation |

**v0.1.1-dev.292**, image
`4164953c841c2b22476bd552a12496aac2b7f95e9b26de04d986c3238e2ed83c`,
was installed by a parallel delivery before this closure. Its normal profile and
FR/EN notes already contain the fix. Seven runtime source files and the loaded
image were explicitly compared; no additional restart/service interruption was
needed. My redundant 294 reconstruction was stopped, claiming neither qualification
nor installation. Newer sources stay outside this proof. Launcher `status` was
waiting for the lifecycle lock held by my build; final observation used the
protected API and checked process identity before/after, without bypassing a
mutating operation. Private browser profiles remain under Git-ignored `evidence/`.

Byte-bound self-review: [source-review](../../../plans/actions-confiance/verification-blp/20261101-attribution-source-review.json).
No new operator policy: historical dominance is a BLP invariant, schema/identifiers
are protocol, fixture values are test inputs, image IO buffer size is technical
allocation rather than a read cutoff. Modified/new functions have contracts;
errors were traced through engine, HTTP and CLI with positive controls/recovery.
Original compiler diagnostics and incomplete recipes are preserved.

Local lesson: policy-edit authority is not declassification authority. Guard both
producer and retained-history reads. A bounded recipe must create its prerequisites;
a retained native-mission journey is not a generic HTTP fixture. Preserve compiler
diagnostics before artifact binding and inspect the binder's actual stdout format.
With concurrent work, inspect an already-qualified/installed image and its relevant
inputs before rebuilding; do not wait on exclusive `status` behind your own build.

Limits: self-review only; prepared-call PAEA routes were not qualified here;
digest/path proves neither physical nor copy lineage. Full preparation attribution,
authenticated principal, mediation and inheritance remain pending. Previously
lowered histories refuse; no historical repair command is delivered. The observed
instance has no Trust journal; no real-data migration/bootstrap was performed.
**BLP-04 remains partial and BLP-06 unavailable; no transversal confidentiality or
independent acceptance is claimed.**

## Resume after commit/push — public producer, 1 November 2026

The monotonicity fix is retained in `a76fb3ef5`, pushed to
`origin/codex/clear-product-method-names`. BLP-04 replan: the engine already
provides I3/admin-protected `blp-object-attribute`, but the fixed HTTP and CLI
allowlists refuse it before its consumer. Operators therefore cannot produce
the attribution required by readers through those interfaces.

Two options: wait for the complete preparation source registry, or connect the
existing object control without extending its authority. Codex chooses the
second, bounded to public allowlists, CLI help, admission/refusal/replay witnesses
and documentation. No client-selected endpoint, new policy, implicit principal
or automatic classification. Existing Admin configures policy; this increment
adds no object-attribution form. Digest/path control proves no physical source;
preparation, inheritance and mediation remain open.

Delivery criteria: reproduce current HTTP refusal with a valid intent; admit
the same authenticated HTTP/CLI operation; reject session-only/weak bearer,
stale revisions, conflicts and downgrades without mutation; verify exact replay
and recovery. Run permanent recipes into fresh evidence, then qualify/install
the normal binary and check actual version/served resources. Owner: Codex.
State: regression tests being written; no newly verified behavior claimed.

Method incident: the ad hoc HTTP counterexample inherited no explicit TMPDIR;
`TempDir` created and removed its owned fixture in the system temporary directory,
contrary to the project rule. Original outputs remain, without qualification.
The next HTTP check used the permanent recipe's configured private TMPDIR and
passed with verified integrity. The CLI counterexample uses its own durable
private fixture. CLI recipe now accepts `SWARM_BLP_PROOF_OUTPUT` to retain actual
journey requests/channels in the chosen proof folder instead of only linking a
local fixture path. No real credentials.

### Public-producer increment result

- HTTP `20261101-public-producer-web-01`: PASS, integrity verified; one autonomous
  test covers HTTP attribution, session-only/weak bearer refusal, stale revisions,
  downgrade, replay, conflict and recovery.
- CLI `20261101-public-producer-cli-01`: PASS, integrity verified; 25 assertions
  on the qualified production image. Public operations produce valid history;
  only invalid history is fixture-injected. All 78 request/channel artifacts and
  their final manifest are bound to the digest printed in stdout and reverified.
  On the old image the CLI counterexample stops at `command_failed` parsing,
  before expected authentication.
- Delivery `20261101-public-producer-installed-01`: PASS; only the managed service
  restarted using `--prepared-build`, **v0.1.1-dev.295**, image
  `e1adea56e4cf53e0787d362f55e14bbd53d40c82a38b3815a081b0a33e203c73`.
  Normal profile retained, nine runtime sources matched, 56 browser assertions
  for FR/EN/themes/version/protected assets, plus FR/EN CLI help checks. Compiled
  commit `a76fb3ef5`; the dialog discloses uncommitted changes.

Evidence clarification: inherited `service_restarted:false` in the immutable
probe result describes only that read-only probe. `delivery-scope.json` binds
original results and restart channels with `service_restarted_by_increment:true`;
the local probe template now uses an explicit field. No functional result is
rewritten. The single build completed in 5m34s despite concurrent Git maintenance;
CPU/stage progression and memory pressure remain in
`20261101-public-producer-build-contention.json`. No foreign process stopped,
arbitrary deadline or duplicate build.

Policies/functions/errors review:
[hashes and limits](../../../plans/actions-confiance/verification-blp/20261101-public-producer-source-review.json).
Fixed operation names are protocol vocabulary; no new operator policy or ceiling.
CLI/HTTP contracts document actual authority; producer errors and existing
close/publication/output handling remain intact. HTTP evidence is reused only for
unchanged web/engine sources; the later English CLI-help clarification is not
consumed by that target and was checked on the normal delivered image. Subsequent
changes are evidence/documentation only and require no behavioral rebuild.

Local RETEX: verify the public producer path before requiring its attribution
from readers; Admin intent establishes no physical provenance. Retain detailed
channels, not just local paths, and review scope fields when deriving a probe.
Use the recipe runner's configured TMPDIR from the first counterexample; the
initial ad hoc location mistake remains retained. Same-context review only,
without independent acceptance.

Next retained work: BF-05 → BF-07 → BF-16, complete preparation-source attribution,
then authenticated principal/domain binding and result inheritance. Newly public
manual object control supplies none of these contracts. BLP-04 stays partial;
BLP-05/06, explicit existing-object migration and transversal RETEX remain open.
No live Trust bootstrap, provider call, declassification or global conformance.

## BLP-04 resumption — recorded-source check, 1 November 2026

Codex resumes after the public producer. Native v3 covers prompt bytes, but no
consumer joins control attributions of its parts. Two routes: enforce a policy
on existing sends immediately, or first build an authenticated diagnostic on
the actual recorded prompt. The second avoids claiming provider mediation.
Decided increment: readonly `blp-preparation-check` via API/CLI under the Trust
reservation; authenticated identity, explicit revision, current attribution for
every exact part (including framing), join and clearance check. No request
labels, public default, mutation or provider call. Historical records without
provenance refuse. Errors, close and output retain causes. Refusal/recovery
tests, FR/EN documentation and local installation are required. This diagnostic
does not bind the send principal or authorize dispatch/destination.
Status: implementation underway; BLP-04 remains partial. Owner: Codex.

Recipe incident: `20261101-preparation-check-engine-01` remains intact FAIL:
bootstrap supplied two identities while `prepare_identities` requires three.
Cause: incomplete derived fixture before the BLP check; add the third fixture
identity without changing product rules. Another Rust build consumes the same
sources, so Rust edits wait for completion to preserve its compiler inputs;
independent Python/documentation work continues.

Coordination incident: concurrent commit `b4cbfa2255` included BLP sources and
still-open evidence; this session issued no commit. History stays intact.
Engine run 02 subsequently completed normally: PASS, one named test, stable
sources and VERIFIED integrity. Intermediate committed bytes are not the final
evidence. The public recipe is still being completed, with no PASS claimed.
Normal build 296 qualifies its own bytes/Git metadata; read its compiled commit
from the artifact rather than infer it from the current HEAD.

Delivery 296 refused: compilation succeeded (1m23s), but qualification refused
`profile-effective-input-changed:rust/crates/swarm-web/src/service.rs` after a
concurrent Authority-gate edit. Original channels/hashes retained in
`20261101-preparation-check-build-refusal-01`; image unqualified/uninstalled.
Options: reexamine current state and qualify a fresh prebind, or make another
complete source copy. The first preserves direct checkout development/cache
without bypassing the guard. Fresh prebind must include observed service.rs bytes.

Increment criteria (not transversal closure): readonly diagnostic with one named
engine test already PASS; normal-image public journey with 21 CLI/HTTP assertions
and actual send to an inert local double; exact installed image and served
FR/EN version/resources. The computed classification describes observed context,
not a persisted provider-response attribution. The checked principal belongs to
the diagnostic, not the historical send. Operational docs also expose the absent
inventory screen and still-aggregate JSON field granularity.

Public counterexample: `20261101-preparation-check-public-before-01` on qualified
image 295. Three controls pass: actual public send to inert local double, producer
provenance and complete byte coverage. The new check then refuses at the CLI as
an unavailable operation instead of the expected unattributed-source refusal.
Intact FAIL, VERIFIED integrity; journey channels bound by `counterexample.json`.
This does not claim that historical sending had BLP protection.

HTTP error replan: new typed Root refusals lacked HTTP mapping in trust-command,
falling through to500. Reproducer `20261101-preparation-http-status-before-01`: a
valid HTTP200 BLP control, then ungranted principal blp-access-denied rendered500.
Explicitly unqualified compiled image copied before replacement; defect evidence
only, never installation/acceptance. Second build interrupted by SIGINT of its
verified owned group only: exit130, INCOMPLETE, originals retained in
`20261101-preparation-check-build-interrupted-02`. Decision: typed business-only
Root mapping within trust-command (400/403/404/409/501); corrupt provenance and
cleanup remain500. Typed matrix and public journey now require26 exact assertions.
Rust edits wait for the other compilation/test consuming this shared file.

BF-07/BF-16 prerequisites reexamined: current public send accepts no authenticated
Trust identity. Native stores worker nonce, Root and provider/settings digests,
without a principal. A nonce/provider name cannot replace that binding. Next
contract must bind transport-derived identity to intent/replay, revalidate current
authority at engagement, then persist output classification including errors and
summaries. Provider configuration hash establishes freshness, not external
recipient/account identity. Never add a JSON-claimed principal to bypass this
prerequisite. This diagnostic must never serve as a send ticket; later mediation
must recompute exact sources and current authority.

### Verified result — source diagnostic, 1 November 2026

- Engine `20261101-preparation-check-engine-02`: PASS, one named test, stable
  sources and VERIFIED integrity. Seven relevant current inputs still match;
  HTTP-only edits do not require another engine run.
- Status `20261101-preparation-http-status-01`: PASS, one named test, stable
  sources, VERIFIED integrity. Typed business mapping corrects the reproduced500;
  corrupt provenance/cleanup remain server failures.
- Public `20261101-preparation-check-public-01`: PASS, **26 CLI/HTTP assertions**,
  real prompt produced by inert local double; every attribution uses public
  operations, SQL only reads this owned fixture. Refusals preserve the journal,
  exact observation replays; eight HTTP cases assert exact status/code. All
  **149 artifacts** and final manifest hash verified:
  `9d5e8d828fb2ce014a8132576c993fe9a63deba4227e6918f9e1e698629d6b99`.
- Delivery `20261101-preparation-check-installed-01`: **installed v0.1.1-dev.296**,
  image `e4a802c4dc4d32d90d036169604168c64db15196695d692885369b748686ca72`, normal
  qualified profile `.cache/local-delivery/rust-0ukzkap_/build/profile`, no fixture.
  Compilation finished in37s after memory recovery. Supported exact-image restart,
  zero newer effective inputs at that observation. Thirteen runtime sources
  matched; **56 browser assertions** cover FR/EN/themes/version/protected assets/
  commit/dirty/focus, plus two CLI help checks. Compiled commit
  `b4cbfa22552140fe9a7bcf46ac91165b0abe120d`, modified=true.
  `delivery-scope.json` distinguishes increment restart from readonly probe.
- Inventory `20261101-preparation-flow-inventory-01`: PASS, two steps/five checker
  tests, VERIFIED integrity. Three changed files semantically reviewed before
  hash/anchor update; BF-25 and five references added: **25 flows,50 anchors,
  46 files**, all OPEN for transversal protection. Original stale inventory
  remains intact FAIL/VERIFIED in `...-before-01`.

[Byte-bound policy/function/error review](../../../plans/actions-confiance/verification-blp/20261101-preparation-check-source-review.json).
The attempted cancellation of our third queued build sent no signal: its guard
observed compilation had just started. That build subsequently qualified the
image above. A separate concurrent build refused changed version_history.json/
graph.js inputs; no qualification was borrowed or evidence rewritten. Launcher
reservation serializes builds, not every session's writes: a finished PID does
not establish absence of a new compiler. Git packing disappeared without this
session intervening; its exit status was not observed. Interruption approval
became unnecessary; no foreign process was interrupted, and this session issued
no commit/push.

Local RETEX: trace refusal through HTTP status, close and output; engine PASS
does not prove the public consumer. Derive fixtures from real contracts (three
identities), preserve original failures and verify nested manifests. Update
inventories within the same increment. Qualification actually refused drift and
prepared-build selected only the qualified image. Coordination improvement:
origin=concurrent writes/builds/Git maintenance; value=reduce requalification and
wait without weakening guards; scope=development method; owner=local workflow
maintainer. Alternatives: shared exclusive write window or bounded frozen inputs
through supported delivery. Status=proposal outside this BLP increment; observable=
stable qualification during parallel activity, without treating instructions as
effect. Resume during coordination-contract design; already-running sessions do
not automatically receive this note.

**BLP-04 remains partial**: controlled labels for actual recorded slices, no
physical producer attribution/inventory screen, no original send-principal or
recipient-domain binding, no persisted output inheritance. BLP-05/06, existing
object migration and transversal recipe/review/RETEX remain open. BF-05 → BF-07 →
BF-16 retains these prerequisites; this diagnostic is no dispatch ticket. No live
Trust bootstrap, paid provider, human declassification or global conformance;
same-context review only. Final tracking/inventory edits do not change the
installed executable.

## Documentation resumption — existing state, 1 November 2026

User request: document existing implementation after clarifying that BLP is
incomplete. Replan: contracts/evidence exist but scope is scattered across Biba,
flows and plan history. Alternatives: extend history only, or create a linked
FR/EN operational reference. The latter is implemented in
[Bell-LaPadula in Swarm](../../BELL-LAPADULA.md), linked from README, security
positioning, Biba and flows. Owner: Codex.

The reference distinguishes pure calculation, administration, exact attribution,
targeted PAEA readers, native v3 provenance and post-recording diagnostic. It
explains authority, absent physical lineage/inheritance, real operations and
unfinished packages. Evidence stays bound to its sources/profiles and local
owner-key flow; no Authority/enrollment qualification or global conformance is
added. Version API reread:296, compiled b4cbfa2255, modified=true (dated observation).

Checks: local links/anchors, retained evidence counts and FR/EN review. No code,
policy, validation or error handling changed; no function created/modified and
no Rustdoc/docstring update required. Edited documentation is absent from the
qualified compiler input inventory: no build/restart/new delivery note. Other
sessions' version_history.json edits remain preserved. Persistent links reach
future repository readers; documentation rebuilds no embedded context or running
process. Increment review: `verification-blp/20261101-blp-documentation-review.json`.

## Execution resumption — preparation egress lock, 1 November 2026

User authorizes continued BLP-04/05/result inheritance, then filesystem/network/
descendant mediation, migration and human declassification. Current worker
receives network-capable data after budget/freshness checks without a BLP lease
through emission. Existing results and unrelated work remain preserved.

Alternatives: check only at send receipt (activation/revocation gap before IO),
or serialize actual transport with Trust changes and also refuse before admission.
Choose the second. First step: persisted boolean `preparation_quarantine`, absent/
false retains legacy behavior, true explicitly refuses new sends and pending
worker egress. No activation on live data. Admin policy JSON, API and CLI share
validation/effective value, with explicit FR/EN status. Policy changes wait for
an already-engaged shared reservation; existing stop/deadline policies apply.
No new timeout/cap or implicit role.

Criteria: legacy JSON/digest unchanged for absent/false; reject null/wrong type;
readonly preview/save/reopen/recovery; valid inert sender before activation then
zero emission after refusal; transport recheck, serialized activation against
in-flight egress; preserve causes/observed effect on release failure. Permanent
recipes, byte-bound review, FR/EN docs and qualified local installation. Owner:
Codex, no delegation/paid provider. This lock supplies no authorized classified
send, inherited response label or descendant confinement; these remain open and
work continues beyond the lock.

Pre-delivery availability replan: core-01 validated a global Trust lease during
IO, which would also block business writes. Alternatives: retain blocking with
manual stop, or separate egress serialization and keep Trust only for a snapshot.
Choose dedicated preparation-egress.lock: shared through operation, exclusive
for config-apply/blp-policy-apply, order egress → optional SQL → short Trust.
Enrolled consumer acquires egress before its SQL transaction to avoid admission
inversion. Canonical I3/admin preflight rejects unauthorized waiters; authoritative
authentication repeats. Core-01 remains historical, superseded for concurrency;
11/11 pure model sources remain unchanged. New witness requires egress writer
blocked, business Trust writer available and weak caller refused without waiting.
Public journey also covers already-engaged CLI emission.

Evidence recovery: locks-02 refuses qualification despite 1+15 passing tests because UI translations changed during execution. These are not compiler inputs for engine tests; remove those four inputs from the engine-only collector and retain UI verification in its public journey. The original refusal is preserved.

Method check: assumed npm script `i18n:check` does not exist; the refusal is retained in the session. The actual package.json contract provides `i18n:build`, whose generator accepts `--check`; verification uses that direct command without changing compiled resources.

Replan public-01: 25 transport assertions pass; browser fails before interaction on an assumed test URL. native_admin.rs serves trust-native.html at /trust-actions.html, not /trust-native.html. Correct journey and route → compiled-source mapping; enrich HTTP diagnostics with credential-free pathname. No product change, tolerance or hidden404; resume with corrected inputs.

## Quarantine qualification and delivery — November1,2026

Implemented, verified and installed version299, SHA256
`75ad879faf4807d33b6b9c3ff5a71e570aa76d252f394da3b309d95ecadda5e9`.
Normal native profile without fixtures; pending-turn helper compiled separately.
Retained checks: pure model11 (unchanged core-01 sources), release1 + confidentiality15
+ enrollment4 (locks-03), public28 and Admin59 (public-02), inventory54 anchors/
25 OPEN flows and5 checker regressions (inventory-01). Inert receivers observe
sockets and actual requests, including API/CLI engaged-operation serialization,
no-egress refusal, queued production worker refusal and explicit recovery.

Exact prepared restart, running process/version/FR/EN notes/protected assets,
56 browser assertions and FR/EN CLI help verified (installed-01). Compiled commit
d4e219eeaf, modified=true. Launcher discloses one newer compiler input pending.
No live Trust bootstrap, data classification or quarantine activation; no paid
provider, commit or push performed by this increment. Third-party work preserved.
Evidence and byte-bound self-review: verification-blp/20261101-preparation-quarantine-*.
Original locks-01/02 and public-01 refusals remain intact. Persistent FR/EN contracts
and recipes transmit the verified lesson, without rebuilding already-started agent
contexts. No independent acceptance or global conformity is claimed.

Continue BLP-04 with authenticated admission principal, producer classifications
and immutable captured-byte domain, then BLP-05 with fixed destination/account/current
policy at transport and BF-16 inheritance onto results/errors/proposals/readers.
Quarantine is an explicit block, without an authorized classified-send route.
OS/descendant mediation, existing-data migration and BLP-06 remain open.

## BLP-04 resumed — attribution from captured source, 1 November 2026

After quarantine299, the diagnostic resolves prompt slices but public attribution
still requires an operator-supplied digest/path. Options: immediately persist an
execution domain before protecting its readers/destinations, or first connect
attribution to the actual producer. Choose the latter. Codex alone implements
`blp-preparation-attribute`: authenticated I3/admin, clearance for the explicit
classification, current Trust/BLP/turn revisions, exact parent, Root instance and
complete provenance; select a captured slice index instead of supplying its hash
or path. Reuse attribution uniqueness, no declassification and publication/replay.
Capture context participates in event identity; source text is never returned.

Required checks: persistent CLI/API, identical replay, invalid index, wrong
parent/revisions, authority before row lookup, forged input/refusal without
mutation, diagnostic join after source classification, identified native profile
and local installation. Identical slices remain one object per digest/project/
revision; another event does not relabel them. This is post-capture control
attribution, without durable physical lineage, admission principal, immutable
domain, allowed classified egress or output inheritance. These remain open.

Replan de recette / recipe recovery: before-01 échoue avant le nouveau parcours
(KeyError revision). Le DTO réel expose source_revision ; correction du consommateur
de test, preuve originale conservée, reprise dans before-02. Aucun comportement
produit ne découle de ce premier échec.

Verification availability recovery: compilation remains active under observed
memory pressure (14GiB RAM,14GiB/15GiB swap) and concurrent Git pack-objects
(~6GiB, another parent). No arbitrary timeout or interruption of others; monitor
outputs/CPU while doing independent documentation work. Slowness is not a test result.

Pre-delivery review found a redundant nested SQL transaction when the consumer
already owns one. engine-01 was deliberately interrupted (only its own rustc)
and remains an intact FAIL. Use one complete row read instead; retain a valid
control, caller-transaction preservation, corruption/Root/history refusal and
recovery regression. No production failure was observed: the signed enrollment
vocabulary does not yet expose BLP operations. This change neither adds them nor
qualifies enrollment authentication. The touched CLI help also incorrectly said
there was no execution control despite quarantine; FR/EN now say partial controls
and distinguish authenticated blp-check from Admin-only administration.

Retained improvement: operators need to distinguish slices and identical digests
without SQL access. Value: understandable explicit classification; scope: future
protected source inventory; owner BLP/UX; status OPEN. Observable: identity/
clearance before metadata, correct producer selection, no unauthorized classified
text. This API does not provide that screen.

engine-02 recovery: both tests pass, but the inherited collector includes web
sources for its later step; native_admin.rs changes during the engine test and
correctly refuses the whole batch. Split engine and HTTP adapter recipes using
their actual dependencies. Engine reuses the locks-03 dependency closure; web
remains in its own collector and the qualified-image public journey. Lesson:
bind evidence per compiled stage instead of inheriting a combined collector.
Original engine-02 remains intact/FAIL.

Method recovery: engine-03 was recompiling despite the2 unchanged results, so
only its own compiler was stopped; retain the refusal. engine-reuse.json records
reuse of2 named engine-02 tests and522 retained engine inputs identical before,
after and currently. The original batch remains FAIL, without full compiler-
graph or production-profile claims. Keep the corrected engine recipe for future
relevant changes.

Observed server is303 (another delivery;299 remains historical). Build304
compiles in38.88s but qualification refuses concurrent edits to locales/en.json,
web/i18n-en.js and web/native-authority-admin.js. No restart or304 installation.
Originals retained in build-01. Resume with a new effective-input snapshot after
Authority/UI edits, an independent HTTP recipe and qualified-image public flow.

build-02/adapters-01 recovery: the named HTTP unit passes1/1, but its batch
refuses Authority source changes during compilation. The second build also
compiles then refuses changed audit.rs/server.rs. These are actual compiler
dependencies: no exclusion or installation of a refused candidate. Both builds
and the adapter batch remain intact FAIL. User coordination requested: finish
or pause Authority edits for a qualification window; owner Authority session/
user. None of its processes was interrupted. Independently extend the public
engine operation regression: actual captured attribution, refusal/replay/event
conflict, corruption/Root/history and no declassification after renewal. Only
engine regression sources change; no runtime change after build-02.

public-01 recovery: CLI attribution and HTTP replay pass. The first HTTP refusal
assertion fails because CLI tuples (code, status) were reused as (status, code).
The original response is403 authorization_required, matching the contract.
Explicitly convert tuple order for the HTTP consumer; preserve original failure,
change no runtime code and rerun the same qualified341e71bb… image with a fresh fixture.

## Captured-source producer delivery — 1 November 2026

Installed and verified **v0.1.1-dev.304**, image SHA256
`341e71bb2e67d47d0c468c04f8c7ecd38f313fb0686b60af88ba675d50cbade6`,
compiled commit821dbb340ceb99b323fd4bb9a7f2bbdc106b0e9c, modified=true. The third
build obtained stable inputs and a qualified native profile without fixtures;
refused builds remain intact and were never installed. Exact prepared-image
restart reported0 newer compiler inputs then. Authority coordination required
no interruption of its processes.

Evidence: engine-04,2 named tests PASS/VERIFIED; adapters-02,1 HTTP unit PASS/
VERIFIED; public-02,103 CLI/HTTP assertions PASS/VERIFIED, including actual HTTP
publication then CLI replay and attribution under quarantine without lifting it;
inventory-02,56 anchors/25 OPEN flows and5 regressions; installed-01,21 matching
runtime inputs and56 browser checks covering version, FR/EN notes, actual commit
and dirty disclosure. Engine functional test bytes remain unchanged after adding
its Rustdoc only (separate test-doc-review); no invented rerun for documentation.
Original recipe errors, interruptions and concurrent changes remain retained.

Byte-bound self-review: `20261101-preparation-source-producer-source-review.json` covering policies, functions, errors
and documentation. No independent review, global acceptance or transversal BLP
claim. No paid call, real classification, Trust bootstrap, real policy activation,
commit or push by this increment.

BLP-04 gains post-capture control attribution, not an admission-time domain.
Next retained work: authenticated admission sender, inherited producer labels and
fixed domain; actual account/recipient transport binding (BLP-05), then results
and BF-16 readers. Files/network/descendants, migration and human declassification
remain open. `runtime_enforced:false` stays justified. Docs/recipes persist but
do not replace already running agent contexts.

Final check: concurrent stash restoration creates conflicts in both PAEA-APEX
languages. Original bytes (losslessly compressed) and Git stages retained in
documentation-conflict-01; preserve historical299 then304 sections and adjacent
text, resolving only these two documentation index entries. No commit/runtime
change. Working/index diff checks and documentation review refreshed.

## BLP-04 resumed — fixed domain before egress, 1 November 2026

User authorizes admission-time domain work. Compare promotion of historical/
pending turns with exposed legacy readers/dispatch against a separate immutable
admission with no executable turn. Choose the latter: reuse the exact prompt
producer, capture without SQL/budget mutation, explicitly attribute slices through
a cleared I3/admin operation, then join all current attributions for the actual
authenticated admitting principal. Unknown sources refuse without a capsule;
no role, label, account or destination is inferred.

Private create-only capsule uses existing native receipt framing (observed extent/
SHA), binds opened Root, principal, policy revisions/digest and complete prompt
bytes. Its namespace is separate from legacy turns/readers/workers. Immutable
replay, current principal/clearance metadata reads, policy/Root/corruption refusal.
Admission reserves no budget, contacts no provider and grants no transport.
Quarantine neither blocks this local no-provider-IO work nor changes implicitly.
Reuse persisted context and storage-contention policies. Required evidence:
producer parity, no mutation on capture/refusal, authority before lookup,
pre-admission attribution, missing-source/clearance/revision/no-declassification
refusals, fixed capsule/replay/reopen, corruption/uncertain publication handling,
no turns/budget/egress, actual CLI/API, FR/EN docs and qualified local delivery.
Codex alone; no delegation/paid provider/live policy changes. Recipient, output/
reader, OS and migration contracts remain open.

engine-01 recovery: normal compilation refuses AtomicStorageFault through its
fixture-only public reexport. Use the existing internal type as Trust storage
does; do not enable fixture features. Also retain requested level/target/context
mode without credentials for the future still-unauthorized transport integration.

engine-02 recovery:6 cases pass including admission; a historical test expects
active legacy replay to refuse. Actual304 instead reproduces readonly/pending
with the row unchanged; replay_turn bytes match the304 profile exactly. Keep
original FAIL. Correct the test contract: readonly observation, no native
dispatch, native claim refused and reservation unchanged. Rerun only this case;
the six passing test bodies/runtime code stay unchanged. Retain actual public
legacy-baseline-01 evidence. Git maintenance finishes without interruption;
no suspension was applied without a response granting authority.

build-01 compiles normally in40.88s but qualification refuses changed Authority
audit.rs/server.rs during compilation. Originals retained; no installation or
dependency exclusion. Next build uses the updated actual Authority inputs.
Observed instance before this build was308 (another delivery).

public-01 recovery: CLI admission/join/no-mutation checks pass; a local http
function shadows the imported Python module before the first HTTP request.
Rename only that helper to request_http and update the inherited evidence prefix.
Keep original FAIL, use a fresh fixture and the same qualified309 image; no runtime change.

Actual delivery recovery:309 runs internally on38434 behind Authority38433.
Owner-key probe verifies image/version/22 sources/notes then gets403 for protected
assets, as required by Authority mode. No gate weakened or human credential
fabricated. installed-01 retains refusal; installed-02 explicitly reports
PARTIAL_AUTHORITY_UI_NOT_VERIFIED with22 sources and three asset403 controls.
Browser host unavailable; actual signed-in UI observation requested from user.
Newer Authority/assertion.rs wire-v2 source is outside image309; its delivery is
not claimed and belongs to Authority work. Authority status reports active but
installed binary differs from latest build; no human-session diagnosis inferred.

Independent UI fixture uses the SAME production image. First driver incorrectly
calls observed Server.request during async capture; actual contract requires
observe=False for separate HTTP. Correct only driver, fresh ui-02, preserve live
Authority UI criterion OPEN. Shared browser recipe now explicitly distinguishes
owned-image fixture from managed instance with identical assertions and a private
fixture marker, never an authentication bypass.

## Pre-transport admission state — 1 November 2026

Implemented code is active in **v0.1.1-dev.309**, image SHA256
`c12004bf64f1ac5371d78ebeb4ab2d852da9b0e83d3e4a692b8fb28268dbc69b`.
Qualified normal native profile, no fixtures; exact shared-producer body parity.
Actual admitting Trust principal, all-current-source join and immutable private
capsule, with no provider IO, executable turn, budget reservation or sink grant.
Routing metadata contains no key and supplies no recipient authority.

Code evidence:6 unchanged passed engine-02 test bodies reused, plus1 corrected
legacy case PASS/VERIFIED (engine-reuse/legacy-01). Original engine-02 remains
FAIL, not a fabricated full-batch PASS; its only failure reproduces on304 and
the test is corrected to actual readonly replay. Public-02:74 CLI/HTTP assertions
PASS/VERIFIED, including join distinct from maximum clearance, real HTTP input
attribution, revocation, frame/Root/label/JSON corruption controls/recovery and
no provider/turn/reservation. Legacy-public-01:103 previous assertions pass on
the same image with an inert CLI provider. Inventory-01:60 anchors/25 OPEN flows
and5 checker regressions.

Exact prepared-image restart activates309. Installed-02 verifies running image/
PID, version, FR/EN notes/help and22 runtime files. Newer Authority assertion.rs
v2 source is outside this image and remains Authority delivery work. The other
workstream switched the real instance to Authority: daemon38433, engine38434.
Three protected assets correctly refuse the owner cookie403; no gate disabled.
Real Authority UI verification remains **OPEN**, owned by user/session: actual
sign-in and Version dialog observation; browser host currently unavailable.
Installed-02 explicitly reports PARTIAL_AUTHORITY_UI_NOT_VERIFIED, never UI PASS.

Ui-02 separately passes56 browser assertions on an owned Root using the SAME
production image (actual served assets, FR/EN notes, version/commit). This does
not establish real Authority-session acceptance. Original refusals retained;
no live classification/policy changes, paid provider, commit/push or fabricated
human identity. [Byte-bound source review](../../../plans/actions-confiance/verification-blp/20261101-preparation-admission-source-review.json) covers policies/functions/errors/docs;
self-review only. Next BLP-05 binds current account/recipient/key and rights/
budgets at actual transport, then output inheritance/readers and OS/descendants.
Upstream transformations, migration, human declassification and capsule retirement
remain open; runtime_enforced:false stays justified. Persistent docs/recipes do
not replace existing native contexts or establish recurring supervision.

## Resume BLP-05 — recipient and last-use transport, 1 November 2026

Need: BLP-05 — effective account/recipient control at last use, before provider
emission. User choice confirmed: **explicit sink binding** — one operation
records a destination label for the exact account/endpoint
(base_url+model+config digest); the classified send revalidates
capsule+destination+policy/budget at last use under the egress lock; the legacy
send path is **not** modified.

### Delivered (implemented + locally tested, not activated in the worker)

- `blp-preparation-destination-attribute`: privileged administrator, mandatory
  `base_url`/`model` echo of the current connection, canonical digest recomputed
  by the Root (never caller-supplied), key never an input; goes through the
  `blp-object-attribute` producer (exact-once uniqueness, inherited
  anti-declassification guard). [`preparation_transport.rs`](../../../../rust/crates/swarm-engine/src/root_store/preparation_transport.rs)
- `blp-preparation-transport`: under the shared egress lock, full capsule
  re-verification, connection/provider_source/settings freshness since
  admission, current binding for that exact digest, last-use no-write-down
  dominance, read-only budget observation; fixed non-emissive reply
  (`transport-authorized-not-sent`, `runtime_authorized:false`,
  `runtime_enforced:false`), publication `None`.
- Dispatch readonly + egress: `trust_store.rs:69-122`; HTTP mapping
  400/404/409/403/500 in `swarm-web/src/service/trust.rs`; CLI whitelist
  `swarm-cli/src/trust.rs`; public readonly `blp-preparation-transport`.
- Integration test `rust/crates/swarm-engine/tests/preparation_transport.rs`:
  **PASS 1/1 (2.0s)** — not-attributed refusal, TOCTOU echo, admin-only
  binding, key-free recorded binding, exact replay, fixed non-emissive decision,
  identical replay, ownership, zero SQL reservations, self-cleaned fixture.
- Public journey `tests/blp_transport_public.py` + recipe
  `tools/verification/recipes/blp-preparation-transport-public.json`:
  **executed on 2 November 2026 on the qualified binary** sha256 `41f00bf9…`
  (profile `rust-zrtk4hyt`, locally installed, v0.1.1-dev.312): **PASS 65/65
  assertions** (`BLP_TRANSPORT_JOURNEY_SHA256` `0690d1e0…`), recipe
  `recipe.py run` PASS then `recipe.py verify` `VERIFIED/PASS` (evidence
  `.swarm-fixtures/blp-transport-recipe-20261102/`, journey
  `.swarm-fixtures/blp-transport-proof-20261102/`). The journey also proves
  identical replay, no-write-down (`blp-access-denied`), policy-renewal
  invalidation (3 revisions), settings freshness, identity revocation, the
  full HTTP map (200/400/404/404-ownership/403) and zero provider launch or
  SQL mutation (sentinel + counters). No `/tmp` residue from the journey.
- BF inventory (`tools/paea/check_blp_inventory.py`): refreshed, 65
  references / 25 flows, 0 drift, +5 transport references (producer,
  dispatch, HTTP).
- Improvement surfaced by execution: `preparation_calls.rs` allocated its
  fixtures via `std::env::temp_dir()` (31 October `/tmp` interdiction);
  fixed on the established pattern (`TMPDIR` required + `/tmp` refusal,
  birth record, `Drop` retaining the root on panic only). Suite
  **7/7 PASS**, proof `/tmp` before=after, zero remaining root.
- Byte-bound review: [source-review](../../../plans/actions-confiance/verification-blp/20261102-preparation-transport-source-review.json).
- FR/EN documentation: BELL-LAPADULA (new section), BIBA-RUST (full technical
  contract + diagram), BLP-FLOWS (BF-05/BF-07 revision paragraph).

### Limits retained

The decision does not engage the real transport (BF-07 OPEN); result/reader
inheritance, OS/descendant confinement, legacy sends, upstream transformations:
unchanged. `runtime_enforced:false` stays justified. The worker hook (Api branch
of `preparation_worker.rs`) is the **next** increment, not this one; no existing
send path was touched. No real data classified, no paid provider, no real
capsule.

### Next step

Next increment: the worker emission hook (Api branch of
`preparation_worker.rs`) calling the last-use decision before any classified
send; the legacy path stays untouched. Increment RETEX:
[RETEX-BLP-05-20261102](../../../plans/actions-confiance/paea/RETEX-BLP-05-20261102.en.md).

## BF-07 resumption — worker emission hook, 2 November 2026

Need: BF-07 — the API worker (`_prepare_turn`, Api branch of
`swarm-cli/src/preparation_worker.rs`) must call the last-use transport
decision before any classified send; the legacy path (zero admission capsule)
stays intact. Builds on BLP-05 (the `blp-preparation-transport` decision),
without touching that decision or its refusals.

### Design verified before writing

- The connection name comes from the recorded turn's `provider` field (never a
  caller argument); the worker's effective `base_url`/`model` are passed as
  the echo, exactly like the decision — the
  `blp-destination-echo-mismatch` refusal closes the claim→send TOCTOU.
- The turn custody reader (`gated_turn`, `pub(super)` in
  `preparation_call.rs`) re-reads the row under the material guard and repeats
  the possession check token=SHA256(nonce) + PID/stamp/host; exact `running`
  status (the state left by the claim). `Native` stays private; the
  `TurnCustody` projection exposes neither prompt, provenance nor nonce.
- The principal comes from the receipt written by the admission producer
  (self-authenticating: `admission_id == identity(principal, preparation_id,
  event_id)` — tampering the principal breaks the digest). The worker holds no
  principal token; possession of a claimed turn is never an identity.
- Admission↔turn coupling: `request_hash` is canonical over the whole
  `PreparationSend` struct **including `event_id`** — verified by running the
  test (two distinct event ids produce two distinct hashes); a receipt
  therefore matches a turn's (preparation_id, provider, request_hash) for the
  same event id.
- Locks: the gate runs inside the shared egress reservation already held by
  the worker (shared+shared never blocks; the quarantine snapshot is re-taken,
  strictly no weaker). The egress → Trust → settings order is preserved; the
  short Trust reservation is taken only after at least one matching receipt
  is detected.

### Delivered (implemented + locally tested, not activated in production)

- `RootStore::preparation_worker_transport_gate(id, token, base_url, model)`
  in `preparation_transport.rs`: zero matching receipts → `Ok(())` **before
  any Trust-state read** (legacy path unchanged); ≥1 receipt → full
  re-verification of every capsule (receipt principal, instance, revisions,
  policy digest, every source re-join, output-label reconstruction),
  settings/provider_source freshness, destination attribution for the
  resolved digest of the recorded connection, last-use no-write-down,
  read-only budget observation. Any failure refuses before the send; the gate
  contacts nothing, reserves nothing, writes nothing to SQL.
- Fail-closed scan: only exact-grammar names
  `blp-admission-{64 hex}.receipt` are candidates; a matching receipt that
  cannot be read or decoded refuses (`blp-admission-not-found` /
  `blp-admission-record-invalid`), never a silent pass.
- CLI: the token is threaded from `execute()` into `run()`;
  `preparation_worker_transport_gate(id, token, &r.base_url, &r.model)` runs
  before the request is built (`swarm_api_client::complete`) — no socket
  opens before the decision.
- Integration test
  `rust/crates/swarm-engine/tests/preparation_worker_transport.rs`:
  **PASS 1/1 (1.8s)** — zero admissions → immediate `Ok` without reservation;
  tampered token → `preparation-worker-registration-mismatch`; admission
  without destination → `blp-destination-not-attributed`; wrong echo (both
  base_url and model) → `blp-destination-echo-mismatch`; exact attribution →
  the gate passes; no gate reservation. Self-cleaning fixture,
  `.swarm-fixtures` area.
- Permanent recipe
  `tools/verification/recipes/blp-preparation-worker-gate.json`:
  **PASS 3/3 steps** (`recipe.py run`, evidence `.cache/bf07-proof/`) —
  gate (refusals + legacy path), unchanged neighbor suites
  (`preparation_calls` 7/7, `preparation_transport` 1/1), `swarm-cli`
  compilation with token threading.

### Limits retained

The gate is active only in the Rust worker's Api branch; the CLI path
(`preparation_worker_run_cli`) keeps its own egress control (outside BF-07
scope). No real data classified, no paid provider, no real capsule.
Result/reader inheritance (BF-16) and OS/descendant confinement: open.
"Implemented + locally tested" is neither activated in production nor
accepted; Bell-LaPadula is not declared finished.

### Next step

BF-16: domain inheritance on responses/errors/propositions and readers. The
BF-07 commit/push awaits the usual explicit authorization.

## BF-16a — dialogue reader control — 5 November 2026

First BF-16 sub-increment, delivered after the reproduced pre-fix hole: an
admitted send with a classified answer was served by the public dialogue
read to a bare session with no control at all.

- **Implemented**: scan-then-decide in
  `RootStore::preparation_dialogue(id, reader_token)` (SQL read first as
  decision input, then a fail-closed admission-receipt scan; zero matching
  receipt = legacy path, ≥1 = PA-09 identity then three-pass recovery —
  full re-verification, stale-receipt coverage by a current equivalent of
  same principal/Root/label/lineage, `check_read` on every receipt
  classification). Four typed codes:
  `blp-dialogue-authentication-required`, `blp-dialogue-policy-required`,
  `blp-dialogue-policy-stale`, `blp-dialogue-access-denied`. Web: the
  `reader_bearer_token` extractor (one non-empty Bearer, typed refusal
  before Root open) + 403 on the four codes. CLI: `SWARM_TRUST_TOKEN`
  (empty = bare session). The internal producer path (`preparation_call`)
  stays uncontrolled, deliberately. `reader_identity_state`/
  `ReaderIdentityState` widened to `pub(super)` with no logic change. The
  staleness recovery and stored-corruption guards are the work received
  from the delegated sessions of 3–4 November 2026 (images 325/331,
  detailed history in `docs/actions-confiance-rust.md`); this entry
  documents BF-16a as a whole.
- **Locally tested**: integration test `preparation_dialogue_readers.rs` —
  bare session/unknown token/under-cleared-reader matrix (refusal, zero
  bytes served), recovery by a cleared reader, legacy continuity without
  admission; 2/2 green. Neighbor BF-07 gate: 1/1 green. Permanent recipe
  `tools/verification/recipes/blp-preparation-dialogue-readers.json`:
  PASS 3/3, evidence
  `.swarm-fixtures/blp-dialogue-readers-proof-20261105b-2479227` (re-run on
  the evolved tree after the staleness reception; integrity VERIFIED;
  `.tgz` archive + SHA-256 copied and re-read on
  `/media/fpizzi/Lexar/swarm-preuves/2026-11-05/`, index
  `.swarm-fixtures/PROOF-INDEX.md`). The public HTTP boundary and the
  staleness recovery are received via
  `tools/verification/recipes/blp-dialogue-reader-public.json` (93
  checks, images 325/331, history in
  `docs/actions-confiance-rust.md`). The full `cargo test -p
  swarm-engine` suite was not replayed end to end after interruptions; the
  targeted evidence covers the two affected suites.
- **Limits retained**: presentation reads only — `preparation_show`,
  `preparation_history` and adopted documents linked by `source_turn`
  still lack reader control (next BF-16 increment, closed by BF-16b
  below); inheritance on errors/propositions, file/network/descendant
  mediation, migration of existing data and human declassification
  (BLP-06) remain open. No local install performed (the workspace check
  is still blocked by the authority session's concurrent WIP, E0063
  `authority_ledger.rs`); no commit/push without explicit authorization;
  "implemented + locally tested" is neither activated in production nor
  accepted.

## BF-16b — readers of show, history and release-review — 6 November 2026

Second BF-16 sub-increment: the dialogue's reader verdict extends to the
other presentation frontiers, closing the "read through show/history what
the dialogue refuses" bypass.

- **Implemented**: the verdict extracted into
  `preparation_dialogue::reader_verdict` (pub(super)) shared by
  `read_controlled` (dialogue), `preparation_show(id, reader_token)` and
  `preparation_history(id, kind, before, reader_token)` — same
  `blp-dialogue-*` codes, same scan-then-decide, same three-pass
  recovery, same clearance; nothing is served before `Ok(())`.
  Show/history build their decision keys through the `recorded_turn_keys`
  helper (recorded turns decoded under the same snapshot, prompts
  cleared, row identity mismatch refusing like dialogue, no active-history
  observation). Zero matched receipts = legacy path kept.
  `preparation_release_review(id, reader_token)` forwards the token to
  show. Web: `reader_bearer_token` extracted on show/history/
  release-review; 403 unchanged on the four shared codes. CLI:
  `SWARM_TRUST_TOKEN` resolved once for show/release-review/dialogue/
  history.
- **Locally tested**: `preparation_dialogue_readers.rs` 4/4 green — the
  two retained BF-16a tests plus `show_and_history_inherit_the_dialogue_
  reader_verdict` (bare session + under-cleared reader refused on show
  and history for every kind, recovery by the cleared sender, history
  `besoin` revision 1 served to the cleared reader) and
  `show_and_history_without_any_matched_admission_keep_legacy_bare_read`
  (bare-session continuity without admission on both frontiers).
  Web/CLI regressors: `swarm-web --test service` 10/10,
  `--test confidentiality_http` green, `swarm-cli` run before the
  concurrent `trust/data_policy.rs` WIP (transient E0432 observed after,
  the delegated session's scope). `preparation_show` gains a
  `reader_token` parameter: callers adapted (web, CLI, release-review).
- **Limits retained**: presentation reads only — execution, adoption and
  other channels stay outside the verdict; inheritance on
  errors/propositions, file/network/descendant mediation, migration of
  existing data and human declassification (BLP-06) remain open. No local
  install (same concurrent WIP block); no commit/push without explicit
  authorization; "implemented + locally tested" is neither activated in
  production nor accepted. The permanent recipe
  `blp-preparation-dialogue-readers.json` covers the dialogue frontier
  and the BF-07 gate; a recipe extension covering show/history is kept as
  follow-up work (the integration tests stand as evidence meanwhile).


## BF-16c — readers of send (replay and first admission) and stop (terminal turn) — 8 November 2026

Third BF-16 sub-increment: the last two frontiers that served a recorded
turn with no reader control, closing the "obtain through send (replay) or
stop (terminal turn) what the dialogue refuses" bypass.

- **Implemented**: `preparation_send(raw, environment, reader_token)` —
  the replay (an already-recorded provider/request_hash triple) decides
  the shared `reader_verdict` over `recorded_turn_keys` before serving
  the recorded `PreparationTurn` (classified answer included, the prompt
  is never serialized); the first admission decides the same verdict
  **inside the write transaction**, after `save` and before the budget
  reservation — a refusal rolls the whole admission back (no turn row, no
  reservation, no byte; an under-cleared caller cannot consume the budget
  of an admission it cannot read). `preparation_stop(preparation_id,
  turn_id, environment, reader_token)` — both terminal-turn serving
  branches (legacy row, native) decide the verdict when the turn is no
  longer active, before returning; a stop on an active turn remains a
  producer transition, not a read. The monitor's settle pass passes
  explicit `None` with the producer-exemption comment (mirroring
  `preparation_call` left uncontrolled since BF-16a; `None` is never an
  authorization). Web: `preparation_calls` extracts `reader_bearer_token`
  like the GET frontiers (capability requests still readonly and outside
  the verdict); the four shared codes map to 403 under operation
  `preparation-call` in `service.rs`. CLI: `SWARM_TRUST_TOKEN` resolved
  once for send/stop in `calls()`.
- **Locally tested**: `preparation_dialogue_readers.rs` 6/6 — two new
  tests `send_replay_and_stop_serve_classified_turn_without_reader_
  control` (bare session + under-cleared reader refused on both the
  replay and the first admitted send, admission rolled back; terminal
  turn refused to a bare session and an unknown token; recovery by the
  cleared sender) and `send_replay_and_stop_without_any_matched_admission_
  keep_legacy_bare_path` (legacy send/stop continuity without admission).
  Red→green proof: under the previous signatures, the replay served the
  turn exactly where the refusal is asserted. Regressors:
  `preparation_calls` 24/24, `confidentiality` 1/1,
  `preparation_worker_transport` adapted (legacy/native),
  `confidentiality_http` + web `service` 11/11, CLI gate 2/2, workspace
  check 0 errors. Callers adapted: examples and suites (legacy sends/
  stops pass explicit `None`).
- **Limits retained**: presentation reads only — execution, adoption and
  other channels stay outside the verdict; inheritance on
  errors/propositions, file/network/descendant mediation, migration of
  existing data and human declassification (BLP-06) remain open. No local
  install at this stage; no commit/push without explicit authorization;
  "implemented + locally tested" is neither activated in production nor
  accepted.


## BF-16b non-regression evidence — swarm-cli mapping, 7 November 2026

Closing the evidence BF-16b called for: classify every remaining `swarm-cli`
failure as pre-existing debt or a reader-lot regression. The BF-16a/16b lot
touches only `preparation_dialogue`, `preparation_read`,
`preparation_show/history/release_review` and their callers; the families
below are outside that surface and nothing in the lot moved them.

- **agent_launch — 4 pass / 9 fail (BT02, transversal Biba debt)**:
  reconfirmed after removing the diagnostic instrumentation
  (`agent_execution_preflight.rs`/`agent_launch_preflight.rs` back to
  HEAD, empty diff). The 9 native failures are the 30 October BT02 debt
  (6706e201b): `workspace==root` fixtures → `biba-workspace-overlaps-root`
  pre-exec refusal → `status: failed`, `exit_code: None`. Owner Codex,
  dossier BIBA-TRANSVERSE-APEX.md. Outside the BF-16 scope.
- **Full mapping (7 November, after a clean rebuild) — the TMPDIR family
  widens to 13 failures across 8 targets**: agent_runtime ×3, budget ×1,
  planning ×2, pricing ×1, provider_configuration ×3, provider_policy ×1,
  run_limits ×1, task_model ×1. Two confirmed mechanisms: (a) sequential —
  in a `for flush in [false, true]` loop, iteration 2 creates its
  `tempfile::tempdir()` with TMPDIR still pointing at iteration 1's
  already-dropped `.swarm-fixtures` → NotFound
  `/tmp/.tmpXXX/.swarm-fixtures/.tmpYYY`; (b) parallel — the Rust harness
  runs a binary's tests on concurrent threads and `env::set_var` is
  process-wide: test A exports TMPDIR to its root, test B creates its
  tempdir (or launches its CLI child that reads its `--input`) under A's
  tree, A drops its root → nested NotFound
  (`/tmp/.tmpA/.swarm-fixtures/.tmpB/.swarm-fixtures/.tmpC`, observed at
  agent_runtime.rs:23) or child ENOENT (`input-document-io` errno 2,
  provider_configuration.rs:32). Product/test-typology debt introduced by
  f9ff58214 (31 October); fix at the contract level (freeze TMPDIR per
  test or move to subprocess calls). None of these targets touches the
  BF-16 reader surface.
- **PAEA capture guard — expected behavior, not a product failure**: the
  23 reds from the first paea pass (no TMPDIR) are the explicit guard at
  `paea_capture_stream.rs:272` ("explicit TMPDIR is required … never
  /tmp") — the /tmp interdiction applied to capture evidence. With TMPDIR
  pointed explicitly, the full paea block is green:
  paea_reconcile/promote/settlement_delivered/approval_web 31 pass / 15
  ignored (retained canonical fixtures required), paea_settlement 1 pass /
  3 ignored.
- **Enrollment ancestor conflict — environment, not a defect**:
  `Documents::open` → `open_private_directory` (swarm-enrollment-config
  store.rs:105) requires non-group/other-writable ancestors;
  `/home/fpizzi/Documents/ChatGPT` and `swarm` are 775 → `path` refusal
  even with a TMPDIR under `.swarm-fixtures`. A TMPDIR under
  `/home/fpizzi` (755) clears the block: `paea_settlement` green. The
  run directory `/home/fpizzi/swarm-run-tmp` (0700, outside the repo)
  serves as the compliant local TMPDIR area for the suites; to be
  documented in the permanent recipe.
- **planning `planning_cli_enables_new_work_and_refuses_unported_
  preparations` — catalogue-read, never green**: isolated failure
  confirmed as `agent-provider-catalogue-read` vs expected
  `command_not_ported` (planning.rs:417, `provider:"unported"`
  iteration). The test uses a subprocess `call()`: outside the TMPDIR
  family. Code reconstruction at the creation commit 174b89106:
  `validate_planning_enable_target` never refuses a provider (only
  new-work/budget/repository), and `reviewer_bootstrap::prepare` →
  `observe_provider` → `agent_launch::provider` requires `providers.json`
  (ENOENT → `agent-provider-catalogue-read`; the `absent()` fallback only
  covers ai-connections.json); the `PlanningBootstrap(e) => e.code()`
  mapping is identical on both sides; the fixture (identical at creation
  and today, file never modified) never seeds `providers.json` and `init`
  never creates it — only `providers config init` does. The provider
  iteration therefore never could produce `command_not_ported`: the
  "never green" hypothesis is confirmed by code reconstruction; direct
  historical execution remains unavailable (the 3 November binary was
  deleted, rtk logs only cover today). Since 298640e07, `validate`
  receives `local_request` (repository cleared), so the repository
  iteration now follows the same bootstrap path and would fail too.
  Owner Codex; candidate fixes: seed `providers.json` in the fixture OR
  refuse the provider before bootstrap. Full evidence chain:
  `.swarm-fixtures/diag-instr/planning-catalogue-read-finding.md`.
- **Latent (recorded, out of scope)**: a non-canonical `--root` makes
  the parent observe a different TMPDIR than the helper
  (`apply_runtime_temporary_directory` joins the raw root text, the
  helper receives the canonical root). No current failure reduces to it;
  watch during recipe migration.
- **Environment observation (this session)**: conflict between the /tmp
  interdiction (tmpfs wiped at reboot) and `fixture()` helpers built on
  `tempfile::tempdir()` → `/tmp`. The affected suites cannot be run in
  compliance with the interdiction nor relocated without source edits;
  to settle at the test-contract level (owner Codex); meanwhile point
  TMPDIR explicitly outside /tmp when the suite allows it (the guards
  additionally require non-group-writable ancestors — see the enrollment
  ancestor conflict above).

Lesson: none of the remaining swarm-cli failures is attributable to the
BF-16b reader lot; every red traces to dated debt (30-31 October) older
than the lot or older than the test itself. Exhaustive mapping of the
swarm-cli targets executed on 7 November: cli 17, identity_* 37, gate 2,
graph_batch 1, linux_prerequisites 2, model_policy 2, trust_enrollment 2,
agent_workflow 2, work_list/work_view/web_bootstrap/version_no_repository
18, protection ×6 22, planning_report 2, pricing 2/1F, provider_policy
1/1F, reported_costs 2, run_limits 1/1F, settings_output 3, task_model
1/1F, task_report 1, provider_configuration 1/3F, paea_* 33 green/18
ignored, budget 2/1F, agent_runtime 1/3F, planning 2/3F (catalogue-read
never green + 2 TMPDIR), agent_launch 4/9F (BT02 Biba).

## Class-2 closure — tempdir migrations under governed areas, 7 November 2026

Closing the environment finding above ("the `fixture()` calls going through
`tempfile::tempdir()` → `/tmp`"): the 151 bare allocation sites under
`rust/crates` are migrated to the `tempdir_in(<area>)`/`new_in(<area>)`
contract (RAII `TempDir` kept, `tools/check_tmp_interdiction.py` guard
extended). Distribution: swarm-engine 14 (repository area), swarm-web 6
(repository area), swarm-cli 86 (repository area; `version_no_repository.rs`,
the 4 identity modules `src/identity_{operator,provisioning,service,
service_call}/tests.rs`, the 7 trust_enrollment + identity_*.rs integration
suites, cli.rs, paea_settlement and the 4 declaration-bearing paea suites
(promote, transfer, approval_web, settlement_delivered) → fallback area:
their flow reads anchors/declarations through `open_private_directory`,
which refuses the checkout's 775 ancestors — corrected after a first pass on
the repository area had caused and withdrawn 7 identity `--lib` reds, then
8 integration reds (2 trust_enrollment + 6 identity_checkpoint) and 3
cli/paea reds during the verification runs), enrollment crates +
linux-prerequisites 51 (inline
fallback area: the `open_private_directory` ancestor walk refuses the
checkout's 775 ancestors, hence `~/.local/state/swarm-fixtures/` 0700;
consumers have no swarm-engine dependency, so the fallback is replicated
inline per crate). Repository git
safety established: every engine git call pins cwd+`--git-dir .` and strips
`GIT_*` (managed_repository.rs:33); only `git_metadata.rs` contracts
"no ancestor repository" (fallback area, D04). The guard moves from
"tempfile without `dir=`" to "any bare Rust tempfile allocation":
0 violations, 10 review items (documented/pre-existing).

Verification: `cargo check --tests` 0 errors on the 7 touched crates
(engine/web/cli on defaults; config/state/bootstrap/linux-prerequisites and
authority with `--features test-fixtures`, the project recipe convention —
the plain `--tests` reds on `install_with_hook`/`FixtureFault` pre-exist at
HEAD, feature-gated, outside the verification path); `cargo test`
config/state/bootstrap/linux-prerequisites 110 green, authority 207 green,
trust_enrollment 2 green + identity_*.rs 37 green after the area correction.
Docs: verification README FR/EN (governed form + fallback area),
`fixture_area.rs` module doc (D05). Work evidence:
`rust/.swarm-fixtures/d04-class2-files.txt`, `d05-violations.txt`,
`d05-guard-final.txt` (final guard: 0 violations), `d05-cli-files.txt`,
`d05-cli-final-noff.txt` (final full run). Permanent recipe
`tools/verification/recipes/rust-class2-fixture-areas.json` (guard + identity
`--lib` + trust_enrollment + identity_checkpoint): PASS 4/4, verification
VERIFIED, evidence `.swarm-fixtures/class2-recipe-20261108T030000Z`; dedicated
section in `tools/verification/README.md` (EN/FR).

swarm-engine `--lib` reds qualified **out of the class-2 scope** (the tests
and their chain's sources are identical to HEAD, empty diffs as proof):
- `preparation_policy_explicit_defaults_legacy_and_unlimited` and
  `mission_report_settings_absence_is_not_default_adoption`: the v52 drift
  (`schema_version()` returns 52 via `has_web_access_policy`, test expected
  41) — recorded v52 debt, Codex wave;
- `captured_source_reuses_caller_transaction…`/`recorded_sources_authentication…`
  require a pinned TMPDIR ("recipe fixture area required") — green with
  `TMPDIR=/home/fpizzi/swarm-run-tmp` (0700, outside the checkout; to fold
  into the permanent recipes);
- `agent_supervision`/`web_session`: green in isolation, concurrency flakes
  (ephemeral port / process-wide writes);
- **`mission_status_reports_contained_scan_symlinks_refusal_recovery` and
  `current_review_report_and_artifact_staleness_recover`: debt dated to the
  Biba batch 2496c031b (28 October)** — at the test's origin (174b89106)
  `artifact_metadata` called `local_file` directly (source =
  `LocalPathError`, downcast OK); the `anchored_artifact::with`
  interposition makes `GateEvaluationError::source()` an
  `AnchoredArtifactError`, the `LocalPathError` downcast
  (mission_status_reports.rs:1118) fails and `semantic_artifact` no longer
  recognizes the refusal (gate-evidence-invalid). Unrelated to the fixture
  area change (root `.swarm-fixtures` vs `/tmp` does not enter the
  containment check). Owner: Codex Biba wave; lead: project the
  `LocalPathError` source through `AnchoredArtifactError::Path` in
  `anchored_error`, or downcast the transitive source.

swarm-cli verification (governed TMPDIR; the 7 November cartography ran
without `--no-fail-fast`, completed on the 8th by a full `--no-fail-fast`
run): identity `--lib` suites 18 green after the area correction (fallback
required — see the distribution above); `agent_launch` 5 green / 8 red =
the mapped BT02 Biba debt (workspace==root →
`biba-workspace-overlaps-root`, CLI dead before READY), owner Codex,
BIBA-TRANSVERSE-APEX.md dossier; final full run: 258 green / 9 red /
20 ignored — the 9 = the 8 agent_launch + 1 planning
`agent-provider-catalogue-read` (the "never green" debt, reconstructed at
the creating commit 174b89106; red-run evidence:
`.swarm-fixtures/d05-cli-final-noff.txt`, final full run; the
reconstruction diagnosis belongs to the Codex lot — no verifiable
evidence file as of 8 Nov, owner Codex); the 20 ignored = canonical-fixture/recipe guards for dedicated
recipes; with the canonical `SWARM_PAEA_SOURCE_FIXTURE` the 5 paea suites
add 29 green. The first full-pass cli.rs/paea_settlement reds were
introduced by this batch (repository area under declaration-bearing suites)
and then withdrawn — trust_enrollment was green in the 7 November BF-16b
cartography, so were the first-pass integration reds.


## Administration and qualification — 5 November 2026

The user-adopted [Security and evidence plan](SECURITY-PROOFS-APEX.md) supplements
BLP-01–07 with engine/API/CLI observation, UX, administration, authorized recipes
and an internal qualification dossier. This link changes no BLP conformity, test
or acceptance state.

## BF-16d — readers of proposal adoption, command replay and the list — 8 November 2026

Fourth BF-16 sub-increment: the three remaining frontiers that served an
admitted answer's classified domain with no reader control, closing the
"obtain through adoption, command replay or the list what the dialogue
refuses" bypass. The errors axis was already covered by BF-16a/b/c
(frontier-controlled DTOs, static safe codes only); this increment closes
propositions and the list (BLP-APEX:801).

- **Implemented**: `preparation_command(raw, environment, reader_token)`
  gains `reader_token` as its last parameter (mirroring
  `preparation_send`) — the `use-proposal` adoption copies the source
  turn's classified answer into `documents`/`adopted_brief` (the document
  inherits its source's domain) and the receipt replay serves a
  historical `Preparation` body; the shared `reader_verdict` (keyed on
  `recorded_turn_keys`) decides **inside the Immediate transaction, after
  `apply` and before the commit** — a refusal rolls back the document,
  the revision bump and the receipt row through the existing rollback;
  one call covers both adoption AND replay.
  `preparation_list(reader_token)` judges each row exactly like
  `preparation_show` — the first matched-admission row without a
  sufficient reader refuses the whole list before any byte (serving
  unclassified rows while refusing others would disclose the existence
  of classified content through the response itself). Legacy continuity:
  zero matched receipts keep the bare path (adoption, identical replay at
  the current revision, list) — the decision stays keyed on matched
  admissions, never on policy presence. Budget views: counters only, out
  of scope (BF-16a choice unchanged). Web: `POST
  /api/v1/preparations/command` and `GET /api/v1/preparations` extract
  `reader_bearer_token` with the shared extractor (a malformed header
  refuses before any Root read); the shared codes' 403 mapping was
  already wired in `service.rs`. CLI: `SWARM_TRUST_TOKEN` resolved in
  `write()` (command) and the readonly path (list).
- **Locally tested**: `preparation_dialogue_readers.rs` 8/8 — new
  `use_proposal_replay_and_list_adopt_classified_answer_without_reader_
  control` (bare session refused on adoption/replay/list, no classified
  byte in any refusal, under-cleared → `blp-dialogue-access-denied`,
  unknown token → typed Authorization refusal, traceless refusal —
  revision unchanged, no brief document, no receipt row — recovery by the
  cleared sender) and its twin
  `use_proposal_replay_and_list_without_any_matched_admission_keep_
  legacy_bare_path`. Red→green proof: signatures threaded first (the
  test's 10 call sites failed compilation), then the verdict — the
  established runtime red shows the bare session ADOPT the classified
  brief exactly where the refusal is asserted. Regressors:
  `preparation_writes` 5/5, `preparation_calls` 7/7, `preparation_plan`
  1/1, `preparation_transport` 1/1, `preparation_worker_transport` 1/1,
  `confidentiality` 16/16, web `server_mutations` 1/1, workspace check
  0 errors. Callers adapted: CLI `write()`/`list`, web
  `preparation_command`/list, the `preparation_quarantine_fixture`
  example, transport suites (pre-admission sites pass explicit `None`).
- **Limits retained**: presentation reads only — execution, conversion
  and other channels stay outside the verdict; file/network/descendant
  mediation, migration of existing data (BLP-03) and human
  declassification (BLP-06) remain open. Installed locally the same day
  (24 October decision): `VERIFIED_NATIVE_CANDIDATE` sha256
  `dbc888fb…df3d9` delivered via `install.sh --mode native`, the 38434
  engine service restarted under the same contract (PID verified by
  sha256 of `/proc/<pid>/exe`); the HTTP flow stays protected by the
  pre-existing web auth (403 without a session, refusal retained).
  No commit/push without explicit authorization; "implemented + tested +
  installed" is neither activated in production nor accepted.

## BF-14a — network mediation: explicit Internet-socket refusal for provider helpers — 9 November 2026

Authorizing line: BLP-APEX.md:727 ("… then file/network/descendant mediation,
migration and declassification."). This increment is the network part of that
mediation (the BF-14 gap: "Internet destination control absent"), before data
migration (BLP-03) and human declassification (BLP-06).

### Finding and contract

The mandatory Biba profile refuses AF_UNIX but leaves `socket(AF_INET)`/AF_INET6
reachable for classified provider helpers — a compromised or prompt-injected
helper could open an outbound Internet connection. The policy is not derived
from code: it is an explicit operator choice, persisted in the confidentiality
policy and changed through the existing Admin surface
(`blp-policy-preview`/`blp-policy-apply`, Trust I3). New
`provider_internet_denied: bool` field in `swarm-confidentiality::Policy`
(conditional serialization; absent and `false` encode identically — historical
digests preserved, mirroring `preparation_quarantine`).

### Implementation (engine constraint: `#![forbid(unsafe_code)]`)

- **Kernel filter** (`swarm-biba-linux/src/kernel.rs`, additive extension of
  Codex's crate — his `seccomp()` filter and `enforce()` untouched):
  `deny_internet()` installs a second seccomp filter AFTER the mandatory
  profile. Filters stack and can only restrict: EPERM on `socket`/`socketpair`
  when the requested domain is AF_INET or AF_INET6, every other domain keeps
  its existing verdict. Pose failure refuses; no unconfined fallback.
- **Exec point** (`native_provider_exec.rs`): `PreparedExec` carries an
  `internet_denied: bool` set by the engine (`with_internet_denied`, never
  accepted from JSON); `execute()` poses `biba::enforce`, then the additive
  refusal, then `execve` — no re-read after restriction.
- **Decision frozen at each gate**: every transport reads its own
  `provider_internet_denied_snapshot()` under its own Trust reservation
  (`root_store/provider_network.rs`) and passes it to the exec point. All five
  sites are wired: agent gate (`agent_execution_gate.rs`), planner
  (`planning_native.rs`), reviewer (`reviewer_native/gate.rs`), preparation
  CLI (`preparation_call_cli.rs`) and task validation
  (`task_validation/{process,biba}.rs`). Corrupt or unsupported state refuses,
  never falls back to false.
- **Exposure**: `capability()` (`blp-show` diagnostics) reports
  `provider_internet_denied` with `runtime_enforced:false` — the persisted
  value is not proof of enforcement.

### Tested locally

- `swarm-confidentiality`: the new test (absent→false, explicit false
  byte-identical to absent, true round-trip, successors both directions,
  invalid values refused) plus the twin quarantine guard. Suite green.
- `swarm-engine --test confidentiality` **18/18**: Admin control (preview
  refused on the worker token, applied as Admin), `blp-show` reflects the
  value, snapshot reads true, authority renewal does not lift it, explicit
  back-to-false at the expected revision; corrupted state → snapshot errors,
  never falls back. Project-local TMPDIR (`~/swarm-run-tmp`), never /tmp.
- `cargo check --workspace` and `--all-targets`: 0 errors (the 16 pre-existing
  `swarm-enrollment-authority/tests/service_installation.rs` errors are Codex's
  work in flight behind `#[cfg(feature="test-fixtures")]`, preserved unfixed).
- **OS-level recipe** `biba-linux-boundary` extended with the `native-internet`
  case (drivers compiled against cargo's real rlib, never the product binary):
  unconfined control → INET created; **mandatory profile alone → INET still
  created (the BF-14 gap is demonstrated at kernel level)**; profile + additive
  refusal → uniform EPERM on INET/INET6/INET-socketpair (seccomp evaluates
  before the kernel's family validation), AF_UNIX verdict unchanged.
  Environment facts distinguished from policy verdicts: this kernel has no
  IPv6 (EAFNOSUPPORT=97) and the kernel only implements socketpair for AF_UNIX
  (EOPNOTSUPP=95). Full run **PASS** with retained evidence.

### Installed

On 9 November (24 October decision): qualified candidate
`VERIFIED_NATIVE_CANDIDATE` sha256 `24024aba…3ec5` produced by
`build_native.py --install-candidate` (the first run failed on
`profile-effective-input-changed:version_history.json` — my own concurrent
edit, fixed by re-running without touching sources; a method defect, not a
product defect), delivered as `bin/swarm` (the old `dbc888fb…` archived under
`.swarm-fixtures/bin-archives/`), engine service 38434 restarted under the
same contract (PID 557693, sha256 of `/proc/<pid>/exe` = candidate sha, HTTP
flows 403 without a session — pre-existing web auth retained). No commit/push
without explicit authorization; "implemented + tested + installed" is neither
activated in production nor accepted.

### Limits

The refusal covers Internet socket creation by classified helpers; full
destination (account, resolution, proxy), already-established descendant
channels and other egress shapes (files, caches, telemetry) remain outside it —
BF-14/BF-15 stay OPEN. Implemented + tested + installed ≠ activated in
production ≠ accepted. No commit/push without explicit authorization.

## BLP-03a — explicit migration of existing objects: Admin form — 9 November 2026

Originating authorization: BLP-APEX.md:727 (BLP-03: "persisted reviewed
configuration, explicit migration, Admin/API/CLI"). The
history/operations/CLI batch of BLP-03 was VERIFIED on 31 October 2026
(tests 5/5); the migration of existing unclassified objects remained. This
increment delivers the operator surface of that migration: an Admin form in
the Trust dialog (`trust-actions.html`), one gesture = exactly one object,
driving the existing tested `blp-object-attribute` producer (its contract
unchanged).

### Finding and contract

No Admin surface exposed `blp-object-attribute`: attributing an existing
object required the API/CLI (no scan exists and none is introduced). The
form carries the producer's exact contract
(`swarm-engine/src/trust/confidentiality.rs`, `deny_unknown_fields`):
canonical SHA-256 digest (64 lowercase hex), absolute project path,
classification {level, compartments} chosen from the re-read policy —
never invented by the page — and a non-empty reason. Two gestures:
preview (no change applied), then save which replays the SAME event bytes
(idempotence retained). One gesture = exactly one object; no automatic
scan of unclassified objects exists.

### Implementation

- `web/trust-native.html`: sub-fieldset "Attribution of an existing object
  (migration)" inside `#blp-settings` — `#blp-object-digest`,
  `#blp-object-path`, `#blp-object-level` (select fed from the re-read
  policy), `#blp-object-compartments` (multi-select, none preselected),
  `#blp-object-reason`, `role="status"` line, `#blp-attribute-preview`
  then `#blp-attribute-apply` (disabled until a preview validates).
- `web/trust-confidentiality.js`: the `SwarmTrustConfidentiality` module
  extended — level/compartment options rebuilt at every re-read from
  `policy.levels`/`policy.compartments`; client validation (digest,
  absolute path without NUL, level chosen, non-empty reason);
  `blp-object-attribute` event with `expected_revision`/
  `expected_trust_revision` decoded as exact decimals (no Number
  rounding); preview then replay of the same bytes; preview invalidated
  on any input; full reset when Admin closes (no private material
  retained); `runtime_enforced` stays `false` (no transversal
  activation).
- `locales/en.json` + `npm run i18n:build`: 13 EN entries added (9 HTML,
  4 JS); `web/i18n-en.js` regenerated.

### Tests (red → green, qualified production image)

- `tests/blp_object_attribute_public.py` +
  `tests/blp_object_attribute_ui.cjs`: Puppeteer driver against the real
  Admin of the qualified binary — full journey (form present, levels from
  the re-read policy, preview `attributed===true`, save replaying the
  same bytes), exact persistence (one attribution per journey, eight
  distinct objects — the producer refuses a second recording of the same
  digest/path/revision triple under another event id;
  digest/path/level/actor), typed 403 refusals (session alone, weak
  bearer) without mutation, console-error suppression, 2 languages × 2
  themes × 2 widths, served-asset digests = the profile's `after.json`.
- RED (pre-form binary, run1 `064e624a…`): CLI journey PASS, browser
  failed (form absent) — evidence retained at
  `/home/fpizzi/.local/state/swarm-fixtures/objattr-profile/red-1/`.
- GREEN (run3 `a166e2b8…`, profile `VERIFIED_DEFAULT_NATIVE_PROFILE`,
  archived sources stable): `OBJECT_ATTRIBUTE_PUBLIC_PASS 4`
  (policy-current, admin-ui-attribution-form-journey,
  one-explicit-attribution-persisted, qualified-production-image-unchanged)
  — evidence `/home/fpizzi/.local/state/swarm-fixtures/objattr-profile/green-2/`.
- Races diagnosed while tuning (product unchanged, evidence retained):
  `green-0-hung` (controls inside a closed `<details>` — silent clicks),
  `green-0b-dup409` (one object for eight journeys — the producer refuses
  a second record of the same triple), `green-0c-escape-race` (post-apply
  wait satisfied in flight; Escape closed the dialog during the request
  and focus restoration targeted a still-disabled button). The final test
  waits for the completion text set after the re-read, then presses
  Escape.
- Permanent recipe: `tools/verification/recipes/blp-object-attribute-public.json`
  (inputs = tests + embedded assets; env `SWARM_BLP_BINARY`,
  `SWARM_BLP_PROFILE`, `SWARM_BLP_PROOF_OUTPUT`).

### Limits

Attribution remains a per-object operator decision, under the existing
Admin authentication (typed 403s retained); `runtime_enforced:false` — no
transversal activation. Automatic scanning of unclassified objects stays
nonexistent by design (no enumeration); file mediation (BF-13/BF-15) and
human declassification (BLP-06) remain OPEN. Implemented + tested ≠
installed ≠ activated in production ≠ accepted. No commit/push without
explicit authorization.

## BLP-06 — implementation contract prepared for Codex — 9 November 2026

Exact human declassification requires three engine changes that remain Codex's
scope (the "Codex-only" decision of 24 October 2026; the engine files carry
Codex's concurrent BF-14a diff right now). This contract fixes the exact
expected behavior; this session wrote no engine code.

### 1. Signed vocabulary — `swarm-enrollment-authority/src/control/trust_action.rs`

Add `ObjectDeclassify` to the closed `Operation` enum (`as_str` →
`"blp-object-declassify"`, added to the closed list in `Operation::parse`).
The name binds a signed permission; it grants no role and selects no process
or endpoint.

### 2. Producer — `swarm-engine/src/trust/confidentiality.rs`

A new `op=="blp-object-declassify"` branch in `execute()` (after
`authentication::require_administrator(&actor)`) that:

- decodes a `DeclassifyChange` `deny_unknown_fields`: `{schema_version:1,
  event_id, expected_revision, expected_trust_revision, canonical_digest,
  project_path, recipients, reason}` — the same validations as
  `blp-object-attribute` (event_id `swarm_identity_contract::name`, non-empty
  control-free reason, lowercase 64-hex digest, absolute path without `\0`,
  `expected_trust_revision==st.revision`, `expected_revision==entry.revision`,
  current entry, else the same typed codes `blp-policy-required`,
  `blp-trust-revision-conflict`, `blp-policy-stale`);
- refuses unless the actor is an enrolled human: `actor.enrollment` must be
  `Some` (source `Source::Enrolled` only — a local I3/admin bearer never
  declassifies) and the enrolled principal stays `kind=="human"`, already
  guaranteed by `trust_as_enrolled` (`trust_enrollment.rs:73`) — typed error
  `blp-declassification-human-required` otherwise. The admin clearance alone
  never constitutes the BLP-06 human approval (rule already recorded above);
- requires exactly one current attribution for the object
  (`entry.attribution_for(digest, path)` — `None` →
  `blp-declassification-object-unknown`, never a fallback to a stale entry);
- replays the exact event bytes (same `event_id` → recorded reply,
  `trust_publication:"replayed"`; known `event_id` with different bytes →
  `event_conflict`), the same exact-once contract as attribution;
- requires `recipients` to be the non-empty list of exact validated
  recipients (existing non-disabled enrolled identities by `principal_id`; an
  unknown or disabled recipient → `blp-declassification-recipient-unknown`),
  never a role or a label;
- produces a NEW COPY: the effect records a
  `Declassification{canonical_digest, project_path, policy_revision,
  recipients, actor (enrolled principal), at, reason}` on the current entry
  and a `blp-declassification` event — it never modifies or removes the
  existing attribution (the recorded label stays the read authority;
  declassification explicitly authorizes those recipients for those exact
  bytes, it does not rewrite the object);
- stays subject to history reading: any prior `Declassification` of the same
  triple refused on re-issue under another event_id
  (`blp-declassification-conflict`) — exact-once applies here too;
- keeps `runtime_enforced:false` in the reply, and
  `declassification_available` reflecting the real state (currently `false`;
  flips to `true` only when this operation is wired).

### 3. Reader — `root_store/reader_guard.rs`

`check_read` keeps refusing by default for an attributed object;
declassification does NOT widen `check_read`: the classified-send path
(preparation/admission) consumes it, revalidating recipient+capsule as today,
with the current `Declassification` as an additional proof binding exact
recipient + exact bytes. No generic "read without a label" command appears.

### 4. Wiring — web + CLI

- `swarm-web/src/service/trust.rs`: add `"blp-object-declassify"` to the
  allowed POST list (mutating, not readonly);
- `swarm-cli/src/trust.rs` + FR/EN help in `lib.rs`: same addition, documented
  "enrolled human identity required, new copy, no label downgrade".

### Observable criteria (expected red→green test)

1. Local admin bearer + attributed confidential object →
   `blp-object-declassify` → `blp-declassification-human-required`, no
   mutation.
2. Enrolled human actor + exact recipients + bytes → records
   `blp-declassification`, attribution unchanged on re-read, reply
   `runtime_enforced:false`.
3. Re-issue of the same triple under another event_id → `409
   blp-declassification-conflict`.
4. Disabled or unknown recipient → typed refusal, no mutation.
5. A summary, an empty output or an LLM filter never produces this effect
   (the producer is callable only through this signed operation).

Limits retained: the classified-send path (BF-13/BF-15) remains the only
planned consumer; `blp-object-declassify` itself creates no filesystem or
network effect. Implemented + tested ≠ installed ≠ activated.

### Targeted global recipe on a single candidate — 9 November 2026

The acquisition re-examination requested by BLP-07: the key public permanent
recipes were re-run on **one single qualified candidate** (`a166e2b8…`,
profile `VERIFIED_DEFAULT_NATIVE_PROFILE` from the retained run3), evidence
`~/.local/state/swarm-fixtures/objattr-profile/global-{1,2,3,5,6}/`, each
re-verified **VERIFIED**:

- `blp-preparation-transport-public` PASS (global-1);
- `blp-dialogue-reader-public` PASS (global-2);
- `blp-preparation-check-public` PASS (global-3);
- `blp-preparation-admission-public` PASS (global-5);
- `blp-flow-inventory` PASS (global-6) after re-pinning `reference_count`
  69→74 (drift following the BF-16 wirings, same discipline as earlier
  repins);
- `blp-object-attribute-public` already VERIFIED on this candidate the same
  day (`recipe-run1`).

Gaps found: `blp-preparation-quarantine-public` **not re-run** — its driver
requires `SWARM_BLP_HELPER` (the compiled
`preparation_quarantine_fixture` example binary); the example carries the
uncommitted BF-16 caller adaptation and compiling it would pull Codex's
concurrent BF-14a diff into `swarm-engine`. The retained 1 Nov PASS
(`verification-blp/20261101-preparation-quarantine-public-02/`) stays
VERIFIED on its original candidate. `blp-preparation-provenance-public` not
re-run for the same reason (cargo steps). **Aggregated recipe delivered**: `tools/verification/recipes/blp-aggregate-public.json`
(6 child steps, one per recipe above, order inventory → check → admission →
dialogue-reader → transport → UI; fresh child outputs under the fixture area
via `configured_area()` + unique suffix, created by the child runner itself).
First pass on candidate `a166e2b8…`: **PASS 6/6 steps**, outer verify
**VERIFIED** + all 6 child evidence trees re-verified VERIFIED; `/tmp`
clean. Launch environment: `SWARM_BLP_BINARY` + `SWARM_BLP_PROFILE` (the
candidate under test); `SWARM_BLP_PROOF_OUTPUT` is allocated per step, never
inherited. One first-tuning defect (child root pre-created by `fixture_root`
refused by `recipe-output-must-be-new-outside-live-state`) was fixed in the
recipe itself; the refusal evidence is retained in the replaced
`global-agg-1` tree. Still outside the aggregate:
`blp-preparation-quarantine-public` and `blp-preparation-provenance-public`
(helper/compilation, concurrent diff) and the cargo-test recipes (concurrent
engine WIP) — the aggregate is the **public** global recipe on a frozen
candidate, not an engine requalification.
