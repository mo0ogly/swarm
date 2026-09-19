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
pathlib.Path("docs").mkdir(exist_ok=True)
for id in ["g1","g2","g3","g4","g5","g6"]: pathlib.Path("docs/"+id+".md").write_text("# Rapport de recette\\nLivrable factice, aucune IA.")
time.sleep(1)
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

const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
(async()=>{
 const url=await new Promise(resolve=>{let text='';server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m)resolve(m[0])})});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(20000);await page.setViewport({width:1440,height:1100});await page.goto(url);await page.waitForSelector('#pilot-mission');await page.select('#work',dense.id);await page.waitForFunction(()=>document.getElementById('title').textContent==='Plan dense');
 assert.equal(cli('mission','status',dense.id).enabled,false);
 assert.equal(await page.$eval('#mission-primary',e=>e.textContent),'Résoudre le blocage');
 assert.match(await page.$eval('#mission-link',e=>e.href),new RegExp('/session/[^?]+\\?work='+dense.id));
 assert.equal(await page.$$eval('.mission-brief p',es=>es.length),2);
 await page.click('#pilot-launch-all');await page.waitForSelector('#modal[open] #field-provider');await page.focus('#field-provider');await page.keyboard.press('End');await page.keyboard.press('Enter');await page.waitForFunction(()=>!document.getElementById('confirm').disabled);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.screenshot({path:path.join(outDir,'config-'+theme+'.png'),fullPage:true})}
 await page.click('#confirm');await page.waitForFunction(()=>!document.getElementById('modal').open);await page.waitForFunction(()=>document.querySelector('#mission-summary').textContent.includes('Résultat produit'));
 assert.equal(cli('mission','status',dense.id).enabled,true);
 await page.waitForFunction(()=>snapshot.agents.length>=2);
 assert.equal(cli('agent','list',dense.id).agents.filter(x=>x.agent.task_id==='g3').length,0);
 const before=cli('agent','list',dense.id).agents.length;
 await page.click('#pilot-launch-all');await page.waitForSelector('#modal[open]');
 assert.match(await page.$eval('#modal',e=>e.textContent),/Suivi de la mission/);
 assert.equal(await page.$eval('#confirm',e=>e.hidden),true);
 assert.equal(await page.$('#field-provider'),null);
 assert.doesNotMatch(await page.$eval('#modal',e=>e.textContent),/relancez ce bouton/);
 assert.equal(cli('agent','list',dense.id).agents.length,before);
 await page.click('#cancel');

 await page.click('[data-mission-action=control]');await page.waitForFunction(()=>document.querySelector('#mission-summary h3').textContent==='Mission en pause');assert.equal(cli('mission','status',dense.id).paused,true);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);const missing=await page.evaluate(()=>{const css=[...document.styleSheets].flatMap(s=>[...s.cssRules].map(r=>r.cssText)).join(' ');const tokens=[...new Set([...css.matchAll(/var\((--wattson-[\w-]+)/g)].map(m=>m[1]))];const style=getComputedStyle(document.documentElement);return tokens.filter(t=>!style.getPropertyValue(t).trim())});assert.deepEqual(missing,[]);await page.screenshot({path:path.join(outDir,'results-'+theme+'.png'),fullPage:true})}
 assert.equal(await page.$eval('#mission-primary',e=>e.textContent),'Examiner le résultat');
 await page.click('#mission-primary');await page.waitForSelector('#modal[open]');assert.match(await page.$eval('#modal',e=>e.textContent),/rapport|Rapport/);await page.keyboard.press('Escape');
 await page.waitForFunction(()=>!document.getElementById('modal').open);
 await page.click('#pilot-launch-all');await page.waitForFunction(()=>document.getElementById('modal-title').textContent==='Mission en pause');
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.screenshot({path:path.join(outDir,'suivi-'+theme+'.png'),fullPage:true})}
 const resume=await page.$('#modal-fields .primary');assert.ok(resume);await resume.click();await page.waitForFunction(()=>!snapshot.mission.paused).catch(async e=>{console.error(await page.$eval('#message',e=>e.textContent));throw e});assert.equal(cli('mission','status',dense.id).paused,false);assert.ok(await page.$$eval('.graph-arete',es=>es.length)>0);
 await page.reload();await page.waitForFunction(()=>snapshot?.mission.enabled===true);assert.deepEqual(errors,[]);
 const result={status:'PASS',checks:['autorisation explicite depuis le web','configuration unique','Lancer tout active la mission persistante','espace commun libéré : deuxième départ automatique','dépendance non validée retenue','second clic affiche le suivi sans lancement doublé','processus factice et résultat à examiner','pause et reprise effectives moteur','accès direct à la revue','persistance au rechargement','flèches conservées','deux thèmes et jetons définis'],errors};fs.writeFileSync(path.join(outDir,'ui.json'),JSON.stringify(result,null,2));console.log(result);
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{cli('mission','stop',dense.id);if(browser)await browser.close();for(const {agent}of cli('agent','list',dense.id).agents)if(['running','starting','queued'].includes(agent.status))cli('agent','stop',agent.id);server.kill()});
