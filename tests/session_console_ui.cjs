// Recette du graphe vivant : travail vide, plan dense, tâche en échec, et une
// tentative en cours dont l'action est lisible. Aucun modèle appelé.
'use strict';
const assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn,execFileSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]),outDir=path.resolve(process.argv[3]);
fs.mkdirSync(outDir,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-graph-'));
const checks=[],errors=[],external=[];
const cli=(...args)=>{const r=execFileSync(binary,['--root',root,'--json',...args].filter(x=>x!==undefined),{encoding:'utf8',input:args.input});return r.startsWith('{')||r.startsWith('[')?JSON.parse(r):null};
const send=(args,data)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,'--input','-'],{encoding:'utf8',input:JSON.stringify(data)}));
const uuid=()=>require('node:crypto').randomUUID().replace(/-/g,'');
const mutate=(args,revision,fields)=>send(args,{schema_version:1,event_id:uuid(),expected_revision:revision,...fields}).work;

cli('init');
// Fournisseur factice : annonce un outil puis reste en vie, pour observer une
// tentative réelle sans appeler un modèle.
const provider=path.join(root,'provider.py');
fs.writeFileSync(provider,`import json,sys,time
sys.stdin.read()
print(json.dumps({"type":"assistant","message":{"content":[{"type":"text","text":"Je mesure la charge avant de poursuivre."}]}}),flush=True)
print(json.dumps({"type":"assistant","message":{"content":[{"type":"tool_use","id":"probe","name":"Bash","input":{"description":"Mesurer la consommation CPU","command":"python3 probe.py"}}]}}),flush=True)
print(json.dumps({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"probe","content":"charge 12%"}]}}),flush=True)
import pathlib
while not pathlib.Path("release").exists(): time.sleep(.1)
`);
fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{recette:{command:process.env.PYTHON_BIN||'/usr/bin/python3',args:[provider],env_allow:[]}}}));

let dense=mutate(['work','create'],0,{title:'Plan dense',objective:'Observer un graphe à plusieurs niveaux',scope:'recette',criteria:['graphe lisible'],next:'lancer'});
const chaine=[['g1',[]],['g2',[]],['g3',['g1']],['g4',['g2','g3']],['g5',['g4']],['g6',['g4']]];
for(const [id,depends] of chaine) dense=mutate(['task','add',dense.id],dense.revision,{id,title:'Tâche '+id,deliverable:'rapport '+id,criteria:['preuve'],owner:'recette',next:'lancer',depends});
cli('autonomy',dense.id,'manuel');

let vide=mutate(['work','create'],0,{title:'Travail vide',objective:'Aucune tâche',scope:'recette',criteria:['écran vide lisible'],next:'ajouter une tâche'});
cli('autonomy',vide.id,'manuel');

let echec=mutate(['work','create'],0,{title:'Travail en échec',objective:'Observer un blocage',scope:'recette',criteria:['motif visible'],next:'corriger'});
echec=mutate(['task','add',echec.id],echec.revision,{id:'e1',title:'Tâche bloquée',deliverable:'rapport',criteria:['preuve'],owner:'recette',next:'corriger'});
echec=mutate(['task','update',echec.id],echec.revision,{id:'e1',status:'blocked',blocker:'Dépendance externe indisponible',next:'Décider de la suite'});
cli('autonomy',echec.id,'manuel');

const started=send(['agent','start',dense.id],{schema_version:1,event_id:uuid(),expected_revision:dense.revision,task_id:'g2',provider:'recette',workspace:root,role:'worker',instruction:'recette locale',capture_output:true});
const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
(async()=>{
 const url=await new Promise(resolve=>{let text='';server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m)resolve(m[0])})});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome'});const page=await browser.newPage(),csp=[];page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(/Content Security Policy|Refused to/.test(m.text()))csp.push(m.text())});page.setDefaultTimeout(20000);await page.setViewport({width:1440,height:1100});await page.goto(url);await page.waitForSelector('#pilot-mission');await page.select('#work',dense.id);await page.waitForFunction(()=>snapshot?.agents?.some(x=>x.agent.task_id==='g2'));
 const graphSession='.graph-go[data-task-session="g2"]';await page.waitForSelector(graphSession,{visible:true});
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.screenshot({path:path.join(outDir,'graph-'+theme+'.png'),fullPage:true})}
 await page.$eval(graphSession,e=>e.focus());await page.keyboard.press('Enter');await page.waitForSelector('#agent-terminal-dialog iframe');assert.equal(new URL(await page.$eval('#agent-terminal-dialog iframe',e=>e.src)).searchParams.get('agent'),started.agent.id);
 await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.getElementById('agent-terminal-dialog'));assert.equal(await page.evaluate(()=>document.activeElement.dataset.sessionLocation),'graph');
 await page.click('#mode');await page.click('[data-view="tasks"]');
 const sessionButton='#tasks-body [data-task="g2"] [data-task-session="g2"]';
 await page.waitForSelector(sessionButton,{visible:true});assert.equal(await page.$eval(sessionButton,e=>e.textContent),'Voir l’agent travailler');assert.equal(await page.$eval(sessionButton,e=>e.dataset.agentSession),started.agent.id);
 await page.focus(sessionButton);await page.keyboard.press('Enter');await page.waitForSelector('#agent-terminal-dialog iframe');assert.equal(new URL(await page.$eval('#agent-terminal-dialog iframe',e=>e.src)).searchParams.get('agent'),started.agent.id);
 const frame=await(await page.$('#agent-terminal-dialog iframe')).contentFrame();await frame.waitForSelector('#session-console .monaco-editor');await frame.waitForFunction(()=>document.querySelector('#session-console-fallback').textContent.includes('ACTIVITÉ'));
 await frame.waitForFunction(()=>document.querySelector('#session-console-fallback').textContent.includes('charge 12%'));assert.match(await frame.$eval('#session-console-fallback',e=>e.textContent),/MESSAGE · Je mesure/);assert.doesNotMatch(await frame.$eval('#session-console-fallback',e=>e.textContent),/tool_use_id/);
 assert.match(await frame.$eval('#console-mode-label',e=>e.textContent),/Journal de l’agent/);assert.equal(await frame.$eval('#terminal-screen',e=>e.hidden),true);assert.equal(await frame.$eval('#terminal-claim',e=>e.disabled),true);assert.match(await frame.$eval('#terminal-status',e=>e.textContent),/Agent automatisé/);assert.match(await frame.$eval('#session-cost',e=>e.textContent),/Coût inconnu/);assert.ok(await frame.$('.console-time'));assert.ok(await frame.$('.console-type-action'));assert.ok(await frame.$('.console-message-current'));
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await frame.waitForFunction(t=>document.documentElement.dataset.theme===t,{},theme);const missing=await frame.evaluate(()=>{const s=getComputedStyle(document.documentElement);return ['info-fond','info-encre','attention-fond','attention-encre','alerte-fond','alerte-encre','champ','texte'].filter(x=>!s.getPropertyValue('--wattson-'+x).trim())});assert.deepEqual(missing,[]);await frame.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));await frame.waitForFunction(()=>document.querySelector('.view-lines')?.getBoundingClientRect().width>200);await page.screenshot({path:path.join(outDir,theme+'.png'),fullPage:true})}
 await frame.click('#console-follow');assert.equal(await frame.$eval('#console-follow',e=>e.getAttribute('aria-pressed')),'false');await frame.click('#terminal-toggle');assert.equal(await frame.$eval('#terminal-screen',e=>e.hidden),false);await frame.click('#console-toggle');assert.equal(await frame.$eval('#session-console-wrap',e=>e.hidden),false);
 await frame.click('.monaco-editor');await page.keyboard.down('Control');await page.keyboard.press('f');await page.keyboard.up('Control');await frame.waitForSelector('.find-widget.visible');
 await page.keyboard.press('Escape');await frame.waitForFunction(()=>!document.querySelector('.find-widget.visible'));assert.ok(await page.$('#agent-terminal-dialog[open]'));
 const ended=await frame.evaluate(async()=>{const {sessionProgress}=await import('/session-console.js');return ['failed','interrupted','completed'].map(status=>sessionProgress({status,progress:{pending_tools:1},desired:'stop'}))});for(const text of ended)assert.doesNotMatch(text,/attente/);const sessionURL=frame.url();
 await page.evaluate(()=>{tasksKey='';render()});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#agent-terminal-dialog'));assert.equal(await page.evaluate(sel=>document.activeElement===document.querySelector(sel),sessionButton),true);
 fs.writeFileSync(path.join(root,'release'),'ok');await page.waitForFunction(sel=>document.querySelector(sel)?.textContent==='Voir la session',{},sessionButton);assert.equal(await page.$eval(sessionButton,e=>e.dataset.agentSession),started.agent.id);
 const fallback=await browser.newPage();await fallback.setRequestInterception(true);fallback.on('request',r=>r.url().includes('/lib/monaco/editor.js')?r.abort():r.continue());await fallback.goto(sessionURL);await fallback.waitForFunction(()=>document.getElementById('console-mode-label').textContent.includes('affichage simplifié'));assert.match(await fallback.$eval('#session-console-fallback',e=>e.textContent),/ACTIVITÉ/);await fallback.close();
 assert.deepEqual(errors,[]);assert.deepEqual(csp,[]);assert.equal(cli('agent','list',dense.id).agents.length,1);const result={status:'PASS',checks:['vrai agent automatisé, aucun modèle appelé','accès clavier en un clic depuis la tâche','dernière tentative exacte et libellé actif/terminé','retour du focus après remplacement du bouton','console Monaco par défaut','sorties colorées, numéros de ligne et recherche','saisie désactivée en mode automatisé','suivi automatique débrayable','terminal brut toujours accessible','deux thèmes, jetons et CSP vérifiés','Échap ferme la recherche avant la session','Monaco indisponible : repli texte sans perte de lecture','fin prioritaire sur outil sans réponse'],errors,csp};fs.writeFileSync(path.join(outDir,'ui.json'),JSON.stringify(result,null,2));console.log(result);
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{fs.writeFileSync(path.join(root,'release'),'ok');if(browser)await browser.close();server.kill()});
