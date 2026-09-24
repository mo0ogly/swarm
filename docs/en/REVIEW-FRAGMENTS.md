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
