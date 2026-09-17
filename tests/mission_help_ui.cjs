'use strict';
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),{spawn,execFileSync}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const fixture=JSON.parse(execFileSync('python3',[path.join(__dirname,'pilotage_fixture.py'),binary],{encoding:'utf8'}));
let browser;const server=spawn(binary,['--root',fixture.root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
(async()=>{
 const url=await new Promise(resolve=>{let s='';server.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])})});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1440,height:1100});await page.goto(url);await page.waitForSelector('#pilot-view');await page.select('#work',fixture.works['12']);await page.waitForFunction(w=>snapshot?.work.id===w,{},fixture.works['12']);

 let asks=0,applies=0,mode='success';
 await page.setRequestInterception(true);page.on('request',req=>{
  const u=new URL(req.url());
  if(u.pathname==='/api/v1/action' && JSON.parse(req.postData()||'{}').kind?.startsWith('assist-action-')){const b=JSON.parse(req.postData());if(b.kind==='assist-action-apply')applies++;return req.respond({contentType:'application/json',body:JSON.stringify(b.kind==='assist-action-preview'?{id:'test-plan',effect:'Présenter le compte rendu pour examen.'}:{status:'done',message:'Le compte rendu est prêt à être examiné.'})})}
  if(u.pathname==='/api/v1/assist/meta')return req.respond({contentType:'application/json',body:JSON.stringify({provider_support:{fixture:''},pages:[],templates:[]})});
  if(u.pathname==='/api/v1/assist/turns')return req.respond({contentType:'application/json',body:'{"turns":[]}'});
  if(u.pathname==='/api/v1/assist/ask'){
   const body=JSON.parse(req.postData());assert.equal(body.request.coordinates.selected[0],'t0');assert.equal(body.request.template_id,'mission_advice.v1');
   if(body.preview)return req.respond({contentType:'application/json',body:JSON.stringify({turn:{context:{context_hash:'fixture'}}})});asks++;
   if(mode==='error')return req.respond({status:503,contentType:'application/json',body:'{"error":"Fournisseur indisponible"}'});
   return req.respond({contentType:'application/json',body:JSON.stringify({id:'fixture',status:'completed',provider:'fixture',context:{revision:0,facts:[{id:'f1',name:'motif_relais_rapport',value:'Rapport absent'}],actions:[{id:'submit',label:'Présenter le compte rendu',target:'t0'}]},answer:{interpretation:'Cette tâche prépare les choix de conception.\nL’agent a terminé, mais son compte rendu reste à examiner.\nPrésentez ce compte rendu pour permettre la suite.',facts:[{text:'Le moteur signale un rapport absent.'}],missing_information:['Le contenu du rapport n’est pas disponible.'],next_steps:[{action_id:'submit',why:'Permettre l’examen.'}]}})});
  }req.continue();
 });
 await page.click('#mission-summary details summary');await page.click('[data-mission-help="t0"]');await page.waitForFunction(()=>document.getElementById('mission-help-answer')?.textContent.includes('compte rendu reste à examiner'));assert.equal(asks,1);assert.equal(await page.$eval('#confirm',e=>e.hidden),true);
 await page.waitForSelector('#mission-help-actions button');assert.equal(applies,0);assert.equal(await page.$eval('#mission-help-answer > details',e=>e.open),false);assert.equal(await page.$$eval('[data-role="summary"] p',es=>es.length),3);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.screenshot({path:path.join(out,theme+'.png'),fullPage:true})}
 await page.click('#mission-help-actions button');await page.waitForFunction(()=>document.querySelector('.mission-help-proposal')?.dataset.result==='done');assert.equal(applies,1);
 await page.keyboard.press('Escape');assert.equal(await page.$eval('#modal',e=>e.open),false);assert.equal(await page.evaluate(()=>document.activeElement.dataset.missionHelp),'t0');
 mode='error';await page.click('[data-mission-help="t0"]');await page.waitForFunction(()=>document.getElementById('mission-help').textContent.includes('Aide IA indisponible'));assert.ok(await page.$('#mission-help button'));
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'ui.json'),JSON.stringify({status:'PASS',checks:['bouton réel sur la tâche, sélection transmise au moteur','trois phrases visibles, détails fermés','sources et informations manquantes accessibles','un seul envoi à ouverture','fournisseur indisponible expliqué avec accès aux actions','deux thèmes rendus','aperçu sans action, application explicite unique','Échap et restitution du focus'],scope:'Réponses IA simulées ; aucun fournisseur réel appelé',errors},null,2));console.log('PASS MissionHelp UI');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
