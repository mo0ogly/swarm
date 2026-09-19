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

const started=send(['agent','start',dense.id],{schema_version:1,event_id:uuid(),expected_revision:dense.revision,task_id:'g2',provider:'recette',workspace:root,role:'worker',instruction:'recette locale'});
const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
(async()=>{
 const url=await new Promise(resolve=>{let text='';server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m)resolve(m[0])})});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(20000);await page.setViewport({width:1440,height:1100});await page.goto(url);await page.waitForSelector('#pilot-mission');await page.select('#work',dense.id);await page.waitForFunction(()=>document.getElementById('title').textContent==='Plan dense');await page.waitForSelector('.graph-go[data-task="g1"]');
 const open=async()=>{await page.click('.graph-go[data-task="g1"]');await page.waitForFunction(()=>document.getElementById('launch-eligibility')?.textContent.includes('En attente de l’espace'));};
 await open();assert.equal(await page.$eval('#confirm',e=>e.hidden),true);assert.equal(await page.$eval('#modal-fields details',e=>e.open),false);assert.match(await page.$eval('#launch-resolution',e=>e.textContent),/Tâche g2/);assert.equal(cli('agent','list',dense.id).agents.length,1);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.screenshot({path:path.join(outDir,theme+'.png'),fullPage:true})}
 await page.click('#launch-resolution button');await page.waitForSelector('#pilot-inspector[open]');assert.match(await page.$eval('#pilot-inspector',e=>e.textContent),/Tâche g2/);await page.keyboard.press('Escape');await open();
 await page.click('#field-instruction');await page.keyboard.type(' Consigne conservée');assert.match(await page.$eval('#field-instruction',e=>e.value),/Consigne conservée/);
 fs.writeFileSync(path.join(root,'release'),'ok');await page.waitForFunction(()=>snapshot.agents.every(x=>!['queued','starting','running','stopping'].includes(x.agent.status)));
 await page.click('#launch-guidance > button');await page.waitForFunction(()=>document.getElementById('launch-eligibility')?.textContent.includes('Conditions réunies'));assert.equal(await page.$eval('#confirm',e=>e.hidden),false);assert.match(await page.$eval('#field-instruction',e=>e.value),/Consigne conservée/);assert.equal(cli('agent','list',dense.id).agents.length,1);
 assert.deepEqual(errors,[]);const result={status:'PASS',checks:['attente affichée automatiquement avant confirmation','vraie tâche active identifiée','options avancées repliées','aucun départ sur aperçu','suivi direct de la tâche active','libération reconnue dans la même fenêtre malgré révision modifiée','consigne conservée','deux thèmes rendus'],errors};fs.writeFileSync(path.join(outDir,'ui.json'),JSON.stringify(result,null,2));console.log(result);
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{fs.writeFileSync(path.join(root,'release'),'ok');if(browser)await browser.close();server.kill()});
