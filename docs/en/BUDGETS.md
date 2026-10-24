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

Choose **Model pricing** in **Admin** or **AI budgets and costs**. Add a provider, model,
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
writes have no version-count policy. No business price or volume ceiling is
imposed. Amounts must be finite and unrepresentable calculations are refused.
CLI/API counts use nonnegative signed 64-bit integers; the browser requires exact
JavaScript integers. Existing format guards remain: 2 MiB catalogue files and
12,000-byte HTTP requests, refused without truncating or deleting history. These
are technical guards, not budgets or retention policy; paginated storage remains
outstanding before the catalogue reaches its format guard.

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

## Assisted discovery and distinct evidence

The [pricing-discovery skill](../../.claude/skills/pricing-discovery/SKILL.md)
prepares a proposal from opened official pages, recording retrieval date, units,
currency and applicability (platform, tier, cache, region). It bundles no prices
and schedules no refresh. Authorized application appends through Admin/API/CLI
and checks read-back; research alone does not mutate the catalogue. Sources and
conditions remain text in the current schema, without automatic authenticity or
applicability validation.

```mermaid
flowchart LR
    S[Opened official sources] --> P[Dated proposal and conditions]
    P --> V[Authorized application in Admin]
    V --> C[Versioned project catalogue]
    C --> Q[Simulation with explicit volumes]
    Q -. currently no automatic coupling .-> R[Estimated mission reservation]
    U[Provider-reported usage] --> T[All retained agent attempts]
    T --> B[Budget read]
    E[Read failure] --> F[Refusal with preserved cause]
```

The flat per-launch reserve is not automatically recalculated from a price.
An estimate is not an invoice; other currencies are not implicitly converted to
USD. Reported agent costs include all retained attempts, independently of recent
UI pagination. Read failures refuse the budget view as
`reported_cost_unavailable`, preserving an internal cause excluded from public
JSON rather than pretending the cost is absent.

The HTTP/Admin journey belongs to the Go engine. Native Rust catalogue and quote
commands now provide `pricing list|save|estimate` on an initialized private native
root; native HTTP/Admin and Go-root cutover remain outstanding. Compatible pricing
JSON does not authorize live root sharing between engines. The skill establishes
neither installed delivery nor authenticated actors. Financial reservations remain
separate.

```sh
swarm-rust pricing list --root NATIVE_ROOT --json
swarm-rust pricing save --root NATIVE_ROOT --input price.json --json
swarm-rust pricing estimate --root NATIVE_ROOT --input volumes.json --json
```

Requests use the same fields as the Go examples above. List returns the exact file
digest; supply it in `expected_digest` when appending. The engine replaces submitted
version/actor/time. CLI resources close before checked output/flush. Failure after
publication remains nonzero: re-read before recovery. Pre-rename storage failures
preserve the original; post-rename durability failures report
`local-publication-uncertain`, never rollback. Fault injection is absent from the
production profile. Source/applicability remain unauthenticated text.

Rust preserves the actual Go producer's initialized values: `{}` or `null` are
empty schema1 catalogues; `rates:null` preserves JSON null. These are decode rules,
not default prices. An empty file, invalid document or invalid retained price
refuses reads. Floating simulations follow Go and do not guarantee invoice-grade
decimal precision. See [migration evidence and limits](RUST-MIGRATION.md).

Finite quotes preserve the legacy Go floating evaluation order. Only an overflowing
intermediate product is rescaled; a nonfinite total refuses. A retained actual
legacy-Go witness pins this behavior rather than comparing only two changed ports.

Evidence71:445Rust tests,46default CLI tests,18targeted Go tests and101comparisons
on8476stable inputs. Catalogue/CLI verified; native HTTP/Admin remain unported.
Same-session Codex review; no server replacement.

## Editing in the local Rust cockpit
“Set budget” uses the existing dialog: USD amounts, source/date, a financial preview
without writes, then a second confirmation. Saving uses the current revision and
engine journal without changing reservations, consumption or active agents. A zero
cap means unlimited; missing policy leaves blank fields and adopts no amounts.
Known refusal, lost response and known save with failed read-back are distinguished.
These cases require a fresh read; no save is automatically retried. Price catalogue
and other commands remain separate connections. See the
[Rust route contract](RUST-MIGRATION.md#connection1468--actual-mission-budget-editing).

```mermaid
sequenceDiagram
    actor U as User
    participant I as Existing dialog
    participant H as Protected budget API
    participant E as Engine and bound Root
    U->>I: Open settings
    I->>H: GET policy and revision
    H->>E: Read without mutation
    E-->>I: Nullable policy and revision
    U->>I: Preview
    I->>H: POST preview
    H-->>I: Financial validation without writes
    U->>I: Confirm
    I->>H: POST apply, revision and event
    H->>E: budget.configure, CAS and journal
    E-->>I: Recorded Work
    I->>H: GET read-back
    H-->>I: Current state
    Note over I: Lost response or failed read-back: diagnostic and fresh read, no automatic replay
```

## Quotas in the Rust cockpit
The existing button opens the same grouped component and help as native Admin.
The selected mission is preselected only when actually listed with planning;
otherwise selection is explicit. The dialog never enables planning. Saved limits
and consumption come from `/api/v1/quotas`; preview/apply reuse existing engine guards.
Edits invalidate confirmation; reason, revision, usage and ancestor limits remain
engine-controlled. No automatic launch. Malformed views refuse before fields become
editable. Errors stay visible, without automatic retry; reselecting rereads the Work.

```mermaid
flowchart LR
    A[Existing quota button] --> B[Actual Work list]
    B --> C{Work has planning}
    C -->|No| D[Explicit selection, no activation]
    C -->|Yes| E[Read limits and consumption]
    E --> F[Preview without writes]
    F --> G[Confirm, engine guards and revision]
    G --> H[Read back saved values]
```
