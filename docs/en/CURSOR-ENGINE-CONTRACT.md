# Engine contract: alignment with Cursor's final design

[Français](../architecture/CURSOR-ENGINE-CONTRACT.md)

Source: [Cursor — The final system design](https://cursor.com/blog/self-driving-codebases#the-final-system-design), accessed September 19, 2026.

This document separates a design rule, its engine enforcement and evidence that
it ran. “Cursor compliant” is not a certification: the article describes a
research experiment and its tradeoffs. Deterministic tests alone do not prove
that a live Swarm mission finishes without assistance.

## Responsibilities

| Role | Responsibility | Permitted effect |
|---|---|---|
| Root planner | Owns the complete requirement; decomposes and adapts the work. | Structured planning decisions, no coding commands. |
| Scope planner | Owns its delegated slice; may delegate recursively within limits. | Tasks and decisions within its scope, no coding commands. |
| Worker | Completes a bounded task in its own repository copy. | Local changes and a handoff to the owning planner. |
| Independent reviewer | Examines the result and evidence for the candidate revision. | A reasoned opinion, without writing tools or self-acceptance. |
| Deterministic controller | Executes checks and enforces publication rules. | Receipts, refusals and atomic transitions when all conditions hold. |

The first three roles follow Cursor's final design. Mandatory independent review
is an additional Swarm requirement: Cursor removed its judge during the evolution
described in the article. Swarm requires a green revision before acceptance;
Cursor describes tolerating transient errors to increase throughput.

Repository copies separate changes and Git references. They are not a security
boundary against a malicious process with the host account's permissions. These
guarantees concern public engine operations; an administrator who can alter the
database or executables is outside that boundary.

## Rules to verify in the engine

| Invariant | Required refusal | Evidence to retain |
|---|---|---|
| Planners do not code in a hierarchical mission | An explicit planner/subplanner launch is rejected, never silently converted into a worker. | CLI and HTTP response; no residual agent, reservation or copy. |
| Exclusive delegation | A delegated requirement cannot be reassigned or handled by the wrong owner. | Rejected decision; unchanged revision and unconsumed events. |
| Worker isolation | Two attempts cannot write into one shared copy. | Paths, references and observed contents of two actual test copies. |
| Handoff ownership | Foreign scope, old attempt, another copy's file or incorrect digest is rejected. | Unchanged state after refusal; one valid return to its owner. |
| Reactivation | A current handoff wakes its planner; replay does not duplicate it. | Store reopened with attempt and event identities preserved. |
| Descendant closure | A parent cannot close with open children, unfinished work or stale evidence. | Each refusal exercised with freshness checks. |
| Independent acceptance | Passing checks without favorable review, or stale/unknown/negative review, are insufficient. | Candidate SHA, attempt, contract, hashed review context and receipts. |
| Bounded recovery | Known quota, exhausted limits or an already reserved call do not grant extra attempts. | Preserved counters/history; durable hold and explicit cause. |

See the [provider quota guide](PROVIDER-QUOTAS.md) for holds and explicit clearance.

Legacy manual launches without hierarchical organization remain compatible. They
are not autonomous missions demonstrating these properties; an autonomous launch
requires the complete organization.

## Remaining demonstrations

1. **Handoff quality.** The current contract carries findings and artifacts. It
   does not yet require separate fields for changes, limitations, deviations and
   requested decisions. Correct routing does not establish this quality.
2. **Live autonomy.** Observe a requirement, delegation, production, reasoned
   rejection, correction and closure using a real provider. Count simulated
   responses and external host interventions separately.
3. **Visible recovery.** After manually clearing a hold without a reset time,
   historical evidence must remain visible without appearing as a new active
   rejection. Permission to try again does not establish provider availability.
4. **Throughput and concurrency.** Measure the integration lock during checks and
   review, and the cumulative context size. Current serialization and refusal of
   oversized context must remain visible.

## Definition of done

Normal process exit is insufficient. An accepted result needs the tested
revision, actual commands with timestamps and exit codes, report, independent
opinion, decision, freshness, deliverables and external intervention count.
A host-written fix can strengthen Swarm; it must not become fabricated autonomous
success inside the mission evaluating it.

## Evidence size and review transport

The review prompt remains capped at 192 KiB, including instructions. The engine
preserves the diff from the repository base, reports, receipts and sources from
the same candidate. It first reduces unchanged patch context and removes only
new source files already present in full in the patch, after exact comparison.

If JSON escaping still exceeds the cap, long texts use complete UTF-8 blocks
identified by field, byte length and SHA-256. Other metadata remains JSON. The
canonical stored context and its digest stay unchanged. No summary replaces
proof and no earlier change is removed from the diff. A packet that still exceeds
the cap is rejected before calling the provider. Process tests reconstruct the
whole packet and compare every field with stored evidence; genuinely oversized
packets remain rejected.
