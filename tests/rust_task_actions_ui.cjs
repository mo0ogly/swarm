'use strict';
/** Verify real protected card, action and export on a submitted human-policy task. */
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const puppeteer = require('puppeteer');
const cfg = JSON.parse(fs.readFileSync(0, 'utf8'));
const card = '.graph-task-actions[data-task="task"]';
const journal = 'dialog:has(#native-task-actions-title)';
const detail = 'dialog:has(#native-task-action-title)';

(async () => {
  const browser = await puppeteer.launch({
    executablePath: process.env.CHROME_BIN || '/opt/google/chrome/chrome',
    timeout: 0, protocolTimeout: 0, args: ['--no-sandbox']
  });
  try {
    for (const [lang, theme, width] of [
      ['fr', 'etat', 1440], ['fr', 'sombre', 320], ['en', 'etat', 390], ['en', 'sombre', 1440]
    ]) {
      const context = await browser.createBrowserContext();
      const page = await context.newPage();
      page.setDefaultTimeout(0);
      page.setDefaultNavigationTimeout(0);
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      page.on('requestfailed', request => errors.push(request.failure().errorText));
      page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
      const origin = 'http://' + cfg.address;
      const name = 'swarm_session_' + crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0, 12);
      await context.setCookie({name, value:cfg.key, url:origin, httpOnly:true, sameSite:'Strict'});
      await page.setViewport({width,height:900});
      const params = new URLSearchParams({work:cfg.work, view:'conduite', read:'history', lang, theme});
      assert.equal((await page.goto(origin + '/?' + params, {waitUntil:'networkidle0'})).status(), 200);
      await page.waitForFunction(() => !document.querySelector('#main-content').hasAttribute('aria-busy'));
      await page.$eval(card, element => element.focus());
      await page.keyboard.press('Enter');
      await page.waitForSelector(journal + ' a[download]', {visible:true});
      const text = await page.$eval(journal, element => element.textContent);
      const expected = lang === 'fr'
        ? ['Fichier d’actions', 'Plan de la tâche', 'Échec observé', 'Ajustements', 'En attente de votre revue humaine.']
        : ['Action file', 'Task plan', 'Observed failure', 'Adjustments', 'Awaiting your human review.'];
      for (const item of expected) assert(text.includes(item), 'missing ' + item);
      assert(!text.includes('NEVER-EXPOSE'));
      assert(text.includes('Check result and adjust if needed'));
      if (lang === 'fr') assert(text.includes('Instruction de départ enregistrée'));
      else assert(text.includes('Recorded starting instruction'));
      const download = await page.$eval(journal + ' a[download]', async element =>
        ({name:element.download, body:await (await fetch(element.href)).json()}));
      assert.equal(download.name, 'task-actions.json');
      assert.equal(download.body.work, cfg.work);
      assert(download.body.review.awaiting_human);
      assert.equal(download.body.attempts[0].observation.actions[0].family, 'read');
      const bounds = await page.$eval(journal, element => {
        const rect = element.getBoundingClientRect();
        return rect.width <= innerWidth && rect.left >= 0 && rect.right <= innerWidth;
      });
      assert(bounds);
      await page.screenshot({path: cfg.output + '/task-actions-' + lang + '-' + theme + '.png'});
      const action = journal + ' li:first-child button';
      await page.focus(action);
      await page.keyboard.press('Enter');
      assert(await page.$(detail + '[open]'));
      assert((await page.$eval(detail, element => element.textContent)).includes(lang === 'fr' ? 'Succès observé' : 'Observed success'));
      await page.screenshot({path: cfg.output + '/action-detail-' + lang + '-' + theme + '.png'});
      await page.keyboard.press('Escape');
      await page.waitForFunction(selector => !document.querySelector('#native-task-action-title') &&
        document.activeElement.matches(selector), {}, action);
      await page.keyboard.press('Escape');
      await page.waitForFunction(selector => !document.querySelector('#native-task-actions-title') &&
        document.activeElement.matches(selector), {}, card);
      assert.deepEqual(errors, []);
      console.log('PASS protected card, JSON export, action detail and keyboard focus',lang,theme,width);
      await context.close();
    }
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
