---
name: product-planning
description: "PRD, journeys, stories, architecture and validated plans"
---

# Product planning for Swarm

Read `tools/agent-workflows/CONTRACT.md`. Planning grants no implementation or
adoption authority. Preparation uses supplied documents only; planners and
subplanners have no execution tools. Ask only material unanswered questions and
reuse adopted decisions. Product framing precedes each feature cycle.

## PRD and perimeter
Establish existing/greenfield/replacement mode, users, problem, why now, operating
context, core value loop, included capabilities, exclusions, constraints,
differentiator and measurable success. An external reference is optional and
must be verified before claiming parity. Excluded or unverified references create
no requirements. Record these sections in the PRD.

## Journeys and stories
Describe user goals and sequences. Split into independently shippable user-value
slices, with stable `s<number>-<slug>` IDs, user, action/title, value, observable
criteria, dependencies and execution notes. Frontend, backend and persistence
are tasks within a slice. Score complexity 1–5: split every 5; a 4 needs an explicit
risk. Split stories needing roughly more than ten tasks. Cover each included
capability, preserve exclusions and record coverage.

## Independent story review
Use a distinct context to review PRD coverage, exclusion leaks, vertical slices,
criteria, dependencies/cycles, overlaps and complexity. Retain findings and
readiness verdict. Critical coverage/scope defects block expansion until corrected
and reviewed. The planner cannot substitute its judgement or invent a reviewer.

## Architecture
Request an authorised worker's inspection of existing manifests, entry points,
modules, interfaces, data/control flows, persistence, security, tests, deployment
and operational boundaries. Distinguish active/dead/prototype with evidence.
Preserve accepted decisions and design the smallest justified delta. Greenfield
needs PRD/stories first, stack comparison and ADR before scaffold. ADRs retain
context, options/rejections, decision and consequences. Missing inspected code
remains an open question; planners do not execute that inspection.

## Design and research
Require an existing system or explicit visual direction: no arbitrary palette or
components. Request tokens, typography/spacing/radius, inventory, patterns, states
and do/don't rules. UI stories reference that system. Design their exact fields,
actions, empty/loading/error/success, keyboard/accessibility, responsive behaviour
and gaps. Choose authorised agent design or an external brief; wait for external
results rather than substituting a design. Mockups communicate intent; production
uses real components. Missing direction/system/result remains a visible blocker.
Research actual files, symbols/signatures, callers, consumers, persistence, tests,
traps and blast radius. Unverified dependencies become prerequisite checks.

## Plan and product structure
Plan linked requirements, owned files and final size estimates, dependencies,
deliverable, observable gates, checks, rollback, failure recovery and decisions.
Structural discoveries return to Plan. Implementation requires explicit adoption
and launch authority; `validated: yes` never replaces revision-bound engine checks.
Use `product` with mode existing/greenfield/replacement, journeys
(id,title,goal,story_ids) and stories (id,title,user,value,criteria,complexity,risk
for a 4,depends,task_ids). Lists use explicit IDs. Unplanned stories have
`task_ids=[]`; shared tasks retain one ID; unassigned tasks stay common/unclassified.
Task `phase`: frame/requirements/stories/story-review/architecture/design-system/
research/design/plan/implement/review/deliver/retex. Story links describe a roadmap;
executable order also needs task `depends`. Preserve the existing 8-task/document
limits using bounded increments.

## Optional retrospective
After fresh acceptance and supervisor closure, offer RETEX/quick wins with a
sourced observation or hypothesis, user value, effort, risk and success criterion.
The user chooses a new increment to prepare/adopt; no implicit agent or budget.
Preserve the accepted recipe. A received event can ground `revise-instructions`
for future owned tasks, validated by default and rejectable before reservation;
criteria remain unchanged. An adopted RETEX task depends on the recipe task.
