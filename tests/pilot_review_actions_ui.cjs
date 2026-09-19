'use strict';
// Read-only recipe against a named existing work; no agent or business command.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const [binary,root,work,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
let browser;const server=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
(async()=>{
 const url=await new Promise((resolve,reject)=>{let s='';server.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.once('exit',c=>reject(Error('Server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();await page.setViewport({width:1440,height:1000});await page.goto(url);await page.waitForSelector('#pilot-view');
 await page.select('#work',work);await page.waitForFunction(id=>snapshot?.work.id===id,{},work);

 let mutations=0;page.on('request',req=>{if(req.method()==='POST'&&new URL(req.url()).pathname==='/api/v1/action')mutations++});
 for(const task of ['review-claude-host','review-codex-host']){
  await page.evaluate(id=>Pilot.inspect('task',id),task);
  await page.waitForFunction(id=>document.querySelector('.pilot-review-controls [aria-describedby]')?.getAttribute('aria-describedby').includes(id),{},task);
  assert.equal(await page.$eval('.pilot-review-controls [data-review-action=accepted]',e=>e.disabled),true);
  for(const kind of ['report','gate','override'])assert.equal(await page.$eval('.pilot-review-controls [data-review-action='+kind+']',e=>e.disabled),false);
  for(const theme of ['etat','sombre']){
   await page.evaluate(t=>setTheme(t),theme);
   await page.$eval('#pilot-inspector',e=>e.scrollTop=0);
   await page.screenshot({path:path.join(out,task+'-'+theme+'.png')});
   await page.click('.pilot-review-controls [data-review-action=override]');
   await page.waitForFunction(()=>modalContext?.action==='override');
   assert.equal(await page.$eval('#confirm',e=>e.textContent),'Confirmer la dérogation');
   assert.ok(await page.$('#field-note'));
   await page.focus('#field-note');await page.screenshot({path:path.join(out,task+'-decision-'+theme+'.png')});
   await page.keyboard.press('Escape');
  }
  await page.click('.pilot-review-controls [data-review-action=gate]');await page.waitForFunction(()=>modalContext?.action==='gate');await page.keyboard.press('Escape');
 }
 assert.equal(mutations,0);
 console.log('PASS: tâches 3 et 4, boutons visibles, validation désactivée avec motif, dérogation et preuves accessibles, confirmations explicites, deux thèmes ; aucune décision enregistrée.');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
