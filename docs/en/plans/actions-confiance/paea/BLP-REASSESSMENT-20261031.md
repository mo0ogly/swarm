# Reassessment of previous PAEA work against BLP — October 31, 2026

[Français](../../../../plans/actions-confiance/paea/BLP-REASSESSMENT-20261031.md) ·
[PAEA plan](../PAEA-APEX.md) · [BLP technical plan](../BLP-APEX.md) ·
[115-action register](BLP-REASSESSMENT-20261031-ANNEX.md) ·
[Data and hashes](../../../../plans/actions-confiance/paea/BLP-REASSESSMENT-20261031.json).

## Result and scope

The applicability reassessment covers **115 documented actions × 7 BLP packages =
805 decisions**: 92 increment files (01–93 excluding 82) and 23 actions 94–116 in
the main plan. This is documentary review coverage, not implementation progress
or compliance. Each action has source/section hashes, seven decisions, a reasoned
family, reference producers/consumers, historical version mentions, an assigned
gap/owner and a recovery condition.

Method: review retained increment descriptions/limits and registers; assess
representative current producer/consumer contracts by responsibility; verify
retained BLP recipe integrity and compare inputs. Families share semantic
rationales, not keyword-based conformity verdicts. This is neither a review of
every function in every increment, a fresh behavioral campaign nor independent
acceptance. Historical outcomes remain unchanged.

**Decision: reopen or retain open the applicable confidentiality criteria.
BLP-07 remains open.** Integrity, exact approval, uniqueness and recovery evidence
retains its historical value; it establishes no transversal confidentiality.

## Review by responsibility

A = applicable/open; P = useful partial evidence/open; N = not applicable to the
primitive alone with an explicit rationale; C = conditional on lower-classified
output, unresolved until actual classifications/recipients are established.
C grants no exemption. The annex contains all 805 decisions.

| Family / actions | Examined producer → consumer | Preserved result / required recovery | Gap |
|---|---|---|---|
| Offline contracts / 12 | SourceRef/canonicalization → correspondence CLI/API | Keep structure and hashes; actual attribution, reader clearance and classified output remain missing; pure computation declassifies nothing | BLP-RET-01 |
| UI/API / 17 | Checkers/engine preview → page/response/download | Rendering/session evidence remains useful; verify actual readers, metadata and errors under BLP | BLP-RET-05 |
| Laboratory / 4 | Fixture effects/pilot → oracles/reports | Keep effect oracles; add non-disclosure and declassification scenarios | BLP-RET-03 |
| Identity / 21 | Current account/permission → held control | Held authority is useful; BLP clearance and distinct declassification rights remain missing; human fixture is not a real human | BLP-RET-04 |
| Private storage / 8 | Descriptor-bound IO → packages/service artifact | Keep atomicity, modes and zeroization; classify copies/readers in composing layers | BLP-RET-02 |
| Linux boundaries / 12 | Kernel subject/channel → dispatcher | Peer attribution establishes neither authorized flow nor Internet confinement | BLP-RET-03 |
| Settlement / 8 | Guarded call → ledger/receipt/stdout | Keep agreement, uniqueness and recovery; classify context, ledger, readers and receipts | BLP-RET-01 |
| Git / 16 | Held source → pack/import/ref | Preserve OLD/idempotence; object transfer discloses before ref publication | BLP-RET-03 |
| PA-09 reports / 2 | Journals → recomputation/export | Correspondence is useful; protect identifiers/reports; recomputation is not confidentiality | BLP-RET-05 |
| Qualification/delivery / 6 | Profile/evidence → declaration/register | Keep bounded historical evidence; requalify exact BLP candidate; referenced E2 originals not found | BLP-RET-06 |
| Approval / 5 | Engine action/agreement → approver/last use | Keep exact Git agreement/revocation; specific declassification is still missing | BLP-RET-04 |
| Tools/leases / 4 | Preflight/memory → launch/recovery | Cited proofs reject mutation/stale leases; bind domain/sources/policy to actual consumers | BLP-RET-02 |

N does not exempt callers. IO and credential primitives need not become a second
policy engine; composing protected producers/consumers still owe applicable BLP
checks. Ledger writes and Git promotion grant no declassification right.

## Open findings, owners and recovery conditions

All findings below are **OPEN**. Blocking means blocking the corresponding BLP
claim, without erasing earlier historical outcomes.

| ID / severity | Observation and impact | Owner / observable closure condition |
|---|---|---|
| BLP-RET-01 / blocking | SourceRef labels are non-authoritative. The engine ExecutionDomain consumer located is diagnostic blp-check, explicitly runtime_authorized:false. Canonicalization establishes no provenance. | PAEA/BLP integration: attribute actual versioned bytes and current policies to sources/recipients; demonstrate authorized control and refusal without transmission. BLP-01/02/03/04, I03/I13. |
| BLP-RET-02 / blocking | Fixed domains/join exist in the pure monitor; retained evidence does not establish binding across actual contexts, memory, compaction, recovery, copies and secrets. Memory path guards are not BLP admission. | Launch/Memory/Storage: trace exposed context/derivatives, explicitly classify legacy data, preserve restrictions and deny incompatible recovery; retain Biba regressions. BLP-03/04, I03/I13/I14. |
| BLP-RET-03 / blocking | Inspected Biba profile allows readable leaves outside the private root and denies AF_UNIX sockets; it provides no general Internet destination control. Effect permissions and pack protocol do not control disclosure. | Confinement/connectors: mediate the required bounded BLP subset, inventory actual reads/egress including descendants; valid control then receiving-oracle non-disclosure, refusal/recovery without fallback. BLP-05, I06/I07/I12/I13. |
| BLP-RET-04 / blocking | declassification_available:false; existing Git/effect agreements do not authorize lowering classification. Real human consent to the declassified copy is not established. | Identity/approvals, sponsor for real person: specific right, exact bytes/target classification/recipients/revision, retained original, revocation before release and substitution refusal. BLP-06, I04/I05/I10/I13/I17. |
| BLP-RET-05 / major | Preview responses expose declared proposal/context under native session. Reports/downloads/errors/evidence are not qualified for distinct BLP readers. No real-data exploitation was attempted. | UX/API/PA-09: shared reader/output policy across CLI/API without leaks through values, identifiers, errors, logs or exports; authorized/denied reader tests and FR/EN on exact binary. BLP-04/05/07, I03/I13. |
| BLP-RET-06 / major | Referenced DECLARATION-E2.md and TRIAGE-PAEA17.md under claude-e2-arbitrage-20261031T135859Z were not found by direct read or targeted search including ignored files. This does not establish absence from an unexamined archive. Historical PA-10 closure is not reaccepted here. | PAEA author/qualification: locate originals/hashes and bound profile or create explicit new evidence without altering originals. This announcement cannot close BLP-07. |
| BLP-RET-07 / minor traceability | Plan says 82 was unassigned; increment83 claims qualification82. No action82 is invented. | History owner: locate primary source and append attributed erratum; preserve both statements meanwhile. |
| BLP-RET-08 / major evidence | Intact BLP recipe records 10+5+14 actually executed tests, with declarative coverage lacking full production profile, real UI/API or network oracle. Of 525 declared inputs only tools/verification/recipe.py differs at observation. | BLP qualification: retain historical PASS with its original runner; assess changed-runner effects before reuse and complete effective-input/public-flow evidence on target candidate. BLP-07 stays partial. |

## Retained evidence and limits

`recipe.py verify` on `verification-blp/20261031-recipe-run` returned
`integrity:VERIFIED`, `recorded_status:PASS`. Original outputs confirm 10 monitor,
5 engine integration and 14 Biba coexistence tests. The listing step is not
another test. No new behavioral execution was needed to establish those
historical results. Input comparison found 524 unchanged entries and one changed
runner out of 525; reuse remains bounded to retained bytes/runner, not a fresh
PASS on the entire checkout. See
[verification details](../../../../plans/actions-confiance/paea/evidence/blp-reassessment-20261031-01/recipe-verification.json).

Per-action review retains references; it does not verify the integrity of every
old cited evidence directory. Historical versions/hashes are not represented as
the running server. No build, provider call, live database change or restart.

### Evidence added during review: foundation delivery/UI

BLP tracking changed during plan editing: the patch refused before writing;
the concurrent Claude session's new sections and retrospective were then read.
Initial reviewed inputs remain archived; this supplement does not replace them.

Additional artifact inspection: build282 reports VERIFIED_NATIVE_CANDIDATE and
VERIFIED_DEFAULT_NATIVE_PROFILE, no_fixture_feature:true, matching SHA256
ea072ccc00fb5ed77ae88a6a53b3d4eba9d406a8d55fedf8e50ed01e3663bbe6.
Browser checks.json contains nine checks and zero errors. They cover the page,
no-clearance example, typed refusals, FR/EN, theme and width390, not positive
administration or BLP flows. That session records installation282 followed by
concurrent replacement283 without its BLP release note embedded. This review
does not revalidate the active process and claims neither as currently verified
here. The build record itself declares installed:false.

This additional evidence narrows foundation UI observation gaps; it closes no
actual confidentiality gap. BLP-RET-08 concerns remaining positive/actual-flow
coverage, target profile freshness and runner reuse, not absence of build282.
The delivery retrospective remains a bounded result. Transversal BLP-07 still
depends on BLP-01/04/05/06 and applicable open findings. See the
[sourced supplement](../../../../plans/actions-confiance/paea/evidence/blp-reassessment-20261031-01/supplement.json).

## Recovery decision and transmission

Two routes compared: repeat all historical campaigns, or preserve their scope
and target missing BLP boundaries. Choose the latter to retain useful proofs
without treating integrity test volume as confidentiality evidence.

1. BLP integration: close BLP-RET-01 then producer/consumer BLP-RET-02; saved
   policy alone closes neither.
2. Confinement/connectors: BLP-RET-03 with current owners, preserving their files
   and processes and the deferred remainder of AC-02.
3. Identity/UX: BLP-RET-04/05; roles, LLMs and operator sessions cannot replace
   required human approval. Independent work need not wait for the human.
4. Qualification: BLP-RET-06/08, then BLP-07 on the exact candidate/profile and
   separate review. BLP-RET-07 remains assigned provenance debt.

The per-action register and findings are persisted and linked from both plans,
ready for the user-selected GLM/SKYNET recovery. Reception by an already running
session, supervisor decision and engine effect remain unverified. This audit
sends no external messages or launches. The documentary reassessment is complete
within its scope; corrections, confidentiality verification and global acceptance
remain open.

Audit/documentation only: no runtime policy, product function or error-path
changes; no release-note update or installation needed. Existing policy/function/
error contracts were inspected at the stated boundaries, without claiming a
complete review of the entire codebase.
