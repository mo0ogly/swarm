# PAEA operational guide integration

[Français](../../../../plans/actions-confiance/paea/GUIDE-INTEGRATION.md) ·
[Adopted plan](../PAEA-APEX.md) · [Retained guide](../../../../security/PAEA_v0.2_Guide_Operationnel.docx)

## Status and decision

The user supplies the guide to understand PAEA and inform implementation. It is
a **working operational reference**, assessed against actual sources, proof and
existing decisions. Document prescriptions are proposed requirements, not new
execution, delegation, payment or deployment authority. The original DOCX stays
unchanged, SHA256 `e621efa0953ff056cf12b318236849764feeff16f9e2d2f3c5b4016d75774298`.
Keep PA-00…13 and both fictitious-payment/Git pilots. IAM and industrial safety
remain generality examples outside authorized execution.

Read all 288 text/table blocks and both figures; no tracked changes/notes. A
34-page render was produced and relevant pages inspected. This is a research
architecture and recipe guide, not an already deployed system. Annex-E external
references are not reassessed here.

The current **PAEA checks** screen checks file correspondence. The guide also
requires consequence decisions, authority, human approval, protected commitment,
independent effect observation and deployment decisions. Advance toward a complete
pilot; more shape assertions alone do not demonstrate an effect boundary.

## Mapping and actual state

| Guide | Plan/owner | Observed status and required outcome |
|---|---|---|
| §§1–3, annex A: consequences/risk/utility | PA-00/01, business/risk owner + Codex | General objective retained; attributable effect card, consequence analysis, oracle and decision still needed. No C3 inferred from amount. |
| §4: threat/TCB | PA-00/02/04/09, identity/operations | Target architecture only; inventory actual accounts, privileges, secrets, FDs, sockets, paths and versions. Component names prove no isolation. |
| §5: guardians/point of no return | PA-02/03, authority/effect services | Protected interfaces not demonstrated. Current checks where the sink can still refuse; ALLOW/coherent receipt is not an effect. |
| §6: provenance/memory/confidentiality | PA-05/06, memory/identity | Data shape exists; attribution and preservation across read/summary/compaction/withdrawal/transmission remain unproven. |
| §7: identity/delegation/keys | PA-02/06/07, protected control | IA3 dependencies retained. Resolve actual principal/instance/mandate, protect keys, attenuate rights/check audience; see ERR-01. |
| §8: canonical intent/exact approval | PA-01/03/08, contract/business services | Canonicalization/reports locally tested on dev77; context is still supplied data. Approval shape is not authenticated. Supplier/invoice/account resolution and presented UI evidence remain open. |
| §9: policy/revocation/order | PA-02/03/07, control/sink | Specified, sink proof pending. Persist operator policy with Admin/API/CLI; no constants copied from examples. |
| §10: mediation/egress | PA-04/05, AC-02 | Rust/AC-02 prerequisites retained. Witness then bypass attempts under actual agent/descendant rights; every path declared. Observational eBPF is not enforcement. |
| §11: uniqueness/UNKNOWN | PA-03/09, sink/verification | Pure lifecycle tested; no protected PAEA intent registry or real reconciliation proven. Retain attempts/intent/receipt/consumption; no fresh key to escape UNKNOWN. |
| §12: F-12 journey | PA-00…10, integration | Adapt steps A…L to both adopted pilots. A narrated scenario is not demonstrated behavior. |
| §13: IAM/industrial safety | Outside current execution, sponsor | Retain lessons; no live connector, safety policy or new mandate adopted. |
| §14/annex B:17 invariants | All plan owners | Existing register retained; **0/17 end-to-end demonstrated**. Bind each property to owner/boundary/oracle/counterexample/proof. |
| §15/annex C: falsifiable recipes | PA-03/09/10/12, verification | 18 CLI tests/973 calls and174 bridge assertions remain separate local proofs. Three initial recipes need independent sink state and valid controls before faults. |
| §16: B2/ablation/statistics | PA-11/12, evaluator/sponsor | Existing E3 protocol prepared, not preregistered. Realistic B2/comparable conditions, independence, false refusals, utility, cost, latency and unknowns; assertion totals are not independent trials. |
| §17: incidents/requalification/humans | PA-08/09/10/13, operations/supervisors | Connect procedures to actual supported operations/effects. AI review is not human approval; memory withdrawal/secret deletion does not revoke prior copies. |
| §18: dossiers/status | PA-00/09/10/11, dossier owner | Separate design, identified realization and qualification. Retain NOT EXAMINED, SPECIFIED, IMPLEMENTED, LOCALLY TESTED, INDEPENDENTLY EVALUATED, REFUTED, OUT OF SCOPE. No E2/E3 claimed. |
| §§19–20: milestones/workshops | All plan owners | J1→PA-00/01/02/03/09; J2→04/05; J3→06/07; J4→11/12; J5→11/12/13. Five workshops are a method, not a new schedule/monitor. |

## Clarifications before technical consumption

**PAEA-ERR-01 also applies to guide §7.1 (rendered page10).** Content-bound single
use does not replace current revocation checking. Counterexample: token issued,
revocation recorded, still-unused first consumption; a sink checking only content
and uniqueness may accept it. Retain PA-02/03/12 and §9.2's requirement: current
epoch/state under sink-recognized commitment order, or declared propagation and
refusal of unsupported immediate-revocation scope. T05/T17 must include **first**
consumption after revocation. Owner: effect control; requirement clarified,
distributed proof still pending. Original guide/historical evidence unchanged.

**PAEA-GUIDE-02: illustrative representations.** Section8 explicitly calls its
JSON pedagogical/unstabilized; it does not replace `contract-v1.schema.json`.
Figure2 includes `montant:4200.00`, while the text/current profile use
`amount_minor:"420000"`. Do not copy figures as payloads, policy, product branding
or conformance schema. Current profile rejects numeric amounts, claims no JCS and
retains v1 vectors. Owner: contract/docs; documented consumption rule, no schema
change or security-test result inferred from an illustration.

**PAEA-GUIDE-03: debt identity.** The guide illustrates `payment:S01:F-12`;
current intent derives from `project_id+invoice_id`. Compatibility requires the
protected producer to resolve `invoice_id` to a project-unique debt object, not
a raw invoice reference reused across suppliers. Require that registry contract
and collision/alias/correction tests before PA-03. Keep v1 now; if the registry
cannot provide it, use explicit versioned schema/migration, never an implicit
hash-field addition. Owner: business service/PA-03; semantic prerequisite open,
no protected business uniqueness inferred from current hashes.

## First operational slice and target UX

Prepare an **effect card**, accountable owner, threat model, access graph,
canonical intent, policy, commitment point and three oracle-backed recipes. This
prepares PA-02/03 without bypassing protected authority or Rust/AC-02 prerequisites.
Payments stay fictitious; Git remains the adopted second domain. Actual empowered
people/services must be identified rather than invented by the model.

| Initial recipe | Control and fault | Required sink observation |
|---|---|---|
| T04 substitution | Valid approved action then relevant mutation; equivalent key ordering accepted under contract | No changed-content effect; retain earlier approval/effects. |
| T06/T21 lost response | Legitimate commitment then lost reply/crash; fresh attempt key proposed | One business operation; UNKNOWN/reconciliation; absent idempotence/stable status refuses assurance. |
| T05/T17 concurrent revocation | Valid capacity, barrier before commit, ordered revocation then first use; reverse ordering too | Revoke-before-commit refuses; earlier commitment remains historical; sink order, not two client clocks. |

Target PAEA menu in separable Projects/Processes UX must support:

1. Action dossier: consequence, owner, sources, justified classification/risk scope;
   missing values remain visible.
2. Exact action/diffs, observed mandate/policy and approval by an empowered identity
   where required by policy.
3. Actual commitment/refusal/UNKNOWN/reconciliation, without unrestricted reissue
   or deriving “paid” from a coherent report.
4. Incidents/recovery and design/realization/qualification dossiers, versions,
   evidence/limits and attributed decisions.
5. Authorized **laboratory** campaigns and sink-oracle results, separate from
   business execution and developer tests.

**Undelivered target.** The current screen remains correspondence-only; do not
invent business routes/acceptance to populate it. Future metrics/policies follow
the shared configuration contract, not example F-12, five minutes,200actions/hour
or illustrative C3. Changing a menu title never implements this operational flow.

## First executable preparation

[Pilot dossier](OPERATIONAL-PILOT.md) and [increment 14](INCREMENT-14.md): isolated
SQLite/Git effects, three fault families in both domains, separate oracle
process and raw debt-ID counterexample. Seven tests PASS, integrity VERIFIED;
negative witness detects the weakened Git guard. Simulated authority/common UID,
no campaign menu or protected native delivery. These proofs prepare PA-02/03;
**0/17 end-to-end invariants** remains unchanged.

### GUIDE-03 follow-up — increment 16

The pure resolver and `protection resolve-payment` CLI now implement an explicit
supplier/reference → stable debt mapping. Collisions and stale versions refuse;
two suppliers may share a reference. [Contract](../../../PAEA-CONTRACT.md) and
[increment 16](INCREMENT-16.md). Protected production, ID continuity across
snapshots and freshness at commitment remain open; GUIDE-03 is not globally closed.

### Operator journey — increment 17

PAEA checks now links to invoice-reference resolution using registry/request
files and the shared API/CLI resolver. Exact amount/revision, non-authority
markers and raw export; [evidence and limits](INCREMENT-17.md). This activates
neither a protected registry nor UX laboratory campaigns.


### Guided examples — increment 20

All three views provide versioned fictional file pairs to download and explicitly
select. Help explains receipt sink/state and expected data results; [evidence and
limits](INCREMENT-20.md). The link triggers no laboratory campaign, project data
operation or native protection.


### Read exact terms — increment21

The menu offers Inspect an action: exact terms, declared context, ordered sources,
i64 values and shared CLI/API report. [Evidence/limits](INCREMENT-21.md): reading
prepares an approval request without creating an approver, approval or effect.


### Supplied approval correspondence — increment22

CLI/API/menu now consume exact supplied content/policy binding data; invalid inputs
refuse and mismatches yield explicit negative reports. [Evidence/limits](INCREMENT-22.md):
no authenticated approver,current rights,human attention or protected approval is
established. IA3/AC02 prerequisites remain open.


### Campaign access — increment23

The menu exposes the laboratory command and local report reading. Eight real
scenarios/oracles,506nativeassertions and56installeddev109checks PASS; the raw
alias counterexample remains explicit. No browser execution,human approval or
protected invariant established. [Results and recovery](INCREMENT-23.md).
