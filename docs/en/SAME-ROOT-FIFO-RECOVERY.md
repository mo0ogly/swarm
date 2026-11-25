# FIFO across three missions and conductor recovery

[Français](../architecture/SAME-ROOT-FIFO-RECOVERY.md) · [Swarm architecture](ARCHITECTURE.md#12-concurrency-across-missions-in-one-root)

P1.2 qualification on 22 November 2026. Three Work objects share one isolated
Root. The Web conductor, protected routes and mission leases are real. Planners
and workers use inert local programs. Actual cost is zero. Estimated
reservations are fixture values.

Exercised image: `v0.1.1-dev.389`, declared commit `50f1daee`, `modified:true`,
SHA-256 `2fa03ed1ffe14d9243f681027085d0e6f61cbc018fd3c4102e85128f00a9567c`.
The qualification is bound to this image and the archives below.

## Control and conductor interruptions

| Journey | Precondition | Observed effect |
| --- | --- | --- |
| Control without a crash | A holds `shared` on turn 1. B requests `shared/nested` on turn 2. C uses `separate` on turn 3. | B waits. C starts while A is running. After turn 1 releases, B acquires turn 2. |
| Cooperative stop | A and C workers run. B waits for A. | `SIGINT` stops the fixture server. A and C survive. After reopen and A’s release, B acquires its original turn while C continues. |
| Crash between transactions | A and C are active, and B is still waiting. | The conductor receives `SIGKILL` and exits `-9`. Reopen finds the same position and blocker. The new conductor claims B’s generation 2 lease after lease expiry. The fixture saves a 30-second value, which establishes no guaranteed recovery time. B acquires turn 2 on A’s release, with one Agent. |
| Crash before admission commit | A has released turn 1. C is active. B has entered the admission transaction. | The breakpoint is reached after writing the acquired turn and the `acquired` journal event, inside `agent_launch::stage`, before `COMMIT`. Only the fixture conductor receives `SIGKILL` and exits `-9`. |

## Durable readback and recovery after the admission crash

The tested image reaches `workspace_turn_write::acquire+0xec4`. The `crash/debugger.log` records this point after the `acquired` event was written. The Agent insert and the budget reservation follow this return in `agent_launch::stage`. The offset is specific to the tested image.

Before recovery, A is released and B is still `waiting` on turn **2**, with
its original request time. B’s journal only contains `enqueued`. B has no
recorded Agent or reservation. C remains active. These observations establish
that the interrupted acquisition at the tested point left no durable effect.

After reopen, B’s new conductor claims its generation **2** lease. B acquires
the **same turn 2** and keeps the original request time. C retains its PID and
Agent during B’s admission. The final readback has three released turns and
no blocker. B has exactly `enqueued → acquired → released` on turn 2, with the
same Agent for acquisition and release, and one estimated reservation.

This scenario qualifies the window **before COMMIT** on this image. The final
states describe fixture attempts and queue.

## During COMMIT and before worker start after COMMIT

On the same image, a control run without interruption finished. For the
interruption during COMMIT, a page write had succeeded and SQLite had a
journal. The first read-only probe refused the hot journal. Normal restart
found B waiting on turn 2 at the original request time, with no durable Agent
or reservation. With a generation 2 lease, B acquired and released the same
turn with one estimated reservation. C retained its Agent and PID. All three
turns were released.

After COMMIT returned successfully and before worker start, readback found
B queued with its acquired turn, Agent and reservation. The control run
finished. After the interruption, B’s worker exited with code 2 on restart
and the conductor stopped while preserving the active intention. Full
recovery remains open. Worker loss, fairness under load and cross-Root
scheduling remain open.

The catalogue binds the observations to records in
`fifo-commit-crash-20261122.VHnoWq` and preserves the earlier failure in
`fifo-commit-crash-20261122.AWWUvx`.

Read-only inspection found the Agent still queued after adoption from
generation 1 to 2. The Work revision is 9, the frozen launch expected 8, and
the adoption expected 9. The conductor receipt retained exit code 2.
The child supervisor’s stderr goes to `/dev/null`. Its internal cause
remains open.

Reproduction through `mission_watch_restart_seed` needs the SDK and normal
binary built from matching sources. Preflight found 11,080,065,024 available
host bytes for a configured reserve of 21,474,836,480. The Toshiba target was
admitted and the host reserve is preserved. Records are in
`p12-sdk-build-20261122.mnSQZQ`.

## After-COMMIT diagnosis and successful witness

On the same v0.1.1-dev.389 image, replaying the crash with child supervisor
stderr captured produced `agent-execution-preflight-expired`. B's preflight
was valid for 30 s, also the mission lease staleness threshold. In the
original receipts, generation 2 was acquired 1.056 s after preflight expiry.
In the diagnostic replay, it was acquired 1.621 s after expiry. The
timestamps and trace are in `p12-historical-diagnostic-20261122.ECyyHg`.

A separate isolated Root exercised the same cut with a 60 s preflight and
the same 30 s lease threshold. B resumes its original Agent, attempt, turn
2, timestamp and reservation. The journal records one adoption from 1 to 2
and one estimated settlement. C keeps its Agent and PID. All three turns are
released. The 60 s duration is a fixture input. The expired-preflight path
continues to refuse.

A separate v0.1.1-dev.390 witness uses the matching SDK to produce the
committed unstarted Agent. The first SDK case refused with
`agent-execution-preflight-changed`. Captures at both process boundaries
isolated `TMPDIR` among the allowlisted variables. The fixture SDK call now
uses the effective Root setting as the CLI does. With that setting aligned,
B's adoption preserves its Agent, attempt, turn and reservation. C keeps
the same process. The witness uses a confinement-compatible file observation
and retains the historical socket fixture failures.
The archive is `p12-queued-restart-20261122.ldo3Yj`.

The historical 60 s witness established recovery within that validity window.
The observation beyond it led to an adoption-bound preflight receipt restricted
to the original scope.

## Preflight renewal at adoption

On Linux, for a never-started automatic Agent, the adoption transaction can
retain a fresh preflight observation after the original receipt expires. It
checks the effective lease, current consent, revision and the original preflight
scope. Provider, model, workspace or environment drift refuses the operation.
The original receipt, Agent, attempt, turn and reservation retain their identity.

The v2 receipt stays tied to the adoption event. The supervisor and the final
helper check its freshness before startup. Replaying an adoption reuses the
recorded receipt with its original expiry. A v1 link keeps the original
preflight. The historical observations above keep their individual scope.

The [expired preflight recipe](../../tools/verification/recipes/mission-watch-expired-preflight.json) waits for the original receipt's recorded expiry. It checks the renewed receipt, C's continued execution and single settlement on isolated Work.

## Evidence originals

| Archive from 22 November | Main record |
| --- | --- |
| `same-root-fifo-20261122.schtsm` | `06-persisted-settlement.json` and `04-sigint-worker-survival.json` |
| `same-root-fifo-sigkill-20261122.gE9V3k` | `06-persisted-settlement.json` and `readback.json` |
| `fifo-admission-crash-20261122.1H4Abh` | `control/08-persisted-settlement.json`, `crash/05-transaction-interruption.json`, `crash/06-crash-durable-readback.json`, `crash/07-b-original-turn-and-c-continuity.json`, `crash/08-persisted-settlement.json` and `crash/debugger.log` |

[Hashes and scope](../diagrams/architecture-contracts.json) ·
[Queue writer](../../rust/crates/swarm-engine/src/workspace_turn_write.rs) ·
[Admission transaction](../../rust/crates/swarm-engine/src/agent_launch.rs)
