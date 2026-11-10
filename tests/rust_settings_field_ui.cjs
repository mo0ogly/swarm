#!/usr/bin/env node
'use strict';
/**
 * Visual check of one engine setting in the real Admin of a real native server.
 *
 * Usage: node tests/rust_settings_field_ui.cjs NATIVE_BINARY FIELD OUTPUT
 *
 * For FR/EN x etat/sombre: fresh fixture Root (configured fixture area, never
 * /tmp), real `swarm-rust web` listener, single-use ticket sign-in to
 * /native-runtime.html, "Régler le moteur" opened, the group holding
 * [data-setting=FIELD] unfolded, field screenshot, label translated through
 * the catalog/English locale, saved value shown in the input. Also checks the
 * field is reachable by keyboard focus and stays within a 390 px viewport.
 *
 * Effects: fixture Roots, owned servers (stopped with SIGINT), screenshots,
 * Chrome profile, private TMPDIR and checks.json under OUTPUT / fixture area.
 * Errors: assertion, page error or console error exits non-zero.
 * Limits: renders and reads only; no preview/apply is submitted.
 */
const fs = require('node:fs'), path = require('node:path'), net = require('node:net');
const assert = require('node:assert/strict'), {spawn, execFileSync} = require('node:child_process');
const puppeteer = require('puppeteer');
const {fixtureRoot} = require('./fixture_area.cjs');

if (process.argv.length !== 5) throw Error('Usage: rust_settings_field_ui.cjs NATIVE_BINARY FIELD OUTPUT');
const binary = path.resolve(process.argv[2]), field = process.argv[3], output = path.resolve(process.argv[4]);
if (!/^[a-z0-9_]+$/.test(field)) throw Error('Invalid field name');
if (fs.existsSync(output)) throw Error('Fresh output required');
fs.mkdirSync(output, {recursive: true, mode: 0o700});
const repo = path.resolve(__dirname, '..');
const catalog = JSON.parse(fs.readFileSync(path.join(repo, 'rust/crates/swarm-engine/config/engine-settings-catalog.json'), 'utf8'));
const english = JSON.parse(fs.readFileSync(path.join(repo, 'locales/en.json'), 'utf8'));
const entry = catalog.fields.find(f => f.name === field);
if (!entry) throw Error('Unknown catalog field ' + field);
const scratch = fixtureRoot('ft-');  // short: Chrome binds an AF_UNIX socket under TMPDIR
const env = {...process.env, TMPDIR: scratch};
const checks = [], servers = [];
const check = (name, value) => { assert(value, name); checks.push(name); console.log('PASS ' + name); };
const freePort = () => new Promise((resolve, reject) => {
  const s = net.createServer(); s.once('error', reject);
  s.listen(0, '127.0.0.1', () => { const {port} = s.address(); s.close(() => resolve(port)); });
});

(async () => {
  let browser, failed;
  try {
    browser = await puppeteer.launch({headless: true, executablePath: process.env.CHROME_BIN || '/opt/google/chrome/chrome',
      userDataDir: path.join(output, 'chrome-profile'), env, args: ['--no-sandbox'], timeout: 0, protocolTimeout: 0});
    for (const lang of ['fr', 'en']) for (const theme of ['etat', 'sombre']) {
      const label = lang + '-' + theme;
      const root = fixtureRoot('swarm-field-ui-' + label + '-');
      execFileSync(binary, ['--root', root, 'init', '--json'], {stdio: 'ignore', env});
      const saved = JSON.parse(execFileSync(binary, ['--root', root, 'engine-settings', 'show', '--json'], {env})).values[field];
      const address = '127.0.0.1:' + await freePort();
      const child = spawn(binary, ['--root', root, 'web', address], {stdio: 'ignore', env}); servers.push(child);
      for (;;) { if (child.exitCode !== null) throw Error('server exited'); try { if ((await fetch('http://' + address + '/login')).ok) break; } catch {} await new Promise(r => setTimeout(r, 100)); }
      const link = JSON.parse(execFileSync(binary, ['--root', root, 'web-ticket', address, '/native-runtime.html?view=admin&lang=' + lang + '&theme=' + theme, '--json'], {env})).url;
      const tab = await browser.newPage(), errors = [];
      tab.setDefaultTimeout(0); tab.setDefaultNavigationTimeout(0);
      tab.on('pageerror', e => errors.push(e.message));
      tab.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      await tab.setViewport({width: 1440, height: 960});
      await tab.goto(link, {waitUntil: 'load'});
      await tab.waitForSelector('#native-settings-admin [data-action="edit"]:not([disabled])', {visible: true});
      await tab.click('#native-settings-admin [data-action="edit"]');
      await tab.waitForSelector('dialog[open] [data-setting="' + field + '"]');
      await tab.$eval('[data-setting="' + field + '"]', e => { e.closest('details').open = true; e.scrollIntoView({block: 'center'}); });
      const box = await (await tab.$('[data-setting="' + field + '"]')).evaluateHandle(e => e.closest('.engine-settings-field'));
      await box.asElement().screenshot({path: path.join(output, label + '.png')});
      const text = await box.evaluate(e => e.innerText.replace(/\s+/g, ' '));
      const expected = lang === 'en' ? english[entry.label] : entry.label;
      check(label + ':label', typeof expected === 'string' && text.includes(expected));
      const value = await tab.$eval('[data-setting="' + field + '"]', e => e.value);
      check(label + ':saved-value-shown', value === saved);
      check(label + ':theme', await tab.evaluate(() => document.documentElement.dataset.theme) === theme);
      await tab.focus('[data-setting="' + field + '"]');
      check(label + ':keyboard-focus', await tab.$eval('[data-setting="' + field + '"]', e => document.activeElement === e));
      await tab.setViewport({width: 390, height: 844});
      await tab.$eval('[data-setting="' + field + '"]', e => e.scrollIntoView({block: 'center'}));
      const b = await (await tab.$('[data-setting="' + field + '"]')).boundingBox();
      check(label + ':mobile-within-viewport', b && b.x >= 0 && b.x + b.width <= 390);
      await box.asElement().screenshot({path: path.join(output, label + '-mobile.png')});
      check(label + ':no-page-errors', errors.length === 0);
      await tab.close();
    }
    fs.writeFileSync(path.join(output, 'checks.json'), JSON.stringify({status: 'PASS', field, checks}, null, 1), {mode: 0o600});
    console.log('SETTING_FIELD_UI_VERIFIED ' + field + ' ' + checks.length);
  } catch (error) { failed = error; } finally {
    if (browser) await browser.close();
    for (const c of servers) if (c.exitCode === null) { c.kill('SIGINT'); await new Promise(r => c.once('exit', r)); }
  }
  if (failed) { console.error(failed); process.exit(1); }
})();
