'use strict';
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-recovery-'));
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined}));
cli(['init']);const work=cli(['work','create'],{schema_version:1,event_id:crypto.randomUUID(),expected_revision:0,title:'Parcours de résolution',objective:'Annuler une attente puis préparer sa reprise',scope:'Recette isolée',criteria:['Résultat visible'],next:'Vérifier'}).work.id;cli(['autonomy',work,'autonome']);
fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{fixture:{command:'/bin/false',args:[],env_allow:[]}}}));
execFileSync('python3',['-c',`import json,sqlite3,sys
from pathlib import Path
r=Path(sys.argv[1]);assert r.name.startswith('swarm-recovery-');wid=sys.argv[2]
c=sqlite3.connect(r/'.swarm/state.db');w=json.loads(c.execute('SELECT body FROM works WHERE id=?',(wid,)).fetchone()[0])
w['tasks']=[]
for tid,status in [('pending','queued'),('foreign','starting')]:
 attempt='attempt-'+tid
 w['tasks'].append(dict(id=tid,title='Mission '+tid,status='running',depends=[],deliverable='Rapport',criteria=['Rapport relu'],attempts=[dict(id=attempt,status='recorded',started='2026-09-15T08:00:00Z')],next='Reprendre explicitement'))
 w['tasks'][-1]['launch_profile']=dict(provider='fixture',role='worker',workspace=str(r/tid),instruction='Recette',timeout_seconds=30)
 a=dict(id='agent-'+tid,task_id=tid,work_id=wid,attempt_id=attempt,provider='fixture',role='worker',workspace=str(r/tid),status=status,host='autre-session',started='2026-09-15T08:00:00Z',progress={})
 if status=='starting': a['supervisor_pid']=123;a['heartbeat']='2026-09-15T08:00:00Z'
 c.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,body,request,desired) VALUES(?,?,?,?,?,?,?,?)',(a['id'],wid,tid,a['workspace'],status,json.dumps(a),'{}','stop'))
 c.execute("INSERT INTO reservations(agent_id,work_id,amount,state) VALUES(?,?,1,'reserved')",(a['id'],wid))
c.execute("UPDATE agents SET status='starting' WHERE id='agent-pending'")
w['revision']+=1;c.execute('UPDATE works SET revision=?,body=? WHERE id=?',(w['revision'],json.dumps(w),wid));c.commit()
`,root,work]);
let browser;const errors=[];const server=spawn(binary,['--root',root,'web','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
(async()=>{
 const url=await new Promise((resolve,reject)=>{let s='';server.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});server.once('exit',c=>reject(Error('Server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1440,height:1000});await page.goto(url);await page.waitForSelector('#pilot-view');
 await page.evaluate(()=>Pilot.inspect('task','pending'));await page.waitForSelector('[data-recovery-action=reconcile]');
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.$eval('#pilot-inspector',e=>e.scrollTop=0);await page.screenshot({path:path.join(out,'annulation-'+theme+'.png')});
  await page.click('[data-recovery-action=reconcile]');await page.waitForFunction(()=>modalContext?.cancelPending);assert.equal(await page.$eval('#confirm',e=>e.textContent),'Confirmer l’annulation du démarrage');await page.screenshot({path:path.join(out,'confirmation-'+theme+'.png')});await page.keyboard.press('Escape');
 }
 await page.click('[data-recovery-action=reconcile]');await page.waitForFunction(()=>modalContext?.cancelPending);await page.click('#confirm');
 await page.waitForFunction(()=>snapshot?.work.tasks.find(t=>t.id==='pending').status==='blocked');
 assert.equal(await page.evaluate(()=>snapshot.agents.find(x=>x.agent.id==='agent-pending').agent.status),'interrupted');
 await page.waitForSelector('[data-recovery-action=retry]');assert.equal(await page.$('[data-recovery-action=reconcile]'),null);
 const released=execFileSync('python3',['-c',"import sqlite3,sys;print(sqlite3.connect(sys.argv[1]).execute(\"SELECT state FROM reservations WHERE agent_id='agent-pending'\").fetchone()[0])",path.join(root,'.swarm/state.db')],{encoding:'utf8'}).trim();assert.equal(released,'released');
 assert.equal(await page.evaluate(()=>snapshot.agents.length),2);
 assert.equal(await page.evaluate(()=>snapshot.agents.find(x=>x.agent.id==='agent-pending').agent.stop_kind),'operateur');
 for(const theme of ['etat','sombre']){await page.evaluate(t=>setTheme(t),theme);await page.$eval('#pilot-inspector',e=>e.scrollTop=0);await page.screenshot({path:path.join(out,'reprise-'+theme+'.png')})}
 await page.reload();await page.waitForSelector('[data-recovery-action=retry]');await page.click('[data-recovery-action=retry]');await page.waitForFunction(()=>modalContext?.action==='retry');assert.equal(await page.$eval('#field-agent',e=>e.value),'agent-pending');await page.keyboard.press('Escape');
 await page.evaluate(()=>Pilot.inspect('task','foreign'));await page.waitForFunction(()=>document.querySelector('#pilot-recovery')?.textContent.includes('session d’origine'));
 assert.equal(await page.$('#pilot-recovery [data-recovery-action=reconcile]'),null);assert.equal(await page.$('#pilot-recovery [data-recovery-action=retry]'),null);
 assert.deepEqual(errors,[]);console.log(JSON.stringify({status:'PASS',root,checks:['annulation réelle du cas SQL starting / JSON queued via confirmation','réservation libérée','état interrompu et tâche bloquée','relance visible après rechargement et formulaire prérempli','exécution étrangère démarrée protégée','deux thèmes et Échap'],provider_launched:false}));
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{await browser?.close();server.kill()});
