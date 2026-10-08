# Swarm repository map

Swarm is a Go application with a CLI and a local web interface for preparing,
organizing and tracking missions assigned to AI agents. This map describes
where sources, documentation and development tools belong.

## Where to work

| Path | Purpose |
| --- | --- |
| `cmd/swarm/main.go` | Executable entry point calling the engine |
| `internal/engine/` | Go CLI, server, storage, orchestration and Go unit tests |
| `resources.go` | Embeds canonical assets in the binary with `go:embed` |
| `web/` | HTML/CSS/JavaScript served by the engine |
| `frontend/` | Existing frontend sources and rebuild toolchain |
| `locales/` | Translation catalogue |
| `config/` | Public defaults; secrets belong outside Git |
| `contracts/` | Data contracts and schemas |
| `tests/` | CLI, HTTP, browser and acceptance harnesses |
| `testdata/` | Data fixtures for Go tests |
| `tools/` | Verification, agent workflows and development utilities |
| `scripts/` | Auxiliary scripts, including translation and screenshots |
| `deploy/` | Container entry point and deployment files |
| `docs/` | Documentation, screenshots, plans and reports |
| `docs/training/casa-pizza/` | French/English training guides, screenshots and reference application |
| `docs/en/` | English documentation |
| `benchmarks/` | Benchmark scenarios and tools |
| `bin/` | Locally built binaries, ignored by Git |
| `test-results/` | Generated verification artifacts, ignored by Git |
| `.claude/skills/` | Canonical methods; `.agents/skills` references them |
| `.github/` | CI workflows and GitHub templates |

The root retains normal entry documents: French/English READMEs, license,
security, contributing and agent instructions, installation documentation,
Go/npm manifests, Dockerfile, Makefile and the `swarm`/`swarm.sh` launchers.

## Finding engine code

The engine remains one Go package. Existing filename prefixes identify topics
without introducing packages with circular dependencies.

| Prefix under `internal/engine` | Topic |
| --- | --- |
| `main`, `console`, `*_cli` | CLI commands and rendering |
| `web`, `prephase`, `preparation` | Web server and preparation |
| `agent`, `agents`, `provider`, `role_model` | Execution, providers and models |
| `planning`, `conductor`, `supervision`, `pilotage` | Planning and supervision |
| `managed`, `workspace`, `recovery` | Integration, workspaces and recovery |
| `validation`, `evidence`, `review`, `independent` | Checks, evidence and review |
| `graph`, `automation` | Graphs and automation |
| `storage`, `model`, `migration` | Storage and data models |
| `*_test.go` | Go tests alongside the engine code they verify |

This path migration does not split the engine into domain packages. Dated
reports may cite former root Go paths; current sources with the same filename
are under `internal/engine`. Historical manifests remain evidence of their
own candidates.

## Build and check

Run from the repository root:

```sh
make build
go test -timeout 25m ./...
go vet ./...
npm test
make smoke
python3 tools/check_distribution.py
python3 tools/agent-workflows/check.py
```

For focused cases: `go test ./internal/engine -run '^TestVersion'`.
For a development binary without injected release identity:
`go build -o bin/swarm ./cmd/swarm`. Identified builds use `build.sh`.
The root package now contains assets: `go build .` no longer builds the executable.
Moving a Go file across packages requires checking dependencies and test coverage.

## Assets and private data

`go:embed` cannot read parent directories. One root Go file therefore embeds
assets at their canonical paths, without generated copies to synchronize.

Mission databases and secrets under `.swarm`, API keys, certificates, private
Compose overrides and Skynet installation documents are excluded from publication.
A live mission database is never a test fixture. Building a candidate does not
replace the binary of an already running server.
