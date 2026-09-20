---
name: replan
description: "Revise an existing Swarm plan after a concrete failure, invalid assumption or changed requirement. Use to recover blocked coordination without resetting attempts, losing task history or repeating the same failing action."
---

# Replan without hiding the failure

Read `tools/agent-workflows/CONTRACT.md`. Load the canonical plan, public runtime
state, last attempt and relevant evidence. In preparation, propose a revision only;
do not invoke recovery operations or claim live state not supplied in the context.

1. State expected versus observed behavior and identify the first failed precondition.
   Classify the cause: code, environment, unavailable provider, quota, workspace,
   stale evidence, missing specification or exhausted budget. Cite actual evidence.
2. Separate completed/proven work, reusable work needing fresh checks and unfinished
   requirements. Keep requirement IDs, task IDs, attempt counts and historical proof.
3. Compare at least two feasible recovery paths for a structural change. Choose the
   smallest authorized route using risk, cost, reversibility and effect on dependencies.
4. Update the plan revision and decision rationale. Adjust scope, dependencies,
   ownership and tests only where justified. Do not silently remove requirements,
   independent review, isolation or acceptance checks to make the plan pass.
5. Make resumption conditional on a verifiable changed precondition. Never refund
   attempts, raise limits, clear quota delays early, rename a failed task or create
   a replacement mission to evade a ceiling. Use supported public operations only.
6. Continue independent authorized work. If the blocked path requires a new decision
   or authorization, give one precise question and explain why it changes the outcome.

Deliver cause/evidence, options/decision, old-to-new task mapping, affected acceptance
checks, next allowed action and who owns it. Distinguish proposed revision from one
actually validated/applied by the engine. Do not mark a paused or exhausted task done.
