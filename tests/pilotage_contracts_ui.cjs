'use strict';
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto');
const {spawn,execFileSync}=require('node:child_process'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-contracts-')),checks=[],errors=[];
const fingerprint=crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex');
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined}));
cli(['init']);const work=cli(['work','create'],{schema_version:1,event_id:crypto.randomUUID(),expected_revision:0,title:'Contrats pilotage',objective:'Recette isolée',scope:'Fixtures uniquement',criteria:['Identités exactes'],next:'Examiner'}).work.id;cli(['autonomy',work,'manuel']);
const host=os.hostname()+':'+fs.readFileSync('/proc/sys/kernel/random/boot_id','utf8').trim()+':'+fs.readlinkSync('/proc/self/ns/pid');
const task=(id,depends=[],status='todo')=>({id,title:'Mission '+id,depends,status,deliverable:'docs/'+id+'.md',criteria:['Preuve de '+id],owner:'recette',next:'Examiner',blocker:status==='blocked'?'Blocage de recette':''});
const initialTasks=[task('A'),task('B'),task('S',['A','B'],'running'),task('L',['S'],'submitted'),task('U',[],'running')];
const agent=(id,task_id)=>({id,work_id:work,task_id,attempt_id:'attempt-'+id,provider:'fixture',role:'worker',workspace:path.join(root,id),status:'running',host,heartbeat:new Date(Date.now()-3600000).toISOString(),started:new Date(Date.now()-3600000).toISOString(),limits:{silence_seconds:30},progress:{last_result_at:new Date(Date.now()-3600000).toISOString()}});
function patch(data){execFileSync('python3',['-c',`import json,sqlite3,sys
from pathlib import Path
r=Path(sys.argv[1]);assert r.name.startswith('swarm-contracts-')
c=sqlite3.connect(r/'.swarm/state.db');wid=sys.argv[2];d=json.loads(sys.argv[3])
if 'tasks' in d:
 w=json.loads(c.execute('SELECT body FROM works WHERE id=?',(wid,)).fetchone()[0]);w['tasks']=d['tasks'];w['revision']+=1;c.execute('UPDATE works SET revision=?,body=? WHERE id=?',(w['revision'],json.dumps(w),wid))
for a in d.get('agents',[]):
 c.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET status=excluded.status,body=excluded.body',(a['id'],wid,a['task_id'],a['workspace'],a['status'],json.dumps(a),'{}'))
if 'delete_decision' in d:c.execute('DELETE FROM decisions WHERE work_id=? AND id=?',(wid,d['delete_decision']))
for dec in d.get('decisions',[]):c.execute('INSERT INTO decisions(id,work_id,body) VALUES(?,?,?)',(dec['id'],wid,json.dumps(dec)))
c.commit()`,root,work,JSON.stringify(data)])}
const running=agent('live-S','S');running.heartbeat=new Date(Date.now()+60000).toISOString(); // stable synthetic live observation while UI scenarios run
const unknown=agent('lost-U','U');patch({tasks:initialTasks,agents:[running,unknown]});
let browser,page;const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
async function select(id,value){const index=await page.$eval(id,(e,v)=>[...e.options].findIndex(o=>o.value===v),value);assert.ok(index>=0);await page.focus(id);await page.keyboard.press('Home');for(let i=0;i<index;i++)await page.keyboard.press('ArrowDown');await page.keyboard.press('Enter')}
const refresh=()=>page.evaluate(()=>refresh(true));
const ids=()=>page.$$eval('.graph-noeud',ns=>ns.map(n=>n.dataset.task).sort());
const fold=async id=>{await page.$eval('.graph-fold[data-task="'+id+'"]',e=>e.focus());await page.keyboard.press('Enter')};
const counts=()=>page.$eval('#conduite-accueil .pilot-summary',e=>e.textContent);
(async()=>{
 const url=await new Promise((resolve,reject)=>{let s='';const timer=setTimeout(()=>reject(Error('No URL')),15000);server.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}})});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});page=await browser.newPage();page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1440,height:1000});await page.goto(url);await page.waitForSelector('#pilot-view');
 // Every operational summary segment has a tested destination.
 assert.match(await counts(),/1 agent démarré/);assert.match(await counts(),/1 tentatives sans signal confirmé/);assert.match(await counts(),/1 résultats à examiner/);
 for(const [key,expected]of [['active',['live-S']],['unknown',['lost-U']],['review',['L']]]){
  await page.click('[data-summary-key="'+key+'"]');assert.equal(await page.$eval('#pilot-filter',e=>e.value),key);assert.deepEqual(await page.$$eval('.pilot-card-title',ns=>ns.map(n=>n.dataset.pilotIdentity).sort()),expected);
 }
 const interventionCount=await page.evaluate(()=>Pilot.interventions().length);await page.click('[data-summary-key="interventions"]');await page.waitForSelector('#pilot-inspector[open]');assert.equal(await page.evaluate(()=>Pilot.queue.ids.length),interventionCount);await page.click('[data-inspector-action="return"]');
 await select('#pilot-filter','all');const initialCounts=await counts();await select('#pilot-view','dependencies');await fold('A');await fold('B');assert.equal(await counts(),initialCounts);
 await select('#pilot-filter','unknown');assert.equal(await counts(),initialCounts);await select('#pilot-filter','all');await page.click('#pilot-expand');
 checks.push('UX01 : chaque segment opérationnel ouvre sa cible exacte ; compteurs invariants au repli et aux filtres');
 for(const theme of ['etat','sombre']){
  await page.evaluate(t=>setTheme(t),theme);await page.click('#pilot-expand');await fold('A');assert.deepEqual(await ids(),['A','B','L','S','U']);
  assert.match(await page.$eval('.graph-fold[data-task="A"]',e=>e.textContent),/visibles ailleurs/);
  await fold('B');assert.deepEqual(await ids(),['A','B','U']);assert.match(await page.$eval('.graph-fold[data-task="B"]',e=>e.textContent),/2 tâches · 1 agents · 1 alertes/);
  patch({tasks:[...initialTasks,task('N',['L'],'blocked'),task('R')]});await refresh();assert.deepEqual(await ids(),['A','B','R','U']);
  assert.match(await page.$eval('.graph-fold[data-task="B"]',e=>e.textContent),/3 tâches · 1 agents · 2 alertes/);
  assert.equal(await page.evaluate(()=>document.activeElement?.dataset.task),'B');
  patch({tasks:initialTasks});await refresh();assert.deepEqual(await ids(),['A','B','U']);assert.match(await page.$eval('.graph-fold[data-task="B"]',e=>e.textContent),/2 tâches · 1 agents · 1 alertes/);
  assert.equal(await page.evaluate(()=>document.activeElement?.dataset.task),'B');await page.screenshot({path:path.join(out,'topologie-'+theme+'.png'),fullPage:true});
 }
 checks.push('AG05 : racines multiples, descendant partagé, ajout/suppression réels de topologie et recomptage tâches/agents/alertes, focus maintenu aux deux thèmes');
 // Remove an actual stored decision after it enters a frozen queue.
 patch({decisions:[{id:'deleted-during-visit',task_id:'A',kind:'execution',summary:'Demande de recette',evidence:'Décision supprimée pendant lecture',created:'2000-01-01T00:00:00Z'}]});await refresh();await page.click('#pilot-next');await page.waitForFunction(()=>Pilot.state.selection.id==='deleted-during-visit');
 const order=await page.evaluate(()=>Pilot.queue.ids);patch({delete_decision:'deleted-during-visit'});await refresh();
 assert.equal(await page.evaluate(()=>Pilot.state.selection.id),'deleted-during-visit');assert.deepEqual(await page.evaluate(()=>Pilot.queue.ids),order);assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/supprimé|indisponible/);
 assert.equal(await page.$('[data-inspector-action="decision"]'),null);await page.click('[data-inspector-action="next"]');assert.notEqual(await page.evaluate(()=>Pilot.state.selection.id),'deleted-during-visit');await page.click('[data-inspector-action="return"]');
 checks.push('UX03 : décision canonique supprimée pendant file, ordre/identité conservés, commande absente et navigation opérationnelle');
 await select('#pilot-view','agents');await select('#pilot-filter','all');await page.click('[data-pilot-identity="lost-U"]');await page.waitForSelector('#pilot-inspector[open]');assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Signal absent/);
 unknown.heartbeat=new Date().toISOString();patch({agents:[unknown]});await refresh();
 assert.equal(await page.evaluate(()=>snapshot.pilotage.health['lost-U'].process_state),'running');assert.equal(await page.evaluate(()=>snapshot.pilotage.health['lost-U'].activity_state),'old');assert.match(await counts(),/2 agents démarrés/);assert.doesNotMatch(await counts(),/sans signal confirmé/);
 assert.match(await page.$eval('[data-pilot-identity="lost-U"]',e=>e.closest('article').textContent),/Agent démarré/);assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Agent démarré/);assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Dernier résultat ancien/);
 assert.equal(await page.evaluate(()=>snapshot.work.tasks.find(t=>t.id==='U').status),'running');assert.equal(await page.evaluate(()=>Pilot.state.selection.id),'lost-U');
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.screenshot({path:path.join(out,'signal-retabli-'+theme+'.png'),fullPage:true})}
 assert.match(await page.$eval('#conduite-inbox',e=>e.textContent),/Signal rétabli — incident à examiner/);assert.match(await page.$eval('#decisions-list',e=>e.textContent),/Signal rétabli — incident à examiner/);
 await page.evaluate(()=>openDecision(snapshot.decisions.find(d=>d.kind==='silence')));assert.match(await page.$eval('#modal',e=>e.textContent),/Un signal récent est de nouveau reçu/);await page.evaluate(()=>document.getElementById('modal').close());
 checks.push('UX02 : heartbeat rétabli côté serveur actualise résumé/carte/panneau sans transformer activité ancienne en progrès ni tâche en validation');
 // Typed parent/retry relations remain separate from shared task prerequisites.
 await page.click('.pilot-inspector-head button');
 const previous=agent('previous-S','S'),parent=agent('parent-A','A');previous.status='completed';parent.status='completed';running.previous=previous.id;running.parent=parent.id;
 patch({agents:[running,previous,parent]});await refresh();await page.click('[data-pilot-identity="live-S"]');await page.waitForSelector('#pilot-relations');
 const links=await page.$$eval('#pilot-relations [data-relation]',ns=>ns.map(n=>({kind:n.dataset.relation,source:n.dataset.sourceAgent,target:n.dataset.targetAgent})));
 assert.deepEqual(links,[{kind:'parent',source:'parent-A',target:'live-S'},{kind:'previous',source:'previous-S',target:'live-S'}]);
 assert.deepEqual(await page.evaluate(()=>snapshot.pilotage.tasks.S.waiting_on),['A','B']);assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Prérequis manquants : A, B/);
 assert.deepEqual(await page.evaluate(()=>PilotGraph.descendants(snapshot.work.tasks,'A')),['L','S']);
 for(const theme of ['etat','sombre']){
  await page.evaluate(t=>setTheme(t),theme);await page.$eval('#pilot-relations',e=>e.scrollIntoView({block:'center'}));await (await page.$('#pilot-relations')).screenshot({path:path.join(out,'relations-'+theme+'.png')});
  assert.deepEqual(await page.evaluate(()=>{const css=[...document.styleSheets].flatMap(s=>[...s.cssRules].map(r=>r.cssText)).join('');const tokens=[...new Set([...css.matchAll(/var\((--wattson-[\w-]+)/g)].map(m=>m[1]))],style=getComputedStyle(document.documentElement);return tokens.filter(t=>!style.getPropertyValue(t).trim())}),[]);
 }
 await page.click('[data-inspector-action="Tentative précédente"]');await page.waitForFunction(()=>Pilot.state.selection.id==='previous-S');assert.equal(await page.evaluate(()=>Pilot.state.selection.id),'previous-S');
 await page.evaluate(()=>Pilot.inspect('agent','live-S'));await page.waitForSelector('#pilot-relations');await page.click('[data-inspector-action="Agent parent"]');await page.waitForFunction(()=>Pilot.state.selection.id==='parent-A');assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Mission A/);
 checks.push('AG06 : filiation et reprise avec tracés, légende, sources et cibles exactes ; prérequis partagés et descendants uniques ; jetons résolus aux deux thèmes');
 await page.evaluate(()=>Pilot.inspect('agent','live-S'));await page.waitForSelector('#pilot-reports [data-inspector-action="gate"]');await page.evaluate(()=>PilotInspector.readReport('/rapport-absent-recette.md'));await page.waitForSelector('.pilot-report-error');
 assert.match(await page.$eval('.pilot-report-error',e=>e.textContent),/Vérifiez sa disponibilité/);assert.doesNotMatch(await page.$eval('.pilot-report-error',e=>e.textContent),/lstat|no such file/);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.$eval('.pilot-report-error',e=>e.scrollIntoView({block:'center'}));await (await page.$('#pilot-inspector')).screenshot({path:path.join(out,'rapport-erreur-'+theme+'.png')})}
 checks.push('Erreur rapport locale : message opérationnel près du lien, sans erreur système brute, deux thèmes');
 assert.deepEqual(errors,[]);assert.equal(crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex'),fingerprint);
 const result={status:'PASS',at:new Date().toISOString(),binary,sha256:fingerprint,fixture_root:root,checks,errors,scope:'Base et sessions exclusivement factices ; aucun fournisseur lancé ; mutations SQLite limitées au corpus temporaire'};
 fs.writeFileSync(path.join(out,'contracts.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
})().catch(async e=>{console.error(e.stack);console.error(JSON.stringify({checks,errors}));await page?.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
