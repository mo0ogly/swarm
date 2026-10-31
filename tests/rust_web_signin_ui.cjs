#!/usr/bin/env node
'use strict';
/**
 * Browser verification of the cockpit sign-in ergonomics on a real native server.
 *
 * Usage: node tests/rust_web_signin_ui.cjs NATIVE_BINARY OUTPUT
 *
 * For each of FR/EN x etat/sombre and each cockpit page (/live.html,
 * /native-runtime.html): isolated Root under .swarm-fixtures, real
 * `swarm-rust web` listener, single-use ticket from `swarm-rust web-ticket`,
 * browser navigation to the ticket link (no key typed), visible translated
 * "Sign out" control reachable by keyboard, header screenshot, activation, landing
 * on /login with the owner cookie removed and the protected API refused; then
 * the same ticket link is replayed and must be refused (401 page).
 *
 * Effects: creates fixture Roots and servers it owns (stopped with SIGINT,
 * Roots kept on failure for inspection), screenshots, checks.json and the
 * Chrome profile in OUTPUT; nothing is written under /tmp.
 * Errors: any failed assertion, page error, console error or unexpected HTTP
 * status aborts with a non-zero exit after stopping owned servers.
 * Limits: no live Root, no provider call; proves browser behaviour of this
 * build only, not separation from same-UID agents.
 */
const fs = require('node:fs'), path = require('node:path'), net = require('node:net');
const assert = require('node:assert/strict'), {spawn, execFileSync} = require('node:child_process');
const puppeteer = require('puppeteer');
const {fixtureRoot} = require('./fixture_area.cjs');

if (process.argv.length !== 4) throw Error('Usage: rust_web_signin_ui.cjs NATIVE_BINARY OUTPUT');
const binary = path.resolve(process.argv[2]), output = path.resolve(process.argv[3]);
if (fs.existsSync(output)) throw Error('Fresh output required');
fs.mkdirSync(output, {recursive: true, mode: 0o700});
const checks = [], servers = [];
// Every child (Chrome, native servers, CLI) gets a private TMPDIR in the
// fixture area (never /tmp). Kept short: Chrome binds an AF_UNIX socket under
// it and the kernel limits socket paths to 108 bytes.
const scratch = fixtureRoot('ct-');
const env = {...process.env, TMPDIR: scratch};
const check = (name, value) => { assert(value, name); checks.push(name); console.log('PASS ' + name); };

/** Reserve a free loopback port (released before the server binds it). */
const freePort = () => new Promise((resolve, reject) => {
  const s = net.createServer(); s.once('error', reject);
  s.listen(0, '127.0.0.1', () => { const {port} = s.address(); s.close(() => resolve(port)); });
});

/** Start an owned native server on a fresh fixture Root; wait until /login answers. */
async function server(label) {
  const root = fixtureRoot('swarm-signin-ui-' + label + '-');
  execFileSync(binary, ['--root', root, 'init', '--json'], {stdio: 'ignore', env});
  const address = '127.0.0.1:' + await freePort();
  const child = spawn(binary, ['--root', root, 'web', address], {stdio: 'ignore', env});
  servers.push(child);
  for (;;) {
    if (child.exitCode !== null) throw Error('server exited early: ' + child.exitCode);
    try { const r = await fetch('http://' + address + '/login'); if (r.status === 200) break; } catch {}
    await new Promise(r => setTimeout(r, 100));
  }
  return {root, address};
}

/** Issue a ticket link through the public CLI; the token never reaches logs. */
function ticket(s, next) {
  const out = execFileSync(binary, ['--root', s.root, 'web-ticket', s.address, next, '--json'], {env});
  return JSON.parse(out).url;
}

(async () => {
  let browser, failed;
  try {
    browser = await puppeteer.launch({headless: true, executablePath: process.env.CHROME_BIN || '/opt/google/chrome/chrome',
      // Explicit profile under the output (never the default TMPDIR=/tmp).
      userDataDir: path.join(output, 'chrome-profile'), env,
      args: ['--no-sandbox'], timeout: 0, protocolTimeout: 0});
    for (const page of ['/live.html', '/native-runtime.html']) for (const lang of ['fr', 'en']) for (const theme of ['etat', 'sombre']) {
      const label = page.replace(/[/.]/g, '') + '-' + lang + '-' + theme;
      const s = await server(label), base = 'http://' + s.address;
      const tab = await browser.newPage(), errors = [];
      tab.setDefaultTimeout(0); tab.setDefaultNavigationTimeout(0);
      await tab.setViewport({width: 1280, height: 800});
      tab.on('pageerror', e => errors.push(e.message));
      tab.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      const link = ticket(s, page + '?lang=' + lang + '&theme=' + theme);
      await tab.goto(link, {waitUntil: 'load'});
      check(label + ':ticket-landed', new URL(tab.url()).pathname === page);
      const button = page === '/live.html' ? '#live-logout' : '[data-action="logout"]';
      await tab.waitForSelector(button + ':not([disabled])', {visible: true});
      // Deliberately no wait for cockpit startup: an early click must still sign
      // out (regression guard: the button once needed the startup CSRF token).
      const text = await tab.$eval(button, e => e.textContent.trim());
      check(label + ':translated', text === (lang === 'en' ? 'Sign out' : 'Se déconnecter'));
      check(label + ':theme', await tab.evaluate(() => document.documentElement.dataset.theme) === theme);
      await tab.focus(button);
      check(label + ':keyboard-focus', await tab.$eval(button, e => document.activeElement === e && getComputedStyle(e).visibility === 'visible'));
      const header = await tab.$(page === '/live.html' ? 'header.live-rail' : 'header');
      await header.screenshot({path: path.join(output, label + '.png')});
      await tab.setViewport({width: 390, height: 844});
      const box = await (await tab.$(button)).boundingBox();
      check(label + ':mobile-reachable', box && box.width > 0 && box.x >= 0 && box.x + box.width <= 390);
      await tab.screenshot({path: path.join(output, label + '-mobile.png')});
      await tab.setViewport({width: 1280, height: 800});
      await tab.focus(button);
      await Promise.all([tab.waitForNavigation({waitUntil: 'load'}), tab.keyboard.press('Enter')]);
      assert.equal(new URL(tab.url()).pathname, '/login', label + ':landed-on-login (actual ' + tab.url() + ')');
      check(label + ':landed-on-login', true);
      const cookies = await tab.cookies(base);
      check(label + ':cookie-removed', !cookies.some(c => c.name.startsWith('swarm_session')));
      const api = await tab.evaluate(async () => (await fetch('/api/v1/native-version', {credentials: 'same-origin'})).status);
      check(label + ':api-refused-after-logout', api === 403 || api === 401);
      const replay = await tab.goto(link, {waitUntil: 'load'});
      check(label + ':ticket-replay-refused', replay.status() === 401);
      check(label + ':no-page-errors', errors.filter(e => !/40[13]/.test(e)).length === 0);
      await tab.close();
    }
    fs.writeFileSync(path.join(output, 'checks.json'), JSON.stringify({status: 'PASS', checks}, null, 1), {mode: 0o600});
    console.log('SIGNIN_UI_VERIFIED ' + checks.length);
  } catch (error) { failed = error; } finally {
    if (browser) await browser.close();
    for (const child of servers) if (child.exitCode === null) { child.kill('SIGINT'); await new Promise(r => child.once('exit', r)); }
  }
  if (failed) { console.error(failed); process.exit(1); }
})();
