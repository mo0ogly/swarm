# Repeat a fresh Swarm installation

[Français](../FRESH-INSTALL.md) · [Full installation guide](INSTALL.md)

This Linux recipe separates Swarm source code, the controlled project and agent
data. Each participant uses their own directories and credentials. Start with
an empty project rather than copying a demonstration `.swarm` database.

## Native installation

Requirements: Git, Bash, Python 3 and Go 1.24 or newer. From a directory with no
existing `swarm` checkout:

```sh
git clone https://github.com/mo0ogly/swarm.git
cd swarm
mkdir -p "$HOME/projects/swarm-trial"
./install.sh --mode native --bin-dir "$HOME/.local/bin" \
  --project "$HOME/projects/swarm-trial"
./swarm.sh configure --root "$HOME/projects/swarm-trial" \
  --binary "$HOME/.local/bin/swarm" --address 127.0.0.1:18792
./swarm.sh start
```

The launcher opens a private session link. Use `./swarm.sh open` to reconnect.
Do not share the session link or `.swarm` files. The normal URL is
`http://127.0.0.1:18792/`. If another server occupies this port, the launcher
refuses to replace it; select another address with `configure`.

```sh
./swarm.sh status
./swarm.sh restart
./swarm.sh logs
./swarm.sh stop
```

Restart retains project data and connections. `swarm.sh` is at the checkout root
beside `install.sh`; run these commands there. Native agent tools need separate
installation and authentication on the host; see the [full guide](INSTALL.md).

## Docker installation

Additional requirements: a local Docker Engine on Linux, accessible to your
user, and Compose v2. Choose either Docker or native for a given port.

```sh
git clone https://github.com/mo0ogly/swarm.git
cd swarm
mkdir -p "$HOME/projects/swarm-docker-trial"
./install.sh --project "$HOME/projects/swarm-docker-trial" \
  --agent-home "$HOME/.local/share/swarm/agents-trial" --port 18787
docker compose --env-file deploy/install.env ps
docker compose --env-file deploy/install.env logs --tail 20 swarm
```

Open the private session link from the logs. The project is mounted at
`/workspace`, and agent data at `/home/swarm`. Swarm's source checkout is not
the user project. The image contains no preinstalled agent provider or API key.

```sh
docker compose --env-file deploy/install.env restart swarm
docker compose --env-file deploy/install.env down
docker compose --env-file deploy/install.env up -d --wait
```

These commands retain both mounted directories. `swarm.sh` manages the native
server; Compose manages Docker. A private `compose.override.yaml` can add the
certificate authorities required by an on-premise API; follow the override
section of the [installation guide](INSTALL.md). The API and its certificates
must be accessible inside the container. Keep TLS verification enabled.

## Interface acceptance checklist

1. Open the cockpit, then **Prepare a project**.
2. Enter a title and requirements, then save the requirements.
3. Open **Prepare with AI**. **Analysis and planning** is APEX.
4. Check all five methods: analysis and planning, guided workflow, diagnosis,
   examination and improvement. They are bundled with Swarm; an empty project
   needs neither a `.claude` directory nor a Claude installation.
5. Add your own AI and test the connection. If it fails, copy the diagnostic
   log and redact internal information before sharing it.
6. Send a preparation request. This proposes text and a plan; it does not build
   the application, start agents or accept results.
7. Read and adopt the brief, then check the plan and team. Execution requires
   a provider with tools, explicit limits and authorization.
8. Restart using the command for your installation mode. Confirm that saved
   requirements and connections remain; a retained API key needs no re-entry.

See the [user guide](USER-GUIDE.md) for subsequent steps and task graphs, and the
[bilingual pizza training kit](../training/casa-pizza/README.md) for a detailed
workshop. Installing Swarm or loading a method does not prove successful delivery
of a real mission with an AI provider.

The shipped sources are versioned in [`.claude/skills`](../../.claude/skills),
with preparation commands in [`.claude/commands`](../../.claude/commands) and
the [shared contract](../../tools/agent-workflows/CONTRACT.md). The build embeds
them in the binary and image. An empty user project does not need these files.
Project instruction profiles are separate: `.claude`, `AGENTS.md` or other
profiles may be unavailable in an empty project while bundled methods remain
available.

## Check the method catalogue

```sh
# Native: use the installed binary and the server's exact project root.
"$HOME/.local/bin/swarm" --root "$HOME/projects/swarm-trial" --json prepare methods

# Docker: run from the checkout containing deploy/install.env.
docker compose --env-file deploy/install.env exec -T swarm \
  swarm --root /workspace --json prepare methods
```

All five entries should have `available: true` and a `sha256` fingerprint. Existing
project method files take precedence. Symbolic links, invalid files or denied
access remain explicit errors; repair those files rather than expanding the
project boundary. See the [method rules](AGENT-METHODS.md).

To update an existing checkout, stop missions and the server cleanly, back up
the project, then run `git pull --ff-only`. If Git refuses, preserve local changes
and examine the divergence. Rebuild with `install.sh` in the selected mode and
restart. Pulling Git changes alone does not replace an installed native binary
or a running container.

[KS product workflow and graph views](PRODUCT-WORKFLOW.md).
