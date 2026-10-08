# Business prerequisites and control recovery

These changes follow the billing benchmark audit of 7 October 2026. The candidate
starts from `main` (`cbf0075`), rather than the benchmark's historical binary
(`2ef5330`, modified sources). Historical results remain unchanged.

## Guard execution before launch

The operator may declare ordering rules between work requirements:

```json
{
  "schema_version": 1,
  "event_id": "billing-create-1",
  "expected_revision": 0,
  "title": "Billing",
  "objective": "Prepare and settle authorized invoices",
  "scope": "Isolated synthetic benchmark",
  "criteria": ["Approved batch", "Exact, unique settlement"],
  "requirement_prerequisites": {"req-2": ["req-1"]}
}
```

Create with `swarm --root PROJECT work create --input work.json`; inspect with
`swarm --root PROJECT --json work show ID`. Public work operations expose the
same JSON contract. Configure rules before creating tasks. `work update` cannot
change or remove them afterwards. Cycles, duplicates and unknown requirements
are rejected. Works without rules retain their existing behavior.

In a hierarchical mission, a task mapped to `req-2` waits for **another accepted
task with fresh evidence** covering `req-1`, even when the planner omits
`depends`. A `waived` override does not qualify. Scheduling, manual agent launch
and the public `running` transition enforce the rule before reserving an
attempt. With configured rules, tasks without declared requirements cannot run.

**Limit:** the engine checks declared classifications; it cannot infer that an
arbitrary command makes a payment. The operator must control requirement
mapping, tools and business-service permissions. The payment service must
transactionally enforce authorization, batch digest, idempotency and accounting
invariants. A launch guard does not prevent concurrent changes during execution.
These changes do not qualify Swarm as a production financial system.

## Stale evidence

An accepted dependency whose evidence files change holds the dependent branch.
Its owner receives a `dependency_stale` event even while the scope is already
open. Identical proof drift produces one durable event instead of rewriting the
work revision on every poll. Accepted status and history are preserved.
Revalidate existing results through public operations; notification is neither
fresh evidence nor acceptance.

## Separate unavailable controls from nonconforming results

An operator-authorized control may specify availability failure codes:

```json
{
  "id": "check-lot",
  "command": ["python3", "check_lot.py"],
  "criteria": [1],
  "justification": "Checks the batch against the available ledger.",
  "timeout_seconds": 15,
  "environment_exit_codes": [3]
}
```

Codes must be unique integers between 1 and 255; zero can never represent an
environment failure. Authorize the policy through `validation preview/apply` or
planning controls. The receipt retains `environment_failure: true`, the exit
code, output and digests. Validation remains blocked, without automatically
launching another producer. Undeclared codes retain their objective-failure
meaning. Correct classification is the control author's responsibility.

After actually repairing and checking the service, use
`validation recheck-preview/recheck-apply` on **the same completed attempt**.
To keep an identical policy after an identified environment failure, supply
`environment_recovery_reason` (8–500 characters), `recheck_completed: true`,
`intent: "replace"`, the current policy and current revision. Apply the exact
returned `preview_token`. The reason records the operator's assertion; it does
not independently verify service recovery. Controls must pass again and current
review/gates remain required where configured. Previous failure receipts stay
available. No identical automatic retry loop is introduced.

## SQLite contention

Current `main` already reserves the writer before reading mutation state.
Bounded transaction recovery additionally covers SQLite BUSY (base code 5,
including BUSY_SNAPSHOT), preserving the same event, request and expected
revision. Replaying an already recorded event cannot reserve a review or
planning decision twice. Revision conflicts and other errors do not become
successes. Persistent locks ultimately return an error.

Defaults live in `config/storage-retry.json`. CLI and server read the local
`.swarm/storage-retry.json` override:

```json
{"schema_version": 1, "busy_retries": 3, "busy_retry_delay_ms": 25}
```

`busy_retries`: 0–10 additional attempts; delay: 0–1000 ms. The existing
connection busy timeout may apply on each attempt. Agent preparation uses the
same configuration. Replace the override atomically; symlinks, unknown fields
and invalid values fail closed. Configuration is file based; **this change does
not add an administrator form**. Provider budgets and production-attempt
ceilings are unchanged.

## Audit disposition and benchmark

| Finding | Disposition |
| --- | --- |
| D1: attempt-workspace report | Existing main protection, tested without a root copy |
| D2: silent stale dependency | Visible reason and owner event; evidence history preserved |
| D3: infrastructure interpreted as business failure | Declared codes and explicit existing-result revalidation |
| D5: planning/review contention | Existing early writer reservation plus bounded idempotent recovery |
| D6: planner omits dependencies | Operator requirement rules enforced before launch |
| D4: abrupt exit 137 | Explicit recovery preserved; no blind restart |

The benchmark now rejects missing, non-object or incomplete client JSON.
Valid client output still does **not** establish business success. On execution
error, previously reported charges and calls with unknown costs are recovered
from the usage journal. Valid lines in partially corrupt journals survive;
the campaign stops if usage cannot be recovered completely. Historical results
are neither recalculated nor replaced. Scripts and fixtures do not demonstrate
real-provider autonomy.
