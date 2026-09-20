# Recover an exhausted task

In **Agent management**, a blocked card and its details offer **Authorize an
additional attempt** when the current allowance is exhausted. Enter a reason and
a concrete correction: missing sources, a verified precondition or partial work
to reuse. Repeating the same failed action does not resolve its cause.

Confirmation adds exactly one attempt, with a ceiling of three. It preserves
attempt history, reports, checks, independent opinions and tool budgets. The task
remains blocked and unaccepted. The launch form opens next and shows any remaining
conditions. Authorization itself never starts a provider. Escape or Close cancels
without changing the work. A stale form is rejected; close and reopen it.

The engine refuses this operation while an agent or review is running, while an
attempt remains available, or once three have been authorized. After three attempts, use the separate exceptional flow below. There is no
automatic increase in the attempt allowance.

## CLI equivalent

Read the current revision using `swarm --json work show WORK`, then create:

```json
{
  "schema_version": 1,
  "event_id": "unique-operator-decision",
  "expected_revision": 42,
  "task_id": "my-task",
  "reason": "Explicit decision after examining the failure",
  "recovery_instruction": "Supply missing candidate sources and rerun the same evidence checks."
}
```

```sh
swarm planning extend-attempt WORK --input recovery.json
```

This operator allowance does not modify the hierarchical requirements, dependencies
or owner. Replaying an `event_id` cannot grant another attempt. A stale revision is
rejected. The `task.attempt-extension` event records the reason and instructions.

## Supply candidate sources to the independent reviewer

For managed Git integration, the producer may add
`docs/TASK-ID.review-context.json` inside the mission's repository subdirectory:

```json
{"version":1,"files":["managed_review.go","managed_review_test.go","tests/engine_acceptance.cjs"]}
```

File paths are relative to the repository root. The engine reads complete files
from the tested candidate commit, never a mutable checkout. Each source includes
its contents, path, Git blob and SHA-256 digest. Selecting files does not guarantee
sufficient evidence; a reviewer must still return unknown for unsupported criteria.

Limits: 8 KiB manifest; 24 distinct files; 96 KiB per file; 128 KiB combined sources;
192 KiB overall review context. Invalid paths, missing files, symlinks, binary
content and oversized input are rejected without truncation or a review call.
Without a manifest, the existing diff/report/control-receipt flow remains available.
This is explicit context delivery, not general file watching or direct worker messaging.

## After three attempts: one explicit corrective authorization

**Prepare one corrective attempt** is available on the task card, recovery
diagnosis and task inspector. The editable proposal repeats the latest independent
rejection. Reading or cancelling it changes nothing and calls no AI. The proposal
does not establish that the review diagnosis is complete or accurate.

**Authorize this corrective attempt** grants exactly one fourth attempt. The
active conductor handles it when launch conditions permit. Pauses, dependencies,
workspace reservations, storage checks, budgets and reviewer availability still
apply. Acceptance requires checks and a new independent review of the corrected
candidate. History, criteria and previous verdicts remain intact.

The engine requires a blocked task with three used attempts, an independent
rejection of the latest attempt, an available reviewer budget, a new instruction,
a reason and explicit confirmation. Active task agents prevent authorization.
The atomic operation is idempotent; double clicks cannot add two attempts, and
stale forms are rejected. The `task.corrective-recovery` event and the task's
`corrective_recovery` record retain the actor, timestamp, revision, review,
attempt, reason and instruction. A fifth attempt cannot be granted through this
flow, including after task edits. Planners and page-assistant proposals cannot
authorize it; the local operator must do so. The local API/CLI remains within the
operator's trust boundary.

```json
{
  "schema_version": 1,
  "event_id": "unique-corrective-recovery",
  "expected_revision": 42,
  "task_id": "my-task",
  "review_id": "latest-review-id",
  "attempt_id": "latest-attempt-id",
  "confirm_recovery": true,
  "reason": "Targeted correction after reading the independent rejection",
  "recovery_instruction": "Correct the missing evidence identified in this review and rerun every check against the corrected candidate."
}
```

```sh
swarm planning authorize-recovery WORK --input corrective-recovery.json
```

Review this JSON before sending it: an active conductor may start the authorized
attempt. Tool, cost and reviewer budgets remain unchanged. Other tasks retain
their own allowances. Environment failures still require verified preconditions.
