'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {spawn,spawnSync}=require('node:child_process');
const puppeteer=require(process.env.PUPPETEER_MODULE||'puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-q6-'));
const run=(args,input)=>{const r=spawnSync(binary,['--root',root,...args],{encoding:'utf8',input,timeout:10000});if(r.status!==0||r.signal||r.error)throw Object.assign(r.error||Error(r.stderr),{result:r});return r.stdout};
const json=(args,input)=>JSON.parse(run(['--json',...args,...(input!==undefined?['--input','-']:[])],input===undefined?undefined:JSON.stringify(input)));
const id=()=>require('node:crypto').randomUUID().replaceAll('-','');
json(['init']);
const provider=path.join(root,'environment-provider.py');
fs.writeFileSync(provider,`import json,sys
sys.stdin.read()
print(json.dumps({"type":"item.started","item":{"id":"mount","type":"command_execution","command":"mount /mnt/recette"}}),flush=True)
print(json.dumps({"type":"item.completed","item":{"id":"mount","type":"command_execution","status":"failed","exit_code":1,"error":"operation not permitted by sandbox mount"}}),flush=True)
raise SystemExit(2)
`);
fs.writeFileSync(path.join(root,'.swarm/providers.json'),JSON.stringify({schema_version:1,providers:{recette:{command:process.env.PYTHON_BIN||'/usr/bin/python3',args:[provider],env_allow:[],limits:{max_tool_calls:9,max_repeated_calls:2}}}}));
let w=json(['work','create'],{schema_version:1,event_id:id(),expected_revision:0,title:'Recette Q6',objective:'Reprendre seulement après vérification',scope:'racine isolée',criteria:['pas de relance vide'],next:'lancer'}).work;
w=json(['task','add',w.id],{schema_version:1,event_id:id(),expected_revision:w.revision,id:'q6',title:'Montage indisponible',deliverable:'docs/q6.md',criteria:['reprise explicite'],owner:'recette',next:'vérifier le montage'}).work;
json(['agent','start',w.id],{schema_version:1,event_id:id(),expected_revision:w.revision,task_id:'q6',provider:'recette',workspace:root,role:'worker',instruction:'recette locale',capture_output:true});
let ended;for(let i=0;i<80;i++){ended=json(['agent','list',w.id]).agents[0]?.agent;if(ended&&!['queued','starting','running','stopping'].includes(ended.status))break;Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,100)}
assert.equal(ended.status,'failed');assert.ok(ended.diagnostic.items.some(x=>x.category==='environment'));
const server=spawn(binary,['--root',root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
(async()=>{
 const url=await new Promise((resolve,reject)=>{let text='',err='';const timer=setTimeout(()=>reject(Error(text+err)),10000);server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});server.stderr.on('data',d=>err+=d);server.on('error',reject)});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome'});const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(15000);await page.setViewport({width:1280,height:900});await page.goto(url+'?work='+w.id);await page.waitForFunction(()=>snapshot?.agents?.[0]?.agent?.diagnostic?.items?.some(x=>x.category==='environment'));
 await page.click('#mode');await page.click('[data-view="tasks"]');await page.click('#tasks-body [data-task="q6"] button');await page.waitForSelector('#modal[open] #field-action');
 const actions=await page.$$eval('#field-action option',es=>es.map(e=>e.value));assert.ok(actions.includes('retry'));assert.ok(!actions.includes('start'));
 await page.select('#field-action','retry');await page.waitForSelector('#field-precondition_evidence',{visible:true});assert.equal(await page.$eval('#field-precondition_evidence',e=>e.required),true);
 await page.$eval('#field-precondition_evidence',e=>e.scrollIntoView({block:'center'}));
 for(const theme of ['etat','sombre']){await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);const visible=await page.$eval('#field-precondition_evidence',e=>{const s=getComputedStyle(e),r=e.getBoundingClientRect();return s.visibility!=='hidden'&&r.width>0&&r.height>0});assert.equal(visible,true);await page.screenshot({path:path.join(out,theme+'.png'),fullPage:true})}
 await page.focus('#field-precondition_evidence');await page.keyboard.type('Lecture hote et ecriture de recette verifiees');assert.equal(await page.evaluate(()=>document.activeElement.id),'field-precondition_evidence');await page.keyboard.press('Tab');assert.equal(await page.evaluate(()=>document.activeElement.id),'field-level');
 await page.click('#confirm');await page.waitForFunction(()=>!document.getElementById('modal').open);await page.waitForFunction(()=>snapshot.agents.filter(x=>x.agent.task_id==='q6').length===2);
 const resumed=await page.evaluate(()=>snapshot.agents.find(x=>x.agent.precondition_evidence)?.agent);assert.match(resumed.precondition_evidence,/Lecture hote/);assert.equal(resumed.limits.max_tool_calls,9);assert.equal(resumed.limits.max_repeated_calls,2);
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['relance automatique retenue','départ simple retiré','preuve requise au clavier','reprise explicite enregistrée','limites conservées','deux thèmes'],errors},null,2));console.log('PASS Q6 reprise environnement, clavier et deux thèmes');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(async()=>{if(browser)await browser.close();server.kill()});
