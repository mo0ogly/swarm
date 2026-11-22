'use strict';
/** Verify stable status and replay a genuine captured conflict for UI coverage.
 * stdin contains only the owned session input. No payment or browser writes. */
const fs = require('node:fs');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const {join} = require('node:path');
const puppeteer = require('puppeteer');
const c = JSON.parse(fs.readFileSync(0, 'utf8'));

(async () => {
  const browser = await puppeteer.launch({
    headless: true, executablePath: process.env.CHROME_BIN || '/usr/bin/google-chrome',
    timeout: 0, protocolTimeout: 0, args: ['--no-sandbox', '--disable-background-networking'],
    userDataDir: join(c.root, 'chrome-' + c.phase),
  });
  try {
    await verify(browser, 'fr', 'etat', 1440);
    await verify(browser, 'en', 'sombre', 390);
  } finally {
    await browser.close();
  }
})().catch(e => {console.error(e.stack);process.exitCode = 1;});

/** Check the same protected cockpit and the operator's keyboard refresh. */
async function verify(browser, lang, theme, width) {
    const context = await browser.createBrowserContext();
    try {
    const page = await context.newPage();
    page.setDefaultTimeout(0);
    page.setDefaultNavigationTimeout(0);
    await page.setViewport({width, height: 1000});
    const origin = 'http://' + c.address;
    const error = [];
    const replayed = [];
    const expectedConsoleConflict = [];
    const expectedStatus = url => url.pathname === '/api/v1/mission' && url.searchParams.get('action') === 'status';
    page.on('pageerror', cause => error.push(cause.message));
    page.on('console', message => {
      if (message.type() !== 'error') return;
      if (c.conflict && expectedStatus(new URL(message.location().url || origin)) && message.text().includes('409')) {
        expectedConsoleConflict.push(message.text());
      } else {
        error.push(message.text());
      }
    });
    page.on('requestfailed', r => error.push(new URL(r.url()).pathname));
    page.on('request', r => {
      if (new URL(r.url()).origin !== origin || !['GET', 'HEAD'].includes(r.method())) error.push('unexpected request');
    });
    page.on('response', r => {
      if (r.status() >= 400 && !(c.conflict && expectedStatus(new URL(r.url())) && r.status() === 409)) {
        error.push(new URL(r.url()).pathname + ' ' + r.status());
      }
    });
    const name = 'swarm_session_' + crypto.createHash('sha256').update(c.address).digest('hex').slice(0, 12);
    await context.setCookie({name, value: c.key, url: origin, httpOnly: true, sameSite: 'Strict'});
    if (c.conflict) {
      assert.equal(c.conflict.failure.code, 'mission-status-observation-changed');
      await page.setRequestInterception(true);
      page.on('request', r => {
        if (expectedStatus(new URL(r.url())) && replayed.length === 0) {
          replayed.push(r.url());
          return r.respond({status: 409, contentType: 'application/json', body: JSON.stringify(c.conflict)});
        }
        return r.continue();
      });
    }
    const url = origin + '/?' + new URLSearchParams({lang, theme, work: c.work, view: 'conduite', read: 'current'});
    await page.goto(url, {waitUntil: 'networkidle0'});
    assert.equal(await page.$eval('#work', e => e.value), c.work);
    assert(await page.$eval('html', (el, args) => el.lang === args.lang && el.dataset.theme === args.theme, {lang, theme}));
    const notice = lang === 'fr' ? 'La mission a évolué pendant la lecture.' : 'The mission changed while its status was being read.';
    if (c.conflict) {
      const message = await page.$eval('#conduite-plan', e => e.innerText);
      assert(message.includes(notice));
      await page.$eval('#conduite-plan', e => e.scrollIntoView({block: 'center'}));
      await page.screenshot({path: join(c.out, 'conflit-' + lang + '.png')});
      const refresh = page.waitForResponse(r => expectedStatus(new URL(r.url())) && r.status() === 200);
      await page.focus('#refresh');
      await page.keyboard.press('Enter');
      await refresh;
      assert.equal(replayed.length, 1);
    }
    await page.waitForFunction(() => !document.querySelector('#main-content').hasAttribute('aria-busy') &&
      !!document.querySelector('#agents-list [data-agent-state="completed"]'));
    assert(!(await page.$eval('#conduite-plan', e => e.innerText)).includes(notice));
    assert.deepEqual(error, []);
    assert(expectedConsoleConflict.length <= (c.conflict ? 1 : 0));
    await page.screenshot({path: join(c.out, c.phase + '-' + lang + '.png')});
    console.log('PASS cockpit ' + c.phase + ' ' + lang);
    } finally {
      await context.close();
    }
}
