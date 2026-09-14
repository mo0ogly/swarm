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
(async()=>{
 const url=await new Promise((resolve,reject)=>{let out='';const timer=setTimeout(()=>reject(new Error('serveur non démarré : '+out)),15000);server.stdout.on('data',d=>{out+=d;const m=out.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m){clearTimeout(timer);resolve(m[0])}});server.on('exit',c=>reject(new Error('serveur arrêté '+c)))});
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const page=await browser.newPage();
 page.setDefaultTimeout(20000);
 page.on('pageerror',e=>errors.push(e.message));
 page.on('response',r=>{if(r.status()>=400&&!r.url().endsWith('/favicon.ico'))errors.push('HTTP '+r.status()+' '+r.url())});
 page.on('request',r=>{const u=new URL(r.url());if(!['localhost','127.0.0.1'].includes(u.hostname)&&u.protocol!=='data:')external.push(r.url())});
 await page.setViewport({width:1500,height:1000});
 await page.goto(url);
 await page.waitForSelector('#conduite:not([hidden])');

 const choisir=async id=>{await page.select('#work',id);await page.waitForFunction(t=>document.getElementById('title').textContent===t,{},{ 'x':0 }.x===0?(await page.evaluate(()=>null),undefined):undefined)};
 // Sélection par identifiant, puis attente du rendu correspondant.
 const ouvrir=async(id,titre)=>{await page.select('#work',id);await page.waitForFunction(t=>document.getElementById('title').textContent===t,{},titre)};

 await ouvrir(dense.id,'Plan dense');
 await page.waitForFunction(()=>document.querySelectorAll('.graph-noeud').length===6);
 const aretes=await page.$$eval('.graph-arete',es=>es.length);
 assert.equal(aretes,6,'arêtes de dépendance attendues : 6, obtenues '+aretes);
 checks.push('plan dense : 6 nœuds et 6 arêtes de dépendance');

 // Le superviseur persiste l'activité au tic suivant : attendre l'action, pas
 // seulement la présence du compteur.
 await page.waitForFunction(()=>[...document.querySelectorAll('.graph-noeud')].some(n=>/CPU|probe|Bash/.test(n.textContent)),{timeout:25000});
 const g1=(await noeuds(page)).find(t=>t.startsWith('g1'));
 assert.match(g1,/En cours/,'état de la tâche absent : '+g1);
 assert.match(g1,/Mesurer la consommation CPU|probe/,'action courante absente : '+g1);
 assert.match(g1,/appels/,'compteur d’appels absent : '+g1);
 // Le mot « coût » ne prouve rien : exiger une valeur, sinon la mention
 // explicite d'absence. Un libellé qui dit toujours « non rapporté » parce
 // qu'il lit un champ inexistant passerait un test sur le seul mot.
 assert.match(g1,/coût rapporté : \d+\.\d{2} USD|coût : non rapporté/,'coût illisible : '+g1);
 checks.push('tentative vivante : action en français, appels et coût');

 // Fraîcheur du signal. Ce bloc de graph.js est devenu inatteignable une fois,
 // sans qu'aucun contrôle ne s'en aperçoive : un nœud muet avait l'air normal.
 // Sur une tentative vivante, la ligne attendue est « dernier résultat » ; les
 // deux autres formes viennent du même bloc, donc l'assertion les couvre toutes
 // en portée. Ce qu'elle ne couvre pas : le calcul du délai de silence
 // lui-même, qui demanderait d'attendre l'expiration de la limite.
 // Tant qu'aucun résultat d'outil n'est revenu, le nœud n'a rien à dire de sa
 // fraîcheur et se tait à juste titre : attendre le premier résultat.
 await page.waitForFunction(()=>[...document.querySelectorAll('.graph-noeud')]
   .some(n=>/· [1-9]\d* résultats/.test(n.textContent)),{timeout:25000});
 const g1frais=(await noeuds(page)).find(t=>t.startsWith('g1'));
 assert.match(g1frais,/dernier résultat : \d{2}:\d{2}|signal perdu depuis \d+ s|signal jamais reçu/,
   'fraîcheur du signal absente du nœud : '+g1frais);
 checks.push('fraîcheur du signal portée par le nœud');
 await page.screenshot({path:path.join(outDir,'graphe-dense-etat.png'),fullPage:true});
 await page.click('#theme');await new Promise(r=>setTimeout(r,150));
 await page.screenshot({path:path.join(outDir,'graphe-dense-sombre.png'),fullPage:true});
 await page.click('#theme');
 checks.push('graphe rendu dans les deux thèmes');

 // Clavier : un nœud est atteignable et ouvre le dialogue de sa tâche.
 await page.$eval('.graph-noeud',n=>n.focus());
 await page.keyboard.press('Enter');
 await page.waitForSelector('#modal[open] #field-action',{visible:true});
 const titre=await page.$eval('#modal-title',e=>e.textContent);
 assert.match(titre,/^g\d/,'le nœud doit ouvrir sa propre tâche : '+titre);
 await page.click('#cancel');
 checks.push('nœud atteignable au clavier et lié à sa tâche');

 await ouvrir(vide.id,'Travail vide');
 await page.waitForFunction(()=>/graphe apparaîtra/.test(document.getElementById('graph').textContent));
 assert.equal((await noeuds(page)).length,0);
 checks.push('travail vide : message d’orientation, aucun nœud');

 await ouvrir(echec.id,'Travail en échec');
 await page.waitForFunction(()=>document.querySelectorAll('.graph-noeud').length===1);
 const etat=await page.$eval('.graph-noeud',n=>n.dataset.etat);
 assert.equal(etat,'alerte','une tâche bloquée doit se distinguer : '+etat);
 await page.screenshot({path:path.join(outDir,'graphe-echec.png'),fullPage:true});
 checks.push('tâche bloquée distinguée dans le graphe');

 // Le cumul de coût de la tâche doit être une valeur ou une absence déclarée.
 // Un libellé constant qui dit toujours « non rapporté » parce qu'il lit un
 // champ inexistant passerait un test portant sur le seul mot « coût ».
 await ouvrir(dense.id,'Plan dense');
 await page.waitForFunction(()=>document.querySelectorAll('.graph-noeud').length===6);
 const textes=(await noeuds(page)).join(' | ');
 assert.match(textes,/coût de la tâche : (\d+\.\d{2} USD|non rapporté)/,'cumul de coût illisible : '+textes);
 assert.ok(!/coût de la tâche : 0\.00/.test(textes),'un coût inconnu ne doit pas s’afficher 0,00 : '+textes);
 checks.push('cumul de coût par tâche : valeur ou absence déclarée, jamais 0,00');

 const anime=await page.evaluate(()=>[...document.querySelectorAll('#graph *')].some(e=>{const s=getComputedStyle(e);return s.animationName!=='none'||s.transitionDuration!=='0s'}));
 assert.equal(anime,false,'aucune animation ne doit masquer une alerte');
 checks.push('aucune animation dans le graphe');

 await browser.close();
 const bilan={status:errors.length||external.length?'FAIL':'PASS',checks,errors,external};
 fs.writeFileSync(path.join(outDir,'graph.json'),JSON.stringify(bilan,null,1));
 console.log(JSON.stringify(bilan,null,1));
 server.kill();process.exit(bilan.status==='PASS'?0:1);
})().catch(e=>{console.error(String(e));server.kill();process.exit(1)});
