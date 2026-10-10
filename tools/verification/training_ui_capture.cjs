'use strict';
// Refresh genuine Swarm tutorial captures, in a disposable CLI/API workshop.
// No agent starts, no validation policy is applied, no real mission is used.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict');
const {execFileSync,spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);
if(!process.argv[2]||!process.argv[3])throw Error('usage: training_ui_capture.cjs BINARY OUTPUT');
fs.mkdirSync(out,{recursive:true});const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-training-ui-'));
fs.cpSync(path.resolve('docs/training/casa-pizza/projet-pizza'),root,{recursive:true});
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined}));
cli(['init']);execFileSync('python3',['docs/training/casa-pizza/creer_plan_pedagogique.py','--swarm',binary,'--projet',root]);
const w=cli(['work','list']).works?.[0]||cli(['work','list'])[0];
if(!w?.id)throw Error('teaching mission missing');
cli(['autonomy',w.id,'manuel']);const before=cli(['work','show',w.id]).work;
const server=spawn(binary,['--root',root,'web','127.0.0.1:0']);let browser,page;
const errors=[],failures=[],inflight=new Set(),navigationAborts=new WeakSet();
async function visit(url){for(const r of inflight)navigationAborts.add(r);await page.goto(url,{waitUntil:'domcontentloaded'})}
async function capture(name,selector){
 const e=await page.$(selector);if(!e)throw Error('capture target missing '+selector);
 if(selector==='#modal'){
  const box=await e.boundingBox(),offset=await page.evaluate(()=>({x:scrollX,y:scrollY}));
  await page.screenshot({path:path.join(out,name+'.png'),clip:{...box,x:box.x+offset.x,y:box.y+offset.y},captureBeyondViewport:false});
 }else{
  await e.evaluate(el=>el.scrollIntoView({block:'start'}));
  await e.screenshot({path:path.join(out,name+'.png')});
 }
}
async function openTasks(){await page.$eval('[data-nav-group=conduite]',e=>e.open=true);await page.click('[data-view=tasks]');await page.waitForSelector('#tasks:not([hidden])')}
async function openValidation(){await openTasks();await page.click('#tasks-body tr[data-task=verify] button:nth-child(2)');await page.waitForSelector('#validation-mode')}
async function fill(field,value){const selector='[data-validation-field='+field+']';await page.$eval(selector,e=>e.scrollIntoView({block:'center'}));await page.focus(selector);await page.$eval(selector,e=>e.value='');await page.type(selector,value)}
(async()=>{
 const session=await new Promise((resolve,reject)=>{let s='';const timer=setTimeout(()=>reject(Error('server startup timeout')),15000);server.stdout.on('data',x=>{s+=x;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});server.on('exit',c=>reject(Error('server exited '+c)))});
 const base=new URL(session).origin;
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 page=await browser.newPage();await page.setViewport({width:1440,height:1100});
 page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 page.on('request',r=>inflight.add(r));page.on('requestfinished',r=>inflight.delete(r));
 page.on('requestfailed',r=>{inflight.delete(r);if(!(r.failure()?.errorText==='net::ERR_ABORTED'&&navigationAborts.has(r)))failures.push(new URL(r.url()).pathname+' '+r.failure()?.errorText)});
 page.on('response',r=>{if(r.status()>=400)failures.push(r.status()+' '+new URL(r.url()).pathname)});
 await visit(session);await page.waitForSelector('.graph-noeud');
 for(const lang of ['fr','en']){
  await visit(base+'/prepare.html?lang='+lang);await page.waitForFunction(()=>!document.querySelector('#create').disabled);
  await page.type('#new-title','Casa Pizza — Atelier pédagogique');
  await page.type('#new-need','Créer une application locale de livraison de pizzas : catalogue, panier, commande persistante, suivi client et espace restaurant. Prix et accès vérifiés côté serveur. Preuves : tests HTTP et recette humaine. Aucun paiement réel.');
  await page.click('#create');await page.waitForFunction(()=>!document.querySelector('#documents').hidden);
  await capture('00-besoin-'+lang,'main');
  await visit(base+'/?work='+w.id+'&lang='+lang+'&view=conduite');await page.waitForFunction(()=>document.querySelectorAll('.graph-noeud').length===6);
  await page.evaluate(()=>setTheme('etat'));await page.click('#pilot-fit');
  await capture('13-graphe-'+lang,'#pilot-canvas');
  await page.click('#nav-collapse');assert.equal(await page.$eval('.rail',e=>getComputedStyle(e).display),'none');
  if(lang==='fr'){
   await capture('overview-01','#pilot-canvas');
   await page.click('#pilot-zoom-in');await capture('overview-02','#pilot-canvas');
   await page.click('.graph-fold[data-task=server]');await capture('overview-03','#pilot-canvas');
   await page.$eval('.graph-tools',e=>e.open=true);await page.click('#pilot-expand');await page.click('#pilot-fit');
   await page.click('.graph-noeud[data-task=journey]');await page.waitForSelector('#pilot-inspector[open]');await capture('overview-04','#pilot-inspector');await page.keyboard.press('Escape');
  }
  await page.click('.graph-noeud[data-task=verify]');await page.waitForSelector('#pilot-inspector[open]');
  await page.click('[data-inspector-action=actions]');await page.waitForSelector('#modal[open]');
  await capture('14-tache-'+lang,'#modal');await page.keyboard.press('Escape');
  await page.click('#pilot-inspector header button');await page.waitForFunction(()=>!document.querySelector('#pilot-inspector').open);
  if(lang==='fr'){await page.evaluate(()=>setTheme('sombre'));await capture('18-graphe-sombre','#pilot-canvas');await page.evaluate(()=>setTheme('etat'))}
  await page.click('#nav-collapse');await openValidation();
  await capture('15-humaine-'+lang,'#modal');
  await page.select('#validation-mode','automatic');await page.click('#validation-add-control');
  await fill('id','tests-http');await fill('inputs','app.py\ntests/test_app.py\ntests/test_http.py');
  await page.select('[data-validation-field=program]','python3');
  await fill('args','-W\nerror::ResourceWarning\n-m\nunittest\n-v');
  await fill('justification','Les tests vérifient les montants, les accès et les transitions des commandes avec HTTP.');
  await page.click('[data-criterion="1"]');
  await page.$eval('[data-validation-field=program]',e=>e.scrollIntoView({block:'center'}));await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await capture('19-commande-'+lang,'#modal');
  assert.equal(await page.$eval('[data-validation-field=program]',e=>{const r=e.getBoundingClientRect();return r.top>0&&r.bottom<innerHeight}),true,'program is visible in command capture');
  await page.$eval('[data-validation-field=timeout]',e=>e.scrollIntoView({block:'center'}));await capture('16-controle-'+lang,'#modal');
  await page.click('#confirm');await page.waitForSelector('#preview:not([hidden])');
  assert.equal(await page.$eval('#modal-error',e=>e.hidden),true);await capture('17-apercu-'+lang,'#modal');
  await page.click('#cancel');await page.waitForFunction(()=>!document.querySelector('#modal').open);
 }
 const after=cli(['work','show',w.id]).work;
 assert.equal(after.revision,before.revision);assert.deepEqual(after.tasks,before.tasks);assert.deepEqual(cli(['agent','list',w.id]).agents,[]);
 assert.deepEqual(errors,[]);assert.deepEqual(failures,[]);
 fs.writeFileSync(path.join(out,'capture-result.json'),JSON.stringify({status:'PASS',binary,root,work:w.id,revision:after.revision,recorded:'2026-10-10',languages:['fr','en'],themes:['etat','sombre'],agents:0,policies_applied:0,errors,failures,captures:fs.readdirSync(out).filter(f=>f.endsWith('.png'))},null,2));
 console.log('PASS real training captures; unchanged teaching plan; preview cancelled; no agents');
})().catch(async e=>{console.error(e);if(page){await page.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({error:e.stack,errors,failures,root},null,2))}process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
