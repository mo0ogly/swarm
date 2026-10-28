'use strict';
// Real server, isolated mission, no agents. Exercise the hidden iframe boundary.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-log-visibility-'));
const cli=(args,data)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(data?['--input','-']:[])],{encoding:'utf8',input:data?JSON.stringify(data):undefined}));
cli(['init']);const work=cli(['work','create'],{schema_version:1,event_id:crypto.randomUUID(),expected_revision:0,title:'Console visibility fixture',objective:'Keep logs readable after hidden resizing',scope:'UI only',criteria:['No hidden layout errors'],next:'Read logs'}).work;cli(['autonomy',work.id,'manuel']);
const app=spawn(binary,['--root',root,'web','127.0.0.1:0']);let browser;const errors=[],pending=[];
(async()=>{
 const session=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});app.on('error',reject);app.on('exit',c=>reject(Error('server exit '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
 page.on('pageerror',e=>errors.push(e.stack));page.on('console',m=>{if(m.type()==='error')pending.push(Promise.all(m.args().map(a=>a.evaluate(v=>v?.stack||v?.message||String(v)).catch(()=>m.text()))).then(values=>errors.push(...values)))});
 await page.setViewport({width:1440,height:1000});await page.goto(session);
 await page.waitForFunction(id=>typeof snapshot!=='undefined'&&snapshot?.work?.id===id,{},work.id);
 await page.$eval('[data-nav-group="suivi"]',e=>e.open=true);await page.click('[data-view="logs"]');
 await page.waitForSelector('#journal-console-frame',{visible:true});const frame=await(await page.$('#journal-console-frame')).contentFrame();
 await frame.waitForSelector('.view-line');await page.waitForFunction(()=>document.getElementById('log-lines').textContent.includes('[work.create]'));
 const original=await page.$eval('#log-lines',e=>e.textContent);assert.match(original,/Console visibility fixture/,'real engine activity is present');
 await page.$eval('[data-nav-group="suivi"]',e=>e.open=true);await page.click('[data-view="budget"]');
 await page.waitForFunction(()=>document.getElementById('logs').hidden);
 console.log('Resizing the hidden journal iframe');
 for(const width of [390,1440]){
  await page.setViewport({width,height:1000});await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
 }
 await page.$eval('[data-nav-group="suivi"]',e=>e.open=true);await page.click('[data-view="logs"]');
 await page.waitForSelector('#journal-console-frame',{visible:true});await frame.waitForSelector('.view-line');
 await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
 assert.equal(await page.$eval('#log-lines',e=>e.textContent),original,'original logs retained after navigation and resizing');
 assert.ok(await frame.$eval('.view-lines',e=>e.textContent.trim().length>0),'rich console repaints when shown');
 await Promise.allSettled(pending);assert.deepEqual(errors,[],'hidden console must never measure DOM wrapping ranges');
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',errors,root,checks:['real engine logs','hide via navigation','mobile/desktop resize while hidden','return and repaint','original content retained','no console errors']},null,2)+'\n');console.log('PASS log console visibility');
})().catch(async e=>{await Promise.allSettled(pending);fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({error:e.stack,errors,root},null,2)+'\n');console.error(e.stack);process.exitCode=1}).finally(async()=>{await browser?.close();app.kill()});
