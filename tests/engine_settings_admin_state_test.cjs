/** Regression for terminal Admin waits, using actual DOM in an owned browser.
 * Each failure follows a valid control, without any temporal bound or live API.
 * Unknown phases refuse before page access; no provider/mission is touched. */
'use strict';
const assert=require('node:assert/strict'),puppeteer=require('puppeteer'),{waitForSettingsState}=require('./helpers/engine_settings_admin_state.cjs');
(async()=>{const browser=await puppeteer.launch({timeout:0,protocolTimeout:0,headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',args:['--no-sandbox']});try{const page=await browser.newPage();page.setDefaultTimeout(0);let cases=0;
 for(const phase of ['loaded','preview','saved']) {
  const form=phase==='preview'?'<dialog data-engine-settings open><p role="alert" hidden></p><section data-setting-preview></section></dialog>':'';
  await page.setContent('<main class="engine-settings-admin"><p role="status">Ready</p><button data-action="edit">Edit</button>'+form+'</main>');
  assert.equal((await waitForSettingsState(page,phase)).status,'ready');
  await page.$eval('[role="status"]',e=>{e.classList.add('alert');e.textContent='Original fixture refusal';});
  await assert.rejects(()=>waitForSettingsState(page,phase),e=>e.code==='admin-state-failure'&&e.phase===phase&&e.message==='Original fixture refusal');cases++;
  await page.$eval('[role="status"]',e=>{e.classList.remove('alert');e.textContent='Ready';});assert.equal((await waitForSettingsState(page,phase)).status,'ready');
 }
 await assert.rejects(()=>waitForSettingsState(null,'unknown'),/Unknown Admin state phase/);cases++;
 await page.setContent('<main class="engine-settings-admin"><p role="status">Ready</p><button data-action="edit">Edit</button><dialog data-engine-settings open><p role="alert">Visible modal refusal</p><section data-setting-preview hidden></section></dialog></main>');
 for(const phase of ['preview','saved']){await assert.rejects(()=>waitForSettingsState(page,phase),e=>e.message==='Visible modal refusal');cases++;}
 console.log('PASS '+cases+' terminal Admin wait regression cases');
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
