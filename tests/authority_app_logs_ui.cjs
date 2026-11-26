'use strict';

/**
 * Exercise the Admin application log controls in Chromium with synthetic
 * authority responses. The live daemon and production account are untouched.
 * Usage: node tests/authority_app_logs_ui.cjs [FRESH_OUTPUT_DIRECTORY]
 * With no directory, allocate private evidence in the configured TMPDIR.
 * Chrome also uses TMPDIR. The test fails on the first unmet assertion.
 */
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const puppeteer = require('puppeteer');

const [requestedOutput] = process.argv.slice(2);
if (!process.env.TMPDIR) throw Error('TMPDIR must point to the configured fixture area');
process.env.TMPDIR = fs.realpathSync(process.env.TMPDIR);
const output = requestedOutput || fs.mkdtempSync(path.join(process.env.TMPDIR, 'admin-log-browser-'));
if (requestedOutput) fs.mkdirSync(output, {mode: 0o700});

(async () => {
  const browser = await puppeteer.launch({
    executablePath: process.env.CHROME_BIN || '/opt/google/chrome/chrome',
    args: ['--no-sandbox'],
  });
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.setContent('<html lang="fr" data-theme="etat"><body><main></main></body></html>');
    await page.evaluate(() => {
      const levels = ['critical', 'error', 'warn', 'info', 'debug'];
      const rows = [
        {at: 1, archiveAt: 5, code: 'archived', family: 'fixture', phase: 'first', level: 'warn'},
        {at: 2, code: 'active', family: 'fixture', phase: '<fixture>'.repeat(40), level: 'info'},
      ];
      const diagnostics = {max_bytes: 8192, archives: 3, page_rows: 10};
      window.logRequests = [];
      let revision = 'fixture-revision';
      let config = {log_level: 'info', argon2: {}};
      window.fetch = async (target, options) => {
        const url = new URL(target, 'https://swarm.invalid');
        const json = (data, status = 200) => new Response(JSON.stringify(data), {status});
        switch (url.pathname.split('/').pop()) {
          case 'native-capabilities': return json({access: {mode: 'authority'}});
          case 'me': return json({
            account: {subject: 'fixture', display_name: 'Fixture', roles: [], integrity: 2},
            administer: !window.auditOnly,
            permissions: window.auditOnly ? ['audit.read'] : ['audit.read', 'accounts.manage'],
          });
          case 'keys': return json({keys: [], now: 0});
          case 'accounts': return json({accounts: []});
          case 'groups': return json({groups: []});
          case 'sessions': return json({sessions: []});
          case 'policy': return json({error: 'fixture-unavailable'}, 404);
          case 'config':
            if (options.method === 'POST') {
              window.configRequest = JSON.parse(options.body);
              if (window.configRequest.revision !== revision) return json({error: 'authority-revision-stale'}, 409);
              config = window.configRequest.config;
              revision += '-saved';
              return json({applied: true, revision});
            }
            return json({config, revision, log_levels: levels});
          case 'log': {
            const q = url.searchParams.get('q') || '';
            const level = url.searchParams.get('minimum_level') || url.searchParams.get('level') || '';
            const snapshot = url.searchParams.get('snapshot');
            const limit = Number(url.searchParams.get('limit'));
            const offset = Number(url.searchParams.get('offset'));
            const order = url.searchParams.get('order');
            const position = url.searchParams.get('position');
            const source = url.searchParams.get('source');
            const windowBytes = url.searchParams.get('window_bytes');
            window.logRequests.push({q, level, snapshot, limit, offset, order, position, windowBytes, source});
            if (q === 'held') await new Promise(resolve => { window.releaseOlderRead = resolve; });
            if (window.refuseLog) return json({error: 'authority-diag-io'}, 500);
            if (snapshot && window.staleLog) return json({error: 'authority-log-snapshot-stale'}, 409);
            const matches = rows.filter(row => !window.noArchives || !row.archiveAt)
              .filter(row => !level || levels.indexOf(row.level) <= levels.indexOf(level))
              .filter(row => source==='active'?!row.archiveAt:source==='archives'?!!row.archiveAt:true)
              .filter(row => JSON.stringify(row).includes(q));
            if (order === 'newest') matches.reverse();
            const base = {level: config.log_level, diagnostics, archives: window.noArchives ? 0 : 1, log_levels: levels, history_version: 1, rotation_version: 1};
            const page = matches.slice(offset, offset + limit);
            return limit === 0 ? json({...base, lines: matches.map(row=>JSON.stringify(row))}) : json({
              ...base, lines: page.map(row=>JSON.stringify(row)),
              line_archive_at: page.map(row=>row.archiveAt||null),
              rotation: {active_bytes: 380, archived_bytes: 200, last_rotation_at: 5},
              offset, limit, total: matches.length, snapshot: 'test-snapshot',
              window_bytes: Number(windowBytes || 300),
            });
          }
          default: return json({chain_valid: true, sessions: []});
        }
      };
    });
    const repo = path.resolve(__dirname, '..');
    for (const style of ['web/wattson_themes.css', 'frontend/engine-settings/admin.css', 'web/swarm-design.css']) {
      await page.addStyleTag({path: path.join(repo, style)});
    }
    await page.addScriptTag({path: path.join(repo, 'web/i18n-en.js')});
    await page.addScriptTag({path: path.join(repo, 'web/native-authority-admin.js')});
    await page.evaluate(async () => {
      window.mountLogs = async () => {
        const main = document.querySelector('main');
        delete main.dataset.authorityMounted;
        await SwarmAuthorityAdmin.mount(main, null, {
          translate: text => document.documentElement.lang === 'en' ? SwarmEnglish[text] || text : text,
        });
      };
      await window.mountLogs();
      document.querySelector('[data-authority-log-jump]').focus();
    });

    const control = name => '[data-authority-log-' + name + ']';
    const values = () => page.$$eval(control('results') + ' tbody tr', rows => rows.map(row => row.cells[4].textContent));
    const search = control('search');
    const level = control('level');
    const lastRequest = () => page.evaluate(() => window.logRequests.at(-1));
    const criteria = async () => { const {q, level} = await lastRequest(); return {q, level}; };
    await page.keyboard.press('Enter');
    assert.ok(await page.$(control('refresh')), 'application log has a dedicated refresh command');
    await page.waitForFunction(() => document.querySelector('[data-authority-log-results] tbody'));
    assert.equal(await page.evaluate(() => document.activeElement.id), 'authority-application-log-title');
    assert.ok(await page.$eval(search, field => field.form.clientHeight >= field.offsetHeight &&
      getComputedStyle(field.form).overflowY === 'visible'), 'search controls are not clipped by the dialog');
    await page.$eval(control('options'), panel=>{panel.open=true});

    assert.deepEqual(await values(), ['active', 'archived']);
    const journalTab = control('journal-tab');
    const rotationTab = control('rotation-tab');
    assert.equal(await page.$eval(journalTab, tab => tab.getAttribute('aria-selected')), 'true');
    assert.equal(await page.$eval(control('rotation'), region => region.checkVisibility()), false);
    await page.focus(journalTab);
    await page.keyboard.press('ArrowRight');
    assert.equal(await page.$eval(rotationTab, tab => tab.getAttribute('aria-selected')), 'true');
    assert.equal(await page.$eval(control('rotation') + ' progress', bar => bar.value), 380);
    assert.match(await page.$eval(control('rotation'), region => region.textContent), /Dernière rotation/);
    assert.equal(await page.$eval(control('results'), region => region.checkVisibility()), false);
    await page.$eval('[data-authority-log-setting="max_bytes"]', input => { input.value = '4096'; });
    await page.keyboard.press('Home');
    assert.equal(await page.$eval(journalTab, tab => tab === document.activeElement), true);
    await page.keyboard.press('End');
    assert.equal(await page.$eval('[data-authority-log-setting="max_bytes"]', input => input.value), '4096');
    await page.$eval('[data-authority-log-setting="max_bytes"]', input => { input.value = '8192'; });
    await page.keyboard.press('ArrowLeft');
    await page.select(control('source'), 'archives');
    await page.$eval(search, field => field.form.requestSubmit());
    assert.equal((await lastRequest()).source, 'archives');
    assert.deepEqual(await values(), ['archived']);
    assert.match(await page.$eval(control('results') + ' tbody tr', row => row.cells[7].textContent), /Archive du/);
    assert.match(await page.$eval(control('filter-source'), button => button.textContent), /Archives conservées/);
    await page.evaluate(() => { window.noArchives = true; });
    await page.click(control('refresh'));
    assert.match(await page.$eval(control('results'), e => e.textContent), /Aucune archive conservée/);
    await page.evaluate(() => { window.noArchives = false; });
    await page.$eval(control('reset'), button => button.click());

    await page.select(control('order'), 'oldest');
    assert.deepEqual(await values(), ['archived', 'active']);

    await page.$eval(control('rows'), field => { field.value = '1'; field.form.requestSubmit(); });
    assert.deepEqual(await values(), ['archived']);
    await page.click(control('next-page'));
    assert.deepEqual(await values(), ['active']);
    assert.equal((await lastRequest()).snapshot, 'test-snapshot');
    await page.evaluate(() => { window.staleLog = true; });
    await page.click(control('prev-page'));
    assert.match(await page.$eval(control('status'), e => e.textContent), /authority-log-snapshot-stale/);
    await page.evaluate(() => { window.staleLog = false; });
    await page.type(search, 'draft');
    const expired = await lastRequest();
    await page.click(control('retry'));
    assert.deepEqual(await lastRequest(), {...expired, snapshot: null, offset: 0});
    assert.deepEqual(await values(), ['archived']);
    await page.$eval(search, field => { field.value = ''; });
    await page.select(control('position'), 'all');
    await page.$eval(control('rows'), field => { field.value = '10'; field.form.requestSubmit(); });
    assert.equal((await lastRequest()).windowBytes, null);
    assert.deepEqual(await values(), ['archived', 'active']);
    const wrap = control('wrap');
    assert.equal(await page.$eval(wrap, b => b.getAttribute('aria-pressed')), 'true');
    await page.click(wrap);
    assert.equal(await page.$eval(wrap, b => b.getAttribute('aria-pressed')), 'false');
    assert.equal(await page.$eval(control('results'), e => e.querySelectorAll('fixture').length), 0);
    await page.click(wrap);

    await page.type(search, 'archived');
    await page.select(level, 'warn');
    await page.focus(search);
    await page.keyboard.press('Enter');
    assert.deepEqual(await values(), ['archived']);
    assert.deepEqual(await page.$$eval(control('results') + ' mark', marks => marks.map(mark => mark.textContent)), ['archived']);
    await page.click(control('next-match'));
    assert.equal(await page.evaluate(() => document.activeElement.cells[4].textContent), 'archived');
    await page.evaluate(() => {
      window.exportedLogs = [];
      URL.createObjectURL = blob => {
        window.exportedLogs.push(blob.text()); return 'blob:fixture';
      };
      HTMLAnchorElement.prototype.click = () => {};
    });
    await page.click(control('text'));
    await page.click(control('html'));
    const [txt, html] = await page.evaluate(() => Promise.all(window.exportedLogs));
    assert.match(txt, /^Date\tNiveau/);
    assert.match(html, /^<!doctype html>/);
    assert.match(html, /archived/);
    assert.doesNotMatch(html, /<script/);
    assert.doesNotMatch(html, /<mark/);
    await page.type(search, '-draft');
    await page.$eval(control('refresh'), button => button.click());
    assert.deepEqual(await criteria(), {q: 'archived', level: 'warn'});

    await page.click(control('filter-position'));
    assert.equal((await lastRequest()).position, 'end');
    assert.deepEqual(await criteria(), {q: 'archived', level: 'warn'});
    assert.equal(await page.$eval(search, field => field.value), 'archived-draft');

    await page.evaluate(() => { window.refuseLog = true; });
    await page.$eval(control('refresh'), button => button.click());
    assert.match(await page.$eval(control('status'), e => e.textContent), /authority-diag-io/);
    assert.deepEqual(await values(), []);
    await page.evaluate(() => { window.refuseLog = false; });
    await page.$eval(control('refresh'), button => button.click());
    assert.deepEqual(await values(), ['archived']);

    await page.click(rotationTab);
    await page.evaluate(() => { window.refuseLog = true; });
    await page.click(control('save'));
    assert.equal(await page.$eval(control('status'), e => e.checkVisibility()), true);
    assert.match(await page.$eval(control('status'), e => e.textContent), /authority-diag-io/);
    await page.evaluate(() => { window.refuseLog = false; });
    assert.deepEqual(await criteria(), {q: 'archived', level: 'warn'});
    await page.click(control('save'));
    assert.equal(await page.evaluate(() => window.configRequest.revision), 'fixture-revision-saved');
    await page.click(journalTab);

    await page.$eval(search, field => { field.value = 'held'; field.form.requestSubmit(); });
    assert.equal(await page.$eval(control('results'), e => e.getAttribute('aria-busy')), 'true');
    await page.$eval(control('reset'), button => button.click());
    await page.evaluate(() => window.releaseOlderRead());
    assert.deepEqual(await criteria(), {q: '', level: ''});
    assert.deepEqual(await values(), ['active', 'archived']);
    await page.type(search, 'no-matches');
    await page.keyboard.press('Enter');
    assert.deepEqual(await values(), []);
    assert.match(await page.$eval(control('status'), e => e.textContent), /0/);
    assert.match(await page.$eval(control('results'), e => e.textContent), /Aucune ligne correspondante/);
    await page.click(control('filter-q'));
    assert.deepEqual(await criteria(), {q: '', level: ''});
    assert.equal(await page.$eval(search, field => field === document.activeElement), true);
    await page.$eval(search, field => { field.value = '<fixture>'; field.form.requestSubmit(); });
    assert.equal(await page.$$eval(control('results') + ' mark', marks => marks.length), 40);
    assert.equal(await page.$$eval(control('results') + ' fixture', elements => elements.length), 0);
    assert.deepEqual(errors, []);

    await page.keyboard.press('Escape');
    await page.waitForFunction(() => !document.querySelector('dialog[open]'));
    assert.equal(await page.evaluate(() => Object.hasOwn(document.activeElement.dataset, 'authorityLogJump')), true);
    await page.setViewport({width: 390, height: 844});
    for (const lang of ['fr', 'en']) {
      await page.evaluate(async lang => {
        document.documentElement.lang = lang;
        await window.mountLogs();
      }, lang);
      await page.click(control('jump'));
      await page.waitForFunction(() => document.querySelector('[data-authority-log-results] tbody'));
      assert.equal(await page.$eval(control('refresh'), b => b.textContent), lang === 'en' ? 'Refresh' : 'Actualiser');
      assert.equal(await page.$eval(rotationTab, b => b.textContent), lang === 'en' ? 'Rotation and retention' : 'Rotation et rétention');
      await page.$eval(search, field => { field.value = '<long-filter>'.repeat(12); field.form.requestSubmit(); });
      assert.match(await page.$eval(control('filter-q'), b => b.getAttribute('aria-label')),
        lang === 'en' ? /^Remove filter Search/ : /^Retirer le filtre Recherche/);
      assert.equal(await page.$eval(control('results'), e => e.textContent), lang === 'en' ? 'No matching lines.' : 'Aucune ligne correspondante.');
      for (const theme of ['etat', 'sombre']) {
        await page.evaluate(theme => { document.documentElement.dataset.theme = theme; }, theme);
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'no viewport overflow');
        assert.ok(await page.$eval(control('filter-q'), b => b.scrollWidth <= b.clientWidth), 'long filter text wraps inside its button');
        await page.$eval(control('filter-q'), b => b.click());
        assert.ok(await page.$eval(control('results'), display => {
          const region = display.querySelector('[role="region"]');
          const heading = region.querySelector('th');
          return region.scrollWidth > region.clientWidth && getComputedStyle(heading).whiteSpace === 'nowrap';
        }), 'readable columns scroll inside their own region');
        await page.screenshot({path: path.join(output, `logs-${lang}-${theme}-mobile.png`)});
        if (theme === 'etat') await page.$eval(search, field => { field.value = '<long-filter>'.repeat(12); field.form.requestSubmit(); });
      }
      await page.keyboard.press('Escape');
    }
    await page.evaluate(async () => {
      window.auditOnly = true;
      await window.mountLogs();
    });
    assert.equal(await page.$(control('jump')), null);
    assert.equal(await page.$('[data-audit-tab="log"]'), null);
    console.log('PASS Admin application logs: keyboard tabs, literal highlights, applied filter chips, empty archives, stale snapshot resume, language, themes, order, wrap, search, level, refresh, revision, safe refusal, recovery, race, rotation, source, pagination, window, matches, TXT/HTML, reset and visibility');
    console.log('SCREENSHOTS ' + output);
  } finally {
    await browser.close();
  }
})().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
