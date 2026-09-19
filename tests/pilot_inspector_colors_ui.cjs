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

 await page.evaluate(()=>{Pilot.state.view='dependencies';Pilot.state.search='';Pilot.state.filter='all';Pilot.changed();Pilot.inspect('task','review-codex')});
 await page.waitForSelector('#pilot-inspector[open] .pilot-mission');
 assert.equal(await page.$$eval('.graph-arete',es=>es.length),0);
 assert.match(await page.$eval('#pilot-status',e=>e.textContent),/aucune dépendance déclarée/);
 assert.match(await page.$eval('.pilot-mission',e=>e.textContent),/Contrats du moteur/);
 for(const theme of ['etat','sombre']){
  await page.evaluate(t=>setTheme(t),theme);
  const colors=await page.$eval('.pilot-mission',e=>({bg:getComputedStyle(e).backgroundColor,ink:getComputedStyle(e).color,parent:getComputedStyle(e.parentElement).backgroundColor}));
  assert.notEqual(colors.bg,colors.ink);
  const unknown=await page.evaluate(()=>{const style=getComputedStyle(document.documentElement);return [...new Set([...document.styleSheets].flatMap(sheet=>{try{return [...sheet.cssRules].flatMap(rule=>[...rule.cssText.matchAll(/var\((--wattson-[\w-]+)/g)].map(m=>m[1]))}catch{return []}}))].filter(t=>!style.getPropertyValue(t).trim())});assert.deepEqual(unknown,[]);
  await page.focus('#pilot-inspector button');await page.screenshot({path:path.join(out,'codex-'+theme+'.png')});
 }
 await page.evaluate(()=>refresh(true));assert.equal(await page.$$eval('#pilot-inspector .pilot-mission',es=>es.length),1);
 await page.keyboard.press('Escape');assert.equal(await page.$eval('#pilot-inspector',e=>e.open),false);
 console.log('PASS: revue réelle sans dépendances, mission Codex colorée, thèmes résolus, focus, refresh et Échap.');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
