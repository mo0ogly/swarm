# French / English verification — 29 September 2026

## Corrections

- Task-card guidance now translates both missing deliverables and missing instructions.
- The plain-text CLI console translates headings, gate labels, request labels and
  command help before formatting. Mission content is passed as formatting arguments
  and is not translated.
- The browser test's simulated team now uses valid provider identifiers. The old
  identifiers contained spaces and punctuation; the engine correctly refused them
  and the test could not load its snapshot. No runtime validation was weakened.
- The browser recipe now also captures French connection and preparation screens
  in both themes. English documentation captures were refreshed after the fix.

## Evidence

| Check | Result / scope |
|---|---|
| `go test -timeout 25m ./...` and `go vet ./...` | PASS: full Go suite (333.254 seconds), vet exit 0 |
| `npm test` | PASS: graph, overview, refresh, runner and locale contracts |
| Targeted Go language / plain-console tests | PASS: FR/EN, engine error details and preserved user content |
| `npm run test:i18n-ui` | PASS: language switch/persistence, unchanged CLI JSON and mission content, planner/reviewer graph, connection dialog, help, draft navigation guard, both themes, no JavaScript errors |
| `node scripts/readme-screenshots.cjs --lang=en` | PASS: eight screenshots, visible arrows, no AI calls |
| Local server `127.0.0.1:18789` | Updated binary; authenticated asset checked; live browser confirmed FR/EN navigation, corrected catalogue and no JavaScript errors |
| Distribution links and `git diff --check` | PASS |

Browser evidence is under `test-results/i18n/` (local, untracked). The old local
binary was retained outside the repository for rollback. No mission was active
at restart and no AI execution was requested.

## Limits

This checks presentation and language preservation, not every possible provider
error. User-entered objectives, identifiers, reports and raw provider output keep
their original language. The browser team is a fixture, not an autonomous run.
This verification does not claim a complete real-provider web-only mission.
