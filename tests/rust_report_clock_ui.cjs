'use strict';
/** Protected Admin v40 report clock tolerance: FR/EN and both themes, save/reopen,
 * nonnullable zero/count, pure preview, help/Escape/focus and mobile/desktop.
 * Uses only owned fixture settings; no provider or global watchdog. The native
 * CLI verifies persistence, not launch authority or global migration parity. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{spawnSync}=require('node:child_process'),puppeteer=require('puppeteer');
const state=require('./helpers/engine_settings_admin_state.cjs'),runtimeTools=require('./helpers/browser_verification_runtime.cjs'),{finishOwnedResources}=require('./helpers/verification_cleanup.cjs');
// Current settings schema = newest introduced_schema_version in the catalog.
const CURRENT_SETTINGS=Math.max(...JSON.parse(fs.readFileSync(path.join(__dirname,'../rust/crates/swarm-engine/config/engine-settings-catalog.json'),'utf8')).fields.map(f=>f.introduced_schema_version||0));
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),repo=path.resolve(__dirname,'..'),output=path.resolve(cfg.output);fs.mkdirSync(output,{recursive:false});
let browser,primary,runtime,serial=0;const checks=[],errors=[];
/** Read the authoritative journal with exact retained channels; no implicit retry. */
function shown(){const n=++serial,r=spawnSync(cfg.binary,['engine-settings','show','--root',cfg.root,'--json'],{encoding:'utf8',timeout:0});fs.writeFileSync(path.join(output,'cli-'+n+'.stdout'),r.stdout??'');fs.writeFileSync(path.join(output,'cli-'+n+'.stderr'),r.stderr??'');if(r.error)throw r.error;assert.equal(r.status,0);return JSON.parse(r.stdout)}
(async()=>{try{
 runtime=runtimeTools.begin(repo,output);browser=await puppeteer.launch({...runtimeTools.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',userDataDir:path.join(cfg.root,'chrome-report-clock'),args:['--no-sandbox']});await runtimeTools.recordBrowser(runtime,browser);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const label=lang+'-'+theme;console.log('RUN report-clock '+label);
  const context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  let reject;const failed=new Promise((_,r)=>{reject=r}),guard=p=>Promise.race([p,failed]);
  page.on('pageerror',e=>{errors.push(e.message);reject(e)});page.on('requestfailed',r=>{errors.push(new URL(r.url()).pathname);reject(Error('Resource failed'))});page.on('console',m=>{if(m.type()==='error'){errors.push(m.text());reject(Error('Console error'))}});page.on('response',r=>{if(r.status()>=400){errors.push({path:new URL(r.url()).pathname,status:r.status()});reject(Error('Unexpected HTTP refusal'))}});
  await page.setViewport({width:1440,height:1000});await guard(page.goto('http://'+cfg.address+'/?lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'}));await page.type('#access-key',cfg.key);await guard(Promise.all([page.waitForNavigation({waitUntil:'networkidle0'}),page.click('button[type=submit]')]));await guard(state.waitForSettingsState(page,'loaded'));
  const before=shown();assert.equal(before.schema_version,CURRENT_SETTINGS);// newest catalog version, not a fixed pin
  await page.click('[data-action=edit]');await guard(page.waitForSelector('dialog[data-engine-settings][open]'));await page.click('[data-settings-group=agents] > summary');
  const selector='[data-setting=agent_report_clock_skew_seconds]';
  assert.equal(await page.$('[data-unlimited=agent_report_clock_skew_seconds]'),null,'setting is not nullable');
  assert.equal(await page.$eval(selector,e=>e.tagName),'INPUT');
  await page.click('[data-settings-help=agent_report_clock_skew_seconds]');await guard(page.waitForSelector('dialog.engine-settings-help[open]'));
  const text=await page.$eval('dialog.engine-settings-help[open]',e=>e.textContent);assert(!text.includes('undefined'));assert(lang==='fr'?text.includes('tentative'):text.includes('attempt'),'help translated');
  await page.keyboard.press('Escape');await guard(page.waitForFunction(()=>!document.querySelector('dialog.engine-settings-help[open]')&&document.activeElement?.dataset.settingsHelp==='agent_report_clock_skew_seconds'));
  const bounds=[['agent_report_clock_skew_seconds',['0','1','2','3'][checks.length]]];
  for(const [name,v] of bounds){await page.$eval('[data-setting='+name+']',(e,x)=>{e.value=x;e.dispatchEvent(new Event('input',{bubbles:true}))},v)}
  await page.$eval('[data-setting-reason]',e=>{e.value='Report clock through Admin';e.dispatchEvent(new Event('input',{bubbles:true}))});
  for(const width of [1440,390]){await page.setViewport({width,height:width===390?844:1000});await page.$eval(selector,e=>e.closest('.engine-settings-field').scrollIntoView({block:'center'}));assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:path.join(output,label+'-'+width+'.png'),fullPage:true})}
  await page.click('[data-action=confirm]');await guard(state.waitForSettingsState(page,'preview'));assert.deepEqual(shown(),before);
  await page.click('[data-action=confirm]');await guard(state.waitForSettingsState(page,'saved'));const saved=shown();for(const [name,v] of bounds)assert.equal(saved.values[name],v,name);assert.equal(saved.history.length,before.history.length+1);assert.deepEqual(saved.history.slice(0,-1),before.history);
  await page.click('[data-action=edit]');await guard(page.waitForSelector('dialog[data-engine-settings][open]'));await page.click('[data-settings-group=agents] > summary');for(const [name,v] of bounds){assert.equal(await page.$eval('[data-setting='+name+']',e=>e.value),v)}await page.keyboard.press('Escape');await guard(page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings][open]')));
  checks.push({lang,theme,bounds:Object.fromEntries(bounds),preview_pure:true,saved_reopened:true,history_preserved:true,help_escape_focus:true,desktop_mobile:true});await context.close();
 }
 assert.equal(errors.length,0);const proof=runtimeTools.finish(runtime,repo);fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({status:'PASS',checks,errors,runtime:proof,expected_rejected_requests:0,expected_configuration_outcomes:4,provider_started:false},null,2)+'\n');
}catch(error){primary={error};fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({status:'FAIL',type:error.name,message:error.message,stack:error.stack,errors},null,2)+'\n')}finally{await finishOwnedResources(primary,[()=>browser?.close()])}})();
