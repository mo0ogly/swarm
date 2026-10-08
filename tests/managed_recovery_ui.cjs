'use strict';
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {spawn,execFileSync}=require('child_process'),puppeteer=require('puppeteer');
const [binary,out,root,work]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined}));
const read=()=>cli(['work','show',work]).work;const initial=read();
const pending=cli(['agent','prepared',work]);assert.equal(pending.preparations.length,1);assert.equal(pending.preparations[0].id,'prepared-browser');
let browser,server,page;const errors=[],failures=[];
(async()=>{
 server=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const address=await new Promise((resolve,reject)=>{let text='';server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});page=await browser.newPage();
 page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 page.on('response',r=>{if(r.status()>=400)failures.push(r.status()+' '+r.url())});page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED'))failures.push(r.url())});
 const url=new URL(address);url.searchParams.set('work',work);
 await page.setViewport({width:1440,height:1050});
 for(const lang of ['fr','en']){
  url.searchParams.set('lang',lang);await page.goto(url.href);await page.waitForFunction(()=>snapshot?.task_actions?.first);
  for(const theme of ['sombre','etat']){
   await page.evaluate(t=>setTheme(t),theme);await page.evaluate(()=>Pilot.inspect('task','second'));
   await page.waitForFunction(()=>document.body.innerText.includes(SwarmI18n.t('Résultat à compléter')));
   await page.waitForSelector('#pilot-reports [data-inspector-action^="report:"]');
   if(lang==='en')assert.match(await page.$eval('body',e=>e.innerText),/Incomplete delivery: per-criterion delivery manifest missing/);
   await page.click('#pilot-reports [data-inspector-action^="report:"]');
   await page.waitForFunction(()=>document.querySelector('#modal').open && document.querySelector('#modal').textContent.includes('Rapport nouveau second'));
   await page.waitForFunction(()=>document.querySelectorAll('#report-summary [aria-live] p').length===2);
   await page.screenshot({path:path.join(out,`report-${lang}-${theme}.png`)});
   await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#modal').open);
   await page.evaluate(()=>Pilot.inspect('task','first'));
   assert.equal(await page.$eval('#pilot-inspector',e=>e.scrollTop),0,'new task restores the beginning of its inspector');
   await page.waitForSelector('#pilot-inspector [data-recovery-action="resume-launch"]');await page.click('#pilot-inspector [data-recovery-action="resume-launch"]');
   await page.waitForSelector('#prepared-launch-summary');
   const content=await page.$eval('#modal',e=>e.textContent);
   assert.match(content,/Verify prepared launch recovery/);assert.match(content,/managed-review-fixture/);
   if(lang==='en')assert.doesNotMatch(content,/Reprendre le lancement|Le lancement a été|Fournisseur|Consigne pour/);
   assert.equal(await page.$eval('#confirm',e=>e.disabled),false);
   await page.click('#modal-help-toggle');await page.waitForSelector('[data-topic="prepared-launch"]');await page.screenshot({path:path.join(out,`prepared-help-${lang}-${theme}.png`)});await page.click('#modal-help-toggle');
   await page.screenshot({path:path.join(out,`prepared-${lang}-${theme}.png`)});
   await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#modal').open);
   assert.equal(await page.evaluate(()=>document.activeElement?.dataset.recoveryAction),'resume-launch','Escape restores focus');
   assert.deepEqual(read().tasks,initial.tasks,'cancel changed task state');
  }
 }
 await page.click('#pilot-inspector [data-recovery-action="resume-launch"]');await page.waitForSelector('#prepared-launch-summary');
 await page.setViewport({width:390,height:844});assert.equal(await page.$eval('#modal',e=>e.scrollWidth<=e.clientWidth+1),true,'mobile overflow');assert.equal(await page.$eval('#prepared-launch-summary',e=>e.scrollWidth<=e.clientWidth+1),true,'settings clipped on mobile');
 await page.screenshot({path:path.join(out,'prepared-mobile.png')});await page.click('#confirm');
 await page.waitForFunction(()=>snapshot?.work.tasks.find(t=>t.id==='first').attempts?.length===1);
 await page.waitForFunction(()=>snapshot?.agents.some(x=>x.agent.id==='prepared-browser'&&!['queued','starting','running','stopping'].includes(x.agent.status)),{timeout:30000});
 const after=read();assert.equal(after.tasks[0].attempts.length,1);assert.notEqual(after.tasks[0].status,'accepted');
 const replay=cli(['agent','resume-launch',work],{schema_version:1,prepared_id:'prepared-browser',expected_revision:initial.revision});assert.equal(replay.created,false);assert.equal(replay.agent.id,'prepared-browser');
 assert.equal(fs.readFileSync(path.join(root,'review-fixture/worker-starts'),'utf8'),'start\n','provider launched more than once');
 assert.equal(cli(['agent','prepared',work]).preparations.length,0);
 assert.deepEqual(errors,[]);assert.deepEqual(failures,[]);
 const result={status:'PASS',checks:['CLI discovers original preparation','incomplete delivery diagnosis and attributed report opened','visible inspector recovery button','French/English and dark/light','help modal','Escape restores focus','cancel retains state','mobile layout','web resumes original launch','CLI replay does not duplicate provider','one attempt, no acceptance','no browser errors']};fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
})().catch(async e=>{console.error(e);console.error(JSON.stringify({errors,failures}));if(page){await page.screenshot({path:path.join(out,'failure.png')});console.error(await page.$eval('body',e=>e.innerText.slice(-4000)))}process.exitCode=1}).finally(async()=>{await browser?.close();server?.kill()});
