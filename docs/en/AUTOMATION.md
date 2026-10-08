# Durable automation

## Service currently available in the engine

The engine has an internal service for durable resume requests, time schedules,
and local HTTP reception of external events. Program CLI and web administration
belong to C04.

A C01 request contains:

- a stable idempotency key selected by the caller;
- a bounded source, an existing target mission, and the sole
  `request_resume` action;
- a SHA-256 digest of the canonical content;
- distinct request and occurrence identities.

Transactional `Store` tables are the single authority. Replaying the same key
and content returns the existing record. Reusing the key with different content
returns `idempotency_conflict` and creates no second occurrence.

## Authorization, claiming, and waiting

Reception requires explicit local mission authorization. Claiming revalidates
that authorization, autonomy, pause state, planning ceilings, and whether the
target is closed. A closed mission is durably rejected with `terminal_target`;
no mission or attempt is created.

An occurrence is claimed by one conductor under a lease. The current versioned
setting is `version: 1`, with a default `claim_lease_seconds: 30` and an allowed
range of 5 to 300 seconds. An occupied workspace leaves the occurrence in
`waiting` with reason `workspace_wait`, the actor, and next action
`retry_after_resource_release`, without consuming an attempt.

## Restart recovery

The `request_resume` effect is a durable engine signal with an identity derived
from the request and stored in the mission database. If the process stops after
the effect but before occurrence completion, restart finds that identity and
completes the same request without applying the effect again. If effect presence
or absence cannot be established, the service retains `uncertain_effect` and
requires operator reconciliation; it does not promise exactly-once execution at
an external provider.

The `executed` state means that the request was processed. It does not mean the
mission is complete or its result has been accepted.

## C02 time schedules

A schedule always targets an existing mission and the sole `request_resume`
action. Supported forms are `once`, `daily`, and `weekly`. Recurrences must
provide both `until_local` and `max_occurrences`, so they cannot be infinite.
The timezone must be an explicit IANA name (`UTC` is accepted and `Local` is
rejected), while previews return UTC instants.

The DST policy is deliberately strict: nonexistent or ambiguous local times are
rejected (`nonexistent_local_time` or `ambiguous_local_time`) instead of being
silently shifted or selected. If the clock moves backwards before the last
observed instant, the schedule is held and records `clock_moved_backward`.

Creation always yields the `disabled` state. Enabling, pausing, and archiving are
explicit revisioned operations; an archived schedule cannot be re-enabled. On
restart, `missed_policy=skip` records every past occurrence without a request or
burst. Concurrent occurrences targeting a mission that already has an active
request are coalesced into that request while retaining every source and reason.

Causal recovery requires a new SHA-256 evidence digest and checks that the
previous cause actually changed. It revalidates authorization, ceilings, and
workspace availability and rereads the provider cooldown. It changes no
attempt, budget, or cost and never clears a cooldown.

Operational settings are versioned in `config/automation.json`:
`claim_lease_seconds=30`, `due_grace_seconds=60`,
`max_preview_occurrences=20`, and `max_schedule_occurrences=366`. A local
`.swarm/automation.json` may use the same versioned contract; bounds are checked
before use. The engine installs no autonomous poller: an authorized host calls
the tick explicitly, which then submits through the C01 service.

## Schedule archiving and restoration

Trash retains schedules, journals, request origins and causal-recovery evidence;
in-place restoration preserves their states. ZIP archives containing automation
data use manifest version2 and require a compatible importer; version1 archives
remain readable. Export reads these tables in the same transaction as the work
and event history. Import rejects foreign targets/references, duplicate tables
or columns and unauthorized tables. Previously enabled schedules are imported
disabled: an archive grants no launch authorization. Identifiers and journals
are retained, and an existing work is never overwritten.

## C03 external events

The `POST /api/v1/automation/external` route is reachable only through the
existing loopback web server. It accepts a bounded JSON document containing
exactly `schema_version`, `event_id`, `target_work_id`, and the sole
`request_resume` action. `X-Swarm-Key-ID`, `X-Swarm-Timestamp`, and
`X-Swarm-Signature` are required. The signature is
`sha256=HMAC-SHA256(secret, timestamp + "\n" + exact_body)`: changing JSON
whitespace changes the signed content and conflicts when an `event_id` is
reused.

Keys are read from `.swarm/automation-external-secrets.json`, which must be a
regular local file with no group or world permissions. Each active key lists
its authorized missions and actions. Rotation and revocation are reread at
reception, claim time, and immediately before the effect; an old signature
never creates lasting authority. Never place this file, a real key, or a
received body in a report, command, public log, or repository.

The versioned policy defaults to `external_max_payload_bytes=65536`,
`external_rate_limit=60`, `external_rate_window_seconds=60`, and
`external_timestamp_window_seconds=300`. Their accepted ranges are 256..1048576
bytes, 1..10000 events, 1..3600 seconds, and 1..3600 seconds. The journal keeps
only identities/digests, outcome, and reason code—never the body or secret. An
accepted event means only that a durable request exists. Request processing,
mission outcome, and acceptance remain three separate states.
