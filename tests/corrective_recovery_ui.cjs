'use strict';
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const [binary,out,root,work]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined}));
const read=()=>cli(['work','show',work]).work;const before=read();
let browser,server,page,expectedConflict=false;const errors=[],failed=[];let conflicts=0,paidCalls=0;
(async()=>{
 server=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const address=await new Promise((resolve,reject)=>{let text='';server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});page=await browser.newPage();
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{if(m.type()==='error'&&!(expectedConflict&&m.text().includes('409')))errors.push(m.text())});
 page.on('response',r=>{if(r.status()>=400){if(expectedConflict&&r.status()===409&&r.url().includes('authorize-recovery'))conflicts++;else failed.push(r.status()+' '+r.url())}});
 page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED'))failed.push(r.url())});
 page.on('request',r=>{if(r.url().includes('/api/v1/assist/ask'))paidCalls++});
 const url=new URL(address);url.searchParams.set('work',work);await page.setViewport({width:1440,height:1050});
 for(const lang of ['fr','en']){
  url.searchParams.set('lang',lang);await page.goto(url.href);await page.waitForSelector('[data-corrective-recovery="first"]');
  for(const theme of ['etat','sombre']){
   await page.evaluate(t=>setTheme(t),theme);await page.click('[data-corrective-recovery="first"]');await page.waitForSelector('#corrective-recovery-summary');
   assert.match(await page.$eval('#corrective-recovery-summary',e=>e.textContent),/3 \/ 3 → 4/);
   assert.match(await page.$eval('#field-recovery_instruction',e=>e.value),/Missing evidence/);
   assert.equal(await page.$eval('#action-form',e=>e.checkValidity()),true);
   if(lang==='en')assert.doesNotMatch(await page.$eval('#modal',e=>e.innerText),/Pourquoi autoriser|Tentatives consommées|Autoriser cet|Correction proposée/);
   await page.click('#modal-help-toggle');await page.waitForSelector('[data-topic="corrective-recovery"]');await page.click('#modal-help-toggle');await page.focus('#field-recovery_instruction');await page.screenshot({path:path.join(out,`recovery-${lang}-${theme}.png`)});
   await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#modal').open);
   assert.equal(await page.evaluate(()=>document.activeElement?.dataset.correctiveRecovery),'first','Escape focus');assert.deepEqual(read(),before,'cancel mutated work');
  }
 }
 await page.click('[data-mission-action="first"]');await page.waitForFunction(()=>document.querySelector('#modal').open);
 await page.click('#modal [data-corrective-recovery="first"]');await page.waitForSelector('#corrective-recovery-summary');
 const current=read();cli(['task','update',work],{schema_version:1,event_id:crypto.randomUUID(),expected_revision:current.revision,id:'first',next:'Concurrent update for stale authorization test'});
 expectedConflict=true;await page.click('#confirm');await page.waitForFunction(()=>!document.querySelector('#modal-error').hidden);
 assert.equal(conflicts,1);assert.equal(read().tasks[0].plan_max_attempts,3);await page.keyboard.press('Escape');expectedConflict=false;
 await page.reload();await page.waitForSelector('[data-corrective-recovery="first"]');await page.click('[data-corrective-recovery="first"]');await page.waitForSelector('#corrective-recovery-summary');
 await page.setViewport({width:390,height:844});await page.screenshot({path:path.join(out,'recovery-mobile.png')});assert.equal(await page.$eval('#modal',e=>e.scrollWidth<=e.clientWidth+1),true,'mobile overflow');
 await page.click('#confirm');await page.waitForFunction(()=>snapshot.work.tasks[0].corrective_recovery);
 const after=read();assert.equal(after.tasks[0].plan_max_attempts,4);assert.deepEqual(after.tasks[0].attempts,before.tasks[0].attempts);assert.deepEqual(after.tasks[0].independent_review,before.tasks[0].independent_review);assert.deepEqual(after.planning,before.planning);
 await page.reload();await page.waitForFunction(()=>snapshot?.work.tasks[0].corrective_recovery);
 assert.equal(await page.$('[data-corrective-recovery="first"]'),null,'duplicate authorization button');
 assert.equal(await page.evaluate(()=>snapshot.mission.paused),true);assert.equal(await page.evaluate(()=>snapshot.agents.length),0);
 assert.equal(await page.evaluate(()=>snapshot.mission.tasks.find(t=>t.id==='first').state),'waiting');
 assert.equal(paidCalls,0);assert.deepEqual(errors,[]);assert.deepEqual(failed,[]);
 const result={status:'PASS',checks:['real isolated server and CLI','FR/EN and both themes','review-based editable instruction','priority and diagnosis buttons','Escape and preserved state','stale confirmation refused','single persisted grant','pause retained; no providers called','mobile layout','no browser errors']};fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
})().catch(async e=>{console.error(e);console.error(JSON.stringify({errors,failed}));if(page){await page.screenshot({path:path.join(out,'failure.png')});console.error(await page.$eval('#modal',e=>e.innerText))}process.exitCode=1}).finally(async()=>{await browser?.close();server?.kill()});
