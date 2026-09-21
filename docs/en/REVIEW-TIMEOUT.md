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
