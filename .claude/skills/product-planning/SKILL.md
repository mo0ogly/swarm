---
name: product-planning
description: "PRD, journeys, stories, architecture and validated plans"
---

# Product planning for Swarm

Read `tools/agent-workflows/CONTRACT.md`. This is the planning part of the product
application lifecycle, not permission to implement or adopt anything. Preparation
uses supplied documents only; planners and subplanners have no execution tools.
Ask only material unanswered questions. Reuse the current adopted decisions.

## PRD and replacement perimeter
Establish mode (existing capability, greenfield or replacement), users, problem,
why now, operating context, core loop, included capabilities, exclusions,
constraints, differentiator and measurable success. An external product is optional;
its behaviour must be verified before claiming parity. Never invent a requirement
from an excluded or unverified reference. Produce a PRD with those sections.

## Journeys and end-to-end stories
Describe each user's goal and sequence of actions. Split into independently
shippable user-value slices, not technical layers. Each story has a stable
`s<number>-<slug>` ID, user, action/title, value, observable acceptance criteria,
dependencies and execution notes. Frontend, backend and persistence are tasks
inside a slice. Score complexity 1–5: split every 5 before planning; a 4 needs an
explicit risk. Split again if one story needs roughly more than ten tasks.
Give every included capability at least one covering story; exclusions stay out.
Record coverage, not just a list of nice-looking story titles.

## Story review before expansion
Request a distinct review context of the PRD-to-story coverage, exclusion leaks,
vertical slices, criteria, dependency order/cycles, overlaps and complexity.
Keep the findings and readiness verdict. A critical coverage or scope defect
blocks expansion until corrected and reviewed again. A planner cannot replace
this review with its own judgement or fabricate a reviewer result.

## Architecture
For existing code, request inspected as-is evidence: manifests, entry points,
modules, data/control flows, interfaces, persistence, security, tests, deployment
and operational boundaries. Preserve accepted decisions. Define only the smallest
justified capability delta. Greenfield requires the PRD and stories first; compare
stack options from their constraints, capture the chosen ADR before scaffolding.
Each ADR records context, options/rejections, decision and consequences; preserve
old ADRs rather than rewriting history. Missing inspected code is an open question,
not an architectural fact. Inspection belongs to an authorised worker task.

## Design system and screen design
An existing system or explicit visual direction is mandatory. No arbitrary palette,
components or visual identity. Request tokens (colour, typography, spacing, radius),
component inventory, UI patterns, four states and do/don't rules. UI stories must
reference that system. Design covers only their screens and acceptance criteria.
Choose authorised agent design or an external design brief; an external choice
waits for its result and does not silently generate a substitute. Record exact
fields/actions, empty/loading/error/success, responsiveness, accessibility and gaps.
A mockup communicates intent; production must use the application's real components.
Missing system, direction or returned external output remains a visible blocker.

## Research and story plan
Research records actual files/symbols/signatures, callers, consumers, persistence,
tests, traps and blast radius; distinguish active/dead/prototype using evidence.
Unverified dependencies become prerequisite checks. Plan with linked requirements,
owned files, final file-size estimates, dependencies, deliverable, observable gates,
checks, rollback, failure recovery and decisions. Structural discoveries return to
Plan; no improvised expansion. No implementation before explicit plan adoption and
launch authorisation. A file named `validated: yes` never substitutes for Swarm's
revision-bound adoption, review or dispatch controls.

## Product structure in a Swarm action plan
Use optional `product` with `mode` existing/greenfield/replacement, `journeys`
(id,title,goal,story_ids) and `stories` (id,title,user,value,criteria,complexity,
risk for a 4,depends,task_ids). Lists reference explicit IDs. Stories not yet broken
into tasks have task_ids=[] and remain unplanned. Shared tasks may appear in several
stories; unassigned tasks stay in the common/unclassified view. Optional task `phase`
is frame/requirements/stories/story-review/architecture/design-system/research/
design/plan/implement/review/deliver. Story relationships describe the roadmap;
executable ordering must also be expressed in task depends. Retain the existing
8-task and document limits: plan bounded increments, never inflate these limits.
