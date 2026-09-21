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

## What a worker receives when retrying

The engine automatically prepares `.git/swarm-recovery.json` in the **new isolated
copy**. The worker prompt provides its location and SHA-256. CLI, web and conductor
launches use the same preparation. When the engine has recorded an immutable Git
result, an operator no longer needs to transfer its patch manually.

The packet binds the work, task, previous agent and attempt to the planner's
correction, failure reason and, when available, the full independent review and
its receipt. The receipt is read again and checked against its recorded digest.
The attributed Git result is fetched into the new copy under
`refs/swarm/recovery/previous`. The worker can inspect its report and changes with
`git show` and `git diff`, using the base and result commits in the JSON, without
reading or modifying the previous worker's files.

HEAD still starts at the current accepted candidate. The worker examines, reuses
and corrects relevant changes in its own copy and resolves any conflicts. The
rejected result is neither published nor applied automatically. The packet stays
in Git metadata, outside deliverable files. New checks, a delivery manifest and
independent review remain required against the new candidate.

If no immutable Git result was recorded, the packet explicitly says so; unsubmitted
working files are not imported. Inconsistent attribution, a changed receipt or a
modified prepared packet prevents launch and preserves the copies. Packets are
limited to 512 KiB without truncation. Handoff preparation makes no AI call and
grants no additional attempt. It supports an authorized retry; it does not replace
the operator's decision when the attempt limit has been reached.

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

## Recovering an interrupted result without rewriting history

An execution limit takes precedence over earlier incidental tool errors. The engine
sends the owner a JSON interruption record under `.swarm/interruptions/`, including
the attempt identity, workspace, stop reason, counters and last observed action.
It is an engine observation, not a worker report or acceptance evidence, and does
not snapshot workspace files. Storage failures remain explicit.

Worker instructions identify one code workspace and require an early, regularly
updated report. These instructions are guidance, not an OS sandbox or a guarantee
that a provider will follow them.

For explicitly requested external repairs, an operator may finish the stopped
copy and submit one examined revision through `planning submit-recovered-result`.
The operation launches no worker and preserves the interrupted process, attempt
and budgets. The reviewer receipt identifies `external_repair` and its actor;
such a repair does not demonstrate autonomous completion.

After reviewing the diff and evidence, run `git add -A` and `git write-tree` in the
copy. Supply JSON with `schema_version: 1`, `event_id`, `expected_revision`,
`task_id`, `agent_id`, `attempt_id`, `confirm_recovery: true`, `result_tree` and
a meaningful `reason`. Changed files, active or replaced attempts, missing reports
and incomplete delivery declarations are rejected. `docs/TASK.delivery.json` is
mandatory even for legacy attempts.

Normal checks and independent review still cover the same candidate. A rejection
stays blocked; no fifth attempt is created. Replaying the same event uses the
recorded revision and does not repeat a rejected review. A second repair submission
for the task is refused. Review incidents use the existing explicit retry path.

Oversized review context can come from unchanged diff lines. Above 160 KiB of
serialized context, the engine uses three Git context lines instead of forty.
Every changed line, complete source file and receipt is retained. The final
192 KiB limit remains mandatory. Explicit replay of the same external submission
can resume this preflight failure only if no review of that attempt has started;
an existing review never triggers an implicit additional call.

### Correct a rejected result without restarting its producer

`planning revise-recovered-result` submits an explicitly examined external
correction tied to the latest `review_id` in `changes_requested`. Supply the
same fields as `submit-recovered-result`, plus that ID and the current revision.
The tree must differ from the rejected result and match the examined files.
The engine preserves the previous opinion, receipt and Git references. The new
candidate undergoes checks and review within the existing budget. No production
attempt or counter is reset. Replaying the same request does not consume another
review. This operation cannot replace an accepted task. External repair remains
recorded as an intervention, not autonomous production.

### Independent opinion format errors

A `pass` citation must be one exact contiguous excerpt from the supplied evidence.
Paraphrases and concatenated excerpts are rejected. The engine now preserves the
reviewer response, including rejected responses, in a private file bound to the
opinion (`reply_path`, `reply_sha256`) and identifies the offending task/criterion.
This file is not approval. Older opinions may have no preserved raw response.

After a demonstrated correction to the format or precondition, `planning
retry-review` allows an explicit retry within the existing budget. There is no
automatic paid retry. Receipts, contexts and reports are reread during inference
and before publication; changed evidence makes the opinion stale.

After `retry-review`, the conductor also resumes an external repair whose
producer remains interrupted. Only the explicitly submitted result awaiting
integration is resumed. Historical process state and attempts stay unchanged.
A rejected submission without an explicit retry triggers no new opinion.
