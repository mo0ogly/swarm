---
name: replan
description: "Recover a Swarm plan after an anomaly, changed assumption or improvement; preserve history, authority and verified outcomes."
---

# Replan without hiding the failure

Read `tools/agent-workflows/CONTRACT.md`; apply its required recursive diagnosis,
continuity and closure rules. Load canonical plan, public state, last attempt and
evidence. In preparation, only propose from supplied context; no recovery operations.

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
   For `unknown` due to missing context, consult `review_evidence_inventory`.
   Name the missing original, its delivery evidence path or supplemental manifest,
   and the check that will make the criterion verifiable. Extra citations alone
   do not supply the original; do not edit an unchanged source just to create a diff.
6. Continue independent authorized work. If the blocked path requires a new decision
   or authorization, give one precise question and explain why it changes the outcome.
7. Reassess the original requirement. Diagnose failed diagnosis/context/control/
   recovery recursively, linked to its parent; consider your method and the engine.
   Verify received context, recorded decision and public effect. An unchanged failure
   needs a different discriminating check or escalation, not another retry.
8. Correct the general faulty contract, never a mission-ID exception. Keep the
   original failing case as a regression guard with relevant variants and invalid
   inputs. Verify reception, decision, application and effect before claiming a
   durable correction; carry the learning into shared future instructions. Label
   temporary measures as temporary and keep the underlying incident open.

Deliver cause/evidence, options/decision, old-to-new task mapping, affected acceptance
checks, next allowed action and who owns it. Distinguish proposed revision from one
actually validated/applied by the engine. Do not mark a paused or exhausted task done.
