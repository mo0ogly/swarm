# Swarm technical reference

[Overview](../../README.en.md) · [User guide](USER-GUIDE.md) ·
[Installation](INSTALL.md) · [Preparation and revision](PREPARATION-UX.md)

Swarm is a Go companion that retains work across agent sessions. It provides
French and English web/terminal interfaces, Markdown views and versioned JSON
contracts. `work list` and `resume` do not start agents or select work implicitly.

## Standalone repository and build

Use `--root` to target a project independently of the Swarm source repository.
Linux and Go 1.24+ are the tested scope. SQLite is embedded through
`modernc.org/sqlite`; no separate SQLite server or CGO is required. Dependencies
are pinned in `go.mod` and `go.sum`. The first build downloads dependencies; the
compiled application can run offline. Model providers may still require network
access. Web resources are embedded in the executable. Node 22/npm rebuild them.

```sh
make build
./bin/swarm --root /path/to/project init
./bin/swarm --root /path/to/project providers init
./bin/swarm --root /path/to/project web 127.0.0.1:18787
make test
make frontend
make smoke
```

Open the session link printed by the server. APEX, KS and PDCA method resources
belong to the controlled project; the standalone repository does not bundle
private project methods, `.swarm/` state, missions or keys. Installed agents
have their own authentication. Custom API connections use **AI and connections**.

## Language and machine-readable contracts

Web: select Français or English; `?lang=en` also selects and remembers English.
CLI: `swarm --lang en help`, or `SWARM_LANG=en`. The explicit option wins.
French is the default. Language affects presentation, not JSON payloads, enum
values, commands, IDs or stored user content. In particular, do not translate
`besoin`, `simple`, `standard`, `exigeant` or slash commands in requests.

`--input -` reads stdin. `--json` returns machine-readable output without display
translation. Common work mutations require `schema_version: 1`, a unique
`event_id`, and `expected_revision` (0 for creation, current revision thereafter).
Preparation has a distinct `version: 1` envelope; see its dedicated guide.
Dates are UTC. Identifiers use ASCII letters, digits, `_` and `-`.

```json
{
  "schema_version": 1,
  "event_id": "create-api-1",
  "expected_revision": 0,
  "title": "Update the API contract",
  "objective": "Client and server share the same schema",
  "scope": "Contract, client, server and associated tests",
  "criteria": ["Contract tests pass on current files"],
  "next": "Define the contract task"
}
```

```sh
swarm work create --input work.json
swarm work list
swarm work show WORK --json
swarm resume WORK
```

## Task contracts and revisions

`task add WORK` takes `id`, `title`, `deliverable`, `criteria`, `owner`, optional
`depends` (existing tasks in the same work), and `next`. `task update WORK` takes
`id` and the fields to change; `status` is optional. Contract fields include
title, deliverable, 1–12 criteria, dependencies, `max_attempts` (1–3) and
`max_tool_calls` (1–100). Missing/null lists stay unchanged; `depends: []` clears
dependencies and `criteria: []` is rejected. Empty strings stay unchanged;
whitespace-only text is rejected.

Contract edits require a `todo` or `blocked` task, no simultaneous status
transition, no active agent in the work and no running task. Unknown, repeated or
cyclic dependencies are rejected atomically. Actual contract changes invalidate
gates, overrides and revalidation; plan checks are recalculated. Old plans,
attempts and events remain historical records. The task is the current contract.

`work update WORK` changes title, objective, scope and criteria under the same
revision envelope. All tasks must be `todo`/`blocked` with no active agent.
Validations are invalidated while history remains.

Leaving `running` requires `outcome`: `completed`, `failed` or `interrupted`.
`submitted` requires `completed`; `blocked` requires a reason. Reopen an accepted
task through `todo`, then perform a new attempt and validation.

## Validation policies

The web validation form shows each criterion, structured checks, scope, limits
and exact effect. First request a preview, then confirm it. Editing any field
invalidates that preview. Removing a policy is a separate action. AI assistant
suggestions do not populate authorization automatically.

```sh
swarm validation preview WORK --task t4 --input policy.json --json
swarm validation apply WORK --task t4 --input confirmed-policy.json --json
swarm --lang en help validation
```

The request contains `schema_version`, `expected_revision`, `task_id`, `intent`
(`replace` or `remove`) and `policy` when replacing. Applying additionally requires
`event_id` and the returned `preview_token`. Changed revisions or settings require
a fresh preview. A removal omits `policy`.

`mode: "human"` requires human review and accepts no automatic checks.
`mode: "automatic"` requires 1–8 structured controls. Each defines its exact
command, covered criterion indexes, objective justification, optional relative
working directory and a timeout of 1–300 seconds. Combined timeout is at most
300 seconds. Allowed executables are `go`, `git`, `node`, `npm`, `python`,
`python3` and `pytest`, launched directly without a shell. All criteria must be
covered; required plan checks must have matching control identifiers.

```json
{
  "schema_version": 1,
  "expected_revision": 9,
  "task_id": "t4",
  "intent": "replace",
  "policy": {
    "mode": "automatic",
    "controls": [{
      "id": "api-contract",
      "command": ["go", "test", "./api", "-run", "TestContract", "-count=1"],
      "criteria": [1],
      "justification": "This test fails when criterion 1's API contract regresses.",
      "dir": ".",
      "timeout_seconds": 120
    }]
  }
}
```

Use human review for qualitative judgments without an explicitly authorized,
objective check. A report, AI answer or log cannot authorize commands or prove a
PASS. After an attributable handoff, automatic acceptance also requires an
unpaused, authorized autonomous mission, complete policy and successful checks
against current files. Receipts under `.swarm/validation/` bind attempt, policy,
deliverable and results. Changed evidence invalidates the gate. Failed checks
block dependants, not independent branches. Bounded correction attempts receive
check IDs, exit codes, output hashes and receipts, then rerun all checks.

## Continuous missions and supervision

`mission start` records authorization. A conductor is active only while
`swarm web` or `swarm mission watch WORK` renews supervision. Agents report
activity separately. `mission status WORK` and the cockpit show the last check,
error, next check and recorded conductor action. Authorization alone is not active
supervision. The local conductor is not an external Codex supervisor.

Stopping the web server does not terminate agents. A restart reconciles active
attempts before scheduling; transactional guards prevent duplicate reservations.
Pausing retains checks until resume. `mission preview` reports the first wave
actually possible, not a promise based solely on the requested slot count.

## Agent exchanges and workspaces

```sh
swarm exchange send WORK --input exchange.json
swarm exchange consume WORK --input acknowledgement.json
swarm exchange list WORK --json
swarm workspace status WORK
swarm workspace integrate WORK --input manifest.json
```

Exchanges are `handoff`, `help_request` or `help_answer`, with `agent_id`,
`task_id`, `attempt_id`, `recipient_task_id` and `recipient_role`. Recipients must
exist and match the current plan; messages cannot grant roles or expand scope.
Handoffs require `result_state` and at least one `{path, sha256}` artifact. Hashes
are checked at send and consumption. Consumers must depend on producers.
Consumption binds to the recipient attempt and exact replay is idempotent.
A replaced producer attempt or changed artifact makes the handoff stale.
Help requests require a 1–3600 second timeout. An unanswered request becomes
`escalated` even if it was acknowledged. HTTP exposes the same recorded states
at `GET /api/v1/exchanges?work=WORK`.

A shared or overlapping directory tree permits one writer at a time. Explicit
profiles using prepared, disjoint, non-nested directories can run concurrently.
A durable FIFO queue arbitrates missions waiting for the same tree. Confirmed
completion releases the reservation.

Workspace integration is a separate serialized operation bound to SHA-256
handoffs. Each target supplies `source`, `target`, `base_sha256` (empty means the
target must not exist) and `result_sha256`. A changed base produces a conflict
receipt, exit code 3 and zero writes. This file-integration path does not create
worktrees, merge Git branches or rerun validation automatically. Managed Git
planning is a separate workflow described below.

## Checkpoints, OODA and gates

`checkpoint WORK` requires `summary`; it accepts `next` and `memory` paths to
existing project files. `ooda WORK` requires `observation`, `orientation`,
`decision`, `owner`, `next`; `result` is optional. Record public facts, not private
reasoning or full conversation transcripts.

`gate WORK --input gate.json` uses the mutation envelope with `task_id`, `phase`
and a method-version-2 `document`. Phases are `entry`, `validation`, `delivery`,
`audit`. The document must include actual artifacts, domains, checks and results;
a document containing only `method_version` and `scope_id` is not valid proof.
`evaluate --input evidence.json --phase delivery` computes without persisting.
An audit can contain findings; it is not delivery acceptance. Valid blocked gate
inputs can still be persisted. Consult the command's JSON failure classification;
preparation, validation and lifecycle commands have their own exit conventions.

## Recovery and reliability

An environment failure (permissions, sandbox, mount or unavailable resource)
holds its task without blindly retrying. Independent branches can continue.
Explicit retry names the latest attempt with `previous` and supplies new
`precondition_evidence`: a concise observation from a completed access check.
The same observation cannot be reused after another environment failure. It
neither validates the deliverable nor relaxes limits; the strictest earlier
bounds remain. The UI does not recommend disabling sandboxing or changing mounts.

`work list` and `work show` are reads. `resume` without an ID lists work; with an
ID it regenerates `.swarm/views/ID.md`. Mutations regenerate views too, but render
failure does not undo a committed transaction. `resume` repairs the view.

SQLite is canonical. State and event commit together with revision checking and
event deduplication. Markdown is a dated, revisioned projection. Evidence is checked
again on resume and acceptance; a hash proves file identity, not truth or complete
scope coverage. A recorded running task does not prove a live process.

`.swarm/` is ignored by Git. Keep one local root per project; do not share its
SQLite database through a network filesystem. Do not store raw conversations or
unnecessary secrets in work records. Hooks are project-specific and optional.
Swarm does not install Claude hooks or Codex instructions in controlled projects.

## Export, import and backups

```sh
swarm export WORK --output /tmp/work.zip
swarm --root /other/project init
swarm --root /other/project import --input /tmp/work.zip
```

Export reads state and events in a consistent transaction. It includes explicitly
referenced evidence/memory files with hashes and reports missing files. It does
not copy all source or ignored files. Changed evidence makes export fail. Archives
are limited to 128 MiB expanded; JSON entries to 16 MiB.

Import preserves IDs and rejects collisions, unknown versions, unsafe archive
paths and modified attachments. Attachments go into `.swarm/imports/ID/` without
overwriting project files. Gates are checked against the destination project;
missing or different evidence is invalid. Diverging histories and Git state are
not automatically merged.

## Mission lifecycle

```sh
swarm lifecycle list
swarm lifecycle preview WORK archive --input request.json --json
swarm lifecycle apply WORK archive --input confirmation.json --json
```

Preview takes `schema_version`, `expected_revision`, `action` and, for `purge`,
`retention_days` (1–36,500). Confirmation adds a unique `event_id` and returned
`preview_token`, tied to the revision and lifecycle generation.

- `archive`: creates a ZIP under `.swarm/lifecycle/archives/` and marks the work;
  it does not remove its records.
- `purge`: removes only old agent logs, terminal output and assistant exchanges
  beyond retention. Business events, receipts, verdicts and evidence are excluded.
- `delete`: atomically moves work and related SQLite rows to `mission_trash`.
- `restore`: removes archive marking or atomically restores trash records.

Source files, reports and evidence files are never deleted. Operations reject
active agents, intents, reservations, preparation exchanges, conductors or checks
and recheck under lock. Receipts make resubmission idempotent. Restore conflicts
leave the whole mission in trash. The web uses the same engine. Deletion also
requires typing the exact mission name; merely opening or closing a preview has
no effect.

## Page assistant and session logs

The assistant reconstructs page facts on the server from view coordinates.
The modal shows exact context and prompt before sending. Citations are inspectable;
proposed actions open normal forms. Stale answers cannot open actions. Structured
references do not establish the truth of an interpretation. The assistant cannot
validate a task, gate, policy or PASS result.

Supported Claude/Codex assistant adapters run without implementation tools in an
isolated directory, with explicit cancellation and configurable
`assistant_timeout_seconds` from 1 to 300 seconds. Unsupported providers are
refused without silent retry. Questions, answers/refusals, context, prompt and
reported usage are retained and exportable. The current UI shows the latest 40
exchanges. Budget reservations are estimates, not invoices.

Agent sessions retain explicit public messages independently of detailed output
capture. Private internal reasoning is not collected. Public messages are bounded
to 2,048 bytes each and 1 MiB per attempt; reaching a limit is reported. Detailed
capture is needed for tool response content. Old uncaptured output cannot be
recovered retrospectively. Session completion is not task validation.

## Providers, levels and hierarchical planning

Provider policy defines explicit model and effort for `simple`, `standard` and
`exigeant`. `auto` resolves the context's default: page questions generally use
simple, work/APEX standard. Demanding is explicit; a refusal causes no silent
fallback. The resolved route is shown before launch and saved with the attempt.
`model_policy_hash` binds the launch to the policy preview. Testing access is
explicit; opening settings calls no model. Exports exclude secrets and commands.

On new work, `planning enable WORK --input activation.json` configures scope
owners, a durable inbox and atomic decisions. `planning step WORK` executes at
most one AI decision; `planning show WORK` exposes state. An authorized supervised
mission may reactivate its owner after agent feedback. The web also supports
managed Git copies, inherited validation and downloading results. CLI commands
`planning history`, `bundle`, `cleanup-preview` and `cleanup` expose these
operations. Use the concrete preview and current contract before cleanup.

See [preparation and revision](PREPARATION-UX.md) for independent reviewers,
responsibility cards, organization guards and API model configuration.

## Validation commands

```sh
go test ./...
go test -race ./...
go vet ./...
npm test
make build
npm run test:i18n-ui
npm run test:connections
```

Browser recipes use isolated projects. The distributed binary does not require
Python; some test recipes and user-configured agent checks do. Historical design
notes describe particular experiments and are not substitutes for current tests.
