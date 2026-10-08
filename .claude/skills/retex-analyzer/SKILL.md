---
name: retex-analyzer
description: "Produce an evidence-based retrospective of a Swarm mission or delivery, separating autonomous progress, manual interventions, failed attempts and unproven outcomes. Use for RETEX, lessons learned or prioritized improvements after execution."
---

# Learn from the actual run

Read `tools/agent-workflows/CONTRACT.md`. Inspect the mission's public state, original
requirements, revisions, reports, attempts, checks and intervention log. Treat old
reports as historical evidence; use current engine state for current acceptance.
In preparation, draft the measurement plan only, without inventing a past run.

1. Define the interval and baseline. Describe the intended outcome and actual result
   in a few plain sentences. Retain failed and blocked tasks in the account.
2. Trace the important sequence: dispatch, handoff, checks, review, acceptance,
   retry and interruption. Separate provider, engine, environment and operator causes.
3. Report only available measurements: accepted/required results, failed attempts,
   retries with/without changed preconditions, external interventions, wait reasons,
   elapsed time and reported usage/cost. Unknown cost is unknown, not zero.
4. Distinguish real provider runs from fixtures and simulations. State what evidence
   establishes, and where autonomy, reliability or independence remains unproven.
   Compare like-for-like baselines or explain the break in comparability.
5. Identify what worked and the few most costly failure mechanisms. Support causes
   with observations; label hypotheses and propose a check to discriminate them.
6. Prioritize improvements by impact, confidence, effort and regression risk. For
   each give an owner role, changed behavior, acceptance test and success measure.
   Keep quick wins separate from engine changes requiring architectural work.

Output: outcome, evidence/limits, measures, lessons, prioritized next actions and
the smallest useful next experiment. Do not launch a new mission, change limits or
publish reports without authority. A RETEX is not a substitute for completing the
original unmet requirements or obtaining an independent acceptance decision.
