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
 await page.click('#tasks-body [data-task="'+id+'"] button');
 await page.waitForSelector('#modal[open] #field-action',{visible:true});
 await page.waitForFunction(id=>modalContext?.task===id&&document.querySelector('#modal-title').textContent.length>0,{},id);
 // Depuis S28 l'action présélectionnée est celle que l'oracle conseille pour
 // l'état de la tâche : supposer « start » revenait à mesurer l'oracle. La
 // recette choisit explicitement l'action qu'elle veut exercer.
 await page.select('#field-action',action);
 assert.equal(await page.$eval('#field-action',e=>e.value),action,'Action shown differs from requested action');
}
async function confirm(){await page.click('#confirm');await page.waitForFunction(()=>!document.querySelector('#modal').open||!document.querySelector('#modal-error').hidden);if(await page.$eval('#modal',e=>e.open))throw new Error(await text('#modal-error'))}
async function shot(name){await page.screenshot({path:path.join(reportDir,name+'.png'),fullPage:true})}
(async()=>{
 try{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('Server did not start')),10000);server.stdout.on('data',d=>{out+=d;const m=out.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m){clearTimeout(timer);resolve(m[0])}});server.on('exit',code=>reject(new Error('Server exited '+code)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox'],userDataDir:fs.mkdtempSync(path.join(os.tmpdir(),'swarm-ui-'))});
 page=await browser.newPage();page.setDefaultTimeout(15000);await page.setViewport({width:1440,height:1000});page.on('pageerror',e=>errors.push(e.message));
 await page.goto(url);await page.waitForSelector('#tasks-body [data-task="UI-01"]');await page.waitForSelector('#mode');if(await page.evaluate(()=>document.body.dataset.mode)==='conduite'){await page.click('#mode');await page.waitForFunction(()=>document.body.dataset.mode==='expert')};await shot('etat-tasks');
 await openTask('UI-01');assert.match(await text('#preview'),/Périmètre/);await page.keyboard.press('Escape');assert.equal(await page.$eval('#modal',e=>e.open),false);// Ce qui compte est le retour du focus sur le bouton qui a ouvert le dialogue,
 // pas son libellé : depuis S28 celui-ci suit l'action conseillée par l'oracle
 // des actions, donc l'écrire en dur ne mesurait plus que l'oracle.
 assert.equal(await page.$eval(':focus',e=>e.closest('[data-task]')?.dataset.task||''),'UI-01');
 assert.equal(await page.$eval(':focus',e=>e.tagName),'BUTTON');assert.equal(await text('#active-count'),'0');checks.push('Cancel / Escape / focus return without launch');
 // Dépendance non satisfaite. Depuis S28, le départ n'est plus proposé du tout :
 // l'oracle des actions retire « start » et porte le motif sur le bouton, avant
 // toute tentative. La recette vérifie donc le refus là où il a lieu désormais,
 // et non un message d'erreur de modale qui ne peut plus apparaître.
 await page.click('[data-view="tasks"]');
 await page.waitForFunction(()=>/[Dd]épendance/.test(document.querySelector('#tasks-body [data-task="UI-02"] button').title));
 await page.click('#tasks-body [data-task="UI-02"] button');
 await page.waitForSelector('#modal[open] #field-action',{visible:true});
 const offertes=await page.$$eval('#field-action option',es=>es.map(e=>e.value));
 assert.ok(!offertes.includes('start'),'le départ reste proposé malgré une dépendance non satisfaite : '+offertes.join(', '));
 await shot('etat-dependency-refusal');await page.click('#cancel');checks.push('Dependency refusal explained before any launch');
 await openTask('UI-01');await page.select('#field-provider','recette');await page.select('#field-role','subplanner');await fill('#field-instruction','Recette isolée : produire uniquement les événements de fixture.');await shot('etat-launch-modal');await confirm();try{await has('#active-count','1')}catch(err){const etat=await page.evaluate(()=>({count:document.querySelector('#active-count').textContent,agents:(window.snapshot?.agents||[]).map(x=>x.agent.status+':'+x.agent.task_id),taches:(window.snapshot?.work.tasks||[]).map(t=>t.id+'='+t.status)}));console.error('DIAG',JSON.stringify(etat));throw err}await shot('etat-active');
 // Depuis SC-17, une tentative qui se termine en ayant produit un handoff
 // lisible n'est plus bloquée pour que l'humain la relaie : le conducteur la
 // soumet. La recette attend donc l'état relayé, et vérifie que l'acceptation
 // reste refusée à ce stade — c'est la limite du relais automatique.
 // La table porte l'état de validation, pas le statut brut : après relais la
 // tâche est à vérifier, et l'action conseillée est la gate — c'est-à-dire
 // le geste que le produit refuse d'automatiser.
 await has('#tasks-body [data-task="UI-01"]','À vérifier');
 await has('#tasks-body [data-task="UI-01"]','gate');
 assert.equal(await page.$eval('#accepted-count',e=>e.textContent),'0','le relais automatique ne doit jamais valoir acceptation');
 checks.push('Task/provider/role/workspace/instruction launch; handoff relayed without acceptance');
 await page.click('[data-view="agents"]');await has('#agents-list','subplanner');await page.click('#agents-list button');
 // Le dialogue se peuple en deux temps : la liste des actions vient d'un appel
 // réseau. Sélectionner avant qu'elle soit remplie laissait l'action à sa
 // valeur conseillée, sans erreur visible, et la recette confirmait autre chose
 // que ce qu'elle croyait.
 await page.waitForFunction(()=>[...document.querySelectorAll('#field-action option')].some(o=>o.value==='retry'));
 await page.select('#field-action','retry');
 await page.waitForFunction(()=>document.querySelector('#field-action').value==='retry');
 await confirm();await has('#active-count','1');await page.click('[data-view="tasks"]');await openTask('UI-01','stop');const options=await page.$$eval('#field-agent option',es=>es.map(e=>({value:e.value,text:e.textContent})));const running=options.find(x=>/En cours|Démarrage|En attente|queued\/unconfirmed/.test(x.text));assert.ok(running,JSON.stringify(options));await page.select('#field-agent',running.value);await page.click('#cancel');await has('#active-count','1');await openTask('UI-01','stop');await page.select('#field-agent',running.value);await confirm();await has('#active-count','0');checks.push('Retry preserves role; cancel stop has no effect; selected attempt stopped');
 await page.click('[data-view="logs"]');await has('#log-lines','#');await fill('#log-search','Processus');await page.click('#log-form button');await has('#log-lines','Processus');await page.click('#log-live');assert.equal(await text('#log-live'),'Mettre en pause');await page.click('#log-live');await shot('etat-logs');checks.push('Search / live / pause logs through controls');
 await page.click('[data-view="decisions"]');await page.waitForSelector('#decisions-list button');await page.click('#decisions-list button');await fill('#field-note','Résultat examiné ; la revue de la tâche reste distincte.');await confirm();await has('#decisions-list','Résultat examiné');await page.click('[data-view="tasks"]');await has('#tasks-body [data-task="UI-01"]','Bloquée');checks.push('Decision acknowledgement persisted without task acceptance');
 await openTask('UI-01','report');await page.click('#confirm');await has('#preview','Rapport de fixture');assert.equal(await page.$('#preview img'),null);assert.equal(await page.evaluate(()=>window.injected),undefined);await page.click('#cancel');await openTask('UI-01','submit');await confirm();await has('#tasks-body [data-task="UI-01"]','À vérifier');
 await openTask('UI-01','gate');await fill('#field-name','Recette du parcours');await page.click('#confirm');await has('#confirm','Confirmer l’enregistrement');await shot('etat-gate-preview');await page.click('#cancel');
 // Un aperçu annulé n'enregistre pas de gate, donc l'acceptation reste
 // interdite. Depuis S28 elle n'est plus offerte du tout : le dialogue ne
 // liste que les actions disponibles et porte le motif sur le bouton de la
 // tâche. On vérifie donc l'absence de l'option et la lisibilité du motif,
 // et non un message d'erreur qui ne peut plus apparaître.
 await page.click('[data-view="tasks"]');
 const motif=await page.$eval('#tasks-body [data-task="UI-01"] button',e=>e.title);
 assert.match(motif,/gate/i,'le motif du refus d’acceptation doit être lisible sans ouvrir le dialogue : '+motif);
 await page.click('#tasks-body [data-task="UI-01"] button');
 await page.waitForSelector('#modal[open] #field-action',{visible:true});
 const offertesGate=await page.$$eval('#field-action option',es=>es.map(e=>e.value));
 assert.ok(!offertesGate.includes('accepted'),'acceptation offerte sans gate enregistrée : '+offertesGate.join(', '));
 await page.click('#cancel');
 checks.push('Report read/submitted; cancelled preview records no gate and blocks acceptance');
 await openTask('UI-01','gate');await fill('#field-name','Recette du parcours');await page.click('#confirm');await has('#confirm','Confirmer l’enregistrement');await confirm();await openTask('UI-01','accepted');await has('#preview','PASS');await confirm();await has('#tasks-body [data-task="UI-01"]','Acceptée');checks.push('Gate preview/record then acceptance with live row refresh');
 // Filesystem drift must be detected without a database revision or manual refresh.
 fs.appendFileSync(path.join(root,'docs/UI-01-handoff.md'),'\nModification postérieure à la validation.\n');
 await has('#tasks-body [data-task="UI-01"]','à revalider');await has('#accepted-count','0');await has('#validation-status','validation finale reste à obtenir');
 await page.click('#tasks-body [data-task="UI-01"] summary');await has('#tasks-body [data-task="UI-01"]','Preuve modifiée : docs/UI-01-handoff.md');
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await shot(theme+'-stale');}
 await page.evaluate(()=>document.documentElement.dataset.theme='etat');
 // La preuve modifiée périme l'acceptation de UI-01, donc la dépendance de
 // UI-02 n'est plus fraîche. Depuis S28 le départ n'est plus proposé : le
 // refus se lit sur le bouton de la tâche, avant toute tentative.
 await page.click('[data-view="tasks"]');
 const motifPerime=await page.$eval('#tasks-body [data-task="UI-02"] button',e=>e.title);
 assert.match(motifPerime,/dépendance/i,'le motif de la dépendance périmée doit rester lisible : '+motifPerime);
 await page.click('#tasks-body [data-task="UI-02"] button');
 await page.waitForSelector('#modal[open] #field-action',{visible:true});
 const offertesPerime=await page.$$eval('#field-action option',es=>es.map(e=>e.value));
 assert.ok(!offertesPerime.includes('start'),'départ proposé malgré une preuve périmée : '+offertesPerime.join(', '));
 await page.click('#cancel');
 await page.click('[data-view="decisions"]');
 // La liste des décisions est reconstruite à chaque rafraîchissement du
 // cockpit : un locator qui attend la stabilité du nœud n'aboutit jamais. On
 // déclenche le bouton par son texte, sur l'état courant du DOM.
 await page.waitForFunction(()=>[...document.querySelectorAll('#decisions-list button')].some(b=>b.textContent.includes('Revalider cette tâche')));
 await page.evaluate(()=>[...document.querySelectorAll('#decisions-list button')].find(b=>b.textContent.includes('Revalider cette tâche')).click());await has('#preview','Preuve modifiée');
 // Le bouton enchaîne deux temps asynchrones : ouverture du dialogue puis
 // choix de l'action selon l'état. Lire le champ avant la fin donnait une
 // chaîne vide, et l'assertion mesurait la course, pas le comportement.
 await page.waitForFunction(()=>document.querySelector('#field-action')?.value);
 assert.equal(await page.$eval('#field-action',e=>e.value),'reopen','revalider une tâche acceptée doit proposer sa réouverture');await confirm();
 // A rehashed gate alone cannot pass: explicit new report and unchanged rubric required.
 const gatePath=path.join(root,'docs/UI-01.evidence.json'),gate=JSON.parse(fs.readFileSync(gatePath));
 gate.artifacts['docs/UI-01-handoff.md']=require('crypto').createHash('sha256').update(fs.readFileSync(path.join(root,'docs/UI-01-handoff.md'))).digest('hex');fs.writeFileSync(gatePath,JSON.stringify(gate));
 // La tâche vient d'être rouverte : elle est à faire, donc la gate n'est pas
 // proposée. Depuis S28 le refus se lit avant d'ouvrir le dialogue, et le
 // motif nomme le rapport manquant — une simple ré-empreinte ne peut donc même
 // pas être tentée depuis l'écran.
 await page.click('[data-view="tasks"]');
 const motifGate=await page.$eval('#tasks-body [data-task="UI-01"] button',e=>e.title);
 assert.match(motifGate,/rapport/i,'le motif du refus de gate doit nommer le rapport manquant : '+motifGate);
 await page.click('#tasks-body [data-task="UI-01"] button');
 await page.waitForSelector('#modal[open] #field-action',{visible:true});
 const offertesRehash=await page.$$eval('#field-action option',es=>es.map(e=>e.value));
 assert.ok(!offertesRehash.includes('gate'),'gate proposée sur une tâche sans rapport soumis : '+offertesRehash.join(', '));
 await page.click('#cancel');
 // The fixture control really runs; this is not a business security certification.
 const result=execFileSync('python3',['-c','from pathlib import Path; p=Path("docs/UI-01-handoff.md"); assert "Modification postérieure" in p.read_text(); print("PASS : contenu modifié contrôlé dans la fixture")'],{cwd:root,encoding:'utf8'});
 const report='docs/UI-01-revalidation-handoff.md';fs.writeFileSync(path.join(root,report),'# Nouvelle recette\n'+result);
 gate.artifacts[report]=require('crypto').createHash('sha256').update(fs.readFileSync(path.join(root,report))).digest('hex');gate.results[0].evidence.push(report);fs.writeFileSync(gatePath,JSON.stringify(gate));
 await openTask('UI-01','submit');await page.select('#field-path',report);await confirm();
 await openTask('UI-01','gate');await fill('#field-name','Revalidation du parcours');await page.click('#confirm');await has('#confirm','Confirmer l’enregistrement');await confirm();
 await openTask('UI-01','accepted');await confirm();await has('#accepted-count','1');await has('#tasks-body [data-task="UI-01"]','Acceptée');
 await page.click('[data-view="decisions"]');await has('#decisions-list','Revalidation constatée');await page.click('[data-view="tasks"]');
 checks.push('Drift visible live; stale dependency blocked; rehash alone refused; new executed control report and gate accepted; alert resolved');

 await page.click('[data-view="resume"]');await page.click('#ooda');for(const [k,v]of Object.entries({observation:'Parcours UI observé.',orientation:'Les états sont distincts.',decision:'Examiner la suite.',result:'Première tâche acceptée.',next:'Vérifier le budget de UI-02.'}))await fill('#field-'+k,v);await confirm();await has('#resume-text','Vérifier le budget');await shot('etat-ooda');await page.click('#visit');await has('#message','marquée comme vue');checks.push('OODA and per-operator visit retained');
 await page.click('[data-view="budget"]');await has('#usage-list','12 entrée / 8 sortie');await has('#usage-list','Cache lu : 19');await page.click('#budget-edit');for(const [k,v]of Object.entries({limit:'0.1',reserve:'1',source:'Estimation de fixture en USD par lancement',date:'2026-09-12'}))await fill('#field-'+k,v);await confirm();
 // Budget insuffisant : depuis S28 le lancement n'est plus proposé, et le motif
 // est lisible avant toute tentative.
 await page.click('[data-view="tasks"]');
 await page.waitForFunction(()=>/[Bb]udget/.test(document.querySelector('#tasks-body [data-task="UI-02"] button').title));
 await page.click('#tasks-body [data-task="UI-02"] button');
 await page.waitForSelector('#modal[open] #field-action',{visible:true});
 const offertesBudget=await page.$$eval('#field-action option',es=>es.map(e=>e.value));
 assert.ok(!offertesBudget.includes('start'),'lancement proposé malgré un budget insuffisant : '+offertesBudget.join(', '));
 await shot('etat-budget-refusal');await page.click('#cancel');checks.push('Declared tokens distinguished from estimated budget; exhausted budget blocks launch');
 await openTask('UI-02','override');await page.click('#confirm');await page.waitForFunction(()=>!document.querySelector('#modal-error').hidden);await fill('#field-note','Dérogation de recette uniquement ; aucun contrôle transformé en PASS.');await confirm();await has('#tasks-body [data-task="UI-02"]','Dérogation');checks.push('Forced validation requires reason and remains a derogation');
 await page.click('#theme');assert.equal(await page.$eval('html',e=>e.dataset.theme),'sombre');await shot('sombre-tasks');// La capture porte sur le rendu du dialogue en thème sombre, pas sur une
 // action précise : « accepter » n'est pas proposée sur une tâche sans rapport,
 // et la lecture du rapport l'est toujours.
 await openTask('UI-02','report');await shot('sombre-modal');await page.keyboard.press('Escape');
 await page.setViewport({width:390,height:844});await shot('sombre-mobile');assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'mobile page overflow');await page.setViewport({width:1440,height:1000});
 await page.reload();await page.waitForSelector('#tasks-body [data-task="UI-01"]');await has('#tasks-body [data-task="UI-01"]','Acceptée');await page.click('[data-view="decisions"]');await has('#decisions-list','Résultat examiné');checks.push('Reload preserves task state, decisions and theme');
 for(const theme of ['etat','sombre']){if(await page.$eval('html',e=>e.dataset.theme)!==theme)await page.click('#theme');for(const name of ['tasks','agents','decisions','logs','resume','budget']){await page.click('[data-view="'+name+'"]');await shot(theme+'-'+name+'-final');}await page.click('#budget-edit');await page.focus('#field-limit');await shot(theme+'-budget-focus');await page.keyboard.press('Escape');}
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(reportDir,'result.json'),JSON.stringify({status:'PASS',fixture_root:root,checks,page_errors:errors,model_calls:0},null,2));
 console.log(JSON.stringify({status:'PASS',checks:checks.length,report:reportDir}));
 }catch(e){if(page){await shot('failure').catch(()=>{});fs.writeFileSync(path.join(reportDir,'failure.txt'),await page.$eval('body',e=>e.innerText).catch(()=>''))}fs.writeFileSync(path.join(reportDir,'result.json'),JSON.stringify({status:'FAIL',error:e.stack,checks,page_errors:errors,fixture_root:root},null,2));throw e}
 finally{await browser?.close();server.kill('SIGTERM')}
})().catch(e=>{console.error(e);process.exitCode=1});
