'use strict';
// Explicitly requested live cancellation of the named stuck review; no retry launch.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const [binary,root,work,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
let browser;const server=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
(async()=>{
 const url=await new Promise((resolve,reject)=>{let s='';server.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.once('exit',c=>reject(Error('Server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();await page.setViewport({width:1440,height:1000});await page.goto(url);await page.waitForSelector('#pilot-view');
 await page.select('#work',work);await page.waitForFunction(id=>snapshot?.work.id===id,{},work);

 const expected='review-2301b0fbbf1842f7b3aa5937fc20efa8';
 await page.evaluate(()=>Pilot.inspect('task','review-codex'));await page.waitForSelector('[data-recovery-action=reconcile]');
 assert.equal(await page.evaluate(()=>snapshot.agents.find(x=>x.agent.task_id==='review-codex').agent.id),expected);
 assert.equal(await page.evaluate(id=>snapshot.pilotage.health[id].can_cancel_pending,expected),true);
 await page.click('[data-recovery-action=reconcile]');await page.waitForFunction(()=>modalContext?.cancelPending);
 assert.equal(await page.$eval('#field-agent',e=>e.value),expected);
 await page.click('#confirm');
 await page.waitForFunction(()=>snapshot.work.tasks.find(t=>t.id==='review-codex').status==='blocked',{timeout:15000});
 assert.equal(await page.evaluate(id=>snapshot.agents.find(x=>x.agent.id===id).agent.status,expected),'interrupted');
 await page.reload();await page.waitForSelector('[data-recovery-action=retry]');
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.$eval('#pilot-inspector',e=>e.scrollTop=0);await page.screenshot({path:path.join(out,'annulation-reelle-'+theme+'.png')})}
 console.log('PASS : tentative Codex ciblée annulée, tâche à reprendre, relance visible après rechargement. Aucun fournisseur relancé.');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
