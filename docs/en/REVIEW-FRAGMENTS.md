# Bounded review of large candidates

## Integration status

The engine now falls back to fragments when per-task batches exceed the transport limit. Public retry of an unpaid size refusal preflights complete evidence while preserving the candidate and attempt. Each inspection, evidence selection and final decision has a separate durable reservation. Finalization replays original replies; only the existing publication transaction accepts a result after rechecking evidence, criteria, provider and model.

Tests use a deterministic local provider: passing verdict, final refusal, unpaid preflight retry without another producer, and replay without duplicate calls, a second publication on the accepted baseline, and invalidation when that baseline’s final journal is corrupted. **Explicit recovery of interrupted inspections preserves the same journal, successful inspections and every spent call.** It checks the remaining budget and consumes a durable authorization under the review lock. Reconciliation alone does not retry an interrupted call. Final selection and decision follow the same rule: a new reservation needs explicit authorization tied to the interrupted call and sufficient remaining budget. An already durable passing verdict can be finalized without another call, even when the budget is exhausted. The read-only preflight retains `executable:false`: it is not an execution permit.

The following sections describe implementation stages, not proof of deployment or a successful real AI review.

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

## Independent selection before decision

A first final call requests necessary original artifacts by inventory index and digest. Its `ready` state only permits examining that selection; `unknown` preserves insufficient evidence and does not authorize the decision. Per-artifact JSON costs, references and remaining capacity are supplied. The engine measures the complete message and rejects duplicate, stale or oversized selections.

Preflight checks inspection instructions and schemas plus maximum selection and decision messages before the first charge. Table transport preserves all excerpts and opinions; only repeated JSON keys are removed. These functions do not yet run a provider or publish results.

### Response capacity before a call

New packets must also accommodate a complete response: the engine measures one `inspected` finding per artifact, including identities and 96-byte reason and evidence fields, and reserves 2 KiB for formatting within the 16 KiB reply limit. Splitting preserves every artifact and recalculates required calls. An oversized existing packet is rejected before a call; its journals remain readable and consumed calls remain charged. This check guarantees neither real analysis latency nor room for every possible request for additional evidence.

### Resuming an oversized existing packet

`planning retry-review` repartitions an existing plan that exceeds response capacity only when the review is in error, all reservations are interrupted, and no finding or final decision exists. The original plan and journal remain anchored under `replanned_from` and are checked on every read. Counters remain unchanged; the current budget must cover the new plan before it is stored. Only one repartition is supported; recovery with existing findings retains its plan. `planning fragment-preview` also examines this interrupted review without calls or mutations.

### Interrupted call diagnostics

Process errors include received bytes, parsed JSON event count, the last recognized event type, and whether a final event was observed. These counters prove neither useful progress nor acceptance. Events are parsed within the existing input limit; additional bytes are drained and counted. Counters contain no message content or identifiers. They distinguish a silent call from output without completion, without claiming a provider root cause. Deadlines, budgets and recovery rules remain unchanged.

An explicit retry reads the current reviewer deadline and records it on the resumed review. Evidence, reservations and consumed calls are preserved. Changing the deadline neither triggers a retry nor guarantees provider completion.

### Engine recovery preview

Run `swarm planning recovery-preview WORK --input request.json` with `{"task_id":"ID"}`, or GET `/api/v1/planning?work=WORK&action=recovery-preview&task=ID`. The engine verifies candidate evidence and the durable journal, counts reusable inspections, and reports remaining calls, available calls, missing calls and minimum review limit. Retry refusal uses the same budget calculation. Preview never calls a provider or changes limits.

Sufficient budget is not proof that the previous failure has been corrected. An existing final decision journal, an oversized historical plan or inconsistent evidence is explicitly refused. This preview covers interrupted inspections, not every recovery phase.

Diagnostics also distinguish assistant messages, system events and provider-declared `system/api_retry` events. Only predefined subtypes are retained; unknown values become `other`. A system event does not demonstrate ongoing analysis. Historical failures lacking these counters cannot be retrospectively reclassified.

## Cross-fragment context questions

A completed `unknown` inspection is durable evidence of uncertainty, not approval. It can be reused without charging for the same packet again while remaining packets are inspected. A demonstrated `fail` still stops the review. Final evidence selection receives every `needs` question, identified by packet, artifact and index, and selects required originals from the complete inventory.

When questions exist, the final response contains `review` and `resolutions`. Every question requires exactly one independent resolution with a rationale and, for `resolved`, a quotation from visible original evidence. Missing or duplicate resolutions and invented quotations invalidate the response. Any unresolved or failed question prevents acceptance even when all task criteria claim `pass`. Missing explicit questions, tampered evidence and oversized context remain blockers. Actual message sizes are rechecked without truncation.

Original `unknown` inspections are never rewritten. Publication reconstructs prompts and revalidates all resolutions. Historical reviews without questions retain their format. The engine checks identity, coverage and quotations; semantic relevance remains the independent reviewer's responsibility.

### Packet-bound provider contract

The provider schema fixes the exact finding count, candidate and packet identities, index range and allowed artifact hashes. Its actual size is included in preflight before reservation. The engine still independently verifies uniqueness, index/hash associations, coverage, quotations and byte limits after receipt. A stricter schema does not prove analysis quality or provider reliability; incomplete output remains rejected and the actual call remains charged.

### Early rejection and explicit correction

An inspection can stop the review when it demonstrates a defect. This is stored
as `changes_requested`, even when subsequent packets have not been read. It
approves no criterion and cannot authorize publication. Transport failures stay
`error`; their message is not evidence of a rejection.

For legacy reviews stored as `error` despite a durable rejection,
`planning revise-recovered-result` verifies the original journal, hashes,
candidate, attempt, evidence files and current configuration before permitting
an explicitly attributed correction. An altered journal or an interrupted call
is insufficient. The old opinion remains unchanged, the external repair is
identified as such, consumed budgets remain spent, and the new candidate must
pass checks and independent review again.

## Deadline and host suspend

Each planning or review call retains its authorized deadline. On Linux, the
runner also checks `CLOCK_BOOTTIME`, which includes suspend: after wake-up, an
expired call is interrupted instead of receiving its remaining active time.
The engine cannot act while the host is asleep. The reservation remains consumed;
interruption never constitutes acceptance. Previously recorded inspections remain
reusable subject to their existing integrity checks.

An injected-clock test simulates one hour of suspend and checks termination of
a live local provider despite a ten-minute active-time allowance. This test does
not constitute an independent review of the mission candidate.
