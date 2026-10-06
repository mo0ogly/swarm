# Revising a blocked hierarchical contract

A phase-ordering mistake can require closure before the acceptance that enables it. Recovery does not approve the result artificially: an operator explicitly revises the criterion text, then fresh checks and independent review examine the revised contract.

Public command: `swarm task update WORK --input revision.json`.

The request includes `schema_version`, `event_id`, `expected_revision`, `id`, `criteria`, `confirm_contract_revision: true`, `expected_contract` (the latest review's `contract` fingerprint) and `contract_revision_reason` (an explicit reason of 16–2000 characters). Each criterion retains its position; removing criteria is rejected.

Preconditions: a publicly paused mission, a blocked task, no active agent or reviewer, an open scope, and current revision and contract fingerprint. Managed integration repositories are not supported by this operation yet.

The amendment changes no identity, deliverable, dependencies, owner, global requirements, history, attempts, budgets or provider delays. It archives the previous review and invalidates old gates and automatic evidence. The task remains blocked until the existing result is resumed through public operations with fresh checks and review. The audit records the confirmation, previous fingerprint and reason; replaying the same event does not amend the work twice.

Finalization remains: independent review → fresh acceptances → public child/parent closure → supervised installation under its preconditions. This operation grants no acceptance, closure or publication authority.
