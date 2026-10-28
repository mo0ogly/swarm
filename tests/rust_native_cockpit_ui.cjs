'use strict';
/** Final UX journey against an owned normal Rust HTTP server. Live Work reads
 * precede explicitly intercepted presentation faults; those faults never modify
 * the datastore or establish engine recovery. No provider or mission write is
 * requested. Inputs/secrets arrive through stdin; browser/global watchdogs off. */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const cfg=JSON.parse(fs.readFileSync(0,'utf8')),checks=[],errors=[];let browser;
/** Emit named observable assertions, with no access-key or private response data. */
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name)}
(async()=>{try{
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(cfg.root,'chrome-cockpit')});
 for(const lang of ['fr','en'])for(const theme of ['sombre','etat']){
  const context=await browser.createBrowserContext(),page=await context.newPage(),label=lang+'-'+theme;page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);page.on('pageerror',e=>errors.push(e.message));
  const posts=[];page.on('request',r=>{if(r.method()==='POST')posts.push(new URL(r.url()).pathname)});
  const address='http://'+cfg.address;const crypto=require('node:crypto'),cookie='swarm_session_'+crypto.createHash('sha256').update(cfg.address).digest('hex').slice(0,12);
  await context.setCookie({name:cookie,value:cfg.key,url:address,httpOnly:true,sameSite:'Strict'});
  await page.goto(address+'/native-runtime.html?lang='+lang+'&theme='+theme,{waitUntil:'networkidle0'});
  await page.waitForSelector('[data-work]');check(label+'-actual-mission-list',(await page.$$('[data-work]')).length===2);
  check(label+'-no-automatic-selection',await page.$eval('[data-cockpit-detail]',e=>!e.hasAttribute('aria-busy')&&!e.querySelector('.native-facts')));
  await page.click('[data-work="'+cfg.works[0]+'"]');await page.waitForFunction(()=>!!document.querySelector('[data-cockpit-detail] .native-facts'));
  check(label+'-actual-selected-title',await page.$eval('[data-cockpit-detail] h3',e=>e.textContent)===cfg.title);
  check(label+'-user-text-never-html',!(await page.$('[data-cockpit-detail] img')));
  check(label+'-actual-objective',await page.$eval('.native-objective',e=>e.textContent)==='Owned objective <b>text</b>');
  await page.type('input[type=search]','does-not-match');check(label+'-search-empty',(await page.$$('[data-work]')).length===0);await page.$eval('input[type=search]',e=>{e.value='';e.dispatchEvent(new Event('input',{bubbles:true}))});
  for(const width of [1440,1024,760,390,320]){await page.setViewport({width,height:1000});check(label+'-no-overflow-'+width,await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));if(width===1440||width===390)await page.screenshot({path:path.join(cfg.output,label+'-'+width+'.png'),fullPage:true});}
  await page.click('[data-native-view=admin]');check(label+'-admin-navigation-and-focus',await page.$eval('#native-settings-title',e=>document.activeElement===e)&&await page.$eval('#native-missions-view',e=>e.hidden));
  await page.waitForSelector('[data-action=edit]');await page.click('[data-action=edit]');await page.waitForSelector('dialog[open]');await page.keyboard.press('Escape');check(label+'-admin-dialog-escape',!(await page.$('dialog[open]'))&&await page.$eval('[data-action=edit]',e=>document.activeElement===e));
  await page.click('[data-native-view=missions]');check(label+'-language-and-theme',await page.$eval('html',(e,l,t)=>e.lang===l&&e.dataset.theme===t,lang,theme));check(label+'-translated-heading',await page.$eval('#native-missions-title',e=>e.textContent)===(lang==='en'?'My missions':'Mes missions'));
  if(lang==='fr'&&theme==='sombre'){
   let mode='status-refusal',held,release;
   await page.setRequestInterception(true);
   page.on('request',async r=>{if(r.isInterceptResolutionHandled())return;const u=new URL(r.url());if(u.pathname==='/api/v1/works'&&mode==='invalid-list')return r.respond({status:200,contentType:'application/json',body:'{}'});if(u.pathname==='/api/v1/works'&&mode==='empty-list')return r.respond({status:200,contentType:'application/json',body:'[]'});if(u.pathname==='/api/v1/mission'&&u.searchParams.get('action')==='status'){
    if(mode==='status-refusal')return r.respond({status:200,contentType:'application/json',body:'{}'});
    if(mode==='race'&&u.searchParams.get('work')===cfg.works[0]){held=r;release();return;}
   }await r.continue()});
   await page.click('[data-cockpit-detail] .section-heading button');await page.waitForFunction(()=>!!document.querySelector('[data-cockpit-detail] .native-facts')&&!document.querySelector('[data-cockpit-detail]').hasAttribute('aria-busy'));check('fixture-invalid-status-visible',await page.$eval('[data-cockpit-detail]',e=>e.textContent.includes('web-native-status-invalid')&&e.textContent.includes('données enregistrées')));
   mode='race';const waiting=new Promise(resolve=>{release=resolve});await page.click('[data-work="'+cfg.works[0]+'"]');await waiting;await page.click('[data-work="'+cfg.works[1]+'"]');await held.respond({status:200,contentType:'application/json',body:JSON.stringify({summary:'OLD-REPLY-MUST-NOT-WIN',tasks:[]})});
   await page.waitForFunction(id=>!!document.querySelector('[data-cockpit-detail] .native-facts')&&document.querySelector('[data-cockpit-detail]').textContent.includes(id),{},cfg.works[1]);check('fixture-old-response-cannot-replace-selection',!(await page.$eval('[data-cockpit-detail]',e=>e.textContent.includes('OLD-REPLY-MUST-NOT-WIN'))));
   mode='invalid-list';await page.click('[data-cockpit-refresh]');await page.waitForFunction(()=>document.querySelector('.native-mission-browser [role=status]').textContent.includes('web-native-work-list-invalid'));check('fixture-invalid-list-not-empty-success',(await page.$$('[data-work]')).length===0);
   mode='empty-list';await page.click('[data-cockpit-refresh]');await page.waitForFunction(()=>document.querySelector('.native-mission-browser [role=status]').textContent==='Aucune mission enregistrée.');check('fixture-empty-list-explicit',true);
   mode='healthy';await page.click('[data-cockpit-refresh]');await page.waitForSelector('[data-work]');check('actual-list-recovery',(await page.$$('[data-work]')).length===2);
  }
  check(label+'-no-mission-or-settings-writes',posts.length===0);await context.close();
 }
 check('no-browser-script-errors',errors.length===0);
 fs.writeFileSync(path.join(cfg.output,'result.json'),JSON.stringify({status:'PASS',checks,errors,browser:await browser.version(),scope:'Actual Rust Work/control/status read UI plus separately identified client-response fixtures; no provider, migration/global acceptance or true network fault',watchdogs:null},null,2)+'\n');
 }finally{if(browser)await browser.close()}})().catch(error=>{fs.writeFileSync(path.join(cfg.output,'failure.json'),JSON.stringify({status:'FAIL',checks,error:{name:error.name,message:error.message}},null,2)+'\n');console.error(error.message);process.exitCode=1});
