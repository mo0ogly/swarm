'use strict';
/** Confirm the served opinion and explicitly approve it through the native UI. */
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const puppeteer = require('puppeteer');
const {address, key, work} = JSON.parse(fs.readFileSync(0, 'utf8'));

(async () => {
  const browser = await puppeteer.launch({headless:true, executablePath:process.env.CHROME_BIN || '/opt/google/chrome/chrome',
    timeout:0, protocolTimeout:0, args:['--no-sandbox']});
  try {
    const cookieName = 'swarm_session_' + crypto.createHash('sha256').update(address).digest('hex').slice(0,12);
    const origin = 'http://' + address;
    const panel = '.native-workflow-architecture';
    for (const [lang, theme, width] of [['fr','etat',1440],['en','sombre',390]]) {
      const context = await browser.createBrowserContext();
      const page = await context.newPage();
      page.setDefaultTimeout(0);
      page.setDefaultNavigationTimeout(0);
      let posts = 0;
      page.on('request', request => {
        if (request.method() === 'POST' && new URL(request.url()).pathname.endsWith('/workflow-architecture')) posts++;
      });
      await context.setCookie({name:cookieName,value:key,url:origin,httpOnly:true,sameSite:'Strict'});
      await page.setViewport({width,height:900});
      assert.equal((await page.goto(origin+'/?'+new URLSearchParams({view:'conduite',read:'history',work,lang,theme}),
        {waitUntil:'networkidle0'})).status(),200);
      const expected = lang === 'fr' ? 'Workflow recommandé · Séquentiel' : 'Recommended workflow · Sequential';
      await page.waitForFunction(text => document.querySelector('#pilot-workflow-advice')?.textContent.startsWith(text), {}, expected);
      assert.equal(posts,0);
      await page.click('#conduite-show-team');
      await page.waitForFunction((selector, text) => document.querySelector(selector+' [role=status]')?.textContent===text,
        {}, panel, lang==='fr'?'Séquentiel':'Sequential');
      const visible = await page.$eval(panel, el=>el.textContent);
      assert(visible.includes(lang==='fr'?'Les dépendances imposent un ordre unique.':'Dependencies require a single execution order.'));
      assert.equal(posts,0);
      if (lang==='fr') {
        const response = page.waitForResponse(r=>r.request().method()==='POST'&&new URL(r.url()).pathname.endsWith('/workflow-architecture'));
        await page.click(panel+' button');
        assert.equal((await response).status(),200);
        assert.equal(posts,1);
      }
      await page.waitForFunction((selector, text) => document.querySelector(selector)?.textContent.includes(text),
        {}, panel, lang==='fr'?'Avis validé pour cette révision.':'Assessment approved for this revision.');
      console.log('PASS workflow architecture browser',lang,theme,width);
      await context.close();
    }
  } finally {
    await browser.close();
  }
})().catch(error => {console.error(error);process.exitCode=1;});
