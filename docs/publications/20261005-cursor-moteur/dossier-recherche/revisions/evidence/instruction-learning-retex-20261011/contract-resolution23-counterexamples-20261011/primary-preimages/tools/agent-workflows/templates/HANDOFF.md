# Task result — <task ID>

Use the report path assigned by Swarm, normally `docs/<task ID>.md` in the worker's
own workspace. Replace placeholders with observed facts. Do not write a shared
coordination file or edit another worker's report. Keep the opening summary useful
even when a reader sees only a bounded excerpt. Never hide a critical limit at the end.

## Outcome in two sentences

<What changed and its useful effect. What remains failed, untested or blocked.>

## Identity and scope

- Work / task / attempt / producer: <supplied identifiers, or explicitly unknown>
- Role and assigned scope: <scope>
- Base / candidate revision and dirty changes: <observed references>
- State: <completed implementation / partial / blocked; not engine acceptance>

## Findings the responsible planner must know

<Important discoveries, deviations, concerns and feedback, including effects on
other tasks. Name the affected task/scope when known and propose a next action.
Do not modify or contact another worker to resolve the issue outside your scope.>

## Changes and verification

| Requirement | Change / file | Exact check and environment | Observed effect | Result | Evidence |
| --- | --- | --- | --- | --- | --- |
| <ID> | <path> | <command/options or interaction> | <assertion> | <PASS/FAIL/PARTIAL/NOT TESTED/N/A with reason> | <artifact> |

For managed delivery, list original text files required for review in
`delivery.criteria[].evidence`; they are attached from the tested Git candidate,
even unchanged. Use `docs/<task ID>.review-context.json` for extra sources.
A report quoting another file does not replace that original. For a recovery,
name the missing input, the new declaration and the verification that resolves
it. Check the supplied source inventory; receipt is not understanding.

Include exit codes and unexpected diagnostics. Distinguish fixtures from real
providers. List relevant negative/recovery checks. A claimed test without an
execution record is not execution evidence.

| Execution | Executor / source revision | Exact arguments | Start / end | Exit | Original stdout / stderr and SHA256 | Retention limit |
| --- | --- | --- | --- | --- | --- | --- |
| <ID> | <worker / operator / controller; candidate and dirty inputs> | <argv> | <dates> | <code> | <paths and digests> | <complete / explicit bound> |

Keep local executions, controller receipts and independent verdicts attributable
to their actual authors. For simultaneous agents, use each task's owned artifacts;
route gaps to its owner and preserve historical records after correction.

## APEX / PDCA checkpoint

- Analysis / PLAN: <criterion and hypothesis addressed>
- Execution / DO: <bounded action actually performed>
- Verification / CHECK: <observable result and evidence above>
- Adjustment / ACT: <correction performed or proposal to the responsible planner>
- Recovery limits: <attempts/budget or quota relevant to the next action; unknown if unavailable>

## Next action and limits

<Who should act, on what evidence, and what precondition must change first.>
<Independent review pending/performed with evidence; current acceptance is owned
by the engine. Reading this report is not proof that a recipient acknowledged it.>
