# T14/I14 — agent-to-agent (A2A) exchange design

[PAEA plan](../../../plans/actions-confiance/PAEA-APEX.md) · [Version française](../../../../plans/actions-confiance/paea/T14-A2A-DESIGN.md)

**Status: design, not implemented, not verified.** This document addresses the
"missing design" row of T14 (invariant I14) in the dashboard. It grants no
permission, activates no protection and closes no invariant: I14 remains NOT
ESTABLISHED in local verification until the code described here exists and a
recipe proves it, and acceptance remains E3 scope (T18).

## Need and contract anchors

PAEA registry:

- I02 Identity/delegation — "Enrolment/current mandate → launch/peer;
  subset of delegable rights" (T03/T14).
- I14 A2A — "Peer/mandate/audience → linked reception and no privileged
  intermediary" (T14/T18).
- Abbreviated requirement: "authenticated A2A message/delegation, bound to a
  reduced mandate with preserved provenance; mTLS channel or signature depending
  on the model".
- PA-06 Composition — "Instructions/data separated; provenance, integrity/
  confidentiality and versions at read/summary/transmission/compaction; bounded
  descendant rights and authenticated peers". Expected verdict: "No instruction
  promotion nor right through memory, A2A, withdrawal or resume in the retained
  scenarios".
- Architecture (PAEA-APEX, target diagram): "The local channel authenticates
  the peer but derives no mandate from its UID alone"; the control and its keys
  are outside the agent domain, under a distinct account.
- T18 (E3) — multi-turn, tool renaming, source→target context, label loss,
  summary/compaction; oracle: "no instruction promotion and no effect outside
  the mandate".

Declared prerequisites and their real state:

| Prerequisite | State as of 2026-11-02 | Scope |
|---|---|---|
| Real identity (IA3 outputs, enrolment) | open, human | I02 first; authenticated real A2A waits for its output |
| `delegation` (schema `max_depth`) | admission closed: `max_depth != 0` → `trust-active-extension-not-ported` | the block stays inactive until contractual porting |
| Delegated mandates | non-empty `parent_mandate_id` → `trust-delegated-mandate-not-ported` (`trust/operations.rs:83`) | I02 bounds delegable rights; I14 circulates them between peers |
| `data_context` | `trust-advanced-admission-not-ported` (`trust/operations.rs`) | preserved provenance of messages, separate lot |
| Authenticated peers (local channel) | UID authenticated but no mandate derived from the UID alone (PAEA-APEX diagram) | identification authorizes nothing without a current mandate |

## Design decisions

D1 — **An A2A message is not authority.** Mere transport of a message between
two instances grants nothing: every capability stays held by the control
(authority daemon) and consumed at the existing engagement point. A message
without a valid current mandate is context (I03), never a right.

D2 — **Reduced mandate, strictly included.** A peer mandate reuses the existing
`mandate-create` chain (I3 admin holder, TTL, `policy_revision`, enumerated
actions, monetary cap, connector): the delegable subset is controlled by the
`delegation.max_depth` policy (0 = the current prohibition, kept by default).
Each mandate carries its parent and its depth; `max_depth = 0` refuses any
delegation, a greater depth is capped by the policy at creation time — not at
use time. The `active()` check at the engagement point remains the only
reference: stale revision, revocation or expiry refuse as today.

D3 — **Explicit audience.** Every A2A mandate names its audience: the named
peer subject (enrolled identity or local principal), never "any instance".
Reception verifies the audience on arrival: a message delivered to a peer other
than its audience is refused at read admission (typed code), not only at use.

D4 — **Provenance preserved, not re-qualified.** The message carries its
original labels (classification, intent, content digest); an intermediary does
not rewrite labels, it appends its routing trace. Unclassified or provenance-less
data stays unprivileged at reception (existing I03/I13 gates apply). No summary
or compaction produces content carrying more rights than its source.

D5 — **No privileged intermediary.** The channel elevates nobody: an A2A relay
that is not the mandate's audience can neither read content not addressed to it
nor engage an effect. The reception oracle is "reception is linked": the
engagement receipt cites the received message (digest) and the current mandate
verified at engagement, so the observer (PA-09) links message → mandate →
action → effect without trusting an agent's narrative.

D6 — **Channel per model.** Local (same machine, authority daemon): mutual
authentication by the existing daemon; the message signature is made by the
sender and verified at reception, the verification key coming from the control,
never from the message. Remote: mutual TLS (mTLS) with certificates issued by
the control; the exact form (mTLS, signature, both) is a decision recorded per
deployment profile, not a run-time choice.

D7 — **Typed refusals and explicit UNKNOWN.** Every failure of the chain
authenticate → current mandate → audience → depth → provenance produces a typed
observable refusal (`a2a-*`) with no effect and no ledger mutation; the sender
sees a delivery UNKNOWN, not an invented success. Recovery goes through public
operations (new mandate, new audience), never a bypass.

## Admission chain to implement (contractual order)

1. **Peer authentication** — a single identity source (local or enrolled, as in
   `trust/authentication.rs`); no mandate derived from the UID alone.
2. **Mandate read** — `active()` at the current revision/clock; a delegated
   mandate (parent + depth) checked against `delegation.max_depth`.
3. **Audience** — the mandate's subject matches this peer and this message;
   otherwise a typed refusal before any context is opened.
4. **Provenance** — message labels and digest verified (received bytes match the
   declared digest); non-conforming content refused, never rewritten.
5. **Bounded context** — the message enters the conversation as labelled data
   (instructions/data separated, PA-06); no instruction promotion from a
   message.
6. **Engagement** — every effect goes through the existing operation (`request`,
   `settlement`, …) with the current mandate re-checked at last use (I05); the
   receipt cites the message digest.

## Out of scope of this design

- Porting the `delegation` block (real activation of `max_depth`) — a distinct
  contractual lot, configuration admission first.
- Delegated mandate chains (`parent_mandate_id`) beyond the first level —
  bounded by I02 and the policy, not exercised here.
- T18 and any adaptive campaign — E3 scope, non-author team.
- Real peer identity — human enrolment (IA3), I02 prerequisite.

## Local verification criteria (once the code exists)

- A depth-1 peer mandate with audience A: A engages a bounded effect, B (another
  peer) is refused at reception (audience), an expired or revoked mandate refuses
  at engagement with today's code.
- A message without a valid current mandate is kept as labelled context and
  authorizes no effect; the oracle reads the ledger: no effect.
- A non-audience relay neither reads the content (existing BLP confidentiality)
  nor engages in its name.
- Revoking the parent mandate between reception and engagement: the effect is
  refused (last use, I05/I17).
- Every refusal is typed, observable, mutation-free; recovery stays public.
