# PAEA — increment 10: standalone report bound to exact inputs

[Français](../../../../plans/actions-confiance/paea/INCREMENT-10.md) · [Plan](../PAEA-APEX.md)

## Need and decision

“next” resumes APEX/replan, root Codex alone, with the no-compilation/no-restart
instruction intact. No agent, provider, live Root or service started. PAEA-OBS-01
resumes PA-01/09: two receipts with different supplied observation digests share
the same v1 report, proven by increment-09 originals. That declared limitation
claimed no authenticity; complete recipe transcripts already mitigated it.

Changing v1 in place breaks strict consumers; external bundles alone do not bind
a standalone report. Selected option: separate `receipt-report` output schema v2,
while inputs and `check-receipt` remain v1. No authority or service alias.

`check_receipt_documents` and `ReceiptReport` join the existing Rust receipt module.
Strict decoding, canonicalization and the same binding/lifecycle checker precede
reuse of `DocumentIdentity` plus SHA-256 of both consumed byte slices. CLI uses
existing readers/writer. FR/EN help/contract, schema and recipe updated; no new
dependency or Python product implementation. Authority/authenticity/effect stay
unestablished.

**Written, uncompiled, unexecuted, uninstalled.** Frozen dev75 baseline records
valid v1 control → absent new command → v1 recovery without a build. This proves
the pre-change missing capability, not new-code success. Seven new tests are
selected only: four library, three CLI on Linux. Full recipe/Rustdoc deferred.

## Contract and review

Output holds version2, scope, raw/canonical document identity, raw receipt hash,
prior/proposed state and three non-authorizing markers. Formatting, transport
aliases and observation changes remain distinguishable through raw identities,
even when canonical action/intent agree. No hash proves signature, time or
independent observation. Protected owners still establish provenance/freshness.
CLI reads remain sequential, without an atomic snapshot.

| Axis | Review and limit |
|---|---|
| Policies | Literal2 and separate command identify output protocol, not operator settings. Existing SHA-256/not-established contract. Test lengths/case arrays/digests are fixtures. No budget, retry, deadline, cap or active policy added. |
| Functions | New public type/function documented; CLI execute/helper/help connected. Thirteen test functions documented. Preconditions, errors, IO, non-authority and possible partial output reviewed. No Rustdoc/type verification under the instruction. |
| Errors | Existing strict decoders/checker run before report construction, preserving codes/JSON causes. Shared reader close/cleanup handling and writer errors retain causes; serialize before write/flush. No empty-success fallback or persisted state. |
| Documentation | FR/EN operation, input/output versions, schema, compatibility and unexecuted status. Local notes explicitly source-only. No increment-09 proof rewritten or claimed as new-code PASS. |

Prepared tests cover both domains, raw formatting, observation changes, transport
aliases, malformed/mismatched/lifecycle errors and recovery. CLI tests cover FR/EN,
preserved v1 shape, runtime/stdin refusals and `/dev/full` output failure, separate
from injected Rust flush failure. No API/UI/protected service connected.

## Evidence and resumption

[Owned evidence](../../../../plans/actions-confiance/paea/evidence/20261025-receipt-report-b9gi7a2v/):
plan/options, originals, pre-change public witness, commands, diff and byte-bound
[final review](../../../../plans/actions-confiance/paea/evidence/20261025-receipt-report-b9gi7a2v/source-review.json).
Formatting/syntax parsing and static schema assertions establish no compilation,
execution of seven tests, build graph or independent review. Actual status is
retained in the evidence directory.

Final integrity checking refused two stale hashes after concurrent writes. The
statically bound sources were reconstructed from originals/diff and SHA-checked:
only generic CLI USAGE help and another workstream's FR/EN note changed. No parser,
PAEA dispatch or receipt checker changed through this drift. Originals/refusal/
diffs retained, foreign additions preserved; syntax/metadata reexamined against
new bytes. This grants no validation of that concurrent workstream.

PAEA-OBS-01: implemented in sources, effect **NOT TESTED**. Owner Codex resumes
after the instruction is lifted: compile affected targets, verify seven new tests
and legacy outputs, Rustdoc and recipe on the same candidate, then supported local
delivery. Increment-09 proof remains valid for its binary/archived inputs; new
sources are not that build's inputs. Protected PA-02/03, E2/E3, install and final
RETEX remain open. Hashes create no new authority.
