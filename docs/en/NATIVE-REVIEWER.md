# Native Rust reviewer: current contract

The native evidence below targets Linux; cross-platform equivalence is not established.

## Actual availability

Previews, reviewer controller, context reader, publication and reconciliation
compile in the normal Rust profile since increment162. `task review` invokes the
configured provider through the same executable's private helper; tests use only
owned inert programs. Web reviewer launch routes remain unported. Native planning
can act on a current verified passed review (increment165), close native scopes
under increment166 evidence, and retry the same task from a current native
correction verdict (increment168). The global conductor, other retry routes and
other protocols remain partial. This establishes no user installation, real independent judgment,
authenticated model identity or acceptance of real user work. Explicit native
task acceptance is verified on owned fixtures in increment164 below, separately
from global acceptance. Qualification162 covers frozen
compiler inputs and excludes concurrent telemetry changes preserved in the
shared checkout and the installed interface.

| Flow | Actual behavior |
| --- | --- |
| `task review-preview` | Read-only native control provenance/current configuration |
| `task review-inputs` | Read-only complete originals, private prompt and digests |
| `quotas show/preview/apply` | Mission limits/reviewer deadline; explicit apply writes |
| `engine-settings show/describe/preview/apply` | Saved global policies; preview is pure |
| `ReviewOutputStream::from_saved_settings` | Saved-policy decoding, no process |
| `planning claim` then `planning call-preview` | Selection and preparation of verified normal context, no model |
| `planning decide` with native review | Existing actions require current passed evidence; historical/non-verdict and other specializations grant no action |
| `task review WORK --input FILE` | Claim/reservation then one private invocation, original and verdict/non-verdict; never acceptance |
| `task review-reconcile REVIEW_ID` | Apply a known original or replay history; no new invocation |
| `task review-retire-stale REVIEW_ID` | Explicit non-verdict for a successful original that became stale; no new invocation |

Both task previews accept `schema_version`, `event_id`, `expected_revision`,
`task_id`. Caller-supplied actor or command assertions grant no authority. Public
preview refuses an ongoing review; internal readback permits only the exact review
attached to its normalized job.

## Native control source mapping

Before public conversion, original readback and replay, the engine verifies
the complete mapping from stored policy through ordered job slots to observed
results: receipt versions, count, original job, ordinal, unique identifiers,
control name and exact arguments. A foreign command result refuses with
`task-validation-receipt-slot-source-changed`; it cannot become a successful or
plausible blocked public artifact. Consistent failed and unattempted slots remain
recordable blocked observations. Missing or unsuccessful results without their
failed flag refuse, preventing blocked data with no attributed control incident. Success also requires attempted process proof,
recorded permission and an observed permitted slot, alongside existing wait and
result checks.

This data check never replaces actual hashes, SQL rows, permissions, identity
or completion evidence and grants no review or execution authority. The S2
cumulative preflight is separate: its future controller must map transport keys
to original tasks/controls and bind the physical candidate and normalized
policies. That wiring remains guarded and is not implemented here. Preserve
originals after a mismatch and reconcile through the supported path after
repairing its cause; never rewrite a receipt to obtain acceptance.

## Source origin of new control jobs

Every new control job records a scalar `native_validation_source_origins` row
in the same transaction as its job and Work: observed source, normalized-controls
digest and optional candidate binding. The job digest combines this origin domain
then existing mission authority. Permissions, publication and review readers
reuse that calculation. Removing or changing the row cannot reproduce the new
job digest. This grants neither execution rights nor model authentication.

Private job receipt **version 2** retains `source_origin` and `effective_policy`.
Readers require both; pure conversion already checks effective policy against
its digest. The Root reader also verifies the SQL row then the job domain with
recorded mission authority. Historical private receipts
**version 1** omit both fields and retain their encoding and legacy base without
read-time backfill. An incomplete v2 receipt cannot become historical. Public
JSON keeps its existing schema. Schema/type/hash/missing-origin/content errors
remain typed refusals; failed writes require caller rollback and never prove
that no rows were staged. Replay and reconciliation do not start replacement
controls for incomplete originals.

Candidate binding is **absent on the delivered ordinary path**. The managed S2
controller stays guarded: physical witnesses, cohort plan, complete review and
candidate publication still need wiring. Source observations and hashes do not
confine hostile processes running under the same user.

### Private cumulative receipts: v3 producer and reader

The reader supports a private v3 variant retaining the full candidate, hence
original/normalized policies per task and every slot. `controls_sha256` binds the
whole plan; `candidate_binding_sha256` binds the entire checkpoint preimage.
Readers check those contents, the original SQL row and Job domain, then original
slots/results. Coherently rewriting JSON with new hashes cannot replace the
recorded origin.

| Private receipt | Origin | Separate effective policy | Complete candidate |
| --- | --- | --- | --- |
| Historical v1 | absent | absent | absent |
| Ordinary v2 | present, no candidate binding | present | absent |
| Cumulative v3 | present, with candidate binding | absent, plan in candidate | present |

Mixed combinations and unknown versions refuse, without downgrade or backfill.
v1/v2 retain their exact bytes; the ordinary producer remains v2. The ordinary
source reader used by permissions/reviews explicitly requires no candidate
binding; the managed producer uses a separate source path checking the complete
preimage, current plan and actual connected Root instance.

**Data fixture example**, not managed execution: accepted task A and current B
each have a `check` control. Transport keys differ, original names remain and no
aggregate policy ceiling is added. If A fails and B passes, public controls stay
B-only but overall state is `blocked`, with A's task/control/transport attributed
in failures. The private receipt retains both observations. Conversion also
requires nonempty command arguments without NUL; two matching invalid values
cannot become proof of execution.

Readers compare normalization shape against only the normalizer's transformations
(trim, authority removal, positive resolution of an originally zero timeout),
without loading current limits or inventing historical limits. Actual cohort
membership and normalization under saved settings still belong at claim time.
Hashes and DTOs prove no permission, native owner, open descriptors or current
Git integrity. **Root still refuses managed launch: the cumulative job producer,
permissions, complete final review and publication remain unwired.** Physical
custody and planning exist as private components; this reader does not enable them.

## Originals, policies and capture

The prompt contains the selected task, canonical submitted report, a different
explicit Markdown deliverable under `docs/` when applicable, native control receipt
and explicitly shareable complete outputs. Markdown links are not traversed;
unauthorized private output is not added. Changed reports, missing declared files,
required PNG/JPEG images or unported positive client windows refuse. Partial outputs
remain attributed as partial, never complete quotation sources. Material count/bytes
include separately quoted outputs as well as their receipt; prompt accounting
includes workflow and JSON escaping. Content is never clipped to pass a cap.

| Policy | Saved source and semantics |
| --- | --- |
| Reviewer context/files/sources | Mission quota editor: zero removes the additional cap; absence requires configuration |
| `review_delivery_max_bytes` | Global settings: zero forbids nonempty files; null removes the cap |
| `review_response_max_bytes` | Global v29 Quality/review group: zero forbids nonempty replies; initial null explicitly unlimited; legacy absence requires adoption |
| `review_timeout_seconds` | Mission quotas, root request field: zero removes total duration; omitted/null preserves current value |
| `agent_capture_max_bytes`, `agent_capture_max_lines` | Global combined stdout/stderr retention; zero retains no content, null removes that cap |
| Poll, grace, drain, startup and storage | Common saved global policy, no second local reviewer ceiling |

These policies use existing common validation and Admin/API/CLI surfaces; adoption
measures neither actual price nor model capacity. Browser exact-integer limits do
not become API/CLI business ceilings. Unrepresentable clock durations refuse.

Capture retains prefixes under combined budgets while counting/hashing every
observed byte, including after intentional truncation. Unfinished lines continue
between chunks per pipe; the combined budget follows observed order, without
inventing a global stdout/stderr timeline. Representation/allocation/IO/parser
failure differs from truncation. Every stdout byte feeds the decoder under its
separate response cap, even with zero raw retention; stderr stays outside JSONL.
Flush proves neither EOF nor fsync.

The exact libtest header filter and finite evidence Vecs are SDK devices. Product
transport strips no header and inherits none of their fixture sizes.

## Candidate lifecycle and private evidence

```mermaid
flowchart LR
  A[Native originals and controls] --> B[Complete prompt and saved policies]
  B --> C[Claim: call, financial reserve, scalar ledger]
  C --> D[Child registration then permission, two commits]
  D --> E[Verified Unix peer and single sealed exec]
  E --> F[BOOTTIME, source checks, live stdout/stderr]
  F --> G[Actual wait and create-only private receipt]
  G --> V[Fresh verdict: passed or changes_requested]
  G --> X[Waited failed call: non-applicable error]
  V --> H[Certificate then Work, inbox, event and ledger transaction]
  X --> H
  H --> C2[Content-addressed certificate2; committed SQL reference]
  C2 --> N[Normal reader: original, event, certificate1 or2, inbox]
  N --> P[Public claim and preview: prepared context]
  P -. Not established .-> M[Model reception, decision and application]
  G --> R[Departed owner: known wait, same host and confirmed absence]
  R --> H
  G --> S[Waited success: observed dependency change]
  S --> H
  G -. Pending .-> U[Missing original or unknown wait]
```

Claim atomically reserves the review record, call and financial estimate. The
`native_review_jobs` ledger uses scalar columns; this does not migrate the main
Work BLOB. The last reserved call remains usable (`0 < calls <= max_calls` with
verified job/funding), without permitting another exhausted claim. A committed
permission can never return to runnable state.

Root/dev/inode coordinates differ from workspace and host/environment scope.
Owner/child derive from actual OS and Unix peer observations, not JSON PIDs.
Registration and permission are separate commits. The channel is CLOEXEC; unnecessary
parent Stdio copies close after spawn; sealed transport is consumed once by exec.
EOF alone proves no success. Private host preparation creates, syncs and removes
a workspace probe; its IO/cleanup effects survive SQL rollback. Public previews
perform no probe and establish no OS confinement.

Total BOOTTIME begins after spawn and includes helper admission. The original TTL
protects admission; it is neither renewed nor reused as provider duration/result
expiry. TTL expiry during an otherwise successful review is not a measured scenario
in current evidence. Once the leader exits, admission/total timers stop and only
saved drain remains. IO/checks are cooperative, not real-time; observations retain
maximum check latency. The controller preserves typed causes, actual wait and any
unreaped Child: serialized JSON cannot replace them.

The framed private receipt is synchronized and **create-only** before terminal SQL.
Version2 requires actual capture; version1 remains readable with unknown capture
and cannot satisfy publication/reconciliation2.

| Field | Meaning |
| --- | --- |
| `process_attempted` | Primitive invoked, not by itself fork/exec proof |
| `grant_attempted` | Grant byte may be exposed, not model identity |
| `waited`, code/signal | Known wait; absence stays unknown |
| `failed` | Call/transport/domain failure, distinct from a valid unfavorable verdict |
| `original_reply` | Exact capped candidate, even invalid; absence differs from SHA(empty) |
| `raw_capture` | Saved policy, prefixes, all-byte counts/hash, separate truncation/failure |
| `provider_facts` | Unverified declarations; absence is not zero cost |

Schema/time/Root/checksum and field consistency are checked. Nontruncated hashes
and line counts must match. Wait without process attempt refuses; partial failures
never become success. These checks establish no business freshness or cryptographic
authenticity. Uncertain write/fsync requires readback, never another provider call.

## Verdicts, errors and reconciliation

`publication::record` rereads receipt2, requires the current original owner and
absent waited child, verifies funding, exact settings generation and complete
materials, then strictly decodes the retained reply. It invents no JSONL/usage/cost.
Work revisions may advance but cannot precede the claim; original intent/hash keep
the original revision. Independent checkpoints remain compatible; changed sources
refuse. Actual dependencies govern freshness. Pause/resume renews the native authority event included in the control source
digest, so old controls remain stale. Scope lease generation is a separate
observation excluded from this digest; an ordinary claim/decision alone does not
revoke unchanged controls.

`passed` leaves the task `submitted`. `changes_requested` blocks it for correction
and keeps `failed=false`: it is not a failed call. No verdict accepts a task.

`publication::reconcile_failed` is distinct: exact failed receipt2, committed
permission, current original owner, known wait and absent child. It preserves
production status, records `error` without applicable criteria and moves the job
to terminal `refused`. An actual second OS process refuses while the original
owner lives; a valid unknown-wait receipt refuses even if the PID appears absent.
It adopts no owner, repeats no provider, refunds nothing and guesses no wait.
Native exclusion ends, but another claim for the same production attempt still
refuses; explicit reviewer retry for that same attempt remains unported.
Complete causes remain private; public reason uses safe diagnostic vocabulary.
Potentially changed sources do not become a fresh verdict.

The compact public certificate has six fields: `schema_version`, `review`,
`work_id`, `task_id`, `source_sha256`, `validation_receipt_sha256`. It excludes
PID/host, raw capture and provider facts. Format2 also omits `review.receipt`
and `review.receipt_sha256`; the complete body SHA256 determines
`.swarm/native-review-public-v2-<sha>.json`, attached after encoding.
The reader also accepts historical format1, named by job hash and containing its
own path (without its own digest). This representation is no operator setting.
Compatibility is verified on immutable15804 inputs: two wire tests, genuine v1
publication and v1 rollback/v2 commit, followed by normal08 public claim/preview
for both formats. Permanent replay: `native-reviewer-certificate-compat.json`. Producer/readers share the format to derive the exact
public read extent, without an arbitrary operator ceiling. Private framed bytes
are still loaded whole, while unused capture arrays are skipped during decoding:
no global memory bound is claimed.

Create-only/sync file, then one Work/inbox/event/ledger transaction: rollback may
leave a certificate without an applicable result. Recovery requires identical
bytes for the same filename. Different format2 bodies may coexist after rollback;
only the reference bound to the committed SQL result is authoritative. Existing
artifacts are neither replaced nor deleted. A second check follows IO, without a
filesystem lease. One connection and
saved storage policy govern mutation. Uncertain commit permits no retry of the
provider; late errors retain known Work revision and original causes. Replay
rereads private original, `review.result`, phase/revision and public certificate;
absent reply digest differs from invalid type. Replay renews no freshness or rights.
Financial estimate remains held; provider settlement is still open.

## Normal consumption and evidence limits

`native_review_context` opens a readonly Root/SQL snapshot and binds terminal ledger,
Root-anchored private original, immutable result, certificate and verdict/error.
Result payload is a review, not a historical Work. Routing matches current inbox,
task owner and original attempt; no historical scope snapshot is invented. Reading
requires no still-live owner and acknowledges/mutates nothing.

Verdict context includes review and complete report reread under saved report cap.
Delivery references contain report/certificate; task projection includes its review
for context freshness. Error context contains review/certificate and explicitly sets
`unavailable_reports[event_id] = native-review-failed-no-applicable-verdict` in both
context and delivery. No current file is loaded as an applicable failure verdict
report. Event/context caps apply without silent omission. Unported managed/legacy
review coordination still refuses.

Normal public claim/preview verifies selection, context digest, actual preparation
and preview purity. `--expected-review-state error|stale` additionally requires state and
inapplicability. This establishes no model reception, decision, application, product
adoption or acceptance. Increment165 separately verifies an active native decision
from a current passed review; preview alone still proves no such effect.

Digests do not protect against a hostile same UID rewriting the entire Root/history.
Authenticated model/human identity, OS confinement, billing, Web routes, active
supervisor decisions, missing/incomplete journals, live/unwaited children and
other unsupported source states remain open. Documentation does not complete them.

## Replay and verification

| Evidence | Retained scope |
| --- | --- |
| [155 final10](../plans/migration-rust/evidence/20261022-155/final10/result.json) | Independent checkpoint, changed-source refusal/recovery, stale pause/resume; no active reviewer during checkpoint |
| [156 final10](../plans/migration-rust/evidence/20261022-156/final10/result.json) | Four actual failed calls, rollback/recovery/replay, real foreign-owner refusal, unknown-wait refusal, explicit incident |
| [156 normal17](../plans/migration-rust/evidence/20261022-156/normal17/result.json) | Normal c546 binary,255archived effective inputs, no fixture feature; not installed |
| [156 public18](../plans/migration-rust/evidence/20261022-156/public-context18/result.json) | Normal claim/preview, error/inapplicability, no model/acknowledgement |
| [153 capture](../plans/migration-rust/evidence/20261021-153/units04/result.json) | Nine capture guards; this module's sources are unchanged by156 |
| [Current plan](../plans/migration-rust/PLAN.md) | Current valid-publication regressions/quality, original failures, limits and next work |

Permanent recipes under `tools/verification/recipes/`:
`native-reviewer-claim-candidate`, `native-reviewer-controlled-process`,
`native-reviewer-private-journal`, `native-reviewer-streaming-capture`,
`native-reviewer-publication`, `native-reviewer-revision-continuity`,
`native-reviewer-failure-reconciliation`, `native-reviewer-departed-owner`,
`native-reviewer-certificate-compat`, `native-reviewer-stale-original`,
`native-reviewer-budget-result`. [Replay instructions](RUST-MIGRATION-REPLAYS.md)
record commands/preconditions. Fixtures are isolated and inert; product deadlines
differ from the absent global watchdog.

SDK/normal executables are qualified separately by features/profile, actual depfiles
and byte hashes, then archived. Concurrent used-source changes refuse qualification.
Use `--sdk-source-root` for the verified retained snapshot; never extract evidence
over checkout, toolchain or a live Root. Relocation and dependency availability are
separate preconditions. Fixture tokens prevent accidental mixups, not hostile UID
forgery. Preserve original refusals; reading historical evidence does not rerun it.
These frozen proofs do not qualify Claude's current shared checkout or installed
server. Detailed chronology stays in PLAN; prior FR/EN pages are retained in
`156/native-reviewer-doc-before14.tar.gz` with their hashes.

## Recovery after owner departure

`publication::reconcile_departed` separates original execution evidence from the
recovering instance. It requires a known-wait original2 on the same host:boot:PID
namespace. The old PID must be absent or have another start stamp, and the waited
child must also be absent. Live owner, changed host or uncertain observations
refuse. No signal, guessed reap, new grant/exec or owner/child scalar replacement
occurs.

A healthy original follows the same complete material/funding/settings-generation
fence, strict decoding and atomic publication as the current-owner path. A failed
original follows failed-call non-verdict finalization. Owner boundary is checked before
preparation and after IO; it is lifecycle evidence, not a disableable operator
policy. Another process also refuses while the original owner lives.
`inspect_departed` is an observation, never a reusable permit. Terminal replay
is historical and reasserts no current departure/freshness.

Recipes observe the applying OS instance, but its agent identity/authority are
not authenticated or durably bound to the business result. I157-ACTOR remains
required before a complete traceability claim. Missing/legacy originals, unknown
wait and live children still refuse. Permanent replay:
`native-reviewer-departed-owner`; exact evidence/scope are in the plan.

## Successful original becoming stale

`retire_stale_departed` is explicit under the same confirmed-departure guard; it
does not replace verdict publication. A successful/waited receipt2, committed
permission, actual child absence, funding and exact running review are required.
Canonical submission and original attempt must match. Preparation requires an
observed change: settings generation (existing strict verdict contract), inactive
authority, successfully read report with a different hash, recomputed native
source, or successfully observed complete inputs. Unreadable data, invalid policy
and other errors still refuse; no catch-all becomes a result. Closed scopes and
incompatible task states may still refuse before complete observation.

A second check after IO precedes atomic publication of `stale`, without criteria
or applicable report artifact. Task status, original timestamps and financial
estimate remain. Normal context uses `native-review-stale-no-applicable-verdict`.
The non-applicable certificate2 may coexist with a rolled-back verdict candidate.
Replay proves the recorded historical outcome; restoring the report does not make
that old outcome applicable. No new execution, automatic retry, acceptance,
settlement or authenticated agent identity is established.Four variants15909 and their public contexts passed: report, settings, pause and
renewed authority. Six recoveries12, four errors11, eight publications13, three
scope units15 and Clippy/Rustdoc14 passed. Inputs are immutable15904; normal08
SHA9c20b0 compiles255sources identical in04 (proof10), without fixtures or
installation.

The stale outcome retains its cause family and original bindings, not a separate
signed snapshot of changed sources. The reader verifies recorded integrity, not
that historical cause against current files. Persistent comparison evidence and
actual applying identity/authority remain part of I157-ACTOR before a complete
traceability claim. Missing/legacy/unknown-wait originals still refuse.

## Result reservation versus a new start budget

An already executed original needs no new reservation at today's tariff. Result
validation requires one correct work/scope/job row within the caller transaction,
a finite nonnegative stored amount and estimated/unobserved state. Preserve that
stored amount: no repricing, debit, refund or invoice inferred from provider text.
Missing/invalid reservation and SQL/type/close errors refuse, never empty success.
Trust in the authoritative SQL row proves no independently signed historical
price or actual settlement. Already observed/settled rows remain unsupported by
this conservative result guard.

`result_inputs` and `retirement_frame` use historical result validation; applicable
verdict source/settings checks remain. Admission, new start and live observation
retain current tariff/limit checks. Admin/CLI budget changes govern later calls
without rewriting reservations or granting execution. The reviewer uses this guard in the normal profile since162. The result financial guard also serves the
normal supervisor-original path since161; common financial reads/start
validation remain normally compiled. Web wiring and active decisions remain open. 16002 reproduced the counterexample;
two results/public contexts09, five normal start guards07, four stale10, eight
publications11 and Clippy/Rustdoc12 passed on05. Normal06 SHA cc2e88 compiles255
effective sources identical in05 (08), without fixture features or installation.

## Run and recover through the normal CLI

`task review WORK --input FILE --root ROOT --json` accepts the four preview fields.
Its `event_id` must differ from a validation or another mutation; exact replay keeps
this ID and never reruns the model. The engine selects its own `_reviewer_gate`
image; callers supply no executable. Unix socket, observed OS identities, sources
and sealed permission are checked before exec. Regular stdin refuses without
writing into that file. No libtest header filter processes provider stdout here.

A `passed` verdict leaves the task submitted; `changes_requested` blocks it for
correction. A failed invocation remains a CLI error even if its original permits
recording an `error` non-verdict. Explicit
`task review-reconcile REVIEW_ID --root ROOT --json` may acknowledge this historical
outcome with `verdict_applicable=false`; that does not make the initial invocation
successful. Replay renews neither freshness nor authority.

Before nonterminal application, the caller must be the exact original owner or
prove its departure on the same host; no live foreign owner is adopted.
`task review-retire-stale REVIEW_ID` requires a successful waited original and an
observed changed dependency; unknown IO is not staleness. Insufficient grant/wait/
journal evidence continues to refuse recovery, preserves the reservation and
never implicitly permits another exec.

Permanent replay `native-reviewer-normal-cli.json` checks seven normal cases,
retains original frames, certificates, programs and channels before PASS, and
checks corruption refusal then exact restoration. An inert program and observed
Linux identity authenticate no agent or human. I157-ACTOR, Web integration,
acceptance and settlement remain open.

## Explicit native task acceptance: verified scope164

The normal CLI can prepare `task update WORK --input FILE --root ROOT --json`
with `schema_version`, a distinct decision `event_id`, current `expected_revision`,
task `id`, `status: "accepted"` and optional `next`. This separate path does not
add acceptance to the worker API restricted to running/blocked. It requires a
submitted worker task in an open scope, with no contract, owner, blocker or
attempt-outcome edit. A changes_requested/error/stale review never suffices.

Root verification rereads the task/current SQL revision, review job/result event,
private original and exact certificate. It also uses the control digest bound in
that original to reread private control job/step frames, rows and control result
event/certificate. This digest comes from the verified Root reader, never acceptance
JSON. Certificate namespace selection only chooses a protocol: declared/malformed
names cannot fall back to ordinary supplied reviews. The API without current
settings refuses native data.

Current report, artifacts, contract, project context, quality policy, gate and
prerequisites remain required; review data is checked again after preparation IO.
The shared current reader applies the same native verification to dependencies.
Historical control evidence is revalidated as a recorded original, without granting
a new execution under current settings. Ordinary/managed read paths gain no
authority; their hierarchical acceptance is outside this native path.

```mermaid
flowchart TD
  L[Current SQL task and revision] --> V[Verify before explicit acceptance]
  R[Review job and result event] --> O[Private reviewer original and certificate]
  O --> H[Control digest bound in original job]
  H --> C[Reread private control job and steps]
  C --> E[Control rows and event and certificate]
  E --> V
  O --> V
  P[Current report contract context gate prerequisites] --> V
  V --> A[This task transitions only when all checks pass]
  V --> F[Refuse without mutation if evidence is missing or changed]
```

This transition creates no call, refund, invoice, mission acceptance or human
identity. Current local access proves no authenticated human or agent authority;
I157-ACTOR remains open. Tests use inert programs/owned fixtures, without user
installation. Recipe `native-reviewer-task-acceptance.json` must require a valid
control, private/public corruption refusal at both stages and changed report,
exact restoration, non-verdict refusal, and reservation/counter/original retention
before PASS.

The existing protected `POST /api/v1/action` route with kind=task uses the same
Root check. Two actual HTTP fixtures verify acceptance, 400 refusal on a changed
reviewer original without mutation, restoration and historical replay without
invocation; only their owned servers were started/stopped. Replay acknowledges
history without new validation or authority. No browser workflow is claimed:
this increment changes no layout/UI.

## From review to follow-up work: verified scope165

An independent_review event is no permission. Before existing active operations,
the engine rereads the exact scoped inbox event, native result, both stages of
originals/certificates and current passed evidence for the same task/attempt.
It also reads the explicitly referenced report under its saved cap, without
pretending it is a handoff event. Existing task, requirement, delegation, budget,
artifact, lease and commit guards remain. The recipe exercises task creation;
it is not an exhaustive delegation campaign.

Context preparation already validates this evidence when selecting the current
passed review, before a new claim or invocation. Archived reviews and error/stale
incidents remain attributed observations for diagnosis, without action rights;
changes_requested is no favorable verdict. Explorations, run observations and
other unported protocols refuse.

```mermaid
sequenceDiagram
  participant C as Recipe commands
  participant E as Rust engine
  participant P as Inert planner program
  participant W as Inert worker program
  C->>E: planning step after native acceptance
  E->>E: Verify current review and originals before claim
  E->>P: Sealed prompt and private permission
  P-->>E: Propose dependent task
  E->>E: Actual wait, retained original and decision fences
  C->>E: agent prepare for follow-up
  E-->>C: Refuse damaged upstream control without reservation
  C->>E: Exact restoration then prepare and run
  E->>W: Start under current evidence and budget
  W-->>E: Distinct report and public handoff
```

Dependent worker admission rechecks accepted-task evidence including private
reviewer/control originals: an old accepted status alone cannot authorize start.
The follow-up remains submitted; no implicit acceptance or human identity is
created. Recipe commands and inert programs drive this flow; it proves neither
global conductor autonomy nor model intelligence. Scope closure still needs a
true physical completion boundary, distinct from an expired lease or request flag.

Recipe: `native-reviewer-active-decision.json`. Planner/new-worker originals,
programs, prompts and reports are retained before PASS. work show rendering may
itself refuse unreadable accepted evidence; the recipe compares read-only
revision/reservations during refusal and complete Work after restoration.
Partial observation improvement I165-OBSERVE remains planned; no empty success
or fictitious freshness display is introduced.

Two active negative variants also run a real inert planner and retain its original:
changes_requested and a reviewer protocol error refuse follow-up creation. Reading
them grants no action; refusal preserves existing reservations and reruns neither
reviewer nor worker.

### Native scoped closure

The `close` implementation is verified on the frozen normal fixture graph in migration lot166 (three public flows, eleven named tests, Clippy and Rustdoc); this is not an installed delivery. It is a sole operation: task/delegation edits cannot be combined with closure. All pending scope events must be consumed, children closed, descendant tasks accepted with current original review/control/report/gate/dependency evidence, and own requirements covered. Prompt `closure_unavailable` describes structural eligibility only, never a physical completion permit.

WorkEngine checks registered native planner, review, validation and worker ownership in its decision transaction. A default-deny internal fence admits only the finishing planner whose healthy durable original, exact source/owner, hash and observed absent child are verified. Unknown wait, live foreign work or changed original refuses. No body flag, expired lease or caller PID authorizes closure. The applied run/usage and closed scope share one commit; parent receives `scope_closed`. This does not establish escaped descendant containment, authenticated human/agent identity, full migration acceptance or autonomous orchestration.

```mermaid
sequenceDiagram
    participant P as Native planner
    participant R as Root original journal
    participant E as WorkEngine transaction
    P->>R: Waited result and durable original
    R->>E: Private run fence and proposal
    E->>E: Pending inputs, descendants, current evidence
    E->>E: Physical ownership and exact finishing original
    E->>E: Close scope, parent event, run and usage commit
```

An unconfirmed physical closure guard remains a typed incident, not a terminal business refusal. It must preserve the original and ownership for reconciliation; missing grant observation cannot release a run. Lot166 retains the pre-fix counterexample separately.

### Scoped worker restart gate

A blocked scoped worker cannot start another attempt without the persisted engine retry signal. The common lifecycle and public admission share this guard, preserving the task attempt maximum; review error/running/queued requires review recovery. This signal is not authenticated supervisor evidence and cannot be supplied as a launch-request override. Native changes_requested production retry uses only the verified correction path below; other review/protocol/managed recovery routes remain unsupported. Manual task behavior is unchanged. Lot167 distinguishes the public branch refusal (no demonstrated bypass) from the reproducible missing common lifecycle precondition.

### Native production retry (migration lot168, fixture verification)

The common reader separates passed acceptance from a current native changes_requested verdict used only by same-task `retry` operations. Private reviewer/control frames, canonical events, public certificates, report/contract/project and current attempt remain mandatory. A retry changes only the persisted correction/signal; it preserves attempts, review and consumption. A signalled start additionally requires the stored planning.decide retry to match task/scope/correction/original review input. Missing or overwritten evidence refuses. The normal CLI fixture verifies a real second worker on the same task after this decision; review protocol errors/managed recovery grant no production retry.

The native worker retry path in increment168 includes a historical-original check at every native execution preflight: exact archived review and preceding completed attempt, canonical correction decision, current report/contract/project, and original control ledger/private slots/public event remain bound. It selects the control by the verified review body's SHA, work and task, and verifies prior producer/attempt; it never fabricates a current validation/preview. The current acceptance readers still require exact current validation data. A complete healthy control and fault/recovery case verify actual execution, refusal before preparation, and refusal at queued preflight after archival; restoring the same original lets the same queued agent run.

The canonical correction remains in task.next through native admission and preflight; the worker prompt includes it. The normal lifecycle archives the old review and clears its validation/gate when it records the new attempt; it creates no successful new review. The fixture preserves handoff.md and submits retry-report.md separately. The second report remains submitted and unaccepted, with no second reviewer/control claimed. Review protocol recovery remains unfinished; worker failure without review is verified separately in lot169.

```mermaid
sequenceDiagram
    participant P as Native planner
    participant W as WorkEngine
    participant R as Original chain and decision
    participant A as Agent admission
    participant G as Native preflight
    participant X as Actual worker
    P->>W: retry same task with new correction
    W->>R: Current native changes_requested and original controls
    W->>W: Commit signal, correction, decision and run usage
    A->>R: Recheck current originals and canonical decision
    A->>A: Archive prior review, new attempt and reservation
    G->>R: Verify archived review and exact prior control frames
    G->>R: Current contract, report, project and canonical correction
    G->>X: Owned process grant after actual preflight
    X->>A: Actual wait/completion and separate report
    Note over A,X: Submitted result requires new validation and review
```

Replay: `native-production-retry.json` runs a healthy actual control before negative cases. It verifies an unsupported public scoped edit refusal, then a fixture-only negative canonical event corruption, damaged control before admission, and damaged prior control at queued preflight. Exact restoration precedes resumption, and both planning-call and worker-reservation ledgers are checked separately. The same preflight function is wired at all native stages; this recipe injects the execution fault before owner claim, not between registration and grant. SHA/receipt/SQL checks still authenticate no semantic agent or human and provide no same-UID hostile-writer or escaped-process containment guarantee.

Worker termination without review and the exact readonly original completion contract are described in [Native worker recovery](NATIVE-WORKER-RECOVERY.md); lot169 is verified on isolated native fixtures.

A producer launch while a review is error/running/queued refuses with `scoped-worker-review-recovery-required`, even without a production retry signal. The diagnostic grants neither a new review nor production: explicit reconciliation/recovery and original evidence remain required. A native error may keep its report submitted; this diagnostic fabricates neither a blocked transition nor a new producer attempt.

## Native protocol retry — decision verified on fixtures (lot172)

The route distinguishes an original typed `review_fragment_protocol` failure with known wait/grant/channel, exit0, healthy capture and no duration/drain/source interruption from an applicable verdict. The latter stays false. The shared reader also verifies current report/contract/project and original controls; other failures/stale/unknown results remain unavailable for active operations. Context carries only task/review/attempt IDs, reviewer_protocol ownership, correction type and private original-body SHA (SR1 framing verified separately), plus the actual verified report within saved limits. candidate_commit stays empty for this non-Git route. Free-form error text grants no authority.

`review-retry` proposes a new reviewer correction on the same task and its current delivered event. The decision preserves report, attempt, error review, controls and spending, updating only next/decision/acknowledgement. Persisted capacity, applicable fields, scope, evidence and absence of concurrent owners are checked. It launches no worker or new review, refunds nothing and creates no acceptance or future success. A second reviewer execution and already acknowledged incident recovery still need implementation/verification; this stage is not complete recovery. Decision only is verified172; complete recovery remains open.

```mermaid
sequenceDiagram
    participant O as Verified native original
    participant P as Scope supervisor
    participant E as Deterministic decision
    participant R as Next review still to implement
    O->>P: Attributed protocol incident and SHA
    P->>E: review-retry with new correction
    E->>O: Check original, controls, report and contract
    E->>E: Acknowledge, preserve history, update next
    Note over E,R: This stage does not launch a new review
```

## New review after correction — verified on fixtures (lot173)

Native recovery observes either the current error before a new claim, or the immediate archived same-attempt error behind a checked running job. It binds private/public/event originals, canonical correction/acknowledgement, current report/contract/project and original controls. No copied Task or fabricated preview removes the error to pass guards. The old review is archived only after checks, with normal new reservation and call accounting.

`TaskReviewInputs.admission_source_sha256` is an additive observation: the current source/admission-authority digest. It stays distinct from `preview.validation.source_sha256`, the original control execution source digest. A new supervisor generation does not rewrite those controls; new admission never reuses their old grant as fresh authority. Job and guards bind current admission and the original receipt separately. The `review_recovery` directive enters the actual JSON/prompt and its SHA; saved prompt limits cover it, it supplies no criterion evidence and it grants no permission. Source/project/registry/funding/host/preflight are rechecked; instructions must satisfy current policy too.

173 verified: actual second reviewer, four pre-admission faults refused/restored, new result initially submitted then separate fixture operator acceptance. Already acknowledged incidents on a fresh activation, transport/source failures and changed source remain separate contracts. This binding establishes no semantic actor identity, hostile same-UID protection or OS confinement.

An ordinary decision/claim does not revoke control authority: source_authority tracks enable/pause/resume events, not every display generation. For new origin-backed jobs, the control SHA also binds its job and effective policy and differs from the admission base. Equality remains possible for historical originals without origin. The173 public pause/resume control proves the distinct case: new admission digest, original control digest/receipt preserved, actual second verdict followed by separate fixture operator acceptance. This establishes no human authentication. Fixed framing describes recovery as attributed response-method/format guidance, outside criterion evidence and without extra tools or permissions.

```mermaid
sequenceDiagram
    participant P as Supervisor
    participant E as Engine and admission
    participant O as Original error and controls
    participant R as New reviewer
    participant H as Fixture operator
    P->>E: Canonical review-retry and correction
    E->>O: Original, controls, report, contract, acknowledgement
    E->>E: Fresh admission source, funding and new claim
    E->>E: Archive old review, preserve attempt and controls
    E->>O: Reread archived origin at preflight stages
    E->>R: Bound prompt with guidance outside criterion evidence
    R->>E: Actual wait and new verdict
    E->>E: Submitted, never automatically accepted
    H->>E: Explicit acceptance using current evidence
```

## Recovery after acknowledgement — verified on fixtures174

Acknowledgement records reception and does not resolve an error. On a fresh activation, the candidate separately includes still-current acknowledged review incidents, their original SHA and verified report. These never become input_events. Saved context, event and report limits still apply; overflow explicitly refuses. The new decision consumes only fresh inputs. Before admitting a new review, Root reads the immutable native_planning_claims context, its canonical claim event, scope/holder/generation/revision bindings and actual acknowledgements. The old decision remains intact. Verified on normal12 (47a701,274 sources without fixture features): full control22, seven exact-restored refusals and public context cap23, same attempt/report/controls/spending. The actual prompt receives the new correction ID and SHA, distinct from the preserved old acknowledgement; replay starts no process. Fixture operator acceptance remains separate and proves no human identity. Tests33/Clippy/Rustdoc10, final documentation14, regressions16/18/19/20. Original failures remain retained; no installation or full migration completion.

```mermaid
flowchart TD
    A["Acknowledged incident, current error"] --> C["Frozen context: original incident and acknowledgement"]
    B["Fresh operator input"] --> C
    C --> D["Decision consumes fresh inputs only"]
    D --> E["Root rechecks immutable claim and context"]
    E --> F["New review, original evidence preserved"]
```

175 verified on fixtures: a terminal signal with confirmed wait with incomplete stream/no reply, healthy capture and typed stopped-stream cause can classify as review_fragment_interrupted/reviewer_runtime. Unknown/expired duration, drain or source, broken capture and EOF alone remain unqualified. Classification grants no rights; unchanged current evidence, explicit correction, capacity, budget and fresh admission remain required. Signal15 is fixture input, not operator policy or product cutoff. Normal07 d6f5d6/274 sources without fixture features,34 tests/Clippy/Rustdoc05. Full flows08/10, four exact-restored faults09 and seven11 plus restored public context cap PASS: pending then acknowledged incident, actual new reviewer receives canonical correction/digest, same attempt/report/controls/spending and old acknowledgement; new reservation/call counted, result initially unaccepted then separate fixture operator acceptance, no-exec replay. Protocol regression12 PASS. Actual original failure01 and refusal02 remain retained; no installation, authenticated identity, unknown-wait recovery or global migration completion established.

```mermaid
flowchart TD
    A["Native original + inapplicable result"] --> B["Verified wait, grant, channel and capture"]
    B --> C["Invalid response protocol"]
    B --> D["Terminal signal + incomplete stream, no reply"]
    B --> E["Unknown, expiry or changed source: refuse"]
    C --> F["Typed routing, no new permission"]
    D --> F
    F --> G["Explicit canonical correction"]
    G --> H["Current candidate and controls, new-review funding"]
    H -->|"failure_code / failure_owner (176)"| I["New review, no new producer"]
```

176 verified on fixtures: review_recovery adds failure_code and failure_owner built from the same verified private-original classification. These observations explain the cause to the new reviewer; they are neither request inputs, arbitrary error text, authenticated identity nor permission. Eligible classes and admission checks from175 stay unchanged. Normal04 c8810c/274 sources without fixture features and34 tests/Clippy/Rustdoc02. Actual prompts05/07/pending09 assert both exact code/owner pairs; nine faults06/08, including wrong kind and wrong owner with recomputed hash, refuse before claim, restore then succeed. Same history/canonical/controls/report/spending, public cap preserved and no-exec replay. Both fields count toward the existing persisted prompt limit. No new permission profile, identity or installation.

177 verified on fixtures: root judge/continue and renew are separate from independent candidate review. One verdict per cycle, renewal only after continue under saved cycle quota (0 unlimited), no activation/decision/spending reset. Root context carries complete JudgeState/history under the existing context cap and its freshness binds this contract; children receive no root history. New reasons use planning_decision_max_reason_bytes; durable consistency must not reapply an old4000byte policy. Judge end still refuses without native managed-candidate contract. Contract, diagram, policy and evidence: [native global judge](NATIVE-GLOBAL-JUDGE.md). Normal06/47tests/control11/faults12/child13/legacy14; no full conductor or installation claim.

## Explicit native managed-candidate preparation

`swarm-rust task validation-candidate WORK --root ROOT --input request.json --json` uses the same strict request as `validation-preview` (`schema_version`, `event_id`, `expected_revision`, `task_id`). This is explicit Git preparation; `event_id` is correlation and a possible future launch identifier, without creating a Root event. An existing job for that identifier or an active controller refuses before Git: this operation does not replace job reconciliation.

Root rereads the Work, canonical submission, actual completed native producer, latest attempt, authorized policies and report bytes. Policies for the current and already accepted tasks are normalized separately with persisted settings. After checked SQL transaction and settings-reservation releases, the engine verifies the retained result, prepares the single-parent candidate and pins its directories. A second Root observation must match the first, including connected database identity, Work and the complete saved settings document. Any concurrent settings change refuses this preparation, even if irrelevant to future controls; this conservative interval check is not a business stopping policy. Descriptors are explicitly closed on every post-pin outcome.

The public result includes commit, complete candidate digest, producer/attempt and cohort counts, without private paths or commands. `job_created`, `controls_started` and `task_accepted` are `false`. Repeating unchanged preparation reuses the checkpoint and copy; errors may leave retained objects, a ref or files and never imply rollback. No job, permit, control, review, acceptance or Work candidate-pointer change is created. Managed `validation-preview` now inspects an existing candidate without preparing a new stage; `validate` uses the executor below. Final managed review still refuses (`review-managed-candidate-unported`). This CLI stage is not yet an automatic conductor launch or a UX action.

```mermaid
sequenceDiagram
    participant CLI as CLI validation-candidate
    participant Root as Root SQL and settings
    participant Git as Private Git candidate
    CLI->>Root: Actual submission, producer and plan
    Root-->>CLI: Bound observation and locks released
    CLI->>Git: Verify result, prepare and pin
    Git-->>CLI: Complete checkpoint and owned witnesses
    CLI->>Root: Reread identity, Work and settings
    Root-->>CLI: Match or stale refusal
    CLI->>CLI: Close witnesses, preserve all causes
    CLI-->>CLI: Digests and counts, no controls started
```

The maintained `native-managed-candidate` recipe exercises actual inert local workers, public handoff, preparation and repetition, refusal/recovery and a public settings mutation from Git. The last case verifies Git holds no Root/settings locks and the final observation refuses stale state. It establishes neither OS confinement, executed cumulative commands nor independent acceptance.

## Native managed controls execution

`task validate WORK` prepares or reuses the candidate outside Root transactions and executes the current plus already accepted task cohort. Policies are normalized separately with persisted settings, without an aggregate ceiling. An exact recorded request is checked before any preparation. Active jobs require reconciliation; recorded jobs replay without commands, reopening the candidate or reading the admission frame.

The private schema1 `native-validation-candidate-…receipt` retains the complete preimage before SQL claim. Claim revalidates actual producer, submission, policies, repository and connected Root directory/database instance, then atomically records all slots, candidate/plan origin, any actual owning lease and Work event. A pre-claim failure may retain files or a Git ref without creating permission. Transport IDs stay separate from original control/result names.

The controller retains directory witnesses during its execution call. It refreshes Git/raw tracked sources before each control; supervision polls check declared inputs, held identities and Root authority without Git under locks or on every poll. The helper checks the actual Unix channel, parent/child/group/permission, reads the preimage bound by SQL origin, pins its own directories outside Root locks and revalidates source before exec. Its cwd remains descriptor-anchored through exec. Existing saved deadlines/output/poll/stop settings apply; no global watchdog or attempt ceiling is added. Unknown waits remain failed recovery, never empty success.

Submitted report artifacts stay Root-relative. Inputs are first resolved/hashed inside the candidate project, then referenced by their actual contained Root-relative paths for existing gate/evidence readers. Those references may expose relative stage coordinates, never an absolute host prefix; a same-named Root file cannot replace a candidate input. Private frames remain private. Final complete inspection precedes v3 original/public receipt and atomic job/slots/Work publication. Public controls are target-task-only, but any cohort failure blocks the overall result. Ordinary failed checks still allow following configured controls; source/transport errors leave later slots explicitly unattempted.

After publication refusal with complete observations, `task validation-reconcile` applies originals after actual owner departure without another command or copy. `task validation-retire` explicitly abandons known observations without adopting results or requiring still-fresh source. Primary/close/wait causes and already recorded Work metadata survive output layers. Held descriptors and sequential observations are not OS confinement against a malicious same-UID writer.

Complete independent managed review, whole-cohort context and protected candidate-pointer publication remain unported. Subsupervisor/root supervisor retain business decisions; passing controls is not acceptance. The `native-managed-validation` recipe uses actual inert local workers, an `app` project, real commands, a public-file refusal/public recovery and replay without a candidate. Its public cohort is the current task; it fabricates no accepted predecessor. Multi-task plan/receipt cases remain separate component evidence.

```mermaid
sequenceDiagram
    participant O as CLI / conductor
    participant R as Root SQL and files
    participant C as Retained candidate
    participant H as Private helper
    participant P as Actual control
    O->>R: Exact recorded request
    alt Recorded job
        R-->>O: Original and current Work, no command
    else New job
        O->>C: Prepare and pin outside Root locks
        O->>R: Private preimage then whole-cohort claim
        loop Every slot
            O->>C: Complete inspection before control
            O->>R: Observed identities then permission and actual lease
            O->>H: Grant byte on private channel
            H->>C: Pin own directories
            H->>R: Revalidate instance, source, policy and permission
            H->>P: Exec from held directory
            P-->>O: Observed wait and output
            O->>R: Immutable slot frame
        end
        O->>C: Final inspection outside Root locks
        O->>R: V3 receipt and atomic result publication
        R-->>O: Pending review or blocked, never accepted
    end
    Note over R: Complete final validator still guarded
```


## Reconstructing the combined receipt from slot originals

`task validation-reconcile` and `task validation-retire` can reconstruct a missing
combined receipt only when **every private slot original already exists**. Recovery
runs no command. Confirmed original-owner departure, actual Root identity and SQL
rows, policy and current source must match the recorded origin. Managed candidate
inspection occurs outside Root locks and explicitly closes its owned witnesses.
Every slot keeps its canonical body, command, identity and actually observed wait;
no missing receipt, result, exit or non-invocation is invented.

An existing unreadable, incomplete or malformed combined original refuses; only
confirmed absence permits reconstruction. Missing historical origin, changed
source, absent slot or unknown process outcome refuses without result publication.
This conservative path does not settle partial batches whose observations are
missing. PID absence alone never releases them.

A private `native-validation-reconstruction-…receipt` preserves the assembled body
before create-only combined publication. Its event/time are reused after an
interruption only when the body still exactly matches verified source/slots.
`finished_at` then describes the assembly observation, not a reconstructed process
exit time. Command results remain the originals. Publication or retirement then
rechecks the originals and its own transactional conditions. An error may leave
either file visible without a SQL result; it proves no rollback. Terminal replay
keeps its historical path without reconstruction. No task acceptance or automatic
command retry is added.

```mermaid
flowchart TD
    R[Explicit reconcile or retirement] --> G{Readable combined receipt}
    G -->|Yes| H[Unchanged original path]
    G -->|Error other than absence| X[Refusal with retained cause]
    G -->|Confirmed absence| O[Departed owner and SQL origin]
    O --> C[Current source and candidate verified]
    C --> S[Every original slot and exact row]
    S --> Q{Known wait or original noninvocation}
    Q -->|No or missing observation| X
    Q -->|Yes| A[Reconstruction original then combined receipt]
    A --> P[Revalidated publication or retirement]
    P --> N[No command rerun or acceptance]
```


## Native complete-candidate inspection before final review

After actual passed native managed controls, `task review-preview` observes
provenance/configuration readonly; `task review-inputs` materializes a retained
Git-candidate inspection projection. Only an existing candidate is inspected: no
new stage, job, permit, reviewer call, debit or acceptance. Final verdict context,
accepted-baseline verification and publication remain unwired. `task review`
therefore still refuses with `review-managed-candidate-unported`, even after a
successful preview.

The inspector verifies private v3 receipt, registered native slots and current
source. Candidate pinning occurs outside Root/settings locks, while descriptors
remain owned through materialization, final outside-lock reinspection and checked
close. A procfd path belongs to that holder; it is never a public reusable source
reference. This does not confine a malicious same-UID writer.

The projection includes every tracked path in the retained candidate repository,
including project prefixes. It does not scan the Swarm Root, untracked files or
live Git origin. Complete text bytes must match recorded raw-source hashes.
Symlinks quote their literal UTF-8 target without reading that target, including
outside-repository targets. Binary/non-UTF8 content, unsupported mode, missing
original or source drift refuses without silent omission or truncation.

Canonical reports and criteria of the current/accepted cohort accompany policies
and results freshly executed in this cumulative job, original result names and
distinct transport keys. The target receipt is also supplied. The projection
declares `inspection_only=true`, `native_invocation_supported=false`,
`native_verdict_supported=false`, `accepted_baseline_verdicts_verified=false`;
it does not claim historical accepted-verdict verification. Truncated control
output stays explicitly marked as such, never invented as complete evidence.

Existing settings govern: `review_delivery_files` bounds the full presented Git
inventory; `review_context_files` counts every material including attributed
duplicates; `review_source_bytes` counts UTF-8 source bytes, and
`review_context_bytes` the final prompt including framing/escaping. Global delivery
read limits also apply to sources. Zero explicitly means no additional bound;
missing/negative values or exceeded limits refuse. Existing public persisted
quotas/validation are consumed; no constant or second ceiling is added. Refusal
and close errors retain causes, never permission.



## Readonly candidate citation protocol

For a managed candidate, `task review-inputs` includes `managed_protocol` in its
response and complete prompt. Ordinary inputs retain their JSON without this
field. The protocol contains the typed cohort context and digest, complete
citation catalogue and digest, and `reply_schema`. Citation version2 and inspection
scope1 are protocol formats, not operator settings. Each task is attributed to its
canonical report, latest recorded attempt, submitting producer, task/policy
contracts and actual job observations. Presence does not authenticate a historical
verdict.

The catalogue shares the reply parser's original producer. Every source remains
byte-for-byte reconstructible, including literal symlink targets. Engine cohort/control metadata remains in the attributed materials and receipt; it is not relabeled as implementation source evidence. Proposed
references are filtered by saved Admin/API/CLI reply text limits; originals are
never clipped or removed from the catalogue. With no admissible reference the
schema still permits `fail` or `unknown`, but no unsupported `pass`. Limits remain
UTF-8 bytes, not JSON Schema characters. The parser also enforces unique tasks and
criterion indices, coverage and quote membership; a schema alone establishes
neither those properties nor criterion truth. For a zero-criterion task the schema requires an explicit empty criteria array, while the parser also accepts an omitted array. This is a stricter response representation, without additional acceptance authority. The normalized-quote minimum governs Markdown fallback only; catalogue references resolve exact original excerpts first.

The saved prompt cap includes context, catalogue, schema and JSON escaping.
`source_bytes` remains the original-material byte sum; representation duplicates
count in the complete prompt. Context declares `inspection_only=true`, without
invented accepted baseline, Git diff or blob identifier inferred from SHA-256.
Native managed invocation, accepted-predecessor verdict applicability, publication
and acceptance remain guarded. A syntactically valid response remains data, never
permission or acceptance.

```mermaid
sequenceDiagram
    participant U as Inspection CLI
    participant R as Root SQL and settings
    participant C as Retained candidate
    U->>R: Verify published v3 job and originals
    R-->>U: Observations then release locks
    U->>C: Pin existing candidate and raw sources
    U->>R: Current source and cohort in snapshot
    U->>C: Complete texts and literal link targets
    U->>U: Shared parser catalogue and saved limits
    R-->>U: Snapshot and settings closed
    U->>C: Final reinspection then checked custody release
    U-->>U: inspection_only protocol and digest
    Note over U,R: No reviewer call or acceptance
    U->>R: Separate native task review request
    R-->>U: Refuse until final protocol is wired
```


## Check supplied cohort output

`task review-check ID --root ROOT --input FILE --json` takes the four
`review-inputs` fields (`schema_version`, `event_id`, `expected_revision`,
`task_id`) plus `prompt_sha256`, the original prompt digest, and `output`, a
UTF-8 string containing supplied JSONL events. Unknown and duplicate fields
refuse. The existing shared command reader's technical input limit remains;
no additional business cutoff is introduced.

The engine rebuilds complete inputs with current saved settings, holds candidate
witnesses throughout decoding, then refreshes and explicitly closes them before
returning. A stale prompt digest, ordinary candidate, incomplete cohort, duplicate
indices, invalid citations, declared tool or missing terminal event refuses.
Actual saved Admin reply/event/text limits apply; refusal preserves its cause
and emits no partial CLI result.

The result contains parsed opinion data and context/catalogue/response digests.
`supplied_output_checked=true` describes supplied bytes only. `provider_started`,
`review_recorded`, `native_execution_proven`, `reviewer_authenticated`,
`verdict_applicable` and `task_accepted` remain false. Even a `pass` opinion proves
neither provider execution, identity nor criterion truth. No debit, reservation,
recorded verdict or Work mutation occurs. No new HTTP endpoint or UI is added.
Managed native invocation and publication still need wiring; `task review`
retains its protection refusal.

The permanent `native-managed-review-check.json` recipe separates positive cases,
refusals and unchanged Work state. Decoder unit tests separately cover retained
original bytes and causes after failure.

```mermaid
sequenceDiagram
    participant U as CLI review-check
    participant R as Root and saved settings
    participant C as Held candidate
    participant D as Tool-free decoder
    U->>R: Revision, task, original prompt SHA and supplied JSONL
    R->>C: Hold and read originals
    R->>R: Rebuild protocol and verify SHA
    R->>D: Saved Admin limits and full cohort
    D-->>R: Opinion or refusal with cause
    R->>C: Refresh then close witnesses
    R-->>U: Data only, no applicable verdict
    Note over U,D: No provider call or acceptance
```


## Managed native reviewer invocation and original recovery

CLI `task review ID` connects the managed candidate to the existing claim, call
debit/reservation, child registration, separate permission, tool-free transport
and opinion publication. Candidate custody is acquired outside Root/settings/SQL
locks and retained by the owner through the outcome. Later boundaries and the
child reacquire the same actual control-job candidate, requiring complete equality
with the current running review; no public JSON can supply that internal allowance.
Git stays outside SQL snapshots. Explicit close preserves primary/cleanup causes.

Preparation preserves actual saved Admin settings, financial reservations, mission
rights, provider/model/executable bindings and original admission TTL. Its host
check creates/removes a probe file: this is not a read-only operation. Internal
provider input declares transport/recording support; `review-inputs` remains a
non-grant inspection. These protocol facts are not operator policies. No timeout,
retry, ceiling or ordinary single-task decoder fallback is introduced.

Every canonical cohort task (selected task plus accepted predecessors) must appear
in the reply. Candidate/catalogue/criteria/citations are checked for the complete
cohort. The private journal freezes actual original protocol, saved reply/text
limits, prompt digest, reply bytes and process observations. Publication separately
compares current complete inputs/settings; an old cap never grants current authority.
Public canonical context is content-addressed; certificate/event retain existing
`managed_tasks`, `candidate_commit`, `previous_candidate`, `context`,
`context_sha256` fields. Aggregate rationale follows Go; only the selected task's
review field is changed.

Partial cohort or invalid transport records an explicit failed call with retained
originals. If publication refuses after known completion, job/reservation remain:
repair the cause and use `task review-reconcile REVIEW` to apply the original
without another provider call. Exact `task review` replay never reruns the provider.
Corrupt context/certificate/journal refuses; exact restoration permits recovery.
Consumption is never reset and declared usage is neither an invoice nor zero cost.

**Authority limit remains:** this is a native opinion on the current candidate,
never task acceptance. Accepted-baseline historical verdicts are not verified by
this `inspection_only` context. `verify_current_native_review` still refuses managed
acceptance/production-retry authority until that consumer contract is ported;
reading a result into supervisor context does not remove this guard. A passed
opinion, certificate or `verdict_applicable` closes no task or mission. The
supervisor owns business decisions subject to required engine evidence.

```mermaid
sequenceDiagram
    participant U as CLI task review
    participant C as Held candidate
    participant R as Root and saved settings
    participant H as Private child
    participant P as Tool-free provider
    U->>C: Acquire outside locks
    U->>R: Atomic claim, call debit and reserve
    U->>R: Observed child then separate permission
    R-->>H: Private channel, kernel identity and grant
    H->>C: Reacquire same candidate before snapshot
    H->>R: Verify running review and complete inputs
    H->>P: Execute with sealed prompt
    P-->>U: Complete cohort JSONL
    U->>U: Wait, decode, retain journal and frozen policy
    U->>C: Reinspect sources before publication
    U->>R: Content-addressed context and atomic opinion
    R-->>U: Recorded opinion, no acceptance
    U->>R: Reconcile or exact replay
    R-->>U: Verified original, no provider call
```

## Native managed prepublication — current observation

`swarm task publication-preview WORK --input REQUEST --root ROOT --json`
uses the same strict native envelope: `schema_version: 1`, `event_id`,
`expected_revision`, `task_id`. The event is a read input coordinate, never a
persisted event. The Root API is `preview_managed_publication`.

The read verifies the framed private original, review job/event/certificate,
public context and original controls. It then retains the actual candidate
outside Root/SQL reservations and verifies current cohort policies, attempts,
completed producers, contracts and canonical reports. Sources are refreshed,
state/settings rechecked and descriptors explicitly closed, including on error.
A submitted task, or one blocked by that exact adverse opinion, is observed
without retry rights. A `changes_requested` opinion can be observed but additionally reports
`managed-passed-opinion-required`; a failed protocol without a verdict, stale
revision, changed source or incomplete original refuses. Unknown failures never
become successful empty observations.

Successful reads expose `native_original_verified`, `current_candidate_verified`,
identifiers/digests and cohort task IDs. `publication_supported` describes the
initial protocol without an accepted baseline; `publication_ready`,
`provider_started` and `task_accepted` stay false. `blockers` distinguishes final
writer admission from the unported cumulative baseline. These describe product
capabilities, not operator policies or cohort ceilings. Effective values still
come from saved Admin settings.
The CLI closes Root before checked output/flush; output failure grants no new
call. No reviewer identity, criterion truth, permission or later freshness lease
is established. Existing acceptance remains refused.

Managed acceptance must also atomically publish the candidate pointer and coherent
cohort evidence. A status-only change could accept code absent from the next
worker copy. The initial protocol is implemented below; Go `managed_integration.go` remains
the reference for the cumulative baseline still to be ported.

```mermaid
sequenceDiagram
    participant U as Operator / supervisor
    participant R as Root / CLI
    participant O as Native originals
    participant C as Retained candidate
    U->>R: publication-preview (task, revision)
    R->>O: Verify opinion, controls, context and certificate
    R->>C: Retain and refresh outside SQL locks
    R->>R: Compare cohort and current state
    R->>C: Refresh then explicitly close
    R-->>U: Verified observations and explicit blockers
    Note over R,U: Preview alone never publishes or accepts
```

## First native managed publication — protected transaction

`swarm task publish WORK --input REQUEST --root ROOT --json` consumes the strict
native envelope (`schema_version`, `event_id`, `expected_revision`, `task_id`).
`RootStore::publish_managed_candidate` publishes the first verified candidate and
accepts its submitted task in **one SQL transaction**: repository candidate
pointer, task, integrated worker row and `managed.publication` event. Its canonical
command contains the accepted Work; request and payload intentionally store the
same bytes under the existing storage contract. No advisory Git branch changes.

Candidate custody and refresh occur outside SQL locks. The writer requires passed
native originals, the full cohort, completed producers/attempts, current saved
policies, actual committed Git reports, artifacts, project context, gate and
prerequisites. Live worker/control/review ownership excludes publication. An
opaque internal proof binds the shared lifecycle transition to these exact inputs;
no request boolean bypasses admission. Artifact names `candidate/`,
`candidate-link/` and `cohort/` refer to Git blobs, literal Git symlink targets and
attributed cohort reports, rather than ordinary files beneath Root.

This initial protocol requires no previously accepted task and a candidate pointer
still equal to the repository base. Cumulative baselines explicitly refuse with
`managed-publication-accepted-baseline-unported`: a porting boundary, not an
operator ceiling or an implicit one-task-per-job rule. `publication-preview` now
reports first-publication support through `publication_supported`; readiness stays
false with `managed-publication-write-admission-required` because final admission
belongs to the protected writer. Preview alone never grants publication permission.

Exact replay verifies the canonical receipt, normalized worker row, originals
and current accepted state without a provider call, control or debit. Collisions,
missing evidence and drift refuse. Current native managed review evidence requires
a real publication and actual Git blobs; generic task update cannot accept a
merely previewed candidate. Historical worker readers may observe an integrated
row only with its receipt; physical execution still requires the original ready
row. Commit and cleanup failures retain causes and known recorded Work coordinates;
an error does not always establish rollback.

The `native-managed-first-publication` recipe uses isolated inert fixtures only.
Temporary SQL injection lives in the explicitly invoked Rust test
`first_publication_sql_rollback_preserves_all_state` on a real retained fixture.
An unexpected permanent trigger is rejected while opening Root and would not
establish the intended rollback. Local tests, GLM static review, build-profile
qualification and installed delivery remain separate evidence. Mission acceptance,
the global judge and cumulative baselines remain open.

```mermaid
sequenceDiagram
    participant S as Supervisor / operator
    participant R as Native Root
    participant C as Retained candidate
    participant D as Canonical SQL
    S->>R: task publish (task, revision, event)
    R->>C: Retain and refresh outside SQL
    R->>D: BEGIN IMMEDIATE, current state/settings
    R->>R: Originals, cohort, Git, gate, prerequisites, idle owners
    alt Missing proof or inconsistent write
        R->>D: Explicit ROLLBACK, retain causes
        R-->>S: Refusal, no empty success
    else Admission and writes verified
        R->>D: Candidate + accepted task + integrated worker + receipt
        R->>D: COMMIT
        R-->>S: Recorded publication, no mission acceptance
    end
    S->>R: Exact event replay
    R->>D: Verify receipt and current state
    R-->>S: Replay without provider or debit
```

The current reader also matches the full binding, cohort contract, result commit,
producer and attempt to the actual candidate reread from the private control
original. A canonical SQL receipt and matching worker row alone are insufficient:
changing both coordinates still refuses. The retained counterexample exposed an
unmatched declared binding; `native-managed-publication-current` covers these
changes and exact restoration.
