# Native planning decision history

[French](../architecture/NATIVE-PLANNING-HISTORY.md). Native reader installed and verified in215; two English labels are being corrected. Candidate implementation;
recipe and installation evidence remain in the [migration plan](../plans/migration-rust/PLAN.md).

GET/HEAD /api/v1/planning-history?work=WORK and CLI planning history WORK return
the Go array of retained decisions in event revision order. Entries contain id,
at, scope, reason and operations. Bodies and extra parameters refuse. Native
requires an existing Work: unknown Work is404, whereas Go returns an empty array.

Work and events share a readonly SQL snapshot, shared settings reservation and
physical Root descriptor checks. Only planning.decide records enter this view.
Malformed payload refuses the entire list, never partial empty success. SQL,
decoding, finalization, rollback, close and physical revalidation retain safe causes.

The decoder reads the complete Go PlanningRequest data contract, accepting unknown
fields like json.Unmarshal. Known types, duplicate/case/null merge, empty arrays
and special dependency serialization survive. Historical data is not admitted as
a new command: no current schema, lease, budget or authority is adopted. Only the
five public fields are exposed, excluding the complete request and private guards.

The cockpit reads explicitly, renders literal text and preserves previous rows
after refusal. Historical integers remain exact beyond JavaScript precision.
Operations are never executed and declared ceilings never become current settings.

No row ceiling, deadline, retention, retry or new operator policy is introduced.
All retained history is read, using memory proportional to its content; existing
network settings still apply. Empty means no decisions in this observation, not
history completeness, planner identity or applied operations. Injected fixtures
are explicitly synthetic unapplied records, never fabricated acceptance.

~~~mermaid
sequenceDiagram
    actor U as User
    participant UI as History disclosure
    participant H as Protected API
    participant R as Bound Root
    participant SQL as SQL snapshot
    U->>UI: Read decisions
    UI->>H: GET planning-history with Work
    H->>R: Method, query and binding checks
    R->>SQL: Read Work and events
    SQL-->>R: Retained data
    R->>R: Decode decisions, never execute
    alt Unreadable data
        R-->>H: Original cause and checked rollback
        H-->>UI: Refusal, previous rows retained
    else Valid read
        R-->>H: Public array and checked close
        H-->>UI: Recorded decisions and exact integers
    end
~~~
