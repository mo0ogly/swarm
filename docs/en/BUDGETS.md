# AI budgets and costs

In **AI budgets and costs**, choose **Set budget**, enter a USD ceiling, an
estimated reservation per launch, its source and reference date. Preview the
change before saving it. Editing a field invalidates the preview. A stale form
is rejected: close it, refresh and review the current settings again.

A zero ceiling disables the estimated financial limit. Lowering the ceiling
neither stops active agents nor refunds existing estimates or reservations.
Other launch conditions still apply.

## CLI

```sh
swarm budget show WORK --json
swarm budget preview WORK --input budget.json --json
swarm budget apply WORK --input budget.json --json
```

Example request (illustrative amounts, not provider prices):

```json
{
  "schema_version": 1,
  "event_id": "budget-authorization-001",
  "expected_revision": 12,
  "budget": {
    "limit_usd": 10,
    "reserve_per_launch_usd": 2,
    "estimate_source": "Documented local estimate",
    "reference_date": "2026-09-23"
  }
}
```

Use the revision returned by `show`. Preview is advisory and does not write.
Apply checks the revision inside the transaction that records the authorization.
Concurrent edits cannot silently replace each other. Replaying the same event
with the same request does not consume another revision. The engine records
the operator and timestamp; submitted identity fields do not override them.

## Coverage and limitations

The estimate envelope covers execution, planning, page assistance and preparation.
**Independent reviews have a separate call limit; their monetary costs are not
included in this envelope.** Provider-reported costs remain separate from estimates.
Missing cost is unknown, not zero. This is not a guaranteed external billing cap.

Model pricing catalogues, defaults for future missions and a unified editor for
planning/review/attempt quotas are not included yet. Existing authorizations are
unchanged.
