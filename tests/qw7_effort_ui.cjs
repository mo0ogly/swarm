'use strict';
// Real cockpit and native dialog on an isolated Go-exported SQLite fixture.
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]||'/tmp/qw7-evidence'),root=path.join(out,'store'),work=fs.readFileSync(path.join(out,'work-id.txt'),'utf8').trim();
async function cli(){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json','work','show',work]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',code=>code?reject(Error(err)):resolve(JSON.parse(text)))})}
(async()=>{
 let app,browser;try{
 const before=await cli();app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const match=text.match(/http:\/\/[^\s]+/);if(match)resolve(match[0])});app.on('exit',c=>reject(Error('server exit '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage(),errors=[],observations=[];page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1400,height:1000});
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre']){
  const u=new URL(url);u.searchParams.set('work',work);u.searchParams.set('lang',lang);await page.goto(u.href);await page.waitForFunction(()=>typeof snapshot!=='undefined'&&snapshot?.mission?.spending?.rows?.some(r=>r.id==='effort'));
  await page.waitForSelector('[data-mission-action="spending"]',{visible:true});await page.evaluate(()=>{if(typeof stream!=='undefined')stream?.close()});
  if(await page.$eval('html',e=>e.dataset.theme)!==theme)await page.click('#theme');
  assert.equal(await page.$eval('html',e=>e.dataset.theme),theme);await page.focus('[data-mission-action="spending"]');await page.keyboard.press('Enter');await page.waitForSelector('#modal[open]');
  const text=await page.$eval('#modal-fields',e=>e.innerText);
  assert.match(text,/150\.000 s/);assert.match(text,lang==='fr'?/2 revue\(s\).*1 reprise\(s\)/:/2 review\(s\).*1 retr/);
  assert.match(text,lang==='fr'?/durée écoulée de la mission/:/elapsed mission time/);
  const unknown=await page.$eval('#modal [data-attempt-ledger="missing"]',e=>e.innerText);assert.match(unknown,lang==='fr'?/Appels d’outils non rapportés/:/Tool calls not reported/);assert.match(unknown,lang==='fr'?/Durée du processus : non rapporté/:/Process duration: not reported/);assert.doesNotMatch(unknown,/0\.000 s|0\.00 USD/);
  if(lang==='en')assert.doesNotMatch(text,/Durée du processus|revue\(s\)|Départ :|Fin :|activité reçue|USD rapportés|reprise\(s\)/);
  await page.$eval('#modal .modal-body',e=>e.scrollTop=0);await page.screenshot({path:path.join(out,`summary-${lang}-${theme}.png`)});
  await page.$eval('#modal [data-attempt-ledger="missing"]',e=>{const b=e.closest('.modal-body');b.scrollTop=e.offsetTop-b.offsetTop});await page.waitForFunction(()=>{const e=document.querySelector('#modal [data-attempt-ledger="missing"]'),r=e.getBoundingClientRect(),b=document.querySelector('#modal .modal-body').getBoundingClientRect();return r.top<b.bottom&&r.bottom>b.top});await page.screenshot({path:path.join(out,`attempts-${lang}-${theme}.png`)});
  await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#modal').open);assert.equal(await page.evaluate(()=>document.activeElement.dataset.missionAction),'spending');
  observations.push({lang,theme,task_duration_ms:150000,tools:11,reviews:2,retries:1,missing_duration:null,missing_tools:'unreported',missing_cost:'unreported',opened_by_keyboard:true,escape_focus:'spending'});
 }
 const after=await cli();assert.equal(after.work.revision,before.work.revision);assert.deepEqual(after.events,before.events);assert.deepEqual(errors,[]);
 const result={status:'PASS',fixture:'isolated synthetic telemetry, real Go CLI/HTTP and cockpit; no real provider execution claimed',observations,history_preserved:true,console_errors:errors};fs.writeFileSync(path.join(out,'browser-result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
 }finally{if(browser)await browser.close();if(app)app.kill('SIGTERM')}
})().catch(e=>{console.error(e);process.exitCode=1});
