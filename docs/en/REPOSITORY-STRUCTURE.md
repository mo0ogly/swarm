# Swarm repository map

Swarm uses a Rust engine, CLI and HTTP server with a local HTML/CSS/JavaScript interface. The functional migration remains in progress; see [the active plan](../plans/migration-rust/PLAN.md).

| Path | Purpose |
| --- | --- |
| `rust/crates/swarm-cli/` | `swarm-rust` command entry and CLI adapters |
| `rust/crates/swarm-engine/` | RootStore, WorkEngine, persisted state and policies |
| `rust/crates/swarm-web/` | Protected HTTP routes and embedded resource inventory |
| `rust/crates/swarm-*/` | Identity, enrollment and JSON contracts |
| `rust/Cargo.toml`, `rust/Cargo.lock`, `.cargo/config.toml` | Workspace, locked dependencies, required SQLite capability |
| `web/`, `frontend/`, `locales/` | Canonical interface sources and translations |
| `contracts/`, `tests/` | Schemas, fixtures, CLI/API/browser verification |
| `tools/verification/`, `tools/rust-migration/` | Permanent recipes and qualified native builds |
| `docs/`, `docs/en/` | FR/EN docs; dated plans/evidence retain history |
| `.claude/skills/` | Canonical agent methods; `.agents/skills/` references them |
| `deploy/`, `Dockerfile`, `compose.yaml` | Local container delivery |
| `swarm`, `swarm.sh`, `build.sh`, `install.sh` | Installed CLI wrapper, lifecycle, Rust build, installer |

Rust `include_bytes!`/`include_str!` embed only declared resources. JavaScript is reused across the migration; its presence does not prove a Rust route is implemented. Go sources are no longer tracked; ignored local copies and dated proofs are references, never a runtime fallback.

Use `./swarm.sh build` for an identified local candidate, then the supported restart path. A plain development build is `cargo build --manifest-path rust/Cargo.toml --locked -p swarm-cli`. Neither automatically migrates old missions or validates every feature.

`.swarm`, credentials, databases, local deployment configuration and build caches are private. Preserve real mission data; use isolated fixture Roots for tests. Rebuilding does not replace a running process. See [installation](INSTALL.md) and [contribution](../../CONTRIBUTING.md).
