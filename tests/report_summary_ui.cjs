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

 let asks=0,mode='success';
 await page.setRequestInterception(true);
 page.on('request',req=>{
  const u=new URL(req.url());
  if(u.pathname==='/api/v1/assist/meta')return req.respond({contentType:'application/json',body:JSON.stringify({provider_support:{fixture:''},pages:[],templates:[]})});
  if(u.pathname==='/api/v1/assist/turns')return req.respond({contentType:'application/json',body:'{"turns":[]}'});
  if(u.pathname==='/api/v1/assist/ask'){
   const body=JSON.parse(req.postData());
   assert.equal(body.request.coordinates.report,'docs/fixture.md');assert.equal(body.request.template_id,'report_summary.v1');
   if(body.preview)return req.respond({contentType:'application/json',body:JSON.stringify({turn:{context:{context_hash:'fixture'}}})});
   asks++;
   if(mode==='error')return req.respond({status:503,contentType:'application/json',body:'{"error":"Fournisseur indisponible"}'});
   return req.respond({contentType:'application/json',body:JSON.stringify({id:'fixture',status:'completed',provider:'fixture',context:{context_hash:'fixture'},answer:{interpretation:'Le rapport annonce la réparation du graphe.\nIl reste à vérifier le résultat avant validation.'}})});
  }
  req.continue();
 });
 const open=()=>page.evaluate(()=>{
  assistMeta={provider_support:{fixture:''}};
  openModal('Conclusions du rapport','Rapport de recette.',{action:'help'});$('confirm').hidden=true;preview('Contenu intégral du rapport de recette.');
  mountReportSummary('docs/fixture.md',snapshot.work.tasks[0].id);
 });
 await open();await page.waitForFunction(()=>document.querySelector('#report-summary').textContent.includes('Il reste à vérifier'));
 assert.equal(asks,1);assert.equal(await page.$$eval('#report-summary>div[aria-live] p',es=>es.length),2);
 for(const theme of ['etat','sombre']){
  await page.evaluate(t=>setTheme(t),theme);
  await page.focus('#report-summary button');
  await page.screenshot({path:path.join(out,'rapport-'+theme+'.png')});
 }
 await page.keyboard.press('Escape');assert.equal(await page.$eval('#modal',e=>e.open),false);
 mode='error';await open();await page.waitForFunction(()=>document.querySelector('#report-summary').textContent.includes('Analyse indisponible'));
 assert.equal(await page.$eval('#preview',e=>e.hidden),false);
 console.log('PASS: lancement automatique, deux lignes, sélection rapport, deux thèmes et focus, fermeture Échap, échec IA avec rapport accessible. Réponses IA simulées ; aucun fournisseur appelé.');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
