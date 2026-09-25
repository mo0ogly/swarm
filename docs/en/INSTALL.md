# Install Swarm

[Français](../../INSTALL.md) · [User guide](USER-GUIDE.md)

The supported installation path is Linux. Choose Docker for a dedicated runtime,
or a native binary to use tools already installed on your machine. The installer
does not request sudo, modify shell profiles or install Docker for you.

## Docker

Requirements: Git, Bash, a local accessible Docker Engine and Compose v2 with
`up --wait`. Building downloads images and dependencies.

```sh
git clone https://github.com/mo0ogly/swarm.git
cd swarm
mkdir -p "$HOME/projects/my-project"
./install.sh --project "$HOME/projects/my-project"
```

Open the session URL printed after startup. Retrieve it again with:

```sh
docker compose --env-file deploy/install.env logs --tail 20 swarm
```

The image contains the compiled Swarm binary with embedded web assets, Node.js,
Python, Git, curl and TLS certificates. It runs with your UID/GID. The project is
mounted at `/workspace`; `/home/swarm` persists agent tools and authentication.
**No agent program, model weights or AI key is bundled.**

Options:

```sh
./install.sh --project "$HOME/projects/my-project" --port 18788 \
  --agent-home "$HOME/.local/share/swarm/agents-demo"
./install.sh --project "$HOME/projects/my-project" --no-start
docker compose --env-file deploy/install.env up -d --wait
```

`deploy/install.env` is private and ignored by Git. The installer refuses to
replace an active Compose service. Stop missions and agents deliberately before
reinstallation.

### Network access

Swarm listens on loopback only. Compose uses `network_mode: host`, without a
published `ports:` mapping. This recipe targets a local Docker Engine on Linux;
Docker Desktop, Windows and remote Docker engines are not qualified here.
Do not change the listener to `0.0.0.0`: Swarm refuses it.

For a remote Linux server, run this from your workstation:

```sh
ssh -L 18787:127.0.0.1:18787 user@server
```

Then open the session link at `127.0.0.1:18787` locally. Keep session links private.

## Configure AI providers

For text APIs, open **AI and connections → Add an AI connection**, enter a compatible
`/chat/completions` service URL, exact model ID and any required key. Test and save.
These connections can prepare, plan and review; they do not automatically gain
file editing tools.

### Skynet from the container, through the host proxy

The Skynet harness installed **on the host** provides an OpenAI-compatible LiteLLM
proxy on `127.0.0.1:4010`. With `network_mode: host`, the container reaches it at
the same address: the image does not change and Skynet keys stay on the host.

In **AI and connections**, add a connection with service URL
`http://127.0.0.1:4010/v1`, a route name exposed by the proxy as model ID, and the
proxy's local master key (`general_settings.master_key` in its LiteLLM
configuration). List the routes from the container:

```sh
docker compose --env-file deploy/install.env exec swarm \
  sh -c 'curl -s -H "Authorization: Bearer $PROXY_KEY" http://127.0.0.1:4010/v1/models'
```

Replace `$PROXY_KEY` with the proxy master key; do not store it in a tracked file.
This connection prepares, plans and reviews; it does not edit files. Running a
tool-enabled `skynet_harness` agent inside the container requires installing and
configuring it there and is not covered by this recipe.

Proxy routes follow the catalogue published by Skynet. An upstream `404` usually
means stale routes: update the harness **on the host**, re-run its installer, then
check with `skynet-doctor`. Swarm never falls back to another model automatically.

### Tool-enabled agents

For implementation, install a tool-enabled agent inside the container and
follow its publisher's authentication instructions:

```sh
docker compose --env-file deploy/install.env exec swarm bash
```

For npm-distributed tools, install the official package and a chosen version with
`npm install -g PACKAGE@VERSION`. The npm prefix `/home/swarm/.local` persists and
its `bin` directory is on PATH. Additional system dependencies belong in a derived
image; installation only in a container's writable layer does not survive recreation.

Host-installed commands are not automatically available in the container.
`providers init` discovers known commands only when it creates the configuration;
it does not overwrite an existing file. After adding a provider, review and update
`/workspace/.swarm/providers.json`. The `skynet_harness` model adapter recognizes
that executable when installed and configured; recognition is not installation
or a full qualification test.

```sh
docker compose --env-file deploy/install.env exec swarm swarm --root /workspace providers show
docker compose --env-file deploy/install.env exec swarm swarm --lang en --root /workspace work list
```

## Native installation

Requirements: Linux and a suitable Go toolchain; see `go.mod` for the declared
version. From the checkout:

```sh
./install.sh --mode native --bin-dir "$HOME/.local/bin"
"$HOME/.local/bin/swarm" --root /path/to/project init
"$HOME/.local/bin/swarm" --root /path/to/project providers init
"$HOME/.local/bin/swarm" --root /path/to/project web 127.0.0.1:18787
```

The native installer builds and installs the binary; it does not start the server.
Agent programs must be installed and authenticated separately. Web assets are
embedded; Node.js is needed to rebuild frontend bundles, not simply to run Swarm.

## Persistence, shutdown and upgrades

Before shutdown or upgrade, inspect active missions and agents. Pausing prevents
new starts but does not stop agents already running. Confirm process termination
before backing up or replacing the environment.

Back up the entire project state, referenced reports/evidence and the persistent
agent home. Exports of individual missions are not a replacement for a full backup
of configuration and authentication. Avoid copying a live SQLite database without
its associated transactional state; use a stopped environment for a filesystem backup.

After stopping mission activity:

```sh
docker compose --env-file deploy/install.env stop swarm
docker compose --env-file deploy/install.env down
```

The bind-mounted project and agent-home directories remain. Do not delete them
as part of a routine restart. To upgrade, back up first, update the checkout and
run the installer again. Storage migrations can prevent older binaries from
opening upgraded state; restore a matching backup rather than forcing a downgrade.

## Troubleshooting and verification

- Docker unavailable: check `docker version` and access to the local engine.
- Port occupied: use another `--port`; do not stop an unrelated service.
- Session refused: reopen the link from this server's logs.
- Agent unavailable: check inside the runtime, not only on the host.
- Permission failure: check actual project ownership and the agent's permissions.
- Empty connection list: add your API or install/configure your agent.

```sh
make build test smoke
make test-install
make test-process
```

The installer recipe requires Docker and a checkout without an existing
`deploy/install.env`. It uses isolated test projects. Process recipes use
simulated deterministic agents without paid model calls. They do not validate
your real provider or your project.
