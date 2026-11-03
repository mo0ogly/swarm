# Exact separated fictitious-payment agreement — P1/G1

[Français](../architecture/PAEA-PAYMENT-AGREEMENT.md) · [tracking](../plans/actions-confiance/paea/P1-G1-20261103.md)

Status: implementation under qualification; this slice establishes neither real
human enrollment nor globally qualified protected effect services.

Existing persisted Trust require_separation remains the sole operator policy,
with its existing validation and Admin/API/CLI interfaces. No new policy is added.
When true, fictitious settlement requires a durable separated Trust approval for
protocol resource `paea-payment/<full canonical digest>`. Existing
mandate-create → request → approve produces this agreement. Amount, currency and
beneficiary come from canonical data and are independently checked. The digest
includes invoice version, context, sources and policy; changes require new approval.

The `paea settlement-call` preparation event exposes approval_resource as data,
never authority. Operators prepare exact terms using existing Trust operations
outside the consuming process. Current signed personal/business authority remains
required; this approval never substitutes for it or creates identities/grants.

At last use, while retaining project/Trust reservations, the engine verifies the
chain/originals, current revision, approved action, distinct actors/current roles,
admitted request, active mandate and exact canonical binding. Mandate expiry is
rechecked before SQLite commitment; this is not real-time assurance between the
check and COMMIT. Advanced role-matrix separation remains explicitly refused.

Receipts add optional agreement: action_id, mandate_id, requester, approver,
action_digest, evidence_hash, original_digest, policy_revision and expires_at.
CLI results retain this reference. It is local audit data, not a capability,
external signature or evidence of human attention. Reads reject inconsistent
agreement metadata. Old receipts without agreement remain readable and establish
no separation. Older binaries rejecting unknown fields cannot read new receipts;
retain compatible images when planning rollback.

Missing/stale/revoked approval or changed contents refuses
paea-settlement-attributed-admission-required before any new effect.
Evidence/receipt corruption preserves typed causes, never successful empty data.
Reconciliation remains a separately authorized read, not new commitment.
Revocation cannot undo a recorded effect; late failures retain known/uncertain
observations under existing contracts.

Fixtures do not replace real human enrollment, protection of keys/services from
the agent domain or E3 assessment. Context fields remain declared: exact binding
does not establish their provenance.
