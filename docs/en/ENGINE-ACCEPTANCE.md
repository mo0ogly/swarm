# Engine contract acceptance

Run these commands from the Swarm repository. Each uses disposable Stores and
repositories; they do not modify the mission open in the cockpit.

| Case | Command | Contract checked |
| --- | --- | --- |
| E1 | `node tests/engine_acceptance.cjs --case launch` | Launch and acceptance review guards; evidence freshness |
| E2 | `node tests/engine_acceptance.cjs --case ownership` | Requirement ownership, delegation, isolation and owner handoffs |
| E3 | `node tests/engine_acceptance.cjs --case revision` | Checks and review on the same revision; stale or changed evidence rejected |
| E4 | `node tests/engine_acceptance.cjs --case recovery` | Interrupted execution, preserved counters, verdict reuse and concurrent ownership |
| E5 | `node tests/engine_acceptance.cjs --case truth` | CLI/web evidence and states; actual browser in French/English, both themes, desktop/mobile |
| E6 | `node tests/engine_acceptance.cjs --case real` | Deterministic defect/refusal/correction/acceptance journey and scope closure |

The historical name `real` does **not** mean a real model is called. Agent decisions
and processes are simulated. These commands consume no model calls and do not
establish end-to-end autonomy.

Requirements: Go, Node.js, repository dependencies, and Chrome/Puppeteer for E5.
The runner rejects missing tests, skipped tests, failures and incomplete results.
`go test ./...` alone skips the E5 browser recipe and does not replace these six
commands.

Record the repository revision, any uncommitted diff, each command and its result.
For live-agent trials, also freeze the engine binary hash and storage schema.
Keep the same engine throughout, record calls per role and every external
intervention. A manual repair remains an intervention; deterministic success
cannot automatically accept a live mission.

Separate preparation: [controlled business-fault trial](CONTROLLED-AUTONOMY-TRIAL.md).
