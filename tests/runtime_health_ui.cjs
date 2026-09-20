'use strict';
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const [binary,out,root,work]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
const read=()=>JSON.parse(execFileSync(binary,['--root',root,'--json','work','show',work],{encoding:'utf8'})).work;
const before=read();let browser,server,page,mode='blocked',snapshotFails=false;const errors=[],failures=[];let paidCalls=0;
(async()=>{
 server=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const address=await new Promise((resolve,reject)=>{let text='';server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});page=await browser.newPage();
 page.on('pageerror',e=>errors.push(e.message));page.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED'))failures.push(r.url())});
 await page.setRequestInterception(true);
 page.on('request',req=>{
  const u=new URL(req.url());
  if(u.pathname==='/api/v1/assist/ask')paidCalls++;
  if(u.pathname==='/api/v1/snapshot'&&snapshotFails)return req.respond({status:500,contentType:'application/json',body:'{"error":"fixture: database unavailable"}'});
  if(u.pathname==='/api/v1/runtime-health'){
   const h={state:mode,launch_allowed:mode==='ready',observed_at:new Date().toISOString(),message:mode==='ready'?'Le stockage est disponible.':'Stockage indisponible ou presque plein : nouveaux départs suspendus.',next_step:'Libérez de l’espace sur le volume signalé ou rétablissez son accès, puis vérifiez à nouveau. Swarm reprendra les opérations autorisées lorsque le stockage sera disponible ; les limites et les avis de revue restent applicables.',volumes:[{kind:'store',path:root,available_bytes:mode==='ready'?4294967296:0,state:mode}]};
   return req.respond({contentType:'application/json',body:JSON.stringify(h)});
  }req.continue();
 });
 const url=new URL(address);url.searchParams.set('work',work);await page.setViewport({width:1440,height:1050});
 for(const lang of ['fr','en']){
  url.searchParams.set('lang',lang);mode='blocked';await page.goto(url.href);await page.waitForFunction(()=>snapshot?.mission.tasks.some(t=>t.attempt_limit_reached));
  for(const theme of ['etat','sombre']){
   mode='blocked';await page.evaluate(async t=>{setTheme(t);await RuntimeHealthPanel.check()},theme);
   await page.waitForSelector('#runtime-health:not([hidden]) button');await page.focus('#runtime-health button');await page.screenshot({path:path.join(out,`incident-${lang}-${theme}.png`)});
   await page.click('#runtime-health button');await page.waitForFunction(()=>document.querySelector('#modal').open);
   const txt=await page.$eval('#modal',e=>e.innerText);assert.match(txt,lang==='fr'?/Stockage de Swarm/:/Swarm storage/);assert.match(txt,/0 MiB/);
   await page.screenshot({path:path.join(out,`diagnostic-${lang}-${theme}.png`)});
   mode='ready';await page.click('#modal-fields button');await page.waitForSelector('#runtime-health[hidden]');assert.equal(await page.$eval('#modal',e=>e.open),true);
   await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#modal').open);
   await page.click('[data-mission-action="first"]');await page.waitForFunction(()=>document.querySelector('#modal').open);
   assert.equal(await page.$eval('#confirm',e=>e.hidden),true);assert.match(await page.$eval('#modal',e=>e.innerText),/Missing evidence/);
   assert.match(await page.$eval('#modal',e=>e.innerText),lang==='fr'?/trois tentatives/:/three-attempt/);
   await page.screenshot({path:path.join(out,`attempts-${lang}-${theme}.png`)});await page.keyboard.press('Escape');
  }
 }
 const download=path.join(out,'downloads');fs.mkdirSync(download,{recursive:true});const client=await page.createCDPSession();await client.send('Page.setDownloadBehavior',{behavior:'allow',downloadPath:download});
 await page.click('[data-mission-action="first"]');await page.waitForFunction(()=>document.querySelector('#modal').open);
 const exportButton=await page.$$('#modal-fields button');await exportButton[exportButton.length-1].click();
 const exported=path.join(download,'swarm-reprise-first.json');for(let i=0;i<40&&!fs.existsSync(exported);i++)await new Promise(r=>setTimeout(r,100));
 const evidence=JSON.parse(fs.readFileSync(exported,'utf8'));assert.equal(evidence.work,work);assert.equal(evidence.revision,before.revision);assert.equal(evidence.attempts.length,3);assert.equal(evidence.independent_review.state,'changes_requested');await page.keyboard.press('Escape');
 mode='blocked';snapshotFails=true;await page.evaluate(async()=>{await refresh(true);await RuntimeHealthPanel.check()});
 assert.equal(await page.$eval('#runtime-health',e=>e.hidden),false,'diagnosis lost with database failure');
 await page.click('#runtime-health button');assert.equal(await page.$eval('#modal',e=>e.open),true);
 await page.setViewport({width:390,height:844});assert.equal(await page.$eval('#modal',e=>e.scrollWidth<=e.clientWidth+1),true,'mobile overflow');assert.equal(await page.$eval('#modal-fields .mission-task',e=>e.scrollWidth<=e.clientWidth+1),true,'storage path clipped');await page.screenshot({path:path.join(out,'mobile.png')});
 assert.equal(paidCalls,0,'diagnosis implicitly used AI');assert.deepEqual(read(),before,'diagnosis changed work');assert.deepEqual(errors,[]);assert.deepEqual(failures,[]);
 const result={status:'PASS',checks:['real isolated server and CLI','disk incident UI with simulated measurements','independent diagnosis despite snapshot failure','manual recheck clears incident','three-attempt refusal diagnosis','French/English dark/light keyboard/mobile','zero AI calls','unchanged task history']};fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
})().catch(async e=>{console.error(e);if(page){await page.screenshot({path:path.join(out,'failure.png')});console.error(await page.$eval('#modal',e=>e.innerText))}process.exitCode=1}).finally(async()=>{await browser?.close();server?.kill()});
