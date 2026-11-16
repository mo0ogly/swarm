# WIP — Gi producer candidate transition

Source reviewed and SDK compilation succeeded; functional verification remains open: do not merge as accepted.

First checkpoint7088121 counterexample R09-CUSTODY-1 is resolved at source through distinct holders. sources::request acquires/refreshes the preclaim candidate outside SQL; its callback retains preparation/claim transactions and records known Work/claim before close. sources::job then acquires custody of the actually recorded running review outside SQL; its callback retains owner/freshness/clock/expiry/history checks. No equality/I0/budget/TTL protection or limit removed. Errors after claim do not imply rollback.

```mermaid
flowchart LR
 A[Preclaim without review] --> B[Recorded claim]
 B --> C[Close preclaim holder]
 C --> D[Holder of exact review]
 D --> E[Checks and expiry]
 E --> F[Close and reports]
```

Reviewer09 APPROVE_SOURCE_ONLY: SHA6b3d83798f27900c0c035197cf96ef41e870693c99adeef52a15b554e0fc9263. Source5c0e1ab29b4eb87673b126f2314c34d5c8ef343d5ca957894d8a8b567f1ff14a; private evidence /run/media/fpizzi/SWARM/swarm-evidence/root00-gi-custody-transition-u41ymcya. One separator blank line removed; no other function body changed.

Still required: actual healthy managed witness before wrong/nonrunning review and cleanup retaining knownWork, then controlled recovery. No functional test/W/SDK/F/installation/acceptance performed for this candidate. Failed Work w-83e055eae3e47c27a0f5043c, last observed rev14/calls0, is untouched. Historical scenarios are not replayed.

## Compile-only observation —16November2026

One offline/locked engine `--lib --no-run` build returned0; no test body executed. Actual engine test=true/features=[],558effective workspace inputs matched947declared candidate bytes archived before Cargo and reread. Frozen temporary ELF9b1d5baae0f4f20a305527dcbce611681a243632f367db41cf603a4df2e43f1c, source commit86c29e91f28dc3df016b8bdcf4d7a4ef01be6b8a. Root receipt82046168701a3ca30654254617955bc5ad2b9cdcd77f9e8335dfede4aba73dae in `/run/media/fpizzi/SWARM/swarm-evidence/root00-sdk5c-build-3l38zm8u`. Scope is compile/borrow checking and workspace test-profile observations, not complete per-unit/compiler/registry/generated authentication, managed witness, W/F, production or installed version. Documentation does not rebuild or rename SDK source.


## Source checkpoint after storage recovery

Managed preparer SHA-256 6a357d1f4f31fb5dd70629c54eea31cfb53425e731cc59dcbdc9b0ddb29fd7ad: two GLM proposals and bounded principal corrections,09 APPROVE_SOURCE_ONLY (receipt 9be1ec21b08420aeab808b9bf8bc7750c6d0f8a7e4b7a68afd59a6e2304de29f). WIP source checkpoint only, no runtime/W/SDK/F or installation claim. Reserve recovered by verified archival of64historical proofs; CLI5f restored locally, full hash reread and787effective inputs unchanged. Separate managed trial allocation follows capacity/provenance admission; no historical campaign replay. Original failures preserved. Erratum: LF[10] correct in both proposals; no delimiter defect.


## Preparatory flow observation —16November2026

After verified local CLI5f restoration and comparison of787effective inputs plus7reviewed dependencies, source6a357d1f saved in91d08318adbeba3be8ddc888343082e4dbf80a95 executed one new managed preparatory flow. Root receipt `MANAGED-PREPARATION-VERIFIED-01.json` SHA-256 f8da5f816643a0a516c1bc022f503df3891708c139f2ce124a384351b828d644 in `/run/media/fpizzi/SWARM/swarm-evidence/root00-managed-preparer-cq1joyiy`:10assertions,123retained files, READY_MANAGED_CONTROLS_NOT_REVIEWED. One inert worker/report handoff, passing native control, read-only inspection without reviewer invocation, calls0. Actual sigint stop with children0/lease released/Root closed and owner reaped. No W/SDK/F, fault, acceptance, merge or installation established. Historical campaigns not replayed. Next W reuses this Work; GLM adapter being prepared, not executed. SDK source86c29 remains compile-only and is not relabelled by this documentation commit.


## Failed W and unexecuted correction checkpoint

W sourceaa009304 actually invoked the reviewer: exit2, review-reply-noncanonical-field, Workw-091168d1977007fbe4807a52 revision19,1of2calls consumed. Principal cause: ordinary PROGRAM/retainer against managed protocol; incomplete review context. SR1/certificate/marker/capture originals retained in managed-W-adapter-glm/FAILED-W-ORIGINALS; no reset/replacement Root. Retainer sourcee0158c6c adds optional managed_protocol for frozen snapshot/addressed context/cohort/I0 marker with explicit ordinary mode preserved. Proposed145-line recovery candidate e073a0fd received09 REQUEST_CHANGES(receiptf80dc8e4e95498d7797a215a31386c192667f885a651ca22e813d1cecee87d0c): ReviewCandidate acquisition/observe_submission, review_recovery and prepare_review_snapshot_using refuse managed recovery. Canonical planning review-retry decision/input acknowledgement also missing. Persistent new script is blocked SOURCE ONLY, not imported/executed; AST/protocol source review only, no second call or recovery PASS. Explicit header changes its hash relative to retained external candidate. Bounded principal supplement after332s frozen GLM stream, server consumption unknown; new managed-recovery-contract-glm packet targets smallest correct port, preserving authority/source/quotas and historical campaigns. Nothing merged/installed.


[Managed recovery plan](MANAGED-REVIEW-RECOVERY-20261116.md):09 audit ready for source curation/design, not patch. Managed admission P2 remains open; current Work unchanged, calls1of2. Full GLM plan read, no code adoption; bounded source/design work before any repair. Reserve recovered through158readback-verified historical proof copies on SWARM; active sources/data remain local.


### MRR-D02 prerequisite — source/helper verified, managed retry not ported

`append_recovery_context` in `review_preview/inputs.rs` shares ordinary and managed corrective context. Guidance remains outside quote materials/catalogue and precedes encoding and the full prompt cap. Source `9a8cebf74b4c6cfdb380fd45a3148152d78e096bf9267dbe56b4baafc658d93f`; independent09 APPROVE_SOURCE_ONLY (turn01a1e550-c91b-7451-8583-e2c2adf6cc83). Three new inert tests PASS (unchanged absence; complete directive with unchanged materials; nonobject refusal after a valid witness), Cargo `--lib` filter `root_store::review_preview::inputs::tests::recovery_`. Evidence and effective source inventory: `/run/media/fpizzi/SWARM/swarm-evidence/root00-managed-preparer-cq1joyiy/prompt-prerequisite-check-01/`. Library-test profile, stable sources; no production build, installation, retry admission or complete managed-flow verification.

Bounded principal supplementation addressed GLM reasoning latency without final patch, with no duplicate call or added cap. Its final answer after531.299s proposed the same shared context; fully read and compared. Its “exactly once” docstring would not establish idempotence and was not adopted. Exact declared usage67451tokens,59693completion; no price or Codex quota inferred. The reviewed/tested principal version remains unchanged after PASS. MRR-D01/D03, full-flow MRR-D02 and P2 stay open. Work19/calls1/2 unchanged. No historical replay, second W, service, provider or acceptance.


### 2026-11-16T22:17:46.225443+00:00 — owned-purpose MRR-D03 prerequisite: source reviewed, compile only

Two files integrated in CI checkout: `task_validation/review_candidate.rs` SHA256`2c4e9e72aaa63ee96f628431f6c218252609a161584b98f79618bbcb3a783918`, `managed_execution.rs` SHA256`e98ca58abb3c9f94f052930e9ac7c89edfe9baf82d980039617b579220fc5b56`. GLM returned a complete proposal after495.539s, declared usage122240tokens; no pricing or Codex quota inferred. Root reconstructed hunks against exact original blocks (one incorrect doc context corrected), denied implicit ReviewerRetry construction by legacy mapping, derived purpose before acquisition and explicitly refused legacy preview for a retry holder. Independent09 APPROVE_SOURCE_ONLY, turn01a1e66b-2350-73f2-9b48-fa159ded1571; primary `/run/media/fpizzi/SWARM/swarm-evidence/root00-managed-preparer-cq1joyiy/managed-review-purpose-glm/REVIEW09-PURPOSE-PRIMARY.json`.

Owned purpose/record project borrowed ReviewSource; explicit helpers preserve purpose across acquisition, preparation and refresh. Private acquire_for_retry/preview_for_retry hooks remain unwired to public operations/claim. Recovery remains observation data, never permission; Root original/canonical-decision verification and consuming rechecks remain required. No policy, count, authority, source digest or control/candidate binding change. New/modified functions documented; typed causes and paired rollback/close reviewed.

Cargo `--lib --no-run --offline --locked` exited0, default library-test profile without extra features, 558effective inputs archived/read back; sources stable before/after. Test ELF`738db0f581eb36080c77d8eccfa208c8cb0084720ebb428baedb6cd36791221e` retained as0400nonexecutable proof in `/run/media/fpizzi/SWARM/swarm-evidence/root00-managed-preparer-cq1joyiy/purpose-prerequisite-compile-01`. **Zero test bodies executed**, no functional PASS inferred. Earlier3helper tests retained/reused only for unchanged scope, not replayed. No installation/acceptance: incomplete preparatory packet and no current production image qualified for this contract. MRR-D01, behavioral MRR-D03, W/SDK/F and P2 stay open.

Recipe limitation: derived runner docstring retained “missing tests” wording although argv/count/summary explicitly state --no-run/0. Prose proves no executed check. Actual runner and archived copy remain unchanged; correct a future recipe’s documentation before use, no replay to replace this evidence.


## Root recovery core: source-only checkpoint

Shared ordinary/managed Root recovery core and exact assembled artifact map:09source approved, library --no-run compilation only,0tests. Public Fresh acquisition remains unwired; no new W/SDK/F, production, installation or acceptance. WIP; do not merge as a verified fix.


## Initial managed selector: WIP

Initial managed selector observes original/decision before custody and revalidates at consumer;09source-only review. No compile or behavioral proof for this selector yet. MRR-T01 ACK recipe, normal profile, witness/retry/close/debit1to2 remain open. WIP not eligible for merge as verified fix.


## Actual managed witness and dispatcher failure

W witness/retry recipe09source-approved. Real healthy witness PASS0to1 (15proofs/8CLIreaped0); missing correction refused without mutation. Original retry FAIL managed planning dispatcher before second reviewer; Work21/reviewer1of2/planner3of3, native call terminalRefused. MRR-D04 and original exhausted Work resumption authority remain open; no reinterpretation/refund. WIP, no merge/install/acceptance.


## MRR-D04 dispatcher: source-only checkpoint

MRR-D04 managed review-retry entry source-ported and09reviewed,one capability literal added,downstream guards unchanged. Not compiled/runtime-verified at checkpoint. OriginalWork21/planner3of3/reviewer1of2 and terminalRefused retained. Quota3to4/child2to3 preview-valid but not authorized/applied. WIP not mergeable as verified fix.
