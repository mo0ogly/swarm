'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),crypto=require('crypto');
const {spawn,execFileSync}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-extension-'));
const cli=(args,input)=>JSON.parse(execFileSync(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])],{encoding:'utf8',input:input?JSON.stringify(input):undefined}));
cli(['init']);let w=cli(['work','create'],{schema_version:1,event_id:'fixture',expected_revision:0,title:'Reprise contrôlée',objective:'Rendre une reprise accessible',scope:'Isolated fixture',criteria:['Evidence retained'],next:'Inspect'}).work;
w=cli(['task','add',w.id],{schema_version:1,event_id:'task',expected_revision:w.revision,id:'exhausted',title:'E3 — Vérification des preuves',owner:'fixture',deliverable:'docs/report.md',criteria:['Evidence checked'],next:'Correct missing sources',max_attempts:2,max_tool_calls:100}).work;
execFileSync('python3',['-c',`import json,sqlite3,sys
from pathlib import Path
r=Path(sys.argv[1]);assert r.name.startswith('swarm-extension-');wid=sys.argv[2]
c=sqlite3.connect(r/'.swarm/state.db');w=json.loads(c.execute('SELECT body FROM works WHERE id=?',(wid,)).fetchone()[0]);t=w['tasks'][0]
t['plan_max_attempts']=2;t['plan_tool_limit']=100;t['status']='blocked';t['blocker']='Plafond de tentatives atteint';t['attempts']=[{'id':'old-1','status':'failed','started':'2026-09-19T10:00:00Z'},{'id':'old-2','status':'completed','started':'2026-09-19T11:00:00Z'}]
t['independent_review']={'id':'old-opinion','state':'changes_requested','attempt':'old-2','reason':'Missing source evidence'}
w['revision']+=1;c.execute('UPDATE works SET revision=?,body=? WHERE id=?',(w['revision'],json.dumps(w),wid));c.commit()
`,root,w.id]);
const read=()=>cli(['work','show',w.id]).work;const before=read();
let browser,app,page;const errors=[],failed=[];let expectedConflict=false,conflictSeen=false;
(async()=>{
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const base=await new Promise((resolve,reject)=>{let s='';app.stdout.on('data',d=>{s+=d;const m=s.match(/http:\/\/\S+\/session\/\S+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});const p=await browser.newPage();page=p;
 p.on('pageerror',e=>errors.push(e.message));p.on('console',m=>{if(m.type()==='error'&&!(expectedConflict&&m.text().includes('409')))errors.push(m.text())});
 p.on('response',r=>{if(r.status()>=400){if(expectedConflict&&r.status()===409&&r.url().includes('action=extend-attempt'))conflictSeen=true;else failed.push(r.status()+' '+r.url())}});
 p.on('requestfailed',r=>{if(!r.failure()?.errorText.includes('ERR_ABORTED'))failed.push(r.url()+' '+r.failure()?.errorText)});
 const url=new URL(base);url.searchParams.set('work',w.id);
 await p.setViewport({width:1440,height:1050});
 for(const lang of ['fr','en']){url.searchParams.set('lang',lang);await p.goto(url.href);await p.waitForSelector('[data-extend-attempt="exhausted"]');
  for(const theme of ['sombre','etat']){await p.evaluate(t=>setTheme(t),theme);const selector='[data-extend-attempt="exhausted"]';await p.click(selector);await p.waitForSelector('#attempt-extension-summary');
   assert.match(await p.$eval('#attempt-extension-summary',e=>e.textContent),/2 \/ 2 → 3/);if(lang==='en')assert.doesNotMatch(await p.$eval('#modal',e=>e.textContent),/Pourquoi autoriser|Historique, rapports|Ce que l’agent|Le plafond est atteint|Autoriser une tentative/);assert.match(await p.$eval('#attempt-extension-summary',e=>e.textContent),/100/);
   assert.equal(await p.$eval('#action-form',e=>e.checkValidity()),false);
   await p.click('#modal-help-toggle');await p.waitForSelector('[data-topic="attempt-extension"]');await p.click('#modal-help-toggle');await p.screenshot({path:path.join(out,`extension-${lang}-${theme}.png`)});await p.keyboard.press('Escape');await p.waitForFunction(()=>!document.querySelector('#modal').open);
   assert.equal(await p.evaluate(()=>document.activeElement?.dataset.extendAttempt),'exhausted','Esc restores focus');assert.deepEqual(read(),before,'cancel changed state');
  }
 }
 await p.evaluate(()=>Pilot.inspect('task','exhausted'));await p.waitForSelector('[data-recovery-action="extend-attempt"]');await p.click('[data-recovery-action="extend-attempt"]');await p.waitForSelector('#attempt-extension-summary');
 await p.type('#field-reason','Operator approved one recovery');await p.type('#field-recovery_instruction','Supply candidate-bound sources and repeat the same evidence checks.');
 const current=read();cli(['task','update',w.id],{schema_version:1,event_id:crypto.randomUUID(),expected_revision:current.revision,id:'exhausted',next:'Concurrent operator update to invalidate the displayed form'});
 expectedConflict=true;await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal-error').hidden);assert.equal(conflictSeen,true);assert.equal(read().tasks[0].plan_max_attempts,2);await p.keyboard.press('Escape');expectedConflict=false;
 await p.reload();await p.waitForSelector('[data-extend-attempt="exhausted"]');await p.click('[data-extend-attempt="exhausted"]');await p.waitForSelector('#attempt-extension-summary');
 await p.type('#field-reason','Operator approved one recovery');await p.type('#field-recovery_instruction','Supply candidate-bound sources and repeat the same evidence checks.');await p.setViewport({width:390,height:844});await p.screenshot({path:path.join(out,'extension-mobile.png')});
 assert.equal(await p.$eval('#modal',e=>e.scrollWidth<=e.clientWidth+1),true,'modal overflows mobile');await p.click('#confirm');
 await p.waitForFunction(()=>snapshot.work.tasks[0].plan_max_attempts===3);const after=read();assert.equal(after.tasks[0].status,'blocked');assert.equal(after.tasks[0].plan_tool_limit,100);assert.deepEqual(after.tasks[0].attempts,before.tasks[0].attempts);assert.deepEqual(after.tasks[0].independent_review,before.tasks[0].independent_review);
 assert.equal(await p.evaluate(()=>snapshot.agents.length),0,'authorization launched a provider');
 if(await p.$('#modal[open]'))await p.keyboard.press('Escape');await p.reload();await p.waitForFunction(()=>snapshot?.work.tasks[0].plan_max_attempts===3);assert.equal(await p.$('[data-extend-attempt="exhausted"]'),null,'unnecessary extension offered while third attempt remains');
 assert.deepEqual(errors,[]);assert.deepEqual(failed,[]);const result={status:'PASS',root,checks:['visible priority and inspector action','French/English × dark/light','required fields','Escape restores focus','cancel preserves state','stale revision refused','authorization persisted','history and review retained','100-tool ceiling retained','no provider started','fourth attempt refused','mobile layout','no unexpected browser errors']};fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
})().catch(async e=>{console.error(e);console.error(JSON.stringify({errors,failed}));if(page){await page.screenshot({path:path.join(out,'failure.png')});console.error(await page.evaluate(()=>({body:document.body.innerText.slice(-5000),actions:snapshot?.task_actions,modal:modalContext}))) }process.exitCode=1}).finally(async()=>{await browser?.close();app?.kill()});
