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
