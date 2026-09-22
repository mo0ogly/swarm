# Independent review timeout

[Français](../REVIEW-TIMEOUT.md)

A completed worker and passing checks can remain blocked when the independent
reviewer times out. The engine preserves the candidate, report, check receipt
and consumed call. It does not invent a favorable review.

The historical default remains **90 seconds** until an explicit recorded decision
changes `planning.reviewer.timeout_seconds`. The supported range is 1–900 seconds.
Each new review records its own `timeout_seconds`.

Read the current mission revision, then submit a request such as:

```json
{
  "schema_version": 1,
  "event_id": "unique-review-timeout-decision",
  "expected_revision": 50,
  "review_timeout_seconds": 300,
  "reason": "Two reviews reached 90 seconds; explicitly allow five minutes without changing the call budget or candidate."
}
```

```sh
swarm planning show WORK
swarm planning review-timeout WORK --input timeout.json
```

The authenticated local API accepts the same request at
`POST /api/v1/planning?work=WORK&action=review-timeout`. This change adds no dedicated
web configuration button; the setting is available in its data.

An active review cannot be reconfigured. Stale revisions are rejected and replaying
the same decision is idempotent. Call budgets, worker attempts, verdicts, models,
checks and evidence remain unchanged.

**Changing the timeout does not retry the review.** A separate `planning retry-review`
request must identify the task, reason and current revision. It consumes another
call within the existing budget while reusing the production attempt and tested
candidate. An unfavorable opinion cannot be rerolled as a provider error. An
exhausted budget remains blocked.

More time does not prove that latency is resolved. Deterministic provider tests
cover timeout enforcement, retry on the same SHA, persistence and counters. Only
the actual independent review can provide an opinion for the live mission.

## Conductor health during review

When a completed managed-repository attempt is reconciled, checks and review run
outside the conductor loop. The conductor keeps updating its heartbeat and checking
other missions. The cross-process Git lock and transactional review claim prevent
another call for the same operation. An ongoing check is not an accepted result.

An `unknown` opinion means that evidence is insufficient. The resulting
`changes_requested` state remains unaccepted and cannot be retried as a provider
failure: complete the deliverable or its evidence within authorized limits, then
review the new candidate. More time cannot supply missing source context or test
receipts.

## Cumulative evidence and review batches

The engine first attempts a single call. When the losslessly encoded request
exceeds 192 KiB, it prepares deterministic task batches. Every batch retains the
same Git candidate, check receipt, full diff, and all cumulative reports and
criteria. Supplemental sources are assigned using their manifests; each source
remains whole. The cumulative limits of 24 files, 128 KiB of sources and 96 KiB
per file still apply. If one task cannot fit in a batch, the engine refuses before
any call, without truncating evidence.

The engine checks the budget for all remaining batches before the first call,
then reserves and counts each call separately. Each batch preserves its ID,
assigned tasks, context and digest, raw reply and digest, and state. An unknown
or unfavorable verdict prevents publication. Acceptance requires exact coverage
of all criteria and revalidation of the same candidate's evidence; passing one
batch does not accept the task.

An interruption requires the existing explicit `planning retry-review` action.
Only durably recorded favorable batches may be reused, after checking the
candidate, contract, provider, model policy, method and every evidence digest.
A paid call without a durable verdict stays consumed; its batch requires another
review within the remaining budget. If all favorable replies are already durable,
finishing their aggregation needs no additional call, even at the budget limit.
This recovery refunds no call and creates no new producer attempt.

These guarantees have isolated subprocess-provider tests in
`managed_review_batch_runtime_test.go`. Those deterministic tests do not establish
real model review quality or the completion of a running mission.

### Exact duplicates between the diff and source files

If even an individual batch exceeds the limit, sources may be transported as
segments. Portions already present in a diff hunk refer to its file and hunk
number; remaining portions stay literal. Each segment and the reconstructed file
carry a byte count and digest. The complete diff, deletions, reports, criteria
and receipts remain present. This is lossless reconstruction, not a summary:
concatenation yields the exact candidate file. Partial or ambiguous matches and
unsupported patches retain their literal source. A packet that still exceeds
the limit is refused before any call.

This transport is attempted only after the entire previous batching preflight
fails. Already valid plans keep exactly the same batches and prompts, preserving
paid reviews. Stored canonical context stays unchanged. Tests reconstruct all
bytes in a separate process and reject tampered content using the digests.

### Retry a refusal before the first call

`planning retry-review WORK --input request.json` also supports a completed result
held by the size preflight with no independent review recorded. The request uses
the existing `schema_version`, `event_id`, `expected_revision`, `task_id` and `reason`
fields. Keep the same `event_id` when replaying the same request.

Before rearming integration, the engine verifies the latest attempt, producer
termination, retained Git result, candidate and contract, receipts, all batch
sizes and the required budget. The original failure must be a size refusal;
failed checks and unfavorable reviews cannot use this path. If the precondition
is still unmet, no retry is recorded. The conductor then examines the same
result. No new producer, refunded calls or increased limits are introduced;
acceptance still requires the actual independent review.

## Retry integration after a failed control

`planning retry-integration WORK --input request.json` retries the **retained Git
result**, without another worker. Unlike `retry-review`, it reruns cumulative
controls before requesting a new independent review. The accepted base must have
changed and descend from the failed control's base. The producer must be completed
with its process confirmed stopped.

The JSON request requires `schema_version: 1`, `event_id`, `expected_revision`,
`task_id`, `agent_id`, `attempt_id`, `result_commit`, `expected_candidate`, and
`reason` (8–2000 characters). Obtain identities from public state. CLI and planning
API enforce the same contract.

Only one retry is allowed per result/base pair. Replaying the same event has no
additional effect. `integration_retries` retains the reservation, original failure
and provenance. Attempts, budgets and evidence are preserved. Existing review
packets cannot be overwritten through this operation. A base or contract change
after reservation stops integration.

For historical failures without a diagnostic, the engine requires the exact failure
event and an earlier publication establishing the base. `legacy_missing_output: true`
explicitly records the missing original output; no output or cause is invented.
Missing provenance prevents retry. A successful reservation is not acceptance:
new cumulative controls and independent review must pass before publication.

## Cumulative dossiers: accepted baseline and incremental review

Repeatedly sending the entire diff since mission start makes review input grow
with history, even for a small new change. Existing bounded review plans remain
unchanged. If even individual task batches overflow, the engine can use an
incremental review:

1. Verify passing reviews, attempts, contracts and policies for every accepted
   task on the published baseline. An accepted status alone is insufficient.
2. Supply those opinions explicitly as **evidence about the previous baseline**,
   retaining original contexts, replies, references and hashes.
3. Supply the full baseline-to-candidate diff, every current report and criterion,
   and all cumulative controls executed on the new candidate.
4. Reload declared sources from the new candidate, including files no longer
   present in the incremental diff.
5. Obtain NEW independent opinions on every criterion and possible regressions.
   An old opinion never accepts the new candidate.

`accepted_baseline` identifies this protocol in the stored context and reviewer
instructions. Historical evidence hashes are checked recursively during review
and before publication. Missing or changed evidence invalidates the chain. Previous
review packets and already admissible batch plans retain their original transport.

This replaces a fresh inspection of the entire history with independent inspection
of changes and their effects against a verified baseline. The reviewer must return
`unknown` if current sources and baseline opinions are insufficient. The 192 KiB
per-call limit, budgets and same-candidate independent verdict requirement remain.
A single change or source set that still exceeds the limit is refused before any
call; this never authorizes truncation or partial acceptance.

### Concurrent writes during recovery

An additional-attempt authorization and a launch reservation acquire SQLite's
write lock before reading their guards. A short concurrent write, such as an agent
heartbeat, is awaited within the existing SQLite timeout instead of requiring a
human to repeat the action. Provider preflight remains outside the transaction.
Previews and rejected launches roll back reservations; replaying the same request
does not create duplicate agents. Stale revisions, exhausted limits and persistent
storage failures remain errors; this is not an unlimited retry policy.

### Orphaned review after server shutdown

A retained conflict may still show a running review after its controller stops.
The conductor checks the interprocess lock held throughout review execution. It
leaves a held lock alone; with a free lock, it rechecks the attempt identity and
records interruption. Candidate, receipts and consumed calls remain unchanged.
The public `planning retry-review` operation then becomes available without new
production. Detection alone never reserves another provider call.

### Ownership of a live review

The integration lock also protects ownership of an in-flight review, not only Git commands. Releasing it during inference lets another conductor misclassify the live review as orphaned and persist a competing result. Until a separate durable ownership mechanism replaces that guarantee, retain the lock through verdict persistence. Concurrency optimizations must prove survival of a second conductor, crash recovery and paid-call uniqueness.

`TestManagedLiveReviewSurvivesConcurrentConductor` blocks a real test subprocess while a second Store attempts reconciliation, then requires a single passing verdict and idempotent publication. It uses a simulated provider and does not establish the quality of real AI reviews.
