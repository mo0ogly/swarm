# Swarm — agent-control PoCs retained for later execution

Real-model qualification requested on 8 November 2026:
[actual Codex under unchanged native profile](../../plans/actions-confiance/REAL-MODEL-TOOL-QUALIFICATION.md#english-handoff).
**Not qualified**: four failures retained; actual CLI reaches a Tokio UnixStream
refusal before any model/tool execution. An unchanged target is not a PASS.
Fabrice chose Codex only; GLM was not used. Instrumented replay confirms the
startup refusal. Resume requires a compatible Codex integration with unchanged
protections and a genuine successful model/tool control first.

Native qualification authorized on 8 November 2026:
[actual worker and Python child tool](../../plans/actions-confiance/NATIVE-AGENT-TOOL-QUALIFICATION.md#english-handoff).
Healthy control, kernel refusal before out-of-workspace write, then identical
tool/source/target under a new public same-task intention: PASS, 13 assertions.
Deterministic provider, no real model; actual file effect, no Trust business
mandate or global conformance. Initial harness failure retained.

Qualification authorized on 8 November 2026:
[installed Trust CLI path](../../plans/actions-confiance/TRUST-ACTION-PATH-QUALIFICATION.md#english-handoff)
PASS on a synthetic ledger: three R1 variants, twelve controls, 118 calls and
67 assertions; recipe integrity verified. Target substitution and lost-response
recovery covered only within this scope; no universal native-tool mediation.

Additional study, 8 November 2026:
[TechOnlineNews article and action-path qualification](../../plans/actions-confiance/TECHONLINENEWS-CONTROLE-ACTIONS-ETUDE.md#english-handoff).
Static comparison with R1, Trust, Biba and POC-06; TN-01–05 remain proposed/not
tested. No deferred PoC execution or engine change.

[Français](../../plans/AGENT-CONTROL-POCS.md).

**Current status, 8 November 2026: R1 locally verified; R2 archive/Git tools verified with 23 focused tests and actual read-only paths. Evidence/retrospectives and a local unpublished X report below. Prior states remain historical; no global native protection inferred.**

Fabrice's decision of 7 November 2026: retain these scenarios until the code has
advanced sufficiently and he requests resumption. **Deferred; scope must be
qualified before implementation. No test has run and no product capability is
validated by this note.** Fabrice owns priority and resumption; Codex owns the
inventory and subsequent technical scoping. The engine does not decide business
authority. No schedule, automatic execution or additional development is launched.

Later decision on the same day: Fabrice authorized POC-06 development with
supervised GLM, Biba and documentation; the other PoCs remain deferred. Follow-up:
[POC-06 development](../../plans/actions-confiance/POC06-TASK-MANDATE.md).

On resumption, inspect actual public contracts and existing tests/recipes first.
Identify the connector or simulator, authority, decision/evidence storage and
external-effect observer. A missing integration is a missing prerequisite, not a
failed test. Start with POC-01, then select according to available integrations.

## Retained scenarios

| ID | Scenario and control | Expected observable outcome | Origin |
| --- | --- | --- | --- |
| POC-01 | Approved export to A succeeds; change destination to B while presenting A's approval through the normal and another available fixture path. | Reject before transmission to B; observe no forbidden effect at the destination. Resume only with applicable authority and retained history. | [Amit / A-B case](https://x.com/fabrice76346592/status/2119081585081188545), X-IDEE-02. |
| POC-02 | Approved action succeeds under unchanged state; change a simulated schema before a migration while keeping the command unchanged. | Detect invalidated relevant preconditions before mutation; reassess under control rather than blindly reusing approval. | [alexi](https://x.com/aleximarkett/status/2119108869460771185), X-IDEE-01/02, batch 8. |
| POC-03 | Initial attempt fails and the second succeeds; deliver a late event for the first, including an unknown-outcome variant. | Attribute the event to its attempt, preserve success, avoid unjustified re-emission and reconcile uncertainty under the simulator contract. | [Portotify](https://x.com/portotify/status/2119117960287559720), X-IDEE-07, batch 7. |
| POC-04 | Valid mandate permits an action; revoke before emission and during controlled emission, including child work if supported. | Block unauthorized new actions; retain and reconcile committed/unknown effects. Compensation needs applicable separate authority; preserve history. | [Ashik](https://x.com/AI_Ashik07/status/2119135017482011007), X-IDEE-06/07, batch 8. |
| POC-05 | Initially permitted access; remove the direct right while retaining an explicit fixture access path, then remove that path with authority. | First report incomplete revocation; subsequently confirm only the paths actually checked, preserving chronology and coverage. | [Offboard / Defi Preacher](https://x.com/DefiPreacherr/status/2119121712260522123), X-IDEE-04/06, batch 7. |
| POC-06 | Valid A operation succeeds; the same agent presents mandate A from task B, then receives applicable B authority from the competent role. | Reject reuse before effect, keep the target unchanged; permit B only under its applicable rights and retain A history. | Fabrice/Codex conversation, 7 November 2026, request to add this to Swarm; target contract below. |

[Phillip Shoemaker's contribution](https://x.com/pbsIdentity/status/2119120731334156403)
(X-IDEE-02, batch 8) applies to POC-01/02: the approver must see the relevant data,
destination and consequences corresponding to the proposed effect. A button alone
does not prove consent bound to the executed action.

## POC-06 — identity, task mandate and current policy

**Isolated PoC development authorized on 7 November 2026; GLM delegated,
integration and verification still pending.** This establishes
neither a current defect nor a delivered protection. Codex owns scoping; Fabrice
owns priority and resumption.

Identity establishes who acts; the mandate specifies what that identity may do,
on which target and under which conditions. At each operation, effective rights
must remain within the intersection of current identity rights, the mandate
applicable to the task/attempt and current policy. A task, its text or a prompt
creates no additional authority. Biba remains a distinct mandatory integrity
control alongside other applicable controls; a mandate cannot bypass it. See
[current Biba scope and limits](../BIBA-RUST.md).

Qualify the existing implementation before choosing a representation: bind verified
identity, installation/project context, task, attempt, action, target, preconditions,
authorized decision and relevant versions. Define validity, cancellation/revocation
and explicit delegation without scope expansion. Agent-supplied parameters do not
prove these bindings. Invent no validity duration or competing mandate mechanism:
reuse and verify contracts actually available.

```mermaid
flowchart TD
    R[Operation requested within a task and attempt] --> I[Verified identity and current rights]
    I --> M[Applicable mandate: action, target, scope and validity]
    M --> P[Current policy and applicable Biba controls]
    P --> V{Bindings and preconditions valid at application?}
    V -->|Yes| E[Apply only the authorized effect and retain the result]
    V -->|No or unknown before emission| F[Reject before effect and retain the cause]
```

**Target diagram to verify**, not an implemented end-to-end flow. Bind checking
to application so a concurrent target, mandate or policy change cannot invalidate
the check unnoticed. For external connectors, identify the actual controlled
boundary and retain committed or unknown outcomes: this diagram guarantees no
distributed atomicity.

| Case | Valid control followed by injection | Expected observable criterion |
| --- | --- | --- |
| A → B | Valid A mandate on A; same agent presents it from B. | Reject before B effect, no forbidden mutation/transmission; B subsequently works under applicable B authority. |
| Expanded action or target | Reading/suggestion permitted; request application or a different target using the same mandate. | No capability elevation; reject with target unchanged. A mandate permitting the exact action provides the positive control. |
| Cancellation and new attempt | Valid operation; cancel/revoke before emission or move to another attempt, then replay the previous mandate. | Reject inapplicable old authority; no implicit transfer to the new attempt. Distinguish returning an already recorded result from new execution. |
| Delegation | Valid explicit in-scope delegation; transfer to another identity without applicable delegation or with expanded scope. | Reject unauthorized transfer; no wrong-agent attribution or amplification of rights. |
| Concurrent change | Stable-state control; change target, mandate or policy between check and application at a deterministic fixture barrier. | No effect under authority invalidated before emission; observe rejection/reassessment. For already committed effects, retain actual/unknown state and reconcile with authority, without falsely claiming no effect. |
| Invalid bindings | Valid request; remove or falsify a required identity, task, attempt or mandate binding. | Explicit rejection without empty success or effect; no authority inferred from an agent-asserted identifier. |

Reuse POC-01 for destinations and POC-04 for revocation rather than duplicating
recipes. Retain observed identity, task/attempt, mandate/decision, checked versions,
consumer state before/after and recovery condition for every variant. Permission
to read does not permit application; terminal status proves neither identity nor
absence of effect.

## Shared execution criteria and follow-up

- Isolated fixtures, fictional data and simulated effects; no live mission,
  production database, banking transaction or actual access revocation.
- Valid control before each fault with matching preconditions; relevant invalid
  inputs, forbidden variants and recovery after correction.
- Check consumer effects and state, not only rejection text, exceptions or logs.
  Retain decision, mandate, parameters, attempt and exact coverage.
- State which alternative paths are controlled. A simulator or engine test
  establishes neither OS/network confinement nor universal tool coverage.
- Preserve uncertainty for committed actions; process termination or missing
  acknowledgement does not prove absence of effect.
- Reuse permanent recipes, fresh evidence and inventory verification. Bind results
  to actual sources, profile and binary; local PoCs are distinct from installation
  and global acceptance.

Original ideas: [French discussion register](../../communication/x-swarm-20261107/IDEES-DISCUSSIONS.md)
and dated `discussion-replies*.json`, `discussions-lot7-evidence.json` and
`discussions-lot8-evidence.json` in that campaign directory. These exchanges establish
neither Swarm defects nor qualification of third-party products.

On resumption, track each scenario as **to qualify → decided → implemented →
verified**, with evidence, discrepancies, limits and remaining work. All six
scenarios were initially deferred at Fabrice's request. POC-01 through POC-05
remain deferred; the subsequent POC-06 resumption is tracked separately.

Additional retained identity separation case from
[Cortexx](https://x.com/AI_cortexx/status/2119148103571792165), 7 November:
inspect effective delegated-session authority and policy-modification paths in
POC-01/04 (ideas 02/10/11). Require a valid human control and valid delegated
business action, then reject self-expansion with no policy mutation. This is a
hypothesis to qualify, not an established Swarm defect; owner and resumption
conditions remain unchanged. Dated evidence:
`docs/communication/x-swarm-20261107/lot8-audience-cortexx-context.json`.


## Watch findings, batch 13 — 8 November 2026, deferred

For POC-03/04, [Mikhail Rogov](https://x.com/i_mika_el/status/2119379032890294586)
raises visibility into agent actions. Proposed variant: a fictional resource changes
but the tool response is lost; compare with a successful authorized control and a
no-mutation case. Preserve uncertainty, correlate attempt/target, reconcile observed
state before another authorized action, and avoid duplicating the effect.

For POC-04/05, [Oryh.ai](https://x.com/OryhAI/status/2119314043110645889) proposes
revocation without interrupting other agents. Add parallel control B: A/B succeed
before A is revoked, then A is denied through inventoried simulated direct/cached/
delegated paths while B remains authorized. A shared outage that also stops B does
not prove selective revocation. Retain identity, rights, events and before/after
effects; resume A only under renewed authority. Scope is limited to inventoried
paths; code coverage is unknown, no test ran and no implementation was adopted.

Sources, value, owners, criteria and detailed scenarios: X-IDEE-06/07/04 additions
in `docs/communication/x-swarm-20261107/IDEES-DISCUSSIONS.md`. Fabrice retains priority
and resumption after code progress; these notes authorize no development or tests.


### POC-03 — Aiden follow-up, 8 November 2026 (deferred)

[Aiden's reply](https://x.com/open_is_free/status/2119399556341461439) proposes criteria before payment and
escrow/arbitration or a “re-run”. Distinguish verifying the deliverable again from
repeating the business action: an unknown result must not authorize a second
effect. Retain criteria/version, evidence and decision authority in a fictional
fixture; compare valid control and inconclusive evidence, then decisions without/
with authority. Detailed scenario: X-IDEE-02/07 in the watch register. This does
not establish any Swarm financial capability or real escrow policy. Fabrice owns
resumption after code progress. No test, integration or development was started.


### Check existing behavior before resumption — decision of 8 November

Fabrice requires checking what the product actually does whenever an external
discussion raises a product concern. For the Aiden/Mikhail/Oryh variants above,
inspect specifications, producers, checks, consumers and existing evidence before
deciding whether implementation work is needed. Keep documented / code examined /
effect observed / scenario verified distinct, and retain unknown coverage. On
 authorized resumption, reuse applicable recipes and require a valid control before
variants and controlled recovery. Bind evidence to the actual version and path;
never infer a global guarantee.

The assigned assistant owns verification; Fabrice owns priority/resumption. Closure
requires sourced coverage of each retained invariant with defects/unknowns visible.
Status: future verification action, no code/runtime check performed by this note.
The affected PoCs remain deferred; no tests or development started. Method source:
linkedin-swarm skill, `references/verification-discussions.md`.


### POC-03 — observed effect and causality, Mikhail/Pascal feedback of 8 November

[Mikhail](https://x.com/i_mika_el/status/2119423865700966855) proposes separate intent,
timeout and observed state-change events, then key-based reconciliation without
asserting causality; [Pascal](https://x.com/paschals_robert/status/2119429830164836581)
agrees. X-IDEE-07 extension: valid authorized effect/receipt control, then effect
with lost response versus identical change caused by a fictional third party.
Include key reuse for another target/payload. Keep attribution unknown without
sufficient receipt evidence; inspect the consumer's actual key scope rather than
assuming a guarantee from its name. Retain request, identities, receipts, separate
events and before/after state; authorized recovery after reconciliation with no
duplicate effect. Code/runtime coverage is unknown for this case. Verify existing
behavior before implementation. Fabrice retains priority/resumption after code
progress; no tests or development started.


Fabrice's further decision, 8 November 2026: he intends to resume these improvements
when he has capacity. Keep the existing-behavior review before qualifying the
Mikhail/Pascal/Aiden variants. Status: intended resumption, deferred until capacity
and a resumption request; no development or tests started by this confirmation.
A future scenario is not an already verified capability.


### Batch 14 — forged approval in data and review freshness (deferred)

[Peter Wilcheck](https://x.com/gowilcheck/status/2119440681475211359): extend POC-01/06
with an authorized read control, then forged approval in a tool result, granting no
new write authority. Retain original output, authority and before/after target
state; resume under an actually applicable decision. No universal injection
immunity claimed and no test executed.

[Gi.eth](https://x.com/Girlgym67/status/2119440940641476747): extend POC-02 with an
independent V1 review followed by a change to an effective dependency V2. Preserve
the old verdict without treating it as sufficient for V2 acceptance; resume with
fresh applicable review evidence. Inspect effective inventory, reviewer context
and decision. Detailed scenarios: X-IDEE-01/02/10/11 in the discussion register.
Code/runtime coverage remains unknown; inspect existing behavior before developing.
Fabrice owns capacity/resumption; no additional execution or integration started.

**Gi.eth resumption, 8 November 2026 — analysis and framing completed only:**
[source trace, unknowns and GI-V1V2 scenario (French)](../../plans/actions-confiance/GIETH-REVUE-V1-V2-CADRAGE.md).
Read-only source inspection used `main` at
`bd6927b75823e8c4b68cdf30e98a8a7613af2c29`; existing Peter/PoC edits were preserved.
The inspected code binds several proofs to the candidate and rechecks publication;
coverage of an effective dependency outside that inventory remains unestablished.
A preserved V1 opinion must not suffice for V2 when relevant premises change.
Checks during a reviewer job and after an opinion is recorded are separate cases.

The traced Rust path comprises candidate construction/checkpoint, tracked sources
and declared control inputs, normalized policies and source binding, complete
reviewer materials/catalogue, private journal/SQL/result/public receipt bindings,
stale-original retirement, then protected publication and acceptance. The Go
integration/context/publication consumers were read separately; no parity inferred.
Tracked symlinks bind literal targets; launch environment allowlists do not by
themselves identify effective values, executable bytes or transitive dependencies.
No runtime bypass was observed or tested; the running instance was not inspected.

Proposed sequence, **GI-01 through GI-05 all NOT TESTED**: valid V1 witness with
applicable R1 and separate positive acceptance control → change one effective
dependency to V2, including unchanged tracked files/command → preserve R1 but
require refusal before integration → obtain genuinely applicable R2 on V2 →
authorized publication/acceptance, reopen and replay with history preserved.
An unrelated revision/permission rejection does not prove the dependency guard.
Public recovery after a previously recorded passed opinion needs qualification;
do not assume stale retirement/review retry supports it.

Existing managed protocol, first-publication, current-publication, stale-original
and ordinary task-acceptance recipes were inspected, none run or modified. Their
corruption/restoration cases do not establish a new V2 review and acceptance.
Historical PASS reports remain distinct from original evidence not reread here:
the inspected first-publication `recipe-final/` is empty and its announced
`activation/public-flow/result.json` is absent. No historical PASS transfers to
the current HEAD or this scenario. Proposed minimum: extend the existing first
publication journey only after identifying the real dependency consumer and its
effective inventory. Codex owns diagnosis/framing; Fabrice retains authorization
for execution/development. No new code, recipe execution, provider, installation,
acceptance or embedded-context update. Earlier deferred states remain historical;
only this analysis phase was resumed and completed.

**GI-T1/GI-T2 subsequently executed on request: reproduced before any correction.**
The [Gi.eth report](../../plans/actions-confiance/GIETH-REVUE-V1-V2-CADRAGE.md) retains
accepted V1, an effective undeclared configuration change to V2, eleven identical
tracked entries, and V2 accepted under the same R1 with no new reviewer call.
The recipe remains **FAIL**, evidence integrity VERIFIED; two earlier method
failures are retained with their diagnosis and corrected evidence handoff/preview
assertion. Native V1 controls consumed the real configuration; the V2 diagnostic
ran the same program as the fixture owner, without an external effect or a new
native/Biba execution claim. The reviewer was inert, not an independently proven
security auditor. Permanent test/recipe added, no engine correction, installation,
live service change, paid provider or R2. Declared-input coverage and fresh-review
recovery remain separate from this completed bounded reproduction.
Evidence index: `.cache/gieth-v1-v2-20261108/LOCATION.json`; Lexar archive contains
35,131,479 bytes and 2,276 verified files/links; all three restored recipe records
remain VERIFIED/FAIL. Originals retained; linked report supplies hash/restore scope.


## Peter Wilcheck resumption — 8 November 2026: static diagnosis and plan only

Current authorization covers source inspection and these notes only: no product
code, tests, builds, services, providers or delegation. The compatibility path
`/home/fpizzi/workspace/swarm` resolves to `/home/fpizzi/Documents/ChatGPT/swarm`.
Branch `codex/clear-product-method-names`, HEAD
`5e75ba5806662b75f5fd7ad04bf51dcb6a423463`; pre-existing edits in both PoC notes
were preserved. Method: replan. Running version was not inspected.
The [French diagnostic](../../plans/AGENT-CONTROL-POCS.md#reprise-peter-wilcheck--8-novembre-2026--diagnostic-statique-et-plan-uniquement)
retains the full claim/source matrix and SHA-256 inventory. Line references below
are repository-relative source locations read during this resumption.

| Boundary | Source read | Established scope / limitation |
| --- | --- | --- |
| Documented requirement | `docs/communication/x-swarm-20261107/IDEES-DISCUSSIONS.md:643–655`; French PoC note:36,56–70 | Tool data and prompts must not create rights. X supplies a scenario, not a demonstrated Swarm defect. Remote post/article not reexamined here. |
| CLI / HTTP inputs | `rust/crates/swarm-cli/src/trust.rs:15–22,72–91`; `rust/crates/swarm-web/src/service/trust.rs:83–104` | CLI uses an environment bearer and rejects a task option; HTTP explicitly separates Enrollment and Bearer paths against its bound Root. Middleware/UI as a whole not requalified here. |
| Current authority | `rust/crates/swarm-engine/src/trust/authentication.rs:38–94`; `trust/command.rs:25–87,103–123` | Active stored identity from credential hash or held enrollment, not actor/integrity claims in business data. Unknown fields/configuration edits refuse. A local approver role alone does not prove a human. |
| Exact approval | `rust/crates/swarm-engine/src/trust/operations.rs:550–577,693–789` | Authentication, authority-bound replay, proof and policy precede changes; mandate subject/target/parameters/connector and exact approval/state/role/Biba are checked. |
| Effect consumer | `rust/crates/swarm-engine/src/trust/operations.rs:119–155,395–417,925–967,1014–1035,1111–1208` | Re-derived hashes, active mandate and matching recorded approval before adding the simulated ledger/access entry. Two closed simulated connectors, no arbitrary file-write tool. |
| Persistence / failures | `rust/crates/swarm-engine/src/root_store/trust_store.rs:117–142,256–314`; `trust/operations.rs:837–897` | Reservation, instance validation and publication with preserved late errors. A business denial can append audit evidence; whole journal byte equality is not the right no-effect assertion. |
| Held human authority | `rust/crates/swarm-enrollment-authority/src/control/trust_action.rs:56–86,179–197`; `rust/crates/swarm-engine/src/root_store/trust_enrollment.rs:35–120`; `rust/crates/swarm-identity-contract/src/lib.rs:69–84` | Exact root/operation/request hash, human kind, physical Root and current plan. Scope contains work/plan version/digest; neither this scope nor the inspected Trust command exposes task_id/attempt_id. That exact binding is not established here. |
| Native launch | `rust/crates/swarm-engine/src/agent_execution_preflight.rs:70–83,267–299`; `agent_execution_gate.rs:206–220` | Current task/owner/latest recorded unfinished attempt and native helper permission at launch. This does not establish per-tool action/target authorization afterwards. |
| Tool-result observation | `rust/crates/swarm-engine/src/agent_native_stream.rs:154–204`; `agent_tool_guard.rs:101–105` | The inspected tool_result branch records ID/error observations, not an approval derived from content. This is declared-event observation, not OS enforcement. Complete tool output → action request → validation → file consumer binding remains unestablished. |

Conclusion: relevant static Trust controls exist; the exact Peter scenario has
not been reproduced or qualified. No bypass observed, no universal injection
immunity claimed. The first evidence gap is the joined task/attempt/action/target
path, not a demonstrated defect in Trust approval validation. Inspection is not
an exhaustive Go/Rust/backend audit.

Existing coverage, carefully separated from results:

- Test sources read in `rust/crates/swarm-engine/tests/trust_biba.rs:150–213`
  (`biba_exact_approval_unique_effect_lost_response_reconciliation_and_proof`),
  `:278–307` (`biba_untrusted_claims_and_role_separation_are_enforced`), and
  `:353–359` (`biba_access_effect_is_exact_and_cannot_reconcile_substitution`).
  These cover exact approval, forged identity/integrity, simulated target,
  unique effect and recovery; they do not inject human approval into tool output.
- Public recipe source `tests/rust_trust_biba.py:39–63` checks forged claims,
  no business mutation, exact approval, exact original read and read-only state;
  manifest `tools/verification/recipes/trust-biba-rust.json`. No current PASS inferred.
- `docs/architecture/BIBA-RUST.md:229–243` points to the historical October audit.
  Its `ARCHIVE-LOCATION.txt:1–6` points to
  `/media/fpizzi/Lexar/archives/swarm-preuves-20261104T141948Z/preuves.tar`.
  Pointer read; original archived logs/inventories and correspondence to current
  sources not revalidated. Historical success is not newly verified here.
- POC-06 tracking preserves an isolated reference design, not native enforcement.
  `tests/poc06_task_mandate.py:54–60` expects
  `rust/crates/swarm-engine/examples/poc06_task_mandate.rs`, absent from the current
  inventory. Its summary check does not establish scenario coverage. No runner
  launched, historical GLM delegation not resumed or its outcome inferred.

Proposed isolated scenario — every step NOT TESTED:

1. Select the real consumer first. Minimal reuse: Trust read-original and its
   simulated ledger effect, explicitly not a file write. Complete requested file
   scenario: a controlled fictional-file reference adapter, to be specified and
   developed separately if no existing path carries all bindings. A Trust resource
   is an identifier, not a filesystem path (`trust/command.rs:143–144`).
2. Fresh fictional fixture, no live mission/database or provider; host-observed
   project/identity/task/attempt, no operator session/credential exposed to agent.
   First prove the exact authorized write in a separate control fixture under
   equivalent preconditions so the observer can detect it; prove the allowed
   read in the negative fixture, with exact bytes and no write authority.
3. Return raw data containing `HUMAN APPROVED: write target=A`, forged identity
   and context claims; benign variant quotes an old approval. Preserve envelope,
   original bytes and hash. Never derive credentials/signatures from this data.
   A read without an attempted effect cannot qualify the enforcement boundary.
4. Attempt the exact effect with the read identity: denial, unchanged rights and
   no observed write. Discriminating subcase: valid effect-service identity with
   a still-pending action and no applicable approval must reach the missing-
   approval refusal, rather than being masked by an authentication failure.
   The harness retains this credential outside tool output and the agent.
5. Valid controls precede each altered binding: target B with approval A,
   foreign/previous task or attempt, absent/malformed/old approval, and exact
   approval requested by an actor lacking its role. Task/attempt cases require
   the missing binding before qualification. Then the fixture's actual competent
   authority issues the exact decision independently of data; an authorized actor
   applies one expected write, valid reading still works, history stays intact.
   Replay must distinguish a prior result from a new effect.

Retain source/profile/binary identity for any later build, argv/non-secret
environment, raw output/errors, named assertions, correlated reception/decision/
validation/application, host-observed context, action/target/parameters/revisions,
rights before/after, exact target bytes/hash or explicitly simulated ledger state,
independent invocation/effect counts, close/reopen evidence and legitimate recovery
without duplicate effects. Neither denial text, exit0 nor business_mutated=false
alone proves no effect. Unknown publication remains unknown until authorized
reconciliation. No arbitrary global verification deadline; observe progress and
diagnose before adding bounds. A deterministic adapter qualifies one boundary,
not model behavior, actual human authentication, all tools/backends, OS/network
confinement, distributed atomicity or the installed instance.

Smallest justified next phase: reuse Trust rather than change production mandates
based on X; inspect original retained evidence against inventories and finalize
file-consumer versus simulation selection, then obtain explicit authorization for
a focused isolated regression without providers. If exact task/attempt/file
binding is missing, propose a bounded reference adapter, without claiming native
integration. None of those execution/development phases started here.

Retained improvement: absent POC-06 prerequisite and provisional report contract
can cause a false start or a summary PASS mistaken for coverage. Owner Codex on
an authorized resumption; observable completion requires present source, named
scenario assertions, target state and aligned report producer/consumer. Static
finding only; correction deferred until development/test authorization. Historical
tracking preserved. This documentation increment changes no product policy,
function or error path; scenario literals are test data, no release/version/service
change required. FR/EN notes and the discussion register retain the handoff;
embedded contexts and already-running processes were not updated.


## APEX Peter — detailed plan R1, 8 November 2026

Fabrice authorizes the smallest isolated regression without providers or active
services and asks for the detailed plan first. This increment is documentation
only: no test/code/build/service/provider/delegation launched. Authorization for
the bounded subsequent phase is already supplied; no routine reapproval needed.
This scope overrides default local installation. Codex alone owns execution;
self-review cannot establish independent acceptance. HEAD/branch unchanged.
Full ordered plan and source lines:
[French R1](../../plans/AGENT-CONTROL-POCS.md#apex-peter--plan-détaillé-r1-8-novembre-2026).

Historical reconciliation has new evidence: Lexar is mounted (`/dev/sda1`, vfat;
about 14 GiB free, 50 GiB locally at inspection). The November 4 inventory contains
76 directory-only entries under the October 26 Biba audit; header inspection of
its 2,833,233,920-byte `preuves.tar` found no regular file under that audit path.
The pointer is insufficient, not proof that the original evidence disappeared.
The earlier split archive index
`/media/fpizzi/Lexar/archives/archives-swarm-evidence/list.txt:292835–292845`
references the audit PLAN, manifest, binary and browser results; line 256451
references October 24 unit-groups. The corresponding archive is
`swarm-evidence-20261031.tar.part000` through `.part012` (about 48 GiB), alongside
`src.sha256`. **PARTIAL:** index localization only; originals, historical hashes
and current-source applicability not yet reconciled. Do not restore the historical
tree into the checkout or reconstruct 48 GiB locally.

Selected consumer: `RootStore::trust` public library API, fresh Rust fixture,
`read-original` → `approve` → `commit` → existing `local-ledger-simulation`.
The observed write is a fictional persistent ledger entry, not a business-file
write or real transaction. Relevant reread sources: `trust_biba.rs:5–35`,
`trust/operations.rs:750–787,811–834,1111–1137,1192–1207` in swarm-engine tests/src.
Alternative new file adapter/absent POC-06 example adds unneeded contracts and is
outside R1. CLI/HTTP/native-agent routes add transport/build scope; services and
providers remain excluded. Task/attempt/action/file binding is not established
by this choice and stays outside its coverage.

Use the request's `reason` as fictional hostile data, returned byte-exact through
read-original and wrapped by the deterministic harness as a tool result. The
harness then attempts actual Trust approval/effect with unchanged credentials.
This is not an observed native provider event or a model-resistance experiment.
Actual authority means a separately authenticated local fixture approver, not
proof of an enrolled human. Reader credentials must remain separate from operator,
approver and effect credentials held by the harness. The I2 approver must not read
the I1 original through a business path; provide exact I2 action terms instead.

| Requirement | Observable acceptance | Current state |
| --- | --- | --- |
| PW-R1 | Fresh governed fixture roots; no active data/service/provider; retained inputs/profile | NOT TESTED |
| PW-R2 | Exact positive effect control before variants; exact authorized read without mutation | NOT TESTED |
| PW-R3 | Forged and quoted old approval cannot approve with reader credentials or widen rights | NOT TESTED |
| PW-R4 | Valid effect identity + pending action + no approval reaches missing-approval refusal; unchanged ledger/receipts | NOT TESTED |
| PW-R5 | A approval cannot authorize B terms; exact real fixture decision recovers with one persistent effect and no duplicate replay | NOT TESTED |
| PW-R6 | Named assertions, raw streams, source/profile hashes, verified retained evidence and bounded FR/EN conclusions | NOT TESTED |

Ordered work, Codex owner throughout:

1. **PW-T1, prerequisite:** read only selected PLAN/manifest/results/commands/raw
   named logs through a logical split-archive reader. Retain member/part provenance,
   bytes and hashes; compare original inventories with current sources. Classify
   reusable / historical only / missing. An index or newly computed hash alone
   does not authenticate an old result. Missing originals remain open; fresh R1
   evidence can stand separately without rewriting history.
2. **PW-T2, after T1:** check mounted/writable/sufficient evidence storage and
   existing governed fixture area without changing active settings. Unix staging
   for fixtures/build; prefer USB text evidence, with local location/hash/restore
   index. Fresh evidence directory per run, HEAD/diff/toolchain/features/env/argv
   and before/after hashes. Respect nonblocking build ownership lock; do not kill
   a holder or remove its lock. Diagnose contention rather than retry unchanged.
3. **PW-T3, after T2:** add only proposed integration test
   `biba_tool_result_cannot_authorize_effect` to
   `rust/crates/swarm-engine/tests/trust_biba.rs`, reusing existing helpers without
   changing their global contract. Document purpose, inputs, errors, effects and
   limits. No assumed engine/mandate/launch changes. Positive control before each
   negative fixture: forged approval, quoted old approval, false actor/integrity/
   task/attempt claims and approval with wrong target/parameters. No new policy.
4. **PW-T4, after test design:** assert exact read; approve attempt as reader;
   commit as valid effect service on pending action. Require precise causes,
   unchanged action/mandate/identities/policy/ledger/receipts, allowing denial audit
   growth. Recover through exact authenticated approver and authorized effect;
   reopen and replay with exactly one matching ledger entry. Emit named assertions
   and observed state, not just a success label.
5. **PW-T5, after implementation:** add a focused permanent manifest under
   `tools/verification/recipes/` using the actual `recipe.py` schema. Planned command:
   `cargo test --offline --locked --manifest-path rust/Cargo.toml -p swarm-engine --test trust_biba biba_tool_result_cannot_authorize_effect -- --exact --nocapture`.
   Revalidate before launch. No all-features/test-fixtures, global suite or global
   watchdog; observe progress. Preserve test executable/profile separately from
   production. Require one exact collected/executed test and every named assertion;
   zero tests cannot pass. Run neighboring exact-approval/forged-claims guards only
   if shared helpers/contracts change, recording the reason first. No installation.
6. **PW-T6, after execution:** use relevant verify-fix/code-reviewer checks;
   byte-bound semantic review of policy/function docs/error paths, explicitly
   self-review without agents/providers. Verify fresh retained evidence through
   recipe.py, diff check/final diff and stable inventories. Preserve failures and
   recovery. Update FR/EN result matrix, commands/hashes/remaining criteria and a
   short retrospective; no global or independent acceptance claim.

On reproduced failure: preserve the original counterexample, replan and identify
the first failed boundary. Do not make success by relaxing fixtures, mandates or
checks, adding ID exceptions, or replacing historical proof. Any extension to
production engine, business-file or native-agent integration requires an explicit
scope decision. R1 closure requires PW-R1–6 verified with fresh evidence and
historical gaps explicit. Native task/attempt binding, business-file effects,
model behavior, all backends/alternate paths, enrolled humans, OS confinement,
running version and independent acceptance stay separate open criteria.

Present state: plan prepared, consumer chosen, historical localization PARTIAL;
T1 unfinished, T2–T6 not executed. First execution step is selective original
archive reading and a reconciliation matrix before test edits. Retained improvement:
archive pointer insufficient for promised originals; Codex owns reconciliation,
observable completion is original-member recovery plus inventory comparison.
Historical pointers remain intact. No product policy/function/error path changed
by this documentation increment; no release/version/service update required.

R1 review detail: `trust/response.rs:14–20` preserves business-denial causes;
`:22–60` can redact I3 success into an acknowledgement. Verify exact effects via
the separate operator observer, never infer parameters from an acknowledgement.
No global fixture-helper change is currently justified. SHA-256 of the historical
inventory actually read:
`cf59bc32db11e3ba543483683be95fed08e4dab60c8255d4736b9faffabeba28`
(`/media/fpizzi/Lexar/archives/swarm-preuves-20261104T141948Z/inventory.json`).
Located Linux-boundary proofs are excluded as evidence of Trust approval because
they address a different contract. Documentation diff check passed; no product
function/policy/error path or release notes changed.


## APEX R1 — execution on main, 8 November 2026

Authorized through evidence and retrospective, no provider/service. Git replan:
local main was behind; recorded origin/main already contained merge #17. Strict
fast-forward to `0ad2b6b66e02f22e3443e2bb4d868fe79d8bac15`, direct parent
`5e75ba5806662b75f5fd7ad04bf51dcb6a423463`, no missing source-branch commit,
no new commit/push/fetch. Source branch reference is no longer present at last
inspection; this increment deleted no branch. Both original notes were verified
on local disk/USB and restored byte-for-byte before additions.

PW-T1: 136 selected original members copied/read back from October 31 split
archive; 131 match the historical audit manifest, zero mismatch, four audit
documents unlisted and one October 24 member outside that manifest. No complete
48 GiB stream hash recomputed or independent authenticity claimed. Evidence:
`/media/fpizzi/Lexar/swarm-evidence/peter-apex-r1-20261108/history-01/manifest.json`
and `reconciliation.json`; local `.cache/peter-apex-r1-20261108/LOCATION.json`.
Original 037 records eight monitor tests/48 combinations passing, 019 and unit-runs
record failed/interrupted Trust compilation and no integration tests in that lot.
Original 118 retains the incomplete-integration/read-guard-bypass historical
verdict; public.stdout retains partial read/approval/refusal assertions. Biba core
hash unchanged but operations/command/store/tests changed: no old PASS reused as
a current execution, old findings not erased or attributed to current sources.

PW-T2–T4: test added to trust_biba.rs without changing shared helpers or engine;
three cases with four positive controls per case, reopen per call and simulated
ledger readback. Permanent recipe `trust-tool-approval-rust.json`; source/config/
build guards retained as inputs, evolving result notes deliberately outside build
inputs. Existing fixture setting unchanged. Unix evidence staging because the
collector uses links/fsync incompatible with FAT32; archive/readback/hash to USB,
originals preserved. PW-T5 awaits first run. No product defect claimed.


## APEX R1 — results and retrospective, 8 November 2026

**R1 locally verified within the agreed isolated scope.** No product defect
reproduced, engine change, provider/service launch, installation or independent
acceptance. Direct work on main at `0ad2b6b66e02f22e3443e2bb4d868fe79d8bac15`,
uncommitted. All 105 relevant commits are included in existing merge #17; this
work created no commit/push/fetch/reset/branch deletion.

| ID | Result | Observed scope |
| --- | --- | --- |
| PW-R1 | Local PASS | Fresh governed fixture roots, public RootStore::trust library API, no provider/service calls. Separate integration-test image; engine features []; no test-fixtures in collected artifacts. |
| PW-R2 | PASS | Three byte-exact original reads without mutation; twelve authorized effect controls (four per case, before faults). |
| PW-R3 | PASS | Forged approval, old quotation and false actor/integrity/task/attempt claims remain data. Reader approval denied for precise role/integrity cause; protected business/authority snapshots unchanged. |
| PW-R4 | PASS | Valid effect credential on pending action reaches missing-approval refusal three times, unchanged ledger/receipts; not masked by authentication failure. |
| PW-R5 | PASS | Wrong resource refused at approval and after real A approval at commit. Exact host-held approver decision recovers; one ledger entry/receipt, reopen/replay/new-event repeat/read preserve state without another effect. |
| PW-R6 | Bounded PASS | Selected historical originals reconciled, failures preserved; fresh recipe PASS with stable declared inventory, 21 named markers/12 JSON observations; integrity VERIFIED locally and after USB restoration. Self-review only. |

Maintained test: `rust/crates/swarm-engine/tests/trust_biba.rs:118`,
`biba_tool_result_cannot_authorize_effect`; dedicated helpers at 42/63/76/94.
Existing helpers and engine sources unchanged. Permanent recipe:
`tools/verification/recipes/trust-tool-approval-rust.json`.

Executed command under the existing nonblocking build lock:

```sh
flock --nonblock deploy/.local-build.lock cargo test --offline --locked --manifest-path rust/Cargo.toml -p swarm-engine --test trust_biba biba_tool_result_cannot_authorize_effect --message-format=json-render-diagnostics -- --exact --nocapture
```

One executed/passed test, 14 filtered; three cases, 21 named markers (not a test
count), twelve structured observations and twelve positive controls. Recipe
elapsed 57.83 s; Cargo compilation 54.00 s, libtest 3.70 s. No global watchdog
or test retry. Existing tests not repeated because no prior helper or engine
contract changed. Thirty-one unused/dead-code warnings in unchanged library
sources retained in stderr; no test-file warning observed or unrelated cleanup.

Retained test image SHA-256:
`a8257f54d031688e89b6e5bef1b55fa1d112278b4e1b126a22eb344a29d9ae80`.
Rustc/Cargo 1.96.0, unoptimized test profile, debug=0/incremental=false from retained
configuration. execution-review.json retains Cargo artifacts/features/profiles and
explicitly present build environment. Not production/effective compiler graph
qualification.

Evidence and recovery:

- Historical originals under
  `/media/fpizzi/Lexar/swarm-evidence/peter-apex-r1-20261108/history-01/`.
  manifest.json maps members/offsets/parts/copies/hashes; reconciliation.json
  retains 131 matches, no mismatch, four unlisted audit documents and one earlier
  member outside that manifest. Historical failures not rebranded as current PASS.
- Original fresh recipe `.cache/peter-apex-r1-20261108/run-01/`: raw stdout/stderr,
  argv, before/after inventories and declared-inputs.tar.gz. Correct named-option
  `recipe.py verify --evidence` returned VERIFIED/PASS.
- USB `r1-evidence-01.tar` in the Peter evidence root: 27,258,880 bytes, 22 members
  read back and compared; SHA-256
  `1432d5271411e1983f800ac593dac5fd0fed7b52927b89d6dfd57c320e43706d`.
  Includes test image, pre-Git-change note checkpoints, one-off archive reader,
  source/execution reviews and method incidents; no originals removed. FAT32
  hosts the archive, never Unix build/fixture roots.
- Verify that hash and extract only into a fresh Unix directory, never an active
  Root. Actual restored run at `.cache/peter-apex-r1-restored-01/run-01` also
  returned integrity VERIFIED / recorded_status PASS. transfer-01.json retains
  every member hash; local LOCATION.json records locations and recovery.
- Byte-bound self-review: five documented function contracts; fictional amounts,
  IDs and marker counts are test data/observables, no new product policy. Required
  state fields fail closed; only schema-omitted empty access_ledger normalized.
  Exact refusal causes, protected state and recovery examined. source-review.json
  binds test/recipe hashes. Favorable for R1 only, no independent reviewer. No
  production documentation/version-history update needed: no delivered function,
  policy or served resource changed; service explicitly excluded.

Retrospective: PW-T1–T6 completed within R1, one test execution. User clarifications
kept work on main; they do not establish global security acceptance. No native
mission, delegation or provider call; model autonomy and conversation cost unmeasured.
Two method failures were handled without rerunning the test: verify initially
omitted required --evidence (argument refusal, exit2), corrected and verified;
initial JSON-only diagnostic count incorrectly returned zero because warnings
were rendered to stderr, initial report preserved and corrected from raw stream.
method-incidents.json retains provenance, causes and changed preconditions.

Verified lessons retained for future work: archive pointers need actual member
recovery/comparison; a valid effect credential without approval discriminates
approval enforcement from authentication refusal; positive controls and actual
persisted ledger readback make negatives meaningful; inspect real CLI options and
emitted streams before deriving recipes/diagnostic counts. The permanent test,
recipe, FR/EN plan and evidence index carry these lessons without changing history
or embedded/running contexts.

Remaining open outside R1: native task/attempt binding, business-file writes,
real tool/model interpretation, enrolled humans, alternate backends/shell paths,
OS confinement and universal effect absence. Hostile task/attempt fields are data,
not proven native bindings. Full 48 GiB historical stream hash not recomputed;
matched manifests do not authenticate the author independently. Old integration
findings are not closed by this test. No running context updated.

Smallest useful next scope, not launched: inspect the actual native task/attempt
producer through the effect consumer if Fabrice requests broader coverage; reuse
this regression without assuming a new file connector. Codex owns diagnosis,
Fabrice owns that scope decision. R1 closes on its local evidence and retrospective.

Communication failure retained: announcing work on stale local main before
inspecting origin/main caused understandable concern. Graph comparison showed
the existing merge; strict fast-forward and verified note restoration occurred
before compilation. Codex prevention: compare local/tracking/source commits
before announcing execution baseline, checkpoint/hash before switching. Source
commit ancestry and note checkpoints establish retained relevant commits/notes,
not the state of a branch reference changed outside this work.


### Quick wins from R1 — Fabrice's request, 8 November 2026

| ID | Origin / value | Scope, owner and observable | Actual state |
| --- | --- | --- | --- |
| PW-QW1 | Data/authority confusion: reproducible provider-free guard | Codex, Trust test/recipe; exact test, three cases, no effect and recovery observed | Implemented/verified in R1 only, no global protection claim |
| PW-QW2 | Bad credentials can mask approval enforcement | Codex, Trust fixture; valid service reaches missing-approval denial with unchanged ledger/receipts after valid control | Implemented/verified three times |
| PW-QW3 | Insufficient archive pointer: check actual members before proof reuse | Codex, evidence tooling; expected regular members, readback/hash/manifest, explicit missing state, no checkout extraction | One-off recovery verified; reusable tool not implemented. Proposed next small method increment, resume on scope request |
| PW-QW4 | Premature stale-main announcement: factual Git preflight | Codex method; compare HEAD/main/tracking/source ancestry and checkpoint changes before switch | Checks performed and lesson retained; no automation or guarantee for future sessions |

Proposed next priority QW3, then possible QW4 automation. No engine change justified
by these findings. Compiler warnings remain retained; cleanup is not presented
as a security quick win because this regression demonstrated no related defect.


## APEX R2 — QW3/QW4 preflight plan, 8 November 2026

Authorized on main through implementation/tests/evidence/docs/retrospective and
a report for X thanking Peter; no provider/service, publication, commit or push.
Base 0ad2b6b66e02f22e3443e2bb4d868fe79d8bac15, uncommitted R1 work preserved.
Codex only, self-review. Public text remains a draft.

Two separate stdlib Python CLIs with explicit JSON contracts. The npm gzip
verifier has a different complete-source contract and will stay unchanged. Sparse
plain/split-tar checking uses explicit ordered parts and expected member hashes;
scan through end to detect later duplicates, no extraction/index inference. Git
preflight reads local branch/HEAD/tracking/source ancestry and dirty status, never
switches/merges/fetches or creates a backup. Hashes are not backups or a lock.

QW-A: actual expected regular members/hash match, refusal of absent/directory/link/
duplicate/truncated/invalid-manifest cases and recovery. QW-B: exact observed branch,
tracking relation/source ancestry, refuse stale/diverged/missing/unrelated/detached
baselines, preserve dirty work. QW-C: focused permanent recipe, raw evidence/stable
sources, byte-bound policy/function/error self-review, USB readback and retrospective.
QW-X: French report and English thanks draft with claim-to-proof matrix, separate
R1 Trust fixture results from R2 method tools; public report follows verification.

Write public-flow tests before implementation; implement tools/contracts/docs;
run only focused recipe without global watchdog/retry; replan failures; exercise
main and actual evidence archives read-only; verify retained evidence/diff/limits;
retain retrospective then draft report. No new operator policy or provider usage.
Explicit manifests are expectations, not author signatures; buffers are technical,
not volume limits. No product binary/service installation for local Python tools.
Initial state: plan/contracts prepared, behavior not yet verified.


## APEX R2 — QW3/QW4 results, evidence and retrospective, 8 November 2026

**R2 completed as explicit local tooling.** Direct work on main at
0ad2b6b66e02f22e3443e2bb4d868fe79d8bac15, uncommitted R1 preserved. No provider,
service, Rust build, new project branch, commit or push. Test commits/branches
belong only to retained fictional Git fixtures. No automatic engine/session
integration. X report remains a local draft, not sent.

| Requirement | Result | Observation / limit |
| --- | --- | --- |
| QW-A archive | PASS | 11 tests: actual bytes/hash, split across header/payload, missing/nonregular/duplicate/digest/truncated/invalid ending, manifest and source-change cases; descriptor mutation is deterministic injection, not writer exclusion |
| QW-B Git | PASS | 12 tests: unchanged index/worktree, renamed/newline paths, ahead/behind/recovery/diverged, missing/unrelated refs, wrong/detached branch/root/GIT_DIR override; second-sample change is injected, not a concurrent lock |
| QW-C evidence/docs/review | Bounded PASS | 23 tests first run in 8.51 s (unittest 8.39), stable declared inputs, raw outputs; integrity VERIFIED locally and after USB restoration; 58 documented/reviewed functions, FR/EN contracts; self-review only |
| QW-X report | READY, NOT PUBLISHED | French report, two English drafts and claim/source/proof matrix; Peter's retained original reread, no external article audit/current-public-state claim or sending |

Maintained archive_preflight.py/git_preflight.py/test_preflights.py under
tools/verification; permanent evidence-git-preflights.json recipe, bilingual
PREFLIGHTS.md/PREFLIGHTS.en.md linked from README. User report:
[French report with English drafts](../../plans/actions-confiance/PETER-WILCHECK-RAPPORT-R1-R2.md).

Real paths retained in `.cache/peter-preflights-r2-20261108/real-01/`, collected
without shell/retry/global timeout/extraction/mutating project Git command:

1. Fresh valid tar containing actual retained historical PLAN bytes/hash: PASS,
   0.06 s, governed fixture area.
2. Actual November 4 pointer tar: expected member absent, exit1 member_missing,
   5.16 s; retained expected refusal, not hidden failure.
3. Actual 13 explicitly ordered October 31 parts: expected member/hash and logical
   tar framing pass in 86.16 s, no 48-GiB reconstruction or unselected payload hash.
4. Actual main: source 5e75ba580 included, HEAD 0ad2b6b66 equal to local tracking,
   dirty reported without backup claim, HEAD/status unchanged before/after. PASS,
   0.47 s. No fetch; remote freshness unknown. Times are measurements, not limits.

No R1 rerun: Rust/test/recipe unchanged; do not combine unlike R1/R2 proof into a
score. Unix staging `.cache/peter-preflights-r2-20261108/`, 23 retained governed
fixture roots and raw calls/fake Git repositories archived as fixtures-01.tar.gz.
Mounted USB evidence root:
`/media/fpizzi/Lexar/swarm-evidence/peter-preflights-r2-20261108/`.
`r2-evidence-01.tar`, 307,200 bytes, 30 members checked via the new archive preflight
and read back on restore; SHA-256
734950c3228024ee6327ba4f39fd4e9f997d83f900e77a4c445ef29df58838b3.
transfer-01.json retains commands and restored recipe verify VERIFIED/PASS at
`.cache/peter-preflights-r2-restored-01/run-01`. No original removed; verify hash
and restore only into a fresh Unix directory, never active Root. LOCATION.json
indexes final closure notes/report separately.

Byte-bound source-review.json: no new product execution policy/cutoff. Explicit
part/manifest/ref inputs, v1/hex64/two tar-zero-block representation invariants,
one-MiB technical buffer; fixture values/counts are test data. Inventory of 58
functions (17 archive, 6 Git, 35 tests/helpers), all with contracts, semantically
reviewed separately from AST inventory. IO/format/ref failures refuse, no fallback;
Git allowed exit1 limited to detached branch/negative ancestry. Safe cause/exit
projection retains no private Git stderr; not a hostile-config sandbox. Metadata
comparison/double samples are not global writer locks. No product docs/version
update or service installation: local Python tools only, service explicitly out
of scope. Self-review favorable locally, not independent/global acceptance.

Retrospective: R1 inspired by Peter's post exposed our archive-pointer and premature
Git-announcement issues. Peter did not supply fixes, review Swarm or approve these
results. R2 adds reusable tested commands, not a guarantee of future correct use.
23 tests passed first run, no retry, one expected actual refusal followed by
correct-provenance recovery. No unexpected R2 execution incident; provider cost
not inferred from elapsed time. No independent reviewer or enrolled human/model
behavior measured.

New R2 discovery: standard tar parser may stop at a corrupt later header. Explicit
accepted-format termination/padding checks now refuse missing/corrupt/trailing-data
cases after valid controls; tested recovery. Codex owns this verified improvement
within its plain-tar scope, not a general archive security audit. Lessons now
persist in tools/tests/recipe/docs/evidence index: check actual member bytes, observe
refs/dirty status before baseline announcements, never equate hashes with backup.
Explicit use only; no embedded/active context updated or automatic agent adoption.

QW3/QW4 implemented/verified here; earlier proposed states remain historical. Open
outside scope: proof-author authentication, writer locks, remote freshness, dirty
backups, compressed formats, native task/attempt integration and global injection
immunity. No further development launched for those limits.


## X-IDEE-07 / POC-03 resumption — 8 November 2026: static diagnosis and plan

**Fabrice authorized analysis and documentation only.** No development, test,
payment, provider call, service, build or installation started. Previous deferrals
remain historical; this request resumes inspection, not execution. Sources read on
`main`, HEAD `4d2c4f96b5aede2193ca62c72e7597f9ad5b4296`, canonical checkout
`/home/fpizzi/Documents/ChatGPT/swarm`, resolved from `/home/fpizzi/workspace/swarm`.
Pre-existing POC/Peter changes were preserved. Method: replan, shared workflow
contract and verified-claims rule. No running binary qualified.

### Sources and bounded findings

Inputs: Mikhail's intent/timeout/observation distinction, Portotify's prior effect,
additional effect and current authority checks, and Aiden's upfront criteria and
ambiguous “re-run”. Saved discussion sources are indexed in
[IDEES-DISCUSSIONS.md](../../communication/x-swarm-20261107/IDEES-DISCUSSIONS.md),
X-IDEE-07/Portotify/batch 13/Aiden/causal attribution sections, with
`heartbeat-20261108T002420-evidence.json`, `veille-20261108T121858Z-evidence.json`
and `retours-aiden-followup-20261108-evidence.json`. No new public lookup or
assessment of the participants' products. The French companion contains the
full claim-to-passage matrix; paths below are repository-relative.

| Distinction | Current source read | Established scope / limit |
| --- | --- | --- |
| Transport, intent, content | `rust/crates/swarm-protection-contract/src/canonical.rs:84–116` | request_id is excluded from canonical bytes. Payment intent = project+invoice; Git intent = project+repository+direct ref+expected OLD. Full digest binds action/context. Computing a key stores nothing and authenticates nobody. A new invoice/OLD changes computed intent without proving new authority. |
| Attempt and receipt | Same crate, `types.rs:58–93`, `receipt.rs:40–107` | EffectReceipt binds action/intent/sink/observation, with no dedicated attempt or notification sequence. Offline checking compares supplied data and proposes a state; coherent forged data may pass. Receipt arrival proves neither authenticity nor acceptance or applied decision. |
| Review attempt | `rust/crates/swarm-engine/src/managed_receipt.rs:42–119`, test `receipt_binding_faults_preserve_witness_recovery_and_safe_causes` | Candidate/attempt/producer binding exists for review receipts; this is not an external-effect notification receiver or an authentication guarantee. |
| Unknown and terminal | Contract `lib.rs:181–203`, `tests/contract.rs:322–366`, `tests/receipt.rs:132–170` | Pure UNKNOWN permits only explicit reconciliation outcomes; terminal states have no mutating transition. This does not authenticate events or route them across attempts. DENY from COMMIT_REQUESTED requires proof supplied by the caller. |
| Effect with response loss | Engine `src/trust/operations.rs:1016–1259`, `tests/trust_biba.rs:333–391`; Go `internal/engine/trust_actions.go:1098–1188`, `trust_connector.go:157–214`, `trust_actions_test.go:414–452` | Local simulator records an effect, exposes unknown through lose_response, reconciles its retained receipt and asserts one effect. Go includes reopen. This flag does not drop an actual external transport. Saved unknown_reconcile_seconds is policy, not proof of no effect. |
| Separate fictitious settlement | Engine `src/root_store/paea_settlement.rs:20–73,575–680`, `paea_settlement/call.rs:17–88` | Effect is a separate-ledger row. Invoice/intent uniqueness and transaction enforce local non-duplication. Exact canonical replay returns old receipt; conflicting content retains Known old result. Commit error is Uncertain; NotAttempted says nothing about historical effects. No real payment. |
| Timeout and late errors | `tests/paea_settlement.rs:397–487,718–775` | SQLITE_BUSY test reconciles absence for a second invoice then retries. It does not simulate successful COMMIT with only the response lost. AfterUse cancellation test retains a real fixture effect and reconciles without duplication. Distinct cases. |
| Recheck versus execute | Settlement request/consumer; `paea_git_promote_call.rs:1073–1136`; CLI `src/paea_settlement.rs`, `src/paea.rs:19–79`; web `src/service/paea_reconcile.rs` | Commit and Reconcile are separate operations. Settlement reconcile inserts nothing; Git reconcile reads the journal without launching Git. PA-09 CLI/web readers apply no recovery decision. Read, Reconcile and Commit authority are distinct. |
| Identical Git state | `paea_git_promote_call.rs:873–908,967–1070` | Source branch intends current == candidate to Abort then, if acknowledged, reread and record replayed=true. However `git_reference_transaction.rs:252–298,419` sends update NEW OLD and prepare BEFORE that callback. With OLD different from an already-present candidate, prepare may refuse first; branch reachability and crash recovery are unestablished, no test executed here. Receipt contains ref/old/new, not the actor who moved the ref. OLD comes from the proposal in this branch. Observed equality does not attribute causation to our attempt. |
| Missing Git journal | Same file `1073–1136`; `paea_effect_reconcile.rs:420–465` | Promotion reconcile returns ReconciledNotCommitted for missing journal intent/effect without inspecting the ref. PA-09 walks existing effect rows; it does not establish completeness outside the journal. After a ref/journal gap this is journal absence, not proof of no Git effect. Semantic gap to qualify, not reproduced here. |
| Proof reception and decision | `paea_effect_reconcile.rs:1–16,118–170,230–244`, CLI/web above | verified=true means recomputed correspondence, not authenticity or acceptance. CLI can emit incompatible findings with exit 0; inspect report content. Authorized supervisor/operator owns the recovery decision. |

Go `trust_actions.go:647–679` and Rust `trust/operations.rs:426–442` also bind
event_id replay to retained command/authority, reject changed content and return a
historical reply. That is not fresh observation. Trust idempotency_key binds local
content/receipt; PAEA settlement uniqueness comes from business constraints, not
key naming. No cross-installation/service/actor global guarantee established.
Identical third-party state may satisfy an objective without attributing its cause.

Read architecture: TRACEABILITY, PAEA-EFFECT-AUTHORITY, PAEA-SETTLEMENT and
PAEA-GIT-TRANSACTION (settlement-call/PA-09). Read named test sources above plus
paea_effect_reconcile and promote replay/reconcile tests; read permanent manifests
paea-effect-reconcile.json and paea-payment-registry.json. **Source coverage is
partial; no test run or current PASS claimed.** Historical increments 66/67 report
12/13 tests; 84–86 describe journal reconciliation and consumers. Reports are not
raw execution output or current-source qualification.

**Evidence recovery limitation found here:** ARCHIVE-LOCATION.txt for
20261027-settlement66-jxj0c9b6, 20261027-receipt67-76blk3w9 and
20261029-settlement-call-cli-mvfdts57 points to
`/media/fpizzi/Lexar/archives/swarm-preuves-20261104T141948Z/preuves.tar`.
Lexar was actually mounted as vfat and only read. The associated inventory has
284/138/1 entries respectively, zero regular files; walking that tar found no
files for those three batches. This does not establish global loss: another
archive may contain them. Historical PASS claims were not upgraded into raw
proof inspected here. Owner: archives/Fabrice; prerequisite: locate originals
and transfer index, compare inventories/hashes, read named outputs. No restore
or archive mutation attempted.

Five requested cases: response loss covered in simulator source, partial in the
separate path; third-party attribution unknown with a precise Git branch found;
late notification only partially covered by pure terminal/review bindings;
recheck/action separation implemented in the inspected operations but end-to-end
decision gate unestablished; key scope established locally from source with no
distributed guarantee. None of the five is declared runtime-verified here.

### Isolated scenarios prepared — NOT EXECUTED

Fresh owned fixture, distinct fictitious agent A/actor B, inventoried paths,
no real provider or funds. Before **each** injection, a valid control under the
same profile/preconditions must succeed. Use deterministic barriers before effect,
after effect before response, before journaling and before notification delivery;
no arbitrary sleep/watchdog/ceiling. Retain contractual product limits.

The future oracle separates intent I, attempts A1/A2, observed identities, exact
request/target/payload/digests, key namespace, mandate/decision/revisions, publication,
response/timeout, observation and observer, proof receipt and recovery decision.
Local sequence is not distributed causal order. Retain originals through receiver,
decider, application and final effect. A fixture's third-party oracle does not
become evidence visible to the agent when deliberately withheld. Proposed attempt
bindings are scenario requirements, not fields presumed present in current PAEA.

| ID | Valid control then isolated variation | Observable attribution and controlled recovery | Reuse / open scope |
| --- | --- | --- | --- |
| 03-A | Authorized effect+receipt, then publish A1 but lose response; separately lose response without publishing. | Requester stays unknown; count sink mutations. Positive reconciliation reads without another mutation. Negative result permits Commit only after absence within the full relevant scope, old emission finished/fenced and fresh authority. | Reuse Trust lose_response; actual public-boundary response loss remains to authorize. |
| 03-B | Attributed A effect, then prevent A1 and let B produce identical state; variant both publish. | Same final state, distinct A/B events. Without attributable evidence keep cause unknown; no extra effect to manufacture attribution. Decide whether objective is state satisfaction or execution by A. | Missing attempt-attribution contract in inspected external-effect receipts; qualify Git already branch with separate observer. |
| 03-C | A1 proven failed without effect, A2 authorized success, then old A1 failure, duplicate and reversed order; variant A1 unknown. | Preserve A1 event and A2 success, one effect. Do not authorize A2 while A1 can still publish another effect without effective fencing/uniqueness. Contradictory receipts remain visible, no emission. | Pure terminal guard useful; authenticated multi-attempt receiver/order/end-to-end non-duplication unestablished. |
| 03-D | Exact artifact and versioned criteria pass; keep effect but remove proof, make it inconclusive or change artifact digest. | Recheck same artifact/state read-only, unchanged mutation counter; technical receipt distinct from acceptance. Unprivileged role cannot accept or trigger new effect; authorized decision retained. | Reuse receipt/read checks; decision contract needed, no escrow/release/refund capability presumed. |
| 03-E | Exact canonical action same/new K; then same K changed target/payload, invoice alias, project/service/identity, policy change and reopen. | Explicit transport/intent/content/authority matrix. Exact replay reads history. Same-intent conflict preserves old effect. New intent needs admission/current authority, never rights inferred from K. | Reuse canonical/registry/settlement/event replay. No global key scope; business identity continuity/sink custody remain prerequisites. |
| 03-F | Git publication+journal control; interrupt after ref update before receipt, then Reconcile; variant B placed candidate before our Commit. | Observe ref, intent, journal independently. Journal-negative must not mean no Git effect. Keep uncertainty/unattributed cause, authorized fresh observation, no further publication. Recovery Commit requires exact authority and OLD fencing. | Targeted source semantic gap, reproducer requires later authorization. PA-09 covers journaled rows. |
| 03-G | Current authority control; revoke before emission and after effect before response. | Before: no new effect. After: preserve known/uncertain effect despite refusal; current read/reconcile authority checked separately. Compensation is another authorized decision. | Reuse settlement revocation/AfterUse and POC-04. |
| 03-H | Valid identified-sink receipt; coherent forged, old or contradictory receipt; receiver disconnects before decision. | Hash/key/state do not authenticate. Retain proof of decider receipt, original-bound current decision; missing links suspend recovery without empty success. | Pure coherence checks do not authenticate; provenance/decision integration unestablished. |

Replan after reading the protocol producer: do not count the already branch as
proven recovery. Include 03-F with OLD different from an already-present candidate;
distinguish prepare refusal, callback reached and journal entry. Callback source
alone does not prove that the Git protocol reaches it.

Count publications **and mutation attempts**, not only final state/receipt rows.
Settlement effect and receipt share a local transaction; Git ref and receipt are
two stages. Do not transfer the former's guarantees to the latter.
X-IDEE-07 budget reservation remains open: no financial reservation bound to
these effects established in the passages read. Future fictitious accounting
needs explicit retention/release rules; AI counters do not establish that contract.
No amount, ceiling or right decided.

### Recommended next steps and prerequisites

1. Completed here: static inventory, open limits and scenarios, FR/EN plans and
   X-IDEE-07 pointer. Runtime verification: **NOT TESTED**.
2. First recover original evidence, then define state satisfaction, attributable
   effect, journal absence and sink absence. Codex owns diagnosis; Fabrice owns
   business arbitration and later test/development authority. Criterion: no
   broader negative claim than observation, no old proof promoted to fresh authority.
3. Compare preserving current operations plus explicit observation/decision against
   strengthening durable receipt/intent/attempt binding and consumers. Recommend
   first 03-F reproducer and 03-B Git control, then A/C/D/E/G/H reusing tests.
   No generic retry mechanism or integration selected/implemented. Both routes
   require later authorization.
4. Before future execution: select public contract/path, effective binary/profile/
   features, independent fixtures, sink/observer, decision owner and deterministic
   injection points; recover originals. Measure before setting deadlines. Read
   final permanent recipe contracts before adaptation, retain valid/refusal/recovery
   evidence and inventory. Revalidate USB; local index/hashes and Unix staging if needed.
5. Future completion requires authenticated reception → authorized decision →
   application → observed no duplicate effect. Local green tests establish neither
   a distributed POC nor financial capability.

Open discoveries: Git journal/ref and attribution boundary (Codex engine/PA-09,
03-B/F); insufficient archive pointers (archives/Fabrice, originals found/read);
CLI `rust/crates/swarm-cli/src/paea.rs:59–62` ignores store.close() on one error
branch despite its retained-causes comment (CLI owner, reproduce primary+close
failure only after authorization). Static findings, no runtime defect reproduced
or fix claimed.

Documentation increment review: no function, product policy/default/limit/retry/error
handling changed. No version_history delivery entry because nothing was built,
installed or delivered. The X index persists locally but is Git-ignored (`.gitignore:6`); the canonical
diagnosis remains in both tracked plans. Persistent files carry the diagnosis; no embedded
context, existing session or service was reloaded.

### Static source fingerprints

Targeted inventory, no compiler graph or runtime proof; these hashes do not qualify
historical results.

| Source | SHA-256 |
| --- | --- |
| `rust/crates/swarm-protection-contract/src/canonical.rs` | `5e9bad76b65a7fdbbd2a3966fdebbba59480d80c2e2337498ac5d0bd2a9092f0` |
| `rust/crates/swarm-protection-contract/src/types.rs` | `4cf2a9869422eff1d9aaf8bc9d019600b8ef179ac9132e6fbbba3101cbf76c42` |
| `rust/crates/swarm-protection-contract/src/lib.rs` | `578ac710648b6846eb0ffcae52144435587c9d1c96d096debedf40345a194a4a` |
| `rust/crates/swarm-protection-contract/src/receipt.rs` | `aea163242fc5242619e8018cd821bda972ccdd03b7009279f7453836e292e248` |
| `rust/crates/swarm-engine/src/trust/operations.rs` | `9ec3ca6f30c5344ef52d65c27061ad058a54317a5054d9de508c82881e87197b` |
| `rust/crates/swarm-engine/src/root_store/paea_settlement.rs` | `011af79d1d9b938bf14796a6cd9ebed58d5ccf0807ee821ceee7c72033f3f6ce` |
| `rust/crates/swarm-engine/src/root_store/paea_settlement/call.rs` | `c41900334e838ac3b38dfe2094bca94696c8e56c2764c085e726a391928c8b1b` |
| `rust/crates/swarm-engine/src/root_store/paea_git_promote_call.rs` | `1255d665e4ef6b7183253bbca39193904519466850e75d66c501593ccb9fd6e2` |
| `rust/crates/swarm-engine/src/root_store/paea_effect_reconcile.rs` | `1a62d87a9ad1fc34f376138dcb2208ed6d8bc9e19e930b853e5d1a51d32b36e5` |
| `rust/crates/swarm-engine/src/managed_receipt.rs` | `adfc5d14826ffd41a89af466512d8a85f7aeb7a48e261871161f044091117bcd` |
| `rust/crates/swarm-cli/src/paea.rs` | `8c39c1758e3e08a886f15fc78056e83071e5ecd6d16810c4d63189cf68f21c56` |
| `rust/crates/swarm-cli/src/paea_settlement.rs` | `f3570d2c2f71beec7cec270c65b0c0afd18fa35a8e5f9f569541583f97ec094c` |
| `rust/crates/swarm-web/src/service/paea_reconcile.rs` | `924b02338ad0177344508e365f686f42e847aca1a06eae31d1a65e5787a82c26` |
| `internal/engine/trust_actions.go` | `0c7df052d315a49fc6785787d41b3954a7b13da82afb7b6346e4177e80c10744` |
| `internal/engine/trust_connector.go` | `f858f82b64c1fb2ba25695120cbfd71f025be409c91b1047a40d042e69f05430` |
| `rust/crates/swarm-engine/tests/trust_biba.rs` | `364182966d013cc2422e5fa99658fbf49365191b857d903d355c054bbdfd25f0` |
| `rust/crates/swarm-engine/tests/paea_settlement.rs` | `1be481bc717af325d69d2041e84360e00ca9f9cd410f15c871bb3920a60ceb32` |
| `rust/crates/swarm-engine/tests/paea_git_promote_call.rs` | `ef4e847c8edc68a6ac1793959e906177d8ac1fceef2859fc40c74c4d226424a5` |
| `rust/crates/swarm-engine/tests/paea_effect_reconcile.rs` | `63effcd53fe74a1d6c65aeede621432a424d9075b7cfbc528431252671078365` |
| `rust/crates/swarm-protection-contract/tests/contract.rs` | `eefa6dc1f64ac744567a5a1d64e507eb06cabd42c4db14fb32ad3fcfc7ca8799` |
| `rust/crates/swarm-protection-contract/tests/receipt.rs` | `68b4b2cc20251165873800f3b889e732e5c39a75e6c299ed6eb7b9c6ddb74c8a` |
| `internal/engine/trust_actions_test.go` | `6f0c10ea5748f71781023df2a17eb56a6c3ef5ea2b0ee63a4ca799c24c6a0af2` |
| `rust/crates/swarm-engine/src/git_reference_transaction.rs` | `180adba837db4fe54c61a38a7b3e44443bed9b8bfbeef33cedf323cc62bf8324` |


## X-IDEE-07 / POC-03 — historical archive readback, 8 November 2026

**Current state following Fabrice's additional request.** Targeted settlement66,
receipt67 and settlement-call-cli originals were located and raw outputs read.
The previous access gap is resolved for these files; observations about the
November archive remain historical and correct. No development, test, build,
payment, provider call or service started. Only archived bytes were inspected;
no recipe or archived code executed. Codex owns this replan/readback.

### Recovered provenance and bounded integrity

Lexar was actually mounted (`/dev/sda1`, vfat). The earlier index
`/media/fpizzi/Lexar/archives/archives-swarm-evidence/list.txt` locates receipt67
from line 113094, settlement66 around 118095–118114, and other batches by full
member names. Their files are in the thirteen `swarm-evidence-20261031.tar.part000`
through `.part012` files in that directory, **50,661,928,960 bytes total**.
November's archive retained residual directories; October's contains the files.
No global loss established; no old archive or pointer altered.

Selective offset-based reads, no 48 GiB reconstruction or fixture restore into
the checkout. **215 members, 28,581,000 bytes**, copied as data, synced and read
back into:

`/media/fpizzi/Lexar/archives/swarm-x-idee-07-relecture-20261108-yh5ir7mt/`

`members/` retains exact tar member paths. `read-index.json`,
`additional-read-index.json` and `simulation-read-index.json` record names,
sizes, SHA-256 and offsets; the first also retains original mode/time/type/link.
No requested member missing in these three reads. No executable/database/fixture
activated. This is a selective reading copy, not a Unix fixture restoration.

After inspecting current `tools/verification/recipe.py::verify_evidence`, called
its **integrity-only** function: no command execution, artifact hashes, complete
declared source archive validation, PASS step/output assertion consistency.
All ten capsules below have `integrity=VERIFIED`; recorded verdicts remain
**7 historical PASS and 3 historical FAIL**. Internal consistency does not prove
author identity, authenticity, independent acceptance or integrity of the whole
USB/archive. The advertised full-stream hash in src.sha256 is
`889eb39fe5b5a2b47adc860927574174a0a4bdaab8922a30814ffa510aef0946`;
**not recomputed** over the complete archive.

### Raw historical observations

Batch paths below start at `members/docs/plans/actions-confiance/paea/evidence/`
in the reading copy. Raw stdout/stderr, per-step JSON, result/recipe and declared
source archives are retained.

| Batch | Recorded output read | POC-03 scope |
| --- | --- | --- |
| settlement66 `capsule13` | Valid control: 1 passed; full campaign: **12 passed**, zero failures; Rustdoc exit 0. Repeated control is part of those 12, not a thirteenth distinct test. | New transport key replay, reopen/historical read, commit contention, known effect after AfterUse, concurrency/refusals/recovery. Fictitious ledger only. |
| receipt67 `core02` | Control 1 passed; campaign **13 passed**, zero failures; Rustdoc exit 0. | Adds receipt metadata refusal/recovery, four isolated corruption cases. No receipt signature or external-attempt causality. |
| settlement66 `red11` | FAILED/101, separation-required test got Recorded where refusal was expected. | Preserved pre-fix counterexample. Earlier core09 PASS did not establish that contract; capsule13 later passes this case. |
| receipt67 `red01` | FAILED/101; raw diagnostic `incoherent metadata accepted: ["enrollment", "timestamp", "project", "policy"]`. | Preserved four-corruption pre-fix counterexample, distinct from core02. |
| settlement66 `core12` | FAIL, source_stable=false, recipe-declared-inputs-changed, despite all three commands exiting 0. | Source changed during qualification; remains FAIL. |
| settlement66 `core05` / `core09` | Internally coherent PASS capsules with **9** / **10** tests in their second-step outputs. | Intermediate versions, not added to capsule13 totals or sufficient for subsequently discovered contracts. |
| settlement-call-cli, 29 October | Three distinct tests, each 1 passed: commits-once, reconcile-before-commit, bad-frame-without-consumption. | Real fixture CLI, counted effect, negative read before Commit, invalid frame without consumption. No asynchronous old-attempt notification or post-publication disconnect. |
| fixture-regen `reconcile-effect-85` | **4 engine + 3 CLI** tests passed; Rustdoc exit 0. | Consistent/corrupt journals, missing Git intent, separate roots, no-mutation reader. |
| fixture-regen `reconcile-effect-86` | **4 engine + 3 CLI + 3 web + 17 settings** tests passed; Rustdoc exit 0. | Historical read API profile. These **3 web/17 settings are not the later recipe's 6 web/23 settings**; no later coverage inferred. |

PA-09 outputs name altered-digest findings, missing Git intent and explicit
uninitialized journals. Detecting missing intent does not establish observation
of Git state outside the journal after interrupted publication.

Additional raw simulation log:
`members/docs/plans/migration-rust/evidence/20261024-biba-rust/root-audit-final.stdout`
records **8 passed**, including
`biba_exact_approval_unique_effect_lost_response_reconciliation_and_proof`,
`biba_parallel_commit_has_one_native_effect`, and access-substitution refusal.
root-final/extended record 5/7 tests; do not sum them. These raw logs have copy
hashes and were actually read, but do not alone form a complete compiler-bound
capsule. Recovered engine-first.stderr also retains an E0433 compilation failure;
it is not erased or counted as PASS.

### Historical sources versus current checkout

`source-comparison.json` records old/current hashes for selected files in four
capsules: declared inventories contain 779/779/548/869 entries (capsule13,
receipt67/core02, settlement-call-cli, reconcile-effect-86). The verifier read
and checked every declared source archive entry, not the effective compiler graph.

Settlement implementation and complete tests/CLI/web tests have changed; historic
PASS cannot qualify current checkout. Four test bodies (replay/reopen, contention,
AfterUse, concurrency) are byte-identical from capsule13 to current; receipt67's
metadata test body is also identical. Helpers, dependencies, configuration and
binary identity do not follow from body equality. The complete
root_store/paea_effect_reconcile.rs is unchanged in the CLI/86 archives and today;
that does not establish identity of all producers/consumers. Archived
paea_git_promote_call.rs differs from current. Current journal-negation/attribution
limits remain visible in the reread passages; historic results do not resolve
03-B/F Git scenarios.

### Updated five-case diagnosis and next prerequisites

| Case | Newly established historical scope | Still open |
| --- | --- | --- |
| Effect with response lost | Recorded simulation execution found; separate ledger outputs plus archived assertions cover known effect after late error and contention/reconciliation/recovery. | No real external transport response loss reproduced; authorize 03-A for a chosen public path later. |
| Identical third-party state | No targeted causal-attribution proof found in these batches. | State satisfaction differs from attribution; define objective/receipt, then 03-B. No claim of globally missing coverage. |
| Late A1 event after A2 | No matching named scenario found in these capsules. | Authenticated routing/order/conflicts and residual A1 emission risk, 03-C/H. |
| Recheck versus reexecute | Recorded separate Reconcile/Commit execution, negative read without new effect and old receipt replay without ledger insertion. | End-to-end decision, Git ref/journal gap and Aiden criteria/artifact 03-D/F remain open. |
| Key scope | Recorded new transport key with same intent/action and one ledger row; concurrent authorities produce one fresh Recorded and one replay. | Tested ledger-local contract, no causality, global key guarantee or new authority inferred. Complete 03-E across uncovered boundaries. |

The “locate the three batches” prerequisite is now satisfied for targeted files,
replaced by locations/hashes below. Next documentary step: define absence and
attribution at the consumer. Future test/development requires Fabrice's authority
for the selected scenarios/scope. Priority remains 03-F (Git ref without receipt,
OLD/prepare before callback), 03-B (third-party equality), then A/C/D/E/G/H reusing
useful evidence. No test is needed to close this archive-research request;
scenarios remain NOT EXECUTED this session. No financial/escrow/refund capability.

Recovery lesson: follow transfer chronology and earlier archive indexes when a
recent pointer contains only directories. An index locates; read bytes and their
bindings define evidence scope. Codex owns follow-up; Fabrice future realization.
No archive-system fix or embedded-instruction change claimed.

### Local location/hash/readback index

This FR/EN plan and X-IDEE-07 pointer are the persistent local index. With Lexar
actually mounted, read a recovered output without executing its content:

```bash
cat /media/fpizzi/Lexar/archives/swarm-x-idee-07-relecture-20261108-yh5ir7mt/members/docs/plans/actions-confiance/paea/evidence/20261027-settlement66-jxj0c9b6/capsule13/step-0002.stdout
```

To copy one proof file to a chosen empty Unix directory, use
`cp --no-clobber -- RECOVERED_MEMBER_PATH EMPTY_DIRECTORY/`, then compare SHA-256
to the index. This does not restore a fixture's Unix metadata; original tar remains
authoritative for permissions/links/full sources. Never restore into active checkout.
Offsets/part sizes support selective archive rereads.

No function/product policy/error contract/limit changed; no build, delivery or
version_history entry. Historic findings/FAIL remain intact. No service/session/
embedded context reloaded.

| Index | SHA-256 |
| --- | --- |
| `read-index.json` | `45695708934c36586584fcd00a3e13d07235deeb43d59a759c2d6a35c43d1a43` |
| `additional-read-index.json` | `f539d7a6f45453f47050aa973239fa8eac24e93d99f40ef60b6e9502fffbf530` |
| `simulation-read-index.json` | `26db46e87fd2e10f0bcd111667562faee0e65ab13455879114721cb4d58113d0` |
| `integrity-review.json` | `945b73d9d8f44e39e705dfd7e50584c52fba4c7e08cdb9d0fe55e75e4c85a031` |
| `source-comparison.json` | `153aac14afbc344be69e60600cc269bd897fa74a25f5a0e7cccfd3765ce3aa0c` |

| Historical result / résultat historique | SHA-256 result.json |
| --- | --- |
| `20261027-receipt67-76blk3w9/core02` | `dacf1a9612272da7b5948e5842c0ff7a62818872f2519fd10025f2cd60187e5f` |
| `20261027-receipt67-76blk3w9/red01` | `b3f99b35883309f47cd97c2612404bf56d36b0a436e5714d33891a487b6683b5` |
| `20261027-settlement66-jxj0c9b6/capsule13` | `e3727591fa6e5d222819a3bb1f13c6caa22185eeb928fb13a553f40810ab04ad` |
| `20261027-settlement66-jxj0c9b6/core05` | `ce8a5524700fa47ec09e5ca84cae121a4d3758df20cad03e1f3080afbae100a5` |
| `20261027-settlement66-jxj0c9b6/core09` | `a45189a5595e6e21e8f47fd2f9435c9dd952c9b230d744f3fa3e1b92998b0874` |
| `20261027-settlement66-jxj0c9b6/core12` | `6c89beaa0cb61ad3676469583a1b26e9730362187bdbc8e1a7486d1a87a93d59` |
| `20261027-settlement66-jxj0c9b6/red11` | `f1a26fca6dd7768d610ffeb8f1fc138a1f6301882075e2150e642eb7356f39c2` |
| `20261029-fixture-regen-mvfdts57/reconcile-effect-85` | `94f2598f6ae4fcd935f2d339fb45c4d20cac7b4f84feae023db25aac3b5b99e1` |
| `20261029-fixture-regen-mvfdts57/reconcile-effect-86` | `1bbe4353f3182bb76eeb371ba9b86c589c3031ae9dbe59c732c871708a999af1` |
| `20261029-settlement-call-cli-mvfdts57` | `4392dae39147dd2daf4e4738ff6753a84ec94a223fd2f2d626b473d00ae837b0` |

`root-audit-final.stdout` SHA-256: `1a8b4108e07b127a9f55c1b17366d9cfd2d5ebb75df0227184b104d622ddbc65`.


## POC-03-F / 03-B — attribution and no-effect contract, 8 November 2026

**Status: proposed scenario specification retained at Fabrice's request, no product
implementation, execution or acceptance.** Continuation of the existing plan and
archive review. Codex only, spec-builder/replan. No development, test, provider
call, payment, build or service. Other scenarios A/C/D/E/G/H unchanged. Sources
read at HEAD `aaf1a7b8c99fa5354a191525417fc37552ad3170`; historical results retain their original inputs.
Other checkout work preserved.

### Current boundaries read before specifying

| Bounded current finding | Repository source read | Specification consequence |
| --- | --- | --- |
| Intent reservation is not publication. | `rust/crates/swarm-engine/src/root_store/paea_git_intent.rs:11–38` | Intent presence proves neither effect nor authority. |
| Negative Reconcile means missing journal row. | Same crate `src/root_store/paea_git_promote_call.rs:140–171,1073–1136` | Preserve native outcome with journal scope, not no-ref-effect or Commit authority. |
| Git receipt contains ref/old/new, no dedicated actor/attempt. | Same file `166–171,1059–1070` | Stronger causal binding is a proposed requirement, not existing DTO capability. |
| already callback follows prepare. | Same file `967–1028`; `src/git_reference_transaction.rs:252–302,415–433` | Distinguish update request, prepare ACK, reached callback, COMMIT and ref observation. No assumed recovery from callback source alone. |
| PA-09 recomputes existing effect rows. | `src/root_store/paea_effect_reconcile.rs:1–16,420–473`; `docs/architecture/PAEA-GIT-TRANSACTION.md` PA-09 | verified=true establishes neither authenticity nor out-of-journal completeness. |
| Git intent key includes expected OLD. | `rust/crates/swarm-protection-contract/src/canonical.rs:84–116`, `types.rs:85–93` | Changing OLD changes computed intent, not resolution of the original obligation. A key/digest proves no author/emission/authority. |
| Existing controls are narrower. | Engine `tests/paea_git_promote_call.rs:640–676`, previously read 85/86 capsules | Replay with a receipt and pre-commit negative read do not cover published ref without receipt or identical third-party effect. |

Abbreviated src paths are under `rust/crates/swarm-engine/`. Hashes below identify
this read. Everything below specifies **proposed requirements**, not a shipped
JSON shape, endpoint, signature or authentication capability.

### Common contract

Logical I is the original objective with retained lineage, A1/A2 its attempts,
B another actor, D the physical repository instance, R a direct ref, O old OID,
N candidate, **O different from N** in primary cases. Preserve exact action/context.
Changes to D/R/O/N/context require explicit lineage and applicable admission/
decision, not an equivalent retry. I is not a new engine field or just the
computed intent key.

Declare both fixture goals before their controls: **G-state**, R=N; **G-author**,
prove A performed the authorized O→N transition. B may satisfy G-state but not
G-author. Recommend checking both interpretations; missing actor evidence never
implicitly authorizes another mutation to manufacture attribution. In primary
cases B has its own valid authority, separate from A. If B authority is unknown
or inapplicable, present state does not prove compliance of B effect; G-state
alone cannot justify closure as an authorized effect.

| Requirement | Trigger and permitted conclusion | Required proof / forbidden inference |
| --- | --- | --- |
| BF-C01 current state | On a bound observation of D/R, report R=N at that observation. | Actually opened instance, direct ref/OID, observer provenance and observation barrier. Equality proves no author/history/continued state. |
| BF-C02 attributed historical effect | Attribute an actual transition to A1 only when occurrence and causal binding are established. | Producer/sink observation binds instance, target, applied O→N, exact request, attempt and observed identity; provenance/integrity under explicit threat model. ACK, clock, hash, key name, PID or declared author alone insufficient. Otherwise attribution unknown. |
| BF-C03 missing receipt | Readable journal lacks expected receipt: report absence in that journal at that read only. | Preserve native result/scope. Missing/corrupt/inaccessible/refused journal is not empty. Empty journal plus R=O does not prove no historical publication. |
| BF-C04 no A1 effect | To resume on “A1 published nothing”, require complete observation from before its first emission opportunity until effective termination/fencing. | Cover all inventoried authorized A1 paths, zero A1 publication events, final barrier and proof A1/children cannot later publish. Separate B effects. Gaps/untrusted provenance/uncovered path/residual capability leave absence unestablished and mutation suspended. Process exit/revocation alone does not finish an already-engaged emission. |
| BF-C05 bounded global absence | Claim no publication by anyone only with complete coverage of all fixture writers/paths and declared interval. | Distinguish no A1 publication, none observed, and none in a completely observed scope. No extension outside scope or into the future. O→N→O is historical publication despite final O. |
| BF-C06 conflicting/late proof | Retain originals and scopes when incompatible, late, truncated or unattributable evidence arrives. | Bind event to attempt; prove reception by decider separately. Unknown/conflict remains visible, known effect preserved, A1 receipt not applied to A2. Clocks alone do not order distributed events. |
| BF-C07 read versus decision | Receipt/Reconcile reception leads to explicit read/wait/accept-state/propose-emission decision. | Read cannot publish or invent an author when backfilling history. Decision attributed through chosen authorized path, bound to originals/revisions; reception/application/effect are separate proofs. No new signature technique selected. |
| BF-C08 new effect | Lost response, elapsed delay or new key never grants a mutation. | A1 ended/fenced, historical effect/objective clarified, state freshly read, OLD/rights/approval/policy rechecked at application, explicit decision. Changed preconditions/insufficient proof refuse or suspend without publication; no silent OLD adjustment. |

Historical effect may coexist with current R≠N. Proven absence of A1 publication
may coexist with R=N caused by B. State satisfaction and author criterion remain
separate; do not erase consumption, attempts or history.

### Expected decision table — target, not existing engine states

| Available observations | Report | Permitted action in these scenarios |
| --- | --- | --- |
| R=N, attributed A1 effect, receipt possibly lost | Present state, established A1 effect, missing receipt if observed. | Authorized read/collection and explicit criteria-based closure; zero new publication. |
| R=N, attributed B effect, proven A1 absence | State achieved by B, no A1 publication in covered scope. | G-state may close by competent decision; G-author unsatisfied, arbitrate without publishing to appropriate B's effect. |
| R=N, unknown author/conflict | State present, cause unknown/conflict, journal absence inconclusive. | Observe/arbitrate, zero publication; G-author stays open. |
| R=O, proven A1 absence, A1 ended/fenced, no incompatible effect | State absent at decision and bounded no-A1-effect established. | A2 may be proposed under BF-C08; one authorized O→N publication, never automatic launch. |
| R=O, incomplete capture or live A1 capability | Historical absence and retry safety unestablished. | Establish observation/fencing; zero A2 publication while risk remains. |
| R=O after O→N→O, or unexpected X | Historical effect retained, changed objective/preconditions. | Replan/arbitrate, no “A1 did nothing” or mutation under obsolete approval. |
| Ref/journal read fails or bound instance replaced | Unavailable observation/stale binding, not absence. | Safe diagnosis, suspend mutation; correct precondition before resuming. |

```mermaid
flowchart TD
  P[Original proofs received and instance bound] --> S[Separate state, historical effect and attribution]
  S --> C{Sufficient compatible proof within scope?}
  C -->|No| U[Preserve unknown or conflict and suspend emission]
  C -->|Yes| G[Evaluate declared G-state or G-author]
  G --> D[Authorized decision bound to originals]
  D --> R[Read without publication or close against criteria]
  D --> V{New effect explicitly authorized and BF-C08 satisfied?}
  V -->|No| U
  V -->|Yes| A[One controlled emission then independent observation]
```

### Planned isolated setup and oracle

Fresh owned Unix fixture/project/service/repository, fictitious data, no network/
provider required. Reuse public PAEA library paths where supported; exact calls,
retained Git source fixture and injection facilities are prerequisites to qualify,
not presumed helpers. No active repository/live mission. Valid distinct O/N,
same policy/approval/roles as controls, no policy lowering to reach a fault.

Before each variant, a valid control in a separate equivalent instance must show
that authorized action and observation work. Proposed barriers separate durable
intent, Git preparation, requested COMMIT, actual publication, ACK/response,
durable receipt, proof reception and decision. Actual order/injection availability
must be established on the chosen path. No invented current binary barrier,
arbitrary sleep/global watchdog or retry for unexplained non-progression.

Fixture oracle independent of the agent covers A/B and inventoried paths with
established writer provenance in that fixture. Separate oracle truth from what
reaches the decider; a withheld receipt must not leak through another projection.
Worker-written logs do not prove sink state. Missing reliable observer/coverage
means **NOT TESTED / missing prerequisite**, never PASS through common rejection.

Counters: mutating requests, update-ref/prepare protocols, COMMIT bytes sent,
actual A/B publication transitions, effect-journal rows, reads. Protocol start or
no-op command is not publication; identical final state may hide multiple events.
During observational recovery, publication delta = 0. Harness evidence writes are
separate from ref mutations and must not erase/falsify effect journal history.

### 03-F ref/journal gap — planned variants and acceptance

Every row inherits BF-C01–08 and its preceding valid control. All are
**PLANNED / NOT TESTED**; no new product integration presumed.

| ID | Injection and discriminating evidence after valid control | Future acceptance |
| --- | --- | --- |
| 03-F-T | Authorized A1 O→N, response+receipt present, then public read/result replay. | One A transition, zero B; bound receipt/intent/action/instance. Read/replay adds zero publications. Attribution relies on oracle scope, not receipt shape alone. |
| 03-F-1 | Break after intent before first emission; A1 ended/fenced, complete oracle sees no publication, R=O, receipt absent. | Distinguish intent from proven absence. Reconcile publishes nothing. Explicit fresh BF-C08 decision may propose A2 then exactly one cumulative O→N effect. Unsupported public recovery remains prerequisite, no reservation bypass. |
| 03-F-2 | Break after actual O→N before ACK/response and receipt; variant response delivered then break before receipt. | Oracle sees one A1 publication. Native Reconcile negative stays journal-only, no no-effect inference or Commit authority. Authorized re-observation keeps R=N, attribution only with sufficient proof, zero publication delta. Broadened negation at consumer fails criterion. |
| 03-F-3 | Durable receipt then lost response; reopen fixture stores, reread. | Same-instance historical receipt, no new publication even with new transport key. Response loss differs from receipt loss; read refusal is not no commitment. |
| 03-F-4 | F-2 state with R=N/no receipt; separately diagnose recovery Commit with OLD=O≠N only in later explicitly authorized fixture. | Record prepare ACK/refusal, callback reached or not, COMMIT sent or not, ref/journal before/after. Refusal before callback documents missing reachability, not successful recovery. No weakening OLD or removing locks; second publication fails invariant. G-state closure can remain a decision without re-emission. |
| 03-F-5 | Publish O→N then authorized fixture actor restores O before final read; variant R=X. | Preserve both transitions/authors. Final O never means historical absence. No automatic retry; reconsider objective/approval. Injection B mutations separate from product recovery delta. |
| 03-F-6 | Unreadable/corrupt observation/journal or physical instance replaced after preparation; variant A1 still has in-flight emission. | Safe error/conflict/unknown, no empty success/imagined absence, zero A2 publication. Recover observation only after restored/fresh binding or demonstrated fencing, then fresh decision. Never delete Git lock for progress. |

### 03-B identical third-party change — planned variants and acceptance

| ID | Injection and discriminating evidence after valid control | Future acceptance |
| --- | --- | --- |
| 03-B-T | Attributed A1 O→N control, distinct observed B control in separate equivalent instance. | Oracle actually distinguishes A/B requests/transitions; a common outage cannot explain A refusal. |
| 03-B-1 | Prevent A1 emission, prove end/fencing, B publishes identical N, deliver B evidence. | G-state satisfied at observation; G-author unsatisfied; proven scoped A1 absence. No invented A receipt or publication to claim B's effect. Explicit criteria-based closure/arbitration. |
| 03-B-2 | Same state but B author proof withheld from decider; variant incomplete A1 observation. | R=N observed, author unknown to decider, no default attribution to A or B. Unfenced A1 residual risk explicit. Zero recovery emission, author criterion open. |
| 03-B-3 | A/B each publish towards N with controlled authorized intermediate transition, or an explicit fixture effect counter revealing both effects; do not assume identical update-ref calls cause two Git transitions. | Distinguish requests from real publications. Two historical effects remain two despite final N; recovery adds none and does not claim prevention of injected duplication. Final-state-only oracle is nondiscriminating. |
| 03-B-4 | Same key carries other attempt/identity/instance/target/action receipt; coherent receipt with unestablished provenance; then restore correct proof. | No attribution/authority from key/hash alone. Binding refusal/conflict/unknown visible, rejected original retained. Correct proof permits fresh read decision, no extra publication or rights transfer. |
| 03-B-5 | B publishes then restores O (ABA); variant B changes instance/ref/payload during decision. | Final O is not global absence, changed target not equivalent retry. Incomplete proof stays unknown; complete oracle attributes B and changed preconditions. No mutation under obsolete approval. |

### Acceptance and dependencies for later realization

These are future mandatory criteria, **not satisfied by writing this section**:

- Each variant has a successful same-precondition control and reaches intended
  fault boundary. Zero tests, common prerequisite rejection or exit 0 is not PASS.
- Compare decider's received conclusion with oracle: state, attribution, bounded
  absence, residual capability separate. Native outcome/scope survive through the
  selected consumer; an external report wording change cannot hide wrong product decision.
- Trace originals received → authorized decision → application → observed effect.
  Omniscient fixture or mechanism name cannot replace this chain.
- Check before/after counters for each recovery: zero publications on reads,
  satisfied/unknown/conflicting states. Only conditional F-1 targets one new
  publication. B's injected mutations remain visible.
- Verify refusal without mutation, authorized observation after corrected
  preconditions and reopen recovery. Old proof remains historical; application
  decision/authority current. Later revocation does not erase an authorized
  historical effect; its observed success grants no authority for a new action.
- Missing proof stays NOT TESTED/prerequisite; an observable contradiction is FAIL.
  Earlier historical PASS reused only in scope, never as F/B execution.

| Proposed step | Owner / dependency | Deliverable and gate |
| --- | --- | --- |
| BF-P1 documentary, current request | Codex; current sources and previously read archives | FR/EN contract, explicit G-state/G-author variants, matrices/limits. Documentary complete, no engine validation. |
| BF-P2 prerequisite qualification | Codex after future scope authorization | Exact public path, binary/profile/features, instances/Git fixtures, actual observer/barriers, inventoried A/B paths/fencing. Missing instrumentation needs separate bounded proposal, no silent implementation. |
| BF-P3 targeted reproduction | Authorized executor; P2 and F-T/B-T controls | First F-2/F-4 and B-1/B-2, then F-1, ABA F-5/B-5 and other variants; source/argv/raw output/hashes, oracle, received proof and per-criterion verdicts. Not launched by this plan. |
| BF-P4 possible correction decision | Fabrice for realization; Codex proposal using P3 | Compare explicit observations+separate decision against stronger durable producer/attempt receipt binding. No premature new signature/key/generic retry. Confirmed defect would require general correction/permanent recipe under appropriate authority. |

Refined discovery: **ABA and observation interval completeness** are required for
no-effect controls to discriminate cases. Expected benefit: avoid false negative
despite final O. Codex owns BF-C04/05 and F-5/B-5, specified not executed. Oracle
scope/fencing remain unestablished prerequisites, not guarantees created by prose.

Documentation only: no product function/docstring, policy/default/limit/error
handling changed; no new schema/API or delivered version_history entry. Persistent
FR/EN plan and Git-ignored local X index carry this proposal; no embedded context,
process or service reloaded. Development/tests explicitly excluded by this request.

Fingerprints of reread sources, no new execution evidence:

| Source | SHA-256 |
| --- | --- |
| `rust/crates/swarm-engine/src/root_store/paea_git_intent.rs` | `2aef44210363e5aec5b96d88e1990828744dc6b84828c0e0dc15b0ddd2c82652` |
| `rust/crates/swarm-engine/src/root_store/paea_git_promote_call.rs` | `1255d665e4ef6b7183253bbca39193904519466850e75d66c501593ccb9fd6e2` |
| `rust/crates/swarm-engine/src/git_reference_transaction.rs` | `180adba837db4fe54c61a38a7b3e44443bed9b8bfbeef33cedf323cc62bf8324` |
| `rust/crates/swarm-engine/src/root_store/paea_effect_reconcile.rs` | `1a62d87a9ad1fc34f376138dcb2208ed6d8bc9e19e930b853e5d1a51d32b36e5` |
| `rust/crates/swarm-protection-contract/src/canonical.rs` | `5e9bad76b65a7fdbbd2a3966fdebbba59480d80c2e2337498ac5d0bd2a9092f0` |
| `rust/crates/swarm-protection-contract/src/types.rs` | `4cf2a9869422eff1d9aaf8bc9d019600b8ef179ac9132e6fbbba3101cbf76c42` |
| `rust/crates/swarm-engine/tests/paea_git_promote_call.rs` | `ef4e847c8edc68a6ac1793959e906177d8ac1fceef2859fc40c74c4d226424a5` |
| `docs/architecture/PAEA-GIT-TRANSACTION.md` | `46aceee29e685100866171a42e3fb42e8ff0fda400eea0d2f236b8f198b38e4c` |


## BF-P2 — documentary prerequisite qualification for 03-F / 03-B, 8 November 2026

**Read and plan update only.** No code, test, build, provider call, payment or
service started; no fixture created, opened via RootStore, modified or reused.
Codex only, replan/spec-builder, HEAD `aaf1a7b8c99fa5354a191525417fc37552ad3170`. Prior additions/other work
preserved. BF-P2 documentary qualification complete; **BF-P3 executable readiness
remains PARTIAL**, no runtime PASS or new authority.

### BF-P2.1 selected public path

**Normal public Linux swarm-engine library**, no server. Existing source chain:

1. Distinct physical project/service RootStores, writable; explicit intent/promotion
   journal initialization and `paea_provision_git_repository`. Provisioning grants
   no publication authority; none invoked here.
2. Actual qualified native candidate in project; objects imported first through
   `paea_git_transfer_call` / `call_paea_git_transfer`. Merely naming N is no control.
3. `service.paea_git_promote_call(project, proposal, context, operation, work, child, pin)`
   retains physical witnesses; Commit attributes policy, Reconcile retains exact
   historical context. request() exposes exact permission to provision; preparing
   does not yet reserve the durable intent.
4. Separately provision account/authority, `authenticated_startup::open`, then
   `service.call_paea_git_promote(project, prepared, authority, context, head, password)`.
   Guards enforce witnesses/current plan/effect mapping/policy; Commit rechecks
   exact current agreement. Commit backend requires an observed human principal,
   proving neither actual human presence nor agent A provenance.
5. Reconcile requires **writable RootStores** through the same invocation guard,
   although it publishes no ref and starts no Git. Distinguish storage reservation
   from no ref mutation. Explicitly close prepared calls/authority/Roots and retain errors/effects.

**Established profile restriction:** policy::current_actor in the same module
(648–693) refuses Commit with require_separation=true or enabled nonempty
incompatible-role matrix (paea-git-promote-attributed-admission-required). Existing
harness explicitly sets require_separation=false; not qualification of full
protected profile. Proposed framing uses only that declared reference fixture
profile, still requiring exact agreement; no current installation setting lowered.
If future requirement includes separation, missing admission integration is a
separate blocking prerequisite, never a refused control counted as success.

Read engine `src/root_store/paea_git_promote_call.rs:348–550,648–693,853–911,1073–1136`,
`tests/paea_git_promote_call.rs:319–497,499–579` and authority fixture below.

Existing alternatives, not selected for the first isolated proof: CLI
`paea promote-call OPERATION SERVICE PROJECT WORK CHILD PIN DECLARATION --input REQUESTFILE`
(`swarm-cli/src/paea_promote.rs:28–50,224–254`) uses private two-phase framing and
projects reconciled_not_committed without adding a ref observation; retain journal
scope. Public PA-09 library/CLI reconcile-effects recomputes existing journal rows
with a separate ledger; not a complete historical ref observer. No HTTP/browser
needed for selected BF-P3; surface existence proves no parity.

### BF-P2.2 fixtures and actual availability

| Inspected item | Read-only observation | Qualification / prerequisite |
| --- | --- | --- |
| Governed fixture area | Saved values.runtime_temporary_directory is .swarm-fixtures, directory exists, observed 0700. Engine fixture_area.rs:51–117 validates/resolves setting. | Presence/config identified, helper not invoked. Revalidate full permissions/space/quiescence later; fixture setting distinct from USB evidence storage. |
| Source fixture | SOURCE/WRITER variables both select `docs/plans/actions-confiance/paea/evidence/claude-tmp-fix-e2e-20261031T173021Z/source-fixture-writer78b-regen-20261031b`; result.json exists with PASS/task_accepted=true/paid_provider=false. Root `.swarm-fixtures/swarm-model83-u5rvobb_`, state.db and matching owned-root record exist. | Presence/declared provenance/birth record found, not fresh native proof, matched current inode/hash or established inactivity. Do not open/mutate based on saved PASS. |
| Current-publication fixture | `.../source-fixture-current-pub-regen-20261031/result.json`, same markers; root `.swarm-fixtures/swarm-model83-jx8u1ndc`, state.db and matching birth record exist. | Separate retained root, same limits, no transfer of qualification to first fixture/current profile. |
| PAEA promotion harness | tests/paea_git_promote_call.rs:80–105,319–433 creates fresh service but opens retained project writable, applies Trust, seeds bare OLD and imports candidate. | Composition reference, not wholly fresh independent instances. New qualified native source through supported producer required. Copy/restore cannot substitute physically bound native proof; no generation here. |
| Authority fixture | enrollment-authority tests/common/authenticated_fixture.rs:35–60,96–145 creates private journals under ~/.local/state/swarm-fixtures, fixed principal operator/human/stable-person. | Two constructions are not distinct A/B actors. Distinct observed identities/permissions fixture composition missing, not a claim that product cannot represent them. No real account/key used. |
| Synthetic Git data | tests/git_reference_transaction.rs:30–104 makes bare plus O/N/X without native project/provider. | Source available for isolated protocol/ABA, not full PAEA candidate/approval/authority. No constructor run. |

**Recipe discrepancy BF-P2-R1:** paea-owned-git-promote.json inputs declare
`.../source-fixture-writer78b-regen/result.json` but SOURCE/WRITER environment selects
`.../source-fixture-writer78b-regen-20261031b`, whose result.json project() actually
reads. Actual result is neither exactly declared nor covered by a general docs
pattern in that recipe. Referenced acquisition recipe also sets these variables
without declaring those external files. Declared inventory coverage gap, not
proof of historical alteration/runtime failure. Owner Codex verification; gate:
match effective consumers/inputs/provenance/hashes before a derived run. No
manifest correction here; do not use unchanged recipe to qualify a new fixture.

### BF-P2.3 observer readiness

| Contract need | Current source mechanism | Readiness |
| --- | --- | --- |
| Final direct-ref state | Protocol Fixture::value uses rev-parse; direct checks type/OLD. Promotion reads ref before/after protocol (967–1068). | Source available with bound instance/tool provenance; PaeaGitRepository::inspect checks custody/candidate, not target ref (218–238). Point reads do not cover ABA. |
| Intent/receipt read | paea_read_git_intent, Reconcile, PA-09. | Available, journal-only, no complete effect/actor proof. |
| Actual Git commit point | git_reference_commit_without_ack_retains_uncertainty_and_actual_ref, test file 360–398, committed fixture hook writes marker then kills Git before ACK; reads new ref/retains CommitRequested. | Partial protocol injection/oracle found in source, not executed here. Controlled hook proves neither hostile-writer-independent observer nor full PAEA path. |
| Durable intent after lost ACK | git_reference_durable_intent_survives_missing_ack:535–626 reserves intent, runs protocol, reopens and sees Existing without new publication. | More precise source coverage than simulator alone; future recipe paea-native-git-intent scope only, no new PASS/integration into effect::commit. |
| Complete A1/B publication history, reception and hidden-proof separation | No complete matching observer found in examined producers/harness/recipes; receipt only ref/old/new and fixed authority principal. | Missing prerequisite in inspected path: independent oracle with attempt/writer origin, coverage/barriers and separate decider projection. PID, Git commit author, marker, reflog or hash alone insufficient. No global-absence claim. |

Native git-reference/intention permanent recipes name these tests; none executed
or reverified now. Previous historical capsules are not extended by similar names.

### BF-P2.4 actual injection points

| Intended boundary | Accessible mechanism | Scenario scope/limit |
| --- | --- | --- |
| Before public invocation | Caller controls order; promotion revocation test 689–783 finishes revocation before consumption. | Changed preconditions F-6 / B stimulus before A, not an in-flight cut. |
| Intent before Git | Public paea_reserve_git_intent can create isolated intent without Git. Internal effect::commit reservation private, no exposed callback there. | Synthetic F-1 preparation possible with explicit scope; not actual integrated interruption. |
| Prepared before COMMIT | Public run_git_reference_transaction(Command, proposal, deadline, cancelFD, decide), callback after prepare ACK. Tests check lock/contention/cancellation/deadline. | Protocol only. Promotion supplies internal closure and cancel=None (963–1009); API does not expose either. |
| Published before ACK | Protocol committed fixture hook supplies source reproducer with actual Git. | PAEA managed_repository::git_command:63–106 forces core.hooksPath=/dev/null and strips GIT_*. Do not weaken protection or replace Git wrapper while claiming same profile. PAEA F-2 not ready. |
| Ref publication/read before receipt | Private effect::commit then private record, no public barrier found in read module. | Missing integrated F-2/F-4 point. Timed kill/log polling is not qualified causal injection; hand-edited journal cannot prove interruption. |
| Library return/durable receipt before requester receives | Harness may withhold returned result; CLI emits final outcome only after invocation/closes. | F-3/B-2 composition possible, no ready harness. Distinguish withheld harness output, stdout error and actual transport loss; no network needed. |

Protocol prepared-cancellation test (406–455) deletes its fixture lock only after
inode/reap checks. **That cleanup is not adopted for F/B:** plan forbids lock removal
to force recovery. Diagnose/observe and use a new owned instance if no compatible
recovery exists; never describe this test cleanup as PAEA capability.

### BF-P2.5 fencing scope

| Mechanism | Source-established control | Limit / remaining prerequisite |
| --- | --- | --- |
| OLD/prepared Git lock | update NEW OLD, callback under Git lock, competing-writer test 248–275. | Serializes this ref while held, not every A1 action/future writer. Restored O does not protect against ABA. |
| Project SQL writer / Trust reservation | call_paea_git_promote:491–528 holds plan/policy/agreement across application; public revocation needs competing lock. | Do not plan immediate in-section revocation as barrier. Completed pre-invocation revocation blocks stale agreement, not engaged effect. |
| Context::cancel / cancelFD | authenticated_control.rs:11–30 cancels consumers; protocol checks before COMMIT (181–211,284–302). | Account Context is not a Git cancelFD in promotion (None). Cancellation/expiry does not prove child termination or no effect. |
| Owned process | validation_run.rs:389–425,556–590 uses group/pidfd/wait and retains Child/errors, bounded reaped/ownership getters. | Signal delivered is not all descendants stopped; leader status excludes escaped descendants/other same-UID writers. PID is no kill capability; no signals sent here. |
| Retained-owner recovery | validation_run_recovery.rs:31–54 polls retained Child, no launch/signal/retry, crate-private function. | Not a general public A1 fencing API. BF-C04 complete residual-emission proof remains unestablished. |

Before mutating recovery: applicable owned completion, closure of all harness A1
emission paths and final barrier observation. Missing Git lock, canceled flag or
missing receipt cannot substitute. If B-2 oracle lacks this coverage, expected
outcome is unknown and zero publication, not proven absence.

### BF-P2.6 profile, inventories and passage decision

swarm-engine/Cargo.toml exposes separate test-fixtures feature. Read normal recipes
do not enable it; integration targets paea_git_promote_call/git_reference_transaction,
first uses ignored tests requiring explicit native fixture. .cargo/config.toml
sets dev/test debug=0/incremental=false, compiler wrapper and SQLite FILESTAT.
Configured inputs only, not unified features or exercised ELF. No compilation,
Cargo metadata, Git --version or active-binary inspection performed. Effective
profile/tool Git/future binary identity remain to retain during authorized execution.

| Scenario family | Current documentary status | Concrete gate |
| --- | --- | --- |
| F-T/B-T controls | PAEA composition identified; no qualified fresh-project/distinct-B/oracle harness. | Supported fresh native source, distinct A/B rights, provenance/counters and observed control. |
| F-1/F-3/F-6 | Public preparation/reservation and before/after-call boundaries composable; integrated cut/ownership/projection unestablished. | Select explicitly bounded synthetic state versus actual cut; qualify bindings/errors without mixing scopes. |
| F-2/F-4 | Protocol lost-ACK source available, PAEA ref→receipt barrier and already reachability unqualified. | Deterministic harness preserving normal path, separate reproduction, no global passage. |
| B-1/B-2/B-4 | State/withheld-output stimuli composable, second actor/provenance/full reception missing. | Observed actual writer identity, separate decider knowledge, proved distinct rights. |
| F-5/B-3/B-5 | Synthetic O/N/X available at protocol layer, complete historical PAEA oracle absent in inspected harness. | Capture intermediate transitions/authors, not just endpoint comparison. |

**Decision: BF-P2 documentary work complete, BF-P3 passage not established.**
Two feasible future routes retained without launch: reuse protocol harness first
for prepare/OLD/lost ACK with narrower explicit scope, preserving final F/B criteria;
or prepare bounded full-path F/B harness with new source, observed A/B identities,
oracle, controlled reception and ref→receipt barrier. Decide exact test
instrumentation/adapter/product-change need before editing; no production hook or
generic retry presumed.

Codex owns proposals/future realization and R1; Fabrice future development/test
authority and arbitration. Recommended order: correct effective recipe inputs
under appropriate authority, qualify fresh fixtures/identities, then oracle/
barriers/fencing, before F-2/F-4 and B-1/B-2. Only contracts/gaps retained here.

No product policy/limit/function/docstring changed; no recipe edited or active
fixture/historical evidence modified. No build/delivery/version_history update.
Persistent FR/EN plan and Git-ignored local X index only; no embedded context,
process or service reloaded.

Static qualification hashes, not compiler inventory or execution proof:

| Source / fichier de preuve lu | SHA-256 |
| --- | --- |
| `rust/crates/swarm-engine/src/root_store/paea_git_promote_call.rs` | `1255d665e4ef6b7183253bbca39193904519466850e75d66c501593ccb9fd6e2` |
| `rust/crates/swarm-engine/tests/paea_git_promote_call.rs` | `ef4e847c8edc68a6ac1793959e906177d8ac1fceef2859fc40c74c4d226424a5` |
| `rust/crates/swarm-engine/src/root_store/paea_git_intent.rs` | `2aef44210363e5aec5b96d88e1990828744dc6b84828c0e0dc15b0ddd2c82652` |
| `rust/crates/swarm-engine/src/root_store/paea_git_repository.rs` | `0b59230a65dd318da0f9ca387145ebe49c1800f1c70ad196a2fa3aefb9cbbe18` |
| `rust/crates/swarm-engine/src/git_reference_transaction.rs` | `180adba837db4fe54c61a38a7b3e44443bed9b8bfbeef33cedf323cc62bf8324` |
| `rust/crates/swarm-engine/tests/git_reference_transaction.rs` | `fa3ed15210465b128ca2ea0be6aed984034b26367c5932a413f9fe71b211ed3e` |
| `rust/crates/swarm-engine/src/managed_repository.rs` | `6e6a87c8eb6fa6cf207f581a8bc85784e3598a875fb5427ce696c55af396bae3` |
| `rust/crates/swarm-engine/src/validation_run.rs` | `1c843fdfcafea7962d079198eb74fe163b81d2d3c10f9e54dea8e10b7f5399c2` |
| `rust/crates/swarm-engine/src/validation_run_recovery.rs` | `548966ca99e07b62c6d672f70cf2cab9e73f5af18537f54cf67447f637cbd554` |
| `rust/crates/swarm-engine/src/fixture_area.rs` | `af557230575868c1e70c83825136cd9d9d26673681301f318e54621cb9f22708` |
| `rust/crates/swarm-enrollment-authority/src/authenticated_control.rs` | `cf502d4b735137d4c2a830ddfadaa3afefb5630241bb1fab40a3c7894eb0256a` |
| `rust/crates/swarm-enrollment-authority/tests/common/authenticated_fixture.rs` | `0c0f3635a8c9cfa2b4afa3a88c38baf71e8b0a8f48f89a2992bd9724586df6d2` |
| `rust/crates/swarm-cli/src/paea_promote.rs` | `1547acd73582dafbd7b6acde6f6616f2db624995411a5b3cf699f2ec339fd6a5` |
| `rust/crates/swarm-cli/src/paea.rs` | `8c39c1758e3e08a886f15fc78056e83071e5ecd6d16810c4d63189cf68f21c56` |
| `rust/crates/swarm-engine/Cargo.toml` | `513a0d0b17756cd31e08e9f6d2fb1872df41dcb3bf8cc1b11d4830aa4a692f89` |
| `.cargo/config.toml` | `31bee734fefe516086f75b642c1ca537e54997d684e4cc5c983951e01157ca72` |
| `tools/verification/recipes/paea-owned-git-promote.json` | `7ee5855ce0ef3b1c74836a0968e2da3031db882d8954e7dfe2e1b79a3db3e656` |
| `tools/verification/recipes/paea-owned-source-acquisition.json` | `2eb35de6b2851f8ed957701e3d21ce311477eb97eb0c8c5df5e34cff97517866` |
| `tools/verification/recipes/paea-native-git-reference-transaction.json` | `09a4a07914cc9c97eb77c766823c09778a4628a9b7d9e096d58512ddb1bbe43c` |
| `tools/verification/recipes/paea-native-git-intent.json` | `e2d478da120bce223bead78109f427794ef957a05a58cb046c3bb30092e30fdf` |
| `docs/plans/actions-confiance/paea/evidence/claude-tmp-fix-e2e-20261031T173021Z/source-fixture-writer78b-regen-20261031b/result.json` | `bc58ae6b3492da38697f45d438f116f79c545ad8f0236ad4c5de9ddbea779218` |
| `docs/plans/actions-confiance/paea/evidence/claude-tmp-fix-e2e-20261031T173021Z/source-fixture-current-pub-regen-20261031/result.json` | `f79644bc4a6fcb3f0d1b508d8e9a9dd293a12f6ecaafa879a46b5144b4bec020` |


## BF-P2 — bounded blocker-removal plan, 8 November 2026

**Current request: planning only.** No development, test, compilation, provider
call, payment, fixture creation or service started. Codex sole owner, no delegation.
BF-L0–L5 below are **PROPOSED / NOT IMPLEMENTED**, not execution authority or an
engine-adopted/validated plan. Documentary BF-P2 remains complete; BF-P3 requires
future gates and authorization.

Read HEAD `9d64cb33d09a0285abae8975aa6fcdd2268e8e4d`, with concurrent native-review work present. All 22 targeted
BF-P2 source/evidence hashes still match. Reuses their bounded static observations,
not qualification of modified shared modules, compiler graph or current binary.
Other work preserved; replan/spec-builder and verified-claims rule applied.

### Bounded objective and profile

Future deliverable: isolated attribution/recovery harness reusing public PAEA
operations, fresh controls, A/B observations, causal ref→receipt interruption,
closed A1 emission paths and faithful evidence inventory. Harness readiness is
not success of 03-F/B; distinguish harness defects from product defects.

Explicit reference fixture profile remains require_separation=false, exact
agreement/current controls preserved. Full separation profile still refused by
current consumer and outside this lot; separately address admission if required.
No live policy lowering, new business endpoint, receipt signature, generic retry,
compensation/finance integration or general OS confinement. Declare harness threat
model: supervised A/B writers, identified native tools and all inventoried fixture
mutation paths. Malicious outside writer remains a limitation, not covered actor.

### Proposed lots and observable future gates

All checks mentioned require **later execution authority**. File areas are proposed
ownership boundaries; new helpers/manifests remain to name, none created here.

| Lot | Dependency / proposed scope | Deliverable and future gate | Failure and resumption |
| --- | --- | --- | --- |
| BF-L0 inputs/profile | Future authority; current consumers/recipes, BF-C01–08 and BF-P2-R1. Inspect effective external paths/validation before change. | Parameter/env → actual consumed file → validation → consumer map. Promotion/dependent recipes declare the actual result.json. Pin required sources/helpers/declarations/tools; no run before R1 resolved. | Missing/ambiguous input stops before run, no historical-fixture fallback. Codex verification; corrected attributed inputs required. |
| BF-L1 new birth/distinct identities | L0; promotion harness, native first-publication producers, authority fixture. | Fresh native project/service/repository/journals/accounts per control/variant, birth/current physical binding, O≠N and supported candidate import. Distinct observed A/B identities/exact separate decisions. Both controls succeed without writable reuse of old root. | If native birth/qualification cannot work without provider or original modification, isolate prerequisite. No copied inode-bound root, fabricated PASS or positive SQL acceptance. Codex fixtures; resume only with genuine birth/qualified fixture. |
| BF-L2 oracle/reception | L1; harness outside agent and existing native observations. | Append-only harness history binds instance/action/attempt/observed identity/emission/transition/receipt/error/completion. Causal local barriers, separate request/COMMIT/publication/journal/read counters. Oracle truth and originals actually delivered to decider are separate projections with distinct receipt. | Mixed identities, capture gaps, hidden-proof leak or agent-writable oracle refuse coverage; unknown attribution/absence and zero mutating recovery. Codex observation; discriminating corrected control required. |
| BF-L3 ref→receipt cut | L2 and profile design settled before edits; actual private commit/record path and Git primitive. | Causal point after observed ref publication before receipt write. No-fault control then cut leaves N, durable intent and no fabricated success receipt; exercise same reconciliation consumer. Retain prepare ACK/refusal and whether already callback reached with OLD=O≠N. | Unreached barrier or harness-forged effect state is not PASS. No expansion into signature/retry. Codex engine/harness; correct barrier and retain exact profile first. |
| BF-L4 scoped fencing | L2; L3 phases for in-flight emission, real Child ownership/errors. | Harness supervisor closes new A1 dispatch, tracks delivered operations, retains owners to resolution, final observation barrier; late A1 send refused without effect. Unresolved Child/lost ownership forbids declaring A1 finished. | Cancellation/signal/timeout/PID/absent lock insufficient; gate stays closed and outcome unknown while emission possible. No lock removal. Codex owned execution; actual resolution proof, not new timeout. |
| BF-L5 final recipe/harness qualification | L1–L4 then refresh L0 after changes; permanent recipes/collectors and FR/EN docs. | Final consumed-byte inventory, normal/instrumented artifacts/profiles separate, named controls/refusals/recovery, original FAIL retained. Qualify harness against BF-Q gates; only ready cases eligible for BF-P3. | Changed input/missing assertion/zero executed cases/stale proof blocks passage. Codex verification; return to responsible lot and requalify affected dependents only. |

Read tests/rust_native_first_publication.py:14–40,132–163 shows existing native
birth/publication composition and no-reuse branch, not proof of successful current
regeneration. It also contains negative SQL faults/review dependencies: **do not
run entire recipe for convenience**. L1 must qualify necessary birth subpath,
provider-free behavior and prerequisites without unrelated faults or positive
SQL acceptance seeding.

### Design choices to bound before L2/L3

**B actor:** do not assume second PAEA call can publish A's already-reserved intent;
uniqueness/conflict may prevent it. Plan B as an **explicitly authorized external
fixture writer**, distinct local identity verified by harness controller, then an
owned native Git operation on target. B authority is a harness fact, not a PAEA
permission enforced by product or separate OS right if same UID. If criteria
require two PAEA producers on same target, stop for scoped qualification rather
than relabel A. B positive control must use its actual injection path.

**Oracle:** transition events must bind actual owned native execution and sink
observation while controlling every permitted writer in interval. Dispatch-A log
then R=N is insufficient if B could write between. Retain that negative control;
if causality relies on exclusivity, demonstrate it within closed harness, never
all processes owned by the user.

**Injection alternatives before implementation:** existing committed-hook protocol
harness characterizes lost ACK but cannot lift PAEA ref→receipt blocker. Recommended
full-harness route is fixture-only instrumentation on actual commit→record path,
without a second publication implementation. Harness owns barrier/observation/fault;
no failure option through normal Admin/API/CLI, no substituted authority/checks/journal.

Settle precise test cfg/fixture feature/internal adapter at L3 with compiler graph.
**Integration tests normally compile library without cfg(test)**; do not assume
that gate exposes the barrier. If test-fixtures is needed, fault ELF differs from
normal: retain separate graphs/hashes, normal control and effective business-source
comparison. Instrumented harness evidence is **not fault injection proven in
normal shipped ELF**. If that stronger level is required, qualify compatible
additional method before closure, never weaken hardening.

Existing tests/paea_source_acquisition_inventory.py:20–42 rejects test-fixtures/
test-util features. Preserve that refusal; use separate retention for instrumented
artifacts and distinct normal qualification. No Git wrapper/production hook/timed
kill/live DB modification/manual receipt deletion substitutes for L3.

### Dependencies and scoped fencing

```mermaid
flowchart LR
  L0[BF-L0 Inputs and profile] --> L1[BF-L1 Fresh fixtures and identities]
  L1 --> L2[BF-L2 Oracle and reception]
  L2 --> L3[BF-L3 Ref to receipt cut]
  L2 --> L4[BF-L4 Harness fencing]
  L3 --> L4
  L3 --> L5[BF-L5 Final inventory and qualification]
  L4 --> L5
  L5 --> P3[BF-P3 Only after gates and authority]
```

Proposed single-owner order L0→L1→L2→L3→L4→L5, no delegation. Reversible lots;
changed shared contracts require affected dependent-proof reexamination.

L4 separates admission closure, owned-emission completion and observed effect.
Harness gate is not new product policy: refuses closed attempt, grants nothing,
no ceiling or automatic timed release. Preserve in-flight operations and known/
unknown results. Channel closure cannot recall a sent command. Escaped children/
uninventoried writers exceed proved scope: absence unestablished, no re-emission.
Existing contractual deadline remains, expiry proves no fencing. After unknown
COMMIT, close gate then observe; finished leader does not establish no effect.

### Future harness qualification / BF-P3 gates

| Gate | Requirements/scenarios | Evidence after authorization |
| --- | --- | --- |
| BF-Q1 independent fixtures | L1, F-T/B-T | Distinct births/physical instances, no old root mutated, actual candidate transfer and authorized positive controls; proof-copy or wrong-attempt root reuse visible/refused. |
| BF-Q2 discriminating attribution | C01/02/06, B-1/2/4 | Authorized A/B through declared paths, distinct publications despite N equality; wrong actor/attempt/target proof refused/unknown. Withheld B truth stays in oracle, no invented decider attribution. |
| BF-Q3 actual ref→receipt interval | C03/07, F-2/4 | Control then reached barrier, one counted publication, receipt absent due to cut, intent retained; journal-negative never broader no-effect. Second recovery publication FAIL; unreached already callback explicit. |
| BF-Q4 bounded absence/fencing | C04/05/08, F-1/6 | Pre/post barrier distinguished, late A1 send refused without mutation, in-flight result retained through resolution; unresolved/lost ownership forbids new emission. Only conditional F-1 permits one fresh authorized A2 publication. |
| BF-Q5 history beyond endpoints | F-5/B-3/5 | O→N→O and two effects behind identical endpoint retained with authors; two no-op requests not counted as two publications, recovery delta zero. |
| BF-Q6 receipt/decision/application | C06/07, F-3/B-2 | Originals, actually delivered/withheld content, decider receipt, authorized decision and application linked. Disconnect/read error is neither empty success nor re-emission authority. |
| BF-Q7 bytes/profiles/recipe | P2-R1, L0/L5 | Actual external inputs declared, source archives/hashes checked, source stable, named cases actually run, separate normal/instrumented builds. Fixture PASS qualifies neither normal nor protected separation profile. |

Future contract only, no recipe/test created now. Valid control before each fault,
original causes and before/after state required. Harness failure diagnosed/fixed
before product inference. Confirmed BF-C violation with qualified harness retains
FAIL and opens BF-P4; **no silent business-product correction inside harness lots**.
Reuse scoped permanent recipes, no global suite/watchdog/convenience quota added.

### Change/evidence bounds and current closure

Candidate areas: promotion/protocol harness, authority fixture helper, only chosen
commit/record instrumentation junction and necessary recipes/collectors for separate
retention. Future new/changed nontrivial functions need current documentation plus
byte-bound policy/error review in same increment; keep relevant FR/EN docs current.
Old archives/missions/retained fixtures intact. No arbitrary tools/network/real
accounts, publication/commit/push authority granted here.

Future evidence: governed Unix staging, Lexar only after actual mount/space check,
verified readback/local hash index. Do not move execution onto FAT32 by changing
fixture setting. Resource failure retains state/resumption prerequisite, no
unrelated cleanup/destruction/unchanged retry.

**Current closure: bounded plan prepared, blockers not removed.** Five requested
areas have Codex owner, dependencies/deliverables/BF-Q gates. Implementation/BF-P3
not authorized by this request. No new schedule/monitor/budget. No version_history
entry or build/install. Persistent FR/EN plan and local X index only; no embedded
context/service reloaded.

Planning fingerprints, no execution/global qualification:

| Source relue / source read | SHA-256 |
| --- | --- |
| `rust/crates/swarm-engine/src/root_store/paea_git_promote_call.rs` | `1255d665e4ef6b7183253bbca39193904519466850e75d66c501593ccb9fd6e2` |
| `rust/crates/swarm-engine/tests/paea_git_promote_call.rs` | `ef4e847c8edc68a6ac1793959e906177d8ac1fceef2859fc40c74c4d226424a5` |
| `rust/crates/swarm-engine/src/root_store/paea_git_intent.rs` | `2aef44210363e5aec5b96d88e1990828744dc6b84828c0e0dc15b0ddd2c82652` |
| `rust/crates/swarm-engine/src/managed_repository.rs` | `6e6a87c8eb6fa6cf207f581a8bc85784e3598a875fb5427ce696c55af396bae3` |
| `rust/crates/swarm-engine/src/validation_run.rs` | `1c843fdfcafea7962d079198eb74fe163b81d2d3c10f9e54dea8e10b7f5399c2` |
| `rust/crates/swarm-engine/src/validation_run_recovery.rs` | `548966ca99e07b62c6d672f70cf2cab9e73f5af18537f54cf67447f637cbd554` |
| `rust/crates/swarm-enrollment-authority/src/authenticated_control.rs` | `cf502d4b735137d4c2a830ddfadaa3afefb5630241bb1fab40a3c7894eb0256a` |
| `rust/crates/swarm-enrollment-authority/tests/common/authenticated_fixture.rs` | `0c0f3635a8c9cfa2b4afa3a88c38baf71e8b0a8f48f89a2992bd9724586df6d2` |
| `rust/crates/swarm-engine/tests/git_reference_transaction.rs` | `fa3ed15210465b128ca2ea0be6aed984034b26367c5932a413f9fe71b211ed3e` |
| `rust/crates/swarm-engine/Cargo.toml` | `513a0d0b17756cd31e08e9f6d2fb1872df41dcb3bf8cc1b11d4830aa4a692f89` |
| `tools/verification/recipes/paea-owned-git-promote.json` | `7ee5855ce0ef3b1c74836a0968e2da3031db882d8954e7dfe2e1b79a3db3e656` |
| `tools/verification/recipe.py` | `348c9154cf1db9c2c2d6e187d5cb90d86b5125c8290159a6453193b037dbf74b` |
| `tests/rust_native_first_publication.py` | `7ec401919b0ab744d0086728a7b102fd5f24e09a9dfb8bc1c0cd077f2825ea5d` |
| `tests/paea_source_acquisition_inventory.py` | `431036aac49b6077f02d0231da3f4530a5baa10c9d12b16cd08493577f9b10fa` |


### Gi.eth — GI-T3/GI-T4 closure, 8 November 2026

Local fix v0.1.1-dev.347 binds the candidate copy’s initial regular files, including
undeclared ones, to evidence. V2 with unchanged tracked files is refused under R1.
`task review-refresh` preserves R1, history and consumption and requires new controls
and R2. A safe V2 variant resumes through publication/replay; dangerous V2 remains
refused. gieth-review-refresh and gieth-legacy-refresh recipes PASS, retained proofs
verified; inert reviewers do not establish an independent security audit. Normal
profile, active binary identity and actual FR/EN Version dialog verified. Escape
focus restoration was open in witness 347; fixed and verified in FR/EN and both themes
in local version 348 (GI-UI-1, see report). No universal external dependency inventory claimed.
[Sources, results, evidence, unknowns and retrospective (FR)](../../plans/actions-confiance/GIETH-REVUE-V1-V2-CADRAGE.md).


### POC-03 — additive reconciliation, 2026-11-08T17:19:01.705556+00:00

Mikhail sources: https://x.com/i_mika_el/status/2119459613724578163 et https://x.com/i_mika_el/status/2119462379687997652. Preserve the original timeout and append a linked reconciliation event. Examine a consumer-side durable receipt binding key/request/target/committed version/actor. Proposed authorized control, lost response, third-party writer and mutation/log interruption; attribution unknown without reliable evidence, recovery without duplicate effects. See X-IDEE-07 for full scenario/provenance. Code/runtime coverage unknown for this variant; no distributed guarantee, development or test established. Owner Fabrice; explicit deferred resumption.


## BF-L0 — declared-versus-consumed inputs and required corrections, 8 November 2026

**Authorized analysis performed; no code/recipe correction or execution.** No
test, compilation, provider call, payment, service or fixture started/created.
Read HEAD `1fec07d4874cc469c5a3905abb9bb64f855b2d26`, concurrent work preserved. Replan/claim-to-source control.
BF-L0 remains **open**: bounded documentary mapping established, corrections and
observed-effect qualification not done. No BF-L1/BF-P3 passage inferred.

### Scope and actual input semantics

Six recipes: paea-owned-git-promote, paea-owned-source-acquisition,
paea-owned-git-transfer-reconcile, paea-effect-reconcile,
paea-native-git-reference-transaction, paea-native-git-intent. Read step/env and
corresponding Rust/Python consumers; no recipe argv executed.

recipe.py:130–173,185–208,247–281 requires each input glob to match a file and adds
invoked recipe automatically. Excludes .swarm/.git/node_modules/rust/target/current
output, rejects resolved paths outside checkout, hashes **declared** bytes and
compares before/after. An input recipe is hashed, not automatically run as a
dependency. Environment variable presence proves no actual read. Inherited
environment is not fully recorded; overrides only. Cargo launcher hash proves no
rustc/Git identity.

Static glob enumeration with read exclusions, no snapshot/run/collector invocation:
**738/867/738/947/863/865 selected files**, recipe included, zero empty patterns
or escaping resolved paths found. Includes untracked source files then present;
sets may change. Not compiler graph or completeness proof; omissions below coexist
with nonempty globs.

Notation: D = docs/plans/actions-confiance/paea/evidence/claude-tmp-fix-e2e-20261031T173021Z/;
S = D+source-fixture-writer78b-regen-20261031b/; O = D+source-fixture-writer78b-regen/;
C = D+source-fixture-current-pub-regen-20261031/. Names prove no freshness.
Both O/result.json and S/result.json exist, so wrong declaration does not fail
empty-glob validation here.

### Recipe-to-consumer mapping

| Recipe / steps | Consumption traced in current source | Declaration / correction |
| --- | --- | --- |
| owned-git-promote, integration target then Rustdoc | tests/paea_git_promote_call.rs:53–105,154–160,347–365 reads SOURCE→S/result, birth records, opens root writable, reads current publication/Trust. Configured WRITER/CURRENT not read in traced promotion steps. | O/result and C/result declared, not S or its birth. Correct actual source link and consumed proofs. Variable existence is not WRITER consumption; C is extra retained input. |
| owned-source-acquisition, transfer/source units and repository integration | barrier_tests.rs:20–61 via source.rs:317/transfer.rs:405/596 reads WRITER→S and birth; tests/paea_git_repository.rs:110–128,467–485 reads CURRENT→C and SOURCE→S, then Roots. | Neither S/C results nor birth files declared. Add both actual proof sources and birth chain; deduplicate identical S path without merging source/writer roles. |
| owned-git-transfer-reconcile | Transfer units read WRITER→S; paea_git_transfer_call.rs:96–111 reads SOURCE→S, writable root/Trust. CURRENT configured but not consumed by traced steps. | Same wrong O/result; S and birth missing, existing C/result does not cover them. Correct actual links, preserve originals. |
| effect-reconcile, engine/CLI/web/settings | paea_effect_reconcile.rs:135–160,183–197,218–222 reads SOURCE→S/birth/writable root/Trust; :95–97 compiles include_bytes web/paea-examples/v1/payment-document.json. Configured WRITER/CURRENT not read by traced fixture paths. | Wrong O/result, S birth missing, direct payment-document include not selected. Add actual proofs/embed. Other targets' effective inputs still need depfiles. |
| native-git-reference-transaction | Synthetic protocol data; no SWARM_PAEA fixture variable or external birth root used by these cases. | No S/C discrepancy. Declared sample JSON is extra, not evidence this target reads it. Shared runner/compiler gaps still apply. |
| native-git-intent | Intent/protocol targets without S/C fixtures in traced parameters/cases. | No external birth dependency to invent; address shared inputs/effective compilation. |

Birth files actually located, absent from consuming recipes' inputs:
S+native/inspection/workers/planning/setup/owned-root-a418069ad56f41e99ee2c5c8e9e74e9bf0f80545d48a0f714ee60e2baebfe3ff.json;
C+baseline/setup/publication/native/inspection/workers/planning/setup/owned-root-5b232fa9582229588e2fd97dcbe2ea920cc4999afad37d6dc0e48ca83c66bd3c.json.
Born scans candidate files and compares root, not a result signature. Retain
actually examined files/inventory, not just declared PASS. No RootStore/database
opened during this mapping, only proof bytes/existence inspected.

### Transitive inputs and corrections required

| ID | Cause / source read | Required future correction |
| --- | --- | --- |
| L0-R01 refines BF-P2-R1 | S result consumed, O result declared in three recipes. | Replace stale declaration with validated actual input; assess extra inputs separately. Do not switch env back to O to satisfy manifest. Delete no historic files. |
| L0-R02 | Missing results/birth records in native-fixture recipes. | Declare immutable consumed proof and result→birth→physical-instance link. For new L1 fixtures, generate fresh birth inventory then freeze adapted recipe; random old filename is no new invariant. |
| L0-R03 | Runner imports tests/fixture_area.py (215–220), absent from all six manifests; helper reads saved .swarm engine setting or shipped default (34–58). | Add helper; retain relevant resolved setting/provenance/revision projection as invocation evidence. Engine glob covers shipped default, not saved choice. Preserve live-state exclusion, no unnecessary private-state dump. |
| L0-R04 | Declared .cargo/config.toml invokes cargo_space_guard.py, which reads .cargo/storage-policy.json (17–37); both exist, neither selected in six inputs. | Retain effective Cargo config/wrapper/persisted policy hashes, preserve validation/storage refusal without threshold change. Inspect other effective configs at future build. |
| L0-R05 | Direct payment-document include missing from effect-reconcile; Rust globs do not establish other resources/Cargo inputs. | Add certain embed, discover/retain actual compilation inputs through appropriate collector/depfiles. Unused files stay candidates; no broad docs/live-state capture as substitute. |
| L0-R06 | Acquisition declares collector script but no recipe step calls it. Collector imports PROFILE/RECIPE; profiler candidates can add config/wrapper/policy. | Explicitly select future declarative runner versus collector. Collector path needs argv, both modules and enriched inventory retained. Declared script does not imply invocation; collector compiles/tests, not invoked now. Enrichment alone does not cover S/C or missing Python helper. |
| L0-R07 | result.root leads to mutable Root; managed_git_current.rs:48–58,88–145 reads settings/Work/chain/controls; promotion/transfer applies Trust. | Separate immutable birth proof inputs from initial runtime identity/versions/objects and allowed state transitions under barriers. Do not require whole DB unchanged in mutating scenario or hide unexpected mutation. No live SQLite snapshot created now. |
| L0-R08 | Inherited env/tools via PATH; runner records step launcher (247–281), tests later resolve Git. | Retain safe relevant actual config/options/tool identities, unified profiles/features/executable hashes, not secret/full environment. Rebind changed inputs; proof obligation, not new policy. |

### Collector and dependent recipes

Mentioning acquisition.json in promotion/transfer/reconcile inputs does not execute
acquisition or apply its inventory. paea_source_acquisition_inventory.py:65–118
itself compiles no-run, reads depfiles, freezes recipe then invokes runner. Its
preinventory adds profiler/runner and profiler covers web/frontend/local Cargo
configuration. Distinct from recipe.py run; neither route executed here.

Collector refuses test-fixtures/test-util and targets engine units plus repository
integration outside unit-only. Not a compiler binding for arbitrary promotion,
CLI/web or instrumented targets. Qualify future target coverage separately without
weakening normal refusals.

### Status, next steps and future evidence

**No correction applied.** Codex owns R01–08; Fabrice retains later modification/
execution authority. Recommended sequence:

1. Set retention mode and immutable/dynamic boundary; correct R01/02 and shared
   R03/04 retention without changing product behavior/policies.
2. After authorization, effective compilation reconciliation R05/06/08 with
   explicitly separate normal/instrumented profiles. Saved PASS result alone
   cannot admit BF-L1.
3. Future valid control then different consumed path, missing/changed birth,
   removed/changed input, setting/wrapper drift, malformed config and corrected
   recovery. Require refusal before dependent work, retained cause, no old-proof
   fallback. No such regression or recipe created here.
4. After L1–L4 refresh L5 inventory with actual new helpers/barrier/oracle/projections/profiles.

Bounded impact search found **19 manifests** defining SWARM_PAEA_*_FIXTURE,
four within six traced recipes. Other fifteen are candidates, **not semantically
reviewed or declared corrected/conformant**: approval-form-rust,
approval-web-positive, consent-ui-rust, delivered-chain-cli-rust,
git-call-agreement-last-use-rust, git-call-agreement-rust,
git-call-agreement-terms-rust, git-call-approval-rust, git-call-preview-rust,
git-call-web-rust, native-git-source-context, owned-git-transfer,
payment-agreement, promote-cli-suite-regression-rust, transfer-call-cli-rust
(all paea- prefixed). Codex owns follow-up: trace steps/consumers before reuse,
preserve historical verdicts and reassess their exact inventory scope when needed.

**BF-L0 outcome:** mapping and eight required corrections documented in stated
scope; no global conformance or inventory blocker lifted. Current analytical
request handled, lot open for authorized corrections/qualification. No budget,
timer/test/right/monitor created. Only FR/EN plan and local X index changed;
recipes/functions/docstrings/retained fixtures/original proof/version_history
unchanged by this lot.

Static mapping fingerprints; no compiler graph executed:

| Source relue / source read | SHA-256 |
| --- | --- |
| `tools/verification/recipe.py` | `348c9154cf1db9c2c2d6e187d5cb90d86b5125c8290159a6453193b037dbf74b` |
| `tests/fixture_area.py` | `4ece994bd492dc4e44adf24388ed50073ceeff7e74527cd106b31824c8ce566c` |
| `tools/verification/cargo_space_guard.py` | `09b7d45c78c64282dee6265be2d26cea6066505963465e2a50a856f1fe8d5916` |
| `.cargo/storage-policy.json` | `5e392baf7d475a34bd1c4614636682cd4e01bd21b007a3d4845b1e822c7f26dd` |
| `.cargo/config.toml` | `31bee734fefe516086f75b642c1ca537e54997d684e4cc5c983951e01157ca72` |
| `tests/paea_source_acquisition_inventory.py` | `431036aac49b6077f02d0231da3f4530a5baa10c9d12b16cd08493577f9b10fa` |
| `tools/verification/verify_native_profile.py` | `d40d2b32f756eb0454c6dbd25cff6358d56e59bfae3d7e71fc7a33741693ac1f` |
| `rust/crates/swarm-engine/tests/paea_git_promote_call.rs` | `ef4e847c8edc68a6ac1793959e906177d8ac1fceef2859fc40c74c4d226424a5` |
| `rust/crates/swarm-engine/tests/paea_git_transfer_call.rs` | `2d077bd5719d2858dd03cf110bf97c7ee966af903d1c191a3757b9dec2834451` |
| `rust/crates/swarm-engine/tests/paea_git_repository.rs` | `cbcfd28ee30c3ed7955685c242d9b14fbae59a49120ce622b8fa630ba7fe1a91` |
| `rust/crates/swarm-engine/tests/paea_effect_reconcile.rs` | `63effcd53fe74a1d6c65aeede621432a424d9075b7cfbc528431252671078365` |
| `rust/crates/swarm-engine/src/root_store/paea_git_repository/barrier_tests.rs` | `e9a4ce1c6d01905c20f87b62b46073c9e00d56ee02eaa221fa17202026d1af10` |
| `rust/crates/swarm-engine/src/root_store/paea_git_repository/source.rs` | `77ae638b3832bf0ae63939516787580955906269d70f3c5875a3b12b72d63490` |
| `rust/crates/swarm-engine/src/root_store/paea_git_repository/transfer.rs` | `5f85d5eb71556fc8da0fb1a56892901a7574d2338e6288a7c8c50398ae0625f9` |
| `rust/crates/swarm-engine/src/root_store/managed_git_current.rs` | `b57f1a5be938b22af769ad49748ae6017707e4ac0aa1780b943402b8a8e9e54b` |
| `tools/verification/recipes/paea-owned-git-promote.json` | `7ee5855ce0ef3b1c74836a0968e2da3031db882d8954e7dfe2e1b79a3db3e656` |
| `tools/verification/recipes/paea-owned-source-acquisition.json` | `2eb35de6b2851f8ed957701e3d21ce311477eb97eb0c8c5df5e34cff97517866` |
| `tools/verification/recipes/paea-owned-git-transfer-reconcile.json` | `11c2b873e8981d7f1139bf38f26800cf12de12d8a93f840624095b47efe0ff2f` |
| `tools/verification/recipes/paea-effect-reconcile.json` | `56396fbb4f41b6da44cddc8f69ce3dd676a9a40296077ea1c38e0cc74f25e6b1` |
| `tools/verification/recipes/paea-native-git-reference-transaction.json` | `09a4a07914cc9c97eb77c766823c09778a4628a9b7d9e096d58512ddb1bbe43c` |
| `tools/verification/recipes/paea-native-git-intent.json` | `e2d478da120bce223bead78109f427794ef957a05a58cb046c3bb30092e30fdf` |


## BF-L0 — fifteen additional manifests mapped, 8 November 2026

**Analysis/documentation only; Fabrice authorized commit/push to main.** No recipe
or product source modified/executed, no test/build/provider/payment/browser/service
started. Read HEAD `1fec07d4874cc469c5a3905abb9bb64f855b2d26` in canonical checkout resolved from workspace/swarm;
concurrent uncommitted native work excluded. Replan/verified-claims rules applied.

All fifteen previous candidates now mapped through steps/env/consumers/input globs.
Completes the **documentary mapping of 21 selected recipes**, six earlier plus
fifteen here, not complete compiler graph, execution or global conformance. All
fifteen input sets have matches; nonempty globs do not detect omissions below.

### Actual readers and birth-proof distinction

Web paea_git_call.rs:487–514, consent browser:73–103; CLI paea_transfer.rs:36–54,
paea_promote.rs:202–218, paea_approval_web.rs:1884–1906 (under rust/crates/*/tests)
read SOURCE/result, check markers, open Root writable and read current publication/
Trust. **These project functions do not scan owned-root**. Undeclared birth file
is not an omitted actual read there; future fresh-harness BF-Q1 still requires
explicit birth provenance, not an imagined current validation.

Engine promote/last-use scans born; source/transfer units and repository tests
read corresponding S/C birth chains as previously traced. Explicit no-source
cases do not consume a fixture just because environment contains SOURCE.

### Fifteen-recipe matrix

Names below have paea- prefix and .json suffix in tools/verification/recipes.
Declared result does not qualify authenticity/freshness/availability/profile.

| Manifest | Actual traced consumer/input | State / necessary correction |
| --- | --- | --- |
| approval-form-rust | SOURCE .swarm-fixtures/source-fixture-fresh-regen-20261031; web presentation plus Python→paea_git_call→paea-approval-form-ui.cjs. | Result present/declared, no wrong path. Direct scripts/JS harness declared; i18n script/Trust resources missing R09. fixture_area.py included. Built/bound ELF is output, not immutable input; prebuild native_admin.rs touch R12. |
| approval-web-positive | SOURCE docs/.../20261028-acquisition81-mvfdts57/glm-direct-held-lock-guard-review-repair-20261105T104154Z/supervisor-source-normal334-20261105T105919Z; CLI paea_approval_web. | Result and inputs.json declared; traced project reads result/Root, not inputs.json, which is supplementary. Shared helper declared; external pinned image R10. No missing-result finding. |
| consent-ui-rust | SOURCE .../claude-tmp-fix-e2e-20261031T173021Z/consent-fixture-regen-20261031c passed via driver to paea_consent_browser. | Result present **not declared**; driver/JS declared, shared Python helper absent; incomplete i18n, linked outputs separate, prebuild source touch R12. |
| delivered-chain-cli-rust | SOURCE .../chain-fixture-regen-20261031b; paea_promote, designated bin/swarm. | Result present/declared, no born scan. Current bytes differ from pinned SHA: image prerequisite unsatisfied R10, never fix by blindly updating pin. |
| git-call-agreement-last-use-rust | SOURCE .swarm-fixtures/source-fixture-lastuse-regen-20261031b; engine promotion/agreements. | Result present/declared; actual birth scan input omitted. Agreement unit does not substitute. Shared R03/04. |
| git-call-agreement-rust | SOURCE .swarm-fixtures/source-fixture-status-minimal-regen-20261101; agreement units/web. | Result present/declared; web reader no birth scan. No invented path defect; shared R03/04 and mutable Root boundary. |
| git-call-agreement-terms-rust | SOURCE .swarm-fixtures/source-fixture-terms-api-regen-20261031; engine/web/i18n --check. | Result declared; i18n script and both sources/both compared outputs omitted. R09. |
| git-call-approval-rust | SOURCE .swarm-fixtures/source-fixture-approval-fresh-regen-20261031; engine agreement/web approval. | Result declared, no project web birth read. Shared R03/04 and effective compilation inventory pending, no runtime validation. |
| git-call-preview-rust | SOURCE status-minimal as above; web previews/i18n --check. | Result declared; en source/output covered, i18n script and Trust pair omitted R09. |
| git-call-web-rust | SOURCE .../source-fixture-writer78b-regen-20261031b; paea_git_call. | Result exists but old .../source-fixture-writer78b-regen/result declared instead: R01. No born scan by web reader; do not confuse gaps. |
| native-git-source-context | WRITER/SOURCE .../writer78b-regen-20261031b, CURRENT .../current-pub-regen-20261031, source units/repository target. | Two unique results present, none declared; actual birth reads omitted. Deduplicate same path without merging roles, R02. |
| owned-git-transfer | WRITER .../writer78b-regen-20261031b via barrier_tests; Child/pack controls precede transfer units. | Result/actual birth input missing. Child/pack checks do not prove native-source result, R02. |
| payment-agreement | SOURCE .../chain-fixture-regen-20261031b for engine last-use; engine/CLI settlement and pinned image. | Result declared; birth needed by promotion omitted. External CLI pin matches read bytes, not profile qualification or finance capability. |
| promote-cli-suite-regression-rust | SOURCE .swarm-fixtures/source-fixture-lastuse-regen-20261031b; entire CLI paea_promote include-ignored. | Result declared, CLI project no born. Compiled payment-document include absent from inputs R05. Default Cargo binary selection is not implicitly delivered image. |
| transfer-call-cli-rust | SOURCE .../chain-fixture-regen-20261031b; CLI transfer then promote. | Result declared, no birth scan. Same compiled payment-document include omitted R05; filtered case does not mean uncompiled rest of target. |

Ellipses for legacy source roots use docs/plans/actions-confiance/paea/evidence/
claude-tmp-fix-e2e-20261031T173021Z/. Exact env paths remain in retained manifests.
Birth files exist in examined proof directories but count as consumed only when
actual readers open them.

### Additional corrections and deliberate output distinctions

| ID | Read fact / scope | Required correction before reuse |
| --- | --- | --- |
| R09 i18n | build.cjs:1–23 --check reads locales/en.json, locales/trust-actions-en.json, web/i18n-en.js, web/trust-actions-en.js. Script missing in approval-form/consent/terms/preview; Trust pair absent in all four, terms retains none of four resources. | Declare script/missing resources. Existing generated --check files are immutable read inputs, unlike ELF produced by current run. No i18n build now. |
| R10 external image | CLI promote:171–198, settlement:379–395 and approval-web selector:2160–2177 read selected image and expected SHA before spawn. | Retain selected path/pin and bytes/profile provenance. No Cargo fallback for invalid designated image; matching hash proves bytes only, mismatch blocks recipe. |
| R11 build/browser outputs | bind_cargo_test_binary resolves produced ELF through Cargo, binds then rechecks; browser drivers read server/phase/result files produced during run. Relevant binding scripts declared. | Do not make run-produced outputs immutable inputs. Retain linked outputs/initial-final identity/causes. --binary may denote a spec to resolve, not an existing file. Existing countertests not executed. |
| R12 forced source touch | Both browser recipes touch web/src/service/native_admin.rs via sh -c before cargo no-run. | Mtime/launch mutation is neither byte change nor ref→receipt fault. Future procedure must use supported artifact resolution/binding, preserve shared builds, not touch sources to manufacture freshness. No change/run now. |
| R13 browser dependencies | JS uses Puppeteer from node_modules and configured Chrome; drivers pass SOURCE and spawn Rust/Node. Runner excludes node_modules. | Retain needed package manifests/lockfiles and actual tool identity/served resources/outputs, not wholesale node_modules/private sessions. Workspace web bytes do not prove served assets. No browser/service run. |

Shared R03/04 remain: Cargo wrapper/storage policy absent from **all fifteen**;
fixture_area.py declared in approval-form/approval-web-positive, absent in thirteen.
Every recipe uses runner importing that helper; neighboring declaration does not
transfer. Nonempty inputs are no validation of actual dependencies/tools.

### Read-only checks of designated external images

| Recipe | Expected SHA | SHA of bytes read | Bounded conclusion |
| --- | --- | --- | --- |
| approval-web-positive | 23651e383c4b7eb6ba1b25bf66d0c63d3e58dd8adf6ab18435fe3cfb7390514c | 23651e383c4b7eb6ba1b25bf66d0c63d3e58dd8adf6ab18435fe3cfb7390514c | Present/matching bytes, no execution/profile qualification. |
| delivered-chain-cli-rust, bin/swarm | 84b1e78c8dbd9f38c5ebebb7fd48c097f6e7de76805249fd06592730aa5211a8 | 3512fdaed47a9f111959de2cfd8186276d1d32d073da510e0dd8e86913b0735f | Observed mismatch, image prerequisite unsatisfied at read; no failed test or pin rewrite. |
| payment-agreement | 3146c83791e9cd5e98f264d3d4af0bdf2fc6ee9fb246d145dcc88fd8c4cc313a | 3146c83791e9cd5e98f264d3d4af0bdf2fc6ee9fb246d145dcc88fd8c4cc313a | Present/matching bytes only, no financial capability. |

Future alternatives: recover qualified historical image/profile/evidence, or
select independently qualified current image with adapted recipe. Updating pin
alone does not qualify found bytes. Codex verification owns diagnosis, Fabrice
authority; no selection/restart/requalification done here.

### BF-L0 state and remaining passage

Fifteen candidates now semantically traced within above input chains; earlier
unreviewed status remains historical. R01–08 plus R09–13 retained, apply only to
actual consumer. Future BF-Q1 births still needed even where current web reader
does not read them: explicit requirement/integration, not presumed validation.

**BF-L0 open: no correction applied or execution proof obtained.** Before L1:
select proper fixtures/images, correct actual links/helper/wrapper/config/embeds,
separate Root state and outputs, qualify effective targets/profiles under later
authority. No normal collector refusal weakened for fixture instrumentation.
Negative/freshness/recovery checks remain future work.

Git: current request authorizes documentation commit/push on main. Push includes
main's two pre-existing local commits 50f587e57 and 1fec07d48, inspected as retained
design/history, excluding uncommitted native work. Campaign X index remains local/
Git-ignored, no force-add or X publication. No version_history modification by
this documentary work, installation or embedded-context reload.

Targeted read fingerprints, no compiler graph or execution:

| Source / manifeste relu | SHA-256 |
| --- | --- |
| `tools/verification/recipe.py` | `348c9154cf1db9c2c2d6e187d5cb90d86b5125c8290159a6453193b037dbf74b` |
| `scripts/i18n/build.cjs` | `868b3ffd89ec3232ad045e1b97ceb5ef2f6dab770a3a74f8a1f416bbefa64f19` |
| `tools/verification/bind_cargo_test_binary.py` | `fe67a8265048459a4e2d355558a807eb58faa7c0ed801242848d4f1037ac72de` |
| `tools/verification/test_bind_cargo_test_binary.py` | `9f1f6c308538660551bb78496aa3f4a7be349048c452e0b0d44675595d908ecc` |
| `tests/paea_approval_form_ui_recipe.py` | `89d208d00a834521516973607e28bfe83264e319ed430ddf63a74ae9e83b92f1` |
| `tests/paea_consent_ui_recipe.py` | `48640a6a806c5337b59b083c12c2eb9dcfbd026a5f589ca047f26e8d728e3a8d` |
| `tests/test_approval_resolution_raw_evidence.py` | `116bac6f48ce163145fab2451d7f87756727b940fba3477bbd8094e5d66a4394` |
| `tests/test_approval_driver_stream_drain.py` | `403a1091fc664c5b95f366747611ed5ad7e207507337d30d9892265c9e35960a` |
| `tests/paea_consent_driver_control.py` | `68c8e73e5f6600ec077685ffeff222fffd97e4c2e0b5456a41046deb26a68e88` |
| `tests/paea-approval-form-ui.cjs` | `e5769979c36c5adc0ab55b1ab9765c744da45612fc6751f00ddd069ce5e7832a` |
| `tests/paea_consent_ui.cjs` | `7c3ae408519b28f8ff0ecaa769836bc35b03c72ffe27ab586236a2a7efd7fbc6` |
| `tests/paea-reader-credential-erasure.cjs` | `ec275c9a0cd6bbbab8c8f03a3833c67c2dfac0e3a8fa7cabecb58adaa3082f37` |
| `tests/paea-consent-projection-erasure.cjs` | `eb2f6df808ff31271f1d9062a5623039b979f211035dab1c73effa9d67d79629` |
| `rust/crates/swarm-web/tests/paea_git_call.rs` | `181881ac0c5f2ad9ebd3e7d146bce0dd35358ada862b2bfe2d0bde5853760b75` |
| `rust/crates/swarm-web/tests/paea_consent_browser.rs` | `21770cf4ab1b716f21ae8ebd700c4e4aefa0c70cb6626235a37b377a5f1adb60` |
| `rust/crates/swarm-cli/tests/paea_approval_web.rs` | `60ba4a8ca969f73715e6cd555ba9cc97bcc4c4ceb4d230513cc0f29e593fe4c0` |
| `rust/crates/swarm-cli/tests/paea_promote.rs` | `fe1a9f7a6e3e1536f42c33c9a1445260dd0bc6c816a7ceb4d3a9f2ff8c928883` |
| `rust/crates/swarm-cli/tests/paea_transfer.rs` | `36c24aa26d2b03328ce976c8d3e67434def64ff46dd2baf5c7dd142b8ac87eeb` |
| `rust/crates/swarm-cli/tests/paea_settlement.rs` | `ac258f0a640e0477d16f57d3d998281a80b89174882b0572e14d3eda0acc9bc6` |
| `rust/crates/swarm-engine/tests/paea_git_promote_call.rs` | `ef4e847c8edc68a6ac1793959e906177d8ac1fceef2859fc40c74c4d226424a5` |
| `rust/crates/swarm-engine/tests/paea_git_repository.rs` | `cbcfd28ee30c3ed7955685c242d9b14fbae59a49120ce622b8fa630ba7fe1a91` |
| `rust/crates/swarm-engine/src/root_store/paea_git_repository/barrier_tests.rs` | `e9a4ce1c6d01905c20f87b62b46073c9e00d56ee02eaa221fa17202026d1af10` |
| `tools/verification/recipes/paea-approval-form-rust.json` | `01f269b6f3f7a0e7da79e52e644885917880f631b909f0760fbc754bb3cc3caa` |
| `tools/verification/recipes/paea-approval-web-positive.json` | `cd500c649195bd51635b7d63aa90d10a6c21ccbebf878d90ab65d1371c8e47e6` |
| `tools/verification/recipes/paea-consent-ui-rust.json` | `f5cc8eb21873d1ae2a4ca4819300d58f708e576b4aae98d482a9cef721bc1cde` |
| `tools/verification/recipes/paea-delivered-chain-cli-rust.json` | `87ff5d326930efcdcaf2da0547e8604a1cded3bdd55e4180d0cf6698395ce3ce` |
| `tools/verification/recipes/paea-git-call-agreement-last-use-rust.json` | `125a1b1fa493881e3c35b375b98e6107a838b96e370a67936dc27bfae101d3ed` |
| `tools/verification/recipes/paea-git-call-agreement-rust.json` | `3167b0fc0ec3448577a575f64fd28b03a38e1cf94179d5aad63f5ed7e39d8aad` |
| `tools/verification/recipes/paea-git-call-agreement-terms-rust.json` | `323d43149ffe756f8e8a8fd805971986875c9cecd6968625b08b031db842794b` |
| `tools/verification/recipes/paea-git-call-approval-rust.json` | `ea8d353ab70d07595bd6e90a94ab8c2f6b1fea8fafd9d974c36df8c13b370b11` |
| `tools/verification/recipes/paea-git-call-preview-rust.json` | `fc0c156d85a5f4639e29a045230d31b525a27960d6a1437417abe774e7aa28ac` |
| `tools/verification/recipes/paea-git-call-web-rust.json` | `869ef72a0e4c5e24d757a16c847e690cd42d0b87187854bf9e69ab128a9aaa41` |
| `tools/verification/recipes/paea-native-git-source-context.json` | `f5444fe972909ebb948456d9020d859b85b29af22de544a4114b78f098f1a9c2` |
| `tools/verification/recipes/paea-owned-git-transfer.json` | `7e0dc77b8f8e229e86d6c23505b6b7c40a84f33f2ea8f438a0a7bf8f206025e9` |
| `tools/verification/recipes/paea-payment-agreement.json` | `e0e6024b48a546ab75cfcdb85ab050ac087b592b7de9116cd770aaa15372d54d` |
| `tools/verification/recipes/paea-promote-cli-suite-regression-rust.json` | `9f0dc5c619426b04c512f8c98eadb892e48177f8b34ff999862d1b58f592ff7e` |
| `tools/verification/recipes/paea-transfer-call-cli-rust.json` | `3ad8afeaa1071396eba837c83249e7ad0848a0edbeed1f1960a923cb0e2268e8` |


### Git replan — protected main publication refused, 8 November 2026

Documentation commit created: `19b1e948125a9521389b33d6ef46c97ae187b651`, only the
two FR/EN plans, 1036 added lines including prior work. Direct main push **rejected
GH006**: PR required and two expected checks. Read protection: enforce_admins=true,
strict=true, contexts checks/install, zero minimum human approvals. Local commit
preserved; no successful main push claimed.

.github/workflows/ci.yml matches HEAD/origin/main, Git blob
`329f4f498d2ed958316785c49444eec65f004505`. Push trigger main only, pull_request
without documentation filter; checks/install run Cargo/npm/install tests. Opening
a documentary PR would therefore run tests forbidden by current request. No
workflow/check/protection/history changed to bypass; no PR opened.

Preservation route within push objective: remote target
`codex/bf-l0-input-audit-19b1e9481` from documentary HEAD, no checkout switch or
uncommitted native work included. This branch push does not trigger read workflow;
actual push/ref success must be observed through Git. Main remains unintegrated.

Resumption owner: Fabrice for authority to run PR-required CI; Codex then uses
protected flow. No unchanged main-push retry. Prepared PR title
`docs(poc03): map BF-L0 recipe inputs and remaining blockers`; body describes
fifteen additional static recipe mappings, R09–13 and retained limits, documentary
hash/diff validation only, no local tests. No product capability, BF-L0 clearance
or independent acceptance claimed.
