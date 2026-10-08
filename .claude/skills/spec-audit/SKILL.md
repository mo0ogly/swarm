---
name: spec-audit
description: "Review a Swarm brief, plan or architecture for omissions and contradictions before execution. Use for a specification audit, missing role/dependency/acceptance checks, or checking whether a plan is ready; not for reviewing implementation code."
---

# Audit a specification

Read `tools/agent-workflows/CONTRACT.md`. Inspect the requested documents and linked
contracts before declaring something absent. This is a read-only audit of the
specification; write a report only if requested or expected by the current task.
In preparation, analyze only supplied documents and label absent context explicitly.

Check:

- Problem, outcome, scope, exclusions and material unresolved choices.
- Stable requirements with observable criteria, including failure and recovery.
- Planner/worker/reviewer responsibilities and the actual engine guard that owns
  each invariant; review coverage cannot be inferred from task titles.
- Acyclic dependencies, freshness rules, handoff evidence and workspace ownership.
- Public web/CLI behavior, errors, persistence, cancellation and restart semantics.
- Provider capabilities, model selection, budgets, quotas and bounded retries.
- Testing and integration conditions, independent review and completion evidence.

For each finding provide ID, severity (blocker/major/minor), exact location or
missing contract, concrete impact, and the smallest clarification or correction.
Separate a confirmed contradiction from a question caused by unavailable context.
Cross-check related documents to avoid duplicates and false missing-feature claims.

If a previous audit exists, retain its IDs and classify findings as closed with
evidence, still open or newly found. Do not erase open findings by replacing the plan.

Return a readiness verdict with blocking conditions, prioritized findings and
actionable next steps. Do not certify runtime behavior, rewrite application code,
or claim the engine accepted a plan on the strength of a document review.
