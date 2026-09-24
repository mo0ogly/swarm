# Bounded review of large candidates

Status: **complete-evidence Go preflight available through CLI/API; the runtime protocol is not implemented**. This document does not authorize acceptance or modify the stored mission.

Existing task batches repeat the entire cumulative diff. They cannot accommodate a diff that already exceeds the 192 KiB prompt limit. Removing code from an already tested candidate instead requires fresh, matching evidence and an explicit revised delivery.

## Read-only check

Run `python3 tools/review/fragment_preflight.py --git-dir REPOSITORY.git --base BASE_SHA --candidate CANDIDATE_SHA --calls-available N --reserve-final-calls M --output NEW_DIRECTORY`.

The tool resolves immutable commits, checks ancestry and reads the diff without external filters. It groups complete file sections, preserving exact UTF-8 bytes and order. Packets identify the candidate, baseline, diff and inventory hashes, assigned sections and their full contents. Reconstructing all packets must reproduce the original diff exactly.

Each JSON packet reserves 24 KiB within the 192 KiB limit for instructions and response schema. Binary changes, an oversized indivisible file, altered packets, incomplete coverage and insufficient budget are rejected. The output directory must be new. No provider or Store is called.

The estimate covers **only the diff**. Reports, supplemental sources, controls, contracts and final review must still fit. Reserved final calls do not prove that their inputs fit or that their verdict will pass.

## Requirements before runtime activation

- Preflight every inspection and final call, including full evidence and total budget, before spending.
- Bind the plan to candidate, baseline, attempt, contract, policy, receipts, workflow, provider and model.
- Persist original packets, replies, hashes and call reservations. Resume only intact completed replies from the same plan; never refund ambiguous calls.
- Fragment inspection is not task acceptance. Cross-file issues require explicit independent examination.
- Final review uses original evidence for every criterion; summaries may guide navigation but cannot replace evidence. Missing evidence yields `unknown`.
- Publication requires complete coverage, favorable independent final verdicts and fresh controls on the same candidate, followed by identity checks in the publication transaction.
- Preserve existing review formats, hashes and resume behavior.

Tests must cover missing/duplicate/altered fragments, stale identities, insufficient budget, interruption and restart without double spending, incorrect reply attribution, cross-fragment defects and refusal of partial acceptance.

E1–E5 retain their history. E6 retains its identity, attempts and candidate. E7–E8 retain their dependencies. No task is accepted by this feasibility tool.

## Engine preview of the retained candidate

Use `swarm planning fragment-preview WORK --input request.json` with `{"task_id":"TASK"}`, or authenticated GET `/api/v1/planning?work=WORK&task=TASK&action=fragment-preview`.

The engine verifies the stopped attempt, recovered result, retained candidate, contract and receipts. Packets preserve the complete diff, supplemental sources, reports, deliveries, contracts, controls and accepted baseline. The remaining call budget is read from the mission; two final calls are reserved for a future final-review stage.

The result always has `executable:false`. It neither reserves a call nor invokes a provider. Existing retry remains blocked until durable execution and final-review verification are implemented. The preflight does not prove that the future final-review inputs will fit.

## Transactional journal (not enabled)

The Store anchors the plan and each journal version by digest. Reserving a fragment charges one call in the same transaction that records its reservation; competing database connections cannot reserve the same state. Budget refusals and paused missions preserve the previous counters and journal.

After reopening the Store, a reply can be recorded without another charge only for its reservation and the same candidate, attempt, method, provider and model configuration. Earlier replies remain immutable. Successful inspection does not permit publication: the independent final decision and execution integration are still outstanding.

## Final decision input (construction only)

The final decision builder preserves task contracts, reports and checks, separates original excerpts from inspection opinions, and can include whole artifacts requested by index and digest. Unknown references, duplicates, stale content and capacity overflow are rejected. Requested files are never truncated.

The parser receives only original evidence actually included in that call: a quotation present in the complete diff but absent from the final message is rejected. The builder makes no provider call and does not remove the publication guard. Final call reservation, execution, recovery and durable anchoring are still required before activation.

## Final capacity before spending

Each inspection rationale and excerpt is limited to 96 serialized JSON bytes excluding surrounding quotes, counting escapes and UTF-8. The total reply cap remains 16 KiB. Oversized replies are rejected, never shortened. This allows a conservative final-input bound using the actual identities, names, contracts and instructions before any call.

Remaining capacity applies to the complete supplementary evidence JSON, not just its text. It does not guarantee a future evidence request will fit. The engine must check that exact request and reject overflow without truncation; a size check proves neither evidence sufficiency nor a favorable verdict. This calculation is not yet connected to execution.
