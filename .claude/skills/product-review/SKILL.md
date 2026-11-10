---
name: product-review
description: "Independent coverage, design and candidate review"
---

# Product review for Swarm

Read `tools/agent-workflows/CONTRACT.md`. This method defines product review to Swarm's
independent, tool-free reviewer. Inspect only supplied evidence in the assigned
context; never execute tools, rewrite stories/code, launch workers or approve your
own candidate. Missing evidence remains unknown. Native authorised review sessions
may run their own checks; do not claim those checks ran inside Swarm's reviewer.

## PRD and stories
Walk every included PRD capability and map it to at least one end-to-end story.
Check exclusion leaks and invented parity, user value rather than technical-layer
stories, observable criteria, stable sN-slug IDs, duplicates/overlap, acyclic and
executable dependencies, complexity 1–5 (a 5 must be split; a 4 documents risk).
Distinguish roadmap story dependencies from executable task dependencies. Inspect
shared-task mappings for accidental double ownership or duplicated acceptance.
Uncovered capability, scope leak or impossible order is critical; an untestable
slice, unsplit 5 or overlap is major. Report findings without changing the breakdown.

## Architecture and design
Assess inspected as-is evidence before any proposed delta. Check immutable ADRs,
interfaces/persistence/security/operations and requirement-driven choices. Missing
architecture evidence is not replaced by a plausible default. UI stories require
an identified design system and screen intent: components, tokens, exact actions,
empty/loading/error/success, responsiveness, accessibility and explicit design gaps.
A missing system or invented visual identity blocks design readiness. Check the real
components against that intent; do not require copying a mockup's HTML verbatim.

## Candidate and execution evidence
Review the same candidate revision/diff as the supplied controls and adopted plan.
Check scope, criteria, verified APIs/imports, instructions and ADR/design compliance,
regressions and failure recovery. Require exact check commands, tested artifact and
meaningful assertions. An old candidate, invented source or unexecuted test presented
as PASS is a blocking evidence defect. Distinguish self-checks, independently run
checks, doubles, real provider runs and human interventions.

## Verdict and recovery
Report stable finding IDs, critical/major/minor severity, inspected location,
observable impact and smallest correction. Record coverage and PASS/FAIL/PARTIAL/
NOT TESTED, candidate identity, remaining uncertainty and readiness conditions.
Never soften a critical finding to advance delivery. Return a negative or incomplete
proposal when required evidence is missing. Corrective work belongs to a worker;
changed code or contracts require fresh checks and review. Preserve previous findings
and history. Only supported engine operations decide acceptance/integration; labels
such as Stories ready or Ship allowed do not override those operations.
