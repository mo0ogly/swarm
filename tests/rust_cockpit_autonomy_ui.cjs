'use strict';
/** Actual embedded Admin/cockpit over authenticated HTTP on an owned fixture.
 * Explicit saves exercise preview, persistence and the real read consumer in
 * FR/EN and both themes. No provider, live service, polling policy or watchdog.
 * Session credentials arrive only on stdin and are never retained in artifacts.
 */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto'),puppeteer=require('puppeteer');
const {waitForSettingsState}=require('./helpers/engine_settings_admin_state.cjs');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),out=path.resolve(cfg.output),checks=[],errors=[];
fs.mkdirSync(out,{mode:0o700});
const select='select[data-setting="cockpit_default_autonomy"]';
let browser,primary;
/** Assert actual content/state; retain names only after success. */
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name);}
(async()=>{
 try{
  browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox']});
  for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
   const context=await browser.createBrowserContext(),page=await context.newPage(),address='http://'+cfg.address,label=lang+'-'+theme;
   const cookie='swarm_session_'+crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0,12);
   page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
   page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});page.on('requestfailed',r=>errors.push(new URL(r.url()).pathname));
   await context.setCookie({name:cookie,value:cfg.key,url:address,httpOnly:true,sameSite:'Strict'});
   await page.goto(address+'/native-runtime.html?view=admin&lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});
   await waitForSettingsState(page,'loaded');
   if(lang==='fr'&&theme==='etat')for(const [route,expected] of Object.entries(cfg.assets)){
    const actual=await page.evaluate(async route=>{const r=await fetch(route);if(!r.ok)throw Error('fixture-asset-read-refused');const sum=await crypto.subtle.digest('SHA-256',await r.arrayBuffer());return Array.from(new Uint8Array(sum),b=>b.toString(16).padStart(2,'0')).join('')},route);
    check('actual-protected-build-asset-'+route,actual===expected);
   }
   /** Read genuine protected observations; any HTTP/JSON failure aborts. */
   const read=route=>page.evaluate(async route=>{const r=await fetch(route);if(!r.ok)throw Error('fixture-protected-read-refused');return r.json()},route);
   const before=await read('/api/v1/native-settings');
   /** Open the saved editor and reveal only the relevant existing accordion. */
   async function open(){await page.click('[data-action=edit]');await page.waitForSelector(select);await page.$eval('[data-settings-group=planning]',e=>{e.open=true});}
   await open();
   check(label+'-exact-translated-select',await page.$$eval(select+' option',o=>o.map(e=>e.textContent).join('|'))===(lang==='en'?'Manual|Assisted|Autonomous':'Manuel|Assisté|Autonome'));
   check(label+'-initial-default',await page.$eval(select,e=>e.value)==='2');
   await page.focus(select);check(label+'-keyboard-focus',await page.$eval(select,e=>e===document.activeElement));
   await page.click('[data-settings-help="cockpit_default_autonomy"]');await page.waitForSelector('dialog.engine-settings-help[open]');
   const helpText=await page.$eval('dialog.engine-settings-help[open]',e=>e.textContent);
   fs.writeFileSync(path.join(out,label+'-autonomy-help.txt'),helpText);
   check(label+'-help-consent-contract',helpText.includes('consent'));
   await page.keyboard.press('Escape');check(label+'-help-focus-restored',await page.$eval('[data-settings-help="cockpit_default_autonomy"]',e=>e===document.activeElement));
   for(const width of [1440,390]){
    await page.setViewport({width,height:1000});
    check(label+'-no-overflow-'+width,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    await page.screenshot({path:path.join(out,label+'-'+width+'.png')});
   }
   await page.setViewport({width:1440,height:1000});
   /** Save an explicit choice through the actual modal, then read its consumer.
    * Preview must not publish; apply is acknowledged and reread. No direct file
    * writes, guessed revisions or inferred permission are involved.
    */
   async function save(mode,wire){
    const old=await read('/api/v1/native-settings');
    await page.select(select,mode);
    await page.$eval('[data-setting-reason]',(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},'Explicit owned autonomy '+label+' '+mode);
    await page.click('[data-action=confirm]');await waitForSettingsState(page,'preview');
    check(label+'-'+mode+'-preview-no-write',(await read('/api/v1/native-settings')).revision===old.revision);
    await page.click('[data-action=confirm]');await waitForSettingsState(page,'saved');
    const saved=await read('/api/v1/native-settings'),observed=await read('/api/v1/cockpit-read?work='+encodeURIComponent(cfg.work));
    check(label+'-'+mode+'-save-and-real-consumer',saved.values.cockpit_default_autonomy===mode&&BigInt(saved.revision)===BigInt(old.revision)+1n&&observed.autonomy_observation.autonomy===wire&&observed.autonomy_observation.source==='installation'&&observed.autonomy_observation.launch_authorized===false&&observed.autonomy_observation.consent_enabled===false);
    await open();check(label+'-'+mode+'-reopened',await page.$eval(select,e=>e.value)===mode);
   }
   await save('0','manuel');await save('2','autonome');
   await page.keyboard.press('Escape');check(label+'-close-focus-restored',await page.$eval('[data-action=edit]',e=>e===document.activeElement));
   check(label+'-history-preserved',(await read('/api/v1/native-settings')).history.slice(0,before.history.length).every((v,i)=>JSON.stringify(v)===JSON.stringify(before.history[i])));
   await page.goto(address+'/?work='+encodeURIComponent(cfg.work)+'&view=agents&lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});
   await page.waitForFunction(()=>!document.querySelector('#main-content').hasAttribute('aria-busy'));
   check(label+'-actual-cockpit-mode',await page.$eval('#agents-list',(e,lang)=>e.textContent.includes(lang==='en'?'Autonomous':'Autonome')&&e.textContent.includes(lang==='en'?'installation settings':"réglages de l'installation"),lang));
   if(lang==='fr'&&theme==='etat'){
    const verdicts=await page.evaluate(async work=>{
     const response=await fetch('/api/v1/cockpit-read?work='+encodeURIComponent(work));if(!response.ok)throw Error('fixture-positive-dto-read-refused');
     const healthy=await response.json(),panel=SwarmRustCockpitPanels,container=document.querySelector('#agents-list');
     panel.render(healthy);const before=container.textContent,results=[];
     for(const patch of [{autonomy:'invented'},{source:'invented'},{autonomy:null},{consent_enabled:'true'},{launch_authorized:true},{settings_revision:2},{settings_revision:'01'}]){
      let refused=false;try{panel.render({...healthy,autonomy_observation:{...healthy.autonomy_observation,...patch}})}catch(error){refused=error.message==='web-native-observation-invalid'}
      results.push(refused&&container.textContent===before);
     }
     panel.render({...healthy,autonomy_observation:{...healthy.autonomy_observation,settings_revision:'9007199254740993'}});results.push(container.textContent===before);
     const missing={...healthy};delete missing.autonomy_observation;panel.render(missing);results.push(container.textContent.includes('Le mode d’autonomie n’est pas observé dans cette réponse.'));
     panel.render(healthy);results.push(container.textContent===before);return results;
    },cfg.work);
    verdicts.forEach((value,index)=>check('fixture-autonomy-dto-refusal-and-recovery-'+index,value));
   }
   await context.close();
  }
  assert.deepEqual(errors,[]);
  fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks,errors,browser:await browser.version(),node:process.version,puppeteer:require('puppeteer/package.json').version,provider_started:false,installed:false,global_watchdog:null},null,2));
  console.log('COCKPIT_AUTONOMY_BROWSER_VERIFIED '+checks.length);
 }catch(error){primary=error;fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({status:'FAIL',cause:error.stack,checks,errors},null,2));}
 finally{try{if(browser)await browser.close()}catch(cleanup){if(primary)throw new AggregateError([primary,cleanup],'Browser and cleanup failed');throw cleanup}}
 if(primary)throw primary;
})().catch(e=>{console.error(e);process.exitCode=1});
