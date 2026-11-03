#!/usr/bin/env node
/**
 * Component regression for the PA-09 reconciliation-report reader on
 * /paea-effects.html — isolated browser, explicit HTTP doubles.
 *
 * Scope of proof (component only, NOT a native/delivered-service proof):
 *   - the report reader field and its explicit re-read command exist, are
 *     FR/EN labeled, keyboard reachable and mobile-usable in both themes;
 *   - the reconcile GET is the ONLY request carrying the reader credential
 *     (Authorization: Bearer), read from the ephemeral input at request time,
 *     never placed in a URL, never present in web storage, DOM snapshot,
 *     test log or evidence after any exit;
 *   - the credential is erased at EVERY exit: success, typed refusal, parse
 *     failure, network failure;
 *   - every read/refusal first purges ALL previous report DOM data (both
 *     domains, findings, technical pre, collapsed container);
 *   - the initial no-credential read stays an explicit typed refusal
 *     (paea-reader-authentication-required), never a fake success;
 *   - a stale concurrent response — including the held-back initial load —
 *     can never re-display report data after a newer refusal or re-read;
 *   - positive recovery through the same public operation after refusals;
 *   - no mutant request is ever sent and no other endpoint changes contract.
 *
 * Explicit test doubles: every HTTP response comes from the labeled double
 * server in this file (header `X-Test-Double: paea-reconcile-reader-ui`);
 * the served page assets are the real repository files. The Bearer values
 * below are test fixtures against doubles only — no real identity, no
 * simulated authentication passed off as real. Per-phase controls decide the
 * double's replies; the positive witness (SUCCESS_REPORT) is configured at
 * the right moment only — AFTER the initial typed refusal is observed,
 * BEFORE the credentialed re-read — never before, never falsified into a
 * real authority. The native/proxy/public-asset/version proof against the
 * delivered binary remains a separate lot.
 *
 * Failure discipline: every named assertion is journaled; any failed
 * assertion sets a nonzero process exit code; pageerror events fail the run;
 * the raw double-request log and the journal are written under the run
 * output directory (fixture area, never /tmp) and stdout/stderr stay
 * available for retention by the runner. No global watchdog, no sleep, no
 * polling timer and no grep loop: every wait is driven by an explicit event
 * or promise. Races are forced by the runner, not by timing: a control may
 * mark its response `hold`, which the double withholds until the runner
 * calls `release(entry)`, so the response order is causal, not clocked.
 * Waits on the DOM (waitForFunction/waitForSelector) observe real page
 * states and never bound the process; Puppeteer navigation/action timeouts
 * are explicitly disabled (`timeout: 0` via the public API) because the
 * only admissible end conditions are the assertions themselves. A status
 * wait ends as soon as a TERMINAL state renders: the awaited text resolves
 * it, a contradicting terminal state (e.g. a typed refusal while the success
 * text is awaited) ends the wait with expected vs observed instead of
 * waiting forever, and the in-flight 'loading' state is never a result. A
 * page close or navigation during a wait surfaces its own cause. Every
 * terminal double issue (reply, abort,
 * withheld response released, close) settles the entry's `done` promise;
 * a cleanup that ends the run while promises are still pending rejects
 * them with the cause instead of masking it.
 *
 * Usage: node tests/paea_reconcile_reader_ui.cjs <parent-dir>
 *   <parent-dir> is the PARENT under which a FRESH per-run directory is
 *   allocated with fs.mkdtempSync (0700) — never reused, never overwritten:
 *   evidence-*.json, the run manifest and the browser profile of a run live
 *   only in that fresh directory, whose exact path is printed on stdout
 *   (PAEA_RECONCILE_READER_UI_RUN_DIR) and recorded in run-manifest.json.
 *   Without an argument the parent is a fixture root from ./fixture_area.cjs
 *   (never /tmp). Every fs allocation error fails the run without a PASS.
 * Guard mode (no browser, no fixture output): PAEA_RECONCILE_DOUBLE_GUARD=1
 * node tests/paea_reconcile_reader_ui.cjs — runs only the DoubleServer
 * close/wait regression guard below and exits from it alone.
 */
'use strict';
const fs = require('fs');
const http = require('http');
const path = require('path');
const puppeteer = require('puppeteer');
const fixtureArea = require('./fixture_area.cjs');

/** Repository root, derived from this test file's location: this file lives in
 * `<repo>/tests/`, so every served asset path (`web/…`) resolves against the
 * real repository checkout, never a `tests/web/` shadow. Precondition: this
 * file stays in `<repo>/tests/`; move it and this constant must move too.
 * fs errors from a missing/unreadable asset propagate to the caller (no
 * silent 404 substitution hides a wrong root). */
const MAIN = path.join(__dirname, '..');

/** Test-fixture credentials (doubles only; never a real token). Alphanumeric
 * shapes without whitespace, mirroring the producer's `credential_valid`
 * minimum while staying clearly synthetic. */
const CREDENTIAL = 'rrcred0123456789abcdef';
const CREDENTIAL2 = 'rrcredfedcba9876543210';
const CREDENTIAL3 = 'rrcredkeyboard00a0';

/** Typed refusal bodies served by the doubles (producer contract: typed
 * `{"schema_version":1,"error":code,"failure":{"code":code}}`, HTTP 403). */
const BARE_REFUSAL = { schema_version: 1, error: 'paea-reader-authentication-required', failure: { code: 'paea-reader-authentication-required' } };
const STALE_REFUSAL = { schema_version: 1, error: 'paea-reader-policy-stale', failure: { code: 'paea-reader-policy-stale' }, settlement: { state: 'consistent', effect_count: 9 } };
/** A real-shaped non-empty reconciliation report (double fixture data). */
const SUCCESS_REPORT = {
  schema_version: 1,
  settlement: {
    state: 'consistent',
    effect_count: 3,
    findings: [
      { intent_id: 'a-1', action_digest: 'aa11', verified: true, refusals: [] },
      { intent_id: 'a-2', action_digest: 'bb22', verified: true, refusals: [] },
      { intent_id: 'a-3', action_digest: 'cc33', verified: false, refusals: ['paea-reader-attribution-required'] }
    ]
  },
  git: {
    state: 'corrupt',
    codes: ['g-ref-1', 'g-ref-2'],
    findings: [
      { intent_id: 'g-7', action_digest: 'be22', verified: false, refusals: ['g-ref-1'] }
    ]
  }
};
const PREVIEW_UNAVAILABLE = { schema_version: 1, available: false, reason: 'test-double-preview-unavailable' };
const GITCALL_UNBOUND = { schema_version: 1, bound: false, capability: null };
const NATIVE_SESSION = { schema_version: 1, csrf_token: 'a'.repeat(64), csrf_format: 'hmac-sha256-v1', legacy_csrf_header: true, host: '127.0.0.1' };
const APPROVAL_UNBOUND = { schema_version: 1, bound: false, capability: null };

/** Read a real repository file, `rel` being relative to the repository root
 * (e.g. `web/paea-effects.html`). fs errors propagate (ENOENT stays loud). */
function readRel(rel) { return fs.readFileSync(path.join(MAIN, rel)); }

/** Explicitly labeled HTTP double server. Every response carries
 * X-Test-Double. API controls are per-kind and mutable between phases;
 * `sequence` serves its entries in order (staying on the last one) to stage
 * concurrent-response races. A control with `hold: true` withholds its
 * response until the runner explicitly calls `release(entry)` — the race
 * order is thus forced by the runner, never by a timer. Each request entry
 * exposes `done`, a promise settled on every terminal issue (response sent,
 * aborted connection, close); if the double is closed while responses are
 * still withheld, those `done` promises reject with the close cause instead
 * of hanging — and the abandoned HTTP response itself is DESTROYED before the
 * runner waits for the real `http.Server.close`, which would otherwise wait
 * indefinitely on the unfinished connection. `waitForRequests(kind, n)`
 * resolves on the captured-request event, so the runner awaits the REAL
 * arrival of a request instead of assuming it; once the double is closed it
 * rejects with the retained cause instead of enqueueing forever. */
class DoubleServer {
  constructor() {
    this.requests = [];
    this.held = [];
    this.waiters = [];
    this.server = null;
    this.port = 0;
    this.closed = null;
    this.defaults = {
      'reconcile': { status: 403, body: BARE_REFUSAL },
      'preview': { status: 200, body: PREVIEW_UNAVAILABLE },
      'gitcall': { status: 200, body: GITCALL_UNBOUND },
      'native-session': { status: 200, body: NATIVE_SESSION },
      'approval-status': { status: 200, body: APPROVAL_UNBOUND },
      'approval-presentation': { status: 200, body: { schema_version: 1, available: false, reason: 'test-double-presentation-unavailable' } },
      'approval-post': { status: 403, body: { schema_version: 1, error: 'web-csrf-refused', failure: { code: 'web-csrf-refused' } } }
    };
    this.control = JSON.parse(JSON.stringify(this.defaults));
    this.seqIndex = {};
  }
  setControl(kind, c) { this.control[kind] = c; }
  resetControls() { this.control = JSON.parse(JSON.stringify(this.defaults)); this.seqIndex = {}; }
  async listen() {
    this.server = http.createServer((req, res) => this.route(req, res));
    // Server-level errors must surface through the same retained-cause
    // channel as close(), never as an unhandled 'error' event that would
    // crash the runner and mask the phase diagnosis.
    this.server.on('error', err => this.noteClosure(err || Error('double server error event')));
    await new Promise((ok, ko) => { this.server.once('error', ko); this.server.listen(0, '127.0.0.1', () => { this.port = this.server.address().port; ok(); }); });
  }
  /** Record THE closure cause once; idempotent (first cause wins, later calls
   * keep the original so the diagnosis never drifts). */
  noteClosure(err) {
    this.closed = this.closed || (err || Error('double server closed while a response was still held'));
    return this.closed;
  }
  /** Close the double. Any response still withheld by `hold` is abandoned:
   * its HTTP response is DESTROYED first (a real `http.Server.close()` waits
   * for open connections and would hang behind an unfinished response), and
   * its entry `done` promise REJECTS with the retained cause so a runner
   * awaiting it fails loudly instead of hanging or seeing a silent success.
   * Pending `waitForRequests` are rejected with the same cause. A rejection
   * guard keeps every cause observable to its awaiter without becoming
   * unhandled-rejection noise that would mask the original FATAL diagnosis
   * after cleanup. Idempotent: a second close resolves without effect. */
  async close() {
    if (!this.server) return;
    const cause = this.noteClosure();
    for (const h of this.held.splice(0)) {
      h.entry.markDone(cause); // reject done with the cause, first
      h.entry.done.catch(() => {}); // keep the cause observable, not unhandled
      if (h.res && typeof h.res.destroy === 'function') h.res.destroy(cause); // terminate the abandoned HTTP response
    }
    for (const w of this.waiters.splice(0)) { w.ko(cause); if (w.promise) w.promise.catch(() => {}); }
    const s = this.server;
    this.server = null; // a later close() becomes a no-op, not a double wait
    await new Promise(ok => s.close(ok)); // now terminates: no held connection remains
  }
  /** Resolve the first N captured requests of `kind` being reached, or reject
   * with the retained cause once the double is closed — ALWAYS rejected after
   * close, even if the count is already reached (no false resolution past the
   * closure). Event-driven: no polling, no assumed arrival, no waiter
   * enqueued after close. `baseline` is the count already observed before the
   * caller's own action, so a phase awaits only ITS requests (`baseline + k`),
   * never a cumulative count polluted by previous phases. */
  waitForRequests(kind, n, baseline) {
    const b = baseline ?? 0;
    if (this.closed) { const p = Promise.reject(this.closed); p.catch(() => {}); return p; }
    const hit = () => this.requests.filter(r => r.kind === kind).length >= b + n;
    if (hit()) return Promise.resolve();
    let ok, ko;
    const promise = new Promise((res, rej) => { ok = res; ko = rej; });
    this.waiters.push({ kind, n: b + n, ok, ko, promise });
    return promise;
  }
  /** Explicitly release a held response. The runner's causal act that lets
   * the withheld response reach the page; idempotent. */
  release(entry) {
    const i = this.held.findIndex(h => h.entry === entry);
    if (i < 0) return; // already released, aborted or closed
    const h = this.held.splice(i, 1)[0];
    this.reply(h.entry, h.res, h.c);
  }
  route(req, res) {
    const u = new URL(req.url, 'http://double.invalid');
    const p = u.pathname;
    let kind = null;
    if (req.method === 'POST' && p === '/api/v1/paea/git-call/approval') kind = 'approval-post';
    else if (p === '/api/v1/paea/reconcile-effects') kind = 'reconcile';
    else if (p === '/api/v1/paea/git-call/preview') kind = 'preview';
    else if (p === '/api/v1/paea/git-call') kind = 'gitcall';
    else if (p === '/api/v1/native-session') kind = 'native-session';
    else if (p === '/api/v1/paea/git-call/approval') kind = 'approval-status';
    else if (p === '/api/v1/paea/git-call/approval/presentation') kind = 'approval-presentation';
    if (!kind) return this.asset(p, res);
    this.api(kind, req, res, u);
  }
  api(kind, req, res, u) {
    const entry = {
      kind,
      method: req.method,
      url: u.pathname,
      query: Object.fromEntries(u.searchParams),
      authorization: req.headers.authorization ?? null
    };
    entry.done = new Promise((ok, ko) => { entry.markDone = err => { if (err) ko(err); else ok(); }; });
    this.requests.push(entry);
    for (const w of this.waiters.splice(0)) {
      if (this.requests.filter(r => r.kind === w.kind).length >= w.n) w.ok();
      else this.waiters.push(w);
    }
    const c = this.pick(kind);
    if (c.abort) { res.destroy(); entry.markDone(); return; }
    if (c.hold) { this.held.push({ entry, c, res }); return; } // released only by the runner
    this.reply(entry, res, c);
  }
  pick(kind) {
    const c = this.control[kind];
    if (!c) return { status: 404, body: { schema_version: 1, error: 'test-double-no-control', failure: { code: 'test-double-no-control' } } };
    if (Array.isArray(c.sequence)) {
      const i = this.seqIndex[kind] ?? 0;
      this.seqIndex[kind] = Math.min(i + 1, c.sequence.length - 1);
      return c.sequence[i];
    }
    return c;
  }
  reply(entry, res, c) {
    const headers = { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'no-store', 'X-Test-Double': 'paea-reconcile-reader-ui' };
    res.writeHead(c.status ?? 200, headers);
    res.end(typeof c.body === 'string' ? c.body : JSON.stringify(c.body));
    entry.markDone();
  }
  asset(p, res) {
    const map = {
      '/paea-effects.html': ['web/paea-effects.html', 'text/html; charset=utf-8'],
      '/paea-effects.js': ['web/paea-effects.js', 'text/javascript; charset=utf-8'],
      '/paea-approval-form.js': ['web/paea-approval-form.js', 'text/javascript; charset=utf-8'],
      '/i18n.js': ['web/i18n.js', 'text/javascript; charset=utf-8'],
      '/i18n-en.js': ['web/i18n-en.js', 'text/javascript; charset=utf-8'],
      '/paea.css': ['web/paea.css', 'text/css; charset=utf-8'],
      '/swarm-design.css': ['web/swarm-design.css', 'text/css; charset=utf-8'],
      '/wattson_themes.css': ['web/wattson_themes.css', 'text/css; charset=utf-8'],
      '/swarm-logo.png': ['web/swarm-logo.png', 'image/png'],
      '/paea.html': ['web/paea.html', 'text/html; charset=utf-8']
    };
    const hit = map[p];
    if (!hit) { res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8', 'X-Test-Double': 'paea-reconcile-reader-ui' }); res.end('test-double-missing-asset'); return; }
    try {
      const body = readRel(hit[0]);
      res.writeHead(200, { 'Content-Type': hit[1], 'Cache-Control': 'no-store', 'X-Test-Double': 'paea-reconcile-reader-ui' });
      res.end(body);
    } catch (_err) {
      res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8', 'X-Test-Double': 'paea-reconcile-reader-ui' });
      res.end('test-double-missing-asset');
    }
  }
}

/** Allocate a FRESH per-run evidence directory under `parent` — the ONE shared
 * allocator consumed by BOTH the browser run and the regression guard, so the
 * guard exercises the real allocation path.
 *
 * Purpose: two executions under the same parent can never overwrite each
 * other's evidence-*.json or browser-profile, and no pre-existing file is ever
 * removed (this code contains no deletion).
 * Input: `parent`, an absolute path (relative values are resolved against
 * process.cwd()). Precondition: the parent exists or can be created by the
 * caller's own prior step (this function does NOT create the parent); for the
 * no-argument run the parent is a fixture root from ./fixture_area.cjs.
 * Output: object `{ dir, parent, manifest }` where `dir` is a brand-new
 * `run-XXXXXXXX` directory (fs.mkdtempSync, mode 0700) and `manifest` is the
 * sentinel object written to `dir/run-manifest.json` (0600) BEFORE any run
 * output: run_dir, parent, parent_from_argv, monotonic instant
 * (process.hrtime.bigint()), pid, guard_mode.
 * Errors: a parent literally '/tmp' or under '/tmp' is refused with its cause
 * BEFORE any filesystem write (no /tmp allocation ever); every fs error
 * (missing parent => ENOENT, permission, ENOSPC…) propagates to the caller —
 * no fallback, no substitution: a failed allocation can never become a PASS.
 * Side effect: creates one directory + one file under `parent` and prints
 * `PAEA_RECONCILE_READER_UI_RUN_DIR <dir>` on stdout. */
function allocateRunDirectory(parent) {
  if (!path.isAbsolute(parent)) parent = path.resolve(process.cwd(), parent);
  if (parent === '/tmp' || parent.startsWith('/tmp/')) throw Error('/tmp interdiction: ' + parent);
  const dir = fs.mkdtempSync(path.join(parent, 'run-'), { mode: 0o700 });
  const manifest = {
    run_dir: dir,
    parent,
    parent_from_argv: process.argv[2] !== undefined,
    started_monotonic_ns: process.hrtime.bigint().toString(),
    pid: process.pid,
    guard_mode: process.env.PAEA_RECONCILE_DOUBLE_GUARD === '1'
  };
  fs.writeFileSync(path.join(dir, 'run-manifest.json'), JSON.stringify(manifest, null, 2) + '\n', { mode: 0o600 });
  console.log('PAEA_RECONCILE_READER_UI_RUN_DIR ' + dir);
  return { dir, parent, manifest };
}

/** Classify one observed reconcile status against the awaited text — the ONE
 * classifier consumed by BOTH waitText (browser run) and the regression
 * guard, so the contradicting-terminal-state regression is pinned on the
 * real logic, not a copy.
 * Input: `status` = { text, state } as read from #reconcile-status
 * (dataset.state: undefined before the first cycle, 'loading' while a read
 * is in flight, 'success'/'error' once settled); `awaitedText` = the awaited
 * terminal text.
 * Output: 'pending' for a not-yet-started status (no state yet) or an
 * in-flight 'loading' read — neither is ever a result; 'match' when the
 * settled status renders the awaited text; 'contradiction' when a settled
 * terminal state (success or error) renders anything else.
 * Errors: never throws — the caller turns 'contradiction' into the
 * expected-vs-observed failure with the cause kept. No side effect. */
function classifyStatus(status, awaitedText) {
  if (!status.state || status.state === 'loading') return 'pending';
  return status.text.indexOf(awaitedText) >= 0 ? 'match' : 'contradiction';
}

// ---- regression guard: DoubleServer close/wait contract + fresh-evidence
// allocation + wait classification (pure class/fs, no browser). Runs ONLY under
// PAEA_RECONCILE_DOUBLE_GUARD=1: in that mode the browser run and the fixture
// output directory below are skipped entirely and the process exits from the
// guard alone, so the component suite stays untouched. Guards the two
// received FAIL cases: (1) close() must DESTROY the held HTTP response before
// awaiting server close (a real http.Server.close() otherwise waits forever
// on the unfinished connection); (2) waitForRequests after close must reject
// with the retained cause, never enqueue. Also guards: close() rejects held
// `done` with the cause, close is idempotent, release is idempotent, fresh
// per-run directories stay distinct under one parent with the first sentinel
// preserved, /tmp parents refused, the shared status classifier treats
// loading/pending as never-a-result and settles as match/contradiction (the
// received phaseC stall) — and no unhandledRejection. ----
async function doubleCloseGuard() {
  const assert = require('assert');
  const fakeRes = () => {
    const r = { destroyed: false, destroyArg: undefined, writeHead() { this.sent = true; }, end() { this.ended = true; } };
    r.destroy = err => { r.destroyed = true; r.destroyArg = err; };
    return r;
  };
  const unhandled = [];
  const onUnhandled = e => unhandled.push(e);
  process.on('unhandledRejection', onUnhandled);
  try {
    // Case 1 — received FAIL: close() must destroy the held response.
    {
      const d = new DoubleServer();
      d.server = { close(cb) { cb(); } }; // real server double: close resolves only if nothing is held
      d.setControl('reconcile', { hold: true, status: 200, body: { fixture: true } });
      const r = fakeRes();
      d.api('reconcile', { method: 'GET', headers: {} }, r, new URL('http://double.invalid/api/v1/paea/reconcile-effects'));
      await d.waitForRequests('reconcile', 1, 0);
      const rejected = assert.rejects(d.requests[0].done, /double server closed/);
      await d.close();
      await rejected;
      assert.equal(r.destroyed, true, 'close must destroy the held HTTP response before awaiting server close');
      assert.ok(r.destroyArg instanceof Error, 'destroy carries the close cause');
      console.log('PASS guard: close destroys the held HTTP response and rejects its done');
    }
    // Case 2 — received FAIL: waitForRequests after close rejects with the
    // retained cause (even when the count is already reached).
    {
      const d = new DoubleServer();
      d.server = { close(cb) { cb(); } };
      await d.close();
      const late = d.waitForRequests('reconcile', 1, 0);
      await assert.rejects(late, /double server closed/, 'wait after close must reject, never enqueue');
      await d.close(); // idempotent: second close resolves without effect
      console.log('PASS guard: waitForRequests after close rejects with the retained cause; close is idempotent');
    }
    // Companion — release stays idempotent and does not double-settle done.
    {
      const d = new DoubleServer();
      d.setControl('reconcile', { hold: true, status: 200, body: { fixture: true } });
      const r = fakeRes();
      d.api('reconcile', { method: 'GET', headers: {} }, r, new URL('http://double.invalid/api/v1/paea/reconcile-effects'));
      const e = d.requests[0];
      d.release(e);
      await e.done;
      d.release(e); // second release: no-op, done already settled
      assert.equal(r.ended, true, 'released response was sent exactly once');
      console.log('PASS guard: release is idempotent and done settles once');
    }
    // Companion — fresh-evidence allocation through the REAL shared allocator
    // (the same allocateRunDirectory() the browser run consumes): two
    // allocations under the SAME parent yield two DISTINCT run directories
    // with distinct run-manifest identities, the first run's sentinel artifact
    // stays untouched by the second (no overwrite, no deletion), directories
    // carry 0700 rights, and a parent literally under /tmp is refused as a
    // NEGATIVE INPUT before any write (never an attempted /tmp allocation;
    // os.tmpdir() is deliberately not used — the runner pins TMPDIR outside
    // /tmp). No browser session involved.
    {
      const parent = fixtureArea.fixtureRoot('guard-fresh-parent-');
      const first = allocateRunDirectory(parent);
      fs.writeFileSync(path.join(first.dir, 'evidence-final.json'), 'first-run-sentinel', { mode: 0o600 });
      const second = allocateRunDirectory(parent);
      assert.notEqual(first.dir, second.dir, 'two runs under the same parent must allocate distinct run directories');
      assert.notEqual(first.manifest.started_monotonic_ns, second.manifest.started_monotonic_ns, 'run manifests must carry distinct allocation instants');
      assert.equal(JSON.parse(fs.readFileSync(path.join(second.dir, 'run-manifest.json'), 'utf8')).run_dir, second.dir, 'second run manifest records its own path');
      assert.equal(fs.statSync(first.dir).mode & 0o777, 0o700, 'allocated run directory must carry private 0700 rights');
      assert.equal(fs.readFileSync(path.join(first.dir, 'evidence-final.json'), 'utf8'), 'first-run-sentinel', 'first run evidence must survive the second allocation untouched');
      // Negative input: the /tmp interdiction keeps holding for a
      // caller-provided parent; the allocator refuses BEFORE any write.
      assert.throws(() => allocateRunDirectory('/tmp/interdiction-witness'), /tmp interdiction/, 'a parent under /tmp must be refused with its cause');
      console.log('PASS guard: real allocator reused, fresh per-run directories under one parent, 0700 rights, first sentinel preserved, /tmp refused');
    }
    // Companion — wait classification through the REAL shared classifier (the
    // same classifyStatus() the browser waitText consumes): 'loading' and a
    // not-yet-started status are never results, a settled match resolves, and
    // a settled CONTRADICTING terminal state is detected instead of being
    // awaited forever (the received phaseC stall: refusal rendered while the
    // success text was awaited).
    {
      assert.equal(classifyStatus({ text: 'whatever', state: undefined }, 'x'), 'pending', 'a status before the first cycle is pending, never a result');
      assert.equal(classifyStatus({ text: 'Chargement du rapport de rapprochement…', state: 'loading' }, 'x'), 'pending', 'an in-flight read is pending, never a result');
      assert.equal(classifyStatus({ text: 'Rapprochement recalculé affiché pour les deux domaines réels. …', state: 'success' }, 'Rapprochement recalculé affiché pour les deux domaines réels.'), 'match', 'a settled awaited text is a match');
      assert.equal(classifyStatus({ text: 'Lecture du rapport refusée : paea-reader-authentication-required.', state: 'error' }, 'Rapprochement recalculé affiché pour les deux domaines réels.'), 'contradiction', 'a settled refusal while the success text is awaited is a contradiction, not an endless wait');
      assert.equal(classifyStatus({ text: 'Rapprochement recalculé affiché pour les deux domaines réels. …', state: 'success' }, 'Lecture du rapport refusée : paea-reader-policy-stale.'), 'contradiction', 'a settled success while a refusal text is awaited is a contradiction too');
      console.log('PASS guard: real status classifier reused, terminal contradicting states detected, loading/pending never treated as results');
    }
    if (unhandled.length > 0) throw Error('unhandledRejection in guard: ' + unhandled.map(String).join(' | '));
  } finally {
    process.off('unhandledRejection', onUnhandled);
  }
}
if (process.env.PAEA_RECONCILE_DOUBLE_GUARD === '1') {
  doubleCloseGuard().then(() => console.log('DOUBLE_CLOSE_GUARD_VERIFIED'), err => { console.error('DOUBLE_CLOSE_GUARD_FAILED ' + (err && err.message || err)); if (err && err.stack) console.error(err.stack); process.exitCode = 1; });
} else {

// ---- named assertion journal (any failure => nonzero exit) ----
const journal = [];
function ev(name, ok, detail) {
  journal.push({ name, ok: ok === true, detail: detail === undefined ? '' : String(detail) });
  if (ok === true) console.log('PASS ' + name);
  else { console.error('FAIL ' + name + (detail === undefined ? '' : ' :: ' + String(detail))); process.exitCode = 1; }
}
function total() { return journal.length; }
function failures() { return journal.filter(e => !e.ok).length; }

/** Browser-run evidence: the optional argv[2] is the PARENT under which the
 * fresh run directory is allocated by the shared allocator
 * allocateRunDirectory() above; without an argument the parent is a fixture
 * root from ./fixture_area.cjs (never /tmp). Allocation happens before any
 * browser step, so an fs error ends the run without a PASS and without
 * touching the browser. */
const runAlloc = allocateRunDirectory(process.argv[2] || fixtureArea.fixtureRoot('swarm-reconcile-reader-ui-'));
const outDir = runAlloc.dir;

function evsnap(label) {
  fs.writeFileSync(path.join(outDir, 'evidence-' + label + '.json'),
    JSON.stringify({ label, journal, requests: dbl.requests }, null, 2));
}

const dbl = new DoubleServer();
const pageErrors = [];
const requestFailedUrls = [];

/** Wait for the reconcile status to show `text` as a TERMINAL state.
 * Event-driven, no time bound, no timer and no polling loop: the waiter
 * observes the page's own reconcile cycle (loading → next settled status)
 * through the REAL shared classifier `classifyStatus()`, serialized into the
 * page context by source (no logic copy in the browser predicate).
 *
 * A 'loading' status is never a result: the wait resolves when the in-flight
 * cycle's settled status renders with the awaited text. Event sequence: every
 * reconcile command sets the explicit 'loading' state synchronously at
 * command time — before `page.click()`/the navigation resolves (the same
 * input-acknowledgement guarantee phase F asserts on its captured loading
 * state) — so the first poll already observes the NEW cycle in flight and a
 * settled state classified here always belongs to the awaited cycle, never
 * to the previous phase's terminal status. A settled terminal
 * state CONTRADICTING the awaited text (error while the success text is
 * awaited, or the reverse) ends the wait immediately with the diagnosis
 * kept: expected vs observed, state and cause. A page close or navigation
 * during the wait surfaces its own cause instead of waiting for an
 * impossible text; a fetch/parse/response error on the page settles the
 * status into a visible state the classifier reads like any other. Errors:
 * if the awaited settled state never renders, the thrown error keeps desc,
 * expected text, observed text and state — never a timeout artifact. No
 * arbitrary timeout, timer, sleep or retry cap is added by this helper. */
async function waitText(page, text, desc) {
  const observe = () => page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
  try {
    // The page's own cycle settles each read through 'loading' — set
    // synchronously at command time, so the wait starts on the NEW cycle in
    // flight and never classifies the previous phase's inherited terminal
    // status as this read's contradiction. Awaiting a settled status that
    // renders `text` is thus a transition wait, not a same-frame assume. The
    // REAL shared classifier is serialized into the
    // page (its source, no logic copy) and decides match/contradiction there;
    // a contradicting settled state ends the wait on the very same poll.
    await page.waitForFunction((awaited, classifierSrc) => {
      const classify = new Function('return (' + classifierSrc + ')')();
      const el = document.getElementById('reconcile-status');
      return classify({ text: el.textContent, state: el.dataset.state }, awaited) !== 'pending';
    }, { timeout: 0, polling: 'raf' }, text, classifyStatus.toString());
    const settled = await observe();
    if (classifyStatus(settled, text) === 'match') return;
    throw Error('status wait ended on a contradicting terminal state: ' + desc
      + ' :: expected ' + JSON.stringify(text) + ' :: observed ' + JSON.stringify(settled)
      + ' (double control/sequence mismatch or product contract deviation)');
  } catch (err) {
    // page closed or navigation destroyed the execution context: its own
    // cause ends the wait, never an impossible-text indefinite wait
    if (err && (err.message || '').includes('Execution context was destroyed')) {
      throw Error('page closed or navigated during status wait: ' + desc);
    }
    if (err && (err.message || '').startsWith('status wait ended')) throw err;
    const seen = await observe().catch(e => ({ text: '<unavailable: ' + (e && e.message || e) + '>', state: '<unavailable>' }));
    throw Error('status text never observed: ' + desc
      + ' :: expected ' + JSON.stringify(text) + ' :: observed ' + JSON.stringify(seen));
  }
}
async function waitSettled(page) {
  await page.evaluate(() => new Promise(ok => { if (document.readyState === 'complete') ok(); else window.addEventListener('load', ok, { once: true }); }));
  await page.evaluate(() => new Promise(ok => requestAnimationFrame(() => requestAnimationFrame(ok))));
}
async function assertNoPageError(label) {
  ev('no real page error in ' + label, pageErrors.length === 0, pageErrors.join(' | '));
}
/** Credential absence across DOM snapshot + both web storages. */
async function credNotStored(page, label, ...secrets) {
  const snap = await page.evaluate(() => ({ html: document.documentElement.innerHTML, ls: JSON.stringify(localStorage), ss: JSON.stringify(sessionStorage) }));
  const leak = secrets.filter(s => snap.html.includes(s) || snap.ls.includes(s) || snap.ss.includes(s));
  ev('credentials absent from DOM and web storage after ' + label, leak.length === 0, JSON.stringify({ leaked: leak, snap: JSON.stringify(snap).slice(0, 400) }));
}
async function themeRendered(page, theme, label) {
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  ev('theme ' + theme + ' renders a real background (' + label + ')', /^rgba?\(/.test(bg) && !bg.startsWith('rgba(0, 0, 0, 0)'), bg);
}

(async () => {
  let browser = null;
  try {
    await dbl.listen();
    const base = 'http://127.0.0.1:' + dbl.port;
    // Test-fixture Chrome profile (labeled doubles, synthetic credentials):
    // allocated INSIDE this run's fresh directory — never a real user profile,
    // never /tmp, never shared with another run.
    const profile = path.join(outDir, 'browser-profile');
    browser = await puppeteer.launch({
      headless: true,
      executablePath: process.env.CHROME_BIN || '/opt/google/chrome/chrome',
      timeout: 0,
      protocolTimeout: 0,
      args: ['--no-sandbox'],
      userDataDir: profile
    });
    const page = await browser.newPage();
    // Puppeteer default navigation/action timeouts are explicitly disabled
    // through the public API: the test's end conditions are its assertions
    // and settled promises, never an inherited time budget.
    page.setDefaultNavigationTimeout(0);
    page.setDefaultTimeout(0);
    page.on('pageerror', err => pageErrors.push(String(err && err.message || err)));
    page.on('requestfailed', r => requestFailedUrls.push(r.url()));

    const reconcileCount = () => dbl.requests.filter(r => r.kind === 'reconcile').length;
    const reconcileRequests = () => dbl.requests.filter(r => r.kind === 'reconcile');

    // ---------- PHASE A — initial load (FR, état theme), no credential:
    // explicit typed refusal, report DOM purged, reader field/command present,
    // theme toggle keeps language and rewrites nav links ----------
    await page.setViewport({ width: 1280, height: 900 });
    await page.goto(base + '/paea-effects.html?lang=fr&theme=etat', { waitUntil: 'domcontentloaded', timeout: 0 });
    await page.waitForSelector('#reconcile-reload', { visible: true, timeout: 0 });
    await waitSettled(page);
    await waitText(page, 'Lecture du rapport refusée : paea-reader-authentication-required.', 'initial bare refusal shows the typed code');
    const stA = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    ev('initial no-credential read shows the explicit typed refusal (FR)', stA.text.includes('Lecture du rapport refusée : paea-reader-authentication-required.'), stA.text);
    ev('initial refusal is an error state, never a success', stA.state === 'error', stA.state);
    const domA = await page.evaluate(() => ({
      reportHidden: document.getElementById('reconcile-report').hidden,
      raw: document.getElementById('reconcile-report-raw').textContent,
      settlementState: document.getElementById('settlement-state').textContent,
      gitState: document.getElementById('git-state').textContent,
      settlementFindings: document.getElementById('settlement-findings').children.length,
      gitFindings: document.getElementById('git-findings').children.length
    }));
    ev('initial refusal purges all report DOM (container, technical pre, domains, findings)',
      domA.reportHidden === true && domA.raw === '' && domA.settlementState === '' && domA.gitState === '' && domA.settlementFindings === 0 && domA.gitFindings === 0, JSON.stringify(domA));
    const aRe = reconcileRequests();
    ev('initial load issued exactly one reconcile GET', aRe.length === 1, 'count=' + aRe.length);
    ev('initial reconcile request carries no Authorization header', aRe[0].authorization === null, String(aRe[0].authorization));
    ev('initial reconcile request is a GET on the reconcile path', aRe[0].method === 'GET' && aRe[0].url === '/api/v1/paea/reconcile-effects', aRe[0].method + ' ' + aRe[0].url);
    ev('credential input is empty on arrival', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'initial load', CREDENTIAL, CREDENTIAL2, CREDENTIAL3);
    ev('reader field label present (FR)', await page.$eval('label[for="reconcile-reader-credential"]', e => e.textContent.trim()) === 'Jeton lecteur (lecture du rapport de rapprochement classifié)');
    ev('re-read command label present (FR)', await page.$eval('#reconcile-reload', e => e.textContent.trim()) === 'Relire le rapport');
    const shapeA = await page.$eval('#reconcile-reader-credential', e => ({ type: e.type, autocomplete: e.getAttribute('autocomplete'), maxlength: e.getAttribute('maxlength') }));
    ev('reader input is an ephemeral password field without maxlength cutoff',
      shapeA.type === 'password' && shapeA.autocomplete === 'off' && shapeA.maxlength === null, JSON.stringify(shapeA));
    ev('re-read command is a plain button (no form submit, no POST)', await page.$eval('#reconcile-reload', e => e.type) === 'button');
    ev('preview refusal also stays visible (no cross-domain fake success)',
      await page.$eval('#preview-status', e => e.dataset.state) !== 'success');
    // theme toggle: theme changes, language unchanged, nav links rewritten
    await page.click('#paea-theme');
    ev('theme toggle applies sombre', await page.evaluate(() => document.documentElement.dataset.theme) === 'sombre');
    const navA = await page.$eval('nav a[data-paea-link]', a => a.href);
    ev('theme toggle keeps FR language and rewrites nav links (lang=fr&theme=sombre)', navA.includes('lang=fr') && navA.includes('theme=sombre'), navA);
    await page.click('#paea-theme');
    ev('theme toggle returns to état', await page.evaluate(() => document.documentElement.dataset.theme) === 'etat');
    await themeRendered(page, 'etat', 'phase A');
    assertNoPageError('phase A');
    evsnap('phase-a-initial-refusal');

    // ---------- PHASE B — fresh EN + sombre navigation: initial refusal stays
    // typed in EN, labels translated, purge again, scope again ----------
    await page.goto(base + '/paea-effects.html?lang=en&theme=sombre', { waitUntil: 'domcontentloaded', timeout: 0 });
    await page.waitForSelector('#reconcile-reload', { visible: true, timeout: 0 });
    await waitSettled(page);
    await waitText(page, 'Report read refused: paea-reader-authentication-required.', 'EN initial bare refusal');
    const stB = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    ev('EN initial no-credential read shows the dedicated typed refusal (EN)', stB.text.includes('Report read refused: paea-reader-authentication-required.') && stB.state === 'error', JSON.stringify(stB));
    ev('sombre theme applied from the URL', await page.evaluate(() => document.documentElement.dataset.theme) === 'sombre');
    await themeRendered(page, 'sombre', 'phase B');
    const navB = await page.$eval('nav a[data-paea-link]', a => a.href);
    ev('EN navigation links carry lang=en&theme=sombre', navB.includes('lang=en') && navB.includes('theme=sombre'), navB);
    ev('reader field label translated (EN)', await page.$eval('label[for="reconcile-reader-credential"]', e => e.textContent.trim()) === 'Reader token (read of the classified reconciliation report)');
    ev('re-read command label translated (EN)', await page.$eval('#reconcile-reload', e => e.textContent.trim()) === 'Re-read the report');
    const bBase = reconcileCount();
    await page.click('#reconcile-reload');
    await waitText(page, 'Report read refused: paea-reader-authentication-required.', 'EN re-read refusal');
    ev('EN re-read without credential issues exactly one more reconcile GET', reconcileCount() === bBase + 1, bBase + ' -> ' + reconcileCount());
    ev('credential input still empty after credential-less re-read', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    const domB = await page.evaluate(() => ({
      reportHidden: document.getElementById('reconcile-report').hidden,
      raw: document.getElementById('reconcile-report-raw').textContent
    }));
    ev('EN refusal purges the report DOM too', domB.reportHidden === true && domB.raw === '', JSON.stringify(domB));
    await credNotStored(page, 'EN credential-less re-read', CREDENTIAL, CREDENTIAL2, CREDENTIAL3);
    assertNoPageError('phase B');
    evsnap('phase-b-en-sombre');

    // ---------- PHASE C — fresh FR navigation, positive read with credential:
    // real non-empty report rendered, Bearer scoped, erasure ----------
    await page.goto(base + '/paea-effects.html?lang=fr&theme=etat', { waitUntil: 'domcontentloaded', timeout: 0 });
    await page.waitForSelector('#reconcile-reload', { visible: true, timeout: 0 });
    await waitSettled(page);
    await waitText(page, 'Lecture du rapport refusée : paea-reader-authentication-required.', 'FR fresh-load refusal before the positive read');
    // Positive control at the RIGHT moment: AFTER the initial refusal is
    // observed, BEFORE the credential re-read — the labeled double stays a
    // fixture, the product contract is not falsified.
    dbl.setControl('reconcile', { status: 200, body: SUCCESS_REPORT });
    await page.type('#reconcile-reader-credential', CREDENTIAL);
    await page.click('#reconcile-reload');
    await waitText(page, 'Rapprochement recalculé affiché pour les deux domaines réels.', 'positive read with Bearer');
    const stC = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    ev('positive read with credential shows the real report state (FR)', stC.text.includes('Rapprochement recalculé affiché pour les deux domaines réels.') && stC.state === 'success', JSON.stringify(stC));
    const reC = reconcileRequests()[reconcileCount() - 1];
    ev('reconcile re-read carried the Bearer credential exactly', reC.authorization === 'Bearer ' + CREDENTIAL, String(reC.authorization));
    ev('reconcile re-read is a GET with the credential never in the URL',
      reC.method === 'GET' && reC.url === '/api/v1/paea/reconcile-effects' && Object.keys(reC.query).length === 0, reC.method + ' ' + reC.url);
    ev('no other request carries any Authorization header',
      dbl.requests.every(r => r.kind === 'reconcile' || r.authorization === null),
      JSON.stringify(dbl.requests.filter(r => r.kind !== 'reconcile' && r.authorization !== null).map(r => r.kind)));
    const domC = await page.evaluate(() => ({
      reportHidden: document.getElementById('reconcile-report').hidden,
      raw: document.getElementById('reconcile-report-raw').textContent,
      settlementState: document.getElementById('settlement-state').textContent,
      gitState: document.getElementById('git-state').textContent,
      settlementFindings: document.getElementById('settlement-findings').children.length,
      gitFindings: document.getElementById('git-findings').children.length,
      findingsText: document.getElementById('settlement-findings').textContent,
      gitFindingsText: document.getElementById('git-findings').textContent
    }));
    ev('report container opens with the technical raw report', domC.reportHidden === false && domC.raw.includes('settlement') && domC.raw.includes('effect_count'), domC.raw.slice(0, 120));
    ev('settlement domain shows the real reconciled count (non-empty success)',
      domC.settlementState.includes('3 effet(s) durable(s) rapproché(s) : 2 correspondance(s) recalculée(s), 1 écart(s) signalé(s).'), domC.settlementState);
    ev('settlement findings render per intent with recomputed verdicts',
      domC.settlementFindings === 3 && domC.findingsText.includes('intent a-1 · effet aa11 · correspondance recalculée') && domC.findingsText.includes('intent a-3 · effet cc33 · écart'), domC.findingsText);
    ev('git domain shows the corrupt state with its exact codes as list items',
      domC.gitState.includes('Schéma altéré ou incomplet : aucune ligne rapprochée, aucune réparation tentée.') && domC.gitFindings === 2 && domC.gitFindingsText.includes('g-ref-1') && domC.gitFindingsText.includes('g-ref-2'), domC.gitState + ' || ' + domC.gitFindingsText);
    ev('credential erased from the field after success', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'successful read', CREDENTIAL, CREDENTIAL2, CREDENTIAL3);
    ev('credential never rendered in the technical report',
      !domC.raw.includes(CREDENTIAL), domC.raw.slice(0, 200));
    assertNoPageError('phase C');
    evsnap('phase-c-positive-read');

    // ---------- PHASE D — refusal WITH a credential: typed code, full purge of
    // the previous positive report, erasure, recovery available ----------
    dbl.setControl('reconcile', { status: 403, body: STALE_REFUSAL });
    await page.type('#reconcile-reader-credential', CREDENTIAL2);
    await page.click('#reconcile-reload');
    await waitText(page, 'Lecture du rapport refusée : paea-reader-policy-stale.', 'typed refusal with credential');
    const stD = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    ev('refusal with credential shows the exact typed policy code (FR, dedicated text)',
      stD.text.includes('Lecture du rapport refusée : paea-reader-policy-stale.') && stD.state === 'error', JSON.stringify(stD));
    const domD = await page.evaluate(() => ({
      reportHidden: document.getElementById('reconcile-report').hidden,
      raw: document.getElementById('reconcile-report-raw').textContent,
      settlementState: document.getElementById('settlement-state').textContent,
      gitState: document.getElementById('git-state').textContent,
      settlementFindings: document.getElementById('settlement-findings').children.length,
      gitFindings: document.getElementById('git-findings').children.length
    }));
    ev('refusal purges the previous positive report completely (container, pre, states, findings)',
      domD.reportHidden === true && domD.raw === '' && domD.settlementState === '' && domD.gitState === '' && domD.settlementFindings === 0 && domD.gitFindings === 0, JSON.stringify(domD));
    const reD = dbl.requests[dbl.requests.length - 1];
    ev('refused re-read carried the second credential', reD.authorization === 'Bearer ' + CREDENTIAL2, String(reD.authorization));
    ev('refusal body data never rendered (no ledger rows from a 403)', domD.raw === '' && !domD.settlementState.includes('9'), JSON.stringify(domD));
    ev('second credential erased from the field after refusal', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'refused read', CREDENTIAL, CREDENTIAL2);
    ev('re-read command still available after refusal (recovery path stays open)',
      await page.$eval('#reconcile-reload', e => !e.disabled));
    assertNoPageError('phase D');
    evsnap('phase-d-refused-with-credential');

    // ---------- PHASE E — positive recovery through the same public operation ----------
    dbl.resetControls();
    dbl.setControl('reconcile', { status: 200, body: SUCCESS_REPORT });
    await page.type('#reconcile-reader-credential', CREDENTIAL);
    await page.click('#reconcile-reload');
    await waitText(page, 'Rapprochement recalculé affiché pour les deux domaines réels.', 'positive recovery after refusal');
    ev('positive recovery after refusal through the same public operation',
      (await page.$eval('#reconcile-status', e => e.dataset.state)) === 'success');
    const reE = dbl.requests[dbl.requests.length - 1];
    ev('recovery read carried the credential again', reE.authorization === 'Bearer ' + CREDENTIAL, String(reE.authorization));
    ev('credential erased again after recovery', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'recovery read', CREDENTIAL, CREDENTIAL2);
    assertNoPageError('phase E');
    evsnap('phase-e-recovery');

    // ---------- PHASE F — concurrent stale response after a newer refusal
    // (double-click race): the older response is HELD by the double until the
    // runner releases it AFTER the newer refusal is rendered — the response
    // order is forced by the runner, not by timers; the stale response must
    // never re-render ----------
    dbl.resetControls();
    dbl.setControl('reconcile', { sequence: [{ status: 200, body: SUCCESS_REPORT, hold: true }, { status: 403, body: STALE_REFUSAL }] });
    await page.type('#reconcile-reader-credential', CREDENTIAL);
    const fBase = reconcileCount();
    await page.click('#reconcile-reload'); // click #1 -> held would-be success
    const fLoading = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    ev('loading state is explicit during the in-flight read', fLoading.state === 'loading' && fLoading.text.includes('Chargement du rapport de rapprochement…'), JSON.stringify(fLoading));
    await dbl.waitForRequests('reconcile', 1, fBase); // the REAL captured event of click #1
    const fSeq1 = dbl.requests[dbl.requests.length - 1];
    await page.click('#reconcile-reload'); // click #2 -> immediate refusal (newer)
    await dbl.waitForRequests('reconcile', 2, fBase);
    await waitText(page, 'Lecture du rapport refusée : paea-reader-policy-stale.', 'newer refusal wins the race');
    const fSeq2 = dbl.requests[dbl.requests.length - 1];
    ev('both race requests were issued in order (older first)', fSeq1.authorization === 'Bearer ' + CREDENTIAL && fSeq2.authorization === 'Bearer ' + CREDENTIAL, fSeq1.authorization + ' / ' + fSeq2.authorization);
    dbl.release(fSeq1); // only NOW may the stale would-be-success reach the page
    await fSeq1.done; // its terminal issue (sent, or rejected if the double closes first)
    await page.evaluate(() => new Promise(ok => requestAnimationFrame(ok)));
    const stF = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    const domF = await page.evaluate(() => ({
      reportHidden: document.getElementById('reconcile-report').hidden,
      raw: document.getElementById('reconcile-report-raw').textContent,
      settlementFindings: document.getElementById('settlement-findings').children.length
    }));
    ev('stale concurrent response never re-displays report data after the newer refusal',
      stF.text.includes('Lecture du rapport refusée : paea-reader-policy-stale.') && domF.reportHidden === true && domF.raw === '' && domF.settlementFindings === 0, JSON.stringify({ stF, domF }));
    ev('credential erased after the race', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'stale race', CREDENTIAL, CREDENTIAL2);
    assertNoPageError('phase F');
    evsnap('phase-f-stale-race');

    // ---------- PHASE F2 — stale response of the INITIAL LOAD: the initial
    // request's response is HELD until the runner releases it after the
    // re-read refusal is rendered; the runner awaits the REAL first request
    // event instead of assuming the initial load has reached the double. A
    // re-read during the load must keep the initial (older) response from
    // overwriting ----------
    dbl.resetControls();
    dbl.setControl('reconcile', { sequence: [{ status: 403, body: BARE_REFUSAL, hold: true }, { status: 403, body: STALE_REFUSAL }] });
    const f2Base = reconcileCount(); // baseline BEFORE the load: the awaited request is the initial one
    await page.goto(base + '/paea-effects.html?lang=fr&theme=etat', { waitUntil: 'domcontentloaded', timeout: 0 });
    await page.waitForSelector('#reconcile-reload', { visible: true, timeout: 0 });
    await dbl.waitForRequests('reconcile', 1, f2Base); // the REAL initial-load request, not an assumed arrival
    const staleInit = dbl.requests[dbl.requests.length - 1];
    const gLoading = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    ev('initial load shows the explicit loading state while the report is in flight',
      gLoading.state === 'loading' && gLoading.text.includes('Chargement du rapport de rapprochement…'), JSON.stringify(gLoading));
    await page.type('#reconcile-reader-credential', CREDENTIAL);
    await page.click('#reconcile-reload'); // newer explicit re-read during the load
    await dbl.waitForRequests('reconcile', 2, f2Base);
    await waitText(page, 'Lecture du rapport refusée : paea-reader-policy-stale.', 're-read refusal during initial load');
    const before = f2Base + 2; // exactly the initial load + the re-read, nothing more
    dbl.release(staleInit); // only NOW may the initial load's own (older) response reach the page
    await staleInit.done; // its terminal issue (sent, or rejected if the double closes first)
    await page.evaluate(() => new Promise(ok => requestAnimationFrame(ok)));
    const stF2 = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    const domF2 = await page.evaluate(() => ({
      reportHidden: document.getElementById('reconcile-report').hidden,
      raw: document.getElementById('reconcile-report-raw').textContent
    }));
    ev('late response of the initial load never overwrites the newer re-read refusal',
      stF2.text.includes('Lecture du rapport refusée : paea-reader-policy-stale.')
      && !stF2.text.includes('paea-reader-authentication-required')
      && domF2.reportHidden === true && domF2.raw === '', JSON.stringify({ stF2, domF2 }));
    ev('exactly two reconcile requests in the initial-load race (no re-issue storm)', reconcileCount() === before, before + ' -> ' + reconcileCount());
    ev('credential erased after the initial-load race', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'initial-load race', CREDENTIAL, CREDENTIAL2);
    assertNoPageError('phase F2');
    evsnap('phase-f2-initial-load-race');

    // ---------- PHASE G — network failure: explicit refusal state with the
    // real fetch error, credential erased, no report data ----------
    dbl.resetControls();
    dbl.setControl('reconcile', { abort: true });
    await page.type('#reconcile-reader-credential', CREDENTIAL);
    await page.click('#reconcile-reload');
    await waitText(page, 'Refus du rapport : ', 'network failure surfaces an explicit state');
    const stG = await page.$eval('#reconcile-status', e => ({ text: e.textContent, state: e.dataset.state }));
    ev('network failure keeps an explicit error state carrying the real fetch error (no fake success)',
      stG.state === 'error' && stG.text.includes('Refus du rapport : ') && stG.text.includes('Failed to fetch'), JSON.stringify(stG));
    ev('credential erased after network failure', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'network failure', CREDENTIAL, CREDENTIAL2);
    const domG = await page.evaluate(() => ({ reportHidden: document.getElementById('reconcile-report').hidden, raw: document.getElementById('reconcile-report-raw').textContent }));
    ev('network failure leaves no report data behind', domG.reportHidden === true && domG.raw === '', JSON.stringify(domG));
    assertNoPageError('phase G');
    evsnap('phase-g-network-failure');

    // ---------- PHASE H — non-JSON 200: explicit unreadable-response state ----------
    dbl.resetControls();
    dbl.setControl('reconcile', { status: 200, body: 'not-json{' });
    await page.type('#reconcile-reader-credential', CREDENTIAL);
    await page.click('#reconcile-reload');
    await waitText(page, 'Réponse illisible ; vérifier l’état avant reprise', 'parse failure surfaces an explicit state');
    ev('parse failure shows the explicit unreadable-response state (200 included)',
      (await page.$eval('#reconcile-status', e => e.dataset.state)) === 'error');
    ev('credential erased after parse failure', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'parse failure', CREDENTIAL, CREDENTIAL2);
    assertNoPageError('phase H');
    evsnap('phase-h-parse-failure');

    // ---------- PHASE I — endpoint scoping, mutant-request ban, keyboard, mobile ----------
    dbl.resetControls();
    dbl.setControl('reconcile', { status: 200, body: SUCCESS_REPORT });
    await page.type('#reconcile-reader-credential', CREDENTIAL);
    await page.click('#reconcile-reload');
    await waitText(page, 'Rapprochement recalculé affiché pour les deux domaines réels.', 'final positive read');
    ev('only reconcile requests ever carry a credential (scoped Bearer)',
      dbl.requests.every(r => r.kind === 'reconcile' || r.authorization === null),
      JSON.stringify(dbl.requests.filter(r => r.kind !== 'reconcile' && r.authorization !== null).map(r => r.kind)));
    ev('preview and git-call endpoints stay credential-less and GET',
      dbl.requests.filter(r => r.kind === 'preview').every(r => r.method === 'GET' && r.authorization === null)
      && dbl.requests.filter(r => r.kind === 'gitcall').every(r => r.method === 'GET' && r.authorization === null));
    ev('approval endpoints untouched by the report reader',
      dbl.requests.filter(r => r.kind === 'approval-status' || r.kind === 'approval-presentation').every(r => r.authorization === null));
    ev('no mutant request: only GETs were issued by the page',
      dbl.requests.every(r => r.method === 'GET'), JSON.stringify(dbl.requests.filter(r => r.method !== 'GET').map(r => r.method + ' ' + r.url)));
    ev('credential never appears in any request URL or query',
      dbl.requests.every(r => !r.url.includes(CREDENTIAL) && !r.url.includes(CREDENTIAL2) && !r.url.includes(CREDENTIAL3) && Object.keys(r.query).length === 0));
    ev('credential never reaches the page address bar URL', !page.url().includes(CREDENTIAL), page.url());
    // keyboard: Escape clears; Enter submits nothing; explicit click works
    await page.focus('#reconcile-reader-credential');
    await page.type('#reconcile-reader-credential', CREDENTIAL3);
    await page.keyboard.press('Escape');
    ev('Escape clears the credential field', await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await page.type('#reconcile-reader-credential', CREDENTIAL3);
    const kBef = reconcileCount();
    await page.keyboard.press('Enter'); // no form: must NOT submit anything
    ev('Enter in the field sends no request (explicit button only)', reconcileCount() === kBef, kBef + ' -> ' + reconcileCount());
    await page.click('#reconcile-reload');
    await waitText(page, 'Rapprochement recalculé affiché pour les deux domaines réels.', 'keyboard-driven positive read');
    const reK = dbl.requests[dbl.requests.length - 1];
    ev('keyboard-driven re-read carried the credential and erased it',
      reK.authorization === 'Bearer ' + CREDENTIAL3 && await page.$eval('#reconcile-reader-credential', e => e.value) === '');
    await credNotStored(page, 'keyboard cycle', CREDENTIAL, CREDENTIAL2, CREDENTIAL3);
    // mobile layout at 390px
    await page.setViewport({ width: 390, height: 844 });
    await waitSettled(page);
    const mob = await page.evaluate(() => ({
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      fieldVisible: (() => { const e = document.getElementById('reconcile-reader-credential'); const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; })(),
      buttonVisible: (() => { const e = document.getElementById('reconcile-reload'); const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; })(),
      fontSize: getComputedStyle(document.getElementById('reconcile-reader-credential')).fontSize,
      btnHeight: document.getElementById('reconcile-reload').getBoundingClientRect().height
    }));
    ev('no horizontal overflow at 390px with the reader controls', mob.overflow <= 0, JSON.stringify(mob));
    ev('reader field and command visible at 390px', mob.fieldVisible && mob.buttonVisible);
    ev('mobile input font at least 16px (no iOS zoom) per paea.css', parseFloat(mob.fontSize) >= 16, mob.fontSize);
    ev('re-read command keeps a 44px-class touch target at 390px', mob.btnHeight >= 44, String(mob.btnHeight));
    assertNoPageError('phase I');
    evsnap('phase-i-scoping-keyboard-mobile');

    // ---------- final: request-failure ledger is explained (abort phase only) ----------
    ev('request failures limited to the staged network-failure phase',
      requestFailedUrls.every(u => u.endsWith('/api/v1/paea/reconcile-effects')),
      JSON.stringify(requestFailedUrls));
    assertNoPageError('final sweep');

    // Fresh-evidence final check: this run's sentinel manifest still records
    // THIS run directory and the evidence files landed only there.
    {
      const manifest = JSON.parse(fs.readFileSync(path.join(outDir, 'run-manifest.json'), 'utf8'));
      ev('fresh per-run evidence directory retained (manifest matches this run)',
        manifest.run_dir === outDir && manifest.parent === runAlloc.parent && manifest.pid === process.pid,
        JSON.stringify({ run_dir: manifest.run_dir, parent: manifest.parent, pid: manifest.pid }));
    }

    evsnap('final');
    const ko = failures();
    console.log('journal assertions: ' + total() + ' failed: ' + ko);
    if (ko === 0) console.log('PAEA_RECONCILE_READER_UI_VERIFIED ' + total());
    else process.exitCode = 1;
  } catch (err) {
    console.error('FATAL ' + (err && err.message || err));
    if (err && err.stack) console.error(err.stack);
    process.exitCode = 1;
  } finally {
    try { if (browser) await browser.close(); } catch (_err) { /* closed */ }
    await dbl.close();
  }
})();
}
