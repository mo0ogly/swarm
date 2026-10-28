# Task result — <task ID>

Use the assigned report in your owned workspace. Fill placeholders with facts;
put critical limits first. Never edit another worker's report or shared coordination.

## Outcome in two sentences

<What changed and its useful effect. What remains failed, untested or blocked.>

## Identity and scope

- Work / task / attempt / producer: <supplied identifiers, or explicitly unknown>
- Role and assigned scope: <scope>
- Base / candidate revision and dirty changes: <observed references>
- State: <completed implementation / partial / blocked; not engine acceptance>

## Findings the responsible planner must know

<Discoveries, deviations, concerns, affected tasks/scopes and proposed next action.
Route cross-scope issues to the responsible planner; do not contact other workers.>

## Changes and verification

| Requirement | Change / file | Exact check and environment | Observed effect | Result | Evidence |
| --- | --- | --- | --- | --- | --- |
| <ID> | <path> | <command/options or interaction> | <assertion> | <PASS/FAIL/PARTIAL/NOT TESTED/N/A with reason> | <artifact> |

Declare whole original text in `delivery.criteria[].evidence` (tested candidate,
including unchanged files), with extra sources in `docs/<task ID>.review-context.json`.
Quotes do not replace originals. Recovery names the missing input, declaration
and resolving check; inspect the supplied inventory. Receipt is not understanding.

Record exit codes, unexpected diagnostics and negative/recovery checks.
Distinguish fixtures from real providers; test claims need execution records.

| Execution | Executor / source revision | Exact arguments | Start / end | Exit | Original stdout / stderr and SHA256 | Retention limit |
| --- | --- | --- | --- | --- | --- | --- |
| <ID> | <worker / operator / controller; candidate and dirty inputs> | <argv> | <dates> | <code> | <paths and digests> | <complete / explicit bound> |

Attribute local runs, controller receipts and independent verdicts separately.
Parallel agents retain owned artifacts; route gaps to owners and preserve history.

## APEX / PDCA checkpoint

- Analysis / PLAN: <criterion and hypothesis addressed>
- Execution / DO: <bounded action actually performed>
- Verification / CHECK: <observable result and evidence above>
- Adjustment / ACT: <correction performed or proposal to the responsible planner>
- Recovery limits: <attempts/budget or quota relevant to the next action; unknown if unavailable>

## Next action and limits

<Who should act, on what evidence, and what precondition must change first.>
<Independent review and its evidence. The supervisor decides completion under
required checks; the engine validates admissibility. Saving is not receipt.>
