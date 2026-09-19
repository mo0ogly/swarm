// Recette du graphe vivant : travail vide, plan dense, tâche en échec, et une
// tentative en cours dont l'action est lisible. Aucun modèle appelé.
'use strict';
const assert=require('node:assert/strict');
const fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn,spawnSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]),outDir=path.resolve(process.argv[3]);
fs.mkdirSync(outDir,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-graph-'));
const checks=[],errors=[],external=[];
// Some restricted runners report EPERM after a successful synchronous child
// (status 0 and complete output). Accept only that exact contradiction; every
// real command failure still aborts the recipe.
const run=(args,input)=>{const r=spawnSync(binary,args,{encoding:'utf8',input});if(r.status!==0||r.signal||r.error&&r.error.code!=='EPERM')throw Object.assign(r.error||new Error(r.stderr||'commande échouée'),{result:r});return r.stdout};
const cli=(...args)=>{const r=run(['--root',root,'--json',...args].filter(x=>x!==undefined),args.input);return r.startsWith('{')||r.startsWith('[')?JSON.parse(r):null};
const send=(args,data)=>JSON.parse(run(['--root',root,'--json',...args,'--input','-'],JSON.stringify(data)));
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

let server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
(async()=>{
 const url=await new Promise((resolve,reject)=>{let text='',stderr='',settled=false;const finish=(err,value)=>{if(settled)return;settled=true;clearTimeout(timer);err?reject(err):resolve(value)};const timer=setTimeout(()=>finish(new Error('serveur web muet pendant 10 s'+(stderr?' : '+stderr.trim():''))),10000);server.stderr.on('data',d=>stderr+=d);server.on('error',finish);server.on('close',(code,signal)=>finish(new Error('serveur web arrêté avant son URL : code '+code+' signal '+signal+(stderr?' : '+stderr.trim():''))));server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(m)finish(null,m[0])})});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome'});const page=await browser.newPage();let assistCalls=0,missionStarts=0;page.on('request',request=>{if(request.method()==='POST'&&request.url().includes('/api/v1/assist/ask')){try{if(!JSON.parse(request.postData()||'{}').preview)assistCalls++}catch{assistCalls++}}if(request.method()==='POST'&&request.postData()?.includes('"kind":"mission-start"'))missionStarts++});page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(20000);await page.setViewport({width:1440,height:1100});await page.goto(url);await page.waitForSelector('#pilot-mission');await page.select('#work',dense.id);await page.waitForFunction(()=>document.getElementById('title').textContent==='Plan dense');
 assert.equal(cli('mission','status',dense.id).enabled,false);
 assert.equal(await page.$eval('#mission-primary',e=>e.textContent),'Préparer le lancement');
 assert.equal(await page.$$eval('#mission-summary > .mission-intervention',es=>es.length),0);
 assert.match(await page.$eval('#mission-link',e=>e.href),new RegExp('/session/[^?]+\\?work='+dense.id));
 assert.deepEqual(await page.$$eval('.mission-brief dt',es=>es.map(e=>e.textContent)),['Ce qui se passe','Prochaine étape','Qui agit']);
 assert.equal(await page.$eval('.mission-brief [data-actor]',e=>e.dataset.actor),'user');
 const taskDetails=await page.$('#mission-results>summary');await taskDetails.focus();await page.keyboard.press('Enter');await page.waitForFunction(()=>document.querySelector('#mission-results').open);
 assert.deepEqual(await page.$$eval('#mission-results .mission-task:first-of-type .mission-understanding dt',es=>es.map(e=>e.textContent)),['Ce qui se passe','Prochaine étape','Qui agit']);
 assert.equal(await page.$eval('#mission-results .mission-task:first-of-type [data-actor]',e=>e.dataset.actor),'user');
 await page.keyboard.press('Enter');await page.waitForFunction(()=>!document.querySelector('#mission-results').open);
 await page.focus('#mission-primary');await page.keyboard.press('Enter');await page.waitForSelector('#modal[open] #field-provider');assert.equal(assistCalls,0);await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.getElementById('modal').open);assert.equal(await page.evaluate(()=>document.activeElement?.id),'mission-primary');
 await page.keyboard.press('Enter');await page.waitForSelector('#modal[open] #field-provider');await page.focus('#field-provider');await page.keyboard.press('End');await page.keyboard.press('Enter');await page.waitForFunction(()=>!document.getElementById('confirm').disabled);await page.waitForFunction(()=>document.querySelector('#mission-launch-preview h3')?.textContent==='Aperçu du moteur');
 const launchPreview=await page.$eval('#mission-launch-preview',e=>e.textContent);assert.match(launchPreview,/1 départ/);assert.match(launchPreview,/concurrence réelle 1\/2/);assert.match(launchPreview,/Mode séquentiel/);assert.match(launchPreview,/un écrivain/);assert.match(launchPreview,/espace partagé|espace de travail/);assert.match(launchPreview,/Limites conservées/);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.screenshot({path:path.join(outDir,'config-'+theme+'.png'),fullPage:true})}
 await page.click('#confirm',{clickCount:2,delay:20});await page.waitForFunction(()=>!document.getElementById('modal').open);assert.equal(missionStarts,1);await page.waitForFunction(()=>document.querySelector('#mission-summary').textContent.includes('Résultat à vérifier'));
 const cliResult=cli('mission','status',dense.id).tasks.find(t=>t.result.state==='result_to_review')?.result;
 const webResult=await page.evaluate(()=>snapshot.mission.tasks.find(t=>t.result.state==='result_to_review')?.result);
 assert.ok(cliResult);assert.deepEqual(webResult,cliResult);
 assert.equal(await page.$eval('.mission-result[data-result-state="result_to_review"] h5',e=>e.textContent),'Résultat à vérifier');
 assert.deepEqual(await page.$$eval('.mission-result[data-result-state="result_to_review"] dt',es=>es.slice(0,3).map(e=>e.textContent)),['Processus','Rapport','Validation']);
 assert.equal(cli('mission','status',dense.id).enabled,true);
 await page.waitForFunction(()=>snapshot.agents.length>=2);
 assert.equal(cli('agent','list',dense.id).agents.filter(x=>x.agent.task_id==='g3').length,0);
 const before=cli('agent','list',dense.id).agents.length;
 await page.click('#pilot-mission');await page.waitForSelector('#modal[open]');
 assert.match(await page.$eval('#modal',e=>e.textContent),/Réglages de la mission/);
 assert.equal(await page.$eval('#confirm',e=>e.textContent),'Enregistrer et poursuivre');
 assert.ok(await page.$('#field-provider'));
 assert.doesNotMatch(await page.$eval('#modal',e=>e.textContent),/relancez ce bouton/);
 assert.equal(cli('agent','list',dense.id).agents.length,before);
 await page.click('#cancel');

 await page.click('[data-mission-action=control]');await page.waitForFunction(()=>document.querySelector('#mission-summary h3').textContent==='Mission en pause');assert.equal(cli('mission','status',dense.id).paused,true);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);const missing=await page.evaluate(()=>{const css=[...document.styleSheets].flatMap(s=>[...s.cssRules].map(r=>r.cssText)).join(' ');const tokens=[...new Set([...css.matchAll(/var\((--wattson-[\w-]+)/g)].map(m=>m[1]))];const style=getComputedStyle(document.documentElement);return tokens.filter(t=>!style.getPropertyValue(t).trim())});assert.deepEqual(missing,[]);await page.screenshot({path:path.join(outDir,'results-'+theme+'.png'),fullPage:true})}
 assert.equal(await page.$eval('#mission-primary',e=>e.textContent),'Examiner le résultat');
 await page.click('#mission-primary');await page.waitForSelector('#modal[open]');assert.match(await page.$eval('#modal',e=>e.textContent),/rapport|Rapport/);await page.keyboard.press('Escape');
 await page.waitForFunction(()=>!document.getElementById('modal').open);
 await page.waitForFunction(()=>document.querySelector('[data-mission-action=control]').textContent==='Reprendre la mission');
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.screenshot({path:path.join(outDir,'suivi-'+theme+'.png'),fullPage:true})}
 assert.ok(await page.$('[data-mission-action=control]'));await page.locator('[data-mission-action=control]').click();await page.waitForFunction(()=>!snapshot.mission.paused).catch(async e=>{console.error(await page.$eval('#message',e=>e.textContent));throw e});assert.equal(cli('mission','status',dense.id).paused,false);assert.ok(await page.$$eval('.graph-arete[marker-end]',es=>es.length)>0);assert.equal(await page.$eval('#pilot-canvas',e=>e.getBoundingClientRect().height>0),true);
 await page.reload();await page.waitForFunction(()=>snapshot?.mission.enabled===true);assert.deepEqual(errors,[]);
 // A real crashed conductor must expire without losing authorization, and
 // a replacement must retain existing attempts instead of launching duplicates.
 const agentsBeforeCrash=cli('agent','list',dense.id).agents.map(x=>x.agent.id).sort();
 server.kill('SIGKILL');await new Promise(resolve=>server.once('close',resolve));
 const expiryDeadline=Date.now()+40000;let absent;
 do{absent=cli('mission','status',dense.id);if(absent.supervision.state==='absent')break;await new Promise(resolve=>setTimeout(resolve,250))}while(Date.now()<expiryDeadline);
assert.equal(absent.authorized,true);assert.equal(absent.enabled,false);assert.equal(absent.supervision.state,'absent');
 server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
 const restartedURL=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(Error('redémarrage sans URL')),10000);server.once('error',reject);server.stdout.on('data',d=>{text+=d;const match=text.match(/http:\/\/[^\s]+\/session\/[^\s]+/);if(match){clearTimeout(timer);resolve(match[0])}})});
 await page.goto(restartedURL+'?work='+dense.id);await page.waitForFunction(()=>snapshot?.mission.enabled===true);
 const restarted=cli('mission','status',dense.id),webVerdict=await page.evaluate(()=>({enabled:snapshot.mission.enabled,state:snapshot.mission.supervision.state}));
 assert.equal(restarted.enabled,webVerdict.enabled);assert.equal(restarted.supervision.state,webVerdict.state);
 assert.deepEqual(cli('agent','list',dense.id).agents.map(x=>x.agent.id).sort(),agentsBeforeCrash);
 const result={status:'PASS',checks:['arrêt brutal réel : bail expiré et autorisation conservée','redémarrage réel : aucun départ doublé, verdict CLI/web concordant','triplet factuel mission et tâche, acteur dérivé du moteur','navigation clavier du détail factuel','absence de profil présentée comme préparation','configuration commune sans appel IA','clavier, Échap et restitution du focus','aperçu issu du moteur et concurrence réelle','mode commun annoncé séquentiel et écrivain unique','double clic sans lancement doublé','autorisation explicite depuis le web','configuration unique','espace commun libéré : deuxième départ automatique','dépendance non validée retenue','processus factice et résultat à examiner','pause et reprise effectives moteur','accès direct à la revue','persistance au rechargement','flèches conservées','graphe visible','deux thèmes et jetons définis'],errors};fs.writeFileSync(path.join(outDir,'ui.json'),JSON.stringify(result,null,2));console.log(result);
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{cli('mission','stop',dense.id);if(browser)await browser.close();for(const {agent}of cli('agent','list',dense.id).agents)if(['running','starting','queued'].includes(agent.status))cli('agent','stop',agent.id);server.kill()});
