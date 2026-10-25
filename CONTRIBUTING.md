# Contributing to Swarm

Start with [AGENTS.md](AGENTS.md) and the shared
[workflow contract](tools/agent-workflows/CONTRACT.md). Contributions remain
subject to [LICENSE](LICENSE); this project uses a noncommercial license.

## Source layout

See the [repository map](docs/en/REPOSITORY-STRUCTURE.md). Product sources are in `rust/crates/`: `swarm-cli` produces `swarm-rust`, `swarm-engine` owns state and `swarm-web` serves the protected interface. `web/` and `frontend/engine-settings/` provide canonical embedded assets. Ignored Go copies are historical references, not build dependencies.

## Local development

Use Linux, Git, Bash, Python 3, Rust/Cargo (validated toolchain: 1.96.0), a C compiler for bundled SQLite, Node.js 22 and npm. Docker Engine and Compose v2 are needed for container installation checks. No real provider credentials are required.

```sh
cargo fetch --locked --manifest-path rust/Cargo.toml
npm ci --ignore-scripts --no-audit --no-fund
./swarm.sh build
cargo test --manifest-path rust/Cargo.toml --locked -p swarm-engine --lib FILTER
python3 tests/test_rust_distribution.py
npm test
python3 tools/check_distribution.py
git diff --check
```

Replace FILTER with relevant named tests and verify that tests actually ran. `make test` runs Rust library tests plus frontend tests; historical Go parity tools are separate retained references. `make smoke` uses an isolated Root. Select checks from the changed contract; do not add an arbitrary whole-test timeout. Use `./swarm.sh restart` and verify the running binary/Version dialog to install source-managed changes; a plain Cargo build does not install them.

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
