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


## Local evidence and independent acceptance

The producer fills `docs/<task>.delivery.json` with its own observations: executed command, result and tracked evidence files. `pass` means its local checks cover the criterion; `complete` means its local delivery is complete. Neither value accepts the task. Configured control commands and their options are supplied in the prompt, without additional permission to change their definitions. A check that could not run remains `not_tested`.

The engine validates delivery attribution and completeness, executes its controls against the candidate revision, then requests an independent review. Producers and planners must not wait for these future steps before reporting actual local observations. A local `pass` cannot bypass a failed engine control or a rejecting review.

The real trial on September 23, 2026 exposed this confusion: two `not_tested` deliveries stopped the pipeline before engine controls. Instructions and diagnostics were clarified; this does not yet demonstrate a successful new real-model trial.

## Checks as evidence for human review

A `human` policy may include explicitly authorized `controls`. Under an active
mission authorization, the engine runs them and passes its receipt to the
independent reviewer. Success keeps the task awaiting review: it neither accepts
the task nor releases downstream tasks. Qualitative criteria may remain outside
automated coverage.

In “Configure validation”, keep human review, add checks, map their criteria and
list examined files. `inputs` paths are relative to the project root, even when
`dir` differs. The engine hashes these files and the report before checks and
rejects evidence if their contents change. Without `inputs`, only the report is
bound to the receipt; this does not prove that all source files stayed unchanged.

The CLI shares this contract: `swarm --json validation preview WORK --task TASK
--input policy.json`, then `validation apply` using the returned `preview_token`,
the same `expected_revision`, and a new `event_id`. Example `policy` field:

```json
{"mode":"human","controls":[{"id":"tests","command":["go","test","./..."],"dir":".","timeout_seconds":300,"criteria":[1],"justification":"The suite checks the first criterion's behavior.","inputs":["go.mod","engine.go"]}]}
```

Adapt actual paths and criteria before confirming. Changing policy archives the
old review and invalidates receipts; it is refused while review is active.
Current receipts are reused without rerunning unchanged checks on each conductor
pass. Undeclared files are not monitored.

Focused verification of this contract:

```sh
go test -run 'TestHumanControl|TestHumanPolicy' -count=1
go test -race -run 'TestHumanControl|TestHumanPolicy' -count=1
go build -o /tmp/swarm-human-evidence .
node tests/human_evidence_ui.cjs /tmp/swarm-human-evidence /tmp/swarm-human-evidence-ui
```

## Images in independent review

Operator-declared PNG/JPEG control inputs are attached to the Claude reviewer
only when their hashes match the current control receipt. Images must be
under `docs/screenshots/`; report links never authorize additional reads.
Limits: 20 images, 5 MiB per image, 10 MiB total, 20 million pixels per image.
Missing, invalid, oversized or stale files block review before a paid call.
Adapters without image support explicitly reject this review instead of
silently omitting the images.

The reviewer keeps its separate session, no editing tools and the usual
budgets/deadlines. Pixel inspection remains an AI judgment, distinct from
deterministic checks and acceptance. Changed image bytes invalidate the
related hashes. Transport uses the image blocks in the
[Claude streaming format](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode).

## Rechecking a completed result after a correction

A check that did not execute or was interrupted without an exit verdict does
not establish a repairable delivery defect and no longer automatically starts
a worker. A different receipt path does not make an unchanged failure new.
Diagnose and correct the relevant precondition before retrying.

For a task blocked by its checks, the explicit `validation recheck-preview`
then `validation recheck-apply` commands reuse the `recheck_completed` contract
with `"intent": "replace"` and a different,
corrected policy. The other request fields and the exact preview token remain
required. The latest producer must be completed and match the failed receipt.
This resubmits the **existing result**, invalidates current evidence, and creates
no worker, attempt, or additional budget. Checks require an active mission
authorization; independent review and a fresh gate still precede acceptance.
Removing checks or supplying an empty policy is not a recovery mechanism.
The web **Recheck result** action follows the same preview/apply sequence.

## Reading checks and their measurements

`swarm mission status WORK` and web task details use the same evidence
projection. The **current verdict** is shown before history, while prior
failures remain available after a recheck. Each check shows its state, result,
command, timestamps, and measured wall-clock duration.

CPU time is shown as measured only when the process reports it. Cost and tokens
remain `unknown`: Swarm derives neither from command counts nor duration. The
top five ranks only checks with a measured wall-clock duration; older receipts
without one remain in history but are not ranked. These are check-process
durations, not estimates of total mission elapsed time.

After acceptance, the engine automatically closes a non-delegated root only
while every task still has a fresh gate and a current independent review. A
historical incident remains available but does not require another planner
activation; stale evidence or a missing review keeps the root open. Delegated
graphs retain their explicit child/parent closure contract.

### Complete, observable final recipe

`python3 tests/supervision_final_acceptance.py` runs the Go suite, vet,
configuration checks, diff checks, canonical build, and the isolated browser
recipe. The Go runner discovers every test in the package and partitions the
inventory into four disjoint processes. Every discovered test must appear in
Go events and every group must exit 0. Empty or duplicate inventories, a changed
package count, and missing coverage fail the recipe. Subtests run with their
parent test. Go has a 240-second timeout per group; the **engine control retains
its total 300-second deadline**. No group is optional and no test is removed.
A local execution does not replace the engine's evidence receipt.

Browser recipes demonstrate their interactions on an isolated candidate server,
not external-provider autonomy or closure of the actual mission. Process exit
and report presence are not acceptance. Unreported costs remain unknown.

### Agent decomposition: proposed evolution

A validation supervisor could own engine, CLI/configuration, web, and
proof/retrospective groups. Subagents would investigate anomalies and qualitative
criteria; automated checks would remain deterministic processes. The supervisor
would enforce complete coverage of the same candidate, invalidate affected
groups after a correction, and hand the consolidation to an independent reviewer.
Only the engine would decide closure. **This agent organization is proposed;
the four-process test recipe does not implement it.**
