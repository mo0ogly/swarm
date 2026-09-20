---
name: audit-pdca
description: "Run an evidence-based PDCA audit of Swarm's engine, CLI, web, provider integration or agent coordination. Use for audit-pdca/audit_pdca, a reliability audit, or a measured audit-and-fix cycle; do not substitute it for a simple diff review."
---

# Audit PDCA for Swarm

Read `tools/agent-workflows/CONTRACT.md`. Default to **audit-only**: inspect and
write the requested report, without changing application code or runtime state.
Use **fix** mode only when the user authorizes corrections (including `--fix`).
`--score-only` requests measurement, not remediation. Arguments are skill requests.
In Swarm preparation, only the PLAN phase is permitted.

## PLAN — define what is being measured

Read the previous audit for the same scope. Fix the baseline revision, exclusions,
critical behaviors, rubric and available evidence before evaluating results.
Inventory the public operations and expected effects, not just existing tests.

| ID | Trigger / inputs | Preconditions | Expected effect | Observable | Failure cases | Criticality |
| --- | --- | --- | --- | --- | --- | --- |

For coordination audits, include role admission, worker ownership, independent
review, dependency freshness, restart/recovery, quota and attempt limits, partial
reports, acceptance and equivalent CLI/web paths. Verify claims against actual
engine entry points. A missing test or unavailable environment is not compliance.

## DO — collect and, when authorized, correct

Collect reproducible findings: location or public operation, preconditions, observed
result, expected result and impact. Separate facts from hypotheses. Avoid flags
from another project, destructive real-data tests and unrelated automatic changes.

In fix mode, compare alternatives for a major finding, implement a bounded repair
and keep a regression-sensitive test. Preserve the original failing evidence.
Without fix authority, supply an actionable correction proposal and continue the audit.

## CHECK — replay the behavior

Run applicable static, security, behavioral, boundary and regression checks. Exercise
normal, invalid and failure/recovery paths, and persistence/isolation where relevant.
Capture runtime diagnostics, exact command/options, exit status, environment and
revision. UI checks include both themes, languages and keyboard interactions.
Tests must cross the boundary where the defect occurs, with assertions on its effect.

Use `PASS`, `FAIL`, `PARTIAL`, `NOT TESTED`, `NOT APPLICABLE` with reasons.
Missing tools or unavailable providers mean NOT TESTED. Compilation or a label's
presence is structural evidence, not a behavioral PASS.

If a score is requested, define weights and partial-credit rules before checking,
publish evidence coverage alongside the score and make the calculation reproducible.
Critical failures cannot be compensated by high scores elsewhere. Never change the
rubric after observing results or invent a precision unsupported by evidence.

## ACT — decide the next useful action

Compare compatible baselines; identify fixed, unchanged and new findings by stable ID.
Continue authorized remediation while it can improve the result within the agreed
budget. Stop repeated unchanged failures; diagnose and replan. Preserve limits and
quotas. A blocked critical check prevents a compliance claim, even if the audit
report itself is finished.

Produce a concise report with scope/baseline, behavioral matrix, prioritized findings,
commands/artifacts, before/after evidence, optional score and coverage, interventions,
limits and next actions. Distinguish **audit completed** from **system compliant**.
