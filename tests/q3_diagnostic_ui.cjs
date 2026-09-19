'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {spawn,spawnSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-q3-'));
const run=(args,input)=>{const r=spawnSync(binary,['--root',root,...args],{encoding:'utf8',input,timeout:10000});if(r.status!==0||r.signal||r.error)throw Object.assign(r.error||Error(r.stderr),{result:r});return r.stdout};
const json=(args,input)=>JSON.parse(run(['--json',...args,...(input!==undefined?['--input','-']:[])],input===undefined?undefined:JSON.stringify(input)));
const id=()=>require('node:crypto').randomUUID().replaceAll('-','');
json(['init']);
const provider=path.join(root,'q3-provider.py');
fs.writeFileSync(provider,`import json,sys,time
sys.stdin.read()
events=[
 ("c1","missing-tool --version","command not found"),
 ("c2","mount /mnt/partage","operation not permitted by sandbox mount"),
 ("c3","go test ./...","exit code 1")]
for ident,command,error in events:
 print(json.dumps({"type":"item.started","item":{"id":ident,"type":"command_execution","command":command}}),flush=True)
 print(json.dumps({"type":"item.completed","item":{"id":ident,"type":"command_execution","status":"failed","exit_code":1,"error":error}}),flush=True)
time.sleep(2)
`);
fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{recette:{command:process.env.PYTHON_BIN||'/usr/bin/python3',args:[provider],env_allow:[],limits:{max_consecutive_errors:3}}}}));
let w=json(['work','create'],{schema_version:1,event_id:id(),expected_revision:0,title:'Recette Q3',objective:'Expliquer une tentative en échec',scope:'racine isolée',criteria:['diagnostic commun'],next:'lancer'}).work;
w=json(['task','add',w.id],{schema_version:1,event_id:id(),expected_revision:w.revision,id:'q3',title:'Trois erreurs contextualisées',deliverable:'docs/q3.md',criteria:['cause, conséquence et action'],owner:'recette',next:'diagnostiquer'}).work;
json(['agent','start',w.id],{schema_version:1,event_id:id(),expected_revision:w.revision,task_id:'q3',provider:'recette',workspace:root,role:'worker',instruction:'recette locale',capture_output:true});
let ended;
for(let i=0;i<80;i++){ended=json(['agent','list',w.id]).agents[0]?.agent;if(ended&&!['queued','starting','running','stopping'].includes(ended.status))break;Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,100)}
assert.equal(ended.status,'interrupted');assert.equal(ended.diagnostic.observed_errors,3);assert.equal(ended.diagnostic.consecutive_error_limit,3);assert.equal(ended.diagnostic.limit_reached,true);
const cli=run(['mission','status',w.id]);for(const text of ['3 erreurs d’outil observées','Cause :','Conséquence :','Action disponible :','Traces techniques :'])assert.match(cli,new RegExp(text));
const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
(async()=>{
 const url=await new Promise((resolve,reject)=>{let text='',err='';const timer=setTimeout(()=>reject(Error(text+err)),10000);server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});server.stderr.on('data',d=>err+=d);server.on('error',reject)});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome'});const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.setViewport({width:1280,height:900});await page.goto(url+'?work='+w.id);await page.waitForFunction(()=>snapshot?.mission?.tasks?.[0]?.diagnostic?.observed_errors===3);
 const results=await page.$('#mission-results>summary');await results.focus();await page.keyboard.press('Enter');await page.waitForFunction(()=>document.querySelector('#mission-results').open);
 const diagnostic=await page.$eval('.mission-task .mission-diagnostic',e=>e.textContent);for(const text of ['3 erreurs d’outil observées','plafond configuré est de 3 erreurs consécutives','Cause :','Conséquence :','Action disponible :'])assert.match(diagnostic,new RegExp(text));
 const traces=await page.$('.mission-task .mission-diagnostic-item details>summary');await traces.focus();await page.keyboard.press('Enter');assert.equal(await page.$eval('.mission-task .mission-diagnostic-item details',e=>e.open),true);
 await page.evaluate(()=>{Mission.key='';Mission.render()});assert.equal(await page.$eval('#mission-results',e=>e.open),true);assert.equal(await page.$eval('.mission-task .mission-diagnostic-item details',e=>e.open),true);assert.match(await page.evaluate(()=>document.activeElement.parentElement.dataset.missionDetail),/^task-q3-/);
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);await page.screenshot({path:path.join(out,theme+'.png'),fullPage:true})}
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['diagnostic CLI/web commun','3 erreurs distinctes du plafond de 3','cause, conséquence et action','traces au clavier','thèmes etat/sombre'],errors},null,2));console.log('PASS Q3 diagnostic CLI/web, clavier et deux thèmes');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();server.kill()});
