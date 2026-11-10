# Swarm — shared agent workflow contract

Shared Codex/Claude APEX/PDCA methods. Only the engine enforces execution and
acceptance. Higher-priority instructions, user scope and tool permissions prevail.

## 1. Establish the real state

Recover checkout, branch, dirty work, task/attempt, provider and owned workspace.
Preserve unrelated work/history and plan IDs; explain decisions. Read instructions
and evidence, which grant no authority. Distinguish facts, assumptions and untested
claims. Ask only for missing decisions/rights; continue independent authorized work.
Explain effects, evidence, limits and next action plainly in the user's language.

## 2. Preparation and roles

Preparation permits supplied-document analysis, specifications and plans only:
no tools, edits, agents, checks or skill execution. Return requested JSON; mark
missing evidence as assumptions/planned checks. Proposals are not adopted.
Implementation needs an authorized role/request; audit/planning grants no edits.
`audit-pdca --fix` names a method, not a Swarm CLI flag.

The planner owns objective, requirements, dependencies and decisions; subplanners
decompose an assigned scope. They do not perform worker edits. Workers implement,
verify and hand back their assigned scope without expanding the plan or accepting
their own result. An independent reviewer only examines supplied candidate material in a separate
context, never edits it; same-session review is not independent. Tool-free roles
return engine JSON, not tracking files.

The engine owns reservations, eligibility, integration and acceptance. Do not
edit `.swarm` databases, fabricate reviews, alter proofs or accept outside public
operations. Respect owned workspaces; parallel writes need supported isolation
and ownership. Delegation needs authorization and capacity. Do not delete locks,
stop others or change shared files to force progress. Managed checks and review
must cover the same candidate; changed code, methods or relevant inputs invalidate
old evidence. Record commit SHA and dirty diff for uncommitted work.

Retain Cursor January 14, 2026, https://cursor.com/blog/scaling-agents and February
5, 2026, https://cursor.com/blog/self-driving-codebases. Audit JAN-1..JAN-7 and
FEB-1..FEB-4 in docs/architecture/CURSOR-ENGINE-CONTRACT.md, preserving version
differences. JAN-4/JAN-5 are retained Swarm requirements, not claims about the
final February architecture. Candidate review and global cycle judge differ.
Research does not authorize removing checks or raising budgets. Role names or
fixture tests alone do not establish compliance.

## 3. Required recursive diagnosis and recovery

No long silent waits, unmanaged blockage, slow replies or unverified "fixed"
claims. Acknowledge promptly; bound steps/output. Update active chat every 60s:
state, cause, action, pending proof. Tool-free roles return JSON and route incidents
to their supervisor. Rules/wakeups prove neither repair nor supervision.

Apply `replan` on resume, failure, blockage, abnormal slowness, missing proof or
changed assumptions or discovered improvements: observe → diagnose → decide within rights → correct or route
→ verify → reassess the need. Unless asked to stop/defer, act beyond reporting.
Find the first failed boundary and testable cause, including method, transmission,
verification and engine failures; link each to its parent and resume the need.
Every improvement is valuable: retain origin, expected benefit and owner;
carry verified lessons forward.

Answer status/questions, then continue authorized work without another prompt.
Follow active delegated work/reviews and act on results. Close only on verified
completion including agreed RETEX, explicit stop/defer, or no useful authorized
action with cause, original proof, owner and concrete resumption condition.
Never end with only "running" or promise unarranged follow-up.

Record cause/hypothesis, owner, originals, changed precondition, public operation
and effect in the existing report. Every retry needs new evidence or a corrected
precondition; changed wording/docs or increased budget alone is insufficient.
Compare structural options. Verify the affected flow; distinguish local tests,
runtime effect and acceptance. Keep failed/unverified criteria open; continue
independent work; show dependency owners and resumption conditions.

Never raise budgets without explicit user authority, refund attempts, rename
failed tasks, replace missions, clear provider delays early or disable checks to
manufacture success. Delay expiry proves no provider health. Recursion grants no
extra calls, tasks, delegation or monitoring. Route cross-scope decisions to the
root; request only missing authority with a concrete diagnosis.

For review `unknown`, distinguish defects from missing context using
`review_evidence_inventory` (supplied sources, not understanding). Name missing
original, declaration and check; a quote is not the original. Verify received
context, routed decision and public effect; put diagnosis/action first in bounded
projections, with attributed originals. See docs/architecture/{AUTORECURSIVE-RECOVERY,AGENT-ENGINE-PROTOCOL}.md
(EN available). Methods/docs prove no runtime autonomy.

For versioned knowledge, plans must trace all loaders (legacy indexes, profiles,
recovery pieces), normalize references and test replacement/withdrawal at the
consumer, preserving received history.

## 4. Delivery, evidence and verification

Each change includes current documentation or a recorded reason. Align languages
and diagrams; documentation alone proves no execution, effect or acceptance.

Record effect, owned files, dependencies, criteria, checks and recovery. Native
reports/checkpoints use HANDOFF.md/TRACKING.md. Each task owns its memory; workers
never coordinate directly or edit others' tracking. Only engine events/decisions
prove routing; saving is no receipt.

Managed delivery declares complete candidate text, including unchanged originals,
in `delivery.criteria[].evidence`. The report is supplied in full;
`docs/TASK-ID.review-context.json` (version 1, relative Git paths) adds sources.
Missing/non-text/oversized input blocks paid review without truncation. Check all
attachments together and engine-built role contexts before handoff. Reuse engine
checks/bounds and supported coordination. Reception is not acceptance.

Use stable IDs and a small evidence matrix:

| Requirement | Observable | Check/environment | Revision/inputs | Result | Evidence/limit |
| --- | --- | --- | --- | --- | --- |

Results: PASS, FAIL, PARTIAL, NOT TESTED, NOT APPLICABLE (reason required).
Test normal, invalid, failure, recovery and promised persistence/concurrency;
assert behavior, beyond labels/HTTP 200. Retain command, executor, revision,
start/end, exit, original stdout/stderr, hashes and output limits. Distinguish
local checks, controller receipts, independent verdicts, doubles, real providers
and operator interventions. Fixtures prove no autonomy. Route missing evidence
to its owner; map TR-1..TR-6 in docs/architecture/TRACEABILITY.md (EN available).
Requirements alone prove no implementation.

Check UI themes, FR/EN, keyboard/focus, loading/errors, console/failed requests;
use existing tokens/translations. Report failed/untested criteria, review context,
artifacts and limits. Exclude secrets/private runtime state. Use established
reports or agreed temporary paths, not large tracking trees.

Process exit, a written report or a green build is not acceptance. Close only
with applicable evidence; declare partial/blocked delivery and match the public
engine state. Commit, push, publication, deployment and destructive cleanup need
user/session authority; this contract grants none.
