# English interface validation — 19 September 2026

## Corrections

- Browser language selection persists and survives session-link redirects.
- French remains available. User-authored mission and task titles, requirements,
  criteria, reports and provider output retain their original content.
- Known engine errors are translated at the presentation boundary, including
  formatted HTTP codes and provider names. Error codes, HTTP statuses and JSON
  responses retain their original contracts.
- Planner, worker and reviewer card descriptions and planning counters are
  localized, including text composed with JavaScript template expressions.
- Preparation and technical references are available in English.

## Evidence

- Full Go suite passed after the error and session-redirect corrections.
- Focused tests passed for locale arguments, formatted engine messages and
  session-link redirects, including rejection of unsupported language values.
- Node tests passed, including literal placeholder preservation, French fallback
  and shared catalogue consistency.
- Isolated browser recipe passed for both themes, language switching, unchanged
  task data, identical French/English CLI JSON, role cards, connection forms,
  help, draft protection and translated HTTP errors with unchanged error codes.
- The real server on `127.0.0.1:18787` was restarted with the standalone binary.
  A fresh browser session opened its authenticated English link successfully.
  No JavaScript errors were observed. Both rendered themes were inspected.
- Database counts remained 16 missions and 76 recorded attempts. No agent was
  active when the server was replaced. A SQLite backup was made first.
- `git diff --check` and English documentation link checks passed.

The browser checks started no provider call and validated no mission result.
Historical reports and user content are not automatically translated. Unknown
external or system errors retain their original details. Screenshot artifacts are
local under `test-results/i18n/`; runtime credentials are not included here.
