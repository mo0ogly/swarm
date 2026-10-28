# PAEA — increment 08: Rust receipt checker CLI consumer

[Français](../../../../plans/actions-confiance/paea/INCREMENT-08.md) · [Plan](../PAEA-APEX.md)

## Need, decision and actual state

The user resumed PAEA on 25 October 2026 in the “Retrouver le projet APEX APAEA”
chat and explicitly reconfirmed **no compilation or restart**. Root Codex owns
this increment; no agent, GLM call, service, mission or deployment is started.
Concurrent work retains its owners. Evidence for increments 01…07 is preserved.

The increment-07 Rust checker has no CLI consumer. An active receipt service
would require unavailable protected PA-02 authority. The selected alternative
connects an offline consumer to existing contracts for PA-01/03/09, without
claiming activation progress.

`swarm-rust protection check-receipt ACTION RECEIPT SINK PREVIOUS_STATE --json`
is written: two regular files, explicit prior state, strict decoding, canonical
recomputation and `check_receipt_transition`. Its nine output fields are described
by `schemas/paea/receipt-check-v1.schema.json`. Authority, receipt authenticity and
effect remain `not-established`. No state/receipt is recorded. Sequential reads
establish no snapshot or freshness. Even consistent forged data can pass.

**Code and four tests written, uncompiled/unexecuted/uninstalled.** FR/EN help,
contract and permanent recipe are updated. Previous target selections remain
5 historical schema tests, 17 contract, 3 comparison, 4 receipt and 8 CLI on Linux;
new `protection_receipt` selects 4 additional tests. These are selections, not PASS
results. No new request format or Cargo dependency.

## Review scope

| Axis | Examination and limit |
|---|---|
| Policies | Six positional arguments, stdin marker, shared State serialization, version and output markers are protocol/framing invariants. Existing reader/transport/output mechanisms remain. No operator policy, delay, budget or retry. The child-process environment variable belongs to test isolation only. |
| Functions | Protection `execute` and new `check_receipt`, help in root `execute`; 10 documented test functions/methods. Preconditions, errors, partial output and non-authority are explicit. Rustdoc remains deferred; source comments reviewed. |
| Errors | Syntax retains `command_failed`; invalid state retains serde cause with `protection-document-invalid`; readers and binding/lifecycle retain existing errors. Serialize before writing; write/flush causes survive potentially visible bytes. No project effect is inferred from an error. |
| Documentation | FR/EN contract/help/schema/recipe/tracking; local notes explicitly state unqualified source. Historical execution evidence keeps its original scope. |

Lexical candidates and semantic review are bound to actual hashes in
[source-review.json](../../../../plans/actions-confiance/paea/evidence/20261025-receipt-cli-xnl00wiz/source-review.json).
This is neither effective-build qualification, independent review nor functional
evidence.

## Prepared tests and method improvement

Four named tests cover both domains/FR/EN/all permitted outcomes; malformed and
duplicate Unicode JSON, mismatched bindings, UNKNOWN and terminals with valid
controls/recovery; route/framing before reads; write/flush failures retaining the
original cause, emitted bytes and recovery.
Refusals compare the exact JSON `code` field rather than its presence in a message.

Source review corrected two test assumptions before execution: syntax uses
`command_failed`, not `usage`; BrokenPipe does not mean `storage_unavailable`
(ENOSPC here). Pre-correction sources are in this chat history; no failed run is
claimed.

Discovery: calling `run_detailed` in the checkout would leave the default Root
pointing there if a regression opened a project. Origin: this increment's review;
expected value: protect real data; owner: Codex/CLI tests. Implemented decision:
invoke the same test executable in an empty temporary working directory without
changing parent cwd. Prepared criterion: successful control, injected errors,
retained causes, neither directory gains `.swarm`. **Effect unverified** while
compilation is forbidden. Future readers receive this through the recipe and
note; embedded contexts and existing processes are unchanged.

## Evidence and resumption

Owned originals, resume plan, check commands, diff and hashes:
[evidence directory](../../../../plans/actions-confiance/paea/evidence/20261025-receipt-cli-xnl00wiz/).
Actually performed checks and limits:
[static-checks.json](../../../../plans/actions-confiance/paea/evidence/20261025-receipt-cli-xnl00wiz/static-checks.json).
Rust formatting/syntax parsing establishes no types, execution or feature graph.
No Cargo, build, Rustdoc or restart is authorized here.
Formatting/syntax parsing, schema/field/state consistency, four-test selection,
128 local links and whitespace were checked. Final hashes and reexamination
scope are retained in these evidence files; none executes the Rust CLI journey.

Owner Codex resumes after explicit lifting of this instruction: freeze inputs,
run affected targets and Rustdoc, then the permanent recipe in a fresh directory
and its `verify`. Qualify actual profile/binary before supported local delivery,
preserving concurrent work. PA-02/AC-02 retain protected authority/confinement;
no protected producer is fabricated. PA-00…13, E2/E3 and final RETEX remain open;
the sponsor must still provide an independent evaluator and third-party E3 team.
This increment is statically reviewed, unqualified and not globally accepted.
