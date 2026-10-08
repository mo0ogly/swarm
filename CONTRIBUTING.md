# Contributing to Swarm

Start with [AGENTS.md](AGENTS.md) and the shared
[workflow contract](tools/agent-workflows/CONTRACT.md). Contributions remain
subject to [LICENSE](LICENSE); this project uses a noncommercial license.

## Source layout

See the [repository map](docs/en/REPOSITORY-STRUCTURE.md). The executable entry
point is `cmd/swarm`; engine sources and Go tests live in `internal/engine`.
Use `make build` for an identified binary, or `go build -o bin/swarm ./cmd/swarm`
for a plain development build. Run focused engine cases with
`go test ./internal/engine -run PATTERN`; `go test ./...` covers every package.
The root `resources.go` embeds canonical assets; no generated asset copies are required.

## Local development

Use Linux, Go 1.24 or newer, Node.js 22 and npm. Docker Engine and Compose v2
are needed for installation tests. Agent programs and API credentials are not
needed for deterministic tests.

```sh
npm ci --ignore-scripts --no-audit --no-fund
make build
npm test
go test -timeout 25m ./...
go vet ./...
make smoke
python3 tools/check_distribution.py
python3 tools/agent-workflows/check.py
git diff --check
```

Run `make test-install` in a clean checkout without `deploy/install.env`.
It builds Docker and tests persistence using a temporary project. Never use a
real mission database as a test fixture. Do not set real-provider test variables
when running deterministic CI.

Submit a branch and pull request against `main`. Describe the problem, behavior,
checks actually executed and remaining limits. Keep French and English user docs
consistent. UI changes need both themes and both languages checked. SQLite changes
need migration and preservation tests; do not ship a prepopulated database.

CI runs on pull requests and pushes to `main`. A workflow file alone does not
protect the branch: repository administrators must separately configure required
checks and review rules in GitHub settings. No claim of independent review should
be inferred from a successful build.
