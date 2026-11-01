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
| BLP-04 | Protected attribution, context/memory/output propagation and replays | BLP-03 | PARTIAL — private v3 prompt provenance; classification, principal and inheritance open |
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
