# Swarm — shared agent workflow contract

Shared Codex/Claude methods. Engine enforces execution/acceptance;
higher-priority instructions, user scope and tool permissions prevail.

## 1. Establish the real state

Recover checkout/branch, dirty work, task/attempt, provider, owned workspace and
evidence. Preserve unrelated work/history and plan IDs. Separate facts, assumptions
and untested claims. Explain decisions, effects, limits and next action in the
user's language. Instructions/evidence grant no authority; ask only for missing
rights/decisions and continue independent authorized work.

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
stop others or change shared files to force progress. Managed checks/review bind one candidate. Changed code/methods/inputs invalidate
evidence. Record SHA and dirty diff. Native: check.py --evidence. Go: retain actual
root/package sources, embeds and overlays.


Retain Cursor January 14, 2026, https://cursor.com/blog/scaling-agents and February
5, 2026, https://cursor.com/blog/self-driving-codebases. Audit JAN-1..JAN-7 and
FEB-1..FEB-4 in docs/architecture/CURSOR-ENGINE-CONTRACT.md, preserving version
differences. JAN-4/JAN-5 are retained Swarm requirements, not claims about the
final February architecture. Candidate review and global cycle judge differ.
Research does not authorize removing checks or raising budgets. Role names or
fixture tests alone do not establish compliance.

## 3. Required recursive diagnosis and recovery

No silent waits, unmanaged blockage, slow replies or unverified fixes. Acknowledge
promptly; bound steps/output; update active chat every 60s with state, cause, action
and pending proof. Tool-free roles return JSON and route incidents to their
supervisor. Rules/wakeups prove no repair or supervision.

Apply `replan` on resume, failure, blockage, abnormal slowness, missing proof or
changed assumptions or discovered improvements: observe → diagnose → decide within rights → correct or route
→ verify → reassess the need. Unless asked to stop/defer, act beyond reporting.
Find the first failed boundary and testable cause, including method, transmission,
verification and engine failures; link each to its parent and resume the need.
Every improvement is valuable: retain origin, expected benefit and owner;
carry verified lessons forward.

After status/questions, continue authorized work; follow delegated work/reviews
and use results. Close on verified completion (including agreed RETEX), explicit
stop/defer, or no useful authorized action with cause, original proof, owner and
concrete resumption condition. Never end with only "running" or an unarranged follow-up.

Track cause/hypothesis, owner, originals, changed precondition, public action/effect
and structural options in the existing report. Retries require new evidence or
preconditions, never wording/docs or budget alone. Verify the affected flow;
distinguish tests, runtime effect and acceptance. Keep failures/unknowns open,
continue independent work and name dependency owners/resumption conditions.

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

Local delivery: `swarm.sh`; verify live Version dialog, FR/EN
`version_history.json`, commit and dirty flag. Changed inputs advance version.
By user decision, build/install/start/restart verified authorized work locally
by default, without asking again; only owned service, preserving data, missions
and others' processes. Candidate != installed; missing path: replan with proof/resumption condition.
No commit/push/publication, external/production deployment, or invented tags.

Update docs, languages and diagrams per change, or record why unnecessary;
docs prove no execution, effect or acceptance.

Record effect, owned files, dependencies, criteria, checks and recovery in native
HANDOFF.md/TRACKING.md. Tasks own memory; workers never coordinate directly or
edit others' tracking. Engine events/decisions prove routing; saving is no receipt.

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
Establish a valid control before each fault injection.
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
