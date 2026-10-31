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
| BLP-04 | Protected attribution, context/memory/output propagation and replays | BLP-03 | PA-06 |
| BLP-05 | Actual filesystem/network/provider/descendant mediation, no unconfined fallback | BLP-04 | PA-04/05 |
| BLP-06 | Established human identity, approval of exact bytes/recipients, new copy | BLP-03–05 | PA-02/08/09 |
| BLP-07 | Retained recipes, candidate review, local delivery, served assets and RETEX | BLP-01–06 | PA-09/10/11/12/13 |

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
