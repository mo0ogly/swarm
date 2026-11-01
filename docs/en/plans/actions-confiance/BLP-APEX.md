# Bell-LaPadula — PAEA technical subplan

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
| BLP-07 | Retained recipes, candidate review, local delivery, served assets and RETEX | BLP-01–06 | PA-09/10/11/12/13 |

## Current technical state

BLP-04 remains partial: native v3 provenance, public object control and the
authenticated recorded-source diagnostic are verified and installed in296.
Producer source identity, original send-principal binding and output inheritance
remain open. BLP-07 retains global recipe/review/RETEX requirements. See the
verified-result section below for exact image/evidence.

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
  [verification-blp/20261031-ui-qualification/](verification-blp/20261031-ui-qualification/)):
  served page + embedded BLP asset, Admin dialog, typed
  FR/EN refusal messages, 4-level no-clearance example, save disabled without
  preview, theme sombre→etat verified by attribute, no horizontal overflow at
  390 px — 9/9 checks, no unexpected console/page/HTTP errors.
- Still open: RETEX; no independent acceptance. BLP-01 partial; BLP-04–06
  open. `runtime_enforced:false` and `runtime_authorized:false` remain the
  runtime truth; no real content classified, no protected execution.

RETEX written: [paea/RETEX-BLP-20261031.en.md](../../paea/RETEX-BLP-20261031.en.md)
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

The [RETEX](../../paea/RETEX-BLP-20261031.en.md) next experiment, executed: the
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
  [verification-blp/20261101-blp-positive-ui/](verification-blp/20261101-blp-positive-ui/)
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
[hashes and limits](verification-blp/20261101-public-producer-source-review.json).
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

[Byte-bound policy/function/error review](verification-blp/20261101-preparation-check-source-review.json).
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
