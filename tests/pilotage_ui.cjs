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
time.sleep(120)
`);
fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{recette:{command:process.env.PYTHON_BIN||'/usr/bin/python3',args:[provider],env_allow:[]}}}));

let dense=mutate(['work','create'],0,{title:'Plan dense',objective:'Observer un graphe à plusieurs niveaux',scope:'recette',criteria:['graphe lisible'],next:'lancer'});
const chaine=[['g1',[]],['g2',['g1']],['g3',['g1']],['g4',['g2','g3']],['g5',['g4']],['g6',['g4']]];
for(const [id,depends] of chaine) dense=mutate(['task','add',dense.id],dense.revision,{id,title:'Tâche '+id,deliverable:'rapport '+id,criteria:['preuve'],owner:'recette',next:'lancer',depends});
cli('autonomy',dense.id,'manuel');

let vide=mutate(['work','create'],0,{title:'Travail vide',objective:'Aucune tâche',scope:'recette',criteria:['écran vide lisible'],next:'ajouter une tâche'});
cli('autonomy',vide.id,'manuel');

let echec=mutate(['work','create'],0,{title:'Travail en échec',objective:'Observer un blocage',scope:'recette',criteria:['motif visible'],next:'corriger'});
echec=mutate(['task','add',echec.id],echec.revision,{id:'e1',title:'Tâche bloquée',deliverable:'rapport',criteria:['preuve'],owner:'recette',next:'corriger'});
echec=mutate(['task','update',echec.id],echec.revision,{id:'e1',status:'blocked',blocker:'Dépendance externe indisponible',next:'Décider de la suite'});
cli('autonomy',echec.id,'manuel');

const workspace=path.join(root,'ws-g1');fs.mkdirSync(workspace);
send(['agent','start',dense.id],{schema_version:1,event_id:uuid(),expected_revision:cli('work','show',dense.id).work.revision,task_id:'g1',provider:'recette',workspace,role:'worker',timeout_seconds:300,capture_output:true});

const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
const noeuds=page=>page.$$eval('.graph-noeud',ns=>ns.map(n=>n.textContent));
let browser;
(async()=>{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(Error(out)),15000);server.stdout.on('data',d=>{out+=d;const m=out.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m){clearTimeout(timer);resolve(m[0])}})});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage();page.setDefaultTimeout(15000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('request',r=>{if(!['localhost','127.0.0.1'].includes(new URL(r.url()).hostname))external.push(r.url())});
 await page.setViewport({width:1500,height:1000});await page.goto(url);
 await page.waitForSelector('#pilot-view');
 const select=async(selector,value)=>{
  const index=await page.$eval(selector,(e,v)=>[...e.options].findIndex(o=>o.value===v),value);
  assert.ok(index>=0,value);await page.focus(selector);await page.keyboard.press('Home');
  for(let i=0;i<index;i++)await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');await page.waitForFunction((s,v)=>document.querySelector(s).value===v,{},selector,value);
 };
 await page.select('#work',dense.id);await page.waitForFunction(()=>document.getElementById('title').textContent==='Plan dense');
 assert.equal(await page.$eval('#pilot-view',e=>e.value),'dependencies');
 await page.waitForSelector('.graph-arete',{visible:true});
 assert.equal(await page.$eval('#pilot-list',e=>getComputedStyle(e).display),'none');
 await select('#pilot-view','agents');
 await page.waitForSelector('.pilot-card',{visible:true});
 assert.equal(await page.$eval('#pilot-canvas',e=>getComputedStyle(e).display),'none');

 assert.ok(await page.$('.pilot-role'));assert.ok(await page.$('.pilot-guidance'));
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.$eval('#pilot-list',e=>e.scrollIntoView({block:'start'}));await page.screenshot({path:path.join(outDir,'roles-'+theme+'.png')})}
 await page.evaluate(()=>setTheme('etat'));
 await page.click('.pilot-card-title');await page.waitForSelector('#pilot-inspector[open]');
 assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Ce qui était demandé|Prérequis/);
 await page.$eval('#pilot-inspector',e=>e.focus());await page.keyboard.press('Escape');
 checks.push('vue Agents, inspection de mission sans formulaire automatique');
 await select('#pilot-view','dependencies');
 await page.waitForFunction(()=>document.querySelectorAll('.graph-noeud').length===6);
 assert.equal(await page.$eval('#pilot-list',e=>getComputedStyle(e).display),'none');
 assert.equal(await page.$$eval('.graph-arete',es=>es.length),6);
 assert.ok(await page.$$eval('.graph-arete',es=>es.every(e=>e.getAttribute('marker-end').startsWith('url('))));
 const center=async id=>page.$eval('.graph-noeud[data-task="'+id+'"] rect',e=>({x:+e.getAttribute('x'),y:+e.getAttribute('y')}));
 let a=await center('g1'),b=await center('g2');assert.ok(b.x>a.x);
 await select('#pilot-orientation','TB');a=await center('g1');b=await center('g2');assert.ok(b.y>a.y);
 await select('#pilot-detail','detailed');assert.equal(await page.$eval('.graph-noeud',e=>e.querySelectorAll('[data-line]').length),10);
 checks.push('orientation horizontale/verticale, détails indépendants, flèches conservées');
 const fold=async id=>{await page.$eval('.graph-fold[data-task="'+id+'"]',e=>e.focus());await page.keyboard.press('Enter')};
 await fold('g2');assert.equal(await page.$$eval('.graph-noeud',e=>e.length),6);
 await fold('g3');assert.equal(await page.$$eval('.graph-noeud',e=>e.length),3);
 assert.match(await page.$eval('.graph-fold[data-task="g2"]',e=>e.textContent),/3 tâches/);
 await page.click('#pilot-expand');assert.equal(await page.$$eval('.graph-noeud',e=>e.length),6);
 checks.push('losange partagé, compteurs exacts, repli et dépli clavier');
 await fold('g1');
 dense=mutate(['task','update',dense.id],cli('work','show',dense.id).work.revision,{id:'g4',status:'blocked',blocker:'Dépendance indisponible'});await page.evaluate(()=>refresh(true));
 assert.match(await page.$eval('.graph-fold[data-task="g1"]',e=>e.textContent),/1 alertes/);
 await page.type('#pilot-search','g4');await page.waitForSelector('.graph-noeud[data-task="g4"]');
 assert.equal(await page.$$eval('.graph-noeud',ns=>ns.length),1);
 await page.click('#pilot-search',{clickCount:3});await page.keyboard.press('Backspace');await page.click('#pilot-expand');
 for(let i=0;i<20&&await page.evaluate(()=>Pilot.state.zoom<2);i++)await page.click('#pilot-zoom-in');
 assert.ok(await page.evaluate(()=>Math.abs(Pilot.state.zoom-2)<1e-9));
 await page.click('#pilot-fit');assert.ok(await page.evaluate(()=>Pilot.state.zoom>0&&Pilot.state.zoom<=1));
 checks.push('recherche dans une branche repliée et zoom 200 % sans changement métier');
 await page.$eval('.graph-noeud[data-task="g4"]',e=>e.focus());await page.keyboard.press('Enter');
 await page.waitForSelector('#pilot-inspector[open]');
 assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Prérequis manquants : g2, g3/);
 // Desktop inspector is non-modal; folds remain operable behind it.
 await fold('g1');
 assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/masquée/);
 await page.click('[data-inspector-action="reveal"]');
 await page.waitForSelector('.graph-noeud[data-task="g4"]');
 assert.equal(await page.$eval('.graph-noeud[data-task="g4"]',e=>document.activeElement===e),true);
 checks.push('sélection masquée conservée, révélation et focus');
 await page.$eval('#pilot-inspector',e=>e.focus());await page.keyboard.press('Escape');
 await select('#pilot-orientation','LR');await fold('g2');
 await page.reload();await page.waitForSelector('.graph-fold[data-task="g2"]');
 assert.equal(await page.$eval('#pilot-orientation',e=>e.value),'LR');
 assert.equal(await page.$eval('.graph-fold[data-task="g2"]',e=>e.getAttribute('aria-expanded')),'false');
 checks.push('préférences par travail conservées au rechargement');
 await page.click('#pilot-expand');await page.$eval('.graph-noeud[data-task="g1"]',e=>e.focus());
 await page.evaluate(()=>{window.pilotFocusNode=document.activeElement});
 const times=[];
 for(let i=0;i<30;i++){const time=await page.evaluate(async()=>{const t=performance.now();await refresh(true);return performance.now()-t});times.push(time)}
 assert.equal(await page.evaluate(()=>document.activeElement===window.pilotFocusNode&&window.pilotFocusNode.isConnected),true);
 checks.push('focus et identité DOM conservés sur 30 snapshots');
 await page.screenshot({path:path.join(outDir,'pilotage-etat.png'),fullPage:true});
 await page.click('#theme');await page.screenshot({path:path.join(outDir,'pilotage-sombre.png'),fullPage:true});
 await page.$eval('.graph-noeud[data-task="g1"]',e=>e.focus());await page.keyboard.press('Enter');
 await page.screenshot({path:path.join(outDir,'inspector-sombre.png'),fullPage:true});
 await page.click('#theme');await page.screenshot({path:path.join(outDir,'inspector-etat.png'),fullPage:true});
 checks.push('captures deux thèmes, graphe et panneau ouvert');
 assert.equal(await page.$$eval('.graph-role',es=>es.length),6);
 assert.match(await page.$eval('.graph-noeud[data-task="g4"]',e=>e.getAttribute('aria-label')),/À résoudre : Dépendance indisponible/);
 await page.$eval('#pilot-inspector',e=>e.focus());await page.keyboard.press('Escape');
 await page.click('#pilot-help');await page.waitForSelector('#modal[open]');assert.match(await page.$eval('#modal',e=>e.textContent),/Rôle à préciser/);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.screenshot({path:path.join(outDir,'aide-'+theme+'.png')})}
 await page.keyboard.press('Escape');await page.evaluate(()=>setTheme('etat'));
 await page.select('#work',vide.id);await page.waitForFunction(()=>document.getElementById('title').textContent==='Travail vide');
 assert.match(await page.$eval('#pilot-status',e=>e.textContent),/Aucune tâche/);
 await page.select('#work',echec.id);await page.waitForFunction(()=>document.getElementById('title').textContent==='Travail en échec');
 await page.click('#pilot-next');await page.waitForSelector('#pilot-inspector[open]');
 assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Dépendance externe|intervention|bloqu/i);
 checks.push('travail vide, intervention avec cause et impact');
 await page.$eval('#pilot-inspector',e=>e.focus());await page.keyboard.press('Escape');
 await page.setViewport({width:390,height:844});
 await select('#pilot-view','agents');
 await page.click('.pilot-card-title');await page.waitForSelector('#pilot-inspector[open]');
 assert.equal(await page.$eval('#pilot-inspector',e=>e.matches(':modal')),true);
 await page.screenshot({path:path.join(outDir,'mobile-etat.png'),fullPage:true});
 await page.keyboard.press('Escape');assert.equal(await page.$eval('#pilot-inspector',e=>e.open),false);
 checks.push('mobile : dialogue modal, Échap et fermeture');
 // Storage refusé : le cockpit reste opérable et annonce la non-persistance.
 const denied=await browser.newPage();denied.on('pageerror',e=>errors.push(e.message));
 await denied.evaluateOnNewDocument(()=>{Storage.prototype.getItem=function(){throw new DOMException('Denied','SecurityError')};Storage.prototype.setItem=function(){throw new DOMException('Denied','SecurityError')}});
 await denied.goto(new URL(url).origin);await denied.waitForSelector('#pilot-view');
 assert.match(await denied.$eval('#pilot-status',e=>e.textContent),/Préférences indisponibles|non mémorisé/);
 await denied.close();checks.push('stockage local refusé : interface fonctionnelle avec explication');
 for(const theme of ['etat','sombre']){
  const missing=await page.evaluate(theme=>{
   document.documentElement.dataset.theme=theme;
   const css=[...document.styleSheets].flatMap(s=>[...s.cssRules].map(r=>r.cssText)).join('\n');
   const tokens=[...new Set([...css.matchAll(/var\((--wattson-[\w-]+)/g)].map(m=>m[1]))];
   const style=getComputedStyle(document.documentElement);return tokens.filter(t=>!style.getPropertyValue(t).trim());
  },theme);assert.deepEqual(missing,[],'jetons définis pour '+theme);
 }
 checks.push('jetons CSS résolus dans les deux thèmes rendus');
 const totals={status:errors.length||external.length?'FAIL':'PASS',checks,errors,external,refresh_ms:{median:times.sort((a,b)=>a-b)[15],p95:times[28]},scope:'UI réelle avec fournisseur factice, pas de modèle IA'};
 fs.writeFileSync(path.join(outDir,'pilotage.json'),JSON.stringify(totals,null,2));console.log(JSON.stringify(totals,null,2));
 await browser.close();
 const agents=cli('agent','list',dense.id).agents;for(const {agent}of agents)if(['running','starting','queued'].includes(agent.status))cli('agent','stop',agent.id);
 server.kill();process.exit(totals.status==='PASS'?0:1);
})().catch(async e=>{console.error(e.stack);if(browser){const pages=await browser.pages();await pages.at(-1).screenshot({path:path.join(outDir,'failure.png'),fullPage:true}).catch(()=>{});await browser.close()}server.kill();process.exit(1)});
