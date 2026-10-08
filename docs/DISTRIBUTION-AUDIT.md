# Distribution audit — 29 September 2026

Scope: GitHub `main` at `464e2b5`, plus the distribution/documentation changes
listed below. No real AI provider or abandoned mission is started by this audit.

## Findings and changes

- Primary French/English README, installation and user-guide local links resolve.
  French README now explicitly mentions both interface languages.
- `install.sh` and `deploy/entrypoint.sh` are executable and pass shell syntax checks.
  Installation runs from an exported checkout in an isolated temporary project.
- Native installation, Docker build/start, authenticated browser access, CLI use,
  refusal to overwrite a running installation, project preservation and agent-home
  persistence after container recreation pass the real installation test.
- The installation test now checks SQLite integrity, foreign keys and `0600`
  permissions after stopping its disposable container; it also creates the cache
  directory when absent on a fresh machine.
- Targeted migration/backup/lifecycle tests pass: `go test -run
  'Test(Migration|VersionTwoBackup|ProviderCooldownStorageVersion|WorkspaceCoordinationMigrates|PreparationV6Migration|PlanningArchiveAndMigration|Lifecycle|StorageInspection)'
  -count=1 -timeout 3m ./...`.
- No tracked SQLite database, `.swarm/` directory or installer environment file.
  `.gitignore` excludes local state; `.dockerignore` limits image inputs.
- Documentation clarifies startup migration, explicit CLI initialization and full
  stopped-state backup. Inspection commands intentionally refuse silent migration.
  The initial suspicion that Docker web startup skipped migration was disproved
  by inspection of `openStoreWithMigration` and `cliStorageInspection`.
- Added contribution/security guidance, issue/PR templates and a CI workflow with
  read-only repository permissions and actions pinned to verified upstream commits.
  The workflow runs Go, frontend, distribution and real Docker installation tests.
- `npm test`, shared-method checks and `git diff --check` pass. CI YAML parses locally.

## GitHub settings observed, not changed

Public repository; default branch `main`; issues enabled. `main` has no branch
protection (GitHub API returned 404). Recommended administrator follow-up: require
passing CI checks and review for pull requests once the workflow has run successfully.
Local validation of a workflow file is not a successful GitHub Actions run.

GitHub reports license `NOASSERTION`. The existing PolyForm Noncommercial text
and third-party notices remain unchanged. The README explicitly describes the
noncommercial restriction; a standard GitHub layout does not make this an
OSI-approved open-source project. There is no universal “GitHub full standard”
certification, and this report does not claim one.

## Limits

This is Linux/local Docker Engine validation, not Windows/macOS, remote Docker,
all agent sandboxes, or paid-provider authentication validation. No new full Go
suite or full independent product review was completed in this audit. The previous
full-suite run was explicitly stopped when the mission was abandoned. No production
SQLite data was modified or used as a test fixture. No release image was published.
