# Native lifecycle inventory

Candidate GET/HEAD /api/v1/lifecycle/missions and CLI lifecycle list.
The array orders active/archived Works by descending rowid, then trash metadata
by descending deletion time. Fields: work_id, title, revision, state, state_label,
and nonempty changed_at. An empty archive timestamp still means archived.
Trash metadata proves neither valid snapshot files, restorability nor permission.
No private recovery snapshot is exposed.

One SQL snapshot, shared settings reservation and physical Root checks bind the
observation. A malformed Work or SQL/finalization/close error rejects the complete
read, never an empty fallback. HTTP refuses bodies and extra query parameters.
CLI and HTTP share the same reader. No mission/archive/trash/consumption/visit
mutation, provider execution or lifecycle action follows this read.

The explicit FR/EN disclosure renders literal text and exact large revisions,
retaining previous observations after refusal. No automatic fetch, hidden page
cap, retry or new operator policy. Memory grows with retained metadata. Candidate
status; evidence/delivery remain in the migration plan, not global acceptance.

~~~mermaid
sequenceDiagram
    actor User
    participant UI as Explicit inventory
    participant HTTP as Protected API / CLI
    participant Root as Bound Root
    participant SQL as Read snapshot
    User->>UI: Read inventory
    UI->>HTTP: GET lifecycle/missions
    HTTP->>Root: Validate input and binding
    Root->>SQL: Works + archive metadata + trash metadata
    SQL-->>Root: Ordered observations
    Root-->>HTTP: Complete view or preserved error
    HTTP-->>UI: Literal rows and exact revisions
~~~
