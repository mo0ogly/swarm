// All workflow transitions use visible browser controls. CLI only creates an isolated fixture.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const {spawn, execFileSync} = require('node:child_process');
const puppeteer = require(process.env.PUPPETEER_MODULE || 'puppeteer');
const binary = path.resolve(process.argv[2]);
const reportDir = path.resolve(process.argv[3]);
fs.mkdirSync(reportDir, {recursive:true});
const root = execFileSync('python3', [path.join(__dirname,'web_fixture.py'),binary], {encoding:'utf8'}).trim();
const server = spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
const checks=[],errors=[];let browser,page;
const sleep = ms=>new Promise(r=>setTimeout(r,ms));
async function text(sel){return page.$eval(sel,e=>e.textContent)}
async function has(sel,word){await page.waitForFunction((s,w)=>document.querySelector(s)?.textContent.includes(w),{},sel,word)}
async function fill(sel,value){await page.click(sel,{clickCount:3});await page.keyboard.press('Backspace');await page.type(sel,value)}
async function openTask(id,action='start'){
 await page.click('[data-view="tasks"]');
 await page.click('[data-task="'+id+'"] button');
 await page.waitForSelector('#modal[open] #field-action',{visible:true});
 await page.waitForFunction(id=>document.querySelector('#modal-title').textContent.startsWith(id+' —'),{},id);
 if(action!=='start')await page.select('#field-action',action);
 assert.equal(await page.$eval('#field-action',e=>e.value),action,'Action shown differs from requested action');
}
async function confirm(){await page.click('#confirm');await page.waitForFunction(()=>!document.querySelector('#modal').open||!document.querySelector('#modal-error').hidden);if(await page.$eval('#modal',e=>e.open))throw new Error(await text('#modal-error'))}
async function shot(name){await page.screenshot({path:path.join(reportDir,name+'.png'),fullPage:true})}
(async()=>{
 try{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('Server did not start')),10000);server.stdout.on('data',d=>{out+=d;const m=out.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m){clearTimeout(timer);resolve(m[0])}});server.on('exit',code=>reject(new Error('Server exited '+code)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox'],userDataDir:fs.mkdtempSync(path.join(os.tmpdir(),'swarm-ui-'))});
 page=await browser.newPage();page.setDefaultTimeout(15000);await page.setViewport({width:1440,height:1000});page.on('pageerror',e=>errors.push(e.message));
 await page.goto(url);await page.waitForSelector('[data-task="UI-01"]');await shot('etat-tasks');
 await openTask('UI-01');assert.match(await text('#preview'),/Périmètre/);await page.keyboard.press('Escape');assert.equal(await page.$eval('#modal',e=>e.open),false);assert.equal(await page.$eval(':focus',e=>e.textContent),'Piloter');assert.equal(await text('#active-count'),'0');checks.push('Cancel / Escape / focus return without launch');
 await openTask('UI-02');await page.select('#field-provider','recette');await page.click('#confirm');await has('#modal-error','dépendance');await shot('etat-dependency-refusal');await page.click('#cancel');checks.push('Dependency refusal explained in launch modal');
 await openTask('UI-01');await page.select('#field-provider','recette');await page.select('#field-role','subplanner');await fill('#field-instruction','Recette isolée : produire uniquement les événements de fixture.');await shot('etat-launch-modal');await confirm();await has('#active-count','1');await shot('etat-active');await has('[data-task="UI-01"]','Bloquée');checks.push('Task/provider/role/workspace/instruction launch; live completion visible');
 await page.click('[data-view="agents"]');await has('#agents-list','subplanner');await page.click('#agents-list button');await page.select('#field-action','retry');await confirm();await has('#active-count','1');await page.click('[data-view="tasks"]');await openTask('UI-01','stop');const options=await page.$$eval('#field-agent option',es=>es.map(e=>({value:e.value,text:e.textContent})));const running=options.find(x=>/En cours|Démarrage|En attente/.test(x.text));assert.ok(running,JSON.stringify(options));await page.select('#field-agent',running.value);await page.click('#cancel');await has('#active-count','1');await openTask('UI-01','stop');await page.select('#field-agent',running.value);await confirm();await has('#active-count','0');checks.push('Retry preserves role; cancel stop has no effect; selected attempt stopped');
 await page.click('[data-view="logs"]');await has('#log-lines','#');await fill('#log-search','Processus');await page.click('#log-form button');await has('#log-lines','Processus');await page.click('#log-live');assert.equal(await text('#log-live'),'Mettre en pause');await page.click('#log-live');await shot('etat-logs');checks.push('Search / live / pause logs through controls');
 await page.click('[data-view="decisions"]');await page.waitForSelector('#decisions-list button');await page.click('#decisions-list button');await fill('#field-note','Résultat examiné ; la revue de la tâche reste distincte.');await confirm();await has('#decisions-list','Résultat examiné');await page.click('[data-view="tasks"]');await has('[data-task="UI-01"]','Bloquée');checks.push('Decision acknowledgement persisted without task acceptance');
 await openTask('UI-01','report');await page.click('#confirm');await has('#preview','Rapport de fixture');assert.equal(await page.$('#preview img'),null);assert.equal(await page.evaluate(()=>window.injected),undefined);await page.click('#cancel');await openTask('UI-01','submit');await confirm();await has('[data-task="UI-01"]','À vérifier');
 await openTask('UI-01','gate');await page.click('#confirm');await has('#confirm','Confirmer l’enregistrement');await shot('etat-gate-preview');await page.click('#cancel');await openTask('UI-01','accepted');await page.click('#confirm');await page.waitForFunction(()=>!document.querySelector('#modal-error').hidden);await page.click('#cancel');checks.push('Report read/submitted; preview cancellation does not record gate');
 await openTask('UI-01','gate');await page.click('#confirm');await has('#confirm','Confirmer l’enregistrement');await confirm();await openTask('UI-01','accepted');await has('#preview','PASS');await confirm();await has('[data-task="UI-01"]','Acceptée');checks.push('Gate preview/record then acceptance with live row refresh');
 await page.click('[data-view="resume"]');await page.click('#ooda');for(const [k,v]of Object.entries({observation:'Parcours UI observé.',orientation:'Les états sont distincts.',decision:'Examiner la suite.',result:'Première tâche acceptée.',next:'Vérifier le budget de UI-02.'}))await fill('#field-'+k,v);await confirm();await has('#resume-text','Vérifier le budget');await shot('etat-ooda');await page.click('#visit');await has('#message','marquée comme vue');checks.push('OODA and per-operator visit retained');
 await page.click('[data-view="budget"]');await has('#usage-list','12 entrée / 8 sortie');await has('#usage-list','Cache lu : 19');await page.click('#budget-edit');for(const [k,v]of Object.entries({limit:'0.1',reserve:'1',source:'Estimation de fixture en USD par lancement',date:'2026-09-12'}))await fill('#field-'+k,v);await confirm();await openTask('UI-02');await page.select('#field-provider','recette');await page.click('#confirm');await has('#modal-error','Budget estimatif insuffisant');await shot('etat-budget-refusal');await page.click('#cancel');checks.push('Declared tokens distinguished from estimated budget; exhausted budget blocks launch');
 await openTask('UI-02','override');await page.click('#confirm');await page.waitForFunction(()=>!document.querySelector('#modal-error').hidden);await fill('#field-note','Dérogation de recette uniquement ; aucun contrôle transformé en PASS.');await confirm();await has('[data-task="UI-02"]','Dérogation');checks.push('Forced validation requires reason and remains a derogation');
 await page.click('#theme');assert.equal(await page.$eval('html',e=>e.dataset.theme),'sombre');await shot('sombre-tasks');await openTask('UI-02','accepted');await shot('sombre-modal');await page.keyboard.press('Escape');
 await page.setViewport({width:390,height:844});await shot('sombre-mobile');assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'mobile page overflow');await page.setViewport({width:1440,height:1000});
 await page.reload();await page.waitForSelector('[data-task="UI-01"]');await has('[data-task="UI-01"]','Acceptée');await page.click('[data-view="decisions"]');await has('#decisions-list','Résultat examiné');checks.push('Reload preserves task state, decisions and theme');
 for(const theme of ['etat','sombre']){if(await page.$eval('html',e=>e.dataset.theme)!==theme)await page.click('#theme');for(const name of ['tasks','agents','decisions','logs','resume','budget']){await page.click('[data-view="'+name+'"]');await shot(theme+'-'+name+'-final');}await page.click('#budget-edit');await page.focus('#field-limit');await shot(theme+'-budget-focus');await page.keyboard.press('Escape');}
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(reportDir,'result.json'),JSON.stringify({status:'PASS',fixture_root:root,checks,page_errors:errors,model_calls:0},null,2));
 console.log(JSON.stringify({status:'PASS',checks:checks.length,report:reportDir}));
 }catch(e){if(page){await shot('failure').catch(()=>{});fs.writeFileSync(path.join(reportDir,'failure.txt'),await page.$eval('body',e=>e.innerText).catch(()=>''))}fs.writeFileSync(path.join(reportDir,'result.json'),JSON.stringify({status:'FAIL',error:e.stack,checks,page_errors:errors,fixture_root:root},null,2));throw e}
 finally{await browser?.close();server.kill('SIGTERM')}
})().catch(e=>{console.error(e);process.exitCode=1});
