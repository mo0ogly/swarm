'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const fixture=JSON.parse(execFileSync('python3',[path.join(__dirname,'pilotage_fixture.py'),binary],{encoding:'utf8'}));
const {root,works}=fixture,checks=[],errors=[],measures=[];
const cli=(...args)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args],{encoding:'utf8'}));
const update=(work,id,fields)=>JSON.parse(execFileSync(binary,['--root',root,'--json','task','update',work,'--input','-'],{encoding:'utf8',input:JSON.stringify({schema_version:1,event_id:require('crypto').randomUUID(),expected_revision:cli('work','show',work).work.revision,id,...fields})}));
let browser,page;const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});
async function select(p,id,value){const index=await p.$eval(id,(e,v)=>[...e.options].findIndex(o=>o.value===v),value);assert.ok(index>=0);await p.focus(id);await p.keyboard.press('Home');for(let i=0;i<index;i++)await p.keyboard.press('ArrowDown');await p.keyboard.press('Enter')}
async function choose(p,size){await p.select('#work',works[size]);await p.waitForFunction(id=>snapshot?.work.id===id,{},works[size])}
async function capture(name){for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.screenshot({path:path.join(out,name+'-'+theme+'.png'),fullPage:true});if(name==='rapport-manquant')await page.$('#pilot-inspector').then(el=>el.screenshot({path:path.join(out,name+'-detail-'+theme+'.png')}))}}
async function close(p){await p.click('.pilot-inspector-head button');await p.waitForFunction(()=>!document.getElementById('pilot-inspector').open)}
(async()=>{
 const url=await new Promise((resolve,reject)=>{let text='';server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.once('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 page=await browser.newPage();page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1440,height:1000});await page.goto(url);await page.waitForSelector('#pilot-view');
 for(const size of ['0','1','12','50','200']){
  console.log('Corpus '+size);
  const started=Date.now();await choose(page,size);await select(page,'#pilot-view','dependencies');await page.waitForFunction(n=>document.querySelectorAll('.graph-noeud').length===n,{},Number(size));
  const first=Date.now()-started,selection=[],refreshes=[];
  if(Number(size))for(let i=0;i<30;i++){
   const t=await page.evaluate(async()=>{const start=performance.now();Pilot.inspect('task','t0');await new Promise(requestAnimationFrame);return performance.now()-start});selection.push(t);
   refreshes.push(await page.evaluate(async()=>{const start=performance.now();await refresh(true);return performance.now()-start}));
  }
  const idleStart=await page.metrics();await new Promise(r=>setTimeout(r,1000));const idleEnd=await page.metrics();
  const metrics=await page.metrics();const percentile=a=>a.sort((a,b)=>a-b)[Math.floor(a.length*.95)]||0;
  measures.push({nodes:Number(size),first_ms:first,selection_p95_ms:percentile(selection),refresh_p95_ms:percentile(refreshes),heap_bytes:metrics.JSHeapUsedSize,idle_main_thread_percent:100*(idleEnd.TaskDuration-idleStart.TaskDuration)/(idleEnd.Timestamp-idleStart.Timestamp)});
  if(size==='200'){
   await page.$eval('.graph-noeud[data-task="t0"]',e=>e.focus());
   await page.evaluate(()=>{document.getElementById('pilot-canvas').scrollTo(200,100);Pilot.save();window.stableFocus=document.activeElement});
   const saved=await page.evaluate(()=>({x:Pilot.state.x,y:Pilot.state.y,selection:Pilot.state.selection}));
   for(let i=0;i<30;i++){
    execFileSync('python3',['-c',`import json,sqlite3,sys
from pathlib import Path
r=Path(sys.argv[1]);assert r.name.startswith('swarm-pilotage-corpus-')
c=sqlite3.connect(r/'.swarm/state.db');a=json.loads(c.execute("SELECT body FROM agents WHERE id='intention-0'").fetchone()[0]);a['progress']['detail']='Étape '+sys.argv[2];a['progress']['tool_calls']=int(sys.argv[2]);c.execute("UPDATE agents SET body=? WHERE id='intention-0'",(json.dumps(a),));c.commit()`,root,String(i)]);
    await page.evaluate(()=>refresh(true));
    assert.equal(await page.evaluate(()=>document.activeElement===window.stableFocus&&window.stableFocus.isConnected),true);
   }
   assert.deepEqual(await page.evaluate(()=>({x:Pilot.state.x,y:Pilot.state.y,selection:Pilot.state.selection})),saved);
   assert.match(await page.$eval('.graph-noeud[data-task="t0"]',e=>e.textContent),/Étape 29/);
   checks.push('30 progressions distinctes reçues : focus, sélection et viewport inchangés');
   assert.match(await page.$eval('#conduite-accueil',e=>e.textContent),/16 tentatives sans signal confirmé/);await page.screenshot({path:path.join(out,'corpus-200.png'),fullPage:true})}
  if(size==='50')assert.ok(percentile(selection)<=100,JSON.stringify(measures.at(-1)));
  if(Number(size))await close(page);
 }
 checks.push('corpus 0/1/12/50/200 : première lecture, sélection et refresh mesurés ; p95 sélection 50 ≤100 ms');
 await choose(page,'50');await select(page,'#pilot-view','dependencies');await select(page,'#pilot-orientation','TB');
 await page.$eval('.graph-noeud[data-task="t4"]',e=>e.focus());await page.keyboard.press('Enter');
 await page.evaluate(()=>{document.getElementById('pilot-canvas').scrollTo(0,250);Pilot.save()});
 const original=await page.evaluate(()=>JSON.parse(JSON.stringify(Pilot.state)));
 update(works['50'],'t3',{status:'blocked',blocker:'Source externe absente'});await page.evaluate(()=>refresh(true));
 await page.click('#pilot-next');await page.waitForFunction(()=>Pilot.state.selection.id==='t3');
 update(works['50'],'t6',{status:'blocked',blocker:'Deuxième intervention'});await page.evaluate(()=>refresh(true));
 assert.equal(await page.evaluate(()=>Pilot.state.selection.id),'t3');assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/nouvelles interventions/);
 update(works['50'],'t3',{status:'todo',next:'Reprise explicite'});await page.evaluate(()=>refresh(true));assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/ne requiert plus/);
 await page.click('[data-inspector-action="return"]');
 assert.deepEqual(await page.evaluate(()=>Pilot.state.selection),original.selection);
 assert.equal(await page.$eval('#pilot-canvas',e=>e.scrollTop),original.y);
 checks.push('file stable face aux arrivées et résolutions, retour conserve sélection et position initiales');
 await close(page);
 const tab=await browser.newPage();await tab.goto(new URL(url).origin);await tab.waitForSelector('#pilot-view');await choose(tab,'50');
 await select(tab,'#pilot-orientation','LR');await tab.$eval('.graph-noeud[data-task="t1"]',e=>e.focus());await tab.keyboard.press('Enter');
 assert.equal(await page.$eval('#pilot-orientation',e=>e.value),'TB');assert.equal(await page.evaluate(()=>Pilot.state.selection),null);
 await tab.close();checks.push('deux onglets : préférences locales, aucun déplacement de sélection');
 await choose(page,'12');await select(page,'#pilot-view','agents');await page.click('[data-pilot-identity="archive-204"]');
 assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Coût de cette tentative : 0.23 USD/);
 await page.click('[data-inspector-action="Agent parent"]');await page.waitForFunction(()=>PilotInspector.loaded?.agent.id==='archive-001');assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Mission 1/);await close(page);await page.click('[data-pilot-identity="archive-204"]');
 await page.click('[data-inspector-action="Tentative précédente"]');await page.waitForFunction(()=>Pilot.state.selection.id==='archive-203');
 await page.click('[data-inspector-action="Tentative précédente"]');await page.waitForFunction(()=>PilotInspector.loaded?.agent.id==='archive-000');
 assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Exécution en échec/);
 assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Validation actuelle de la tâche/);
 const detail=await page.evaluate(async id=>{const r=await fetch('/api/v1/agent-detail?'+new URLSearchParams({work:id,agent:'archive-000'}));return r.status},works['1']);assert.equal(detail,403);
 checks.push('historique hors 200 : précédente exacte et appartenance au travail vérifiée');await close(page);
 await select(page,'#pilot-detail','detailed');
 await page.evaluate(()=>{snapshot.validation.tasks.t0.state='stale';snapshot.pilotage.health['archive-204'].activity_label='Dernier résultat ancien';Pilot.cards()});
 assert.equal(await page.$eval('[data-pilot-identity="archive-204"]',e=>e.closest('article').dataset.state),'stale');
 assert.match(await page.$eval('[data-pilot-identity="archive-204"]',e=>e.closest('article').textContent),/Dernier résultat ancien/);
 await page.evaluate(()=>refresh(true));checks.push('cartes réactualisées lorsque seules fraîcheur de preuve et activité changent');

 // Read the real fixture report without changing the task or selection.
 await page.click('[data-pilot-identity="archive-204"]');await page.waitForSelector('[data-inspector-action^="report:"]');
 const reportRevision=cli('work','show',works['12']).work.revision;
 await page.click('[data-inspector-action^="report:"]');await page.waitForSelector('#modal[open]');
 assert.match(await page.$eval('#preview',e=>e.textContent),/Conclusions de recette/);await page.click('#cancel');
 assert.equal(await page.evaluate(()=>Pilot.state.selection.id),'archive-204');assert.equal(cli('work','show',works['12']).work.revision,reportRevision);
 fs.renameSync(path.join(root,'docs/t0-handoff.md'),path.join(root,'docs/t0-handoff.hidden'));
 await page.click('[data-inspector-action^="report:"]');await page.waitForSelector('.pilot-report-error');await capture('rapport-manquant');
 assert.match(await page.$eval('.pilot-report-error',e=>e.textContent),/inaccessible/);await close(page);
 checks.push('rapport lu sans mutation, sélection conservée ; preuve disparue expliquée près du lien');
 // Exact archived active target: the request persists an intention, never a confirmed end.
 await page.click('[data-pilot-identity="orphan-0"]');await page.click('[data-inspector-action="actions"]');await page.waitForSelector('#field-action');await page.select('#field-action','stop');
 assert.equal(await page.$eval('#field-agent',e=>e.value),'orphan-0');await page.click('#confirm',{clickCount:2});await page.waitForFunction(()=>!document.getElementById('modal').open);await page.evaluate(()=>refresh(true));
 assert.equal(await page.evaluate(()=>snapshot.pilotage.health['orphan-0'].stop_requested),true);assert.equal(await page.evaluate(()=>snapshot.pilotage.health['orphan-1'].stop_requested),false);
 assert.match(await page.$eval('#pilot-inspector-body',e=>e.textContent),/Arrêt déjà demandé|confirmation attendue/);await close(page);
 checks.push('deux intentions anciennes distinctes des deux reprises terminées ; arrêt cible la session ancienne exacte');
 await page.click('[data-pilot-identity="orphan-0"]');await page.click('[data-inspector-action="actions"]');await page.waitForSelector('#field-action');await page.select('#field-action','reconcile');await page.click('#confirm');await page.waitForFunction(()=>!document.getElementById('modal-error').hidden);
 assert.equal(await page.evaluate(()=>snapshot.agents.find(x=>x.agent.id==='orphan-0').agent.status),'starting');await page.click('#cancel');await close(page);
 checks.push('réconciliation refusée sans inventer la fin de l’ancienne intention');
 await page.click('#pilot-group');await page.$eval('[data-pilot-identity="group:En activité"]',e=>e.focus());await page.keyboard.press('Enter');
 await page.waitForFunction(()=>Pilot.state.groups['En activité']===false);
 await page.evaluate(()=>{snapshot.pilotage.health['orphan-0'].activity_label='Activité actualisée';Pilot.cards()});
 assert.equal(await page.$eval('[data-pilot-identity="group:En activité"]',e=>e.parentNode.open),false);await page.click('#pilot-group');
 checks.push('groupes repliés conservés lors d’une actualisation');
 await page.$eval('#conduite-accueil',e=>{const details=e.closest('details');if(details&&!details.open)details.querySelector('summary').click()});await page.click('[data-summary-key="unknown"]');assert.equal(await page.$$eval('.pilot-card',ns=>ns.length),2);await select(page,'#pilot-filter','all');
 assert.match(await page.$eval('#conduite-state',e=>e.textContent),/2 créneau/);
 await page.evaluate(()=>{snapshot.pilotage.summary.complete=false;Pilot.summary(document.getElementById('conduite-accueil'))});assert.match(await page.$eval('#conduite-accueil',e=>e.textContent),/Nombre d’agents indisponible/);await page.evaluate(()=>refresh(true));
 checks.push('résumé : filtre des deux inconnus, créneaux occupés et agrégat incomplet explicite');
 // Delay an actual task response, then switch work before releasing it.
 let delayed,release;await page.setRequestInterception(true);
 page.on('request',async r=>{if(r.isInterceptResolutionHandled())return;if(!delayed&&r.url().includes('/api/v1/task?')){delayed=r;const response=await fetch(r.url(),{headers:r.headers()});const body=await response.text();release=()=>r.respond({status:response.status,contentType:'application/json',body});return}await r.continue().catch(e=>{if(!/Interception is not enabled|already handled/.test(e.message))throw e})});
 await page.click('[data-pilot-identity="archive-204"]');await page.waitForFunction(()=>document.getElementById('pilot-inspector').open);
 while(!release)await new Promise(r=>setTimeout(r,20));await choose(page,'1');await release();await page.setRequestInterception(false);page.removeAllListeners('request');
 assert.equal(await page.$eval('#pilot-inspector',e=>e.open),false);checks.push('rapport tardif d’un autre travail ignoré');
 fs.renameSync(path.join(root,'docs/t0-handoff.hidden'),path.join(root,'docs/t0-handoff.md'));
 for(const kind of ['report','gate']){
  await page.evaluate(()=>taskDialog('t0'));await page.waitForSelector('#field-action');await page.select('#field-action',kind);
  if(kind==='gate')await page.type('#field-name','Revue de recette');
  let finish;await page.setRequestInterception(true);
  page.on('request',async r=>{if(r.isInterceptResolutionHandled())return;const wanted=kind==='report'?r.url().includes('/api/v1/report?'):r.url().includes('/api/v1/action')&&JSON.parse(r.postData()||'{}').kind==='gate-preview';
   if(wanted&&!finish){const response=await fetch(r.url(),{method:r.method(),headers:r.headers(),body:r.postData()});const body=await response.text();finish=()=>r.respond({status:response.status,contentType:'application/json',body});return}
   await r.continue().catch(e=>{if(!/Interception is not enabled|already handled/.test(e.message))throw e});
  });
  await page.click('#confirm');for(let i=0;!finish&&i<100;i++)await new Promise(r=>setTimeout(r,20));assert.ok(finish);await capture('chargement-'+kind);
  await page.click('#cancel');await page.click('#pilot-help');
  const response=page.waitForResponse(r=>kind==='report'?r.url().includes('/api/v1/report?'):r.url().includes('/api/v1/action'));
  await finish();await (await response).text();await page.evaluate(()=>new Promise(requestAnimationFrame));
  assert.equal(await page.$eval('#modal-title',e=>e.textContent),'Piloter les agents');assert.match(await page.$eval('#preview',e=>e.textContent),/Agents :/);assert.equal(await page.$eval('#confirm',e=>e.hidden),true);
  await page.setRequestInterception(false);page.removeAllListeners('request');await page.click('#cancel');
 }
 checks.push('réponses tardives rapport et gate ne contaminent pas une nouvelle modale');
 // Lose HTTP response after a real task mutation commits. Reuse exact request.
 await page.evaluate(()=>taskDialog('t0'));await page.waitForSelector('#field-action');await page.select('#field-action','assign');await page.$eval('#field-owner',e=>e.value='propriétaire après perte HTTP');
 let committed=false,requestBody;await page.setRequestInterception(true);page.on('request',async r=>{if(r.isInterceptResolutionHandled())return;if(!committed&&r.url().includes('/api/v1/action')&&JSON.parse(r.postData()||'{}').kind==='task'){requestBody=JSON.parse(r.postData());const response=await fetch(r.url(),{method:'POST',headers:r.headers(),body:r.postData()});assert.equal(response.status,200);committed=true;await r.abort('failed');return}await r.continue().catch(e=>{if(!/Interception is not enabled|already handled/.test(e.message))throw e})});
 await page.click('#confirm');await page.waitForFunction(()=>!document.getElementById('modal-error').hidden);assert.equal(committed,true);
 const revision=cli('work','show',works['1']).work.revision;await page.click('#confirm');await page.waitForFunction(()=>!document.getElementById('modal').open);
 assert.equal(cli('work','show',works['1']).work.revision,revision);assert.equal(requestBody.task,'t0');
 await page.setRequestInterception(false);page.removeAllListeners('request');checks.push('réponse perdue après commit : état relu, même event_id, aucune double mutation');
 await page.setOfflineMode(true);await page.evaluate(()=>refresh(true));assert.match(await page.$eval('#connection',e=>e.textContent),/Non actualisé.*dernière lecture/);await capture('deconnexion');await page.setOfflineMode(false);await page.evaluate(()=>refresh(true));assert.match(await page.$eval('#connection',e=>e.textContent),/Connecté/);checks.push('déconnexion datée, état conservé puis reconnexion');
 for(const theme of ['sombre','etat']){await page.evaluate(t=>setTheme(t),theme);await page.screenshot({path:path.join(out,'resilience-'+theme+'.png'),fullPage:true})}
 // An orphan decision keeps navigation even when its historical session is absent.
 execFileSync('python3',['-c',`import sqlite3,json,sys
from pathlib import Path
r=Path(sys.argv[1]);assert r.name.startswith('swarm-pilotage-corpus-');c=sqlite3.connect(r/'.swarm/state.db');d=dict(id='missing-agent-decision',task_id='t0',agent_id='missing-agent',kind='execution',summary='Tentative indisponible',evidence='Recette de suppression',created='2026-09-15T09:00:00Z');c.execute('INSERT INTO decisions(id,work_id,body) VALUES(?,?,?)',(d['id'],sys.argv[2],json.dumps(d)));c.commit()`,root,works['1']]);
 await page.evaluate(()=>refresh(true));await page.click('#pilot-next');await page.waitForFunction(()=>document.getElementById('pilot-inspector-title').textContent==='Tentative indisponible');
 await page.waitForSelector('[data-inspector-action="return"]');await capture('agent-introuvable');await page.click('[data-inspector-action="return"]');
 checks.push('agent historique introuvable : erreur explicite et retour à la vue toujours accessibles');
 const stored=await browser.newPage();await stored.goto(new URL(url).origin);await stored.waitForSelector('#pilot-view');await choose(stored,'1');
 await stored.evaluate(()=>localStorage.setItem(Pilot.key,'{broken'));await stored.reload();await stored.waitForSelector('#pilot-status');assert.match(await stored.$eval('#pilot-status',e=>e.textContent),/Préférences indisponibles/);
 await stored.evaluate(()=>localStorage.setItem(Pilot.key,JSON.stringify({version:1,view:'dependencies',collapsed:['deleted-task'],selection:{kind:'task',id:'deleted-task'}})));await stored.reload();await stored.waitForSelector('#pilot-status');
 assert.equal(await stored.evaluate(()=>Pilot.state.selection),null);assert.deepEqual(await stored.evaluate(()=>Pilot.state.collapsed),[]);await stored.close();
 checks.push('JSON de préférences invalide et identifiants supprimés purgés sans mutation métier');
 const settings=await browser.newPage();await settings.goto('chrome://settings/appearance');
 await settings.evaluate(()=>new Promise(resolve=>chrome.settingsPrivate.setDefaultZoom(2,resolve)));
 const zoomed=await browser.newPage();await zoomed.setViewport({width:1440,height:1000});await zoomed.goto(new URL(url).origin);await zoomed.waitForSelector('#pilot-view');await choose(zoomed,'200');
 assert.equal(await zoomed.evaluate(()=>devicePixelRatio),2);assert.equal(await zoomed.evaluate(()=>innerWidth),720);
 await select(zoomed,'#pilot-view','agents');await zoomed.$eval('.pilot-card-title',e=>e.focus());await zoomed.keyboard.press('Enter');await zoomed.waitForSelector('#pilot-inspector[open]');
 assert.equal(await zoomed.$eval('#pilot-inspector',e=>e.matches(':modal')),true);
 for(const theme of ['etat','sombre']){await zoomed.evaluate(t=>setTheme(t),theme);await zoomed.screenshot({path:path.join(out,'browser-zoom-200-'+theme+'.png')})}
 await zoomed.keyboard.press('Escape');assert.equal(await zoomed.$eval('#pilot-inspector',e=>e.open),false);
 await settings.evaluate(()=>new Promise(resolve=>chrome.settingsPrivate.setDefaultZoom(1,resolve)));await settings.close();await zoomed.close();
 checks.push('zoom navigateur réel 200 %, titres longs, clavier et panneau modal dans les deux thèmes');
 assert.deepEqual(errors,[]);const result={status:'PASS',checks,measures,environment:{cpu:os.cpus()[0].model,platform:os.platform(),browser:await browser.version(),viewport:await page.viewport()},errors,scope:'Fixtures locales uniquement ; aucun modèle IA appelé'};fs.writeFileSync(path.join(out,'resilience.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
})().catch(async e=>{console.error(e.stack);console.error(JSON.stringify({checks,errors,measures}));if(page)fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify(await page.evaluate(()=>({message:document.querySelector('#message')?.innerText,modal:document.querySelector('#modal')?.innerText,open:document.querySelector('#modal')?.open})),null,2));if(page)await page.screenshot({path:path.join(out,'failure.png'),fullPage:true}).catch(()=>{});process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
