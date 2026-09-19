'use strict';
// Live IA recipe: one question recorded in assistant history; provider consumption applies.
// Opt-in invocation only. Does not accept or alter a task.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const [binary,root,work,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
let browser;const server=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
(async()=>{
 const url=await new Promise((resolve,reject)=>{let s='';server.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.once('exit',c=>reject(Error('Server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();await page.setViewport({width:1440,height:1000});await page.goto(url);await page.waitForSelector('#pilot-view');
 await page.select('#work',work);await page.waitForFunction(id=>snapshot?.work.id===id,{},work);

 const selected=await page.evaluate(async()=>{
  for(const task of snapshot.work.tasks){
   const data=await api('/api/v1/task?'+new URLSearchParams({work,task:task.id}));
   if(data.reports?.length){Pilot.state.selection={kind:'task',id:task.id};await PilotInspector.readReport(data.reports[0],task.id);return {task:task.id,report:data.reports[0]}}
  }
  throw Error('Aucun rapport détecté');
 });
 console.log('Rapport sélectionné : '+JSON.stringify(selected));
 await page.waitForFunction(()=>{const e=document.querySelector('#report-summary');return e&&(e.querySelectorAll('[aria-live] p').length===2||e.textContent.includes('indisponible')||e.textContent.includes('Aucune IA'))},{timeout:320000});
 const result=await page.$eval('#report-summary',e=>({state:e.querySelector('[role=status]').textContent,lines:[...e.querySelectorAll('[aria-live] p')].map(p=>p.textContent)}));
 fs.writeFileSync(path.join(out,'live-result.json'),JSON.stringify({...selected,...result},null,2));
 console.log(JSON.stringify(result));
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.screenshot({path:path.join(out,'live-'+theme+'.png')})}
 assert.equal(result.lines.length,2);
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
