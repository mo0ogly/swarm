'use strict';
/** Read-only installed Admin check; never use the mutating fixture browser here.
 * Credentials arrive on stdin and stay out of artifacts. Opening and closing a
 * draft must preserve the entire saved journal. No POST/provider/watchdog.
 */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const {waitForSettingsState}=require('./helpers/engine_settings_admin_state.cjs');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],posts=[],errors=[];let browser;
(async()=>{try{
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const context=await browser.createBrowserContext(),page=await context.newPage(),split=cfg.cookie.indexOf('='),label=lang+'-'+theme;
  page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  page.on('request',r=>{if(r.method()==='POST')posts.push(new URL(r.url()).pathname)});page.on('pageerror',e=>errors.push(e.message));
  await context.setCookie({name:cfg.cookie.slice(0,split),value:cfg.cookie.slice(split+1),url:cfg.base,httpOnly:true,sameSite:'Strict'});
  await page.goto(cfg.base+'/native-runtime.html?view=admin&lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});
  await waitForSettingsState(page,'loaded');
  const before=await page.evaluate(async()=>{const r=await fetch('/api/v1/native-settings');if(!r.ok)throw Error('installed-settings-read-refused');return r.json()});
  assert.equal(before.values.cockpit_default_autonomy,'2');checks.push(label+'-saved-default');
  await page.click('[data-action=edit]');await page.$eval('[data-settings-group=planning]',e=>{e.open=true});
  assert.equal(await page.$eval('select[data-setting=cockpit_default_autonomy]',e=>e.value),'2');checks.push(label+'-reopened-exact-choice');
  await page.keyboard.press('Escape');assert(await page.$eval('[data-action=edit]',e=>e===document.activeElement));checks.push(label+'-escape-focus');
  const after=await page.evaluate(async()=>{const r=await fetch('/api/v1/native-settings');if(!r.ok)throw Error('installed-settings-reread-refused');return r.json()});
  assert.deepEqual(after,before);checks.push(label+'-draft-did-not-adopt-other-groups');await context.close();
 }
 assert.deepEqual(errors,[]);assert.deepEqual(posts,[]);
 fs.writeFileSync(path.join(cfg.output,'installed-autonomy-ui.json'),JSON.stringify({status:'PASS',checks,posts,errors,read_only:true,global_watchdog:null},null,2)+'\n');
 console.log('INSTALLED_AUTONOMY_UI_VERIFIED '+checks.length);
}finally{if(browser)await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
