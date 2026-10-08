'use strict';
// Public CLI setup in an isolated store; no provider calls or live mission writes.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-planning-metrics-'));let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',c=>c?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);
 let w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Planning metrics',objective:'Verify recorded decisions differ from pending handoffs',scope:'isolated',criteria:['Correct metrics'],next:'Inspect counters'})).work;
 w=await cli(['planning','enable',w.id],{schema_version:1,event_id:'enable',expected_revision:w.revision,max_tasks:10,max_decisions:10,max_activations:10});
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1400,height:1000});
 const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang','fr');await page.goto(u.href);
 await page.waitForFunction(()=>document.getElementById('decision-count')?.textContent==='1');
 if(await page.$eval('#mode',e=>/expert/.test(e.textContent)))await page.click('#mode');
 assert.equal(await page.$eval('#decision-count-label',e=>e.textContent),'Retours à traiter');
 w=await cli(['planning','claim',w.id],{schema_version:1,event_id:'claim',expected_revision:w.revision,scope:'root',scope_revision:1,holder:'native-metrics-test',lease_seconds:60});
 let scope=w.planning.scopes[0];
 w=await cli(['planning','decide',w.id],{schema_version:1,event_id:'decide',expected_revision:w.revision,scope:'root',scope_revision:scope.revision,generation:scope.generation,holder:scope.holder,input_events:scope.delivery.events,operations:[],reason:'Observe brief without production or provider invocation'});
 await page.reload();await page.waitForFunction(()=>document.getElementById('decision-count')?.textContent==='0');
 // One registered activation, zero pending events and no model invocation.
 for(const lang of ['fr','en']){
  u.searchParams.set('lang',lang);await page.goto(u.href);
  await page.waitForFunction(()=>document.getElementById('decision-count')?.textContent==='0');
  assert.equal(await page.$eval('#decision-count-label',e=>e.textContent),lang==='fr'?'Retours à traiter':'Pending handoffs');
  const text=await page.$eval('#planning-ownership',e=>e.textContent);
  assert.match(text,lang==='fr'?/1\/10 décisions enregistrées · 1\/10 activations de planification/:/1\/10 recorded decisions · 1\/10 planning activations/);
  assert.doesNotMatch(text,/appels IA|AI calls/);
  for(const theme of ['etat','sombre']){
   await page.evaluate(t=>setTheme(t),theme);
   const missing=await page.evaluate(()=>{const css=[...document.styleSheets].flatMap(s=>[...s.cssRules].map(r=>r.cssText)).join(''),tokens=[...new Set([...css.matchAll(/var\((--wattson-[\w-]+)/g)].map(m=>m[1]))],style=getComputedStyle(document.documentElement);return tokens.filter(t=>!style.getPropertyValue(t).trim())});assert.deepEqual(missing,[]);
   await page.screenshot({path:path.join(out,`metrics-${lang}-${theme}.png`)});
   await (await page.$('#planning-ownership')).screenshot({path:path.join(out,`planning-${lang}-${theme}.png`)});
  }
  const helpLabel=lang==='fr'?'Comprendre les décisions':'Understand decisions';
  for(const b of await page.$$('#planning-ownership button')){if(await b.evaluate(e=>e.textContent)===helpLabel){await b.click();break}}
  await page.waitForSelector('#modal[open]');
  assert.match(await page.$eval('#modal',e=>e.textContent),lang==='fr'?/ne prouve pas qu’un appel a été envoyé au fournisseur/:/does not prove a request was sent to the provider/);
  await page.keyboard.press('Escape');await page.waitForSelector('#modal:not([open])');
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['pending 1 before acknowledgement, 0 after a recorded decision','recorded decisions and activations have distinct labels','FR/EN and both themes, tokens defined','help explains activation is not a provider invocation, Escape closes'],errors},null,2));console.log('PASS: truthful planning metrics, FR/EN, both themes and help');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
