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

The witness covers the window **before COMMIT** on this image. A crash during
COMMIT or after it succeeds, a lost worker, fairness under load and cross-Root
scheduling remain open. The final states describe fixture attempts and queue.

## Evidence originals

| Archive from 22 November | Main record |
| --- | --- |
| `same-root-fifo-20261122.schtsm` | `06-persisted-settlement.json` and `04-sigint-worker-survival.json` |
| `same-root-fifo-sigkill-20261122.gE9V3k` | `06-persisted-settlement.json` and `readback.json` |
| `fifo-admission-crash-20261122.1H4Abh` | `control/08-persisted-settlement.json`, `crash/05-transaction-interruption.json`, `crash/06-crash-durable-readback.json`, `crash/07-b-original-turn-and-c-continuity.json` `crash/08-persisted-settlement.json` and `crash/debugger.log` |

[Hashes and scope](../diagrams/architecture-contracts.json) ·
[Queue writer](../../rust/crates/swarm-engine/src/workspace_turn_write.rs) ·
[Admission transaction](../../rust/crates/swarm-engine/src/agent_launch.rs)
