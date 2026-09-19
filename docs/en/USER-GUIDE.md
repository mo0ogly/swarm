# Using Swarm: from requirements to validated results

[Français](../../GUIDE-UTILISATEUR.md) · [Installation](INSTALL.md) · [Project overview](../../README.en.md)

Swarm lets you prepare, organize and follow an agent team without using an
external Codex conversation to manage it. Agent programs must still be installed,
authenticated and configured in the environment where Swarm runs them.

## Language

Choose **English** in the **Language** selector in the cockpit, preparation page
or agent session. The preference is stored in your browser. A `?lang=en` URL
selects English explicitly; `?lang=fr` selects French. French is the default.
Changing language reloads the view. Save preparation drafts first: the page
blocks language navigation while edits or requests are pending.

For the CLI, use `swarm --lang en …`, or set `SWARM_LANG=en` for the command.
`--lang` overrides the environment. Machine command names, JSON keys, state codes,
paths and preparation slash commands do not change. User documents, task titles,
reports and raw provider output stay in their original language.

## 1. Open the cockpit

Follow the [installation instructions](INSTALL.md). Select the directory of the
project the agents will inspect or modify. It may differ from the Swarm source
repository. Each project stores its own state under `.swarm/`.

From a compiled Swarm checkout:

```sh
./bin/swarm --root /path/to/project init
./bin/swarm --root /path/to/project providers init
./bin/swarm --root /path/to/project web 127.0.0.1:18787
```

Open the **session link printed by the server**. With Docker, retrieve it using:

```sh
docker compose --env-file deploy/install.env logs --tail 20 swarm
```

The link grants access to your cockpit. Keep it out of public screenshots.
Use the mission selector to consult different missions in the same browser tab.
If the server runs on another machine, `127.0.0.1` in your browser refers to your
own computer: use the SSH tunnel described in the installation guide.

## 2. Connect AI providers

Open **AI and connections**. Two types of connection have different capabilities:

| Type | Purpose | Requirements |
|---|---|---|
| API compatible with `/chat/completions` | Preparation, planning and report review | Base URL, exact model ID, API key if required |
| Tool-enabled agent program | Read/edit files and run commands | Installed program, authentication and provider configuration |

Swarm does not include Claude Code, Codex, Skynet or model weights. Programs
installed on the host are not automatically installed inside Docker.

To add an API:

1. Select **Add an AI connection**.
2. Enter a local ID, readable name, service base URL and exact model ID.
3. Use **Test connection**. This sends a short question without mission documents;
   the provider may charge for it.
4. Save separately: testing does not save the connection.
5. Select `api-<id>` in preparation or planning.

A successful text response does not prove file-tool capabilities or compliance
with every structured planning contract. A text-only API cannot be selected as
an implementation worker.

Model levels follow the saved provider policy. Inspect the resolved model before
launch. Swarm does not silently upgrade to a more powerful model. A custom API
connection uses its saved model for all three levels.

When creating missions, select **AI for the planner and reviewer** separately
from **Tool-enabled agent for workers**. The same installed provider can serve
both purposes when its capabilities allow it.

## 3. Prepare a mission

Choose **Prepare a project**. Describe the result, target directory, boundaries
and success criteria. For example:

> Add search to the document list. Preserve existing filters and both themes.
> Check empty results and accented characters. Deliver the code, tests and a
> report explaining the results.

1. Select the preparation provider and level.
2. Discuss the requirements, then review and approve the brief.
3. Request a plan and answer its open decisions.
4. Use **Edit missions** to refine titles, scopes, deliverables, success criteria
   and dependencies.
5. **Check plan** and address reported issues.
6. Choose **Review proposed team**, verify owner, workers, actual review, AI,
   workspace, limits and validation mode, run preflight, then give the single
   summarized authorization.

Checking a plan proves structural consistency, not completion of its work.
Creating missions does not start agents.

### Who validates results?

- **Me — human review of each result:** agents can progress to review, then wait
  for your acceptance.
- **The engine — explicitly authorized checks:** authorize the exact commands
  proving the criteria. Missing coverage or failed checks prevent acceptance.

New preparations configure an independent AI reviewer in a separate session.
Its opinion does not replace required tests or your chosen human acceptance.
For qualitative criteria without objective checks, retain human review.

## 4. Check the team and launch

| Responsibility | What it does |
|---|---|
| Planner / orchestrator | Organizes work, receives handoffs and decides what comes next |
| Branch planner, when configured | Organizes a delegated scope |
| Worker | Performs a task and produces its deliverable |
| Independent reviewer | Reviews criteria and the report in a separate session |
| Engine validator | Runs authorized checks and records validation decisions |

A visible planner card does not mean an AI call is running continuously. Naming
a task “review” does not configure an independent reviewer. Legacy missions may
have missing roles; inspect the organization instead of bypassing launch refusal.

After the single team authorization, open management. Only eligible missions may
start; dependency, budget, pause and workspace checks still apply. Review the
provider, directory, concurrency and limits before any manual start.

Slots set an upper bound on concurrent attempts. Unvalidated dependencies or an
occupied directory can reduce actual concurrency. **Start all** starts tasks ready
now; use the mission with an observed driver for ongoing automatic progression.

## 5. Follow work

Start with the **Agent management** summary: what is happening, the next step and
who acts. Then use the graph or list.

- Dependency arrows go from prerequisite to dependent task. Their verdict includes
  the freshness of evidence.
- Responsibility links connect roles to scopes. Read their legend; do not rely on
  color or line style alone.
- **Horizontal / Vertical** changes layout. **Simplified / Detailed** changes the
  information displayed. Neither changes task ordering.
- **+ / −** collapses or expands branches without deleting tasks.
- Filters and **Fit overview** help locate off-screen branches.
- Click a card for details. **Watch agent work** opens its session. Double-clicking
  a task opens its session when available.

In the session, read timestamps, event types and messages. Received output shows
observable activity, not the model's private reasoning. Detailed capture is
optional; without it, not all raw provider output is retained.

## 6. Understand states and validate

| State | Meaning | Next step |
|---|---|---|
| To do | Task has not started | Read start conditions |
| In progress | An attempt is tracked as running | Also inspect process state and last signal |
| Needs review | A result was submitted | Read report, review and checks |
| Blocked | An obstacle prevents progress | Inspect the reason and proposed recovery |
| Completed and validated | Result accepted | Dependencies can be satisfied while evidence stays current |
| Accepted — revalidation needed | Evidence changed after acceptance | Repeat relevant checks |
| Waiver / Abandoned | Explicit decision, different from success | Read its justification |

**A completed process is not a validated task.** Recent output does not prove the
process is still working. Swarm separates execution, received activity and validation.

For human review, open the result, read its report and criteria, then use the
review and acceptance actions. A gate is a passage check: if delivery is refused,
inspect missing or stale evidence. Acknowledging a notification means only that
you read it.

Automatic validation waits for recorded conditions. Do not relaunch simply because
a review is pending. A hierarchical mission finishes when results are handled
and planners have closed their scopes.

## 7. Resolve blockers

Open the flagged task and diagnosis. **Explain with AI** can clarify context and
propose an action. Review its effect before applying it; advice is not validation.

| Symptom | Useful action |
|---|---|
| Provider unavailable | Install/authenticate it in the actual execution environment and check configuration |
| Only workers visible | Inspect missing organization roles; do not bypass launch refusal |
| Workspace occupied | Identify the reserving attempt; wait for completion or review stopping it |
| Dependency unvalidated/stale | Open the prerequisite and handle review or evidence |
| Attempt completed without report | Inspect output, expected deliverable and location before retrying |
| Repeated tool errors | Fix permissions, resource or failed check; raising the limit does not fix the cause |
| Signal lost/process unconfirmed | Inspect session and host before starting a duplicate |
| Driver absent | Check that web or mission watch runs for the same project |
| Context too large | Reduce documents/excerpts or restart from a shorter brief; no silent truncation |
| Cost not reported | Missing provider measurement, not free usage |
| Refusal after confirmation | Reread state: the mission may have changed since preview |

Fix the cause, then use the proposed recovery. An identical retry under unchanged
conditions is likely to fail again.

## 8. Revise a mission

**Revise this Swarm** returns to its source preparation. Edit the draft, check the
new plan, then **Compare and apply revision**. Review additions, changes and results
that require revalidation.

Editing the draft alone does not change the mission. Applying a revision retains
history, may reopen results, pauses the mission and locks new starts. Review and
authorize the plan again before resuming. Active attempts or some delegations may
prevent application; resolve the refusal before requesting another comparison.

To change arrows, edit the plan's dependencies and check it. Moving the view or
changing graph orientation does not change scheduling.

## 9. Pause, stop and clean up

**Pause** suspends new starts; running agents continue. To stop an attempt, use
its stop action and verify confirmed process termination. Closing the browser
is not stopping agents. Stopping the server does not guarantee stopping agent
processes; stopping a container interrupts its environment.

In **Manage missions**:

| Action | Effect |
|---|---|
| Archive | Files away the mission and creates a ZIP; retains data |
| Restore | Reactivates an archive or recovers a trashed mission |
| Purge history | Removes old logs/output under retention, excluding evidence and receipts |
| Delete | Moves internal data to recoverable trash; does not delete project files |

The preview changes nothing. Read volumes and exclusions before confirmation.
Deletion requires the exact name. Active work can block cleanup; resolve it and
request a fresh preview. See installation for backups and upgrades.

## 10. CLI equivalents

Web and CLI share state only when using the same project root. From the compiled
Swarm checkout, replace paths and IDs:

```sh
./bin/swarm --lang en --root /path/to/project work list
./bin/swarm --lang en --root /path/to/project work show WORK_ID
./bin/swarm --lang en --root /path/to/project mission status WORK_ID
./bin/swarm --lang en --root /path/to/project planning show WORK_ID
./bin/swarm --lang en --root /path/to/project agent list WORK_ID
./bin/swarm --lang en --root /path/to/project agent logs AGENT_ID
./bin/swarm --lang en --root /path/to/project console WORK_ID
./bin/swarm --lang en --root /path/to/project mission pause WORK_ID
./bin/swarm --lang en --root /path/to/project mission resume WORK_ID
```

In Docker:

```sh
docker compose --env-file deploy/install.env exec swarm swarm --lang en --root /workspace work list
```

Use `--json` for scripts. `swarm --lang en help` lists help topics. English aliases
include `management`, `planning`, `tasks`, `validation`, `lifecycle`, `logs`,
`context` and `parity`; existing French topic names still work.

Creation and revision use versioned JSON documents. See the [technical reference](REFERENCE.md)
for schemas. Mutations require the current revision and an operation ID; do not
blindly replace a refused revision. The CLI does not reproduce every web form;
the interactive API connection test is available in the web.

## 11. Data and limitations

State, connections and logs are stored locally under `.swarm/`. API keys are kept
in a file restricted to the process account, without application-level encryption.
Do not publish this directory or session links. Captured output may contain
sensitive project content.

APEX, KS and PDCA methods require their resources in the controlled project; not
all are included. Standard preparation does not implicitly create isolated Git
copies. The advanced managed Git workflow is separate.

The standalone installation recipe used deterministic agent processes. It does
not qualify your real AI provider, authentication or project.
