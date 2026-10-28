'use strict';
/** Actual shared Admin group on an owned preparation-read fixture. Four language/
 * theme cases, saved policy re-opened, help/AI context without provider calls.
 * No watchdog. Record failed requests/console, named assertions and screenshots. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const state=require('./helpers/engine_settings_admin_state.cjs'),cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[],posts=[];let browser;
/** Record only uniquely named observable assertions; failures propagate. */
function check(name,value){assert(value,name);assert(!checks.includes(name));checks.push(name);console.log('PASS '+name)}
(async()=>{try{
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(cfg.root,'chrome-preparation-admin')});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const label=lang+'-'+theme,context=await browser.createBrowserContext(),page=await context.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  const digest=require('node:crypto').createHash('sha256').update(cfg.address).digest('hex');await context.setCookie({name:'swarm_session_'+digest.slice(0,12),value:cfg.key,url:'http://'+cfg.address,httpOnly:true,sameSite:'Strict'});
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});page.on('requestfailed',r=>errors.push(new URL(r.url()).pathname));page.on('response',r=>{if(r.status()>=400)errors.push(new URL(r.url()).pathname+':'+r.status())});page.on('request',r=>{if(!['GET','HEAD'].includes(r.method()))posts.push({method:r.method(),path:new URL(r.url()).pathname})});
  await page.goto('http://'+cfg.address+'/?'+new URLSearchParams({work:cfg.work,view:'admin',lang,theme}),{waitUntil:'networkidle0'});await Promise.all([page.waitForNavigation({waitUntil:'networkidle0'}),page.click('a[href*="native-runtime.html?view=admin"]')]);check(label+'-supported-settings-navigation',new URL(page.url()).pathname==='/native-runtime.html');await state.waitForSettingsState(page,'loaded');
  await page.click('.engine-settings-admin [data-action=edit]');await page.waitForSelector('dialog[data-engine-settings][open]');
  await page.click('[data-settings-group=preparations] > summary');
  check(label+'-logical-translated-group',await page.$eval('[data-settings-group=preparations] > summary',(e,expected)=>e.textContent.includes(expected),lang==='en'?'Preparations and documents':'Préparations et documents'));
  check(label+'-all-eleven-persisted-fields',await page.$$eval('[data-settings-group=preparations] [data-setting]',els=>els.length===11&&els.every(e=>!e.hasAttribute('max'))));
  check(label+'-translated-plan-cap',await page.$eval('[data-setting=preparation_plan_max_bytes]',(e,text)=>Array.from(e.labels||[]).some(l=>l.textContent.includes(text)),lang==='en'?'Normalized plan size':'Taille du plan normalisé'));
  check(label+'-method-is-editable-text',await page.$eval('[data-setting=preparation_default_method]',(e,expected)=>e.type==='text'&&e.inputMode!=='numeric'&&!e.hasAttribute('pattern')&&e.value===expected,label==='fr-etat'?'apex':'debug'));
  check(label+'-actual-saved-value',await page.$eval('[data-setting=preparation_list_limit]',(e,value)=>e.value===value,label==='fr-etat'?'2':'1'));
  await page.click('[data-settings-help=preparation_list_limit]');await page.waitForSelector('.engine-settings-help[open]');
  const helpWitness=await page.$eval('.engine-settings-help[open]',e=>({label:e.getAttribute('aria-labelledby'),heading:document.getElementById(e.getAttribute('aria-labelledby'))?.textContent,text:e.textContent}));fs.writeFileSync(path.join(cfg.output,label+'-help.json'),JSON.stringify(helpWitness,null,2));check(label+'-named-modal-help',!!helpWitness.heading&&helpWitness.text.includes('100'));
  await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.engine-settings-help[open]'));check(label+'-help-focus-restored',await page.$eval('[data-settings-help=preparation_list_limit]',e=>e===document.activeElement));
  await page.click('[data-settings-advice=preparations]');await page.waitForSelector('[data-advice-objective]');await page.type('[data-advice-objective]','Bound preparation reads without losing recorded history.');await page.click('.engine-settings-help[open] .modal-body > button');
  const prompt=await page.$eval('[data-advice-prompt]',e=>e.value),start=prompt.indexOf('\n{');assert(start>=0,'AI context missing');const ai=JSON.parse(prompt.slice(start+1));
  check(label+'-AI-context-actual-contract',ai.group==='preparations'&&ai.document_schema_version===43&&ai.target_fields.length===11&&ai.target_fields.some(f=>f.name==='preparation_list_limit'&&f.saved===(label==='fr-etat'?'2':'1')));
  check(label+'-plan-caps-in-AI-context',['bytes','tasks','assumptions','questions'].every(name=>ai.target_fields.some(f=>f.name==='preparation_plan_max_'+name)));
  await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('.engine-settings-help[open]'));
  for(const width of [1440,390]){await page.setViewport({width,height:1000});check(label+'-no-global-overflow-'+width,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:path.join(cfg.output,label+'-preparation-admin-'+width+'.png'),fullPage:true})}
  if(label==='fr-etat'){
   await page.$eval('[data-setting=preparation_plan_max_tasks]',e=>{e.value='1';e.dispatchEvent(new Event('input',{bubbles:true}))});
   await page.$eval('[data-setting=preparation_default_method]',e=>{e.value='debug';e.dispatchEvent(new Event('input',{bubbles:true}))});
   await page.$eval('[data-setting=preparation_list_limit]',e=>{e.value='1';e.dispatchEvent(new Event('input',{bubbles:true}))});await page.type('[data-setting-reason]','Owned preparation read UI policy witness');
   await page.click('dialog[data-engine-settings] [data-action=confirm]');await state.waitForSettingsState(page,'preview');check('preview-before-publication',await page.$eval('[data-setting-preview]',e=>!e.hidden));await page.click('dialog[data-engine-settings] [data-action=confirm]');await state.waitForSettingsState(page,'saved');
   await page.click('.engine-settings-admin [data-action=edit]');await page.waitForSelector('dialog[data-engine-settings][open]');check('saved-plan-task-cap-reopened',await page.$eval('[data-setting=preparation_plan_max_tasks]',e=>e.value==='1'));check('saved-method-reopened',await page.$eval('[data-setting=preparation_default_method]',e=>e.value==='debug'));check('saved-reopened-value',await page.$eval('[data-setting=preparation_list_limit]',e=>e.value==='1'));
  }
  await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[data-engine-settings]'));await context.close();
 }
 check('no-console-or-request-errors',errors.length===0);check('only-explicit-settings-preview-apply',posts.length===2&&posts.every(p=>p.method==='POST'&&['/api/v1/native-settings/preview','/api/v1/native-settings/apply'].includes(p.path)));
 console.log('PREPARATION_ADMIN_BROWSER_PASS '+checks.length+' assertions');
 }finally{fs.writeFileSync(path.join(cfg.output,'browser-result.json'),JSON.stringify({checks,errors,posts},null,2));if(browser)await browser.close()}
})().catch(e=>{console.error(e.stack);process.exitCode=1});
