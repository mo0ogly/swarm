'use strict';
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]||'test-results/projects-method');fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-projects-method-'));execFileSync(binary,['--root',root,'init']);
const app=spawn(binary,['--root',root,'web','127.0.0.1:0']);let browser;const errors=[],mutations=[];
(async()=>{
 const session=await new Promise((resolve,reject)=>{let s='';const timer=setTimeout(()=>reject(Error('startup timeout')),20000);app.stdout.on('data',x=>{s+=x;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});app.on('exit',c=>reject(Error('exit '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const p=await browser.newPage();p.on('pageerror',e=>errors.push(e.message));p.on('request',r=>{if(r.method()!=='GET')mutations.push(r.url())});await p.goto(session);const base=new URL(session).origin;
 const titles={fr:['Examiner l’existant','Concevoir les écrans','Planifier les tâches','Réaliser','Vérifier le résultat','Livrer'],en:['Examine what exists','Design the screens','Plan the tasks','Implement','Check result','Deliver']};
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  await p.setViewport({width:1440,height:1100});await p.goto(base+'/projects.html?lang='+lang);await p.waitForSelector('#project-models>li');if(await p.$eval('html',e=>e.dataset.theme)!==theme)await p.click('#project-theme');
  assert.equal(await p.$eval('#project-method-dialog',e=>e.open),false);
  await p.focus('#project-method-open');await p.keyboard.press('Enter');await p.waitForSelector('#project-method-dialog[open]');assert.equal(await p.evaluate(()=>document.activeElement.id),'project-method-close');
  assert.equal(await p.$$('.method-foundation li').then(x=>x.length),5);assert.equal(await p.$$('#project-method-stages button').then(x=>x.length),6);
  assert.match(await p.$eval('#method-feature-title',e=>e.textContent),lang==='fr'?/6 étapes à répéter/:/6 stages to repeat/);
  for(let index=0;index<6;index++){
   await p.focus('[data-method-stage="'+index+'"]');await p.keyboard.press('Enter');
   assert.equal(await p.$eval('#project-method-stage-title',e=>e.textContent),titles[lang][index]);
   assert.equal(await p.$$('#project-method-stages [aria-pressed=true]').then(x=>x.length),1);
   assert.equal(await p.$eval('[data-method-stage="'+index+'"]',e=>e.getAttribute('aria-pressed')),'true');
   assert.ok((await p.$eval('#project-method-stage-result',e=>e.textContent)).length>60);assert.ok((await p.$eval('#project-method-stage-proof',e=>e.textContent)).length>35);
  }
  await p.screenshot({path:path.join(out,lang+'-'+theme+'-deliver.png')});
  await p.click('[data-method-stage="2"]');await p.screenshot({path:path.join(out,lang+'-'+theme+'-plan.png')});
  for(const width of [1024,760,390,320]){await p.setViewport({width,height:900});assert.ok(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'page overflow '+width);assert.ok(await p.$eval('#project-method-dialog',e=>e.scrollWidth<=e.clientWidth+1),'dialog overflow '+width)}
  await p.$eval('#project-method-dialog',e=>{e.scrollTop=0});await p.screenshot({path:path.join(out,lang+'-'+theme+'-mobile-foundation.png')});
  await p.$eval('.method-feature',e=>e.scrollIntoView({block:'start'}));await p.screenshot({path:path.join(out,lang+'-'+theme+'-mobile.png')});
  await p.keyboard.press('Escape');await p.waitForFunction(()=>!document.getElementById('project-method-dialog').open);assert.equal(await p.evaluate(()=>document.activeElement.id),'project-method-open');
  await p.click('#project-method-open');assert.equal(await p.$eval('[data-method-stage="0"]',e=>e.getAttribute('aria-pressed')),'true');
  await p.focus('[data-method-stage="0"]');await p.keyboard.down('Shift');await p.keyboard.press('Tab');await p.keyboard.up('Shift');assert.equal(await p.evaluate(()=>document.activeElement.id),'project-method-close');
  for(let n=0;n<12;n++){await p.keyboard.press('Tab');assert.ok(await p.evaluate(()=>document.activeElement.closest('#project-method-dialog')),'native focus trap')}
  await p.click('#project-method-close');await p.waitForFunction(()=>document.activeElement.id==='project-method-open');
 }
 await p.setRequestInterception(true);const intercept=r=>r.url().endsWith('/api/v1/preparations/templates')?r.respond({status:503,contentType:'application/json',body:'{}'}):r.continue();p.on('request',intercept);await p.goto(base+'/projects.html?lang=en');await p.waitForSelector('#project-model-retry:not([hidden])');await p.click('#project-method-open');assert.equal(await p.$eval('#project-method-dialog',e=>e.open),true);await p.keyboard.press('Escape');p.off('request',intercept);await p.setRequestInterception(false);
 assert.deepEqual(errors,[]);assert.deepEqual(mutations,[]);
 const works=JSON.parse(execFileSync(binary,['--root',root,'--json','work','list'],{encoding:'utf8'}));assert.deepEqual(works,[]);
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({result:'PASS',languages:['fr','en'],themes:['etat','sombre'],widths:[1440,1024,760,390,320],checks:['five-shared-six-repeated','six-distinct-stage-outcomes','translation','keyboard-selection','escape-focus-return','close-focus-return','tab-shift-tab-focus-wrap','reopen-reset','catalog-failure-independent','no-mutations-no-missions'],errors,mutations,root},null,2));console.log('PASS product workflow preview on real isolated server');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();app.kill('SIGTERM')});
