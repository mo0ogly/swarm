# Recovering prepared launches and incomplete deliveries

[Français](../ENGINE-RECOVERY.md) · [User guide](USER-GUIDE.md)

## Prepared launch recovery

A prepared working copy is not yet a recorded agent. For new managed launches,
Swarm saves the original operation identity, task, Git base, provider, instructions
and launch settings. A failed agent transaction consumes no attempt and retains
the copy. Recovery survives a server restart and can find the filesystem checkpoint
if interruption occurred before its SQLite record was committed.

SQLite BUSY/LOCKED errors allow up to three registration attempts with the same
identity. These attempts do not spawn a provider. If preparation remains pending,
the conductor stops repeating the launch and exposes an explicit recovery action.

Open the task in **Agent control**, then choose **Resume the prepared launch**.
The modal shows retained settings, help and a confirmation. Cancel and Escape
leave the task unchanged. The interactive terminal offers the same action.

```sh
swarm --root /path/to/project agent prepared WORK_ID
swarm --root /path/to/project agent resume-launch WORK_ID --input resume.json
```

```json
{
  "schema_version": 1,
  "prepared_id": "identity_returned_by_agent_prepared",
  "expected_revision": 42
}
```

Use the revision returned by `agent prepared`. Replaying a confirmation retrieves
the original agent. If its supervisor has not claimed the launch, recovery may
request supervision again; an atomic claim prevents duplicate provider starts.

Changed task instructions, criteria, task budgets or Git base prevent recovery.
Provider configuration, dependencies, budgets, organization and availability are
checked again. Unattributed, redirected or inconsistent copies are never overwritten.
Legacy preparations without saved parameters require the identified original
request; Swarm cannot reconstruct missing settings safely.

## Delivery completeness before independent review

New automated workers in a managed repository receive a template for
`docs/TASK_ID.delivery.json`, alongside their Markdown report. The file must be
included in the submitted Git revision. In a repository subdirectory, `docs/` is
relative to the project directory; evidence paths are relative to the Git root.

```json
{
  "version": 1,
  "task": "example-task",
  "attempt": "identity_supplied_by_the_engine",
  "contract": "digest_supplied_by_the_engine",
  "outcome": "complete",
  "criteria": [
    {
      "index": 1,
      "status": "pass",
      "reason": "Observed behavior and verification limits",
      "controls": ["control_authorized_for_this_criterion"],
      "evidence": ["tests/evidence_test.go", "docs/example-task.md"]
    }
  ]
}
```

Every criterion requires exactly one entry. Task, attempt and contract must match.
Control IDs must be authorized for that criterion. Evidence must exist as regular
files in the examined revision; symlinks and paths escaping the repository are
rejected. The manifest is limited to 32 KiB without truncation.

Use `not_tested` for an untested obligation, `fail` for a failed check, and `partial`
or `blocked` as the overall outcome while any obligation is unproven.
`not_applicable` requires a reason and remains incomplete pending examination;
it cannot remove a requirement.

An absent, invalid, partial or misattributed manifest produces **Result to complete**.
The copy and report remain available, the published candidate stays unchanged,
and no reviewer call is charged. The responsible planner receives an integration
failure event with the reason. Corrections remain within authorized attempt limits.

The task inspector's reports section opens the retained report, including after
a rejected review. The interactive CLI exposes the same report through its read
action. For internal files, the web reader requires attribution to the selected
work and task; it cannot read arbitrary engine files. A reviewed report changed
since recording is not presented as current evidence.

The AI summary also receives the current review state and consumed attempts.
These engine facts take precedence over historical statements in the report.
Its schema requires two short lines; a nonconforming answer remains rejected
without hiding the report or granting acceptance.

A complete manifest allows authorized checks and independent review to proceed.
It never accepts a task. The reviewer receives the candidate, engine receipts,
report, available sources and declaration. Review instructions distinguish a
proven defect from missing evidence or an ambiguous contract. An audit can succeed
with evidence about existing code; absence of production edits is not itself a defect.

## Limits and compatibility

- Structural completeness does not prove semantic coverage. A criterion containing
  several obligations may still be inadequately tested. Planning and review quality
  remain necessary.
- Review guidance does not guarantee a model's judgment. Deterministic tests verify
  the protocol, not real-world AI review quality.
- The manifest requirement applies to new automated managed workers. Historical
  attempts and interactive modes are not retroactively made noncompliant. Any
  supplied manifest is still checked.
- No budget is refunded, no ceiling raised, and no historical review changed.
  An exhausted mission remains blocked.

## Design and verification

Explicit recovery keeps the original identity. Automatically adopting a copy under
a different request would mix ownership, instructions and files. A structured
manifest was chosen over keyword searches in free-form reports; independent review
still assesses the substance.

Reference tests: `managed_preparation_test.go`, `managed_delivery_test.go`,
`result_presentation_test.go`, `tests/managed_recovery_ui.cjs`. The browser recipe
actually spawns a deterministic fixture provider in a temporary root. It is not
a demonstration of a successful autonomous mission using a real AI provider.

```sh
go test ./...
go vet ./...
go test -race -run '^TestManaged(PreparedLaunch|Delivery|CompleteDelivery)' .
npm test
# Build first; Puppeteer and Chrome must be available.
SWARM_RECOVERY_UI_BINARY=/absolute/path/swarm \
SWARM_RECOVERY_UI_OUT=/absolute/path/recipe \
go test -run '^TestManagedRecoveryBrowserRecipe$' -count=1 -v .
```

### Persisted approval with interrupted publication

An explicitly submitted external repair may pass review after an earlier context
size refusal, then encounter a storage error during publication. Replaying the
exact same repair request can finish publication using the saved approval, after
rechecking the candidate, contract, context and receipts. It creates no new model
call or producer attempt. Rejected reviews, altered evidence and different
attempts cannot use this route. A favorable review alone is not acceptance;
verify the public task state after the operation.

The final transaction reserves SQLite's writer before rechecking its evidence.
This prevents another writer from invalidating a deferred read-to-write upgrade.
The reservation changes no revision and rolls back on any failed guard. Test
commands and model review still run outside this transaction. A two-connection
test reproduces SQLITE_BUSY before the fix and verifies one publication after it.

### Retained refusals and conductor polling

An integration in `conflict` retains its diagnosis and evidence. The conductor
does not retry it on every poll: doing so would repeatedly acquire the Git lock
shared with new launches. Already integrated results are also skipped. Public
explicit recovery operations remain available with the same identity, evidence
and budget checks. An explicitly rearmed repair in `integrating` state is still
resumed by the conductor.

### Failed Git control diagnostics

A failed cumulative control now reports a local artifact under
`.swarm/managed/<mission>/diagnostics/control-failure-*.json`. It records the
command, exit code, identities, tested candidate and exact captured output bytes
(`output_base64`, JSON base64), with their digest. Capture remains bounded to
64 KiB; `output_bytes` records the total and `truncated` explicitly marks a
partial capture. Project information may appear in this local diagnostic; it is
not automatically sent to the reviewer. A private Git reference
`refs/swarm/failed-controls/<SHA>` retains the rejected candidate without replacing
the accepted one. These artifacts authorize neither acceptance nor automatic
retry. Failure to preserve them is an explicit refusal.

### Exploration before producing a deliverable

The engine injects bounded shell examples into each new execution prompt: check
whether an optional report exists before `cat`, pass quoted file patterns to
`rg --files`, and distinguish empty-search exit code 1 from error exit code 2.
Validation checks keep their real exit codes; blanket `|| true` masking is not
recommended. These instructions help the agent but do not guarantee compliance.
The engine does not turn failed commands into successes and preserves all limits.
Recovery after interruption requires an explicit correction and an available attempt.

### Grounded quotations in review artifacts

For new fragment inspections, the response schema offers up to two exact short
excerpts from each artifact. The schema constrains their numeric indexes, avoiding
provider restrictions on quotation marks in string enum values. The reviewer selects a location anchor and retains
its judgment: inspected, demonstrated defect or missing evidence. An anchor
locates content; it does not prove correctness. Full content is still supplied,
and the final decision remains a separate step.

Every finding is bound to a required inventory key and the whole packet digest.
Historical replies remain readable under their original checks; rejected replies
are never promoted into evidence. Without a usable excerpt, inspected is not
allowed. Content and schema size are checked before reserving a call. Recovery
preserves completed inspections and consumed calls. Compact transport keys
`v`, `r`, `e`, `n` mean verdict, reason, evidence and required evidence; user-facing
labels remain unchanged.

New dossier partitioning counts content, schema and anchor table separately,
then reserves workflow space. Every actual request is checked before spending.
Historical packets remain readable; new partitioning rules do not rewrite journals.

### Explain a fragment review refusal

`planning diagnose-review WORK --input request.json` spends **one review call
within the existing limit**, preserving the refusal. Supply `schema_version: 1`,
`event_id`, `expected_revision`, `task_id`, `review_id`, `reason`, and optionally
`input_events` with up to eight source paths from the immutable candidate.
A durable fragment refusal is required.

The same reviewer explains each finding as confirmed with an exact quotation,
requiring context, or not reproduced, with an explanation and reproduction steps.
The private artifact and digest are recorded by `review.diagnostic.result`.
This diagnostic never replaces the verdict, accepts a task or restarts production.
Replaying the same event spends no extra call; interruption keeps the reservation
consumed and never triggers an automatic retry.

### Sources across tasks

Each task may declare up to 24 files and 128 KiB of context sources. Cumulative
review retains the complete deduplicated union: two tasks with thirteen files
each are not rejected merely because their union exceeds 24. Per-task limits,
fragment request sizes and call budgets are still enforced. No file is omitted.

## Explicit correction after a reviewer error
`planning revise-recovered-result` accepts `confirm_review_error_repair: true`
in addition to `confirm_recovery: true`, the current revision and review_id,
agent/attempt identities, changed Git tree and operator reason.
This authorizes a new submission after a terminal `error` with intact evidence
and configuration and no reserved call. It does not turn the error into a
proven refusal or approval. A changed tree, checks, fresh review and existing
budgets remain mandatory. Without this flag the previous refusal rule applies.

### Restarting a blocked task

`planning restart-task WORK --input request.json` (also available as the HTTP
`restart-task` action) prepares one fresh production after an explicit operator
decision. It does not refund costs or erase history. Required fields are
`schema_version: 1`, `event_id`, `expected_revision`, `task_id`, `attempt_id`
(the latest attempt), `expected_candidate` (the cumulative starting SHA),
`confirm_recovery: true`, `reason` and a new `recovery_instruction`.

The task must be blocked with its attempt allowance exhausted, have a launch
profile and a configured independent reviewer. No mission agent or independent
review may be active. The engine grants exactly one additional attempt, sets
the task to todo and retains old attempts, reviews and evidence. The next
managed launch creates a fresh numbered copy from the cumulative candidate,
not the rejected copy. This decision does not itself launch a provider.
Dependencies, review budgets and launch checks still apply. Earlier tasks are
not implicitly revalidated. Exact event replay is idempotent; a second grant
before consumption is rejected.
