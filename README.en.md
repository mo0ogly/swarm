# Swarm

<img src="web/swarm-logo.png" alt="Swarm" width="96">

### Give an AI team an objective. See who does what. Verify what it delivers.

[Français](README.md) · [User guide](docs/en/USER-GUIDE.md) · [Installation](docs/en/INSTALL.md)

Swarm is a local cockpit and CLI for organizing software work across AI agents.
It makes responsibilities, dependencies, attempts, evidence and blockers visible,
so that several agents can collaborate without treating every completed process
as a successful result.

## See the graph first

[![See the graph first](docs/screenshots/graph-linkedin/Swarm_graphe_apercu_FR_V2.gif)](docs/screenshots/graph-linkedin/Swarm_graphe_LinkedIn_FR_V2.mp4)

Explore dependencies and branches, inspect a blocker and define validation checks. This one-minute demonstration shows actual browser interactions with French explanations, in a teaching plan. No provider agent is launched; the control configuration is an unsaved example.

[Watch the MP4](docs/screenshots/graph-linkedin/Swarm_graphe_LinkedIn_FR_V2.mp4) · [Still image](docs/screenshots/graph-tour/graph-tour-en.png). GitHub displays the animated GIF preview; the video opens through the link.

## Release v0.1.0

The [first experimental release](https://github.com/mo0ogly/swarm/releases/tag/v0.1.0) adds dependency editing with previews, durable automation programs and configurable administration. The engine retains authorization, budgets, required checks and independent reviews.

See the [release notes](docs/releases/v0.1.0.md), [automation programs guide](docs/en/AUTOMATION-PROGRAMS.md) and [blocked contract recovery](docs/en/CONTRACT-REVISION.md). Quotas of external provider sessions are not observed automatically.

[Business prerequisites and control recovery](docs/en/ENGINE-BUSINESS-CONTROLS.md)

## First run

1. Follow [installation](docs/en/INSTALL.md); for native development, `./swarm.sh start` opens the authenticated cockpit at its stable local address.
2. Open **AI and connections** to configure providers available in your environment.
3. Prepare your objective, constraints and acceptance criteria; review the proposed plan.
4. Check roles, workspaces and limits before authorizing and launching the mission.
5. Follow tasks and examine their evidence. A finished process is not yet a validated result.

See the [illustrated user guide](docs/en/USER-GUIDE.md). Swarm is experimental; read the [qualification evidence and limitations](docs/COMMUNITY-QUALIFICATION.md).

Start with **Choose a mission template** in preparation: [Guided workflow catalogue shared by web and CLI](docs/en/PREPARATION-TEMPLATES.md).

### Casa Pizza hands-on training

Open **Cockpit help → Casa Pizza training** for four captioned video tutorials, GIF previews, transcripts and a **38-page PDF guide**. Resources follow the selected cockpit language; switching language in the reader updates videos, captions, PDF and kit links. The reader also works offline from the kit, without external services or AI calls.

- [English PDF](docs/training/casa-pizza/Swarm_Casa_Pizza_Training_EN.pdf) · [English kit](docs/training/casa-pizza/Swarm_Casa_Pizza_Training_Kit_EN.zip)
- [PDF français](docs/training/casa-pizza/Formation_Swarm_Casa_Pizza.pdf) · [Kit français](docs/training/casa-pizza/Kit_Formation_Swarm_Casa_Pizza.zip)
- [Workshop contents and limitations](docs/training/casa-pizza/README.md)

The videos are sequences of real screenshots, with captions, rather than continuous recordings. The reference application passes eleven tests and remains French in both kits. Word sources are retained for editing. The Swarm logo appears in the cockpit, preparation and reader; its favicon identifies browser tabs.

## What it does

- Prepare requirements with AI, approve a brief and review a structured plan.
- Assign a planner, workers and an independent report reviewer.
- Follow dependencies in a graph with arrows, branch folding and horizontal or
  vertical layouts; switch to a detailed agent list when useful.
- Inspect agent sessions, received messages, tool activity and reported usage.
- Chain authorized tasks while enforcing dependencies, workspace reservations,
  budgets and validation policies.
- Revise missions with a comparison and history rather than overwriting ongoing work.
- Archive, restore, purge old logs or move mission data to recoverable trash.

## From requirements to results

```mermaid
flowchart LR
  A[Requirements] --> B[Reviewed brief]
  B --> C[Checked plan]
  C --> D[Explicit authorization]
  D --> E[Agent team]
  E --> F[Reports and evidence]
  F --> G[Review and required checks]
  G --> H[Validated result]
  G -->|Corrections needed| E
  classDef prepare fill:#e8f0fe,stroke:#3457a0,color:#172b4d
  classDef execute fill:#fff4d6,stroke:#946200,color:#503600
  classDef verify fill:#e5f5eb,stroke:#267243,color:#17472b
  class A,B,C prepare
  class D,E execute
  class F,G,H verify
```

## Responsibilities

The **planner** organizes work and handles agent handoffs. A **branch planner**
can own a delegated scope. **Workers** implement tasks and provide deliverables.
An **independent AI reviewer** examines reports in a separate session. Its opinion
does not replace required deterministic checks or chosen human acceptance.

This organization draws on the planning/worker separation discussed in
[Cursor's self-driving codebases research](https://cursor.com/blog/self-driving-codebases).
Swarm's independent reviewer is an additional design choice, not a claim that it
reproduces every part of Cursor's final architecture.

## See the interface

Actual application captures from **October 1, 2026**, showing an eight-task mission closed with human decisions. This is not evidence of unattended autonomy. User-authored task names retain their original French language.

### Understand the current state

The summary states what is happening, who acts and what comes next. All eight results here are validated against current evidence, with deliverables and reviews still accessible.

![Current mission: 8/8 results validated](docs/screenshots/en/mission-complete-en.png)

### See the team and dependencies

The orchestrator is on the left, workers in the middle and the independent reviewer on the right. **Solid arrows** connect prerequisites to dependent tasks; **dotted arrows** show responsibilities and reviewer handoffs.

![Current graph with orchestrator, workers, reviewer and arrows](docs/screenshots/en/mission-graph-current-en.png)

[Open the graph at full size](docs/screenshots/en/mission-graph-current-en.png)

Horizontal and vertical layouts, detail levels, filters and branch folding change the view without removing tasks. “Why is this task waiting?” explains prerequisites; “Where do calls and costs go?” separates observed consumption.

Preparation turns needs into criteria, tasks and dependencies. Adopting a plan does not launch agents: check roles, connections, workspaces and limits before authorizing execution. See the [illustrated user guide](docs/en/USER-GUIDE.md) and [installation screenshots](docs/en/INSTALL.md).

## Install

Linux with local Docker Engine and Compose v2:

```sh
git clone https://github.com/mo0ogly/swarm.git
cd swarm
mkdir -p "$HOME/projects/my-project"
./install.sh --project "$HOME/projects/my-project"
```

For native local development, use the stable launcher:

```sh
./swarm.sh start
./swarm.sh restart
./swarm.sh status
```

The cockpit stays at `http://127.0.0.1:18792/`. The launcher opens the browser,
replaces its previous server on restart and preserves project data. Use
`./swarm.sh open` to reconnect locally. Python 3 and a local browser opener are required.
For the Docker installation above, use the session link printed by the installer.
Select **English** in the cockpit.
For native installation, configuration, authentication and remote access, see
[Install Swarm](docs/en/INSTALL.md).

**Swarm does not bundle agent programs or model weights.** Install and authenticate
Claude Code, Codex, Skynet or your chosen compatible agent separately. A text API
connection can plan and review; it cannot by itself edit project files.

## CLI

Web and CLI share project state when they use the same `--root`.

```sh
swarm --lang en --root /path/to/project work list
swarm --lang en --root /path/to/project mission status WORK_ID
swarm --lang en help management
```

`SWARM_LANG=en` sets the CLI default. JSON keys, commands and stored user content
retain their original identifiers and language. French remains available.

### Check the binary that is actually running

These forms read identity embedded in the binary and work without an initialized
project or SQLite database:

```sh
swarm version
swarm --version
swarm --json version
```

`devel` means that no verified release tag was injected. `unknown` or `null`
means that the corresponding metadata is unavailable; it is neither a release
nor a successful check. JSON keeps **binary** identity separate from the optional
**local source** state displayed by the web UI. Restart the server after rebuilding
or installing: an existing process keeps running its previous binary.

In both the cockpit and preparation page, **Version and what's new** opens the
same bilingual dialog. It shows the running binary, compares local sources
separately, then lists the embedded history. No release is currently declared,
so the history stays empty instead of inventing a version or date.

![Version and what's new in the cockpit, English, State theme](docs/screenshots/version-history/cockpit-en-etat.png)

[See the FR/EN, State/dark and degraded-state capture manifest](docs/screenshots/version-history/manifest.json).

## Limits that matter

A finished agent is not necessarily a validated task. Workspace reservations may
serialize tasks even with several available slots. Standard preparation does not
automatically create parallel Git copies. Unreported cost remains unknown.
Closing the cockpit does not stop agents. An installed binary and simulated-agent
tests do not qualify a real provider's authentication, permissions or results.

Data is local under the controlled project's `.swarm/`. Keep this directory,
authentication and session links private. This repository supplies twelve
[working methods](docs/en/AGENT-METHODS.md), including planning, system examination and improvement,
debugging, review and lessons learned. All five preparation methods work from the bundled pack in an empty project;
existing local overrides are checked before use.

Method source files are tracked in [`.claude/skills/`](.claude/skills/) and
preparation commands in [`.claude/commands/`](.claude/commands/).
[Fresh installation checklist](docs/en/FRESH-INSTALL.md).

## Repository layout

The CLI entry point is in `cmd/swarm`, the Go engine and its unit tests in
`internal/engine`, the interface in `web` and `frontend`, and guides in `docs`.
See the [repository map](docs/en/REPOSITORY-STRUCTURE.md) and the
[pizza training guides](docs/training/casa-pizza/README.md).

## Documentation and development

- [English user guide](docs/en/USER-GUIDE.md)
- [English installation guide](docs/en/INSTALL.md)
- [Authorize recovery and supply candidate sources to the reviewer](docs/en/ATTEMPT-RECOVERY.md)
- [Storage diagnosis and exhausted attempts](docs/en/RUNTIME-RECOVERY.md)
- [Methods: requirements, specifications, planning, audit, diagnosis and verification](docs/en/AGENT-METHODS.md)
- [Agent reports, evidence delivery and linked decisions](docs/en/AGENT-COMMUNICATION.md)
- [English interface validation](docs/en/I18N-VALIDATION.md)
- [Technical reference](docs/en/REFERENCE.md)
- [Preparation and revision contracts](docs/en/PREPARATION-UX.md)

```sh
make build test smoke
make frontend
make test-install
make test-process
```

## License

Swarm uses [PolyForm Noncommercial 1.0.0](LICENSE). Source is available for
noncommercial use under that license; commercial use requires separate permission.
This is **not an OSI-approved open-source license**. Previous Apache-licensed
versions and third-party components retain their respective rights and notices:
see [licensing details](docs/LICENSING.md) and [third-party notices](THIRD_PARTY_NOTICES.md).

## Contributing and reporting issues

See [CONTRIBUTING.md](CONTRIBUTING.md) for checks and pull requests, and
[SECURITY.md](SECURITY.md) for sensitive reports.

### Recognize a closed mission

The cockpit distinguishes a finished agent from a validated result. A closed
mission reports every result validated against current evidence and a closed
root responsibility. Deliverables, reviews and costs remain accessible.

![Closed mission in the cockpit](docs/screenshots/en/mission-complete-en.png)

*Actual 8/8 recipe with human interventions. The user-authored title remains in French. See the [user guide](docs/en/USER-GUIDE.md#read-counters-and-recognize-completion) and [engine recovery](docs/en/ENGINE-RECOVERY.md).*

## Working methods

Methods provide a shared approach with criteria and evidence; executable engine rules remain authoritative.

| Need | Method |
| --- | --- |
| Turn a need into a verified change | Guided change workflow |
| Examine a system and address authorized findings | Examine and improve |
| Define expected outcomes and acceptance criteria | Build the specification |
| Find omissions and contradictions before execution | Examine the specification |
| Find actionable implementation defects | Review the code |
| Reproduce a problem and identify its cause | Diagnose and fix |
| Recheck the affected flow after a correction | Verify the fix |
| Recover a blocked plan while retaining history | Replan |
| Explain outcomes and prioritize improvements | Lessons learned |

The specification methods cover both requirements writing and plan examination. See the [method guide](docs/en/AGENT-METHODS.md) for actual preparation choices and native-session commands; not every method is a web button.

### Project instructions

Pass repository rules to each role with an explicit project profile. [Configuration and boundaries](docs/en/PROJECT-PROFILE.md).

In **Start / Restart → Skills for this task**, select project methods for a
worker. Choices are recorded with the attempt without automatically running
scripts or changing permissions. The CLI provides `swarm skills list`.
[Selection guide](docs/en/PROJECT-PROFILE.md#skills-selected-for-an-action).

The [application creation workflow](docs/en/PRODUCT-WORKFLOW.md) documents migrated methods, design, gates and application → journey → story → task navigation.


### Configurable SQLite wait

Open **Administration → SQLite storage wait** to inspect or change retries, retry delay and SQLite wait per attempt. The CLI exposes the same persisted settings through `swarm storage-retry show` and `swarm storage-retry apply --input configuration.json`. These settings do not restart a provider or agent. See the [SQLite recovery guide](docs/en/SQLITE-CONTENTION-RECOVERY.md).
