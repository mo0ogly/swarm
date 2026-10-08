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

Include exit codes and unexpected diagnostics. Distinguish fixtures from real
providers. List relevant negative/recovery checks. A claimed test without an
execution record is not execution evidence.

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
