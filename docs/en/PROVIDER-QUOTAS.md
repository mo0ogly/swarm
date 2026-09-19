# Understanding an AI provider quota hold

[Français](../PROVIDER-QUOTAS.md) · [User guide](USER-GUIDE.md)

A quota rejection means the provider refused the call. It proves neither a defect
in the produced code nor task completion. An interrupted attempt remains
unvalidated; its files are retained for inspection.

## Engine behavior

The engine recognizes structured Claude provider rejections: `rate_limit_event`
with `status=rejected`, or a result with `is_error=true` and
`api_error_status=429`. Usage warnings, a number 429 in tool output and prose
inside reports do not create a hold.

The reset time comes only from `resetsAt` and is displayed in UTC. The engine does
not infer a date from text such as “resets 10:50pm”. Holds persist in the local
project state across restarts. New worker, planner and independent reviewer calls
using that provider are held before their budget reservation.

A call reserved before the rejection becomes known may be stopped before it
starts. Attempt and call counters remain consumed. An estimated financial reserve
for a process that never started may be released under the existing rules;
releasing it does not restore an attempt or raise any limit.

A completed production waiting only for review retains its candidate and checks;
the quota does not require repeating the worker. A review call that actually ran
and was rejected retains its failure and consumed call. Retrying that review must
be explicit and remain within budget.

## Inspect the actual cause

The status shared by the cockpit and CLI exposes `provider_cooldowns`, the
rejection, any known reset time and the next action. Agent and result counters
retain their own facts: a quota hold does not mean the mission is finished.

```sh
swarm --root /path/to/project --json providers cooldown show claude
swarm --root /path/to/project --json mission status WORK_ID
```

Expiry permits another evaluation of launch conditions; it does not guarantee
provider availability. A paused mission stays paused. A task at its attempt
ceiling remains blocked after the deadline.

## When no reset time was supplied

Check the account and provider access. Explicit clearance requires the current
`digest` returned by `show`, a unique event identifier and a reason describing
that check:

```json
{
  "schema_version": 1,
  "event_id": "account-check-001",
  "expected_digest": "DIGEST_RETURNED_BY_SHOW",
  "reason": "Account access checked; allow another provider availability check."
}
```

```sh
swarm --root /path/to/project --json providers cooldown clear claude --input check.json
```

The command records the reason and local operator. It refuses to clear a known
future deadline. It launches no agents, accepts no results and does not prove
quota recovery. If the state changed after `show`, read it again.

## Current coverage

Protection covers engine processes: workers, planners and the independent
reviewer. Page and preparation assistants are not yet connected to this hold.
It is shared by provider identifier within one project; it does not automatically
coordinate a shared account across several identifiers or separate projects.

Tests with simulated responses verify refusals, persistence, budgets and recovery.
They do not prove live account availability or an autonomously completed mission.
