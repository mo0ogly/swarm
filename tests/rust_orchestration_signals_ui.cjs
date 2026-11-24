'use strict';
/** Verify the protected status dialog using a real escalated native fixture. */
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const puppeteer = require('puppeteer');
const cfg = JSON.parse(fs.readFileSync(0, 'utf8'));
const trigger = '#conduite-status-signals';
const dialog = 'dialog:has(#native-orchestration-signals-title)';

(async () => {
  const browser = await puppeteer.launch({headless:true, executablePath:process.env.CHROME_BIN || '/opt/google/chrome/chrome',
    timeout:0, protocolTimeout:0, args:['--no-sandbox']});
  try {
    for (const [lang, theme, width] of [['fr','etat',1440],['fr','sombre',320],['en','etat',390],['en','sombre',1440]]) {
      const context = await browser.createBrowserContext();
      const page = await context.newPage();
      const errors = [];
      page.setDefaultTimeout(0);
      page.setDefaultNavigationTimeout(0);
      page.on('pageerror', error => errors.push(error.message));
      page.on('requestfailed', request => errors.push(request.failure().errorText));
      page.on('console', message => {if (message.type() === 'error') errors.push(message.text());});
      const origin = 'http://' + cfg.address;
      const name = 'swarm_session_' + crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0,12);
      await context.setCookie({name,value:cfg.key,url:origin,httpOnly:true,sameSite:'Strict'});
      await page.setViewport({width,height:900});
      const query = new URLSearchParams({view:'conduite',work:cfg.work,read:'history',lang,theme});
      assert.equal((await page.goto(origin+'/?'+query,{waitUntil:'networkidle0'})).status(),200);
      await page.click('#conduite-show-team');
      await page.focus(trigger);
      await page.keyboard.press('Enter');
      await page.waitForFunction(selector => {
        const panel = document.querySelector(selector);
        return panel?.querySelector('.orchestration-signal, [role=status].alert');
      },{},dialog);
      const text = await page.$eval(dialog, el=>el.textContent);
      assert(text.includes('beta → root'),text);
      for (const label of lang==='fr'
        ? ['Arbitrage demandé','Transmis','Reçu dans le contexte','Traité']
        : ['Arbitration requested','Sent','Received in context','Processed']) assert(text.includes(label),text);
      assert(text.includes(cfg.row.received.at) && text.includes(cfg.row.processed.at),text);
      await page.click(dialog+' details > summary');
      const proof = await page.$eval(dialog+' details[open]', el=>el.textContent);
      assert(proof.includes(cfg.row.id) && proof.includes(cfg.row.received.claim) &&
        proof.includes(cfg.row.processed.decision),proof);
      assert(!proof.includes('BETA_PRIVATE_REFUSAL_ORIGINAL'));
      assert(await page.$eval(dialog, el=>el.getBoundingClientRect().width<=innerWidth));
      await page.screenshot({path:cfg.output+'/signals-'+lang+'-'+theme+'.png'});
      await page.keyboard.press('Escape');
      await page.waitForFunction((panel, opener) =>
        !document.querySelector(panel) && document.activeElement.matches(opener),{},dialog,trigger);
      assert.deepEqual(errors,[]);
      console.log('PASS protected orchestration signals dialog',lang,theme,width);
      await context.close();
    }
  } finally {
    await browser.close();
  }
})().catch(error => {console.error(error);process.exitCode=1;});
