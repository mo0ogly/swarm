'use strict';
// Read-only recipe against a named existing work; no agent or business command.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const [binary,root,work,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
let browser;const server=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
(async()=>{
 const url=await new Promise((resolve,reject)=>{let s='';server.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.once('exit',c=>reject(Error('Server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();await page.setViewport({width:1440,height:1000});await page.goto(url);await page.waitForSelector('#pilot-view');
 await page.select('#work',work);await page.waitForFunction(id=>snapshot?.work.id===id,{},work);
 await page.evaluate(()=>localStorage.setItem(Pilot.key,JSON.stringify({version:1,view:'agents',filter:'finished',collapsed:snapshot.work.tasks.map(t=>t.id)})));
 await page.reload();await page.waitForSelector('#pilot-view');assert.equal(await page.$eval('#pilot-view',e=>e.value),'dependencies');
 assert.equal(await page.$eval('#pilot-filter',e=>e.value),'all');
 const expected=await page.evaluate(()=>snapshot.work.tasks.reduce((n,t)=>n+(t.depends||[]).length,0));assert.ok(expected>0);
 assert.equal(await page.$$eval('.graph-arete',es=>es.length),expected);
 assert.equal(await page.$eval('#pilot-list',e=>getComputedStyle(e).display),'none');
 const fit=await page.$eval('#pilot-canvas',e=>{const s=e.querySelector('svg'),r=s.getBoundingClientRect();return {svgWidth:r.width,canvasWidth:e.clientWidth,svgHeight:r.height,canvasHeight:e.clientHeight}});assert.ok(fit.svgWidth<=fit.canvasWidth&&fit.svgHeight<=fit.canvasHeight);
 assert.equal(await page.$eval('#conduite',e=>e.firstElementChild.className),'conduite-principal');
 assert.ok(await page.evaluate(()=>{const m=document.querySelector('#pilot-canvas marker');return Math.abs(Number(m.getAttribute('markerWidth'))*Pilot.state.zoom-9)<.01}));
 await page.click('#pilot-collapse');assert.equal(await page.$$eval('.graph-arete',es=>es.length),0);
 await page.click('#pilot-all-links');assert.equal(await page.$$eval('.graph-arete',es=>es.length),expected);
 const panels=['assistant','conduite-overview','conduite-inbox-section','fil-bloc'];
 for(const id of panels){
  assert.equal(await page.$eval('#'+id,e=>e.open),false);
  await page.focus('#'+id+'>summary');await page.keyboard.press('Enter');assert.equal(await page.$eval('#'+id,e=>e.open),true);
  await page.evaluate(()=>refresh(true));assert.equal(await page.$eval('#'+id,e=>e.open),true);
  await page.focus('#'+id+'>summary');await page.keyboard.press('Enter');assert.equal(await page.$eval('#'+id,e=>e.open),false);
 }
 assert.equal(await page.$eval('#conduite-graph-title',e=>e.textContent),'Pilotage des agents');
 const title=await page.title();
 for(const theme of ['etat','sombre']){
  await page.evaluate(t=>{setTheme(t);window.scrollTo(0,0)},theme);await page.screenshot({path:path.join(out,'accordeons-'+theme+'.png'),fullPage:true});await page.$eval('#graph',e=>e.scrollIntoView({block:'start'}));
  await page.screenshot({path:path.join(out,'graphe-reel-'+theme+'.png')});
 }
 const result={status:'PASS',work,title,edges:expected,nodes:await page.$$eval('.graph-noeud',es=>es.length),checks:['Anciennes préférences cartes migrées vers graphe complet','Flèches du travail réel présentes','Liste réellement invisible en CSS','Deux thèmes rendus','Pilotage en premier ; flèches de 9px au zoom courant ; restauration de toutes les dépendances','Accordéons fermés par défaut, ouverture clavier stable au refresh, graphe hors accordéon'],no_business_command:true};
 fs.writeFileSync(path.join(out,'real-graph.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
