'use strict';
// T7 — REQ-SUM-01 (pre-launch summary: roles/models/effective limits/missing
// prerequisites) and REQ-DIAG-01 (copy diagnostic: cause/attempt/version/next
// action, no secret, no session URL). Reuses the existing web-only preparation
// journey (tests/journey_ui.cjs) and the existing diagnostic fixture
// (tests/q3_diagnostic_ui.cjs) rather than inventing new setup mechanisms.
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {spawn,spawnSync,execFileSync}=require('node:child_process');
let puppeteer;try{puppeteer=require('puppeteer')}catch(e){console.error('prelaunch_diagnostic dependency missing: puppeteer');process.exit(3)}
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
if(!fs.existsSync(binary)){console.error('prelaunch_diagnostic dependency missing: built swarm binary');process.exit(3)}
const checks=[],errors=[];
// A modal makes the theme button inert. Temporarily close/reopen the same
// dialog (without reinitializing form state), then assert the actual theme.
async function captureTheme(page, theme, filename, selector) {
 const dialogs = await page.$$eval('dialog[open]', items => items.map(e=>e.id));
 await page.evaluate(ids => [...ids].reverse().forEach(id=>document.getElementById(id).close()), dialogs);
 if (await page.$eval('html', e => e.dataset.theme) !== theme) await page.click('#theme');
 await page.waitForFunction(t => document.documentElement.dataset.theme === t, {}, theme);
 await page.evaluate(ids => ids.forEach(id=>document.getElementById(id).showModal()), dialogs);
 const target = await page.$(selector);
 await target.evaluate(e => e.scrollIntoView({block:'center'}));
 await target.screenshot({path:path.join(out, filename)});
}


async function partSummary(browser){
 const project=fs.mkdtempSync(path.join(out,'summary-project-'));
 execFileSync(binary,['--root',project,'init'],{encoding:'utf8'});
 for(const name of ['.claude/skills/apex/SKILL.md','tools/agent-workflows/CONTRACT.md']){const target=path.join(project,name);fs.mkdirSync(path.dirname(target),{recursive:true});fs.writeFileSync(target,'Méthode de recette isolée.\n')}
 const provider=path.join(project,'claude');fs.writeFileSync(provider,"#!/bin/sh\nif [ \"$1\" = \"--swarm-preflight\" ]; then printf '%s\\n' '{\"schema_version\":1,\"capabilities\":{\"process\":\"verified\",\"workspace_read\":\"verified\",\"workspace_write\":\"verified\"}}'; exit 0; fi\ncat >/dev/null\nprintf '%s\\n' '{\"type\":\"result\",\"result\":\"{}\"}'\n",{mode:0o700});
 fs.writeFileSync(path.join(project,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{recette:{command:provider,preflight_required:true,preflight_args:['--swarm-preflight'],preflight_kind:'provider-context',preflight_capabilities:['process','workspace_read','workspace_write']}}}));
 const plan=JSON.stringify({version:1,objective:'Créer un livrable vérifié',assumptions:[],questions:[],tasks:[{id:'T1',title:'Produire le livrable',role:'worker',scope:'docs',deliverable:'docs/resultat.md',depends:[],criteria:['Le livrable existe et est relu'],proof:'Commande de contrôle et rapport',entry:'Brief adopté',validation:'Exécuter le contrôle annoncé',delivery:'Livrable et preuve fraîche',stop:'Arrêter après deux échecs identiques',max_attempts:2,max_tool_calls:20}]},null,2);
 const server=spawn(binary,['--root',project,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
 async function fill(page,selector,value){await page.click(selector,{clickCount:3});await page.keyboard.press('Backspace');await page.type(selector,value)}
 try{
  const url=await new Promise((resolve,reject)=>{let output='';const timer=setTimeout(()=>reject(Error('summary web server did not start')),15000);server.stdout.on('data',data=>{output+=data;const match=output.match(/http:\/\/\S+\/session\/\S+/);if(match){clearTimeout(timer);resolve(match[0])}});server.on('exit',code=>reject(Error('summary web server exited '+code)))});
  const page=await browser.newPage();page.setDefaultTimeout(15000);await page.setViewport({width:1280,height:900});page.on('pageerror',e=>errors.push(e.message));
  await page.goto(url,{waitUntil:'domcontentloaded'});
  await page.goto(new URL('/prepare.html',url).href,{waitUntil:'domcontentloaded'});
  await page.waitForSelector('#create-form:not([hidden])');await fill(page,'#new-title','Résumé de lancement');await fill(page,'#new-need','Je veux un livrable documenté, contrôlé et traçable.');await page.click('#create');await page.waitForSelector('#documents:not([hidden])');
  async function waitAction(value){await page.waitForFunction(v=>document.getElementById('advance')?.dataset.action===v,{},value)}
  await page.click('#advance');await page.waitForFunction(()=>document.getElementById('tab-brief').getAttribute('aria-selected')==='true');if(await page.$eval('#plain-editor',e=>e.hidden))await page.click('#editor-toggle');await fill(page,'#plain-editor','# Brief\n\nObjectif : produire un livrable contrôlé.');await waitAction('save');await page.click('#advance');await waitAction('adopt');await page.click('#advance');await waitAction('plan');await page.click('#advance');await page.waitForFunction(()=>document.getElementById('tab-plan').getAttribute('aria-selected')==='true');await fill(page,'#plain-editor',plan);await waitAction('save');await page.click('#advance');await waitAction('validate');await page.click('#advance');await waitAction('convert');
  await page.click('#advance');await page.waitForSelector('#conversion-dialog[open]');

  // REQ-SUM-01 — before any choice: provider, validation and preflight missing.
  let limits=await page.$eval('#summary-limits',e=>e.textContent);
  assert.match(limits,/Tâches \(plan\) : 20 max/);assert.match(limits,/Appels par rôle \(responsable\/vérificateur\) : 40 max/);
  let prereq=await page.$eval('#summary-prerequisites',e=>e.textContent);
  assert.match(prereq,/IA à choisir/);assert.match(prereq,/Validation à choisir/);assert.match(prereq,/Prévol non exécuté/);
  assert.doesNotMatch(prereq,/Dossier de travail à indiquer/,'default workspace "." must not be reported as missing');
  checks.push('Résumé initial : limites par défaut et 3 prérequis manquants affichés (REQ-SUM-01)');

  // Effective limits reflect the live caps before any launch.
  await fill(page,'#organization-tasks','15');await fill(page,'#organization-calls','25');
  await page.waitForFunction(()=>document.getElementById('summary-limits').textContent.includes('15 max'));
  limits=await page.$eval('#summary-limits',e=>e.textContent);assert.match(limits,/Tâches \(plan\) : 15 max · Appels par rôle \(responsable\/vérificateur\) : 25 max/);
  checks.push('Limites effectives suivent les plafonds saisis sans relance manuelle (REQ-SUM-01)');

  await page.select('#organization-provider','recette');await page.select('#organization-validation','human');await page.waitForFunction(()=>document.getElementById('organization-model').textContent.includes('Modèles résolus'));
  assert.match(await page.$eval('#summary-owner',e=>e.textContent),/recette/);assert.match(await page.$eval('#summary-workers',e=>e.textContent),/1/);assert.match(await page.$eval('#summary-review',e=>e.textContent),/recette/);
  prereq=await page.$eval('#summary-prerequisites',e=>e.textContent);assert.doesNotMatch(prereq,/IA à choisir/);assert.doesNotMatch(prereq,/Validation à choisir/);assert.match(prereq,/Prévol non exécuté/);
  checks.push('Rôles et modèles effectifs résolus ; prérequis restant = prévol uniquement (REQ-SUM-01)');
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'summary-missing-'+theme+'.png','#organization-summary');

  await page.click('#organization-preflight');await page.waitForFunction(()=>document.getElementById('organization-preflight-status').classList.contains('success'));
  prereq=await page.$eval('#summary-prerequisites',e=>e.textContent);assert.equal(prereq,'Aucun prérequis manquant — prêt à autoriser.');
  checks.push('Prérequis manquants vides après prévol exploitable (REQ-SUM-01)');
  assert.equal(await page.$eval('#organization-preflight-status',e=>e.textContent),'Conditions vérifiées. Vous pouvez autoriser cette équipe.');
  assert.doesNotMatch(await page.$eval('#organization-preflight-status',e=>e.textContent),/ready|verified|octets/);
  await page.click('#organization-preflight-details');
  await page.waitForSelector('#preflight-details-dialog[open]');
  assert.match(await page.$eval('#preflight-details-content',e=>e.textContent),/ready|compatible/);
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'preflight-details-fr-'+theme+'.png','#preflight-details-dialog');
  // Native Escape returns to the form and restores focus to the trigger.
  await page.keyboard.press('Escape');
  await page.waitForFunction(()=>!document.getElementById('preflight-details-dialog').open&&document.activeElement.id==='organization-preflight-details');
  await page.select('#organization-validation','automatic');
  assert.match(await page.$eval('#summary-prerequisites',e=>e.textContent),/Précisez les arguments/);
  await page.select('#organization-validation','human');
  checks.push('Détails techniques dans une modale, Échap/focus corrects ; contrôles manquants signalés avant autorisation');

  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'summary-ready-'+theme+'.png','#organization-summary');

  // FR/EN — labels translated, no rebuild of the screen.
  await page.keyboard.press('Escape');
  await Promise.all([page.waitForNavigation(),page.select('#swarm-language','en')]);
  await page.waitForFunction(()=>document.documentElement.lang==='en');
  await waitAction('convert');await page.click('#advance');await page.waitForSelector('#conversion-dialog[open]');
  limits=await page.$eval('#summary-limits',e=>e.textContent);assert.match(limits,/Tasks \(plan\): 20 max/);assert.match(limits,/Calls per role \(owner\/reviewer\): 40 max/);
  prereq=await page.$eval('#summary-prerequisites',e=>e.textContent);assert.match(prereq,/AI to choose|Validation to choose|Preflight not run/);
  checks.push('Textes du résumé traduits en anglais sans changer la structure de l’écran (req-22)');
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'summary-en-'+theme+'.png','#organization-summary');
  await page.select('#organization-provider','recette');await page.select('#organization-validation','human');
  await page.waitForFunction(()=>document.getElementById('organization-model').textContent.includes('Models resolved'));
  await page.click('#organization-preflight');
  await page.waitForFunction(()=>document.getElementById('organization-preflight-status').classList.contains('success'));
  assert.equal(await page.$eval('#organization-preflight-status',e=>e.textContent),'Conditions checked. You can authorize this team.');
  await page.click('#organization-preflight-details');
  assert.equal(await page.$eval('#preflight-details-title',e=>e.textContent),'Pre-launch check details');
  assert.match(await page.$eval('#preflight-details-content',e=>e.textContent),/executable and limits valid/);
  assert.doesNotMatch(await page.$eval('#preflight-details-content',e=>e.textContent),/octets disponibles|dossier lisible|sonde configurée/);
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'preflight-details-en-'+theme+'.png','#preflight-details-dialog');
  await page.click('[data-close="preflight-details-dialog"]');
  await page.waitForFunction(()=>document.activeElement.id==='organization-preflight-details');
  checks.push('English preflight: short explanation, translated details dialog and focus restored');

  await page.close();
 }finally{server.kill('SIGTERM')}
}

async function partDiagnostic(browser){
 const root=fs.mkdtempSync(path.join(out,'diagnostic-project-'));
 const run=(args,input)=>{const r=spawnSync(binary,['--root',root,...args],{encoding:'utf8',input,timeout:10000});if(r.status!==0||r.signal||r.error)throw Object.assign(r.error||Error(r.stderr),{result:r});return r.stdout};
 const json=(args,input)=>JSON.parse(run(['--json',...args,...(input!==undefined?['--input','-']:[])],input===undefined?undefined:JSON.stringify(input)));
 const id=()=>require('node:crypto').randomUUID().replaceAll('-','');
 json(['init']);
 const provider=path.join(root,'q7-provider.py');
 fs.writeFileSync(provider,`import json,sys,time
sys.stdin.read()
events=[
 ("c1","missing-tool --version","command not found"),
 ("c2","mount /mnt/partage","operation not permitted by sandbox mount"),
 ("c3","go test ./...","exit code 1")]
for ident,command,error in events:
 print(json.dumps({"type":"item.started","item":{"id":ident,"type":"command_execution","command":command}}),flush=True)
 print(json.dumps({"type":"item.completed","item":{"id":ident,"type":"command_execution","status":"failed","exit_code":1,"error":error}}),flush=True)
time.sleep(2)
`);
 fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{recette:{command:process.env.PYTHON_BIN||'/usr/bin/python3',args:[provider],env_allow:[],limits:{max_consecutive_errors:3}}}}));
 let w=json(['work','create'],{schema_version:1,event_id:id(),expected_revision:0,title:'Recette diagnostic copiable',objective:'Copier un diagnostic sans secret',scope:'racine isolée',criteria:['diagnostic copiable'],next:'lancer'}).work;
 w=json(['task','add',w.id],{schema_version:1,event_id:id(),expected_revision:w.revision,id:'t7',title:'Trois erreurs contextualisées',deliverable:'docs/t7.md',criteria:['cause, tentative, version et action'],owner:'recette',next:'diagnostiquer'}).work;
 json(['agent','start',w.id],{schema_version:1,event_id:id(),expected_revision:w.revision,task_id:'t7',provider:'recette',workspace:root,role:'worker',instruction:'recette locale',capture_output:true});
 let ended;for(let i=0;i<80;i++){ended=json(['agent','list',w.id]).agents[0]?.agent;if(ended&&!['queued','starting','running','stopping'].includes(ended.status))break;Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,100)}
 assert.equal(ended.status,'interrupted');assert.equal(ended.diagnostic.observed_errors,3);
 const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
 try{
  const url=await new Promise((resolve,reject)=>{let text='',err='';const timer=setTimeout(()=>reject(Error(text+err)),10000);server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});server.stderr.on('data',d=>err+=d);server.on('error',reject)});
  const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1280,height:900});
  // Headless Chrome denies real navigator.clipboard writes even after
  // Browser.grantPermissions in this sandbox (no window focus). The app's
  // click handler and the copied text are still real; only the OS clipboard
  // hop is stubbed here so the assertion is deterministic in CI.
  await page.evaluateOnNewDocument(()=>{window.__clipboard='';Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:t=>{window.__clipboard=t;return Promise.resolve()},readText:()=>Promise.resolve(window.__clipboard)}})});
  await page.goto(url+'?work='+w.id);
  await page.waitForFunction(()=>snapshot?.mission?.tasks?.[0]?.diagnostic?.observed_errors===3);
  const results=await page.$('#mission-results>summary');await results.focus();await page.keyboard.press('Enter');await page.waitForFunction(()=>document.querySelector('#mission-results').open);
  await page.waitForSelector('.mission-task .mission-diagnostic-copy button');
  await page.locator('.mission-task .mission-diagnostic-copy button').click();
  await page.waitForFunction(()=>document.querySelector('.mission-task .mission-diagnostic-copy p')?.textContent==='Diagnostic copié.');
  let copied=await page.evaluate(()=>window.__clipboard);
  for(const text of ['Tentative : ','Version : ','Cause : ','Action disponible : '])assert.match(copied,new RegExp(escapeRegExp(text)));
  assert.doesNotMatch(copied,/https?:\/\//,'copied diagnostic must not contain a session URL');
  assert.doesNotMatch(copied,new RegExp(escapeRegExp(root)),'copied diagnostic must not leak the workspace path');
  assert.doesNotMatch(copied,/Traces techniques|mount \/mnt\/partage/,'copied diagnostic must stay short — technical traces remain in the expandable detail, not in the clipboard');
  checks.push('Diagnostic copié en français : cause, tentative, version, action ; ni URL ni chemin d’espace de travail (req-21)');
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'diagnostic-copy-'+theme+'.png','.mission-task .mission-diagnostic');

  await Promise.all([page.waitForNavigation(),page.select('#swarm-language','en')]);
  await page.waitForFunction(()=>document.documentElement.lang==='en');
  await page.waitForFunction(()=>snapshot?.mission?.tasks?.[0]?.diagnostic?.observed_errors===3);
  const detailsOpen=await page.$eval('#mission-results',e=>e.open);if(!detailsOpen){const r2=await page.$('#mission-results>summary');await r2.focus();await page.keyboard.press('Enter');await page.waitForFunction(()=>document.querySelector('#mission-results').open)}
  await page.waitForSelector('.mission-task .mission-diagnostic-copy button');
  assert.equal(await page.$eval('.mission-task .mission-diagnostic-copy button',e=>e.textContent),'Copy diagnostic');
  await page.locator('.mission-task .mission-diagnostic-copy button').click();
  await page.waitForFunction(()=>document.querySelector('.mission-task .mission-diagnostic-copy p')?.textContent==='Diagnostic copied.');
  copied=await page.evaluate(()=>window.__clipboard);
  for(const text of ['Attempt: ','Version: ','Cause: ','Available action: '])assert.match(copied,new RegExp(escapeRegExp(text)));
  assert.doesNotMatch(copied,/https?:\/\//,'copied diagnostic must not contain a session URL (en)');
  assert.match(copied,/3 tool errors observed/);assert.match(copied,/Execution environment/);assert.match(copied,/Inspect the trace, fix the cause/);assert.doesNotMatch(copied,/erreurs d’outil|La configuration nécessaire|Faire vérifier|Examiner la trace/);
  const hostile=await page.evaluate(()=>Mission.diagnosticText({attempt_id:'token=TOPSECRET',observed_errors:3,summary:'Bearer TOPSECRET https://private/session',items:[{category:'limit',label:'TOPSECRET',cause:'password=TOPSECRET',action:'https://private/session',traces:['TOPSECRET']},{category:'__proto__',cause:'TOPSECRET'}]}));
  assert.doesNotMatch(hostile,/TOPSECRET|https?:|password=|Bearer/);
  assert.match(hostile,/Execution limit reached/);assert.match(hostile,/Unknown cause/);
  checks.push('Copie : champs libres hostiles, secret, URL et catégorie inconnue exclus ; texte issu du catalogue seulement');
  checks.push('Diagnostic copié en anglais avec les mêmes garanties (req-21, req-22)');
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'diagnostic-copy-en-'+theme+'.png','.mission-task .mission-diagnostic');
  await page.evaluate(()=>{navigator.clipboard.writeText=()=>Promise.reject(new Error('clipboard denied'))});
  // Keyboard activation also works after screenshot scrolling in the long panel.
  await page.focus('.mission-task .mission-diagnostic-copy button');
  await page.keyboard.press('Enter');
  await page.waitForFunction(()=>document.querySelector('.mission-task .mission-diagnostic-copy p')?.textContent.includes('Copy failed'));
  checks.push('Refus du presse-papiers : erreur explicite, aucun faux succès');
  await page.close();
 }finally{server.kill()}
}
function escapeRegExp(s){return s.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')}

(async()=>{
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox'],userDataDir:fs.mkdtempSync(path.join(os.tmpdir(),'swarm-t7-browser-'))});
 try{
  await partSummary(browser);
  await partDiagnostic(browser);
  assert.deepEqual(errors,[]);
  fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks,errors},null,2));
  fs.rmSync(path.join(out,'failure.txt'),{force:true});
  console.log(checks.join('\n'));
  console.log('PASS T7 pre-launch summary and copyable diagnostic (FR/EN, both themes)');
 }catch(e){fs.writeFileSync(path.join(out,'failure.txt'),e.stack);console.error(e);process.exitCode=1}
 finally{await browser.close()}
})();
