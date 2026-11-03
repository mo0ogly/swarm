# Access administration — APEX, 2 November 2026

Français : [../../../plans/acces-cockpit/ADMIN-APEX.md](../../../plans/acces-cockpit/ADMIN-APEX.md)

## Need

Operator request: accounts, permissions and access logs at least equivalent to
Wattson's **Tools** menu (`wattson_devcontainer/flaskProject`, Python), rebuilt to
Swarm standards (Rust, authority daemon, FR/EN, both themes, mobile, keyboard,
local assets, settings in Admin).

Operator decisions (2 November 2026):
- permission grain: **API families + one "view" permission per menu entry**;
- **account groups** (roles assigned to a group, rights = union).

Starting point: phase B (`PHASE-B-AUTORITE.md`, entries 14 to 16), cockpit view
`/?view=authority` (commit 5c9ca0fe8). The existing work had not been compared to Wattson.

## Wattson reference (inventory of 2 November 2026)

Sources: `admin/admin_rbac_routes.py`, `admin/admin_rbac_roles_routes.py`,
`admin/admin_security_routes.py`, `admin/admin_logging_routes.py`,
`core/audit_logger.py`, `core/rbac.py`.

- Accounts: no local accounts (Keycloak SSO); no user list or page.
- SSO groups: create, rename, duplicate, delete, admin flag, role; "last admin"
  and "self" guards.
- Roles: built-in (`admin`, `analyste`) and custom; matrix of 83 menu resources in
  11 sections; check/uncheck all, counter, orphan keys, read-only banners; deleting
  an assigned role is refused.
- Per-request access log: account, group, IP, /24 network, user agent (not shown),
  method, path, menu, action, status, duration.
- Connections page: period, account, network, "without admin" filters; per-session
  detail, actions filter; 1,000 rows, no pagination.
- Dashboards: 7/30/90/365 period; 4 indicators; 6 charts; last 20.
- Purge: 4 s armed confirmation, 5 calls/min, automatic purge at 365 days.
- Application log level (POST without CSRF protection).

Wattson defects not to reproduce: role cannot be removed, no export or pagination,
RBAC actions only in a file, log level without CSRF.

## Gap matrix

| Wattson | Swarm on 2 November | Requirement |
|---|---|---|
| Accounts (SSO) | account CRUD, password, disable, forgot | kept; search, sort, last sign-in, groups |
| Group CRUD + role | missing | ADM-01 |
| 83-resource / 11-section matrix | 14 flat families | ADM-02, ADM-03 |
| Menu filtered by rights | full menu | ADM-04 |
| Request log (UA, duration, menu) | method, path, status | ADM-05 |
| Connections + filters + detail | sessions, period, account | ADM-06 |
| 4 KPIs + 6 charts | per-day and per-account table | ADM-07 |
| Armed + automatic purge | manual purge | ADM-08 |
| Log level | missing | ADM-09 |
| (missing) admin actions in store | raw intent/outcome | ADM-10 |
| RBAC help | missing | ADM-11 |

IP and network: Swarm listens on loopback only; the IP is always `127.0.0.1`. It is
replaced by the **access means** (browser or API key) and a short **user agent**.

## Requirements

- **ADM-01 Groups**: groups (key, label, roles, members) in the signed account
  journal; create, rename, duplicate, delete, members, roles; effective rights =
  account roles ∪ group roles; refusals `authority-group-exists`,
  `authority-group-missing`, `authority-group-in-use`, `authority-last-admin`,
  `authority-self-lockout`; role removal possible.
- **ADM-02 View permissions**: single `view.<id>` catalog (section, label, exclusive
  API prefixes) covering every cockpit menu entry; test guard `web/index.html` ↔
  catalog; exclusive API refused without the view permission; policy schema 3,
  v2 conversion without access loss.
- **ADM-03 Matrix**: sections, check/uncheck all, counter, orphan keys, read-only
  banners, role usage (accounts, groups).
- **ADM-04 Filtered menu**: `/me` returns permitted views; empty entries and groups
  hidden; direct access without right: clear message and API 403.
- **ADM-05 Enriched log**: duration, view, access means, user agent, detail; old
  lines and chain still valid.
- **ADM-06 Connections**: columns, filters (period, account, view, means, without
  administration, status), search, sort, server pagination, detail, CSV/JSON export.
- **ADM-07 Dashboard**: period, 4 indicators, 6 local SVG charts with alternative
  table, last 20 connections.
- **ADM-08 Purge**: armed confirmation, rate limit, automatic retention
  `audit_retention_days` set in Admin (empty = no automatic purge).
- **ADM-09 Daemon log**: adjustable `log_level`; daemon diagnostics kept in a
  bounded file and readable in Admin.
- **ADM-10 Administration actions**: readable detail of every administration
  mutation, filterable tab.
- **ADM-11 Help and ergonomics**: collapsible help per section, confirmations
  without `prompt()`, keyboard, mobile, FR/EN, both themes.

## Tasks

| T | Requirements | Scope | Acceptance | State |
|---|---|---|---|---|
| T0 | — | this FR/EN register | register reviewed | done |
| T1 | ADM-02 | `swarm-authority/src/policy.rs`, defaults, `tests/policy.rs` | catalog, schema 3, conversion, menu guard, 403 | done (local check) |
| T2 | ADM-01 | `accounts.rs`, `admin.rs`, `tests/admin_api.rs` | groups, union, last admin | done (local check) |
| T3 | ADM-05, ADM-10 | `audit.rs`, `server.rs`, `admin.rs` | enriched fields, compatibility | done (local check) |
| T4 | ADM-06, ADM-07 | `audit.rs`, `admin.rs` | filters, pagination, export, KPIs | done (local check) |
| T5 | ADM-08, ADM-09 | `config.rs`, `server.rs` | retention, level, diagnostics | done (local check) |
| T6 | ADM-03, 04, 06–11 | `web/*authority*.js`, `admin.css`, translations | complete interface | done (local check) |
| T7 | all | `tests/authority_admin_ui.cjs`, recipe | browser FR/EN, themes, mobile, keyboard | done (local check) |
| T8 | — | FR/EN documentation | real behavior and limits | done |
| T9 | — | `./swarm.sh authority install` (operator sudo), `./swarm.sh restart` | verified binary, live flow | to do |

## Limits

- APIs shared between views stay governed by their family (no per-object filtering).
- No SSO or real-time alerting; IP not meaningful on loopback.
- Installing the daemon needs `sudo`, run by the operator.

## Evidence

### T1 (2026-11-02)

- Command: `cargo test -p swarm-authority --test policy` then
  `--test accounts --test audit --test assertion --test check --test admin_api`.
- Result: policy 8/8, other suites 6/6. The full run carries one pre-existing
  unrelated failure: `tests/front_e2e.rs` requires `SWARM_ENGINE_BINARY`
  (prebuilt binary absent from this development environment).
- Verified content: `VIEWS` (16 views), `SCHEMA_VERSION` 3 with v1/v2
  conversion (`view_family_set`, any-of rule — a view is earned through its
  families but never grants the family), distinct `policy-no-view` refusal
  (403, reason in the body), Biba floor on view keys (read), `/me` returns
  `views`, `/policy` exposes the view catalog.
- Test guard `views_match_menu`: bijection between `web/index.html`
  `data-view` (12 entries) + the `authority` entry injected by
  `web/cockpit-authority.js` + the 3 standalone pages, and the catalog;
  exclusive prefixes unique per view.
- Guard `v2_conversion_preserves_access`: the archived v2 default (commit
  562953dba3) converted equals the shipped v3 default; previously allowed v2
  requests preserved; unknown v2 key refused.
- Test defects fixed along the way (lessons): a POST `/api/v1/agent-logs/clear`
  without the right falls on `unclassified.write` (Biba floor 3), not
  `policy-no-grant` — raise the subject to I3; lossless-conversion spot
  checks must only cover what the role actually held in v2.
- No writes under `/tmp` by the tests (temporaries via `CARGO_TARGET_TMPDIR`).
- Limit not covered here: the server-side 403 with body
  `{"error":"authority-policy-denied","reason":"policy-no-view"}` will be
  proven by the T7 browser recipe (the `decide()` decision itself is
  already verified).

### T2 (2026-11-02)

- Command: `cargo test -p swarm-authority` from `rust/` (including
  `--test admin_api`, 120 s, 1/1 green). Same pre-existing unrelated failure
  as T1: `tests/front_e2e.rs` requires `SWARM_ENGINE_BINARY`.
- Signed journal: groups as records `group/<key>/<nonce>` (`GroupBody`
  schema 1, `deny_unknown_fields`), same signing key and same serial mutex
  as accounts; no generation — the current group is the active record with
  the highest `enrolled_sequence`; every change (label, roles, members)
  writes a new record and revokes the others; `group-body-mismatch` on a
  key/schema inconsistency.
- Effective rights union: `Accounts::current()` and `Accounts::list()`
  return the **effective** roles (own ∪ roles of active groups the account
  belongs to, sorted, deduplicated). Sessions, API keys, signed assertions,
  policy decisions and `/me` inherit without further change; a membership
  change applies at the account's next request, without ending sessions.
- API `…/authority/groups`: GET list, POST create (`group-exists` 409),
  `…/K/profile` (roles/members, role removal allowed), `…/K/rename {key}`
  (new key, old lineage revoked, old key reusable — tested),
  `…/K/duplicate` (copies roles and members — tested), `…/K/delete`
  (`group-missing` 404 when absent). Input refusals: `group-key-invalid`,
  `group-label-invalid`, `group-roles-invalid`, `group-members-invalid`
  (400).
- Safety guards (evaluated over the **candidate** effective state against
  the live policy): `authority-self-lockout` (409) when the caller would
  lose administration (tested: dan cannot delete his own admin group),
  `authority-last-admin` (409) when no account would still administer.
  The last-admin guard is defense in depth: not reachable in the test
  because `authority-self-lockout` already blocks demoting the only other
  admin — documented, not covered.
- Design note kept: the guard reads `list()` (already-effective roles),
  then subtracts the groups' current contribution to recover each account's
  own roles before re-uniting with the candidate state — an initial bug
  passed `&[]` and demoted everyone (caught by the test, fixed; regression
  guard = the e2e test).
- No writes under `/tmp` by the tests; module documentation (`accounts.rs`,
  `admin.rs`) updated in the same increment.

### T3 (2026-11-02)

- Commands: `cargo test -p swarm-authority --test admin_api` (1/1 green)
  then `--test audit` (2/2). Same pre-existing unrelated failure as T1/T2:
  `tests/front_e2e.rs` requires `SWARM_ENGINE_BINARY`.
- Enriched (retro-compatible) fields: `duration_ms`, `view`, `client`
  (`browser`|`api-key`), `user_agent` (truncated to 120), `detail` — all
  optional with `serde(default, skip_serializing_if)`: a pre-existing line
  decodes unchanged and the chain stays valid (test
  `pre_enrichment_lines_still_verify`: a pre-enrichment line then an
  enriched one chained onto it; `prev` stays the last field).
- Capture: `Meta` struct in `server.rs` (UA, client, arrival instant) built
  once per request; `outcome` carries `duration_ms`; `intent` and `outcome`
  carry `view`/`client`/`user_agent`; `note` (refusals, consultations)
  carries `view`/`client`/`user_agent`.
- Session events: `sign-in`, `sign-in-refused`, `sign-out`,
  `reset-requested` carry `client`+`user_agent` (`/login` is not an
  exclusive view → no `view` field).
- Readable detail (ADM-10): `audit::admin_detail(method, path)` describes
  the action from the path alone (never the body → no secret): "account
  created" (subject is in the unread body), "password reset: X", "group
  renamed: X", "policy applied", etc. Carried by the `intent` line (written
  before execution); engine relays keep `None` (the path is already the
  detail).
- `audit::view_of(path)`: exclusive prefix → its `view.*` key, else
  `view.authority` for the administration API, else `none`.
- Contract improvement found by the test: relay infrastructure failures
  (unreachable engine, 502) bypassed the journaling block — a page visit
  was not traced when the engine was absent. Fixed: the consultation note
  is written on the early 502 returns too; the e2e assertion (navigation
  `Accept: text/html` without an engine → 502 + consultation line with
  `view: "none"`) is the regression guard.
- Policy refusals: the `refused` line now carries view, client and UA
  (useful to tell browser from API key in the connections view).
- No writes under `/tmp` by the tests; Rustdoc of the modified functions
  (`note`, `intent`, `outcome`, `Meta`, `admin_detail`, `view_of`) updated
  in the same increment.

### T4 (2026-11-02)

- Commands: `cargo test -p swarm-authority --test admin_api` (1/1 green,
  150 s) then `--test audit --test accounts` (3/3). Same pre-existing
  unrelated failure as T1–T3: `tests/front_e2e.rs` requires
  `SWARM_ENGINE_BINARY`.
- Connections (ADM-06): `SessionSummary` gains `clients` (access means seen
  on the session's lines: `browser`, `api-key`) and `views` (`view.*` keys
  touched, `none` included). Route `GET …/audit/sessions`: filters
  `subject`, `client`, `view`, `no_admin=1` (sessions of accounts without an
  `administer` decision on POST …/accounts under the live policy),
  `status=active|signed_out|refused`, search `q` (account or credential id,
  case-insensitive), sort `sort=last_at|first_at|subject|actions|refusals`
  with `dir=asc|desc` (desc default, asc for `subject`), server-side
  pagination `page`/`per_page` (default 50, max 200) with `total`, export
  `format=csv|json` **without pagination** (`Content-Disposition:
  attachment` header). Unknown `status` → 400 `authority-request-invalid`.
- Dashboard (ADM-07): `GET …/audit/dashboard` also returns `kpis`
  (`sign_ins`, `refused_sign_ins`, `actions`, `refusals` — the four
  headline counters), `views` (`audit::view_usage`: consultations/actions/
  refusals per view key, `none` groups engine pages without an exclusive
  view, sorted by consultations then actions descending) and `recent` (the
  20 most recent lines of the period, purge anchor excluded). The 6 SVG
  charts and the alternative table are built UI-side (T6) from
  `days`/`views`/`kpis` — same data, no external call.
- `no_admin` filter: a real `policy::decide(POST …/accounts)` decision per
  account (effective roles, Biba floors included), not a role guess;
  the accounts list is read on the blocking pool.
- No writes under `/tmp`; Rustdoc (`sessions`, `kpis`, `view_usage`,
  `admin.rs` module doc) updated in the same increment.
- Limit: `recent` returns raw lines (event, subject, path, status) — the
  Swarm equivalent of Wattson's "last 20 connections", to be read as visits
  rather than sessions.

### T5 (2026-11-02)

- Commands: `cargo test -p swarm-authority --test admin_api` (1/1 green,
  171 s) then `--test sessions` (2/2). Same pre-existing unrelated failure
  as T1–T4: `tests/front_e2e.rs` requires `SWARM_ENGINE_BINARY`.
- Configuration (ADM-08/09): two fields, `audit_retention_days: Option<i64>`
  (absent/`null` = no automatic purge; otherwise ≥ 1, 0 refused — tested)
  and `log_level: String` (`error|warn|info|debug`, default `info`), both
  `serde(default)`: earlier `authority.json` documents still decode
  (tested); validation refuses an unknown level. Embedded default: 365
  days, `info` (asserted in `tests/sessions.rs`).
- Armed purge (ADM-08): the manual `POST …/audit/purge` is rate-limited to
  5 calls per rolling minute (`Front::consume_purge_slot`, in-memory
  timestamps) — the 6th call answers 429 `authority-purge-rate` with
  `Retry-After: 60` (tested: 5 × 200 then 429 back to back). E2E proof
  needed a fix: the test's first two calls were invalid (not counted) and
  the suite spans over a minute — six consecutive calls prove the window.
- Automatic purge: daemon background task (`spawn_maintenance`, purge at
  start-up then every 6 h) following `audit_retention_days`; actor
  `automatic` in the purge anchor; a failure (broken chain, IO) is traced
  in the diagnostics log, never blocking. Not tested in real time (6 h
  window) — the underlying purge is the same as the manual one, already
  tested.
- Daemon log (ADM-09): new `diag.rs` module — a bounded JSONL file
  `authority.log` (8 MiB, front-truncated by half, atomic write via
  `.tmp`+rename+fsync), a minimum level filtering lines, `GET
  …/authority/log` returns the last 200 lines + current level + applied
  level; the level follows the config applied live (tested: `debug`
  applied → `/log` reflects it). Writes are best effort: a diagnostics
  failure never affects a response.
- No writes under `/tmp` (verified); Rustdoc and docstrings (`diag.rs`
  whole, `config.rs`, `sessions.rs`, `admin.rs`) updated in the same
  increment.
- Limit: the automatic purge does not go through the rate limit — it calls
  `audit.purge` directly (not the endpoint); only the manual path is
  limited.

### T6 (2026-11-03)

- Complete interface (ADM-01/03/04/06–11), three files: `web/native-authority-admin.js`
  (authority mount), `web/cockpit-authority.js` (menu filtering),
  `frontend/engine-settings/admin.css` (styles), plus `locales/en.json`
  (+74 keys) and regenerated `web/i18n-en.js` (`npm run i18n:build`, pure
  insertion verified: 0 deleted lines).
- Groups (ADM-01 UI): table with Create/Edit/Duplicate/Delete calling the
  exact server contracts (`POST /groups`, `/profile`, `/rename`,
  `/duplicate`, `/delete`); changing the identifier while editing becomes
  a rename followed by a profile update.
- Matrix (ADM-03): catalog sections with Check all/Clear all (the admin
  role excluded by construction), `view.<id>` rows shown, orphan keys
  reported as a warning, per-role counters (held rights + real usage:
  N accounts, M groups); deferred refresh when another panel reloads
  data.
- Menu (ADM-04): `cockpit-authority.js` hides non-permitted entries and
  emptied navigation groups (groups containing `<a>` links —
  preparation/trust/PAEA — always stay visible); `cockpit-native.js`
  refuses direct access to a non-permitted view with the clear message,
  and the API remains the authority (server 403 proven in T1). The
  `views_match_menu` guard (index.html ⇄ `policy::VIEWS` bijection) is
  still green after adding the `authority` entry: 8/8
  (`cargo test -p swarm-authority --test policy`).
- Access log (ADM-06/10): connections with filters (dates, subject, view,
  client, admin excluded, status, search), clickable sort, pagination
  (« ‹ › », 50/page, server total), CSV/JSON export via
  `Content-Disposition` (pagination ignored on export, T4 contract);
  enriched detail (No., date, event, request, view, client, result,
  duration, detail).
- Dashboard (ADM-07): 4 KPIs (`sign_ins`, `refused_sign_ins`, `actions`,
  `refusals`), 6 local SVG charts (bars, `viewBox`, per-bar tooltip) —
  sign-ins/actions/consultations/refusals per day and
  consultations/actions per view (`none` shown as "engine pages"),
  collapsible alternative table per chart, accounts, latest 20 entries.
- Administration actions (ADM-10): tab fed by `GET /audit` filtered
  client-side on `event === 'intent'` plus the filters' date window (the
  server exposes no `event` parameter — documented limit; volumes stay
  bounded by the chosen period).
- Daemon log (ADM-09): tab reading `GET /log` (last 200 lines, bounded
  scrolling `pre`) and live level change (structured edit of the current
  config + `POST /config` with `revision`, re-read after write).
- Purge and retention (ADM-08): manual purge (regex + dialog confirmation
  + server rate limit) and the `audit_retention_days` field (empty =
  delete the field = disabled, integer ≥ 1 otherwise, re-read after
  write).
- Help (ADM-11): one collapsible block per section (`<details>`, touch
  height ≥ 44 px).
- Checks: `node --check` green on the three JS files; `cargo test
  -p swarm-web` — 15 suites, 0 failures (green lib compile after fixing
  the `Event` initializer in `authority_ledger.rs` with the five T3
  fields, and `native_admin.rs` brought back in line with the real
  contract: `/` has served the cockpit page since 0aa7335e18, its
  obsolete `app.js` assertion replaced with `cockpit-native.js`, and its
  fixture moved from `std::env::temp_dir()` to `.swarm-fixtures/`
  per the `/tmp` interdiction).
- Limits: the `blp-revision-conflict` failure of `tests/paea_git_call.rs`
  run in bulk (`cargo test --test paea_git_call`) is a race between
  parallel tests over the shared declared fixture — each isolated test
  (or the permanent recipe, which runs every test sequentially with
  `--exact`) is green; defect of the other lot, not modified here (the
  file was being edited by a concurrent session at diagnosis time).
- Visual verification (themes, mobile, keyboard): deferred to T7 (browser
  recipe) — this task does not prove rendering.

### T7 (2026-11-03)

- Real browser journey (`tests/authority_admin_ui.cjs`): a real
  `swarm-authority serve` daemon in front of a real engine embedding the web
  assets, sign-in through `/login` then — account creation with focus
  restoration, refusal variant, API key (secret shown once, Bearer
  afterwards), habilitations matrix with a custom role + save + offered on
  account creation, access journal (connections → action detail → dashboard
  → purge refused without valid days), FR/EN × light/dark (per-card
  screenshots, full English translation verified key by key), 390 px with
  no global overflow, mobile sign-in.
- Defect fixed along the way (cause retained): the rendering-wait predicate
  required **all** `<details>` blocks to be filled, including « Aide »
  (never a table) — latency masked by timing until instrumentation; the
  correct contract (every **open** block filled) lives in `cardReady()`
  with a 60 s bound justified by observation (rendering ≤ 26 s, cold
  Chrome, 500 ms polling) and a failure diagnostic keeping the rendered
  state (details/open/filled, buttons present).
- Permanent recipe `tools/verification/recipes/authority-admin-ui-rust.json`
  (i18n `--check` then managed-binary qualification through the wrapper
  `tests/authority_admin_ui_recipe.py` — binaries supplied, never rebuilt;
  SHA-256 of engine and daemon frozen in `result.json`; fresh output
  required; /tmp refused before any allocation): `recipe.py run` PASS
  (2 steps) then `recipe.py verify`
  `{"integrity":"VERIFIED","recorded_status":"PASS"}` — evidence retained
  under `.swarm-fixtures/` (older qualification outputs deleted after the
  retained proof was kept; no project residue under /tmp).
- 41 named checks, 0 failures (the earlier 42 proof counted the removed
  « debug-after-login » — proof counter aligned in the wrapper).
- Limits: local verification on supplied qualified binaries — neither
  independent acceptance nor installation (T9); the recipe does not
  exercise remote flows (relay) or key rotation beyond secret-shown-once.

### T8 (2026-11-03)

- Product documentation FR/EN: new section « Administration “Comptes et
  accès” (authority mode) » in `docs/architecture/NATIVE-LOCAL-LAUNCHER.md`
  and `docs/en/NATIVE-LOCAL-LAUNCHER.md` — real scope (accounts, one-time
  API keys, groups/union, 30-row admin-locked matrix, journal/filters/
  export/KPIs, purge and retention, daemon settings with `revision`), API
  contract `/api/v1/authority/*`, families + `view.<id>` granularity,
  deny-by-default and Biba floors, signed hash-chained journal, permanent
  recipe and its limits (no independent acceptance, relay and rotation out
  of scope).
- Plan register: T7/T8 sections added above and the table updated.
