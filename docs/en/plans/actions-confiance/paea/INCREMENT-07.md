# PAEA — increment 07: Rust receipt/action/lifecycle correspondence

[Français](../../../../plans/actions-confiance/paea/INCREMENT-07.md) · [Plan](../PAEA-APEX.md)

Authorized “go next” resumes APEX/replan with internal GLM contribution. Earlier
comparison cannot bind an observation to an action; `decode_receipt` only checks
shape. Rather than add another receipt/service, this source increment reuses the
existing canonical action, receipt and lifecycle contracts.

`check_receipt_transition(action, receipt, expected_sink, previous)` validates the
receipt and expected-sink identifier, matches exact action/intent/sink, then asks
`State::transition` for the declared outcome's proposed state. Exported from the
existing Rust library. No new Python component or Cargo dependency.

**Written, uncompiled, uninstalled; four Rust tests written, none executed.**
No-compilation/no-restart instruction remains active. No new CLI/API command or
runtime mutation. This prepares PA-01/03/09, without closing an effect/proof lot.

## Contract and errors

Receipt validation also applies to directly constructed Rust values. Expected
sink uses the existing identifier grammar, not a destination policy. No amount,
rights, state or effect is mutated; no new operator setting, timeout/retry/limit.

COMMITTED/UNKNOWN may follow COMMIT_REQUESTED; DENIED follows earlier nonterminal
phases allowed by the existing lifecycle. Only explicit reconciliation outcomes
can follow UNKNOWN. Raw COMMITTED/DENIED/UNKNOWN after UNKNOWN refuse. Terminal
states cannot advance/replay; reading a stored result is a separate operation.

Static errors: existing `protection-effect-receipt`, new
`protection-receipt-expected-sink`, `protection-receipt-binding`, and existing
`protection-transition-invalid`. Existing Error is retained, no private values
printed. The pure function performs no IO, publication, flush, retry or partial
mutation.

**Success authenticates nothing.** All inputs can be supplied data; matching
forged receipts can pass. No independent observation, signature, actual observed
artifact digest, current authority, protected ordering or uniqueness/consumption
is checked. The protected owner must establish these before persisting/acting on
a proposed state. An agent-supplied “verified” flag cannot supply that boundary.
Existing receipt schema is unchanged.

## Pending checks and dependencies

Four unexecuted Rust tests cover both domains/all supported outcomes; mismatched
action/intent/sink and alternative content under the same intent; UNKNOWN/terminal
relabel/replay refusals; invalid receipt/expected sink. Controls/recovery surround
faults and inputs stay unchanged. Frozen forged fixtures demonstrate the explicit
non-authenticity limit.

`paea-contract` recipe now selects the exact receipt target/four names; it is not
run. Earlier selections remain 17 contract, 3 comparison, 8 Linux CLI and 5 legacy
schema/oracle tests. Counts are selections, not passing results for this candidate.
Byte-bound policy/function/error review cannot substitute for compilation or
independent acceptance.

Actual Identity producers already validate human enrollment/permissions.
`decode_active` explicitly permits old Statement/Expected pairs after revocation;
no freshness is inferred. PA-02 Identity/integration and AC-02 protected-boundary
handoffs remain open. Codex must compile/run affected targets/Rustdoc/recipe once
user lifts no-compilation, then connect authenticated services and observations.
Sponsor must still appoint E3 evaluator and third-party team. Foreign sessions,
suspended missions/data/processes stay untouched. No commit/push/install/publish.
Full PA-00…13 program and final retrospective remain open.

## GLM and actual static checks

One SKYNET call finished `stop`, no client cap or imposed effort: 11,736 input /
12,666 output / 24,402 total reported tokens, 84.484 seconds. Requested/reported
`glm-5.2` does not attest backend version. Complete producers were supplied, not
full PA-01/03/09 criteria; review cannot establish program coverage or independence.
No functional defect reported on the initial source; this is not a test result.

Integrated explicit exhaustive State matching instead of `_` to force review on
new states, plus pre-commit denial documentation. Tests pin case-sensitive sink,
representation-valid forged observation digest and compatible changed transport
key. GLM's statement that no transport alias can pass does not describe this
contract: request_id is intentionally excluded. Private CanonicalAction fields
establish representation, not authenticated context.

Stable rustfmt formatted and syntax-parsed the two new files, exit0/no diagnostic.
Command/tool identity and originals retained. This is **not compilation or a test**;
no type checking, macro expansion or effect verification. JSON syntax, selected
test names/counts, note links, hashes and whitespace diff are checked separately.
No effect process, server or compilation-dependent test was launched.
