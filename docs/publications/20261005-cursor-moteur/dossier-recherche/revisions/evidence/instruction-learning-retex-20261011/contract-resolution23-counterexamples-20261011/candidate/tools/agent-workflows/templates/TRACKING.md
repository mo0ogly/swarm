# Local action checkpoint — <task / attempt>

Use in the owning task's report, never for cross-agent notifications. Update current
state; retain significant decisions/evidence in the handoff, without raw transcripts.
Tool-free roles return their required JSON instead of writing files.

## Current state

- Requirement and expected observable: <ID and effect>
- Known state / hypothesis: <separate fact from assumption>
- Current action / next action: <one bounded action each>
- Blocker and changed precondition needed: <cause, owner, evidence>
- Remaining limits: <observed attempts/budget or unknown>
- Linked problem / parent: <original requirement or failing diagnostic/control>
- Iteration progress: <new discriminating fact or changed precondition; no unchanged retry>
- Reassessment: <original requirement now proved, still blocked or requiring another bounded check>

## Significant actions and decisions

| ID | Time / revision | APEX phase / PDCA phase | Action or alternatives | Observed result / evidence | Decision / next owner |
| --- | --- | --- | --- | --- | --- |
| <stable ID> | <observed> | <Analyze/Plan/Execute/Verify; PLAN/DO/CHECK/ACT> | <bounded> | <result> | <decision> |

## Interactions

| Source / attempt | Event or report reference and digest | Intended recipient | State proven by the engine | Required response |
| --- | --- | --- | --- | --- |
| <identity> | <public reference> | <owning scope> | <pending/delivered/decision recorded/unknown> | <action> |

Saving, event emission, receipt and decision are separate observations. ACK proves
the recorded operation, not understanding. Use public statuses; never edit the database.
