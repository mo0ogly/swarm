# Machine incidents and mission recovery

A mission is complete only when its results are validated and its owner closes
the work. A running server or passing engine tests do not prove mission completion.

## Storage unavailable

The cockpit displays a global alert with **Storage diagnosis**. This local check
makes no AI call and does not query SQLite, so it remains available when mission
reads fail. **Check again** refreshes measurements without deleting any files.

```sh
swarm --root /path/to/project --lang en doctor
swarm --root /path/to/project --json doctor
```

`doctor` neither creates nor opens the database. It measures the `.swarm` and
process temporary directories. Exit status is 2 when a volume is unavailable or
below the safety margin; otherwise 0. JSON includes paths, available bytes and
inodes. A missing `.swarm` directory is not treated as healthy.

- Warn below 1 GiB available.
- Suspend new launches below 256 MiB or 256 available inodes.
- Failed measurements also suspend launches.
- Agent preparation, supervisor starts, planning activations and independent
  reviews check storage before starting a new call.
- The driver rechecks each cycle and resumes reconciliation and already
  authorized operations after recovery. Rejections, exhausted limits and pauses
  still apply.
- ENOSPC and SQLite FULL map to `storage_unavailable` and HTTP 507. They never
  mean that a command succeeded.

This is a preflight check, not an allocation guarantee. Running agents can fill
a volume between checks. Remote providers and external workspace volumes are
outside these measurements. Identify disk usage before cleanup; rebuildable
compiler caches can be cleaned with their tools. Reports, databases, failed
working copies and history are not caches. Check public state after interrupted
writes before resuming.

## Exhausted attempts

**Mission blocked — attempts exhausted** appears when no task is running and a
blocked task has used its allowance. **Inspect attempts and rejections** opens
retained facts, the latest independent review and links to task reports.
**Save the diagnosis** downloads JSON containing the revision, task, attempts and
review. This flow does not call an AI provider.

`swarm mission status WORK` shows the same facts. JSON task fields include
`attempts_used`, `attempts_allowed` and `attempt_limit_reached`.

After three attempts, **Prepare one corrective attempt** opens a proposal based
on the latest independent rejection. Explicit confirmation authorizes one extra
attempt with history and independent review preserved. See
[bounded recovery](ATTEMPT-RECOVERY.md) for requirements, limits and the CLI.
If unavailable, the action explains the reason; an earlier exceptional grant,
a missing or running review, or an exhausted reviewer budget cannot be bypassed.

This recovery capability alone does not prove a real mission will succeed.
Completion requires an effective correction and its evidence.
