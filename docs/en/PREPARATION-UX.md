# Prepare and revise a Swarm from the web

[User guide](USER-GUIDE.md) · [Technical reference](REFERENCE.md) ·
[French version](../../PREPARATION-UX.md)

## From requirements to a team

1. Choose **Prepare a project** in the cockpit, including in simple mode.
2. Describe the requirements. Choose an AI provider and level; the resolved model
   is shown before sending. Preparation calls cannot modify repository files.
3. Review and adopt the brief. Request a plan, answer open decisions and edit
   missions: title, role, scope, deliverable, criteria, dependencies and limits.
   The web form does not require you to write JSON.
4. Check the plan. This checks its contract, decisions and absence of cycles;
   it does not prove that deliverables have been produced.
5. Review the proposed team: owner, workers, actual reviewer, AI models,
   workspace, limits and acceptance mode. Run preflight; it checks the worker
   and workspace without creating a mission or starting an agent.
6. **Authorize this team and its missions** is the single confirmation. It
   atomically creates the organization and then permits only eligible starts.
   Dependency, budget, pause and occupied-workspace guards still apply.

The AI owner is recorded in the responsibility tree, separately from task
relationships. It handles agent feedback. A task whose role is `planner` alone
is not a durable mission owner. New prepared missions also configure a separate
AI reviewer. Historical missions are not silently assigned this role.

### Explicit validation

- **Human review:** starts and exchanges may run automatically, but each result
  waits for human acceptance. The planner's opinion alone cannot accept it.
- **Authorized engine checks:** the operator supplies exact programs and
  arguments proving entry, validation, delivery and each criterion. Incomplete
  coverage is rejected. A task allows at most eight checks and 300 seconds of
  combined execution. The form proposes 30 seconds per check; with more than
  five criteria, group coverage meaningfully or use human review.

The selected workspace is shared by default. Reservations can serialize tasks.
Preparation does not automatically create parallel Git copies.

## Revise an existing mission

**Revise this Swarm** returns to its source preparation. Editing the draft does
not change running tasks.

After checking the new plan, comparison shows modified tasks, added tasks and
results needing revalidation through dependency propagation. Applying the revision
is atomic: it preserves old plans and attempts, reopens affected results, pauses
the mission and locks future starts. Authorize starts again, then resume.

The transaction refuses stale revisions, active attempts, decisions in progress
and already delegated branches. It also refuses deletion of an organizational
mission or criterion, preserving responsibility coverage. With automatic
validation, added tasks or changed criteria require explicit reauthorization of
the policy; they are not silently converted to human review. A refused revision
leaves the draft available.

## AI and connections

This menu is available in simple mode and from preparation. It configures the
three model levels for installed CLI providers, shows availability and offers an
explicit test.

The default level follows provider policy. Swarm does not freely choose any model;
the demanding level remains an explicit choice. Preparation, planner and agents
use the same model resolver. Model and effort are sent to the executable and the
route is retained with the call or organization. Stale configuration is rejected
rather than silently upgrading the model.

This page does not install agent programs. It can add API connections and local
keys. Installed programs remain declared in `.swarm/providers.json` and authenticate
in their own execution environment.

## CLI: shared mutations and guarantees

```sh
swarm prepare conversion ID
swarm prepare create-missions ID --input request.json
swarm prepare revise-missions ID --input request.json
swarm prepare release-plan ID --input request.json
swarm prepare send ID --input request.json
```

- `conversion` previews the plan and expected effect.
- `create-missions` takes `organization` with `provider`, `level`, `workspace`,
  `validation`, `max_tasks`, `max_calls` and optional `controls` by local task ID.
- `revise-missions` takes the checked draft version, `sha256`,
  `expected_revision`, `expected_work_revision` and a unique `event_id`.
- `release-plan` authorizes the recorded plan.
- `send` accepts optional `level` and `model_policy_hash`; `auto` uses the
  provider's working default.

Preparation mutations use `version: 1`, `action` and `preparation_id`.
CLI and HTTP call the same transactional engine. The terminal uses the same
versioned JSON document, but does not provide the web's graphical mission editor.
An older conversion without `organization` remains manual: the organization guard
prevents incomplete autonomous execution.

## Hand a result to the owner

For an organization without managed Git integration, a worker writes
`docs/<task-id>.md` inside its workspace and exits normally. It does not need to
construct a handoff JSON request. The conductor locates the report, checks the
current attempt, normal completion, existence, freshness and unambiguous
attribution. The file must remain within project scope.

The change to review-pending state and the message to the owner are recorded in
one transaction with attempt identity and report hash. Repeated handoffs do not
create duplicates. Revision or write conflicts cause bounded retries with a fresh
read, never skipped checks. Interrupted attempts and stale, empty, ambiguous or
changed reports still require examination.

Handoff is not acceptance. The recorded validation policy remains applicable.
Independent AI review does not replace configured human acceptance. Historical
missions retain their configuration; installing an update does not retrospectively
turn an interrupted attempt into a success.

## Independent reviewer for prepared missions

New preparations configure planner, workers and AI reviewer. The reviewer uses
the selected team provider and level in a separate session, without tools or the
production conversation. The chosen call limit applies separately to planner and
reviewer. Each call also reserves money from the shared budget.

After a completed attempt submits its report, the conductor starts a review bounded
to 90 seconds. It sends the criteria and report, at most 48 KB without truncation.
The opinion must address every criterion. Favorable conclusions must quote an
exact passage from the report; missing external evidence must be requested.
This document review does not claim to execute tests or inspect source code.
Configured deterministic checks and human acceptance remain required.

The opinion is bound to the attempt, criteria and report hash. Changing any of
these invalidates it. Correction requests reach the planner and block the task;
the planner may propose another attempt within existing limits. Call failures are
recorded without an unbounded paid retry loop. A server interruption consumes the
call, never implies success.

```sh
swarm planning configure-reviewer WORK --input request.json
swarm planning review-step WORK
swarm mission status WORK
swarm planning show WORK
```

`configure-reviewer` explicitly adds this role to a historical mission without an
active worker. Fields: `provider`, `level`, `max_activations`, `schema_version`,
`event_id`, `expected_revision`. `review-step` requires an unpaused mission.
`mission status` presents opinions; `planning show` retains their structured detail.

Historical organizations keep their human or deterministic policy until explicit
reviewer configuration. New preparations carry `reviewer_required`: removing the
configuration cannot bypass launch or acceptance guards. Managed Git repositories
retain their integration controller and tests; that controller is not presented
as this AI reviewer. Storage version 20 includes a prior backup and rejects older
binaries.

### Responsibilities in graph and list

The orchestrator, configured subplanners and reviewer appear alongside production
tasks as soon as the organization exists. Their cards describe configured services,
not necessarily active calls. Paused, waiting and calling states are distinguished.
The agent list exposes the same responsibilities.

Dotted arrows connect planners to their scopes and tasks to the reviewer. Solid
arrows retain task dependencies. Click a role to read decisions or opinions.
Roles stay visible when tasks are filtered or collapsed. Missing reviewer
configuration is explicitly shown; a managed repository's deterministic
integration controller remains a separate role.

## Add your own AI

In **AI and connections → Add an AI connection**, enter a local identifier,
display name, base URL and exact model identifier. Ollama and vLLM/LiteLLM presets
suggest local URLs; any compatible `POST /chat/completions` URL can be entered.
An outdated model catalogue is never treated as evidence of availability.

**Test connection** explicitly sends a short question with no mission context,
with a maximum wait of 15 seconds. It shows the answer and elapsed time, or an
error. Saving is separate and launches no mission. The test demonstrates a text
response, not every structured planner contract. Business responses still pass
existing strict contract validation.

Saved connections appear as `api-<identifier>` in preparation and planner/reviewer
selectors. Calls use a separate process without tools, a 60-second HTTP timeout,
at most 1 MB of input and 1 MiB of response. The initiating workflow's own limits,
reservations and checks remain applicable. Token counts and costs are not inferred
from a connection-test response.

During conversion, choose the planner/reviewer AI and a tool-enabled worker agent.
An installed agent may serve both. A text API cannot be selected as a worker; the
engine rejects it before creation. Levels follow each provider's policy. Custom
API connections use their recorded model at all three levels; they do not claim
three distinct models. Existing organizations remain unchanged.

Keys are stored in `.swarm/ai-connections.json` with mode `0600`: access is limited
to the process account, without application-level encryption. Lists and policy
exports never return keys. An empty key retains the existing one; explicit removal
clears it. Changing destination requires explicitly supplying the key, and network
redirects are refused. Concurrent changes are rejected. A call prepared against
an older configuration cannot silently use the new one.

Configuration can edit or disable a connection. Disabling removes it from future
choices but does not kill a running call or replace installed agents.

```sh
swarm connections list
swarm connections save --input connection.json
```

`list` returns secret-free connections and their digest. `save` uses the same
engine as the web: `version: 1`, `expected_digest`, `connection` (`id`, `label`,
`base_url`, `model`, `disabled`, optional `key`) and `replace_key`. Interactive
connection testing is available in the web; this CLI save command makes no call.
Custom connections are not implicitly activated for every mission.
