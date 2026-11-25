'use strict';
/** Verify served cockpit launch diagnostics and exact asset bytes on an owned
 * Rust server. Browser requests within this scenario are reads only. */
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const puppeteer = require('puppeteer');

const cfg = JSON.parse(fs.readFileSync(0, 'utf8'));
const address = 'http://' + cfg.address;
const cookie = 'swarm_session_' + crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0, 12);
const notice = '#native-launch-diagnostics';

/** Check the exact displayed reason, locale and theme at the selected width.
 * The protected browser session is scoped to this private fixture origin. */
async function verify(browser, lang, theme) {
  const context = await browser.createBrowserContext();
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(0);
    page.setDefaultNavigationTimeout(0);
    await page.setViewport({ width: lang === 'fr' ? 390 : 1440, height: 900 });
    const errors = [];
    const writes = [];
    const diagnosticRequests = [];
    const assetResponses = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('requestfailed', request => errors.push(new URL(request.url()).pathname));
    page.on('request', request => {
      if (!['GET', 'HEAD'].includes(request.method())) writes.push(new URL(request.url()).pathname);
      if (new URL(request.url()).searchParams.get('action') === 'launch-diagnostics') diagnosticRequests.push(request);
    });
    page.on('response', response => {
      const name = new URL(response.url()).pathname.split('/').pop();
      if (['cockpit-native.js', 'cockpit-native-read.js', 'i18n-en.js'].includes(name)) {
        assetResponses.push([name, response]);
      }
      if (response.status() >= 400) errors.push(new URL(response.url()).pathname + ' ' + response.status());
    });
    await context.setCookie({ name: cookie, value: cfg.key, url: address, httpOnly: true, sameSite: 'Strict' });
    const query = new URLSearchParams({ lang, theme, work: cfg.work });
    await page.goto(address + '/?' + query, { waitUntil: 'networkidle0' });
    await page.waitForFunction(work => document.querySelector('#work')?.value === work &&
      !document.querySelector('#main-content').hasAttribute('aria-busy') &&
      !!document.querySelector('#tasks-body tr'), {}, cfg.work);
    assert(await page.$eval('html', (html, locale, skin) =>
      html.lang === locale && html.dataset.theme === skin, lang, theme));
    if (cfg.phase === 'baseline') {
      assert.equal(await page.$(notice), null);
      assert.equal(diagnosticRequests.length, 0);
    } else {
      const contents = await page.$eval(notice, el => el.textContent);
      assert(contents.includes(lang === 'en' ? 'Automatic agent startup' : 'Démarrage automatique des agents'));
      if (cfg.phase === 'paused') {
        assert(contents.includes('planning-mission-paused'));
        assert(contents.includes(lang === 'en' ? 'The mission is paused.' : 'La mission est en pause.'));
        assert.equal(diagnosticRequests.length, 0);
      } else {
        assert.equal(diagnosticRequests.length, 1);
        assert(contents.includes('current-independent-review-required'));
        const reasons = await page.$$eval(notice + ' li', rows => rows.map(row => row.textContent));
        assert(reasons.some(reason => reason.includes('grpc') && reason.includes('current-independent-review-required')));
        if (cfg.phase === 'blocked') {
          const explanation = lang === 'en' ? 'The provider file is not executable.' :
            'Le fichier du fournisseur n’est pas exécutable.';
          assert(reasons.some(reason => reason.includes('Run') && reason.includes(explanation) &&
            reason.includes('agent-provider-not-executable')));
          assert(reasons.some(reason => reason.includes('parallel') && reason.includes(explanation)));
        } else {
          assert(!contents.includes('agent-provider-not-executable'));
        }
      }
      assert.equal(assetResponses.length, 3);
      for (const [name, response] of assetResponses) {
        assert.deepEqual(Buffer.from(await response.content()), fs.readFileSync(path.join('web', name)), name);
      }
      assert(await page.$eval(notice, el => el.scrollWidth <= el.clientWidth));
    }
    assert.deepEqual(errors, []);
    assert.deepEqual(writes, []);
    console.log('PASS cockpit-' + cfg.phase + '-' + lang + '-' + theme);
  } finally {
    await context.close();
  }
}

(async () => {
  const browser = await puppeteer.launch({
    headless: true, executablePath: process.env.CHROME_BIN || '/opt/google/chrome/chrome',
    timeout: 0, protocolTimeout: 0, args: ['--no-sandbox'],
    userDataDir: path.join(cfg.root, 'chrome-launch-' + cfg.phase),
  });
  try {
    await verify(browser, 'fr', 'etat');
    if (cfg.phase !== 'baseline') await verify(browser, 'en', 'sombre');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error.stack); process.exitCode = 1; });
