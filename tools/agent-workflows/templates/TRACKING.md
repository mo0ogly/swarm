# Local action checkpoint — <task / attempt>

Use this compact section in your assigned working report when useful. It is
private working memory for the owning task, not a cross-agent notification channel.
Rewrite the current-state section as evidence changes; preserve significant
decisions and their evidence in the final handoff. Do not accumulate raw transcripts.
Tool-free planners/reviewers report these facts in their required JSON response
instead of attempting to write a file.

## Current state

- Requirement and expected observable: <ID and effect>
- Known state / hypothesis: <separate fact from assumption>
- Current action / next action: <one bounded action each>
- Blocker and changed precondition needed: <cause, owner, evidence>
- Remaining limits: <observed attempts/budget or unknown>

## Significant actions and decisions

| ID | Time / revision | APEX phase / PDCA phase | Action or alternatives | Observed result / evidence | Decision / next owner |
| --- | --- | --- | --- | --- | --- |
| <stable ID> | <observed> | <Analyze/Plan/Execute/Verify; PLAN/DO/CHECK/ACT> | <bounded> | <result> | <decision> |

## Interactions

| Source / attempt | Event or report reference and digest | Intended recipient | State proven by the engine | Required response |
| --- | --- | --- | --- | --- |
| <identity> | <public reference> | <owning scope> | <pending/delivered/decision recorded/unknown> | <action> |

Writing a report, emitting an event, supplying context and recording a decision
are different observations. Never label a file "read" merely because it exists.
An engine acknowledgement proves the recorded operation, not human-like understanding.
Use public Swarm state for authoritative statuses; never copy this table into its database.
