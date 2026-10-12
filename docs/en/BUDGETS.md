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

Defaults for future missions are not included yet. Installation does not change
existing authorizations.

## Versioned model pricing

Choose **Model pricing** in **AI budgets and costs**. Add a provider, model,
three-letter currency, source, reference date and prices per million tokens.
The catalogue is project-wide. Blank means unknown; explicit zero means free.
No prices are prefilled or fetched automatically.

**Create a new version** retains previous prices. Concurrent edits are rejected.
**Estimate with this price** requires non-cached input, output, cache-read and
cache-write volumes separately. Missing prices for used categories yield an
unknown amount, not a partial total presented as complete. Do not include cached
input in the non-cached input count.

```sh
swarm pricing list --json
swarm pricing save --input price.json --json
swarm pricing estimate --input volumes.json --json
```

Example price (illustrative values):

```json
{"schema_version":1,"expected_digest":"DIGEST_FROM_LIST","rate":{"provider":"my-provider","model":"my-model","currency":"USD","input_per_million":2,"output_per_million":5,"cache_read_per_million":null,"cache_write_per_million":null,"source":"Documented local estimate","reference_date":"2026-09-23"}}
```

Example volumes:

```json
{"rate_version":"VERSION_FROM_SAVE","non_cached_input_tokens":1000000,"output_tokens":1000000,"cache_read_tokens":0,"cache_write_tokens":0}
```

This yields an estimated USD 7. The JSON includes the exact price version and
volumes, so later catalogue updates cannot change its meaning. Save the output
when needed as evidence; simulations are not recorded as mission expenditure.
No currency conversion is performed.

This catalogue does not automatically price historical provider events or alter
per-launch reservations. Verify the rates you enter. Subscriptions, tiers, cache
retention durations and discounts require appropriately selected rates; they are
not inferred. Prior versions cannot be deleted through this interface; further
writes are refused after 1,000 versions.

## Engine settings

In **Admin**, open **Engine settings**; the same dialog is available in
**AI budgets and costs**. The compact table separates current usage from limits;
history and local allocations remain available on demand. Enter settings and a
reason, preview, then authorize. Editing invalidates the preview; stale revisions are rejected.
Consumption, history, failures and pauses are preserved.

These persisted policies share one source of truth across web, API and CLI.
No second fixed ceiling of 100, 200 or 500 is applied in the engine or form.

| `limits` field | Meaning |
| --- | --- |
| `planning_tasks` | Maximum tasks; positive integer, at least the current count |
| `planning_activations` | Global activations; positive integer, at least consumed |
| `planning_decisions` | Global decisions; positive integer, at least consumed |
| `review_calls` | Configured reviewer calls; positive integer, at least consumed, or `null` without a reviewer |
| `planning_scopes` | Maximum scopes; zero means no extra ceiling, otherwise at least the current count |
| `delegation_depth` | Maximum levels; root counts as level 1, zero means no extra ceiling, otherwise at least current depth |
| `judge_cycles` | Maximum judge cycles; zero means no extra ceiling, otherwise at least the current cycle |

Omitted optional task/scope/depth/cycle fields preserve their current value. Zero
does not remove call budgets, permissions or other checks. Subplanner allocations
remain unchanged and still apply. Active planner reservations and reviews prevent
changes until released. `GET /api/v1/quotas` exposes values, consumption, revision
and latest authorization; `remaining` is `null` for an unlimited dimension.
Technical bounds remain distinct from policies; integers must be representable
without precision loss.

```sh
swarm quotas show WORK --json
swarm quotas preview WORK --input quotas.json --json
swarm quotas apply WORK --input quotas.json --json
```

Example request; use the revision and consumption returned by `show`:

```json
{"schema_version":1,"event_id":"quota-authorization-001","expected_revision":12,"limits":{"planning_tasks":30,"planning_activations":60,"planning_decisions":40,"review_calls":20,"planning_scopes":0,"delegation_depth":0,"judge_cycles":0},"reason":"Explicit authorization after reviewing remaining work"}
```

Without a configured reviewer, `review_calls` must be `null`. This operation does
not create a reviewer. The engine records the operator and time. Exact event
replays are idempotent; concurrent stale changes are rejected.

Saving does not directly launch an agent. An active mission may use the new
allowance on its next conductor pass. Failures remain failures and pauses remain
in effect. Production retries still require task-level reauthorization with
corrective instructions; this form does not bypass that protection.

```mermaid
flowchart LR
  A[Web or CLI: limits and reason] --> B[Read-only preview]
  B --> C[Explicit authorization]
  C --> D{Revision and consumption compatible?}
  D -->|No| E[Reject and preserve state]
  D -->|Yes| F[Record limits and event]
  F --> G[Conductor still checks all other conditions]
```

### Adjusting a planning scope

The `quotas preview/apply` request accepts optional
`"scope_activations":{"preuve-livraison":21}` alongside `limits`.
Only named scope limits change, following human authorization. Consumed counters
are preserved, and before/after limits are recorded with the actor and reason.
Unknown scopes, limits below consumption, and limits above the parent/global
limit are rejected. Explicitly bounded parents must be raised in the same request
when needed. Active planning leases and reviews still prevent quota changes.

## Browser timeouts are separate from budgets

The **Browser request waiting time** Admin dialog configures general API calls,
snapshots and connection tests. `0` disables interface expiry; agent budgets and
server execution deadlines remain separate. Preview, save, history and CLI use
the shared Go contract. See [units, effects and limits](WEB-REQUEST-POLICY.md).
